//! Plain CPU-side data types that form the renderer's public interface.
//!
//! None of these types reference wgpu, so game code (and the asset crate) can
//! build them without knowing anything about the GPU side.

use glam::{Mat4, Vec2, Vec3, Vec4};

macro_rules! id_type {
    ($(#[$m:meta])* $name:ident) => {
        $(#[$m])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(pub u32);
    };
}

id_type!(
    /// Handle to a mesh uploaded with [`crate::Renderer::create_mesh`].
    MeshId
);
id_type!(
    /// Handle to a texture uploaded with [`crate::Renderer::create_texture`].
    TextureId
);
id_type!(
    /// Handle to a material created with [`crate::Renderer::create_material`].
    MaterialId
);
id_type!(
    /// Handle to a placed mesh instance.
    InstanceId
);
id_type!(
    /// Handle to an environment HDRI uploaded with [`crate::Renderer::set_hdri`].
    HdriId
);
id_type!(
    /// Handle to a persistent projected decal.
    DecalId
);

/// Pixel layout of an [`ImageData`] mip level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageFormat {
    /// 1 byte per pixel (roughness, metalness, AO, masks).
    R8,
    /// 2 bytes per pixel.
    Rg8,
    /// 4 bytes per pixel. With `srgb = true` the RGB channels are sRGB encoded.
    Rgba8,
    /// 8 bytes per pixel, IEEE half floats (little endian).
    Rgba16F,
}

impl ImageFormat {
    pub fn bytes_per_pixel(self) -> usize {
        match self {
            ImageFormat::R8 => 1,
            ImageFormat::Rg8 => 2,
            ImageFormat::Rgba8 => 4,
            ImageFormat::Rgba16F => 8,
        }
    }
}

/// A CPU image with an optional precomputed mip chain.
///
/// `mips[0]` is the full resolution level; each following level halves the
/// size (rounded down, minimum 1). If only one level is supplied the renderer
/// generates the rest (gamma-correct for sRGB, renormalising for normal maps
/// is left to the producer). Normal maps are expected in OpenGL (+Y) convention.
#[derive(Clone, Debug)]
pub struct ImageData {
    pub width: u32,
    pub height: u32,
    pub format: ImageFormat,
    pub mips: Vec<Vec<u8>>,
    pub srgb: bool,
}

impl ImageData {
    /// Single-level RGBA8 image.
    pub fn rgba8(width: u32, height: u32, data: Vec<u8>, srgb: bool) -> Self {
        assert_eq!(data.len(), (width * height * 4) as usize);
        Self {
            width,
            height,
            format: ImageFormat::Rgba8,
            mips: vec![data],
            srgb,
        }
    }

    /// Single-level one channel image.
    pub fn r8(width: u32, height: u32, data: Vec<u8>) -> Self {
        assert_eq!(data.len(), (width * height) as usize);
        Self {
            width,
            height,
            format: ImageFormat::R8,
            mips: vec![data],
            srgb: false,
        }
    }

    /// Loads PNG/JPEG from disk into an RGBA8 image (no mips; the renderer generates them).
    pub fn load(path: &std::path::Path, srgb: bool) -> anyhow::Result<Self> {
        let img = image::open(path)?.to_rgba8();
        let (w, h) = img.dimensions();
        Ok(Self::rgba8(w, h, img.into_raw(), srgb))
    }

    /// Box-filtered downscale so the largest side is at most `max_size` (keeps power-of-two ratios).
    pub fn downscaled(&self, max_size: u32) -> Self {
        let mut w = self.width;
        let mut h = self.height;
        let mut level = 0;
        while (w > max_size || h > max_size) && level + 1 < self.mips.len() {
            w = (w / 2).max(1);
            h = (h / 2).max(1);
            level += 1;
        }
        if level > 0 {
            return Self {
                width: w,
                height: h,
                format: self.format,
                mips: self.mips[level..].to_vec(),
                srgb: self.srgb,
            };
        }
        let mut out = self.clone();
        out.mips.truncate(1);
        while out.width > max_size || out.height > max_size {
            let next = crate::texture::downsample_level(
                &out.mips[0],
                out.width,
                out.height,
                out.format,
                out.srgb,
            );
            out.width = (out.width / 2).max(1);
            out.height = (out.height / 2).max(1);
            out.mips[0] = next;
        }
        out
    }
}

/// Linear HDR environment map in equirectangular (lat-long) layout, RGB f32.
#[derive(Clone, Debug)]
pub struct HdrImage {
    pub width: u32,
    pub height: u32,
    pub rgb_f32: Vec<f32>,
}

impl HdrImage {
    /// Loads a Radiance `.hdr` file.
    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        let img = image::open(path)?.to_rgb32f();
        let (w, h) = img.dimensions();
        Ok(Self {
            width: w,
            height: h,
            rgb_f32: img.into_raw(),
        })
    }
}

/// Renderer-agnostic triangle mesh. All per-vertex arrays must have the same
/// length as `positions` (tangents may be empty: they are then generated from UVs).
#[derive(Clone, Debug, Default)]
pub struct MeshData {
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    /// xyz tangent, w = bitangent sign.
    pub tangents: Vec<Vec4>,
    pub uvs: Vec<Vec2>,
    pub uvs2: Option<Vec<Vec2>>,
    /// Linear RGBA vertex colour (multiplies base colour; RGB also multiplies emissive).
    pub colors: Option<Vec<Vec4>>,
    pub indices: Vec<u32>,
}

impl MeshData {
    /// Recomputes smooth vertex normals from triangle geometry (area weighted).
    pub fn compute_normals(&mut self) {
        let mut n = vec![Vec3::ZERO; self.positions.len()];
        for t in self.indices.chunks_exact(3) {
            let (a, b, c) = (t[0] as usize, t[1] as usize, t[2] as usize);
            let fnrm = (self.positions[b] - self.positions[a])
                .cross(self.positions[c] - self.positions[a]);
            n[a] += fnrm;
            n[b] += fnrm;
            n[c] += fnrm;
        }
        self.normals = n.into_iter().map(|v| v.normalize_or(Vec3::Y)).collect();
    }

    /// Generates per-vertex tangents from UVs (Lengyel's method).
    pub fn compute_tangents(&mut self) {
        let count = self.positions.len();
        if self.uvs.len() != count {
            self.uvs = vec![Vec2::ZERO; count];
        }
        if self.normals.len() != count {
            self.compute_normals();
        }
        let mut tan = vec![Vec3::ZERO; count];
        let mut bit = vec![Vec3::ZERO; count];
        for t in self.indices.chunks_exact(3) {
            let (a, b, c) = (t[0] as usize, t[1] as usize, t[2] as usize);
            let e1 = self.positions[b] - self.positions[a];
            let e2 = self.positions[c] - self.positions[a];
            let d1 = self.uvs[b] - self.uvs[a];
            let d2 = self.uvs[c] - self.uvs[a];
            let det = d1.x * d2.y - d2.x * d1.y;
            if det.abs() < 1e-12 {
                continue;
            }
            let r = 1.0 / det;
            let sdir = (e1 * d2.y - e2 * d1.y) * r;
            let tdir = (e2 * d1.x - e1 * d2.x) * r;
            for &i in &[a, b, c] {
                tan[i] += sdir;
                bit[i] += tdir;
            }
        }
        self.tangents = (0..count)
            .map(|i| {
                let n = self.normals[i];
                let mut t = tan[i] - n * n.dot(tan[i]);
                if t.length_squared() < 1e-12 {
                    t = n.any_orthonormal_vector();
                }
                let t = t.normalize();
                // Bitangent = cross(n, t) * w points towards decreasing v (image "up"),
                // which matches OpenGL (+Y) normal maps with top-left UV origin (glTF).
                let w = if n.cross(t).dot(bit[i]) < 0.0 {
                    1.0
                } else {
                    -1.0
                };
                t.extend(w)
            })
            .collect();
    }

    /// Axis aligned bounds of the positions.
    pub fn bounds(&self) -> (Vec3, Vec3) {
        let mut lo = Vec3::splat(f32::MAX);
        let mut hi = Vec3::splat(f32::MIN);
        for p in &self.positions {
            lo = lo.min(*p);
            hi = hi.max(*p);
        }
        if self.positions.is_empty() {
            (Vec3::ZERO, Vec3::ZERO)
        } else {
            (lo, hi)
        }
    }

    /// Appends another mesh (with a transform) to this one.
    pub fn append(&mut self, other: &MeshData, xf: Mat4) {
        let base = self.positions.len() as u32;
        let nm = Mat4::from_mat3(glam::Mat3::from_mat4(xf).inverse().transpose());
        let had_uv2 = self.uvs2.is_some() || other.uvs2.is_some();
        let had_col = self.colors.is_some() || other.colors.is_some();
        if had_uv2 && self.uvs2.is_none() {
            self.uvs2 = Some(self.uvs.clone());
        }
        if had_col && self.colors.is_none() {
            self.colors = Some(vec![Vec4::ONE; self.positions.len()]);
        }
        for i in 0..other.positions.len() {
            self.positions.push(xf.transform_point3(other.positions[i]));
            self.normals.push(
                nm.transform_vector3(*other.normals.get(i).unwrap_or(&Vec3::Y))
                    .normalize_or(Vec3::Y),
            );
            if let Some(t) = other.tangents.get(i) {
                let tt = xf.transform_vector3(t.truncate()).normalize_or(Vec3::X);
                self.tangents.push(tt.extend(t.w));
            }
            let uv = *other.uvs.get(i).unwrap_or(&Vec2::ZERO);
            self.uvs.push(uv);
            if let Some(u2) = self.uvs2.as_mut() {
                u2.push(other.uvs2.as_ref().map(|v| v[i]).unwrap_or(uv));
            }
            if let Some(c) = self.colors.as_mut() {
                c.push(other.colors.as_ref().map(|v| v[i]).unwrap_or(Vec4::ONE));
            }
        }
        if self.tangents.len() != self.positions.len() {
            self.tangents.clear();
        }
        self.indices.extend(other.indices.iter().map(|i| i + base));
    }
}

/// How a material's textures are mapped.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UvMode {
    /// Mesh UV set 0 (`uv * uv_scale + uv_offset`).
    Mesh,
    /// Mesh UV set 1.
    Mesh2,
    /// Object-space triplanar projection; `uv_scale` = repeats per metre.
    Triplanar,
    /// World-space top-down (XZ) projection, ideal for roads/terrain; `uv_scale` = repeats per metre.
    WorldXZ,
}

/// Transparency handling.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AlphaMode {
    Opaque,
    /// Alpha tested with the given cutoff (foliage, fences). Rendered in the G-buffer.
    Mask(f32),
    /// Alpha blended in the forward pass (sorted back-to-front).
    Blend,
}

/// Metallic-roughness PBR material with optional extra layers.
#[derive(Clone, Debug)]
pub struct Material {
    /// Linear base colour; `a` = opacity for `AlphaMode::Blend`/`Mask`.
    pub base_color: Vec4,
    pub metallic: f32,
    pub roughness: f32,
    /// Specular reflectance for dielectrics (0.5 = F0 of 4%).
    pub reflectance: f32,
    /// Emitted luminance in nits (cd/m²), linear RGB.
    pub emissive: Vec3,
    pub normal_scale: f32,
    pub occlusion_strength: f32,
    pub albedo_tex: Option<TextureId>,
    pub normal_tex: Option<TextureId>,
    /// Roughness in R (or glTF packed metallic-roughness when `packed_metal_rough`).
    pub roughness_tex: Option<TextureId>,
    pub metallic_tex: Option<TextureId>,
    pub ao_tex: Option<TextureId>,
    pub emissive_tex: Option<TextureId>,
    /// `roughness_tex` is a glTF metallicRoughness texture (G = roughness, B = metallic).
    pub packed_metal_rough: bool,
    pub uv_mode: UvMode,
    pub uv_scale: Vec2,
    pub uv_offset: Vec2,
    /// Clear-coat layer strength (car paint), 0..1.
    pub clearcoat: f32,
    pub clearcoat_roughness: f32,
    /// Metallic flake sparkle strength under the clear coat (0 = none).
    pub flake: f32,
    /// Flake cell size in metres (object space).
    pub flake_scale: f32,
    /// Thin-glass transmission 0..1 (forces forward rendering).
    pub transmission: f32,
    pub ior: f32,
    pub alpha_mode: AlphaMode,
    pub double_sided: bool,
    /// Foliage-style diffuse transmission 0..1.
    pub translucency: f32,
    /// How strongly rain affects this surface (0 = sheltered / interior).
    pub wet_factor: f32,
    /// Cast shadows when instanced.
    pub cast_shadows: bool,
}

impl Default for Material {
    fn default() -> Self {
        Self {
            base_color: Vec4::new(0.8, 0.8, 0.8, 1.0),
            metallic: 0.0,
            roughness: 0.5,
            reflectance: 0.5,
            emissive: Vec3::ZERO,
            normal_scale: 1.0,
            occlusion_strength: 1.0,
            albedo_tex: None,
            normal_tex: None,
            roughness_tex: None,
            metallic_tex: None,
            ao_tex: None,
            emissive_tex: None,
            packed_metal_rough: false,
            uv_mode: UvMode::Mesh,
            uv_scale: Vec2::ONE,
            uv_offset: Vec2::ZERO,
            clearcoat: 0.0,
            clearcoat_roughness: 0.05,
            flake: 0.0,
            flake_scale: 0.002,
            transmission: 0.0,
            ior: 1.5,
            alpha_mode: AlphaMode::Opaque,
            double_sided: false,
            translucency: 0.0,
            wet_factor: 1.0,
            cast_shadows: true,
        }
    }
}

impl Material {
    /// Convenience: solid colour dielectric/metal.
    pub fn solid(color: Vec3, roughness: f32, metallic: f32) -> Self {
        Self {
            base_color: color.extend(1.0),
            roughness,
            metallic,
            ..Default::default()
        }
    }

    /// Typical automotive clear-coated paint.
    pub fn car_paint(color: Vec3, metallic_flakes: bool) -> Self {
        Self {
            base_color: color.extend(1.0),
            metallic: if metallic_flakes { 0.6 } else { 0.0 },
            roughness: if metallic_flakes { 0.38 } else { 0.45 },
            clearcoat: 1.0,
            clearcoat_roughness: 0.03,
            flake: if metallic_flakes { 0.35 } else { 0.0 },
            flake_scale: 0.0015,
            ..Default::default()
        }
    }

    /// Thin tinted glass (car windows).
    pub fn glass(tint: Vec3, opacity: f32) -> Self {
        Self {
            base_color: tint.extend(opacity),
            roughness: 0.02,
            transmission: 1.0,
            alpha_mode: AlphaMode::Blend,
            double_sided: true,
            cast_shadows: false,
            ..Default::default()
        }
    }

    pub(crate) fn is_forward(&self) -> bool {
        self.transmission > 0.0 || self.alpha_mode == AlphaMode::Blend
    }
}

/// Local light shape.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LightKind {
    Point,
    /// Cone angles are half-angles in radians.
    Spot {
        inner_angle: f32,
        outer_angle: f32,
    },
}

/// A local (point/spot) light. Updated every frame with [`crate::Renderer::set_lights`].
#[derive(Clone, Copy, Debug)]
pub struct Light {
    pub kind: LightKind,
    pub position: Vec3,
    /// Direction the spot points to (ignored for point lights).
    pub direction: Vec3,
    /// Linear RGB colour (normalised, luminance ~1).
    pub color: Vec3,
    /// Luminous intensity in candela.
    pub intensity: f32,
    /// Range in metres after which the light is cut off smoothly.
    pub range: f32,
    /// Emitter radius in metres (softens highlights).
    pub source_radius: f32,
    /// Render a shadow map for this light (spot lights only; limited by quality preset).
    pub cast_shadows: bool,
    /// Scatter into fog / rain (light shafts, halos). 0 = none, 1 = physical.
    pub volumetric: f32,
}

impl Default for Light {
    fn default() -> Self {
        Self {
            kind: LightKind::Point,
            position: Vec3::ZERO,
            direction: Vec3::NEG_Y,
            color: Vec3::ONE,
            intensity: 1000.0,
            range: 20.0,
            source_radius: 0.05,
            cast_shadows: false,
            volumetric: 1.0,
        }
    }
}

impl Light {
    pub fn point(position: Vec3, color: Vec3, intensity_cd: f32, range: f32) -> Self {
        Self {
            position,
            color,
            intensity: intensity_cd,
            range,
            ..Default::default()
        }
    }

    pub fn spot(
        position: Vec3,
        direction: Vec3,
        color: Vec3,
        intensity_cd: f32,
        range: f32,
        inner: f32,
        outer: f32,
    ) -> Self {
        Self {
            kind: LightKind::Spot {
                inner_angle: inner,
                outer_angle: outer,
            },
            position,
            direction: direction.normalize_or(Vec3::NEG_Y),
            color,
            intensity: intensity_cd,
            range,
            ..Default::default()
        }
    }
}

/// Approximate linear sRGB colour of a black body (Kelvin), luminance-normalised.
pub fn kelvin_to_rgb(kelvin: f32) -> Vec3 {
    // Tanner Helland fit, then linearised.
    let t = kelvin.clamp(1000.0, 40000.0) / 100.0;
    let r = if t <= 66.0 {
        255.0
    } else {
        329.698_73 * (t - 60.0).powf(-0.133_204_76)
    };
    let g = if t <= 66.0 {
        99.470_8 * t.ln() - 161.119_57
    } else {
        288.122_16 * (t - 60.0).powf(-0.075_514_846)
    };
    let b = if t >= 66.0 {
        255.0
    } else if t <= 19.0 {
        0.0
    } else {
        138.517_73 * (t - 10.0).ln() - 305.044_8
    };
    let lin = |c: f32| {
        let c = (c / 255.0).clamp(0.0, 1.0);
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    let c = Vec3::new(lin(r), lin(g), lin(b));
    let lum = c.dot(Vec3::new(0.2126, 0.7152, 0.0722)).max(1e-4);
    c / lum
}

/// Visual style of a particle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParticleKind {
    /// Soft volumetric-looking puff (tyre smoke, exhaust, dust). Lit.
    Smoke,
    /// Hot additive streak stretched along velocity. Emissive.
    Spark,
    /// Small hard-edged chunk. Lit.
    Debris,
    /// Water droplets / spray. Lit + specular glint.
    Splash,
    /// Plain soft round billboard (generic).
    Soft,
}

/// A camera-facing billboard, supplied by the game every frame.
#[derive(Clone, Copy, Debug)]
pub struct Particle {
    pub position: Vec3,
    /// Half-size in metres.
    pub size: f32,
    /// Linear RGB albedo (lit kinds) or emission tint, `a` = opacity.
    pub color: Vec4,
    /// Emitted luminance in nits (added on top, use for sparks).
    pub emissive: f32,
    /// Rotation in radians around the view axis.
    pub rotation: f32,
    /// World velocity, used to stretch sparks/splashes.
    pub velocity: Vec3,
    pub kind: ParticleKind,
}

impl Default for Particle {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            size: 0.5,
            color: Vec4::new(0.7, 0.7, 0.7, 0.5),
            emissive: 0.0,
            rotation: 0.0,
            velocity: Vec3::ZERO,
            kind: ParticleKind::Smoke,
        }
    }
}

/// Procedural decal pattern.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DecalKind {
    /// Road paint (lane lines, stop lines, crosswalk bars) with wear.
    Paint,
    /// Dashed paint along the decal's local Z axis: `dash` metres painted, `gap` metres empty.
    Dashed { dash: f32, gap: f32 },
    /// Dark rubber tyre mark (streaky along local Z).
    SkidMark,
    /// Darkening stain (oil, dirt, patches of fresh asphalt).
    Stain,
}

/// A projected box decal. `transform` maps the unit cube [-0.5, 0.5]³ to world
/// space; the decal projects along its local Y axis (so for road markings use
/// a flat box: X = width, Y = projection depth, Z = length).
#[derive(Clone, Copy, Debug)]
pub struct Decal {
    pub transform: Mat4,
    pub kind: DecalKind,
    /// Linear albedo and opacity.
    pub color: Vec4,
    pub roughness: f32,
    /// Amount of wear/fade noise 0..1.
    pub wear: f32,
}

impl Decal {
    /// Flat road marking centred at `center` (on the road surface), heading along `dir` (XZ).
    pub fn road(
        center: Vec3,
        dir: Vec3,
        width: f32,
        length: f32,
        kind: DecalKind,
        color: Vec4,
    ) -> Self {
        let d = Vec3::new(dir.x, 0.0, dir.z).normalize_or(Vec3::Z);
        let x = Vec3::Y.cross(d);
        let m = Mat4::from_cols(
            (x * width).extend(0.0),
            (Vec3::Y * 0.4).extend(0.0),
            (d * length).extend(0.0),
            center.extend(1.0),
        );
        Self {
            transform: m,
            kind,
            color,
            roughness: 0.55,
            wear: 0.35,
        }
    }
}

/// Perspective camera.
#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub position: Vec3,
    /// Normalised view direction.
    pub forward: Vec3,
    pub up: Vec3,
    /// Vertical field of view in radians.
    pub fov_y: f32,
    pub near: f32,
    /// Manual exposure override in EV100 (disables auto exposure for this frame).
    pub exposure_ev100: Option<f32>,
    /// Set when the camera teleports, to reset temporal history.
    pub cut: bool,
}

impl Camera {
    pub fn look_at(position: Vec3, target: Vec3, fov_y_deg: f32) -> Self {
        Self {
            position,
            forward: (target - position).normalize_or(Vec3::NEG_Z),
            up: Vec3::Y,
            fov_y: fov_y_deg.to_radians(),
            near: 0.05,
            exposure_ev100: None,
            cut: false,
        }
    }

    pub fn view_matrix(&self) -> Mat4 {
        mat::look_to(self.position, self.forward, self.up)
    }
}

/// Exponential height fog parameters.
#[derive(Clone, Copy, Debug)]
pub struct Fog {
    /// Extinction at `base_height` per metre (0.0005 = clear, 0.02 = dense).
    pub density: f32,
    /// Exponential falloff with height per metre.
    pub height_falloff: f32,
    pub base_height: f32,
    /// Henyey-Greenstein anisotropy for sun in-scattering.
    pub anisotropy: f32,
    /// Distance before fog starts (m).
    pub start: f32,
    /// Scale for volumetric light scattering (lamps, headlights).
    pub volumetric_scale: f32,
}

impl Default for Fog {
    fn default() -> Self {
        Self {
            density: 0.0008,
            height_falloff: 0.05,
            base_height: 0.0,
            anisotropy: 0.6,
            start: 5.0,
            volumetric_scale: 1.0,
        }
    }
}

/// Global lighting/weather state for the frame.
#[derive(Clone, Copy, Debug)]
pub struct Environment {
    /// Direction *towards* the sun.
    pub sun_direction: Vec3,
    /// Solar illuminance at the top of the atmosphere (lux). Real value ≈ 120 000.
    pub sun_illuminance: f32,
    /// Direction towards the moon.
    pub moon_direction: Vec3,
    /// Moon illuminance (lux), full moon ≈ 0.25.
    pub moon_illuminance: f32,
    /// Multiplier on procedural sky radiance.
    pub sky_intensity: f32,
    /// Mie (haze) density multiplier, 1 = clear day.
    pub turbidity: f32,
    pub ground_albedo: f32,
    /// 0 = clear, 1 = fully overcast.
    pub cloud_cover: f32,
    /// Star field brightness at night.
    pub stars: f32,
    /// City light pollution glow at the horizon / cloud base (linear RGB nits).
    pub night_glow: Vec3,
    /// HDRI to use for image-based lighting (and optionally background).
    pub hdri: Option<HdriId>,
    /// HDRI yaw rotation (radians).
    pub hdri_rotation: f32,
    /// Multiplier converting HDRI values to nits.
    pub hdri_intensity: f32,
    /// 0 = procedural sky lighting only, 1 = HDRI only.
    pub hdri_blend: f32,
    /// Show the HDRI as background instead of the procedural sky.
    pub hdri_background: bool,
    pub fog: Fog,
    /// Surface wetness 0..1.
    pub wetness: f32,
    /// Standing water coverage 0..1 (on upward facing wet surfaces).
    pub puddles: f32,
    /// Rain fall intensity 0..1 (streaks and ripples).
    pub rain: f32,
    /// Wind (m/s, XZ) for rain slant and clouds.
    pub wind: Vec2,
    /// Exposure compensation in EV.
    pub exposure_compensation: f32,
    /// Seconds; drives animated effects.
    pub time: f32,
}

impl Default for Environment {
    fn default() -> Self {
        Self {
            sun_direction: Vec3::new(0.3, 0.8, 0.4).normalize(),
            sun_illuminance: 120_000.0,
            moon_direction: Vec3::new(-0.3, 0.5, -0.6).normalize(),
            moon_illuminance: 0.25,
            sky_intensity: 1.0,
            turbidity: 1.0,
            ground_albedo: 0.25,
            cloud_cover: 0.2,
            stars: 1.0,
            night_glow: Vec3::new(0.06, 0.035, 0.02),
            hdri: None,
            hdri_rotation: 0.0,
            hdri_intensity: 1.0,
            hdri_blend: 0.0,
            hdri_background: false,
            fog: Fog::default(),
            wetness: 0.0,
            puddles: 0.0,
            rain: 0.0,
            wind: Vec2::new(2.0, 1.0),
            exposure_compensation: 0.0,
            time: 0.0,
        }
    }
}

impl Environment {
    /// Sun direction for a solar elevation/azimuth in degrees (azimuth 0 = +Z, 90 = +X).
    pub fn sun_from_angles(elevation_deg: f32, azimuth_deg: f32) -> Vec3 {
        let e = elevation_deg.to_radians();
        let a = azimuth_deg.to_radians();
        Vec3::new(e.cos() * a.sin(), e.sin(), e.cos() * a.cos()).normalize()
    }
}

/// Per-frame statistics.
#[derive(Clone, Debug, Default)]
pub struct FrameStats {
    pub frame: u64,
    pub cpu_ms: f32,
    /// GPU time per pass in ms (empty if timestamp queries are unsupported).
    pub gpu_pass_ms: Vec<(&'static str, f32)>,
    pub gpu_total_ms: f32,
    pub draw_calls: u32,
    pub shadow_draw_calls: u32,
    pub instances_drawn: u32,
    pub triangles: u64,
    pub lights: u32,
    pub particles: u32,
    pub decals: u32,
    pub render_width: u32,
    pub render_height: u32,
}

/// Matrix helpers for wgpu conventions (right-handed view space, NDC z in [0, 1], y up).
pub mod mat {
    use glam::camera::rh::{proj::directx, view};
    use glam::{Mat4, Vec3};

    pub fn look_to(eye: Vec3, dir: Vec3, up: Vec3) -> Mat4 {
        view::look_to_mat4(eye, dir, up)
    }
    pub fn look_at(eye: Vec3, target: Vec3, up: Vec3) -> Mat4 {
        view::look_at_mat4(eye, target, up)
    }
    pub fn perspective(fov_y: f32, aspect: f32, near: f32, far: f32) -> Mat4 {
        directx::perspective(fov_y, aspect, near, far)
    }
    /// Reversed-Z infinite perspective (near -> 1, infinity -> 0).
    pub fn perspective_infinite_reverse(fov_y: f32, aspect: f32, near: f32) -> Mat4 {
        directx::perspective_infinite_reverse(fov_y, aspect, near)
    }
    pub fn orthographic(l: f32, r: f32, b: f32, t: f32, near: f32, far: f32) -> Mat4 {
        directx::orthographic(l, r, b, t, near, far)
    }
}
