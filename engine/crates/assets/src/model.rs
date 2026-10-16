//! glTF 2.0 (`.gltf` + `.bin` or `.glb`) loading into flat meshes.
//!
//! * The node hierarchy of the default scene (or the first scene, or all
//!   root nodes if there is none) is flattened: every primitive's vertices are
//!   transformed into model space, normals with the inverse-transpose, and
//!   winding is flipped for mirrored nodes, so the result can be drawn with a
//!   single model matrix.
//! * Missing normals are computed (area-weighted smooth); missing tangents are
//!   generated with MikkTSpace (`bevy_mikktspace`) when UV0 exists, so baked
//!   glTF normal maps (OpenGL convention, as the spec requires) shade correctly.
//! * Materials are glTF metallic-roughness with texture *references*
//!   ([`TextureRef`] -> [`ImageSource`]); decode them with
//!   [`Model::load_texture`] using the usage the slot implies
//!   ([`Material::texture_usage`]).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use glam::{Mat3, Mat4, Vec3, Vec4Swizzles};

use crate::manifest::NormalConvention;
use crate::texture::{self, TextureData, TextureUsage};
use crate::AssetError;

/// Where an image referenced by a material lives.
#[derive(Debug, Clone, PartialEq)]
pub enum ImageSource {
    /// External file (resolved relative to the glTF file).
    File(PathBuf),
    /// Encoded image bytes embedded in a buffer view or data URI.
    Embedded { mime: String, bytes: Arc<Vec<u8>> },
}

/// Texture wrap mode (glTF sampler).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WrapMode {
    #[default]
    Repeat,
    MirroredRepeat,
    ClampToEdge,
}

/// Texture sampler state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SamplerInfo {
    pub wrap_u: WrapMode,
    pub wrap_v: WrapMode,
    /// `false` only when the file explicitly asks for nearest magnification.
    pub mag_linear: bool,
}

impl Default for SamplerInfo {
    fn default() -> Self {
        SamplerInfo {
            wrap_u: WrapMode::Repeat,
            wrap_v: WrapMode::Repeat,
            mag_linear: true,
        }
    }
}

/// `KHR_texture_transform` (applied as `uv' = T * R * S * uv`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UvTransform {
    pub offset: [f32; 2],
    pub rotation: f32,
    pub scale: [f32; 2],
}

/// Reference from a material slot to an image.
#[derive(Debug, Clone, PartialEq)]
pub struct TextureRef {
    /// Index into [`Model::images`].
    pub image: usize,
    pub tex_coord: u32,
    pub sampler: SamplerInfo,
    pub transform: Option<UvTransform>,
}

/// glTF alpha mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AlphaMode {
    #[default]
    Opaque,
    Mask,
    Blend,
}

/// glTF metallic-roughness material.
#[derive(Debug, Clone, PartialEq)]
pub struct Material {
    pub name: String,
    /// Linear RGBA multiplier.
    pub base_color_factor: [f32; 4],
    /// sRGB.
    pub base_color_texture: Option<TextureRef>,
    pub metallic_factor: f32,
    pub roughness_factor: f32,
    /// Linear; G = roughness, B = metalness (R is often AO on Poly Haven, see `occlusion_texture`).
    pub metallic_roughness_texture: Option<TextureRef>,
    /// Linear, OpenGL convention (glTF spec).
    pub normal_texture: Option<TextureRef>,
    pub normal_scale: f32,
    /// Linear, R channel.
    pub occlusion_texture: Option<TextureRef>,
    pub occlusion_strength: f32,
    /// Linear RGB multiplier (times `emissive_strength`).
    pub emissive_factor: [f32; 3],
    pub emissive_strength: f32,
    /// sRGB.
    pub emissive_texture: Option<TextureRef>,
    pub alpha_mode: AlphaMode,
    pub alpha_cutoff: f32,
    pub double_sided: bool,
    /// `KHR_materials_transmission` factor (0 = opaque).
    pub transmission: f32,
    /// `KHR_materials_ior` (1.5 default).
    pub ior: f32,
}

impl Default for Material {
    fn default() -> Self {
        Material {
            name: String::new(),
            base_color_factor: [1.0; 4],
            base_color_texture: None,
            metallic_factor: 1.0,
            roughness_factor: 1.0,
            metallic_roughness_texture: None,
            normal_texture: None,
            normal_scale: 1.0,
            occlusion_texture: None,
            occlusion_strength: 1.0,
            emissive_factor: [0.0; 3],
            emissive_strength: 1.0,
            emissive_texture: None,
            alpha_mode: AlphaMode::Opaque,
            alpha_cutoff: 0.5,
            double_sided: false,
            transmission: 0.0,
            ior: 1.5,
        }
    }
}

/// Material texture slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureSlot {
    BaseColor,
    MetallicRoughness,
    Normal,
    Occlusion,
    Emissive,
}

impl Material {
    /// The decoding a slot requires (colour space, normal convention).
    pub fn texture_usage(slot: TextureSlot) -> TextureUsage {
        match slot {
            TextureSlot::BaseColor | TextureSlot::Emissive => TextureUsage::Color,
            TextureSlot::Normal => TextureUsage::Normal(NormalConvention::Gl),
            TextureSlot::MetallicRoughness | TextureSlot::Occlusion => TextureUsage::Data,
        }
    }

    /// Texture reference of a slot.
    pub fn texture(&self, slot: TextureSlot) -> Option<&TextureRef> {
        match slot {
            TextureSlot::BaseColor => self.base_color_texture.as_ref(),
            TextureSlot::MetallicRoughness => self.metallic_roughness_texture.as_ref(),
            TextureSlot::Normal => self.normal_texture.as_ref(),
            TextureSlot::Occlusion => self.occlusion_texture.as_ref(),
            TextureSlot::Emissive => self.emissive_texture.as_ref(),
        }
    }
}

/// One drawable primitive, already in model space.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Mesh {
    /// `"<node name>/<mesh name>#<primitive index>"`.
    pub name: String,
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    /// xyz = tangent, w = bitangent sign (`B = cross(N, T) * w`), glTF convention.
    pub tangents: Vec<[f32; 4]>,
    pub uv0: Vec<[f32; 2]>,
    pub uv1: Option<Vec<[f32; 2]>>,
    /// Linear RGBA vertex colours, if present.
    pub colors: Option<Vec<[f32; 4]>>,
    /// Triangle list, counter-clockwise front faces.
    pub indices: Vec<u32>,
    /// Index into [`Model::materials`]; `None` = glTF default material.
    pub material: Option<usize>,
    /// True when tangents were generated (not read from the file).
    pub generated_tangents: bool,
}

impl Mesh {
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }
}

/// Axis-aligned bounding box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

impl Aabb {
    pub fn empty() -> Self {
        Aabb {
            min: [f32::INFINITY; 3],
            max: [f32::NEG_INFINITY; 3],
        }
    }

    pub fn is_empty(&self) -> bool {
        self.min[0] > self.max[0]
    }

    pub fn extend(&mut self, p: [f32; 3]) {
        for (i, v) in p.into_iter().enumerate() {
            self.min[i] = self.min[i].min(v);
            self.max[i] = self.max[i].max(v);
        }
    }

    pub fn size(&self) -> [f32; 3] {
        [
            self.max[0] - self.min[0],
            self.max[1] - self.min[1],
            self.max[2] - self.min[2],
        ]
    }
}

/// A loaded glTF model.
#[derive(Debug, Clone, PartialEq)]
pub struct Model {
    pub source: PathBuf,
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
    pub images: Vec<ImageSource>,
    pub bounds: Aabb,
}

impl Model {
    /// Load a `.gltf` or `.glb` file.
    pub fn load(path: &Path) -> Result<Self, AssetError> {
        load_gltf(path)
    }

    pub fn triangle_count(&self) -> usize {
        self.meshes.iter().map(Mesh::triangle_count).sum()
    }

    pub fn vertex_count(&self) -> usize {
        self.meshes.iter().map(|m| m.positions.len()).sum()
    }

    /// Decode a referenced image with the given usage (see [`Material::texture_usage`]).
    pub fn load_texture(
        &self,
        tex: &TextureRef,
        usage: TextureUsage,
    ) -> Result<TextureData, AssetError> {
        match self.images.get(tex.image) {
            Some(ImageSource::File(p)) => texture::load_texture(p, usage),
            Some(ImageSource::Embedded { bytes, .. }) => texture::decode_texture(bytes, usage),
            None => Err(AssetError::Gltf {
                path: self.source.clone(),
                msg: format!("image index {} out of range", tex.image),
            }),
        }
    }
}

fn gltf_err(path: &Path, msg: impl Into<String>) -> AssetError {
    AssetError::Gltf {
        path: path.to_path_buf(),
        msg: msg.into(),
    }
}

/// Load a `.gltf` or `.glb` file (see module docs).
pub fn load_gltf(path: &Path) -> Result<Model, AssetError> {
    let bytes = std::fs::read(path).map_err(|e| AssetError::io(path, e))?;
    let gltf = gltf::Gltf::from_slice(&bytes).map_err(|e| gltf_err(path, e.to_string()))?;
    let base = path.parent().unwrap_or(Path::new("."));
    let doc = gltf.document;
    let buffers = gltf::import_buffers(&doc, Some(base), gltf.blob)
        .map_err(|e| gltf_err(path, format!("buffers: {e}")))?;

    // Images.
    let mut images = Vec::new();
    for img in doc.images() {
        images.push(match img.source() {
            gltf::image::Source::View { view, mime_type } => {
                let buf = &buffers[view.buffer().index()];
                let start = view.offset();
                let end = start + view.length();
                let slice = buf
                    .get(start..end)
                    .ok_or_else(|| gltf_err(path, "image buffer view out of range"))?;
                ImageSource::Embedded {
                    mime: mime_type.to_string(),
                    bytes: Arc::new(slice.to_vec()),
                }
            }
            gltf::image::Source::Uri { uri, mime_type } => {
                if let Some(rest) = uri.strip_prefix("data:") {
                    let (meta, data) = rest
                        .split_once(',')
                        .ok_or_else(|| gltf_err(path, "malformed data URI"))?;
                    if !meta.ends_with(";base64") {
                        return Err(gltf_err(path, "only base64 data URIs are supported"));
                    }
                    let mime = meta.trim_end_matches(";base64").to_string();
                    let bytes = base64_decode(data)
                        .ok_or_else(|| gltf_err(path, "invalid base64 in data URI"))?;
                    ImageSource::Embedded {
                        mime: mime_type.map(str::to_string).unwrap_or(mime),
                        bytes: Arc::new(bytes),
                    }
                } else {
                    ImageSource::File(base.join(percent_decode(uri)))
                }
            }
        });
    }

    let tex_ref = |info: &gltf::texture::Info| -> TextureRef {
        make_ref(info.texture(), info.tex_coord(), info.texture_transform())
    };

    // Materials.
    let mut materials = Vec::new();
    for m in doc.materials() {
        let pbr = m.pbr_metallic_roughness();
        let mut mat = Material {
            name: m.name().unwrap_or("").to_string(),
            base_color_factor: pbr.base_color_factor(),
            base_color_texture: pbr.base_color_texture().map(|i| tex_ref(&i)),
            metallic_factor: pbr.metallic_factor(),
            roughness_factor: pbr.roughness_factor(),
            metallic_roughness_texture: pbr.metallic_roughness_texture().map(|i| tex_ref(&i)),
            emissive_factor: m.emissive_factor(),
            emissive_strength: m.emissive_strength().unwrap_or(1.0),
            emissive_texture: m.emissive_texture().map(|i| tex_ref(&i)),
            alpha_mode: match m.alpha_mode() {
                gltf::material::AlphaMode::Opaque => AlphaMode::Opaque,
                gltf::material::AlphaMode::Mask => AlphaMode::Mask,
                gltf::material::AlphaMode::Blend => AlphaMode::Blend,
            },
            alpha_cutoff: m.alpha_cutoff().unwrap_or(0.5),
            double_sided: m.double_sided(),
            transmission: m.transmission().map_or(0.0, |t| t.transmission_factor()),
            ior: m.ior().unwrap_or(1.5),
            ..Material::default()
        };
        if let Some(n) = m.normal_texture() {
            mat.normal_texture = Some(make_ref(n.texture(), n.tex_coord(), None));
            mat.normal_scale = n.scale();
        }
        if let Some(o) = m.occlusion_texture() {
            mat.occlusion_texture = Some(make_ref(o.texture(), o.tex_coord(), None));
            mat.occlusion_strength = o.strength();
        }
        materials.push(mat);
    }

    // Scene traversal.
    let roots: Vec<gltf::Node> = match doc.default_scene().or_else(|| doc.scenes().next()) {
        Some(s) => s.nodes().collect(),
        None => {
            // No scene: treat nodes that are nobody's child as roots.
            let mut is_child = vec![false; doc.nodes().count()];
            for n in doc.nodes() {
                for c in n.children() {
                    is_child[c.index()] = true;
                }
            }
            doc.nodes().filter(|n| !is_child[n.index()]).collect()
        }
    };
    let mut meshes = Vec::new();
    let mut stack: Vec<(gltf::Node, Mat4)> =
        roots.into_iter().map(|n| (n, Mat4::IDENTITY)).collect();
    while let Some((node, parent)) = stack.pop() {
        let local = Mat4::from_cols_array_2d(&node.transform().matrix());
        let world = parent * local;
        if let Some(mesh) = node.mesh() {
            for (pi, prim) in mesh.primitives().enumerate() {
                let name = format!(
                    "{}/{}#{pi}",
                    node.name().unwrap_or("node"),
                    mesh.name().unwrap_or("mesh")
                );
                if let Some(m) = read_primitive(&prim, &buffers, world, name, path)? {
                    meshes.push(m);
                }
            }
        }
        for c in node.children() {
            stack.push((c, world));
        }
    }

    let mut bounds = Aabb::empty();
    for m in &meshes {
        for p in &m.positions {
            bounds.extend(*p);
        }
    }
    Ok(Model {
        source: path.to_path_buf(),
        meshes,
        materials,
        images,
        bounds,
    })
}

fn make_ref(
    tex: gltf::texture::Texture,
    tex_coord: u32,
    transform: Option<gltf::texture::TextureTransform>,
) -> TextureRef {
    let s = tex.sampler();
    let wrap = |w: gltf::texture::WrappingMode| match w {
        gltf::texture::WrappingMode::ClampToEdge => WrapMode::ClampToEdge,
        gltf::texture::WrappingMode::MirroredRepeat => WrapMode::MirroredRepeat,
        gltf::texture::WrappingMode::Repeat => WrapMode::Repeat,
    };
    let tex_coord = transform
        .as_ref()
        .and_then(|t| t.tex_coord())
        .unwrap_or(tex_coord);
    TextureRef {
        image: tex.source().index(),
        tex_coord,
        sampler: SamplerInfo {
            wrap_u: wrap(s.wrap_s()),
            wrap_v: wrap(s.wrap_t()),
            mag_linear: !matches!(s.mag_filter(), Some(gltf::texture::MagFilter::Nearest)),
        },
        transform: transform.map(|t| UvTransform {
            offset: t.offset(),
            rotation: t.rotation(),
            scale: t.scale(),
        }),
    }
}

fn read_primitive(
    prim: &gltf::Primitive,
    buffers: &[gltf::buffer::Data],
    world: Mat4,
    name: String,
    path: &Path,
) -> Result<Option<Mesh>, AssetError> {
    use gltf::mesh::Mode;
    let mode = prim.mode();
    if !matches!(
        mode,
        Mode::Triangles | Mode::TriangleStrip | Mode::TriangleFan
    ) {
        return Ok(None);
    }
    let reader = prim.reader(|b| buffers.get(b.index()).map(|d| d.0.as_slice()));
    let Some(pos) = reader.read_positions() else {
        return Ok(None);
    };
    let positions: Vec<[f32; 3]> = pos.collect();
    let n = positions.len();
    let normals: Option<Vec<[f32; 3]>> = reader.read_normals().map(|i| i.collect());
    let tangents: Option<Vec<[f32; 4]>> = reader.read_tangents().map(|i| i.collect());
    let uv0: Option<Vec<[f32; 2]>> = reader.read_tex_coords(0).map(|i| i.into_f32().collect());
    let uv1: Option<Vec<[f32; 2]>> = reader.read_tex_coords(1).map(|i| i.into_f32().collect());
    let colors: Option<Vec<[f32; 4]>> = reader.read_colors(0).map(|i| i.into_rgba_f32().collect());
    let raw_idx: Vec<u32> = match reader.read_indices() {
        Some(i) => i.into_u32().collect(),
        None => (0..n as u32).collect(),
    };
    if raw_idx.iter().any(|&i| i as usize >= n) {
        return Err(gltf_err(path, format!("{name}: index out of range")));
    }
    let mut indices = match mode {
        Mode::Triangles => raw_idx[..raw_idx.len() / 3 * 3].to_vec(),
        Mode::TriangleStrip => {
            let mut v = Vec::new();
            for i in 2..raw_idx.len() {
                let (a, b, c) = (raw_idx[i - 2], raw_idx[i - 1], raw_idx[i]);
                if i % 2 == 0 {
                    v.extend_from_slice(&[a, b, c]);
                } else {
                    v.extend_from_slice(&[b, a, c]);
                }
            }
            v
        }
        _ => {
            let mut v = Vec::new();
            for i in 2..raw_idx.len() {
                v.extend_from_slice(&[raw_idx[0], raw_idx[i - 1], raw_idx[i]]);
            }
            v
        }
    };

    // Transform to model space.
    let normal_mat = Mat3::from_mat4(world).inverse().transpose();
    let lin = Mat3::from_mat4(world);
    let mirrored = lin.determinant() < 0.0;
    let positions: Vec<[f32; 3]> = positions
        .iter()
        .map(|p| world.transform_point3(Vec3::from(*p)).to_array())
        .collect();
    let mut normals: Vec<[f32; 3]> = match normals {
        Some(ns) => ns
            .iter()
            .map(|nv| {
                (normal_mat * Vec3::from(*nv))
                    .normalize_or(Vec3::Z)
                    .to_array()
            })
            .collect(),
        None => Vec::new(),
    };
    if mirrored {
        for t in indices.chunks_exact_mut(3) {
            t.swap(1, 2);
        }
    }
    if normals.len() != n {
        normals = compute_normals(&positions, &indices);
    }
    let mut generated_tangents = false;
    let tangents: Vec<[f32; 4]> = match (tangents, &uv0) {
        (Some(ts), _) if ts.len() == n => ts
            .iter()
            .map(|t| {
                let v = (lin * glam::Vec4::from(*t).xyz()).normalize_or(Vec3::X);
                let w = if mirrored { -t[3] } else { t[3] };
                [v.x, v.y, v.z, if w < 0.0 { -1.0 } else { 1.0 }]
            })
            .collect(),
        (_, Some(uv)) if uv.len() == n => {
            generated_tangents = true;
            generate_tangents(&positions, &normals, uv, &indices)
        }
        _ => {
            generated_tangents = true;
            normals.iter().map(|nv| fallback_tangent(*nv)).collect()
        }
    };

    Ok(Some(Mesh {
        name,
        positions,
        normals,
        tangents,
        uv0: uv0
            .filter(|u| u.len() == n)
            .unwrap_or_else(|| vec![[0.0, 0.0]; n]),
        uv1: uv1.filter(|u| u.len() == n),
        colors: colors.filter(|c| c.len() == n),
        indices,
        material: prim.material().index(),
        generated_tangents,
    }))
}

/// Area-weighted smooth vertex normals.
pub fn compute_normals(positions: &[[f32; 3]], indices: &[u32]) -> Vec<[f32; 3]> {
    let mut acc = vec![Vec3::ZERO; positions.len()];
    for t in indices.chunks_exact(3) {
        let (a, b, c) = (t[0] as usize, t[1] as usize, t[2] as usize);
        let pa = Vec3::from(positions[a]);
        let fnrm = (Vec3::from(positions[b]) - pa).cross(Vec3::from(positions[c]) - pa);
        acc[a] += fnrm;
        acc[b] += fnrm;
        acc[c] += fnrm;
    }
    acc.into_iter()
        .map(|v| v.normalize_or(Vec3::Y).to_array())
        .collect()
}

fn fallback_tangent(n: [f32; 3]) -> [f32; 4] {
    let n = Vec3::from(n);
    let helper = if n.y.abs() < 0.99 { Vec3::Y } else { Vec3::X };
    let t = helper.cross(n).normalize_or(Vec3::X);
    [t.x, t.y, t.z, 1.0]
}

struct MikkGeom<'a> {
    positions: &'a [[f32; 3]],
    normals: &'a [[f32; 3]],
    uvs: &'a [[f32; 2]],
    indices: &'a [u32],
    out: Vec<[f32; 4]>,
    written: Vec<bool>,
}

impl bevy_mikktspace::Geometry for MikkGeom<'_> {
    fn num_faces(&self) -> usize {
        self.indices.len() / 3
    }
    fn num_vertices_of_face(&self, _face: usize) -> usize {
        3
    }
    fn position(&self, face: usize, vert: usize) -> [f32; 3] {
        self.positions[self.indices[face * 3 + vert] as usize]
    }
    fn normal(&self, face: usize, vert: usize) -> [f32; 3] {
        self.normals[self.indices[face * 3 + vert] as usize]
    }
    fn tex_coord(&self, face: usize, vert: usize) -> [f32; 2] {
        self.uvs[self.indices[face * 3 + vert] as usize]
    }
    fn set_tangent(
        &mut self,
        tangent_space: Option<bevy_mikktspace::TangentSpace>,
        face: usize,
        vert: usize,
    ) {
        let i = self.indices[face * 3 + vert] as usize;
        if let Some(ts) = tangent_space {
            self.out[i] = ts.tangent_encoded();
            self.written[i] = true;
        }
    }
}

/// MikkTSpace tangents for an indexed triangle list (per-vertex; vertices
/// shared by faces with different tangent frames keep the last one written,
/// which matches what most engines do with indexed glTF data).
pub fn generate_tangents(
    positions: &[[f32; 3]],
    normals: &[[f32; 3]],
    uvs: &[[f32; 2]],
    indices: &[u32],
) -> Vec<[f32; 4]> {
    let mut g = MikkGeom {
        positions,
        normals,
        uvs,
        indices,
        out: vec![[0.0; 4]; positions.len()],
        written: vec![false; positions.len()],
    };
    let ok = bevy_mikktspace::generate_tangents(&mut g).is_ok();
    let MikkGeom { out, written, .. } = g;
    out.into_iter()
        .zip(written)
        .zip(normals)
        .map(|((t, w), nv)| if ok && w { t } else { fallback_tangent(*nv) })
        .collect()
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Some(v) = s
                .get(i + 1..i + 3)
                .and_then(|h| u8::from_str_radix(h, 16).ok())
            {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn base64_decode(s: &str) -> Option<Vec<u8>> {
    fn val(c: u8) -> Option<u32> {
        Some(match c {
            b'A'..=b'Z' => (c - b'A') as u32,
            b'a'..=b'z' => (c - b'a' + 26) as u32,
            b'0'..=b'9' => (c - b'0' + 52) as u32,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            _ => return None,
        })
    }
    let clean: Vec<u8> = s
        .bytes()
        .filter(|c| !c.is_ascii_whitespace() && *c != b'=')
        .collect();
    let mut out = Vec::with_capacity(clean.len() * 3 / 4);
    for chunk in clean.chunks(4) {
        let mut acc = 0u32;
        for (i, c) in chunk.iter().enumerate() {
            acc |= val(*c)? << (18 - 6 * i);
        }
        let bytes = [(acc >> 16) as u8, (acc >> 8) as u8, acc as u8];
        match chunk.len() {
            4 => out.extend_from_slice(&bytes),
            3 => out.extend_from_slice(&bytes[..2]),
            2 => out.push(bytes[0]),
            _ => return None,
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_and_percent() {
        assert_eq!(base64_decode("aGVsbG8=").unwrap(), b"hello");
        assert_eq!(base64_decode("aGk").unwrap(), b"hi");
        assert_eq!(percent_decode("a%20b.png"), "a b.png");
        assert_eq!(percent_decode("100%"), "100%");
    }

    #[test]
    fn tangents_follow_uv_u() {
        // Quad in XZ plane facing +Y, U along +X, V along -Z.
        let p = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 0.0, -1.0],
            [0.0, 0.0, -1.0],
        ];
        let uv = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        let idx = [0, 1, 2, 0, 2, 3];
        let nrm = compute_normals(&p, &idx);
        assert!(nrm[0][1] > 0.99);
        let t = generate_tangents(&p, &nrm, &uv, &idx);
        for tv in &t {
            assert!(tv[0] > 0.99, "{tv:?}");
            assert!(tv[3].abs() == 1.0);
        }
    }
}
