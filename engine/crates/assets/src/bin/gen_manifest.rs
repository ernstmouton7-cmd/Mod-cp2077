//! Generate `engine/assets/manifest.toml` and `LICENSES.md` from
//! `engine/assets/sources.toml` by querying the Poly Haven and ambientCG APIs.
//!
//! ```text
//! cargo run -p gridlock-assets --bin gen-manifest --release [-- --assets-dir <dir>] [--curl]
//! ```
//!
//! Each asset's licence is verified on its own page (Poly Haven: the JSON-LD
//! `license` field must be the CC0 1.0 deed; ambientCG: the page must state
//! the CC0 licence). Assets that fail are dropped with a warning.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use gridlock_assets::fetch::{human_bytes, Backend, Client};
use gridlock_assets::manifest::{
    ArchiveKind, ArchiveMember, AssetEntry, AssetKind, Defaults, FileEntry, Manifest,
    NormalConvention, Res, Role, Source, MANIFEST_SCHEMA,
};
use gridlock_assets::{AssetError, AssetStore};
use serde::Deserialize;
use serde_json::Value;

const RESOLUTIONS: [Res; 3] = [Res::R1k, Res::R2k, Res::R4k];

#[derive(Debug, Deserialize)]
struct Sources {
    #[serde(rename = "asset")]
    assets: Vec<SourceAsset>,
}

#[derive(Debug, Deserialize)]
struct SourceAsset {
    id: String,
    kind: AssetKind,
    source: Source,
    slug: String,
    #[serde(default)]
    maps: Option<Vec<String>>,
    #[serde(default)]
    max_res: Option<Res>,
    #[serde(default)]
    optional: bool,
    #[serde(default)]
    metalness: Option<f32>,
    #[serde(default)]
    notes: Option<String>,
}

fn err(msg: impl Into<String>) -> AssetError {
    AssetError::Fetch(msg.into())
}

fn get_json(client: &Client, url: &str) -> Result<Value, AssetError> {
    let bytes = client.get_bytes(url)?;
    serde_json::from_slice(&bytes).map_err(|e| err(format!("{url}: bad JSON: {e}")))
}

fn role_from_name(s: &str) -> Result<Role, AssetError> {
    Ok(match s {
        "albedo" => Role::Albedo,
        "normal" => Role::Normal,
        "arm" => Role::Arm,
        "roughness" => Role::Roughness,
        "metalness" => Role::Metalness,
        "ao" => Role::Ao,
        "height" => Role::Height,
        "emission" => Role::Emission,
        "opacity" => Role::Opacity,
        other => return Err(err(format!("unknown map '{other}'"))),
    })
}

fn base_name(url: &str) -> String {
    url.rsplit('/')
        .next()
        .unwrap_or(url)
        .split('?')
        .next()
        .unwrap_or("")
        .to_string()
}

fn ph_file(v: &Value) -> Option<(String, u64, Option<String>)> {
    Some((
        v.get("url")?.as_str()?.to_string(),
        v.get("size")?.as_u64()?,
        v.get("md5").and_then(Value::as_str).map(str::to_string),
    ))
}

/// Verify the licence on the asset's public page.
fn verify_license(client: &Client, source: Source, slug: &str) -> Result<String, String> {
    match source {
        Source::Polyhaven => {
            let url = format!("https://polyhaven.com/a/{slug}");
            let html = client.get_bytes(&url).map_err(|e| e.to_string())?;
            let html = String::from_utf8_lossy(&html);
            let ok = html
                .contains("\"license\":\"https://creativecommons.org/publicdomain/zero/1.0/\"")
                || html.contains(
                    "\"license\": \"https://creativecommons.org/publicdomain/zero/1.0/\"",
                );
            if ok {
                Ok(format!("{url}: JSON-LD license = https://creativecommons.org/publicdomain/zero/1.0/ (site policy https://polyhaven.com/license)"))
            } else {
                Err(format!("{url}: CC0 licence marker not found"))
            }
        }
        Source::Ambientcg => {
            let url = format!("https://ambientcg.com/view?id={slug}");
            let html = client.get_bytes(&url).map_err(|e| e.to_string())?;
            let html = String::from_utf8_lossy(&html);
            if html.contains("released under the Creative Commons CC0 license") {
                Ok(format!("{url}: page states 'All assets are released under the Creative Commons CC0 license' (site policy https://docs.ambientcg.com/license/)"))
            } else {
                Err(format!("{url}: CC0 statement not found"))
            }
        }
    }
}

fn polyhaven(client: &Client, s: &SourceAsset) -> Result<AssetEntry, AssetError> {
    let info = get_json(
        client,
        &format!("https://api.polyhaven.com/info/{}", s.slug),
    )?;
    let files = get_json(
        client,
        &format!("https://api.polyhaven.com/files/{}", s.slug),
    )?;
    let name = info["name"].as_str().unwrap_or(&s.slug).to_string();
    let authors: Vec<String> = info["authors"]
        .as_object()
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default();
    let tags: Vec<String> = info["tags"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|t| t.as_str().map(str::to_string))
                .take(8)
                .collect()
        })
        .unwrap_or_default();
    let mut tile_size_m = None;
    let mut out_files = Vec::new();

    match s.kind {
        AssetKind::Texture => {
            if let Some(d) = info["dimensions"].as_array() {
                if let (Some(x), Some(y)) = (
                    d.first().and_then(Value::as_f64),
                    d.get(1).and_then(Value::as_f64),
                ) {
                    tile_size_m = Some([(x / 1000.0) as f32, (y / 1000.0) as f32]);
                }
            }
            let wanted: Vec<String> = s.maps.clone().unwrap_or_else(|| {
                ["albedo", "normal", "arm", "height"]
                    .iter()
                    .map(|m| m.to_string())
                    .collect()
            });
            for map in wanted {
                let role = role_from_name(&map)?;
                // (API key, convention) candidates in order of preference.
                let keys: &[(&str, Option<NormalConvention>)] = match role {
                    Role::Albedo => &[("Diffuse", None), ("diff", None)],
                    Role::Normal => &[
                        ("nor_gl", Some(NormalConvention::Gl)),
                        ("nor_dx", Some(NormalConvention::Dx)),
                    ],
                    Role::Arm => &[("arm", None)],
                    Role::Roughness => &[("Rough", None)],
                    Role::Metalness => &[("Metal", None)],
                    Role::Ao => &[("AO", None)],
                    Role::Height => &[("Displacement", None)],
                    Role::Emission => &[("Emission", None), ("emission", None)],
                    Role::Opacity => &[("Mask", None), ("Alpha", None), ("opacity", None)],
                    _ => &[],
                };
                let Some((key, conv)) = keys.iter().find(|(k, _)| files.get(*k).is_some()) else {
                    return Err(err(format!("{}: map '{map}' not offered", s.slug)));
                };
                for res in RESOLUTIONS {
                    let Some(entry) = files[*key][res.as_str()]["jpg"].as_object() else {
                        continue;
                    };
                    let (url, size, md5) = ph_file(&Value::Object(entry.clone()))
                        .ok_or_else(|| err(format!("{}: malformed file entry", s.slug)))?;
                    out_files.push(FileEntry {
                        res,
                        role,
                        path: format!("{res}/{}", base_name(&url)),
                        url,
                        size,
                        md5,
                        normal: *conv,
                        archive: None,
                        members: vec![],
                    });
                }
            }
        }
        AssetKind::Hdri => {
            for res in RESOLUTIONS {
                let Some(entry) = files["hdri"][res.as_str()]["hdr"].as_object() else {
                    continue;
                };
                let (url, size, md5) = ph_file(&Value::Object(entry.clone()))
                    .ok_or_else(|| err(format!("{}: malformed hdri entry", s.slug)))?;
                out_files.push(FileEntry {
                    res,
                    role: Role::Hdri,
                    path: format!("{res}/{}", base_name(&url)),
                    url,
                    size,
                    md5,
                    normal: None,
                    archive: None,
                    members: vec![],
                });
            }
        }
        AssetKind::Model => {
            for res in RESOLUTIONS {
                let g = &files["gltf"][res.as_str()]["gltf"];
                let Some((url, size, md5)) = ph_file(g) else {
                    continue;
                };
                out_files.push(FileEntry {
                    res,
                    role: Role::Gltf,
                    path: format!("{res}/{}", base_name(&url)),
                    url,
                    size,
                    md5,
                    normal: None,
                    archive: None,
                    members: vec![],
                });
                if let Some(inc) = g["include"].as_object() {
                    let sorted: BTreeMap<_, _> = inc.iter().collect();
                    for (rel, v) in sorted {
                        let (url, size, md5) = ph_file(v)
                            .ok_or_else(|| err(format!("{}: malformed include", s.slug)))?;
                        let nconv = rel.contains("_nor_gl").then_some(NormalConvention::Gl);
                        out_files.push(FileEntry {
                            res,
                            role: Role::GltfDependency,
                            path: format!("{res}/{rel}"),
                            url,
                            size,
                            md5,
                            normal: nconv,
                            archive: None,
                            members: vec![],
                        });
                    }
                }
            }
        }
    }
    if out_files.is_empty() {
        return Err(err(format!("{}: no files found", s.slug)));
    }
    Ok(AssetEntry {
        id: s.id.clone(),
        kind: s.kind,
        source: Source::Polyhaven,
        slug: s.slug.clone(),
        name,
        page: format!("https://polyhaven.com/a/{}", s.slug),
        authors,
        license: "CC0-1.0".into(),
        license_evidence: String::new(),
        optional: s.optional,
        max_res: s.max_res,
        tile_size_m,
        metalness: s.metalness,
        tags,
        notes: s.notes.clone(),
        files: out_files,
    })
}

fn ambientcg(client: &Client, s: &SourceAsset) -> Result<AssetEntry, AssetError> {
    if s.kind != AssetKind::Texture {
        return Err(err("only ambientCG materials are supported"));
    }
    let url = format!(
        "https://ambientcg.com/api/v2/full_json?id={}&include=downloadData,mapData,displayData,dimensionsData,tagData",
        s.slug
    );
    let j = get_json(client, &url)?;
    let a = j["foundAssets"]
        .as_array()
        .and_then(|v| {
            v.iter()
                .find(|a| a["assetId"].as_str() == Some(s.slug.as_str()))
        })
        .ok_or_else(|| err(format!("{}: not found on ambientCG", s.slug)))?;
    let maps: Vec<String> = a["maps"]
        .as_array()
        .map(|m| {
            m.iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    // (API map name, our role, file suffix inside the zip)
    let table: [(&str, Role, &str); 8] = [
        ("color", Role::Albedo, "Color"),
        ("normal", Role::Normal, "NormalGL"),
        ("roughness", Role::Roughness, "Roughness"),
        ("metalness", Role::Metalness, "Metalness"),
        ("ambient-occlusion", Role::Ao, "AmbientOcclusion"),
        ("displacement", Role::Height, "Displacement"),
        ("emission", Role::Emission, "Emission"),
        ("opacity", Role::Opacity, "Opacity"),
    ];
    let wanted: Option<Vec<Role>> = s
        .maps
        .as_ref()
        .map(|m| {
            m.iter()
                .map(|x| role_from_name(x))
                .collect::<Result<_, _>>()
        })
        .transpose()?;
    let downloads = a["downloadFolders"]["default"]["downloadFiletypeCategories"]["zip"]
        ["downloads"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut out_files = Vec::new();
    for res in RESOLUTIONS {
        let attr = format!("{}-JPG", res.as_str().to_uppercase());
        let Some(d) = downloads
            .iter()
            .find(|d| d["attribute"].as_str() == Some(attr.as_str()))
        else {
            continue;
        };
        let url = d["downloadLink"]
            .as_str()
            .ok_or_else(|| err("no downloadLink"))?
            .to_string();
        let size = d["size"].as_u64().ok_or_else(|| err("no size"))?;
        let file_name = d["fileName"]
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| base_name(&url));
        let stem = file_name.trim_end_matches(".zip").to_string();
        let mut members = Vec::new();
        for (api, role, suffix) in table {
            if !maps.iter().any(|m| m == api) {
                continue;
            }
            if let Some(w) = &wanted {
                if !w.contains(&role) {
                    continue;
                }
            }
            let name = format!("{stem}_{suffix}.jpg");
            members.push(ArchiveMember {
                role,
                path: format!("{res}/{name}"),
                name,
                normal: (role == Role::Normal).then_some(NormalConvention::Gl),
            });
        }
        out_files.push(FileEntry {
            res,
            role: Role::Archive,
            path: format!("{res}/{file_name}"),
            url,
            size,
            md5: None,
            normal: None,
            archive: Some(ArchiveKind::Zip),
            members,
        });
    }
    if out_files.is_empty() {
        return Err(err(format!("{}: no JPG zips found", s.slug)));
    }
    let dx = a["dimensionX"].as_f64().unwrap_or(0.0);
    let dy = a["dimensionY"].as_f64().unwrap_or(0.0);
    let tile_size_m = (dx > 0.0 && dy > 0.0).then(|| [(dx / 100.0) as f32, (dy / 100.0) as f32]);
    let tags: Vec<String> = a["tags"]
        .as_array()
        .map(|t| {
            t.iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .take(8)
                .collect()
        })
        .unwrap_or_default();
    Ok(AssetEntry {
        id: s.id.clone(),
        kind: s.kind,
        source: Source::Ambientcg,
        slug: s.slug.clone(),
        name: a["displayName"].as_str().unwrap_or(&s.slug).to_string(),
        page: format!("https://ambientcg.com/view?id={}", s.slug),
        authors: vec!["Lennart Demes (ambientCG)".into()],
        license: "CC0-1.0".into(),
        license_evidence: String::new(),
        optional: s.optional,
        max_res: s.max_res,
        tile_size_m,
        metalness: s.metalness,
        tags,
        notes: s.notes.clone(),
        files: out_files,
    })
}

fn today_utc() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0) as i64;
    // Civil-from-days (H. Hinnant).
    let z = secs.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

fn licenses_md(m: &Manifest) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "# Third-party asset licences\n");
    let _ = writeln!(
        s,
        "Generated by `gen-manifest` from `sources.toml` ({}). Every asset below is released \
         under **Creative Commons CC0 1.0 Universal** (public domain dedication, \
         <https://creativecommons.org/publicdomain/zero/1.0/>): it may be used, modified and \
         redistributed for any purpose, commercial or not, without attribution. Credit is given \
         here anyway as a courtesy.\n",
        m.generated.as_deref().unwrap_or("")
    );
    let _ = writeln!(
        s,
        "Site policies: Poly Haven <https://polyhaven.com/license>, ambientCG \
         <https://docs.ambientcg.com/license/>. Each asset's licence was re-checked on its own \
         page when the manifest was generated (column *Verified*).\n"
    );
    for kind in [AssetKind::Texture, AssetKind::Hdri, AssetKind::Model] {
        let title = match kind {
            AssetKind::Texture => "Materials",
            AssetKind::Hdri => "HDRIs",
            AssetKind::Model => "Models",
        };
        let _ = writeln!(s, "## {title}\n");
        let _ = writeln!(
            s,
            "| id | asset | source | author(s) | licence | verified |"
        );
        let _ = writeln!(s, "|---|---|---|---|---|---|");
        for a in m.assets.iter().filter(|a| a.kind == kind) {
            let _ = writeln!(
                s,
                "| `{}`{} | [{}]({}) | {} | {} | CC0 1.0 | {} |",
                a.id,
                if a.optional { " (optional)" } else { "" },
                a.name,
                a.page,
                a.source,
                a.authors.join(", "),
                if a.license_evidence.is_empty() {
                    "no"
                } else {
                    "yes (asset page)"
                }
            );
        }
        let _ = writeln!(s);
    }
    s
}

fn run() -> Result<(), AssetError> {
    let mut args = std::env::args().skip(1);
    let mut assets_dir: Option<PathBuf> = None;
    let mut backend = Backend::Auto;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--assets-dir" => assets_dir = args.next().map(PathBuf::from),
            "--curl" => backend = Backend::Curl,
            "-h" | "--help" => {
                println!("gen-manifest [--assets-dir <dir>] [--curl]");
                return Ok(());
            }
            other => return Err(err(format!("unknown argument '{other}'"))),
        }
    }
    let dir = assets_dir.unwrap_or_else(AssetStore::source_assets_dir);
    let src_path = dir.join("sources.toml");
    let text = std::fs::read_to_string(&src_path)
        .map_err(|e| err(format!("{}: {e}", src_path.display())))?;
    let sources: Sources = toml::from_str(&text).map_err(|e| err(format!("sources.toml: {e}")))?;
    let client = Client::new(backend);

    let mut assets = Vec::new();
    let mut dropped = Vec::new();
    for s in &sources.assets {
        print!("{:<28} {:>10} {:<40} ", s.id, s.source.to_string(), s.slug);
        let r = match s.source {
            Source::Polyhaven => polyhaven(&client, s),
            Source::Ambientcg => ambientcg(&client, s),
        };
        let mut a = match r {
            Ok(a) => a,
            Err(e) => {
                println!("ERROR: {e}");
                dropped.push(format!("{} ({}): {e}", s.id, s.slug));
                continue;
            }
        };
        match verify_license(&client, s.source, &s.slug) {
            Ok(ev) => a.license_evidence = ev,
            Err(e) => {
                println!("DROPPED, licence not verified: {e}");
                dropped.push(format!("{} ({}): {e}", s.id, s.slug));
                continue;
            }
        }
        let sizes: Vec<String> = a
            .resolutions()
            .iter()
            .map(|r| format!("{r}={}", human_bytes(a.download_size(*r))))
            .collect();
        println!("CC0 ok  {}", sizes.join(" "));
        assets.push(a);
    }

    let manifest = Manifest {
        schema: MANIFEST_SCHEMA,
        generated: Some(format!(
            "gen-manifest {} on {} from sources.toml; do not edit by hand",
            env!("CARGO_PKG_VERSION"),
            today_utc()
        )),
        defaults: Defaults::default(),
        assets,
    };
    manifest.validate()?;
    let header = "# GENERATED by `cargo run -p gridlock-assets --bin gen-manifest --release`\n\
                  # from sources.toml. Edit sources.toml, not this file.\n\
                  # Every file: exact URL, byte size and MD5 (Poly Haven) per resolution.\n\n";
    let body = manifest.to_toml_string()?;
    write(&dir.join("manifest.toml"), &format!("{header}{body}"))?;
    write(&dir.join("LICENSES.md"), &licenses_md(&manifest))?;

    println!();
    for (label, opts) in [
        ("1k (textures/models 1k, HDRIs 1k)", (Res::R1k, Res::R1k)),
        (
            "default (textures/models 2k, HDRIs 4k)",
            (Res::R2k, Res::R4k),
        ),
        ("4k (textures/models 4k, HDRIs 4k)", (Res::R4k, Res::R4k)),
    ] {
        let (mut base, mut all) = (0u64, 0u64);
        for a in &manifest.assets {
            let wish = if a.kind == AssetKind::Hdri {
                opts.1
            } else {
                opts.0
            };
            let sz = a.pick_res(wish).map(|r| a.download_size(r)).unwrap_or(0);
            all += sz;
            if !a.optional {
                base += sz;
            }
        }
        println!(
            "{label:<42} default set {:>10}   with optional {:>10}",
            human_bytes(base),
            human_bytes(all)
        );
    }
    if !dropped.is_empty() {
        println!("\nDropped {} asset(s):", dropped.len());
        for d in &dropped {
            println!("  {d}");
        }
    }
    println!("\nwrote {}", dir.join("manifest.toml").display());
    Ok(())
}

fn write(path: &Path, text: &str) -> Result<(), AssetError> {
    std::fs::write(path, text).map_err(|e| err(format!("{}: {e}", path.display())))
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
