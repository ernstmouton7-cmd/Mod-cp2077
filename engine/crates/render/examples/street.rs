//! Headless demo: a procedural city street rendered in four lighting/weather
//! scenarios. Uses CC0 textures, HDRIs and models from `engine/assets/cache`
//! (via `gridlock-assets`) when available, procedural fallbacks otherwise.
//!
//! ```text
//! cargo run -p gridlock-render --release --example street -- [WIDTH HEIGHT] [scene...]
//! ```
//! Scenes: noon, golden, rain, night (default: all). Output: `crates/render/screenshots/*.jpg`.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

use glam::{Mat4, Vec2, Vec3, Vec4};
use gridlock_render::procedural::{self as pg, CarStyle};
use gridlock_render::*;

// ---------------------------------------------------------------------------
// Street layout (metres). Street runs along Z, x = 0 is the centre line.

const ROAD_HALF: f32 = 7.0;
const SIDEWALK: f32 = 4.6;
const CURB_H: f32 = 0.15;
const FACADE_X: f32 = ROAD_HALF + SIDEWALK;
const STREET_LEN: f32 = 340.0;

// ---------------------------------------------------------------------------
// Asset adapters

struct Assets {
    store: Option<gridlock_assets::AssetStore>,
    max_tex: u32,
    cache: HashMap<String, Option<PbrSet>>,
}

#[derive(Clone, Copy)]
struct PbrSet {
    albedo: TextureId,
    normal: Option<TextureId>,
    arm: Option<TextureId>,
    emission: Option<TextureId>,
    tile: Vec2,
}

fn to_image(t: &gridlock_assets::TextureData, max: u32) -> ImageData {
    use gridlock_assets::PixelFormat as P;
    let mut first = 0;
    while first + 1 < t.mips.len() && (t.mips[first].width > max || t.mips[first].height > max) {
        first += 1;
    }
    let lvl = &t.mips[first..];
    let (format, srgb, mips): (ImageFormat, bool, Vec<Vec<u8>>) = match t.format {
        P::Rgba8UnormSrgb => (
            ImageFormat::Rgba8,
            true,
            lvl.iter().map(|m| m.data.clone()).collect(),
        ),
        P::Rgba8Unorm => (
            ImageFormat::Rgba8,
            false,
            lvl.iter().map(|m| m.data.clone()).collect(),
        ),
        P::R8Unorm => (
            ImageFormat::R8,
            false,
            lvl.iter().map(|m| m.data.clone()).collect(),
        ),
        P::R16Unorm => (
            ImageFormat::R8,
            false,
            lvl.iter()
                .map(|m| m.data.chunks_exact(2).map(|c| c[1]).collect())
                .collect(),
        ),
        P::Rgba16Float => (
            ImageFormat::Rgba16F,
            false,
            lvl.iter().map(|m| m.data.clone()).collect(),
        ),
        P::Rgba32Float => (
            ImageFormat::Rgba16F,
            false,
            lvl.iter()
                .map(|m| {
                    m.data
                        .chunks_exact(4)
                        .flat_map(|c| {
                            half::f16::from_f32(f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                                .to_le_bytes()
                        })
                        .collect()
                })
                .collect(),
        ),
    };
    ImageData {
        width: lvl[0].width,
        height: lvl[0].height,
        format,
        mips,
        srgb,
    }
}

impl Assets {
    fn new(max_tex: u32) -> Self {
        let store = gridlock_assets::AssetStore::discover().ok();
        if store.is_none() {
            eprintln!("asset store not found; using procedural textures");
        }
        Self {
            store,
            max_tex,
            cache: HashMap::new(),
        }
    }

    fn pbr(&mut self, r: &mut Renderer, id: &str) -> Option<PbrSet> {
        if let Some(c) = self.cache.get(id) {
            return *c;
        }
        let t0 = Instant::now();
        let res = self.store.as_ref().and_then(|s| match s.load_material(id) {
            Ok(m) => Some(m),
            Err(e) => {
                eprintln!("  {id}: {e}");
                None
            }
        });
        let set = res.map(|m| {
            let albedo = r.create_texture(&to_image(&m.base_color, self.max_tex));
            let normal = m
                .normal
                .as_ref()
                .map(|n| r.create_texture(&to_image(n, self.max_tex)));
            let arm = Some(r.create_texture(&to_image(&m.orm, self.max_tex)));
            let emission = m
                .emission
                .as_ref()
                .map(|e| r.create_texture(&to_image(e, self.max_tex)));
            let tile = m
                .tile_size_m
                .map(|t| Vec2::new(t[0], t[1]))
                .unwrap_or(Vec2::splat(2.0));
            PbrSet {
                albedo,
                normal,
                arm,
                emission,
                tile,
            }
        });
        if set.is_some() {
            println!("  loaded {id} in {:.1}s", t0.elapsed().as_secs_f32());
        }
        self.cache.insert(id.to_string(), set);
        set
    }

    fn hdri(&self, id: &str) -> Option<HdrImage> {
        let h = self.store.as_ref()?.load_hdri(id).ok()?;
        let rgb = h
            .data
            .chunks_exact(4)
            .flat_map(|c| [c[0], c[1], c[2]])
            .collect();
        Some(HdrImage {
            width: h.width,
            height: h.height,
            rgb_f32: rgb,
        })
    }

    /// Loads a glTF model as (mesh, material) pairs. `skip` filters variant materials.
    fn model(&mut self, r: &mut Renderer, id: &str, skip: &[&str]) -> Vec<(MeshId, MaterialId)> {
        let Some(store) = self.store.as_ref() else {
            return Vec::new();
        };
        let Ok(model) = store.load_model(id) else {
            eprintln!("  {id}: not available");
            return Vec::new();
        };
        use gridlock_assets::{Material as GM, TextureSlot};
        let mut mats: HashMap<usize, MaterialId> = HashMap::new();
        let mut out = Vec::new();
        let mut tex_cache: HashMap<(usize, u8), TextureId> = HashMap::new();
        for mesh in &model.meshes {
            let mi = mesh.material.unwrap_or(0);
            let gm = model.materials.get(mi).cloned().unwrap_or_default();
            if skip.iter().any(|s| gm.name.contains(s)) {
                continue;
            }
            let mat = *mats.entry(mi).or_insert_with(|| {
                let mut load = |slot: TextureSlot, tag: u8| -> Option<TextureId> {
                    let tr = gm.texture(slot)?;
                    if let Some(t) = tex_cache.get(&(tr.image, tag)) {
                        return Some(*t);
                    }
                    let td = model.load_texture(tr, GM::texture_usage(slot)).ok()?;
                    let id = r.create_texture(&to_image(&td, 1024));
                    tex_cache.insert((tr.image, tag), id);
                    Some(id)
                };
                let albedo = load(TextureSlot::BaseColor, 0);
                let normal = load(TextureSlot::Normal, 1);
                let mr = load(TextureSlot::MetallicRoughness, 2);
                let ao = load(TextureSlot::Occlusion, 3);
                let emis = load(TextureSlot::Emissive, 4);
                let bc = gm.base_color_factor;
                let blend = gm.alpha_mode == gridlock_assets::AlphaMode::Blend;
                r.create_material(&Material {
                    base_color: Vec4::new(bc[0], bc[1], bc[2], bc[3]),
                    metallic: gm.metallic_factor,
                    roughness: gm.roughness_factor,
                    albedo_tex: albedo,
                    normal_tex: normal,
                    roughness_tex: mr,
                    metallic_tex: None,
                    packed_metal_rough: mr.is_some(),
                    ao_tex: ao,
                    emissive_tex: emis,
                    emissive: Vec3::from(gm.emissive_factor)
                        * gm.emissive_strength
                        * if emis.is_some() { 30.0 } else { 0.0 },
                    normal_scale: gm.normal_scale,
                    alpha_mode: match gm.alpha_mode {
                        gridlock_assets::AlphaMode::Opaque => AlphaMode::Opaque,
                        gridlock_assets::AlphaMode::Mask => AlphaMode::Mask(gm.alpha_cutoff),
                        gridlock_assets::AlphaMode::Blend => AlphaMode::Blend,
                    },
                    transmission: if blend { 0.9 } else { 0.0 },
                    double_sided: gm.double_sided,
                    ..Default::default()
                })
            });
            let md = MeshData {
                positions: mesh.positions.iter().map(|p| Vec3::from(*p)).collect(),
                normals: mesh.normals.iter().map(|p| Vec3::from(*p)).collect(),
                tangents: mesh.tangents.iter().map(|p| Vec4::from(*p)).collect(),
                uvs: mesh.uv0.iter().map(|p| Vec2::from(*p)).collect(),
                uvs2: mesh
                    .uv1
                    .as_ref()
                    .map(|v| v.iter().map(|p| Vec2::from(*p)).collect()),
                colors: mesh
                    .colors
                    .as_ref()
                    .map(|v| v.iter().map(|p| Vec4::from(*p)).collect()),
                indices: mesh.indices.clone(),
            };
            out.push((r.create_mesh(&md), mat));
        }
        out
    }
}

// ---------------------------------------------------------------------------
// Mesh building helpers

#[derive(Default)]
struct Builder {
    m: MeshData,
}

impl Builder {
    /// Quad with explicit outward normal; `uv` gives metre UVs for the 4 corners.
    fn quad(&mut self, p: [Vec3; 4], n: Vec3, uv: [Vec2; 4], color: Vec4) {
        let base = self.m.positions.len() as u32;
        for k in 0..4 {
            self.m.positions.push(p[k]);
            self.m.normals.push(n);
            self.m.uvs.push(uv[k]);
            self.m.colors.get_or_insert_with(Vec::new).push(color);
        }
        let face = (p[1] - p[0]).cross(p[2] - p[0]);
        if face.dot(n) >= 0.0 {
            self.m
                .indices
                .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        } else {
            self.m
                .indices
                .extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
        }
    }

    /// Axis aligned box; UVs in metres per face.
    fn cuboid(&mut self, lo: Vec3, hi: Vec3, color: Vec4) {
        let b = pg::cuboid(lo, hi);
        let base = self.m.positions.len() as u32;
        let n = b.positions.len();
        self.m.positions.extend(b.positions);
        self.m.normals.extend(b.normals);
        self.m.uvs.extend(b.uvs);
        self.m
            .colors
            .get_or_insert_with(Vec::new)
            .extend(std::iter::repeat_n(color, n));
        self.m.indices.extend(b.indices.iter().map(|i| i + base));
    }

    fn finish(mut self) -> MeshData {
        self.m.tangents.clear();
        self.m.compute_tangents();
        self.m
    }
}

struct Rng(u64);
impl Rng {
    fn f(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 40) as f32 / (1u64 << 24) as f32
    }
    fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.f()
    }
}

// ---------------------------------------------------------------------------
// Buildings

struct BuildingMeshes {
    wall: MeshData,
    glass: MeshData,
    shop_glass: MeshData,
    frame: MeshData,
}

/// Facade along Z at x = side * FACADE_X facing the street, from z0 to z1.
fn building(
    side: f32,
    z0: f32,
    z1: f32,
    floors: u32,
    rng: &mut Rng,
    lit_chance: f32,
) -> BuildingMeshes {
    let mut wall = Builder::default();
    let mut glass = Builder::default();
    let mut shop = Builder::default();
    let mut frame = Builder::default();
    let n = Vec3::new(-side, 0.0, 0.0); // facade normal (towards street)
    let depth = 14.0;
    let ground_h = 4.4;
    let floor_h = 3.3;
    let height = ground_h + floor_h * floors as f32;
    let width = z1 - z0;
    // Facade point: u along z, v up, w into the building.
    let fp = |u: f32, v: f32, w: f32| Vec3::new(side * (FACADE_X + w), v, z0 + u);
    let white = Vec4::ONE;
    let bays = ((width / 3.2).floor() as u32).max(1);
    let bay_w = width / bays as f32;
    let win_w = (bay_w * rng.range(0.42, 0.62)).min(2.2);
    let win_h = rng.range(1.5, 2.0);
    let sill = rng.range(0.8, 1.0);
    let rec = 0.2;

    // Facade wall quad pieces (with holes for windows).
    let mut wquad = |b: &mut Builder, u0: f32, u1: f32, v0: f32, v1: f32| {
        b.quad(
            [
                fp(u0, v0, 0.0),
                fp(u1, v0, 0.0),
                fp(u1, v1, 0.0),
                fp(u0, v1, 0.0),
            ],
            n,
            [
                Vec2::new(u0, -v0),
                Vec2::new(u1, -v0),
                Vec2::new(u1, -v1),
                Vec2::new(u0, -v1),
            ],
            white,
        );
    };
    // Ground floor: fascia above shopfronts, piers between them.
    let shop_top = ground_h - 0.9;
    wquad(&mut wall, 0.0, width, shop_top, ground_h);
    let pier = 0.45;
    for b in 0..bays {
        let u0 = b as f32 * bay_w;
        wquad(&mut wall, u0, u0 + pier * 0.5, CURB_H, shop_top);
        wquad(
            &mut wall,
            u0 + bay_w - pier * 0.5,
            u0 + bay_w,
            CURB_H,
            shop_top,
        );
        // Shop glass recessed by 0.12 with reveals and a base kick plate.
        let (a, c) = (u0 + pier * 0.5, u0 + bay_w - pier * 0.5);
        let gb = CURB_H + 0.45;
        wquad(&mut wall, a, c, CURB_H, gb);
        let lit = if rng.f() < 0.75 {
            Vec4::new(1.0, 0.85, 0.65, 1.0) * rng.range(0.6, 1.4)
        } else {
            Vec4::new(0.0, 0.0, 0.0, 1.0)
        };
        shop.quad(
            [
                fp(a, gb, 0.12),
                fp(c, gb, 0.12),
                fp(c, shop_top, 0.12),
                fp(a, shop_top, 0.12),
            ],
            n,
            [
                Vec2::new(a, gb),
                Vec2::new(c, gb),
                Vec2::new(c, shop_top),
                Vec2::new(a, shop_top),
            ],
            lit.with_w(1.0),
        );
        frame.cuboid(
            fp(a, gb, 0.0).min(fp(c, gb + 0.06, 0.12)),
            fp(a, gb, 0.0).max(fp(c, gb + 0.06, 0.12)),
            white,
        );
        frame.cuboid(
            fp(a, shop_top - 0.06, 0.0).min(fp(c, shop_top, 0.12)),
            fp(a, shop_top - 0.06, 0.0).max(fp(c, shop_top, 0.12)),
            white,
        );
        let mid = (a + c) * 0.5;
        frame.cuboid(
            fp(mid - 0.03, gb, 0.05).min(fp(mid + 0.03, shop_top, 0.12)),
            fp(mid - 0.03, gb, 0.05).max(fp(mid + 0.03, shop_top, 0.12)),
            white,
        );
        for (uu, sgn) in [(a, 1.0), (c, -1.0)] {
            let nn = Vec3::new(0.0, 0.0, sgn);
            wall.quad(
                [
                    fp(uu, CURB_H, 0.0),
                    fp(uu, CURB_H, 0.12),
                    fp(uu, shop_top, 0.12),
                    fp(uu, shop_top, 0.0),
                ],
                nn,
                [
                    Vec2::new(0.0, 0.0),
                    Vec2::new(0.12, 0.0),
                    Vec2::new(0.12, -shop_top),
                    Vec2::new(0.0, -shop_top),
                ],
                white,
            );
        }
        wall.quad(
            [
                fp(a, shop_top, 0.0),
                fp(c, shop_top, 0.0),
                fp(c, shop_top, 0.12),
                fp(a, shop_top, 0.12),
            ],
            Vec3::NEG_Y,
            [
                Vec2::new(a, 0.0),
                Vec2::new(c, 0.0),
                Vec2::new(c, 0.12),
                Vec2::new(a, 0.12),
            ],
            white,
        );
    }
    // Upper floors.
    for f in 0..floors {
        let fv = ground_h + f as f32 * floor_h;
        let wv0 = fv + sill;
        let wv1 = wv0 + win_h;
        wquad(&mut wall, 0.0, width, fv, wv0);
        wquad(&mut wall, 0.0, width, wv1, fv + floor_h);
        for b in 0..bays {
            let uc = (b as f32 + 0.5) * bay_w;
            let (a, c) = (uc - win_w * 0.5, uc + win_w * 0.5);
            let u0 = b as f32 * bay_w;
            wquad(&mut wall, u0, a, wv0, wv1);
            wquad(&mut wall, c, u0 + bay_w, wv0, wv1);
            // Reveals.
            for (uu, sgn) in [(a, 1.0), (c, -1.0)] {
                wall.quad(
                    [
                        fp(uu, wv0, 0.0),
                        fp(uu, wv0, rec),
                        fp(uu, wv1, rec),
                        fp(uu, wv1, 0.0),
                    ],
                    Vec3::new(0.0, 0.0, sgn),
                    [
                        Vec2::new(0.0, -wv0),
                        Vec2::new(rec, -wv0),
                        Vec2::new(rec, -wv1),
                        Vec2::new(0.0, -wv1),
                    ],
                    white,
                );
            }
            wall.quad(
                [
                    fp(a, wv1, 0.0),
                    fp(c, wv1, 0.0),
                    fp(c, wv1, rec),
                    fp(a, wv1, rec),
                ],
                Vec3::NEG_Y,
                [
                    Vec2::new(a, 0.0),
                    Vec2::new(c, 0.0),
                    Vec2::new(c, rec),
                    Vec2::new(a, rec),
                ],
                white,
            );
            // Sill (protruding stone ledge).
            frame.cuboid(
                fp(a - 0.05, wv0 - 0.06, -0.05).min(fp(c + 0.05, wv0, rec)),
                fp(a - 0.05, wv0 - 0.06, -0.05).max(fp(c + 0.05, wv0, rec)),
                Vec4::new(0.7, 0.7, 0.7, 1.0),
            );
            // Glass with per-window interior light colour (vertex colour drives emission).
            let lit = rng.f() < lit_chance;
            let tint = if lit {
                let warm = rng.f();
                Vec4::new(1.0, 0.78 + 0.15 * warm, 0.5 + 0.4 * warm, 1.0) * rng.range(0.3, 1.2)
            } else {
                Vec4::new(0.0, 0.0, 0.0, 1.0)
            };
            glass.quad(
                [
                    fp(a, wv0, rec),
                    fp(c, wv0, rec),
                    fp(c, wv1, rec),
                    fp(a, wv1, rec),
                ],
                n,
                [
                    Vec2::new(0.0, 0.0),
                    Vec2::new(1.0, 0.0),
                    Vec2::new(1.0, 1.0),
                    Vec2::new(0.0, 1.0),
                ],
                tint.with_w(1.0),
            );
            // Window frame: mullion + transom.
            let mid = (a + c) * 0.5;
            frame.cuboid(
                fp(mid - 0.025, wv0, rec - 0.04).min(fp(mid + 0.025, wv1, rec)),
                fp(mid - 0.025, wv0, rec - 0.04).max(fp(mid + 0.025, wv1, rec)),
                white,
            );
            let tv = wv1 - 0.45;
            frame.cuboid(
                fp(a, tv - 0.025, rec - 0.04).min(fp(c, tv + 0.025, rec)),
                fp(a, tv - 0.025, rec - 0.04).max(fp(c, tv + 0.025, rec)),
                white,
            );
        }
        // Floor slab line.
        frame.cuboid(
            fp(0.0, fv - 0.1, -0.04).min(fp(width, fv + 0.08, 0.0)),
            fp(0.0, fv - 0.1, -0.04).max(fp(width, fv + 0.08, 0.0)),
            Vec4::new(0.8, 0.8, 0.8, 1.0),
        );
    }
    // Cornice and parapet.
    frame.cuboid(
        fp(-0.05, height, -0.35).min(fp(width + 0.05, height + 0.35, 0.0)),
        fp(-0.05, height, -0.35).max(fp(width + 0.05, height + 0.35, 0.0)),
        Vec4::new(0.85, 0.85, 0.85, 1.0),
    );
    wquad(&mut wall, 0.0, width, height + 0.35, height + 1.0);
    // Ends and roof.
    for (uu, sgn) in [(0.0, -1.0), (width, 1.0)] {
        wall.quad(
            [
                fp(uu, CURB_H, 0.0),
                fp(uu, CURB_H, depth),
                fp(uu, height + 1.0, depth),
                fp(uu, height + 1.0, 0.0),
            ],
            Vec3::new(0.0, 0.0, sgn),
            [
                Vec2::new(0.0, -CURB_H),
                Vec2::new(depth, -CURB_H),
                Vec2::new(depth, -height - 1.0),
                Vec2::new(0.0, -height - 1.0),
            ],
            white,
        );
    }
    wall.quad(
        [
            fp(0.0, height + 1.0, 0.0),
            fp(width, height + 1.0, 0.0),
            fp(width, height + 1.0, depth),
            fp(0.0, height + 1.0, depth),
        ],
        Vec3::Y,
        [
            Vec2::new(0.0, 0.0),
            Vec2::new(width, 0.0),
            Vec2::new(width, depth),
            Vec2::new(0.0, depth),
        ],
        white,
    );
    BuildingMeshes {
        wall: wall.finish(),
        glass: glass.finish(),
        shop_glass: shop.finish(),
        frame: frame.finish(),
    }
}

// ---------------------------------------------------------------------------
// Street furniture

/// Modern LED street lamp: returns (pole/arm mesh, emissive head mesh, light position, light dir).
fn street_lamp(side: f32) -> (MeshData, MeshData, Vec3, Vec3) {
    let mut pole = pg::cylinder(0.09, 0.0, 8.2, 20);
    pole.append(&pg::cylinder(0.16, 0.0, 0.6, 20), Mat4::IDENTITY);
    // Arm reaching over the road (towards -side*X).
    let arm_len = 2.4;
    pole.append(
        &pg::cuboid(Vec3::new(-arm_len, 8.05, -0.05), Vec3::new(0.0, 8.15, 0.05)),
        Mat4::IDENTITY,
    );
    pole.append(
        &pg::cuboid(
            Vec3::new(-arm_len - 0.5, 7.98, -0.17),
            Vec3::new(-arm_len + 0.25, 8.12, 0.17),
        ),
        Mat4::IDENTITY,
    );
    let mut head = pg::cuboid(
        Vec3::new(-arm_len - 0.45, 7.96, -0.13),
        Vec3::new(-arm_len + 0.2, 7.985, 0.13),
    );
    head.compute_tangents();
    let flip = Mat4::from_rotation_y(if side > 0.0 {
        0.0
    } else {
        std::f32::consts::PI
    });
    let mut p2 = MeshData::default();
    p2.append(&pole, flip);
    p2.compute_tangents();
    let mut h2 = MeshData::default();
    h2.append(&head, flip);
    h2.compute_tangents();
    let lp = flip.transform_point3(Vec3::new(-arm_len - 0.12, 7.9, 0.0));
    (p2, h2, lp, Vec3::new(-side * 0.25, -1.0, 0.0).normalize())
}

fn traffic_signal() -> (MeshData, MeshData, [Vec3; 3]) {
    let mut pole = pg::cylinder(0.1, 0.0, 5.6, 16);
    pole.append(
        &pg::cuboid(Vec3::new(-4.5, 5.45, -0.06), Vec3::new(0.0, 5.57, 0.06)),
        Mat4::IDENTITY,
    );
    pole.append(
        &pg::cuboid(Vec3::new(-3.6, 4.35, -0.17), Vec3::new(-3.2, 5.45, 0.12)),
        Mat4::IDENTITY,
    );
    pole.append(
        &pg::cuboid(Vec3::new(-0.2, 2.4, -0.17), Vec3::new(0.2, 3.4, 0.12)),
        Mat4::IDENTITY,
    );
    pole.compute_tangents();
    let mut lenses = MeshData::default();
    let mut pos = [Vec3::ZERO; 3];
    for (i, y) in [5.2f32, 4.9, 4.6].iter().enumerate() {
        lenses.append(
            &pg::sphere(0.11, 12, 6),
            Mat4::from_translation(Vec3::new(-3.4, *y, 0.13))
                * Mat4::from_scale(Vec3::new(1.0, 1.0, 0.35)),
        );
        pos[i] = Vec3::new(-3.4, *y, 0.25);
    }
    lenses.compute_tangents();
    (pole, lenses, pos)
}

// ---------------------------------------------------------------------------
// Scene

struct CarInst {
    pos: Vec3,
    yaw: f32,
    headlights: InstanceId,
    taillights: InstanceId,
}

struct Scene {
    window_glass: Vec<InstanceId>,
    shop_glass: Vec<InstanceId>,
    lamp_heads: Vec<InstanceId>,
    lamp_lights: Vec<(Vec3, Vec3)>,
    signal: (InstanceId, [Vec3; 3]),
    cars: Vec<CarInst>,
    office_facades: Vec<InstanceId>,
}

fn mat_from(set: Option<PbrSet>, fallback: &Material, scale_override: Option<Vec2>) -> Material {
    match set {
        Some(s) => Material {
            albedo_tex: Some(s.albedo),
            normal_tex: s.normal,
            roughness_tex: s.arm,
            ao_tex: s.arm,
            packed_metal_rough: true,
            metallic: 1.0,
            roughness: 1.0,
            base_color: Vec4::ONE,
            uv_scale: Vec2::ONE / scale_override.unwrap_or(s.tile),
            ..fallback.clone()
        },
        None => fallback.clone(),
    }
}

fn build_scene(r: &mut Renderer, a: &mut Assets) -> Scene {
    let t0 = Instant::now();
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);

    // --- Road, sidewalks, curbs ---
    let road_mat = {
        let set = a.pbr(r, "tex.asphalt");
        let fb = {
            let p = pg::asphalt(512);
            Material {
                albedo_tex: Some(r.create_texture(&p.albedo)),
                normal_tex: Some(r.create_texture(&p.normal)),
                roughness_tex: Some(r.create_texture(&p.arm)),
                packed_metal_rough: true,
                uv_scale: Vec2::splat(0.25),
                ..Default::default()
            }
        };
        let mut m = mat_from(set, &fb, None);
        m.uv_mode = UvMode::WorldXZ;
        m.metallic = 0.0;
        m.base_color = Vec4::new(0.85, 0.85, 0.85, 1.0);
        r.create_material(&m)
    };
    let mut road = pg::plane(ROAD_HALF * 2.0, STREET_LEN);
    road.compute_tangents();
    let road_mesh = r.create_mesh(&road);
    r.add_instance(road_mesh, road_mat, Mat4::IDENTITY);
    // Cross street at the intersection (z = -60).
    let cross = r.create_mesh(&pg::plane(200.0, 14.0));
    r.add_instance(
        cross,
        road_mat,
        Mat4::from_translation(Vec3::new(0.0, -0.002, -60.0)),
    );

    let walk_mat = {
        let set = a.pbr(r, "tex.concrete_sidewalk");
        let p = pg::concrete_slabs(512);
        let fb = Material {
            albedo_tex: Some(r.create_texture(&p.albedo)),
            normal_tex: Some(r.create_texture(&p.normal)),
            roughness_tex: Some(r.create_texture(&p.arm)),
            packed_metal_rough: true,
            uv_scale: Vec2::splat(1.0 / 3.0),
            ..Default::default()
        };
        let mut m = mat_from(set, &fb, None);
        m.uv_mode = UvMode::WorldXZ;
        m.metallic = 0.0;
        r.create_material(&m)
    };
    let curb_mat = {
        let set = a.pbr(r, "tex.curb");
        let mut m = mat_from(set, &Material::solid(Vec3::splat(0.45), 0.8, 0.0), None);
        m.uv_mode = UvMode::Triplanar;
        m.uv_scale = Vec2::splat(1.0 / set.map(|s| s.tile.x).unwrap_or(2.0));
        m.metallic = 0.0;
        m.base_color = Vec4::new(0.95, 0.95, 0.95, 1.0);
        r.create_material(&m)
    };
    for side in [-1.0f32, 1.0] {
        // Sidewalk slabs split by the cross street.
        for (za, zb) in [(-STREET_LEN * 0.5, -67.0), (-53.0, STREET_LEN * 0.5)] {
            let len = zb - za;
            let zc = (za + zb) * 0.5;
            let x0 = side * (ROAD_HALF + 0.25);
            let x1 = side * (FACADE_X + 0.5);
            let walk = r.create_mesh(&pg::cuboid(
                Vec3::new(x0.min(x1), 0.0, za),
                Vec3::new(x0.max(x1), CURB_H, zb),
            ));
            r.add_instance(walk, walk_mat, Mat4::IDENTITY);
            let curb = r.create_mesh(&pg::cuboid(
                Vec3::new(-0.125, 0.0, -len * 0.5),
                Vec3::new(0.125, CURB_H + 0.005, len * 0.5),
            ));
            r.add_instance(
                curb,
                curb_mat,
                Mat4::from_translation(Vec3::new(side * (ROAD_HALF + 0.125), 0.0, zc)),
            );
        }
    }

    // --- Road markings (projected decals) ---
    let white = Vec4::new(0.78, 0.78, 0.76, 1.0);
    let yellow = Vec4::new(0.75, 0.52, 0.08, 1.0);
    let segments = [
        (-STREET_LEN * 0.5, -70.0),
        (-50.0, 18.0),
        (26.0, STREET_LEN * 0.5),
    ];
    for (za, zb) in segments {
        let len = zb - za;
        let zc = (za + zb) * 0.5;
        for x in [-0.12f32, 0.12] {
            r.add_decal(&Decal::road(
                Vec3::new(x, 0.0, zc),
                Vec3::Z,
                0.11,
                len,
                DecalKind::Paint,
                yellow,
            ));
        }
        for x in [-3.5f32, 3.5] {
            let mut d = Decal::road(
                Vec3::new(x, 0.0, zc),
                Vec3::Z,
                0.12,
                len,
                DecalKind::Dashed {
                    dash: 3.0,
                    gap: 9.0,
                },
                white,
            );
            d.wear = 0.5;
            r.add_decal(&d);
        }
        for x in [-6.6f32, 6.6] {
            r.add_decal(&Decal::road(
                Vec3::new(x, 0.0, zc),
                Vec3::Z,
                0.12,
                len,
                DecalKind::Paint,
                white,
            ));
        }
    }
    // Crosswalk and stop lines at z = 22.
    for i in 0..12 {
        let x = -6.05 + i as f32 * 1.1;
        let mut d = Decal::road(
            Vec3::new(x, 0.0, 22.0),
            Vec3::Z,
            0.55,
            3.2,
            DecalKind::Paint,
            white,
        );
        d.wear = 0.45;
        r.add_decal(&d);
    }
    r.add_decal(&Decal::road(
        Vec3::new(3.5, 0.0, 25.2),
        Vec3::Z,
        7.0,
        0.45,
        DecalKind::Paint,
        white,
    ));
    r.add_decal(&Decal::road(
        Vec3::new(-3.5, 0.0, 18.8),
        Vec3::Z,
        7.0,
        0.45,
        DecalKind::Paint,
        white,
    ));
    // Patches, oil stains and skid marks for realism.
    for _ in 0..40 {
        let x = rng.range(-6.0, 6.0);
        let z = rng.range(-150.0, 120.0);
        let mut d = Decal::road(
            Vec3::new(x, 0.0, z),
            Vec3::new(rng.range(-0.2, 0.2), 0.0, 1.0),
            rng.range(0.8, 2.5),
            rng.range(1.0, 3.0),
            DecalKind::Stain,
            Vec4::new(0.02, 0.02, 0.02, 0.55),
        );
        d.roughness = 0.5;
        r.add_decal(&d);
    }
    for k in 0..3 {
        let x = 1.0 + k as f32 * 0.4;
        let mut d = Decal::road(
            Vec3::new(x, 0.0, -10.0 - k as f32 * 3.0),
            Vec3::new(0.08, 0.0, 1.0),
            0.24,
            14.0,
            DecalKind::SkidMark,
            Vec4::new(0.01, 0.01, 0.01, 0.8),
        );
        d.roughness = 0.6;
        r.add_decal(&d);
    }

    // --- Buildings ---
    let facade_ids = [
        "tex.brick_facade",
        "tex.plaster",
        "tex.concrete_panels",
        "tex.facade_tiles",
        "tex.brick_industrial",
        "tex.siding",
    ];
    let tints = [
        Vec3::new(1.0, 1.0, 1.0),
        Vec3::new(0.95, 0.88, 0.78),
        Vec3::new(0.9, 0.9, 0.92),
        Vec3::new(1.0, 1.0, 1.0),
        Vec3::new(1.0, 0.97, 0.95),
        Vec3::new(0.85, 0.9, 0.95),
    ];
    let mut wall_mats = Vec::new();
    for (i, id) in facade_ids.iter().enumerate() {
        let set = a.pbr(r, id);
        let fb = {
            let p = pg::brick(256, Vec3::new(0.5, 0.2, 0.12));
            Material {
                albedo_tex: Some(r.create_texture(&p.albedo)),
                normal_tex: Some(r.create_texture(&p.normal)),
                roughness_tex: Some(r.create_texture(&p.arm)),
                packed_metal_rough: true,
                uv_scale: Vec2::splat(0.5),
                ..Default::default()
            }
        };
        let mut m = mat_from(set, &fb, None);
        m.metallic = 0.0;
        m.base_color = tints[i].extend(1.0);
        m.uv_scale.y = -m.uv_scale.y.abs();
        wall_mats.push(r.create_material(&m));
    }
    let window_mat = r.create_material(&Material {
        base_color: Vec4::new(0.03, 0.035, 0.04, 1.0),
        roughness: 0.04,
        reflectance: 0.6,
        emissive: Vec3::new(60.0, 50.0, 40.0),
        wet_factor: 0.3,
        ..Default::default()
    });
    let shop_mat = r.create_material(&Material {
        base_color: Vec4::new(0.03, 0.035, 0.04, 1.0),
        roughness: 0.03,
        reflectance: 0.6,
        emissive: Vec3::new(180.0, 160.0, 140.0),
        wet_factor: 0.2,
        ..Default::default()
    });
    let frame_mats = [
        r.create_material(&Material::solid(Vec3::new(0.06, 0.06, 0.065), 0.35, 0.9)),
        r.create_material(&Material::solid(Vec3::new(0.7, 0.68, 0.64), 0.6, 0.0)),
        r.create_material(&Material::solid(Vec3::new(0.12, 0.2, 0.16), 0.4, 0.0)),
    ];
    let office = a.pbr(r, "tex.facade_office");
    let office_mat = office.map(|s| {
        r.create_material(&Material {
            albedo_tex: Some(s.albedo),
            normal_tex: s.normal,
            roughness_tex: s.arm,
            ao_tex: s.arm,
            emissive_tex: s.emission,
            packed_metal_rough: true,
            metallic: 1.0,
            roughness: 1.0,
            emissive: Vec3::splat(if s.emission.is_some() { 45.0 } else { 0.0 }),
            uv_scale: Vec2::new(1.0 / s.tile.x, -1.0 / s.tile.y),
            ..Default::default()
        })
    });
    let mut scene = Scene {
        window_glass: Vec::new(),
        shop_glass: Vec::new(),
        lamp_heads: Vec::new(),
        lamp_lights: Vec::new(),
        signal: (InstanceId(0), [Vec3::ZERO; 3]),
        cars: Vec::new(),
        office_facades: Vec::new(),
    };
    for side in [-1.0f32, 1.0] {
        let mut z = -STREET_LEN * 0.5 + 5.0;
        let mut k = 0;
        while z < STREET_LEN * 0.5 - 10.0 {
            let w = rng.range(12.0, 24.0);
            let z1 = (z + w).min(STREET_LEN * 0.5);
            if z1 > -67.0 && z < -53.0 {
                z = -52.0;
                continue;
            }
            let floors = (rng.range(3.0, 9.0)) as u32;
            let use_office = office_mat.is_some() && k % 7 == 3;
            if use_office {
                // Office tower with a full-facade texture (windows baked, emission map for night).
                let height = 4.4 + 3.3 * (floors + 4) as f32;
                let mut b = Builder::default();
                let fp = |u: f32, v: f32, wd: f32| Vec3::new(side * (FACADE_X + wd), v, z + u);
                let n = Vec3::new(-side, 0.0, 0.0);
                let ww = z1 - z;
                b.quad(
                    [
                        fp(0.0, CURB_H, 0.0),
                        fp(ww, CURB_H, 0.0),
                        fp(ww, height, 0.0),
                        fp(0.0, height, 0.0),
                    ],
                    n,
                    [
                        Vec2::new(0.0, -CURB_H),
                        Vec2::new(ww, -CURB_H),
                        Vec2::new(ww, -height),
                        Vec2::new(0.0, -height),
                    ],
                    Vec4::ONE,
                );
                for (uu, sgn) in [(0.0, -1.0), (ww, 1.0)] {
                    b.quad(
                        [
                            fp(uu, CURB_H, 0.0),
                            fp(uu, CURB_H, 14.0),
                            fp(uu, height, 14.0),
                            fp(uu, height, 0.0),
                        ],
                        Vec3::new(0.0, 0.0, sgn),
                        [
                            Vec2::new(0.0, 0.0),
                            Vec2::new(14.0, 0.0),
                            Vec2::new(14.0, -height),
                            Vec2::new(0.0, -height),
                        ],
                        Vec4::ONE,
                    );
                }
                let m = r.create_mesh(&b.finish());
                let id = r.add_instance(m, office_mat.unwrap(), Mat4::IDENTITY);
                scene.office_facades.push(id);
            } else {
                let lit = 0.45;
                let bm = building(side, z, z1, floors, &mut rng, lit);
                let wm = wall_mats[(k * 5 + if side > 0.0 { 2 } else { 0 }) % wall_mats.len()];
                let wmesh = r.create_mesh(&bm.wall);
                r.add_instance(wmesh, wm, Mat4::IDENTITY);
                let g = r.create_mesh(&bm.glass);
                scene
                    .window_glass
                    .push(r.add_instance(g, window_mat, Mat4::IDENTITY));
                let sg = r.create_mesh(&bm.shop_glass);
                scene
                    .shop_glass
                    .push(r.add_instance(sg, shop_mat, Mat4::IDENTITY));
                let f = r.create_mesh(&bm.frame);
                r.add_instance(f, frame_mats[k % 3], Mat4::IDENTITY);
            }
            z = z1;
            k += 1;
        }
    }
    // Distant backdrop blocks at the street ends and along the cross street.
    let backdrop = r.create_mesh(&pg::cuboid(
        Vec3::new(-30.0, 0.0, -10.0),
        Vec3::new(30.0, 40.0, 10.0),
    ));
    r.add_instance(
        backdrop,
        wall_mats[2],
        Mat4::from_translation(Vec3::new(0.0, 0.0, -STREET_LEN * 0.5 - 20.0)),
    );
    r.add_instance(
        backdrop,
        wall_mats[0],
        Mat4::from_translation(Vec3::new(0.0, 0.0, STREET_LEN * 0.5 + 20.0)),
    );

    // --- Street lamps ---
    let pole_mat = r.create_material(&Material::solid(Vec3::new(0.18, 0.19, 0.2), 0.45, 0.8));
    let lamp_mat = r.create_material(&Material {
        base_color: Vec4::new(0.9, 0.9, 0.9, 1.0),
        roughness: 0.3,
        emissive: Vec3::new(1.0, 0.92, 0.82) * 40000.0,
        ..Default::default()
    });
    let (pole_l, head_l, lp_l, dir_l) = street_lamp(-1.0);
    let (pole_r, head_r, lp_r, dir_r) = street_lamp(1.0);
    let pm = [r.create_mesh(&pole_l), r.create_mesh(&pole_r)];
    let hm = [r.create_mesh(&head_l), r.create_mesh(&head_r)];
    let mut z: f32 = -150.0;
    while z < 150.0 {
        for (i, side) in [-1.0f32, 1.0].iter().enumerate() {
            let zz = z + if *side > 0.0 { 13.0 } else { 0.0 };
            if (zz + 60.0).abs() < 9.0 {
                continue;
            }
            let base = Vec3::new(side * (ROAD_HALF + 0.6), CURB_H, zz);
            let xf = Mat4::from_translation(base);
            r.add_instance(pm[i], pole_mat, xf);
            scene.lamp_heads.push(r.add_instance(hm[i], lamp_mat, xf));
            let (lp, ld) = if i == 0 { (lp_l, dir_l) } else { (lp_r, dir_r) };
            scene.lamp_lights.push((base + lp, ld));
        }
        z += 26.0;
    }

    // --- Traffic signal at the crosswalk ---
    let (sp, sl, spos) = traffic_signal();
    let sig_pole = r.create_mesh(&sp);
    let sig_lens = r.create_mesh(&sl);
    let sxf = Mat4::from_translation(Vec3::new(ROAD_HALF + 0.5, CURB_H, 24.5))
        * Mat4::from_rotation_y(0.0);
    r.add_instance(sig_pole, pole_mat, sxf);
    let red_mat = r.create_material(&Material {
        base_color: Vec4::new(0.3, 0.02, 0.02, 1.0),
        roughness: 0.2,
        emissive: Vec3::new(1.0, 0.05, 0.02) * 6000.0,
        ..Default::default()
    });
    let sig = r.add_instance(sig_lens, red_mat, sxf);
    scene.signal = (sig, spos.map(|p| sxf.transform_point3(p)));

    // --- Trees ---
    let bark = {
        let set = a.pbr(r, "tex.dirt");
        let mut m = mat_from(
            set,
            &Material::solid(Vec3::new(0.12, 0.09, 0.07), 0.9, 0.0),
            None,
        );
        m.uv_mode = UvMode::Triplanar;
        m.uv_scale = Vec2::splat(1.5);
        m.base_color = Vec4::new(0.45, 0.38, 0.33, 1.0);
        m.metallic = 0.0;
        r.create_material(&m)
    };
    let (leaf_alb, leaf_nrm) = pg::leaf_atlas(512, 7, Vec3::new(0.09, 0.16, 0.04));
    let (la, ln) = (r.create_texture(&leaf_alb), r.create_texture(&leaf_nrm));
    let leaf_mat = r.create_material(&Material {
        albedo_tex: Some(la),
        normal_tex: Some(ln),
        roughness: 0.55,
        translucency: 0.7,
        alpha_mode: AlphaMode::Mask(0.5),
        double_sided: true,
        wet_factor: 0.5,
        ..Default::default()
    });
    for side in [-1.0f32, 1.0] {
        let mut z: f32 = -140.0 + if side > 0.0 { 6.0 } else { 0.0 };
        while z < 140.0 {
            if (z + 60.0).abs() > 10.0 && (z - 22.0).abs() > 6.0 {
                let (t, l) = pg::tree((z * 7.0 + side * 100.0) as i32 as u32, rng.range(5.0, 6.5));
                let tm = r.create_mesh(&t);
                let lm = r.create_mesh(&l);
                let xf = Mat4::from_translation(Vec3::new(side * (ROAD_HALF + 2.2), CURB_H, z));
                r.add_instance(tm, bark, xf);
                r.add_instance(lm, leaf_mat, xf);
            }
            z += 13.0;
        }
    }

    // --- Props from glTF models ---
    let hydrant = a.model(r, "model.fire_hydrant", &["aged"]);
    let can = a.model(r, "model.trash_can", &["rust"]);
    let bench = a.model(r, "model.bench", &[]);
    let ubox = a.model(r, "model.utility_box", &[]);
    let place = |r: &mut Renderer, parts: &[(MeshId, MaterialId)], pos: Vec3, yaw: f32| {
        for (m, mat) in parts {
            r.add_instance(
                *m,
                *mat,
                Mat4::from_translation(pos) * Mat4::from_rotation_y(yaw),
            );
        }
    };
    for side in [-1.0f32, 1.0] {
        let yaw = if side > 0.0 {
            -std::f32::consts::FRAC_PI_2
        } else {
            std::f32::consts::FRAC_PI_2
        };
        place(
            r,
            &hydrant,
            Vec3::new(side * (ROAD_HALF + 0.8), CURB_H, 8.0 * side + 4.0),
            yaw,
        );
        place(
            r,
            &can,
            Vec3::new(side * (ROAD_HALF + 0.8), CURB_H, 34.0 - side * 5.0),
            yaw,
        );
        place(
            r,
            &bench,
            Vec3::new(side * (FACADE_X - 1.2), CURB_H, -20.0 + side * 6.0),
            -yaw,
        );
        place(
            r,
            &ubox,
            Vec3::new(side * (FACADE_X - 0.6), CURB_H, 12.0 - side * 9.0),
            -yaw,
        );
    }

    // --- Cars ---
    let paints = [
        (Vec3::new(0.55, 0.015, 0.02), true),
        (Vec3::new(0.02, 0.025, 0.03), true),
        (Vec3::new(0.6, 0.6, 0.62), true),
        (Vec3::new(0.05, 0.12, 0.35), true),
        (Vec3::new(0.85, 0.85, 0.82), false),
        (Vec3::new(0.18, 0.2, 0.22), true),
    ];
    let glass_mat = r.create_material(&Material::glass(Vec3::new(0.05, 0.06, 0.06), 0.15));
    let interior_mat =
        r.create_material(&Material::solid(Vec3::new(0.025, 0.025, 0.028), 0.7, 0.0));
    let rubber = {
        let set = a.pbr(r, "tex.rubber");
        let mut m = mat_from(
            set,
            &Material::solid(Vec3::splat(0.03), 0.85, 0.0),
            Some(Vec2::splat(0.3)),
        );
        m.base_color = Vec4::new(0.35, 0.35, 0.35, 1.0);
        m.metallic = 0.0;
        r.create_material(&m)
    };
    let rim_mat = r.create_material(&Material::solid(Vec3::new(0.75, 0.75, 0.77), 0.25, 1.0));
    let trim_mat = r.create_material(&Material::solid(Vec3::new(0.02, 0.02, 0.02), 0.45, 0.0));
    let head_mat = r.create_material(&Material {
        base_color: Vec4::new(0.8, 0.8, 0.85, 1.0),
        roughness: 0.05,
        metallic: 0.5,
        emissive: Vec3::new(1.0, 0.97, 0.92) * 60000.0,
        ..Default::default()
    });
    let tail_mat = r.create_material(&Material {
        base_color: Vec4::new(0.4, 0.01, 0.01, 1.0),
        roughness: 0.1,
        emissive: Vec3::new(1.0, 0.02, 0.01) * 1500.0,
        ..Default::default()
    });
    let car_slots: [(Vec3, f32, CarStyle); 6] = [
        (
            Vec3::new(1.75, 0.0, 12.0),
            std::f32::consts::PI,
            CarStyle::default(),
        ),
        (
            Vec3::new(5.25, 0.0, -6.0),
            std::f32::consts::PI,
            CarStyle {
                length: 4.3,
                height: 1.5,
                ..CarStyle::default()
            },
        ),
        (
            Vec3::new(-1.75, 0.0, -14.0),
            0.0,
            CarStyle {
                length: 4.9,
                width: 1.9,
                height: 1.42,
                ..CarStyle::default()
            },
        ),
        (
            Vec3::new(-5.25, 0.0, 2.0),
            0.0,
            CarStyle {
                length: 4.2,
                height: 1.55,
                cabin_end: 0.86,
                ..CarStyle::default()
            },
        ),
        (
            Vec3::new(1.75, 0.0, -38.0),
            std::f32::consts::PI,
            CarStyle {
                height: 1.6,
                length: 4.7,
                cabin_end: 0.9,
                ..CarStyle::default()
            },
        ),
        (Vec3::new(-1.75, 0.0, -48.0), 0.0, CarStyle::default()),
    ];
    for (i, (pos, yaw, style)) in car_slots.iter().enumerate() {
        let c = pg::car(*style);
        let (col, flakes) = paints[i % paints.len()];
        let paint = r.create_material(&Material::car_paint(col, flakes));
        let xf = Mat4::from_translation(*pos) * Mat4::from_rotation_y(*yaw);
        for (mesh, mat) in [
            (&c.body, paint),
            (&c.glass, glass_mat),
            (&c.interior, interior_mat),
            (&c.trim, trim_mat),
        ] {
            let m = r.create_mesh(mesh);
            r.add_instance(m, mat, xf);
        }
        let tm = r.create_mesh(&c.tyre);
        let rm = r.create_mesh(&c.rim);
        for w in c.wheels {
            let flip = if w.x < 0.0 {
                Mat4::from_rotation_y(std::f32::consts::PI)
            } else {
                Mat4::IDENTITY
            };
            let wx = xf * Mat4::from_translation(w) * flip;
            r.add_instance(tm, rubber, wx);
            r.add_instance(rm, rim_mat, wx);
        }
        let hm = r.create_mesh(&c.headlights);
        let tlm = r.create_mesh(&c.taillights);
        let headlights = r.add_instance(hm, head_mat, xf);
        let taillights = r.add_instance(tlm, tail_mat, xf);
        scene.cars.push(CarInst {
            pos: *pos,
            yaw: *yaw,
            headlights,
            taillights,
        });
    }
    println!(
        "scene built in {:.1}s: {} instances",
        t0.elapsed().as_secs_f32(),
        r.instance_count()
    );
    scene
}

// ---------------------------------------------------------------------------
// Scenarios

struct Scenario {
    name: &'static str,
    env: Environment,
    night: bool,
    hdri: Option<(&'static str, f32, bool)>,
    camera: Camera,
}

fn scenarios() -> Vec<Scenario> {
    let cam = Camera::look_at(
        Vec3::new(4.6, 1.55, 38.0),
        Vec3::new(-1.5, 3.2, -20.0),
        58.0,
    );
    let mut noon = Environment {
        sun_direction: Environment::sun_from_angles(58.0, 118.0),
        cloud_cover: 0.35,
        turbidity: 2.0,
        fog: Fog {
            density: 0.0006,
            height_falloff: 0.03,
            ..Default::default()
        },
        ..Default::default()
    };
    noon.exposure_compensation = 0.0;
    let golden = Environment {
        sun_direction: Environment::sun_from_angles(7.0, 196.0),
        cloud_cover: 0.3,
        turbidity: 1.6,
        fog: Fog {
            density: 0.0012,
            height_falloff: 0.02,
            anisotropy: 0.7,
            ..Default::default()
        },
        exposure_compensation: 0.3,
        ..Default::default()
    };
    let rain = Environment {
        sun_direction: Environment::sun_from_angles(32.0, 150.0),
        cloud_cover: 1.0,
        turbidity: 2.5,
        wetness: 1.0,
        puddles: 0.6,
        rain: 0.8,
        fog: Fog {
            density: 0.006,
            height_falloff: 0.015,
            anisotropy: 0.3,
            volumetric_scale: 1.0,
            ..Default::default()
        },
        exposure_compensation: 0.4,
        ..Default::default()
    };
    let night = Environment {
        sun_direction: Environment::sun_from_angles(-28.0, 250.0),
        moon_direction: Environment::sun_from_angles(35.0, 60.0),
        moon_illuminance: 0.2,
        cloud_cover: 0.65,
        wetness: 1.0,
        puddles: 0.5,
        rain: 0.35,
        night_glow: Vec3::new(0.05, 0.03, 0.018),
        fog: Fog {
            density: 0.003,
            height_falloff: 0.02,
            anisotropy: 0.5,
            volumetric_scale: 1.0,
            ..Default::default()
        },
        exposure_compensation: -0.3,
        ..Default::default()
    };
    vec![
        Scenario {
            name: "noon",
            env: noon,
            night: false,
            hdri: None,
            camera: cam,
        },
        Scenario {
            name: "golden",
            env: golden,
            night: false,
            hdri: None,
            camera: cam,
        },
        Scenario {
            name: "rain",
            env: rain,
            night: false,
            hdri: Some(("hdri.overcast", 1.0, true)),
            camera: cam,
        },
        Scenario {
            name: "night",
            env: night,
            night: true,
            hdri: None,
            camera: cam,
        },
    ]
}

fn apply_scenario(r: &mut Renderer, scene: &Scene, s: &Scenario, hdri: Option<HdriId>) {
    let mut env = s.env;
    env.time = 12.0;
    if let (Some(id), Some((_, blend, bg))) = (hdri, s.hdri) {
        env.hdri = Some(id);
        env.hdri_blend = blend;
        env.hdri_background = bg;
    }
    let night = s.night;
    let dusk = env.sun_direction.y < 0.15;
    r.set_environment(&env);
    let on = if night {
        1.0
    } else if dusk {
        0.25
    } else {
        0.0
    };
    for &g in &scene.window_glass {
        r.set_instance_params(g, Vec4::ONE, on, true);
    }
    for &g in &scene.shop_glass {
        r.set_instance_params(g, Vec4::ONE, if night { 1.0 } else { 0.15 }, true);
    }
    for &g in &scene.office_facades {
        r.set_instance_params(g, Vec4::ONE, on, true);
    }
    for &h in &scene.lamp_heads {
        r.set_instance_params(h, Vec4::ONE, if night || dusk { 1.0 } else { 0.0 }, true);
    }
    let rainy = env.rain > 0.0;
    let cars_lit = night || rainy;
    for c in &scene.cars {
        r.set_instance_params(
            c.headlights,
            Vec4::ONE,
            if cars_lit { 1.0 } else { 0.0 },
            true,
        );
        r.set_instance_params(
            c.taillights,
            Vec4::ONE,
            if cars_lit { 1.0 } else { 0.05 },
            true,
        );
    }
    let mut lights = Vec::new();
    if night || dusk {
        for (p, d) in &scene.lamp_lights {
            let mut l = Light::spot(*p, *d, kelvin_to_rgb(4000.0), 6000.0, 32.0, 0.75, 1.25);
            l.source_radius = 0.15;
            lights.push(l);
        }
    }
    // Traffic signal: red lit.
    for (i, p) in scene.signal.1.iter().enumerate() {
        if i == 0 {
            lights.push(Light {
                volumetric: 0.5,
                ..Light::point(*p, Vec3::new(1.0, 0.05, 0.02), 40.0, 6.0)
            });
        }
    }
    if cars_lit {
        for (ci, c) in scene.cars.iter().enumerate() {
            let rot = Mat4::from_rotation_y(c.yaw);
            let fwd = rot.transform_vector3(Vec3::Z);
            let right = rot.transform_vector3(Vec3::X);
            for s in [-0.6f32, 0.6] {
                let p = c.pos + fwd * 2.2 + right * s + Vec3::Y * 0.68;
                let dir = (fwd - Vec3::Y * 0.06).normalize();
                let mut l = Light::spot(p, dir, kelvin_to_rgb(5500.0), 22000.0, 60.0, 0.18, 0.42);
                l.cast_shadows = ci == 0;
                l.source_radius = 0.06;
                lights.push(l);
                let tp = c.pos - fwd * 2.3 + right * s + Vec3::Y * 0.75;
                lights.push(Light {
                    volumetric: 0.3,
                    ..Light::point(tp, Vec3::new(1.0, 0.03, 0.02), 25.0, 5.0)
                });
            }
        }
    }
    r.set_lights(&lights);

    // Particles: exhaust from cars; road spray in the rain.
    let mut parts = Vec::new();
    let mut rng = Rng(77);
    for c in &scene.cars {
        let rot = Mat4::from_rotation_y(c.yaw);
        let fwd = rot.transform_vector3(Vec3::Z);
        let right = rot.transform_vector3(Vec3::X);
        let exhaust = c.pos - fwd * 2.35 + right * 0.5 + Vec3::Y * 0.3;
        for k in 0..6 {
            let t = k as f32 / 6.0;
            parts.push(Particle {
                position: exhaust - fwd * (t * 1.5)
                    + Vec3::Y * (t * 0.5)
                    + right * rng.range(-0.1, 0.1),
                size: 0.15 + t * 0.6,
                color: Vec4::new(0.6, 0.6, 0.62, 0.22 * (1.0 - t)),
                rotation: rng.range(0.0, 6.0),
                ..Default::default()
            });
        }
        if rainy {
            for _ in 0..24 {
                let p = c.pos - fwd * rng.range(1.0, 3.5)
                    + right * rng.range(-1.1, 1.1)
                    + Vec3::Y * rng.range(0.05, 0.6);
                parts.push(Particle {
                    position: p,
                    size: rng.range(0.15, 0.4),
                    color: Vec4::new(0.7, 0.72, 0.75, 0.25),
                    kind: ParticleKind::Splash,
                    velocity: -fwd * 3.0 + Vec3::Y * 1.5,
                    ..Default::default()
                });
            }
        }
    }
    r.set_particles(&parts);
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut w = 960;
    let mut h = 540;
    let mut names: Vec<String> = Vec::new();
    let mut nums = args.iter().filter_map(|a| a.parse::<u32>().ok());
    if let (Some(a), Some(b)) = (nums.next(), nums.next()) {
        w = a;
        h = b;
    }
    for a in &args {
        if a.parse::<u32>().is_err() {
            names.push(a.clone());
        }
    }
    let mut settings = RenderSettings::software();
    if std::env::var("GRIDLOCK_ULTRA").is_ok() {
        settings = RenderSettings::preset(Quality::Ultra);
    }
    if let Ok(v) = std::env::var("GRIDLOCK_DEBUG") {
        settings.debug_view = match v.as_str() {
            "albedo" => DebugView::Albedo,
            "normals" => DebugView::Normals,
            "roughness" => DebugView::Roughness,
            "ao" => DebugView::AmbientOcclusion,
            "sky" => DebugView::SkyVisibility,
            "diffuse" => DebugView::DiffuseLighting,
            "specular" => DebugView::SpecularLighting,
            "ssr" => DebugView::Ssr,
            _ => DebugView::Metallic,
        };
    }
    if let Ok(f) = std::env::var("GRIDLOCK_WARMUP") {
        settings.capture_warmup_frames = f.parse().unwrap_or(8);
    }
    let t0 = Instant::now();
    let mut r = Renderer::new_headless(w, h, settings)?;
    println!(
        "adapter: {:?} ({:?}) init {:.1}s",
        r.adapter_info().name,
        r.adapter_info().backend,
        t0.elapsed().as_secs_f32()
    );
    let mut assets = Assets::new(1024);
    let scene = build_scene(&mut r, &mut assets);
    let out_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("screenshots");
    std::fs::create_dir_all(&out_dir)?;
    let mut hdris: HashMap<&str, HdriId> = HashMap::new();
    for s in scenarios() {
        if !names.is_empty() && !names.iter().any(|n| n == s.name) {
            continue;
        }
        let mut hid = None;
        if let Some((id, _, _)) = s.hdri {
            if let Some(h) = hdris.get(id) {
                hid = Some(*h);
            } else if let Some(img) = assets.hdri(id) {
                let img = normalize_hdri(img, 2500.0);
                let h = r.set_hdri(&img);
                hdris.insert(id, h);
                hid = Some(h);
            }
        }
        apply_scenario(&mut r, &scene, &s, hid);
        r.reset_history();
        let t = Instant::now();
        let suffix = std::env::var("GRIDLOCK_DEBUG")
            .map(|d| format!("_{d}"))
            .unwrap_or_default();
        let path = out_dir.join(format!("{}{suffix}.jpg", s.name));
        let mut cam = s.camera;
        cam.cut = true;
        r.capture_png(&cam, &path)?;
        let st = r.stats();
        println!(
            "{}: {:.1}s ({} frames) -> {} | gpu {:.1} ms {:?} | draws {} (shadow {}) tris {} lights {}",
            s.name,
            t.elapsed().as_secs_f32(),
            r.settings().capture_warmup_frames,
            path.display(),
            st.gpu_total_ms,
            st.gpu_pass_ms.iter().map(|(n, ms)| format!("{n}:{ms:.0}")).collect::<Vec<_>>(),
            st.draw_calls,
            st.shadow_draw_calls,
            st.triangles,
            st.lights
        );
    }
    Ok(())
}

/// Scales an HDRI so its upper hemisphere averages `target_nits`, and clips the sun
/// (the renderer adds the sun analytically).
fn normalize_hdri(mut img: HdrImage, target_nits: f32) -> HdrImage {
    let (w, h) = (img.width as usize, img.height as usize);
    let mut sum = 0.0f64;
    let mut wsum = 0.0f64;
    for y in 0..h / 2 {
        let theta = (y as f32 + 0.5) / h as f32 * std::f32::consts::PI;
        let sw = theta.sin() as f64;
        for x in 0..w {
            let i = (y * w + x) * 3;
            let l =
                0.2126 * img.rgb_f32[i] + 0.7152 * img.rgb_f32[i + 1] + 0.0722 * img.rgb_f32[i + 2];
            sum += l.min(1e4) as f64 * sw;
            wsum += sw;
        }
    }
    let avg = (sum / wsum.max(1e-9)) as f32;
    let k = target_nits / avg.max(1e-6);
    let clip = avg * 30.0;
    for px in img.rgb_f32.chunks_exact_mut(3) {
        let l = 0.2126 * px[0] + 0.7152 * px[1] + 0.0722 * px[2];
        let s = if l > clip { clip / l } else { 1.0 };
        for c in px.iter_mut() {
            *c *= s * k;
        }
    }
    img
}
