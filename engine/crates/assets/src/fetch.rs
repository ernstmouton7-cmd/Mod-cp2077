//! Download support shared by the `fetch-assets` and `gen-manifest` binaries
//! (feature `fetch`, on by default; renderers can depend on this crate with
//! `default-features = false` to skip the HTTP stack).
//!
//! HTTP is done with `ureq` + rustls. Root certificates come from the OS
//! store (`rustls-native-certs`: Windows cert store, macOS keychain,
//! `/etc/ssl` / `SSL_CERT_FILE` on Linux; the bundled Mozilla roots only if
//! the OS store is empty), so TLS-intercepting corporate/agent proxies whose CA is
//! installed in the OS store work. `HTTPS_PROXY` / `HTTP_PROXY` /
//! `ALL_PROXY` / `NO_PROXY` are honoured by ureq. If ureq cannot connect at
//! all, downloads fall back to the system `curl` (shipped with Windows 10+ and
//! every Linux distribution), which has its own proxy/TLS handling.

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use md5::{Digest, Md5};

use crate::manifest::{ArchiveMember, AssetEntry, FileEntry, Manifest, Res, Role};
use crate::AssetError;

/// User agent sent with every request (Poly Haven asks API users to identify themselves).
pub const USER_AGENT: &str = concat!(
    "gridlock-assets/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/; CC0 asset fetcher for the Gridlock engine)"
);

/// Which HTTP implementation to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    /// ureq, falling back to curl on connection-level failures.
    Auto,
    /// ureq only.
    Ureq,
    /// The system `curl` executable only.
    Curl,
}

/// HTTP client used for API queries and downloads.
pub struct Client {
    agent: ureq::Agent,
    backend: Backend,
    curl_available: bool,
    /// Number of attempts per request (>= 1).
    pub attempts: u32,
}

fn fetch_err(msg: impl Into<String>) -> AssetError {
    AssetError::Fetch(msg.into())
}

impl Client {
    pub fn new(backend: Backend) -> Self {
        let mut certs: Vec<ureq::tls::Certificate<'static>> = Vec::new();
        let native = rustls_native_certs::load_native_certs();
        for c in native.certs {
            certs.push(ureq::tls::Certificate::from_der(c.as_ref()).to_owned());
        }
        let n_native = certs.len();
        certs.extend(extra_root_certs());
        let roots = if certs.is_empty() {
            ureq::tls::RootCerts::WebPki
        } else {
            ureq::tls::RootCerts::new_with_certs(&certs)
        };
        if n_native == 0 && certs.is_empty() {
            eprintln!("note: no OS root certificates found, using bundled Mozilla roots");
        }
        let tls = ureq::tls::TlsConfig::builder().root_certs(roots).build();
        let proxy = ureq::Proxy::try_from_env();
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .tls_config(tls)
            .proxy(proxy)
            .user_agent(USER_AGENT)
            .timeout_connect(Some(Duration::from_secs(20)))
            .timeout_recv_response(Some(Duration::from_secs(60)))
            .timeout_recv_body(Some(Duration::from_secs(1800)))
            .max_redirects(10)
            .build()
            .into();
        let curl_available = Command::new(curl_exe())
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        Client {
            agent,
            backend,
            curl_available,
            attempts: 5,
        }
    }

    pub fn backend(&self) -> Backend {
        self.backend
    }

    pub fn curl_available(&self) -> bool {
        self.curl_available
    }

    /// GET a (small) resource into memory, with retries.
    pub fn get_bytes(&self, url: &str) -> Result<Vec<u8>, AssetError> {
        let mut last = String::new();
        for attempt in 0..self.attempts.max(1) {
            if attempt > 0 {
                std::thread::sleep(backoff(attempt));
            }
            let r = if self.backend == Backend::Curl {
                curl_get(url)
            } else {
                match self.ureq_get(url) {
                    Err(Failure::Transport(e))
                        if self.backend == Backend::Auto && self.curl_available =>
                    {
                        curl_get(url).map_err(|c| Failure::Transport(format!("{e}; curl: {c:?}")))
                    }
                    other => other,
                }
            };
            match r {
                Ok(b) => return Ok(b),
                Err(Failure::Fatal(e)) => return Err(fetch_err(format!("{url}: {e}"))),
                Err(Failure::Transport(e)) | Err(Failure::Retry(e)) => last = e,
            }
        }
        Err(fetch_err(format!("{url}: {last}")))
    }

    fn ureq_get(&self, url: &str) -> Result<Vec<u8>, Failure> {
        let mut resp = self.agent.get(url).call().map_err(classify)?;
        let mut v = Vec::new();
        resp.body_mut()
            .as_reader()
            .read_to_end(&mut v)
            .map_err(|e| Failure::Retry(e.to_string()))?;
        Ok(v)
    }

    /// Download `url` to `dest`, resuming a previous `dest.part`, verifying
    /// the size and (if given) the MD5. `progress` receives byte deltas.
    pub fn download(
        &self,
        url: &str,
        dest: &Path,
        size: u64,
        md5: Option<&str>,
        progress: &AtomicU64,
    ) -> Result<(), AssetError> {
        if let Some(p) = dest.parent() {
            fs::create_dir_all(p).map_err(|e| AssetError::io(p, e))?;
        }
        let part = part_path(dest);
        let mut last = String::new();
        let mut use_curl = self.backend == Backend::Curl;
        for attempt in 0..self.attempts.max(1) {
            if attempt > 0 {
                let d = backoff(attempt);
                eprintln!(
                    "\n  retry {attempt}/{} in {:.0}s: {} ({last})",
                    self.attempts - 1,
                    d.as_secs_f32(),
                    file_name(dest)
                );
                std::thread::sleep(d);
            }
            let have = fs::metadata(&part).map(|m| m.len()).unwrap_or(0);
            if have > size {
                let _ = fs::remove_file(&part);
            }
            let r = if fs::metadata(&part).map(|m| m.len()).unwrap_or(0) == size {
                Ok(())
            } else if use_curl {
                curl_download(url, &part, progress)
            } else {
                match self.ureq_download(url, &part, size, progress) {
                    Err(Failure::Transport(e))
                        if self.backend == Backend::Auto && self.curl_available =>
                    {
                        eprintln!("\n  ureq failed ({e}); falling back to curl");
                        use_curl = true;
                        curl_download(url, &part, progress)
                    }
                    other => other,
                }
            };
            match r {
                Ok(()) => {}
                Err(Failure::Fatal(e)) => return Err(fetch_err(format!("{url}: {e}"))),
                Err(Failure::Transport(e)) | Err(Failure::Retry(e)) => {
                    last = e;
                    continue;
                }
            }
            let got = fs::metadata(&part).map(|m| m.len()).unwrap_or(0);
            if got != size {
                last = format!("size mismatch: got {got} bytes, expected {size}");
                if got > size {
                    let _ = fs::remove_file(&part);
                }
                continue;
            }
            if let Some(want) = md5 {
                let have = md5_file(&part)?;
                if !have.eq_ignore_ascii_case(want) {
                    last = format!("md5 mismatch: {have} != {want}");
                    let _ = fs::remove_file(&part);
                    progress.fetch_sub(
                        size.min(progress.load(Ordering::Relaxed)),
                        Ordering::Relaxed,
                    );
                    continue;
                }
            }
            fs::rename(&part, dest).map_err(|e| AssetError::io(dest, e))?;
            return Ok(());
        }
        Err(fetch_err(format!(
            "{url}: giving up after {} attempts: {last}",
            self.attempts
        )))
    }

    fn ureq_download(
        &self,
        url: &str,
        part: &Path,
        size: u64,
        progress: &AtomicU64,
    ) -> Result<(), Failure> {
        let have = fs::metadata(part).map(|m| m.len()).unwrap_or(0);
        let mut req = self.agent.get(url);
        if have > 0 && have < size {
            req = req.header("Range", &format!("bytes={have}-"));
        }
        let mut resp = req.call().map_err(classify)?;
        let resumed = resp.status().as_u16() == 206;
        let mut file = if resumed {
            OpenOptions::new().append(true).open(part)
        } else {
            // Server ignored the range: start over (undo the counted bytes).
            if have > 0 {
                progress.fetch_sub(
                    have.min(progress.load(Ordering::Relaxed)),
                    Ordering::Relaxed,
                );
            }
            File::create(part)
        }
        .map_err(|e| Failure::Fatal(format!("{}: {e}", part.display())))?;
        let mut reader = resp.body_mut().as_reader();
        let mut buf = vec![0u8; 256 * 1024];
        loop {
            let n = match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(Failure::Retry(format!("read: {e}"))),
            };
            file.write_all(&buf[..n])
                .map_err(|e| Failure::Fatal(format!("{}: {e}", part.display())))?;
            progress.fetch_add(n as u64, Ordering::Relaxed);
        }
        file.flush().map_err(|e| Failure::Fatal(e.to_string()))?;
        Ok(())
    }
}

/// Extra roots from the PEM file named by `GRIDLOCK_EXTRA_CA` (for unusual
/// setups; normally the OS store is enough). ureq can't merge its bundled
/// Mozilla roots with a specific list, so those are used only when the OS
/// store (plus extras) is empty.
fn extra_root_certs() -> Vec<ureq::tls::Certificate<'static>> {
    let mut v = Vec::new();
    if let Some(p) = std::env::var_os("GRIDLOCK_EXTRA_CA") {
        if let Ok(pem) = fs::read(&p) {
            for item in ureq::tls::parse_pem(&pem).flatten() {
                if let ureq::tls::PemItem::Certificate(c) = item {
                    v.push(c);
                }
            }
        }
    }
    v
}

#[derive(Debug)]
enum Failure {
    /// Could not talk to the server at all (DNS, TCP, TLS, proxy).
    Transport(String),
    /// Server answered but the transfer failed / 5xx / 429: retry.
    Retry(String),
    /// Retrying won't help (404, 403, local disk error).
    Fatal(String),
}

fn classify(e: ureq::Error) -> Failure {
    match e {
        ureq::Error::StatusCode(c) if c == 429 || c >= 500 => Failure::Retry(format!("HTTP {c}")),
        ureq::Error::StatusCode(c) => Failure::Fatal(format!("HTTP {c}")),
        ureq::Error::Io(io) => Failure::Transport(io.to_string()),
        other => {
            let s = other.to_string();
            Failure::Transport(s)
        }
    }
}

fn backoff(attempt: u32) -> Duration {
    Duration::from_millis(1000u64 << attempt.min(5)).min(Duration::from_secs(30))
}

fn part_path(dest: &Path) -> PathBuf {
    let mut s = dest.as_os_str().to_owned();
    s.push(".part");
    PathBuf::from(s)
}

fn file_name(p: &Path) -> String {
    p.file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn curl_exe() -> &'static str {
    if cfg!(windows) {
        "curl.exe"
    } else {
        "curl"
    }
}

fn curl_base() -> Command {
    let mut c = Command::new(curl_exe());
    c.args([
        "--location",
        "--fail",
        "--silent",
        "--show-error",
        "--user-agent",
        USER_AGENT,
    ]);
    c.args([
        "--connect-timeout",
        "20",
        "--retry",
        "2",
        "--retry-delay",
        "2",
    ]);
    c
}

fn curl_get(url: &str) -> Result<Vec<u8>, Failure> {
    let out = curl_base()
        .arg(url)
        .output()
        .map_err(|e| Failure::Fatal(format!("cannot run curl: {e}")))?;
    if out.status.success() {
        Ok(out.stdout)
    } else {
        Err(Failure::Retry(format!(
            "curl: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )))
    }
}

fn curl_download(url: &str, part: &Path, progress: &AtomicU64) -> Result<(), Failure> {
    let start = fs::metadata(part).map(|m| m.len()).unwrap_or(0);
    let mut child = curl_base()
        .args(["--continue-at", "-", "--output"])
        .arg(part)
        .arg(url)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| Failure::Fatal(format!("cannot run curl: {e}")))?;
    let mut reported = start;
    loop {
        let done = child
            .try_wait()
            .map_err(|e| Failure::Fatal(e.to_string()))?;
        let now = fs::metadata(part).map(|m| m.len()).unwrap_or(0);
        if now > reported {
            progress.fetch_add(now - reported, Ordering::Relaxed);
            reported = now;
        }
        if let Some(status) = done {
            if status.success() {
                return Ok(());
            }
            let mut err = String::new();
            if let Some(mut s) = child.stderr.take() {
                let _ = s.read_to_string(&mut err);
            }
            return Err(Failure::Retry(format!(
                "curl exit {status}: {}",
                err.trim()
            )));
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

/// Hex MD5 of a file.
pub fn md5_file(path: &Path) -> Result<String, AssetError> {
    let mut f = File::open(path).map_err(|e| AssetError::io(path, e))?;
    let mut h = Md5::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf).map_err(|e| AssetError::io(path, e))?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(h.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

/// Extract `members` of a downloaded zip into `asset_dir`.
pub fn extract_zip(
    zip_path: &Path,
    asset_dir: &Path,
    members: &[ArchiveMember],
) -> Result<(), AssetError> {
    let f = File::open(zip_path).map_err(|e| AssetError::io(zip_path, e))?;
    let mut z = zip::ZipArchive::new(f)
        .map_err(|e| fetch_err(format!("{}: bad zip: {e}", zip_path.display())))?;
    for m in members {
        let mut entry = z.by_name(&m.name).map_err(|e| {
            fetch_err(format!(
                "{}: member '{}' missing: {e}",
                zip_path.display(),
                m.name
            ))
        })?;
        let dest = asset_dir.join(&m.path);
        if let Some(p) = dest.parent() {
            fs::create_dir_all(p).map_err(|e| AssetError::io(p, e))?;
        }
        let tmp = part_path(&dest);
        let mut out = File::create(&tmp).map_err(|e| AssetError::io(&tmp, e))?;
        std::io::copy(&mut entry, &mut out).map_err(|e| AssetError::io(&tmp, e))?;
        drop(out);
        fs::rename(&tmp, &dest).map_err(|e| AssetError::io(&dest, e))?;
    }
    Ok(())
}

/// What to fetch.
#[derive(Debug, Clone)]
pub struct PlanOptions {
    /// Resolution wish for textures (capped per asset by `max_res`).
    pub texture_res: Res,
    pub model_res: Res,
    pub hdri_res: Res,
    /// Only ids starting with one of these prefixes (empty = all).
    pub only: Vec<String>,
    /// Include assets marked `optional`.
    pub include_optional: bool,
}

/// One file to (maybe) download.
#[derive(Debug, Clone)]
pub struct Job {
    pub asset_id: String,
    pub res: Res,
    pub file: FileEntry,
    pub dest: PathBuf,
    /// Already complete on disk.
    pub done: bool,
}

/// Select assets and files according to `opts`.
pub fn plan(manifest: &Manifest, cache_dir: &Path, opts: &PlanOptions) -> Vec<Job> {
    let mut jobs = Vec::new();
    for a in &manifest.assets {
        let selected_by_prefix =
            opts.only.is_empty() || opts.only.iter().any(|p| a.id.starts_with(p.as_str()));
        if !selected_by_prefix {
            continue;
        }
        // Optional assets need --all, or an --only prefix that names them.
        if a.optional && !opts.include_optional && opts.only.is_empty() {
            continue;
        }
        let wish = match a.kind {
            crate::AssetKind::Texture => opts.texture_res,
            crate::AssetKind::Model => opts.model_res,
            crate::AssetKind::Hdri => opts.hdri_res,
        };
        let Some(res) = a.pick_res(wish) else {
            continue;
        };
        let dir = cache_dir.join(&a.id);
        for f in a.files_at(res) {
            let dest = dir.join(&f.path);
            let done = is_complete(a, f, &dir);
            jobs.push(Job {
                asset_id: a.id.clone(),
                res,
                file: f.clone(),
                dest,
                done,
            });
        }
    }
    jobs
}

/// Whether a file (or all members of an archive) is already present with the right size.
pub fn is_complete(_a: &AssetEntry, f: &FileEntry, asset_dir: &Path) -> bool {
    if f.role == Role::Archive {
        !f.members.is_empty() && f.members.iter().all(|m| asset_dir.join(&m.path).is_file())
    } else {
        fs::metadata(asset_dir.join(&f.path))
            .map(|m| m.len() == f.size)
            .unwrap_or(false)
    }
}

/// Human-readable byte count.
pub fn human_bytes(b: u64) -> String {
    let b = b as f64;
    if b >= 1e9 {
        format!("{:.2} GB", b / 1e9)
    } else if b >= 1e6 {
        format!("{:.1} MB", b / 1e6)
    } else if b >= 1e3 {
        format!("{:.0} kB", b / 1e3)
    } else {
        format!("{b:.0} B")
    }
}
