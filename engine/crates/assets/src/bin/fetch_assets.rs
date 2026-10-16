//! Download the assets listed in `engine/assets/manifest.toml` into
//! `engine/assets/cache/<id>/<res>/...`.
//!
//! ```text
//! cargo run -p gridlock-assets --bin fetch-assets --release [-- OPTIONS]
//!
//!   --res 1k|2k|4k       texture + model resolution (default: manifest, 2k)
//!   --hdri-res 1k|2k|4k  HDRI resolution (default: manifest, 4k)
//!   --only <prefix>      only ids starting with prefix (repeatable; selects optional assets too)
//!   --all                include assets marked optional (big trees, extra HDRIs)
//!   --list               show what would be fetched and what is cached, then exit
//!   --dry-run            plan only
//!   --verify             re-check MD5 of files already present (re-download on mismatch)
//!   --jobs N             parallel downloads (default 4)
//!   --curl               use the system curl for all transfers
//!   --no-curl-fallback   never fall back to curl
//!   --keep-archives      keep downloaded .zip files after extraction
//!   --assets-dir DIR     directory holding manifest.toml (default: auto-detect)
//!   --cache-dir DIR      download target (default: <assets-dir>/cache)
//! ```
//!
//! Files already present with the right size are skipped; interrupted
//! downloads resume from `*.part`; each transfer is retried with exponential
//! backoff and verified by size (and MD5 where the source publishes one).

use std::collections::VecDeque;
use std::io::{IsTerminal, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use gridlock_assets::fetch::{self, human_bytes, Backend, Client, Job, PlanOptions};
use gridlock_assets::manifest::{Manifest, Res, Role};
use gridlock_assets::AssetStore;

struct Args {
    res: Option<Res>,
    hdri_res: Option<Res>,
    only: Vec<String>,
    all: bool,
    list: bool,
    dry_run: bool,
    verify: bool,
    jobs: usize,
    backend: Backend,
    keep_archives: bool,
    assets_dir: Option<PathBuf>,
    cache_dir: Option<PathBuf>,
}

fn usage() -> &'static str {
    "usage: fetch-assets [--res 1k|2k|4k] [--hdri-res 1k|2k|4k] [--only <id-prefix>]... [--all]\n\
     \x20                   [--list] [--dry-run] [--verify] [--jobs N] [--curl] [--no-curl-fallback]\n\
     \x20                   [--keep-archives] [--assets-dir DIR] [--cache-dir DIR]"
}

fn parse_args() -> Result<Args, String> {
    let mut a = Args {
        res: None,
        hdri_res: None,
        only: vec![],
        all: false,
        list: false,
        dry_run: false,
        verify: false,
        jobs: 4,
        backend: Backend::Auto,
        keep_archives: false,
        assets_dir: None,
        cache_dir: None,
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        let mut val = |name: &str| it.next().ok_or_else(|| format!("{name} needs a value"));
        match arg.as_str() {
            "--res" => a.res = Some(val("--res")?.parse()?),
            "--hdri-res" => a.hdri_res = Some(val("--hdri-res")?.parse()?),
            "--only" => a.only.push(val("--only")?),
            "--all" => a.all = true,
            "--list" => a.list = true,
            "--dry-run" => a.dry_run = true,
            "--verify" => a.verify = true,
            "--jobs" | "-j" => {
                a.jobs = val("--jobs")?
                    .parse()
                    .map_err(|_| "--jobs needs a number".to_string())?
            }
            "--curl" => a.backend = Backend::Curl,
            "--no-curl-fallback" => a.backend = Backend::Ureq,
            "--keep-archives" => a.keep_archives = true,
            "--assets-dir" => a.assets_dir = Some(PathBuf::from(val("--assets-dir")?)),
            "--cache-dir" => a.cache_dir = Some(PathBuf::from(val("--cache-dir")?)),
            "-h" | "--help" => return Err(String::new()),
            other => return Err(format!("unknown argument '{other}'")),
        }
    }
    a.jobs = a.jobs.clamp(1, 16);
    Ok(a)
}

fn fmt_dur(d: Duration) -> String {
    let s = d.as_secs();
    if s >= 3600 {
        format!("{}h{:02}m", s / 3600, (s % 3600) / 60)
    } else if s >= 60 {
        format!("{}m{:02}s", s / 60, s % 60)
    } else {
        format!("{s}s")
    }
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            if !e.is_empty() {
                eprintln!("error: {e}");
            }
            eprintln!("{}", usage());
            return if e.is_empty() {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(2)
            };
        }
    };
    match run(args) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: Args) -> Result<bool, String> {
    let assets_dir = args
        .assets_dir
        .clone()
        .unwrap_or_else(AssetStore::find_assets_dir);
    let manifest_path = assets_dir.join("manifest.toml");
    let manifest = Manifest::load(&manifest_path).map_err(|e| e.to_string())?;
    let cache_dir = args
        .cache_dir
        .clone()
        .or_else(|| std::env::var_os(gridlock_assets::store::ENV_CACHE_DIR).map(PathBuf::from))
        .unwrap_or_else(|| assets_dir.join("cache"));
    let d = &manifest.defaults;
    let opts = PlanOptions {
        texture_res: args.res.unwrap_or(d.texture_res),
        model_res: args.res.unwrap_or(d.model_res),
        hdri_res: args.hdri_res.unwrap_or(d.hdri_res),
        only: args.only.clone(),
        include_optional: args.all,
    };
    let mut jobs = fetch::plan(&manifest, &cache_dir, &opts);
    if jobs.is_empty() {
        println!("nothing selected (check --only)");
        return Ok(true);
    }
    println!("manifest : {}", manifest_path.display());
    println!("cache    : {}", cache_dir.display());
    println!(
        "selection: textures {}, models {}, HDRIs {}{}{}",
        opts.texture_res,
        opts.model_res,
        opts.hdri_res,
        if args.all { ", +optional" } else { "" },
        if args.only.is_empty() {
            String::new()
        } else {
            format!(", only {:?}", args.only)
        }
    );

    if args.verify {
        for j in jobs.iter_mut().filter(|j| j.done && j.file.md5.is_some()) {
            let want = j.file.md5.as_deref().unwrap_or_default();
            match fetch::md5_file(&j.dest) {
                Ok(h) if h.eq_ignore_ascii_case(want) => {}
                _ => {
                    println!("md5 mismatch, will re-download: {}", j.dest.display());
                    let _ = std::fs::remove_file(&j.dest);
                    j.done = false;
                }
            }
        }
    }

    let total: u64 = jobs.iter().map(|j| j.file.size).sum();
    let todo: Vec<Job> = jobs.iter().filter(|j| !j.done).cloned().collect();
    let todo_bytes: u64 = todo.iter().map(|j| j.file.size).sum();

    if args.list || args.dry_run {
        let mut last = String::new();
        for j in &jobs {
            if j.asset_id != last {
                let size: u64 = jobs
                    .iter()
                    .filter(|x| x.asset_id == j.asset_id)
                    .map(|x| x.file.size)
                    .sum();
                let cached = jobs
                    .iter()
                    .filter(|x| x.asset_id == j.asset_id)
                    .all(|x| x.done);
                println!(
                    "{:<30} {:>3}  {:>10}  {}",
                    j.asset_id,
                    j.res,
                    human_bytes(size),
                    if cached { "cached" } else { "missing" }
                );
                last = j.asset_id.clone();
            }
        }
        println!(
            "\n{} files, {} total; {} files / {} to download",
            jobs.len(),
            human_bytes(total),
            todo.len(),
            human_bytes(todo_bytes)
        );
        return Ok(true);
    }

    println!(
        "{} files, {} total; {} already present, downloading {} files / {}",
        jobs.len(),
        human_bytes(total),
        jobs.len() - todo.len(),
        todo.len(),
        human_bytes(todo_bytes)
    );
    if todo.is_empty() {
        println!("everything is up to date");
        return Ok(true);
    }

    let client = Client::new(args.backend);
    if args.backend == Backend::Auto && !client.curl_available() {
        println!("(curl not found: no fallback if ureq fails)");
    }
    let n_todo = todo.len();
    let queue = Mutex::new(VecDeque::from(todo));
    // Bytes already in resumable `.part` files count as done for progress.
    let resumed: u64 = queue
        .lock()
        .expect("queue")
        .iter()
        .map(|j| {
            let mut p = j.dest.clone().into_os_string();
            p.push(".part");
            std::fs::metadata(p)
                .map(|m| m.len().min(j.file.size))
                .unwrap_or(0)
        })
        .sum();
    let bytes = AtomicU64::new(resumed);
    let files_done = AtomicUsize::new(0);
    let failures: Mutex<Vec<String>> = Mutex::new(Vec::new());
    let finished = AtomicBool::new(false);
    let tty = std::io::stdout().is_terminal();
    let print_lock = Mutex::new(());
    let start = Instant::now();

    let status_line = |final_line: bool| {
        let b = bytes.load(Ordering::Relaxed);
        let el = start.elapsed().as_secs_f64().max(0.001);
        let rate = b.saturating_sub(resumed) as f64 / el;
        let eta = if rate > 0.0 && b < todo_bytes {
            fmt_dur(Duration::from_secs_f64((todo_bytes - b) as f64 / rate))
        } else {
            "-".into()
        };
        let pct = if todo_bytes > 0 {
            b as f64 * 100.0 / todo_bytes as f64
        } else {
            100.0
        };
        let s = format!(
            "[{:>3}/{:<3} files] {:>10} / {:<10} {:5.1}%  {:>9}/s  ETA {}",
            files_done.load(Ordering::Relaxed),
            n_todo,
            human_bytes(b),
            human_bytes(todo_bytes),
            pct.min(100.0),
            human_bytes(rate as u64),
            if final_line { "-".to_string() } else { eta }
        );
        format!("{s:<92}")
    };

    std::thread::scope(|s| {
        s.spawn(|| {
            let period = if tty {
                Duration::from_millis(250)
            } else {
                Duration::from_secs(5)
            };
            let mut last = Instant::now();
            while !finished.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(100));
                if last.elapsed() >= period {
                    last = Instant::now();
                    let _g = print_lock.lock();
                    let mut out = std::io::stdout().lock();
                    if tty {
                        let _ = write!(out, "\r{}", status_line(false));
                    } else {
                        let _ = writeln!(out, "{}", status_line(false));
                    }
                    let _ = out.flush();
                }
            }
        });
        let workers: Vec<_> = (0..args.jobs.min(n_todo))
            .map(|_| {
                s.spawn(|| loop {
                    let job = { queue.lock().expect("queue").pop_front() };
                    let Some(job) = job else { break };
                    let res = client
                        .download(
                            &job.file.url,
                            &job.dest,
                            job.file.size,
                            job.file.md5.as_deref(),
                            &bytes,
                        )
                        .and_then(|()| {
                            if job.file.role == Role::Archive {
                                let dir = cache_dir.join(&job.asset_id);
                                fetch::extract_zip(&job.dest, &dir, &job.file.members)?;
                                if !args.keep_archives {
                                    let _ = std::fs::remove_file(&job.dest);
                                }
                            }
                            Ok(())
                        });
                    files_done.fetch_add(1, Ordering::Relaxed);
                    let line = match &res {
                        Ok(()) => format!(
                            "  ok   {:<30} {:>9}  {}",
                            job.asset_id,
                            human_bytes(job.file.size),
                            job.file.path
                        ),
                        Err(e) => {
                            failures
                                .lock()
                                .expect("failures")
                                .push(format!("{}: {e}", job.asset_id));
                            format!("  FAIL {:<30} {}", job.asset_id, e)
                        }
                    };
                    let _g = print_lock.lock();
                    let mut out = std::io::stdout().lock();
                    if tty {
                        let _ = writeln!(out, "\r{line:<92}");
                    } else {
                        let _ = writeln!(out, "{line}");
                    }
                    let _ = out.flush();
                })
            })
            .collect();
        for w in workers {
            let _ = w.join();
        }
        finished.store(true, Ordering::Relaxed);
    });

    println!("\r{}", status_line(true));
    let failures = failures.into_inner().expect("failures");
    println!(
        "done in {}: downloaded {} in {} files",
        fmt_dur(start.elapsed()),
        human_bytes(bytes.load(Ordering::Relaxed).saturating_sub(resumed)),
        n_todo - failures.len()
    );
    let mut on_disk = 0u64;
    for j in fetch::plan(&manifest, &cache_dir, &opts) {
        if j.done {
            on_disk += j.file.size;
        }
    }
    println!(
        "selection now cached: {} of {}",
        human_bytes(on_disk),
        human_bytes(total)
    );
    if failures.is_empty() {
        Ok(true)
    } else {
        eprintln!("\n{} file(s) failed:", failures.len());
        for f in &failures {
            eprintln!("  {f}");
        }
        eprintln!("re-run fetch-assets to retry (partial downloads resume).");
        Ok(false)
    }
}
