//! Adapters from `gridlock-assets` CPU data to renderer textures, materials and meshes.

use glam::{Vec2, Vec3, Vec4};
use gridlock_render::*;
use std::collections::HashMap;
use std::time::Instant;

pub struct Assets {
    store: Option<gridlock_assets::AssetStore>,
    max_tex: u32,
    cache: HashMap<String, Option<PbrSet>>,
}

#[derive(Clone, Copy)]
pub struct PbrSet {
    pub albedo: TextureId,
    pub normal: Option<TextureId>,
    pub arm: Option<TextureId>,
    pub emission: Option<TextureId>,
    pub tile: Vec2,
}

pub fn to_image(t: &gridlock_assets::TextureData, max: u32) -> ImageData {
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
    pub fn new(max_tex: u32) -> Self {
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

    pub fn pbr(&mut self, r: &mut Renderer, id: &str) -> Option<PbrSet> {
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

    pub fn hdri(&self, id: &str) -> Option<HdrImage> {
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
    /// Returns the (mesh, material) pairs and the model's bounds.
    pub fn model(
        &mut self,
        r: &mut Renderer,
        id: &str,
        skip: &[&str],
    ) -> (Vec<(MeshId, MaterialId)>, Vec3, Vec3) {
        let Some(store) = self.store.as_ref() else {
            return (Vec::new(), Vec3::ZERO, Vec3::ZERO);
        };
        let Ok(model) = store.load_model(id) else {
            eprintln!("  {id}: not available");
            return (Vec::new(), Vec3::ZERO, Vec3::ZERO);
        };
        let mut lo = Vec3::splat(f32::MAX);
        let mut hi = Vec3::splat(f32::MIN);
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
            let (a, b) = md.bounds();
            lo = lo.min(a);
            hi = hi.max(b);
            out.push((r.create_mesh(&md), mat));
        }
        (out, lo, hi)
    }
}
