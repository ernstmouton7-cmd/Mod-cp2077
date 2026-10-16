//! Loads every asset of the manifest that is present in the local cache and
//! checks dimensions, mip chains, colour spaces and the normal-map convention.
//!
//! Assets that are not downloaded are skipped with a note, so CI without the
//! cache still passes. Populate the cache with
//! `cargo run -p gridlock-assets --bin fetch-assets --release`, then run
//! `cargo test -p gridlock-assets --release`.

use std::time::Instant;

use gridlock_assets::manifest::{AssetKind, Res, Role};
use gridlock_assets::mips::full_mip_count;
use gridlock_assets::model::{ImageSource, TextureSlot};
use gridlock_assets::{AssetStore, Material, PixelFormat, TextureData};

fn store() -> AssetStore {
    AssetStore::open(AssetStore::source_assets_dir()).expect("manifest.toml must parse")
}

#[test]
fn manifest_is_valid_and_cc0() {
    let s = store();
    let m = s.manifest();
    assert!(!m.assets.is_empty());
    for a in &m.assets {
        assert_eq!(a.license, "CC0-1.0", "{}", a.id);
        assert!(
            !a.license_evidence.is_empty(),
            "{}: licence not verified",
            a.id
        );
        let prefix = match a.kind {
            AssetKind::Texture => "tex.",
            AssetKind::Hdri => "hdri.",
            AssetKind::Model => "model.",
        };
        assert!(
            a.id.starts_with(prefix),
            "{} should start with {prefix}",
            a.id
        );
        for r in [Res::R1k, Res::R2k] {
            assert!(a.files_at(r).next().is_some(), "{} has no {r} files", a.id);
        }
        let def = m.defaults.for_kind(a.kind);
        let res = a.pick_res(def).unwrap();
        let roles: Vec<Role> = a
            .files_at(res)
            .flat_map(|f| {
                if f.members.is_empty() {
                    vec![f.role]
                } else {
                    f.members.iter().map(|m| m.role).collect()
                }
            })
            .collect();
        match a.kind {
            AssetKind::Texture => {
                assert!(roles.contains(&Role::Albedo), "{}", a.id);
                assert!(roles.contains(&Role::Normal), "{}", a.id);
                assert!(
                    roles.contains(&Role::Arm) || roles.contains(&Role::Roughness),
                    "{}",
                    a.id
                );
            }
            AssetKind::Hdri => assert_eq!(roles, vec![Role::Hdri], "{}", a.id),
            AssetKind::Model => assert!(roles.contains(&Role::Gltf), "{}", a.id),
        }
        for f in &a.files {
            assert!(f.size > 0, "{}: {}", a.id, f.path);
            assert!(f.url.starts_with("https://"), "{}: {}", a.id, f.url);
            assert!(f.path.starts_with(f.res.as_str()), "{}: {}", a.id, f.path);
        }
    }
    // Default download set stays reasonable.
    let total: u64 = m
        .assets
        .iter()
        .filter(|a| !a.optional)
        .map(|a| a.download_size(a.pick_res(m.defaults.for_kind(a.kind)).unwrap()))
        .sum();
    println!("default download set: {:.1} MB", total as f64 / 1e6);
    assert!(total < 600_000_000, "default set is {total} bytes");
}

fn check_chain(id: &str, what: &str, t: &TextureData, res: Res) {
    t.validate().unwrap_or_else(|e| panic!("{id} {what}: {e}"));
    let (w, h) = (t.width(), t.height());
    assert_eq!(t.mip_count(), full_mip_count(w, h), "{id} {what}");
    // Square tiles at the nominal size (ambientCG facades may be non-square, but
    // their long side is the nominal size).
    assert_eq!(w.max(h), res.pixels(), "{id} {what}: {w}x{h} at {res}");
}

/// Pearson correlation.
fn corr(a: &[f32], b: &[f32]) -> f32 {
    let n = a.len() as f32;
    let (ma, mb) = (a.iter().sum::<f32>() / n, b.iter().sum::<f32>() / n);
    let (mut sab, mut saa, mut sbb) = (0.0, 0.0, 0.0);
    for (x, y) in a.iter().zip(b) {
        sab += (x - ma) * (y - mb);
        saa += (x - ma) * (x - ma);
        sbb += (y - mb) * (y - mb);
    }
    sab / (saa.sqrt() * sbb.sqrt()).max(1e-12)
}

/// Check that the (converted) normal map is OpenGL convention by comparing it
/// with the gradient of the height map. With rows increasing downwards and
/// +Y = up the image, a GL normal map satisfies `n.x ~ -dh/dcol` and
/// `n.y ~ +dh/drow`. Returns (corr_x, corr_y) or None when not comparable.
fn normal_vs_height(normal: &TextureData, height: &TextureData) -> Option<(f32, f32)> {
    // Compare at ~256 px where both maps are smooth enough.
    let pick = |t: &TextureData| t.mips.iter().position(|m| m.width <= 256);
    let (ln, lh) = (pick(normal)?, pick(height)?);
    let (n, h) = (&normal.mips[ln], &height.mips[lh]);
    if (n.width, n.height) != (h.width, h.height) {
        return None;
    }
    let (w, hh) = (n.width as usize, n.height as usize);
    let bpp = height.format.bytes_per_pixel();
    let hv = |x: usize, y: usize| -> f32 {
        let i = (y % hh * w + x % w) * bpp;
        if bpp == 2 {
            u16::from_ne_bytes([h.data[i], h.data[i + 1]]) as f32 / 65535.0
        } else {
            h.data[i] as f32 / 255.0
        }
    };
    let (mut nx, mut ny, mut gx, mut gy) = (vec![], vec![], vec![], vec![]);
    for y in 0..hh {
        for x in 0..w {
            let i = (y * w + x) * 4;
            nx.push(n.data[i] as f32 / 127.5 - 1.0);
            ny.push(n.data[i + 1] as f32 / 127.5 - 1.0);
            gx.push(hv(x + 1, y) - hv(x + w - 1, y));
            gy.push(hv(x, y + 1) - hv(x, y + hh - 1));
        }
    }
    Some((corr(&nx, &gx), corr(&ny, &gy)))
}

#[test]
fn cached_materials_load() {
    let s = store();
    let mut loaded = 0;
    for id in s.ids(AssetKind::Texture) {
        if !s.is_cached(id) {
            println!("note: {id} not cached, skipped");
            continue;
        }
        let t0 = Instant::now();
        let m = s.load_material(id).unwrap_or_else(|e| panic!("{id}: {e}"));
        let ms = t0.elapsed().as_millis();
        assert_eq!(m.base_color.format, PixelFormat::Rgba8UnormSrgb, "{id}");
        check_chain(id, "albedo", &m.base_color, m.res);
        let n = m
            .normal
            .as_ref()
            .unwrap_or_else(|| panic!("{id}: no normal map"));
        assert_eq!(n.format, PixelFormat::Rgba8Unorm, "{id}");
        check_chain(id, "normal", n, m.res);
        assert_eq!(m.orm.format, PixelFormat::Rgba8Unorm, "{id}");
        check_chain(id, "orm", &m.orm, m.res);
        // Every stored normal is unit length (renormalised per mip) and points out of the surface.
        // (A few texels of strongly bevelled facade maps lean slightly backwards;
        // allow that for < 1% of texels.)
        for (li, lvl) in n.mips.iter().enumerate().step_by(3) {
            let (mut total, mut back) = (0usize, 0usize);
            for px in lvl.data.chunks_exact(4).step_by(97) {
                let v = [
                    px[0] as f32 / 127.5 - 1.0,
                    px[1] as f32 / 127.5 - 1.0,
                    px[2] as f32 / 127.5 - 1.0,
                ];
                let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
                assert!(
                    (len - 1.0).abs() < 0.03,
                    "{id} normal mip {li}: |n| = {len}"
                );
                total += 1;
                back += (v[2] < 0.0) as usize;
            }
            assert!(
                back * 100 <= total,
                "{id} normal mip {li}: {back}/{total} texels face backwards"
            );
        }
        let avg = gridlock_assets::texture::average_normal(n);
        assert!(avg[2] > 0.8, "{id}: average normal {avg:?} not facing +Z");
        assert!(
            m.toksvig_levels > 0,
            "{id}: roughness mips were not widened"
        );
        let mut conv = String::from("-");
        if let Some(h) = &m.height {
            check_chain(id, "height", h, m.res);
            if let Some((cx, cy)) = normal_vs_height(n, h) {
                conv = format!("corr x {cx:+.2} y {cy:+.2}");
                // X does not depend on the convention: it validates that the
                // height map and normal map agree at all. Then Y decides GL vs DX.
                if cx < -0.2 {
                    assert!(
                        cy > 0.1,
                        "{id}: normal map is not OpenGL convention ({conv})"
                    );
                } else {
                    println!("note: {id}: height map does not explain the normal map ({conv}); convention not judged");
                }
            }
        }
        if let Some(e) = &m.emission {
            assert_eq!(e.format, PixelFormat::Rgba8UnormSrgb);
            check_chain(id, "emission", e, m.res);
        }
        println!(
            "ok {id:<28} {} {}x{} mips {:>2} toksvig {:>2} src-normal {:?} height {} [{conv}] {ms} ms",
            m.res,
            m.base_color.width(),
            m.base_color.height(),
            m.base_color.mip_count(),
            m.toksvig_levels,
            m.normal_source_convention.unwrap(),
            if m.height.is_some() { "y" } else { "n" },
        );
        loaded += 1;
    }
    if loaded == 0 {
        println!("note: texture cache is empty; run fetch-assets to test real assets");
    }
}

#[test]
fn cached_hdris_load() {
    let s = store();
    let mut loaded = 0;
    for id in s.ids(AssetKind::Hdri) {
        let Some(res) = s.local_res(id) else {
            println!("note: {id} not cached, skipped");
            continue;
        };
        let t0 = Instant::now();
        let h = s.load_hdri(id).unwrap_or_else(|e| panic!("{id}: {e}"));
        let ms = t0.elapsed().as_millis();
        assert_eq!(h.width, res.pixels(), "{id}");
        assert_eq!(h.width, h.height * 2, "{id}: not 2:1 equirect");
        assert_eq!(h.data.len(), (h.width * h.height * 4) as usize);
        assert!(
            h.data.iter().all(|v| v.is_finite() && *v >= 0.0),
            "{id}: bad texels"
        );
        let max = h.data.iter().cloned().fold(0.0f32, f32::max);
        assert!(max > 1.0, "{id}: not HDR (max {max})");
        let sun = h.estimate_sun();
        if id.starts_with("hdri.day") || id == "hdri.sunset" {
            let s = sun.unwrap_or_else(|| panic!("{id}: no sun found"));
            assert!(
                s.direction[1] > -0.05,
                "{id}: sun below horizon {:?}",
                s.direction
            );
        }
        let half = h.to_rgba16f();
        assert_eq!(half.len(), h.data.len());
        println!(
            "ok {id:<28} {res} {}x{} max {max:>9.1} avg-lum {:>7.3} sun {:?} {ms} ms",
            h.width,
            h.height,
            h.average_luminance(),
            sun.map(|s| s.direction.map(|v| (v * 100.0).round() / 100.0))
        );
        loaded += 1;
    }
    if loaded == 0 {
        println!("note: HDRI cache is empty; run fetch-assets to test real assets");
    }
}

#[test]
fn cached_models_load() {
    let s = store();
    let mut loaded = 0;
    for id in s.ids(AssetKind::Model) {
        if !s.is_cached(id) {
            println!("note: {id} not cached, skipped");
            continue;
        }
        let t0 = Instant::now();
        let m = s.load_model(id).unwrap_or_else(|e| panic!("{id}: {e}"));
        assert!(!m.meshes.is_empty(), "{id}: no meshes");
        assert!(m.triangle_count() > 0, "{id}");
        assert!(!m.bounds.is_empty(), "{id}");
        let mut generated = 0;
        for mesh in &m.meshes {
            let n = mesh.positions.len();
            assert_eq!(mesh.normals.len(), n, "{id} {}", mesh.name);
            assert_eq!(mesh.tangents.len(), n, "{id} {}", mesh.name);
            assert_eq!(mesh.uv0.len(), n, "{id} {}", mesh.name);
            assert_eq!(mesh.indices.len() % 3, 0);
            assert!(mesh.indices.iter().all(|i| (*i as usize) < n));
            for t in mesh.tangents.iter().step_by(31) {
                let l = (t[0] * t[0] + t[1] * t[1] + t[2] * t[2]).sqrt();
                assert!((l - 1.0).abs() < 1e-2, "{id}: tangent length {l}");
                assert!(t[3] == 1.0 || t[3] == -1.0);
            }
            for nv in mesh.normals.iter().step_by(31) {
                let l = (nv[0] * nv[0] + nv[1] * nv[1] + nv[2] * nv[2]).sqrt();
                assert!((l - 1.0).abs() < 1e-2, "{id}: normal length {l}");
            }
            if let Some(mi) = mesh.material {
                assert!(mi < m.materials.len());
            }
            generated += mesh.generated_tangents as usize;
        }
        for img in &m.images {
            if let ImageSource::File(p) = img {
                assert!(p.is_file(), "{id}: missing texture {}", p.display());
            }
        }
        // Decode the first material's base colour and normal map.
        let mut decoded = 0;
        if let Some(mat) = m
            .materials
            .iter()
            .find(|mt| mt.base_color_texture.is_some())
        {
            for slot in [TextureSlot::BaseColor, TextureSlot::Normal] {
                if let Some(r) = mat.texture(slot) {
                    let t = m
                        .load_texture(r, Material::texture_usage(slot))
                        .unwrap_or_else(|e| panic!("{id}: {e}"));
                    t.validate().unwrap();
                    if slot == TextureSlot::BaseColor {
                        assert!(t.format.is_srgb());
                    }
                    decoded += 1;
                }
            }
        }
        let sz = m.bounds.size();
        println!(
            "ok {id:<28} meshes {:>3} tris {:>8} materials {:>2} images {:>2} size {:.2}x{:.2}x{:.2} m gen-tangents {generated} decoded {decoded} {} ms",
            m.meshes.len(),
            m.triangle_count(),
            m.materials.len(),
            m.images.len(),
            sz[0],
            sz[1],
            sz[2],
            t0.elapsed().as_millis()
        );
        loaded += 1;
    }
    if loaded == 0 {
        println!("note: model cache is empty; run fetch-assets to test real assets");
    }
}
