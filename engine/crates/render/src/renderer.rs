//! The [`Renderer`]: resource management, culling/batching and the frame graph.

use std::path::Path;
use std::time::Instant;

use anyhow::{anyhow, Context};
use glam::{Mat3, Mat4, Vec2, Vec3, Vec4};

use crate::frame::*;
use crate::gpu::{self, GpuContext, GrowBuffer};
use crate::pipelines::{self as pl, gbuffer_variant, Layouts, Pipelines};
use crate::settings::{AntiAliasing, RenderSettings, Tonemapper};
use crate::shadow::{fit_cascades, Frustum};
use crate::sky::{IblSet, Sky};
use crate::targets::{Targets, BLOOM_MIPS};
use crate::timing::GpuTimer;
use crate::types::mat;
use crate::types::*;

const CLUSTER_X: u32 = 16;
const CLUSTER_Y: u32 = 9;
const CLUSTER_Z: u32 = 24;
const CLUSTER_FAR: f32 = 600.0;
const MAX_SPOT_SHADOWS: u32 = 8;
const SKYOCC_SLOT: u32 = 4 + MAX_SPOT_SHADOWS;
const SKYOCC_RES: u32 = 1024;
const SKYOCC_SIZE: f32 = 256.0;
const SKYOCC_RANGE: f32 = 400.0;

struct MeshGpu {
    vb0: wgpu::Buffer,
    vb1: wgpu::Buffer,
    ib: wgpu::Buffer,
    index_count: u32,
    vertex_count: u32,
    lo: Vec3,
    hi: Vec3,
}

struct MaterialRec {
    mat: Material,
    buf: wgpu::Buffer,
    bg: wgpu::BindGroup,
}

#[derive(Clone)]
struct InstanceRec {
    mesh: MeshId,
    material: MaterialId,
    transform: Mat4,
    rendered: Mat4,
    tint: Vec4,
    emissive: f32,
    visible: bool,
    lo: Vec3,
    hi: Vec3,
}

#[derive(Clone, Copy)]
struct Draw {
    variant: u8,
    material: u32,
    mesh: u32,
    first: u32,
    count: u32,
}

struct Samplers {
    linear_clamp: wgpu::Sampler,
    linear_repeat: wgpu::Sampler,
    point_clamp: wgpu::Sampler,
    shadow: wgpu::Sampler,
    material: wgpu::Sampler,
}

#[allow(dead_code)]
struct ShadowRes {
    cascades: wgpu::Texture,
    cascade_layers: Vec<wgpu::TextureView>,
    cascade_view: wgpu::TextureView,
    spot: wgpu::Texture,
    spot_layers: Vec<wgpu::TextureView>,
    spot_view: wgpu::TextureView,
    res: u32,
    spot_res: u32,
}

impl ShadowRes {
    fn new(d: &wgpu::Device, s: &RenderSettings) -> Self {
        let layers = s.shadow_cascades.clamp(1, 4);
        let spots = s.spot_shadows.clamp(1, MAX_SPOT_SHADOWS);
        let mk = |label: &str, res: u32, layers: u32| {
            d.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width: res,
                    height: res,
                    depth_or_array_layers: layers,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: pl::SHADOW_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
        };
        let res = s.shadow_resolution.clamp(256, 8192);
        let spot_res = if s.spot_shadows == 0 {
            16
        } else {
            s.spot_shadow_resolution.clamp(128, 4096)
        };
        let cascades = mk("csm", res, layers);
        let spot = mk("spot-shadows", spot_res, spots);
        let layer_views = |t: &wgpu::Texture, n: u32| -> Vec<wgpu::TextureView> {
            (0..n)
                .map(|i| {
                    t.create_view(&wgpu::TextureViewDescriptor {
                        dimension: Some(wgpu::TextureViewDimension::D2),
                        base_array_layer: i,
                        array_layer_count: Some(1),
                        ..Default::default()
                    })
                })
                .collect()
        };
        let arr = |t: &wgpu::Texture| {
            t.create_view(&wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::D2Array),
                ..Default::default()
            })
        };
        Self {
            cascade_layers: layer_views(&cascades, layers),
            cascade_view: arr(&cascades),
            spot_layers: layer_views(&spot, spots),
            spot_view: arr(&spot),
            cascades,
            spot,
            res,
            spot_res,
        }
    }
}

/// Bind groups that reference screen targets or growable buffers.
struct BindGroups {
    instances: wgpu::BindGroup,
    lighting: wgpu::BindGroup,
    gbuf_read: wgpu::BindGroup,
    decal: wgpu::BindGroup,
    gtao: wgpu::BindGroup,
    ao_blur: wgpu::BindGroup,
    ssr: [wgpu::BindGroup; 2],
    particles: wgpu::BindGroup,
    taa: [wgpu::BindGroup; 2],
    bloom_first: [wgpu::BindGroup; 2],
    bloom_mips: Vec<wgpu::BindGroup>,
    post: [wgpu::BindGroup; 2],
    fxaa: wgpu::BindGroup,
    exposure: [wgpu::BindGroup; 2],
    depth: wgpu::BindGroup,
    cluster: wgpu::BindGroup,
    key: u64,
}

/// Physically based renderer. See the crate documentation for the frame graph.
pub struct Renderer {
    gpu: GpuContext,
    surface: Option<(wgpu::Surface<'static>, wgpu::SurfaceConfiguration)>,
    output_format: wgpu::TextureFormat,
    settings: RenderSettings,
    layouts: Layouts,
    pipes: Pipelines,
    samplers: Samplers,
    frame_buf: wgpu::Buffer,
    exposure_buf: wgpu::Buffer,
    exposure_params_buf: wgpu::Buffer,
    histogram_buf: wgpu::Buffer,
    shadow_view_buf: wgpu::Buffer,
    frame_bg: wgpu::BindGroup,
    shadow_frame_bg: wgpu::BindGroup,
    empty_bg: wgpu::BindGroup,
    sh_buf: wgpu::Buffer,
    taa_buf: wgpu::Buffer,
    ssr_buf: wgpu::Buffer,
    post_buf: wgpu::Buffer,
    rain_buf: wgpu::Buffer,
    white: wgpu::TextureView,
    flat_normal: wgpu::TextureView,
    textures: Vec<(wgpu::Texture, wgpu::TextureView)>,
    meshes: Vec<Option<MeshGpu>>,
    materials: Vec<MaterialRec>,
    instances: Vec<Option<InstanceRec>>,
    free_instances: Vec<u32>,
    instance_cpu: Vec<InstanceGpu>,
    dirty_instances: Vec<u32>,
    instance_buf: GrowBuffer,
    visible_buf: GrowBuffer,
    light_buf: GrowBuffer,
    cluster_buf: GrowBuffer,
    spot_mat_buf: GrowBuffer,
    particle_buf: GrowBuffer,
    decal_buf: GrowBuffer,
    line_buf: GrowBuffer,
    sky: Sky,
    hdris: Vec<IblSet>,
    bound_hdri: Option<u32>,
    shadow: ShadowRes,
    targets: Targets,
    bgs: Option<BindGroups>,
    env: Environment,
    lights: Vec<Light>,
    particles: Vec<Particle>,
    decals: Vec<Option<Decal>>,
    dynamic_decals: Vec<Decal>,
    lines: Vec<LineVertex>,
    frame_index: u64,
    history_index: usize,
    history_valid: bool,
    prev_view_proj: Option<Mat4>,
    prev_jitter: Vec2,
    last_render: Option<Instant>,
    sky_key: Option<[f32; 12]>,
    sky_ibl_age: u32,
    instant_exposure: u32,
    timer: Option<GpuTimer>,
    stats: FrameStats,
    skyocc_view: wgpu::TextureView,
    skyocc_state: Option<(Vec3, Mat4, u32)>,
}

fn halton(mut i: u32, b: u32) -> f32 {
    let mut f = 1.0;
    let mut r = 0.0;
    while i > 0 {
        f /= b as f32;
        r += f * (i % b) as f32;
        i /= b;
    }
    r
}

impl Renderer {
    /// Creates a renderer presenting to a window surface (created by the app from its winit window).
    pub fn new_windowed(
        instance: &wgpu::Instance,
        surface: wgpu::Surface<'static>,
        width: u32,
        height: u32,
        settings: RenderSettings,
    ) -> anyhow::Result<Self> {
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
            apply_limit_buckets: false,
        }))
        .map_err(|e| anyhow!("no compatible GPU adapter: {e}"))?;
        let gpu = gpu::create_device(&adapter)?;
        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .or_else(|| caps.formats.first().copied())
            .context("surface has no formats")?;
        let present_mode = if settings.vsync {
            wgpu::PresentMode::AutoVsync
        } else if caps.present_modes.contains(&wgpu::PresentMode::Mailbox) {
            wgpu::PresentMode::Mailbox
        } else {
            wgpu::PresentMode::AutoNoVsync
        };
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            color_space: Default::default(),
            width: width.max(1),
            height: height.max(1),
            present_mode,
            desired_maximum_frame_latency: 2,
            alpha_mode: caps
                .alpha_modes
                .first()
                .copied()
                .unwrap_or(wgpu::CompositeAlphaMode::Auto),
            view_formats: vec![],
        };
        surface.configure(&gpu.device, &config);
        Self::build(
            gpu,
            Some((surface, config)),
            format,
            width,
            height,
            settings,
        )
    }

    /// Creates an offscreen renderer (automated screenshots, tests).
    pub fn new_headless(width: u32, height: u32, settings: RenderSettings) -> anyhow::Result<Self> {
        let (_instance, adapter) = gpu::headless_adapter()?;
        let gpu = gpu::create_device(&adapter)?;
        Self::build(
            gpu,
            None,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            width,
            height,
            settings,
        )
    }

    fn build(
        gpu: GpuContext,
        surface: Option<(wgpu::Surface<'static>, wgpu::SurfaceConfiguration)>,
        output_format: wgpu::TextureFormat,
        width: u32,
        height: u32,
        settings: RenderSettings,
    ) -> anyhow::Result<Self> {
        let d = &gpu.device;
        let q = &gpu.queue;
        let scope = d.push_error_scope(wgpu::ErrorFilter::Validation);
        let layouts = Layouts::new(d);
        let pipes = Pipelines::new(
            d,
            &layouts,
            output_format,
            gpu.features.contains(wgpu::Features::POLYGON_MODE_LINE),
        );
        let sampler =
            |label, addr, filter: wgpu::FilterMode, mip: wgpu::MipmapFilterMode, aniso, cmp| {
                d.create_sampler(&wgpu::SamplerDescriptor {
                    label: Some(label),
                    address_mode_u: addr,
                    address_mode_v: addr,
                    address_mode_w: addr,
                    mag_filter: filter,
                    min_filter: filter,
                    mipmap_filter: mip,
                    lod_min_clamp: 0.0,
                    lod_max_clamp: 32.0,
                    compare: cmp,
                    anisotropy_clamp: aniso,
                    border_color: None,
                })
            };
        use wgpu::{AddressMode as A, FilterMode as Fm, MipmapFilterMode as M};
        let samplers = Samplers {
            linear_clamp: sampler(
                "linear-clamp",
                A::ClampToEdge,
                Fm::Linear,
                M::Linear,
                1,
                None,
            ),
            linear_repeat: sampler("linear-repeat", A::Repeat, Fm::Linear, M::Linear, 1, None),
            point_clamp: sampler(
                "point-clamp",
                A::ClampToEdge,
                Fm::Nearest,
                M::Nearest,
                1,
                None,
            ),
            shadow: sampler(
                "shadow",
                A::ClampToEdge,
                Fm::Linear,
                M::Nearest,
                1,
                Some(wgpu::CompareFunction::LessEqual),
            ),
            material: sampler("material", A::Repeat, Fm::Linear, M::Linear, 16, None),
        };
        use wgpu::BufferUsages as B;
        let frame_buf = gpu::buffer(
            d,
            "frame",
            std::mem::size_of::<FrameUniforms>() as u64,
            B::UNIFORM | B::COPY_DST,
        );
        let ev0 = 12.0f32;
        let exp0 = ExposureGpu {
            pre: 1.0 / (1.2 * 2f32.powf(ev0)),
            current: 1.0 / (1.2 * 2f32.powf(ev0)),
            prev_pre: 1.0 / (1.2 * 2f32.powf(ev0)),
            ev100: ev0,
            avg_lum: 0.0,
            target_ev: ev0,
            pad: [0.0; 2],
        };
        let exposure_buf = gpu::buffer_init(
            d,
            "exposure",
            bytemuck::bytes_of(&exp0),
            B::STORAGE | B::COPY_DST,
        );
        let exposure_params_buf = gpu::buffer(
            d,
            "exposure-params",
            std::mem::size_of::<ExposureParams>() as u64,
            B::UNIFORM | B::COPY_DST,
        );
        let histogram_buf =
            gpu::buffer_init(d, "histogram", &[0u8; 1024], B::STORAGE | B::COPY_DST);
        let shadow_view_buf = gpu::buffer(
            d,
            "shadow-views",
            256 * (SKYOCC_SLOT as u64 + 1),
            B::UNIFORM | B::COPY_DST,
        );
        let skyocc_view = d
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("sky-occlusion"),
                size: wgpu::Extent3d {
                    width: SKYOCC_RES,
                    height: SKYOCC_RES,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: pl::SHADOW_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
            .create_view(&Default::default());
        let frame_bg = gpu::bind_group(
            d,
            "frame",
            &layouts.frame,
            &frame_resources(&frame_buf, &exposure_buf, &samplers, None),
        );
        let shadow_frame_bg = gpu::bind_group(
            d,
            "shadow-frame",
            &layouts.shadow_frame,
            &frame_resources(&frame_buf, &exposure_buf, &samplers, Some(&shadow_view_buf)),
        );
        let empty_bg = gpu::bind_group(d, "empty", &layouts.empty, &[]);
        let sh_buf = gpu::buffer_init(d, "sh", &[0u8; 18 * 16], B::STORAGE | B::COPY_DST);
        let taa_buf = gpu::buffer(d, "taa", 16, B::UNIFORM | B::COPY_DST);
        let ssr_buf = gpu::buffer(d, "ssr", 16, B::UNIFORM | B::COPY_DST);
        let post_buf = gpu::buffer(
            d,
            "post",
            std::mem::size_of::<PostParams>() as u64,
            B::UNIFORM | B::COPY_DST,
        );
        let rain_buf = gpu::buffer(d, "rain", 16, B::UNIFORM | B::COPY_DST);

        let solid = |label: &str, rgba: [u8; 4]| {
            let img = ImageData::rgba8(1, 1, rgba.to_vec(), false);
            let t = crate::texture::upload(d, q, label, &img);
            let v = t.create_view(&Default::default());
            (t, v)
        };
        let (_wt, white) = solid("white", [255, 255, 255, 255]);
        let (_nt, flat_normal) = solid("flat-normal", [128, 128, 255, 255]);

        let sky = Sky::new(
            d,
            q,
            &layouts,
            &frame_bg,
            settings.ibl_resolution.clamp(16, 1024).next_power_of_two(),
        );
        let shadow = ShadowRes::new(d, &settings);
        let st = B::STORAGE;
        let targets = Targets::new(
            d,
            width.max(1),
            height.max(1),
            settings.render_scale.clamp(0.25, 2.0),
            settings.ssao_half_res,
            settings.ssr_half_res,
            output_format,
        );
        let timer = GpuTimer::new(d, q, gpu.features.contains(wgpu::Features::TIMESTAMP_QUERY));
        let mut r = Self {
            surface,
            output_format,
            layouts,
            pipes,
            samplers,
            frame_buf,
            exposure_buf,
            exposure_params_buf,
            histogram_buf,
            shadow_view_buf,
            frame_bg,
            shadow_frame_bg,
            empty_bg,
            sh_buf,
            taa_buf,
            ssr_buf,
            post_buf,
            rain_buf,
            white,
            flat_normal,
            textures: Vec::new(),
            meshes: Vec::new(),
            materials: Vec::new(),
            instances: Vec::new(),
            free_instances: Vec::new(),
            instance_cpu: Vec::new(),
            dirty_instances: Vec::new(),
            instance_buf: GrowBuffer::new(d, "instances", 1 << 16, st),
            visible_buf: GrowBuffer::new(d, "visible", 1 << 14, st),
            light_buf: GrowBuffer::new(d, "lights", 64 * 256, st),
            cluster_buf: GrowBuffer::new(d, "clusters", 4, st),
            spot_mat_buf: GrowBuffer::new(d, "spot-mats", 64 * MAX_SPOT_SHADOWS as u64, st),
            particle_buf: GrowBuffer::new(d, "particles", 64 * 256, st),
            decal_buf: GrowBuffer::new(d, "decals", 208 * 64, st),
            line_buf: GrowBuffer::new(d, "lines", 28 * 1024, B::VERTEX),
            sky,
            hdris: Vec::new(),
            bound_hdri: None,
            shadow,
            targets,
            bgs: None,
            env: Environment::default(),
            lights: Vec::new(),
            particles: Vec::new(),
            decals: Vec::new(),
            dynamic_decals: Vec::new(),
            lines: Vec::new(),
            frame_index: 0,
            history_index: 0,
            history_valid: false,
            prev_view_proj: None,
            prev_jitter: Vec2::ZERO,
            last_render: None,
            sky_key: None,
            sky_ibl_age: 0,
            instant_exposure: 3,
            timer,
            stats: FrameStats::default(),
            skyocc_view,
            skyocc_state: None,
            settings,
            gpu,
        };
        r.ensure_cluster_buffer();
        if let Some(e) = pollster::block_on(scope.pop()) {
            return Err(anyhow!("renderer initialisation failed: {e}"));
        }
        Ok(r)
    }

    // ------------------------------------------------------------------
    // Accessors for egui / app integration.

    pub fn device(&self) -> &wgpu::Device {
        &self.gpu.device
    }
    pub fn queue(&self) -> &wgpu::Queue {
        &self.gpu.queue
    }
    /// Format of the final (swapchain or offscreen) target the overlay callback draws into.
    pub fn surface_format(&self) -> wgpu::TextureFormat {
        self.output_format
    }
    pub fn adapter_info(&self) -> &wgpu::AdapterInfo {
        &self.gpu.adapter_info
    }
    pub fn settings(&self) -> &RenderSettings {
        &self.settings
    }
    pub fn stats(&self) -> FrameStats {
        self.stats.clone()
    }
    pub fn output_size(&self) -> (u32, u32) {
        (self.targets.out_w, self.targets.out_h)
    }

    /// Applies new settings, recreating targets/shadow maps as needed.
    pub fn set_settings(&mut self, s: RenderSettings) {
        let d = &self.gpu.device;
        let shadows_changed = s.shadow_resolution != self.settings.shadow_resolution
            || s.shadow_cascades != self.settings.shadow_cascades
            || s.spot_shadows != self.settings.spot_shadows
            || s.spot_shadow_resolution != self.settings.spot_shadow_resolution;
        let targets_changed = (s.render_scale - self.settings.render_scale).abs() > 1e-4
            || s.ssao_half_res != self.settings.ssao_half_res
            || s.ssr_half_res != self.settings.ssr_half_res;
        if shadows_changed {
            self.shadow = ShadowRes::new(d, &s);
        }
        if s.ibl_resolution != self.settings.ibl_resolution {
            self.sky = Sky::new(
                d,
                &self.gpu.queue,
                &self.layouts,
                &self.frame_bg,
                s.ibl_resolution.clamp(16, 1024).next_power_of_two(),
            );
            self.sky_key = None;
        }
        let vsync_changed = s.vsync != self.settings.vsync;
        self.settings = s;
        if targets_changed {
            self.recreate_targets(self.targets.out_w, self.targets.out_h);
        }
        if vsync_changed {
            if let Some((surf, cfg)) = self.surface.as_mut() {
                cfg.present_mode = if self.settings.vsync {
                    wgpu::PresentMode::AutoVsync
                } else {
                    wgpu::PresentMode::AutoNoVsync
                };
                surf.configure(&self.gpu.device, cfg);
            }
        }
        self.ensure_cluster_buffer();
        self.bgs = None;
    }

    pub fn resize(&mut self, w: u32, h: u32) {
        let (w, h) = (w.max(1), h.max(1));
        if let Some((surf, cfg)) = self.surface.as_mut() {
            cfg.width = w;
            cfg.height = h;
            surf.configure(&self.gpu.device, cfg);
        }
        self.recreate_targets(w, h);
    }

    fn recreate_targets(&mut self, w: u32, h: u32) {
        let s = &self.settings;
        self.targets = Targets::new(
            &self.gpu.device,
            w,
            h,
            s.render_scale.clamp(0.25, 2.0),
            s.ssao_half_res,
            s.ssr_half_res,
            self.output_format,
        );
        self.bgs = None;
        self.history_valid = false;
    }

    /// Forgets temporal history (TAA, exposure adaptation, motion vectors).
    pub fn reset_history(&mut self) {
        self.history_valid = false;
        self.prev_view_proj = None;
        self.instant_exposure = 3;
    }

    fn ensure_cluster_buffer(&mut self) {
        let per = self.settings.max_lights_per_cluster.clamp(8, 512) + 1;
        let size = (CLUSTER_X * CLUSTER_Y * CLUSTER_Z * per * 4) as u64;
        if self.cluster_buf.buffer.size() != size {
            let gen = self.cluster_buf.generation + 1;
            self.cluster_buf = GrowBuffer::new(
                &self.gpu.device,
                "clusters",
                size,
                wgpu::BufferUsages::STORAGE,
            );
            self.cluster_buf.generation = gen;
        }
    }

    // ------------------------------------------------------------------
    // Resources

    pub fn create_texture(&mut self, img: &ImageData) -> TextureId {
        let t = crate::texture::upload(&self.gpu.device, &self.gpu.queue, "texture", img);
        let v = t.create_view(&Default::default());
        self.textures.push((t, v));
        TextureId(self.textures.len() as u32 - 1)
    }

    pub fn create_mesh(&mut self, mesh: &MeshData) -> MeshId {
        let mut m = mesh.clone();
        let n = m.positions.len();
        if m.normals.len() != n {
            m.compute_normals();
        }
        if m.uvs.len() != n {
            m.uvs = vec![Vec2::ZERO; n];
        }
        if m.tangents.len() != n {
            m.compute_tangents();
        }
        let mut v0: Vec<f32> = Vec::with_capacity(n * 6);
        let mut v1: Vec<f32> = Vec::with_capacity(n * 12);
        for i in 0..n {
            v0.extend_from_slice(&m.positions[i].to_array());
            v0.extend_from_slice(&m.normals[i].to_array());
            v1.extend_from_slice(&m.tangents[i].to_array());
            v1.extend_from_slice(&m.uvs[i].to_array());
            let uv2 = m.uvs2.as_ref().map(|u| u[i]).unwrap_or(m.uvs[i]);
            v1.extend_from_slice(&uv2.to_array());
            let c = m.colors.as_ref().map(|c| c[i]).unwrap_or(Vec4::ONE);
            v1.extend_from_slice(&c.to_array());
        }
        let d = &self.gpu.device;
        use wgpu::BufferUsages as B;
        let (lo, hi) = m.bounds();
        let gpu_mesh = MeshGpu {
            vb0: gpu::buffer_init(
                d,
                "mesh-vb0",
                bytemuck::cast_slice(&v0),
                B::VERTEX | B::COPY_DST,
            ),
            vb1: gpu::buffer_init(d, "mesh-vb1", bytemuck::cast_slice(&v1), B::VERTEX),
            ib: gpu::buffer_init(d, "mesh-ib", bytemuck::cast_slice(&m.indices), B::INDEX),
            index_count: m.indices.len() as u32,
            vertex_count: n as u32,
            lo,
            hi,
        };
        self.meshes.push(Some(gpu_mesh));
        MeshId(self.meshes.len() as u32 - 1)
    }

    /// Replaces positions and normals of a mesh (e.g. car body deformation). Counts must match.
    pub fn update_mesh_vertices(&mut self, id: MeshId, positions: &[Vec3], normals: &[Vec3]) {
        let Some(Some(m)) = self.meshes.get_mut(id.0 as usize) else {
            return;
        };
        let n = (m.vertex_count as usize)
            .min(positions.len())
            .min(normals.len());
        let mut v0: Vec<f32> = Vec::with_capacity(n * 6);
        let mut lo = Vec3::splat(f32::MAX);
        let mut hi = Vec3::splat(f32::MIN);
        for i in 0..n {
            v0.extend_from_slice(&positions[i].to_array());
            v0.extend_from_slice(&normals[i].to_array());
            lo = lo.min(positions[i]);
            hi = hi.max(positions[i]);
        }
        self.gpu
            .queue
            .write_buffer(&m.vb0, 0, bytemuck::cast_slice(&v0));
        if n > 0 {
            m.lo = m.lo.min(lo);
            m.hi = m.hi.max(hi);
        }
        // Refresh world bounds of instances using this mesh.
        let (mlo, mhi) = (m.lo, m.hi);
        for inst in self.instances.iter_mut().flatten() {
            if inst.mesh == id {
                let (a, b) = transform_aabb(inst.transform, mlo, mhi);
                inst.lo = a;
                inst.hi = b;
            }
        }
    }

    fn material_gpu(&self, m: &Material) -> MaterialGpu {
        let mut mask = 0u32;
        let bits = [
            m.albedo_tex,
            m.normal_tex,
            m.roughness_tex,
            m.metallic_tex,
            m.ao_tex,
            m.emissive_tex,
        ];
        for (i, b) in bits.iter().enumerate() {
            if b.is_some_and(|t| (t.0 as usize) < self.textures.len()) {
                mask |= 1 << i;
            }
        }
        let uv_mode = match m.uv_mode {
            UvMode::Mesh => 0,
            UvMode::Mesh2 => 1,
            UvMode::Triplanar => 2,
            UvMode::WorldXZ => 3,
        };
        let (alpha_mode, cutoff) = match m.alpha_mode {
            AlphaMode::Opaque => (0, 0.0),
            AlphaMode::Mask(c) => (1, c),
            AlphaMode::Blend => (2, 0.0),
        };
        MaterialGpu {
            base_color: m.base_color,
            emissive: m.emissive.extend(m.normal_scale),
            pbr: Vec4::new(m.metallic, m.roughness, m.occlusion_strength, cutoff),
            coat: Vec4::new(m.clearcoat, m.clearcoat_roughness, m.flake, m.flake_scale),
            extra: Vec4::new(m.transmission, m.ior, m.wet_factor, m.translucency),
            uv: Vec4::new(m.uv_scale.x, m.uv_scale.y, m.uv_offset.x, m.uv_offset.y),
            flags: [uv_mode, mask, m.packed_metal_rough as u32, alpha_mode],
            extra2: Vec4::new(m.reflectance, m.double_sided as u32 as f32, 0.0, 0.0),
        }
    }

    fn material_bg(&self, m: &Material, buf: &wgpu::Buffer) -> wgpu::BindGroup {
        let tex = |t: Option<TextureId>, def: &'_ wgpu::TextureView| -> wgpu::TextureView {
            match t.and_then(|t| self.textures.get(t.0 as usize)) {
                Some((_, v)) => v.clone(),
                None => def.clone(),
            }
        };
        let views = [
            tex(m.albedo_tex, &self.white),
            tex(m.normal_tex, &self.flat_normal),
            tex(m.roughness_tex, &self.white),
            tex(m.metallic_tex, &self.white),
            tex(m.ao_tex, &self.white),
            tex(m.emissive_tex, &self.white),
        ];
        let mut res = vec![buf.as_entire_binding()];
        for v in &views {
            res.push(wgpu::BindingResource::TextureView(v));
        }
        res.push(wgpu::BindingResource::Sampler(&self.samplers.material));
        gpu::bind_group(&self.gpu.device, "material", &self.layouts.material, &res)
    }

    pub fn create_material(&mut self, m: &Material) -> MaterialId {
        let g = self.material_gpu(m);
        let buf = gpu::buffer_init(
            &self.gpu.device,
            "material",
            bytemuck::bytes_of(&g),
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );
        let bg = self.material_bg(m, &buf);
        self.materials.push(MaterialRec {
            mat: m.clone(),
            buf,
            bg,
        });
        MaterialId(self.materials.len() as u32 - 1)
    }

    pub fn update_material(&mut self, id: MaterialId, m: &Material) {
        let g = self.material_gpu(m);
        let Some(rec) = self.materials.get(id.0 as usize) else {
            return;
        };
        self.gpu
            .queue
            .write_buffer(&rec.buf, 0, bytemuck::bytes_of(&g));
        let tex_changed = {
            let o = &rec.mat;
            o.albedo_tex != m.albedo_tex
                || o.normal_tex != m.normal_tex
                || o.roughness_tex != m.roughness_tex
                || o.metallic_tex != m.metallic_tex
                || o.ao_tex != m.ao_tex
                || o.emissive_tex != m.emissive_tex
        };
        let bg = if tex_changed {
            Some(self.material_bg(m, &rec.buf))
        } else {
            None
        };
        let rec = &mut self.materials[id.0 as usize];
        if let Some(bg) = bg {
            rec.bg = bg;
        }
        rec.mat = m.clone();
    }

    // ------------------------------------------------------------------
    // Instances

    pub fn add_instance(
        &mut self,
        mesh: MeshId,
        material: MaterialId,
        transform: Mat4,
    ) -> InstanceId {
        let (mlo, mhi) = self
            .meshes
            .get(mesh.0 as usize)
            .and_then(|m| m.as_ref())
            .map(|m| (m.lo, m.hi))
            .unwrap_or((Vec3::ZERO, Vec3::ZERO));
        let (lo, hi) = transform_aabb(transform, mlo, mhi);
        let rec = InstanceRec {
            mesh,
            material,
            transform,
            rendered: transform,
            tint: Vec4::ONE,
            emissive: 1.0,
            visible: true,
            lo,
            hi,
        };
        let idx = if let Some(i) = self.free_instances.pop() {
            self.instances[i as usize] = Some(rec);
            i
        } else {
            self.instances.push(Some(rec));
            self.instance_cpu.push(InstanceGpu::zeroed_default());
            self.instances.len() as u32 - 1
        };
        self.dirty_instances.push(idx);
        InstanceId(idx)
    }

    /// Moves an instance; the previous transform is kept for motion vectors.
    pub fn set_instance_transform(&mut self, id: InstanceId, t: Mat4) {
        let meshes = &self.meshes;
        if let Some(Some(rec)) = self.instances.get_mut(id.0 as usize) {
            rec.transform = t;
            if let Some(Some(m)) = meshes.get(rec.mesh.0 as usize) {
                let (lo, hi) = transform_aabb(t, m.lo, m.hi);
                rec.lo = lo;
                rec.hi = hi;
            }
            self.dirty_instances.push(id.0);
        }
    }

    pub fn set_instance_params(
        &mut self,
        id: InstanceId,
        tint: Vec4,
        emissive_scale: f32,
        visible: bool,
    ) {
        if let Some(Some(rec)) = self.instances.get_mut(id.0 as usize) {
            rec.tint = tint;
            rec.emissive = emissive_scale;
            rec.visible = visible;
            self.dirty_instances.push(id.0);
        }
    }

    pub fn set_instance_material(&mut self, id: InstanceId, material: MaterialId) {
        if let Some(Some(rec)) = self.instances.get_mut(id.0 as usize) {
            rec.material = material;
        }
    }

    pub fn remove_instance(&mut self, id: InstanceId) {
        if let Some(slot) = self.instances.get_mut(id.0 as usize) {
            if slot.take().is_some() {
                self.free_instances.push(id.0);
            }
        }
    }

    pub fn instance_count(&self) -> usize {
        self.instances.iter().flatten().count()
    }

    // ------------------------------------------------------------------
    // Per-frame scene inputs

    pub fn set_environment(&mut self, env: &Environment) {
        self.env = *env;
    }

    pub fn environment(&self) -> &Environment {
        &self.env
    }

    /// Uploads an equirectangular HDRI and precomputes its IBL on the GPU.
    pub fn set_hdri(&mut self, hdr: &HdrImage) -> HdriId {
        let res = self
            .settings
            .ibl_resolution
            .clamp(16, 1024)
            .next_power_of_two();
        let (data, w, h) =
            crate::texture::hdr_to_rgba16(&hdr.rgb_f32, hdr.width, hdr.height, res * 8);
        let img = ImageData {
            width: w,
            height: h,
            format: ImageFormat::Rgba16F,
            mips: vec![data],
            srgb: false,
        };
        let tex = crate::texture::upload(&self.gpu.device, &self.gpu.queue, "hdri-equirect", &img);
        let view = tex.create_view(&Default::default());
        let mut enc = self.gpu.device.create_command_encoder(&Default::default());
        let set = self
            .sky
            .hdri_set(&self.gpu.device, &mut enc, &self.frame_bg, &view, res);
        self.gpu.queue.submit([enc.finish()]);
        self.hdris.push(set);
        HdriId(self.hdris.len() as u32 - 1)
    }

    pub fn set_lights(&mut self, lights: &[Light]) {
        self.lights.clear();
        self.lights.extend_from_slice(lights);
    }

    pub fn set_particles(&mut self, p: &[Particle]) {
        self.particles.clear();
        self.particles.extend_from_slice(p);
    }

    pub fn add_decal(&mut self, decal: &Decal) -> DecalId {
        self.decals.push(Some(*decal));
        DecalId(self.decals.len() as u32 - 1)
    }

    pub fn remove_decal(&mut self, id: DecalId) {
        if let Some(d) = self.decals.get_mut(id.0 as usize) {
            *d = None;
        }
    }

    /// Per-frame decals (e.g. fresh skid marks); replaced every call.
    pub fn set_dynamic_decals(&mut self, decals: &[Decal]) {
        self.dynamic_decals.clear();
        self.dynamic_decals.extend_from_slice(decals);
    }

    pub fn debug_line(&mut self, a: Vec3, b: Vec3, c: Vec4) {
        self.lines.push(LineVertex {
            pos: a.to_array(),
            color: c.to_array(),
        });
        self.lines.push(LineVertex {
            pos: b.to_array(),
            color: c.to_array(),
        });
    }

    pub fn debug_box(&mut self, lo: Vec3, hi: Vec3, c: Vec4) {
        let p = |i: u32| {
            Vec3::new(
                if i & 1 != 0 { hi.x } else { lo.x },
                if i & 2 != 0 { hi.y } else { lo.y },
                if i & 4 != 0 { hi.z } else { lo.z },
            )
        };
        for (a, b) in [
            (0, 1),
            (2, 3),
            (4, 5),
            (6, 7),
            (0, 2),
            (1, 3),
            (4, 6),
            (5, 7),
            (0, 4),
            (1, 5),
            (2, 6),
            (3, 7),
        ] {
            self.debug_line(p(a), p(b), c);
        }
    }

    // ------------------------------------------------------------------
    // Bind groups

    fn bind_groups_key(&self) -> u64 {
        let mut k = self.instance_buf.generation;
        for g in [
            self.visible_buf.generation,
            self.light_buf.generation,
            self.cluster_buf.generation,
            self.spot_mat_buf.generation,
            self.particle_buf.generation,
            self.decal_buf.generation,
        ] {
            k = k.wrapping_mul(1_000_003).wrapping_add(g);
        }
        k.wrapping_mul(31)
            .wrapping_add(self.bound_hdri.map(|h| h as u64 + 1).unwrap_or(0))
    }

    fn rebuild_bind_groups(&mut self) {
        let d = &self.gpu.device;
        let l = &self.layouts;
        let t = &self.targets;
        use wgpu::BindingResource as R;
        let tv = R::TextureView;
        let instances = gpu::bind_group(
            d,
            "instances",
            &l.instances,
            &[
                self.instance_buf.buffer.as_entire_binding(),
                self.visible_buf.buffer.as_entire_binding(),
            ],
        );
        let hdri_view = match self.bound_hdri {
            Some(i) => &self.hdris[i as usize].spec_view,
            None => &self.sky.sky_ibl.spec_view,
        };
        let lighting = gpu::bind_group(
            d,
            "lighting",
            &l.lighting,
            &[
                tv(&self.shadow.cascade_view),
                tv(&self.shadow.spot_view),
                tv(&self.sky.sky_ibl.spec_view),
                tv(hdri_view),
                tv(&self.sky.brdf_lut_view),
                tv(&self.sky.transmittance_view),
                tv(&self.sky.skyview_view),
                tv(&t.ao.view),
                tv(&t.ssr.view),
                self.light_buf.buffer.as_entire_binding(),
                self.cluster_buf.buffer.as_entire_binding(),
                self.sh_buf.as_entire_binding(),
                self.spot_mat_buf.buffer.as_entire_binding(),
                tv(&self.skyocc_view),
            ],
        );
        let gbuf_read = gpu::bind_group(
            d,
            "gbuf-read",
            &l.gbuf_read,
            &[
                tv(&t.depth.view),
                tv(&t.gb0.view),
                tv(&t.gb1.view),
                tv(&t.gb2.view),
                tv(&t.gb3.view),
            ],
        );
        let decal = gpu::bind_group(
            d,
            "decal",
            &l.decal,
            &[self.decal_buf.buffer.as_entire_binding(), tv(&t.depth.view)],
        );
        let gtao = gpu::bind_group(
            d,
            "gtao",
            &l.ssao,
            &[tv(&t.depth.view), tv(&t.gb1.view), tv(&t.ao.view)],
        );
        let ao_blur = gpu::bind_group(
            d,
            "ao-blur",
            &l.ssao,
            &[tv(&t.depth.view), tv(&t.gb1.view), tv(&t.ao_raw.view)],
        );
        let ssr = [0, 1].map(|i| {
            gpu::bind_group(
                d,
                "ssr",
                &l.ssr,
                &[
                    tv(&t.depth.view),
                    tv(&t.gb1.view),
                    tv(&t.gb2.view),
                    tv(&t.gb3.view),
                    tv(&t.history[i].view),
                    self.ssr_buf.as_entire_binding(),
                ],
            )
        });
        let particles = gpu::bind_group(
            d,
            "particles",
            &l.particles,
            &[
                self.particle_buf.buffer.as_entire_binding(),
                tv(&t.depth_copy.view),
                self.rain_buf.as_entire_binding(),
            ],
        );
        // taa[i] writes history[i] and reads history[1-i].
        let taa = [0, 1].map(|i| {
            gpu::bind_group(
                d,
                "taa",
                &l.taa,
                &[
                    tv(&t.hdr.view),
                    tv(&t.depth.view),
                    tv(&t.gb3.view),
                    tv(&t.history[1 - i].view),
                    self.taa_buf.as_entire_binding(),
                ],
            )
        });
        let bloom_first =
            [0, 1].map(|i| gpu::bind_group(d, "bloom0", &l.single_tex, &[tv(&t.history[i].view)]));
        let bloom_mips = t
            .bloom_views
            .iter()
            .map(|v| gpu::bind_group(d, "bloom-mip", &l.single_tex, &[tv(v)]))
            .collect();
        let post = [0, 1].map(|i| {
            gpu::bind_group(
                d,
                "post",
                &l.post,
                &[
                    tv(&t.history[i].view),
                    tv(&t.bloom_views[0]),
                    tv(&t.gb3.view),
                    self.post_buf.as_entire_binding(),
                    tv(&self.white),
                ],
            )
        });
        let fxaa = gpu::bind_group(
            d,
            "fxaa",
            &l.post,
            &[
                tv(&t.history[0].view),
                tv(&t.bloom_views[0]),
                tv(&t.gb3.view),
                self.post_buf.as_entire_binding(),
                tv(&t.ldr.view),
            ],
        );
        let exposure = [0, 1].map(|i| {
            gpu::bind_group(
                d,
                "exposure",
                &l.exposure,
                &[
                    self.exposure_buf.as_entire_binding(),
                    self.histogram_buf.as_entire_binding(),
                    self.exposure_params_buf.as_entire_binding(),
                    tv(&t.history[i].view),
                ],
            )
        });
        let depth = gpu::bind_group(d, "depth", &l.depth_only, &[tv(&t.depth.view)]);
        let cluster = gpu::bind_group(
            d,
            "cluster",
            &l.cluster,
            &[
                self.light_buf.buffer.as_entire_binding(),
                self.cluster_buf.buffer.as_entire_binding(),
            ],
        );
        let key = self.bind_groups_key();
        self.bgs = Some(BindGroups {
            instances,
            lighting,
            gbuf_read,
            decal,
            gtao,
            ao_blur,
            ssr,
            particles,
            taa,
            bloom_first,
            bloom_mips,
            post,
            fxaa,
            exposure,
            depth,
            cluster,
            key,
        });
    }

    // ------------------------------------------------------------------
    // Frame

    /// Renders a frame. For windowed renderers the result is presented; the
    /// optional overlay callback can record extra passes (egui, HUD) onto the
    /// final target view after post-processing.
    #[allow(clippy::type_complexity)]
    pub fn render(
        &mut self,
        camera: &Camera,
        overlay: Option<
            &mut dyn FnMut(
                &wgpu::Device,
                &wgpu::Queue,
                &mut wgpu::CommandEncoder,
                &wgpu::TextureView,
            ),
        >,
    ) -> anyhow::Result<()> {
        if self.surface.is_some() {
            let frame = {
                let (surf, cfg) = self.surface.as_ref().unwrap();
                match surf.get_current_texture() {
                    wgpu::CurrentSurfaceTexture::Success(t)
                    | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
                    wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                        surf.configure(&self.gpu.device, cfg);
                        return Ok(());
                    }
                    _ => return Ok(()),
                }
            };
            let view = frame.texture.create_view(&Default::default());
            self.render_to(camera, &view, overlay)?;
            self.gpu.queue.present(frame);
        } else {
            let view = self.targets.final_offscreen.view.clone();
            self.render_to(camera, &view, overlay)?;
        }
        Ok(())
    }

    /// Renders (with warm-up frames for TAA/exposure convergence) and saves the
    /// image (PNG or JPEG, chosen by extension). Works for headless renderers.
    pub fn capture_png(&mut self, camera: &Camera, path: &Path) -> anyhow::Result<()> {
        let (w, h, rgba) = self.capture_rgba8(camera)?;
        let img = image::RgbaImage::from_raw(w, h, rgba).context("image buffer")?;
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("png")
            .to_ascii_lowercase();
        if ext == "jpg" || ext == "jpeg" {
            let rgb = image::DynamicImage::ImageRgba8(img).to_rgb8();
            let f = std::fs::File::create(path)?;
            let mut enc =
                image::codecs::jpeg::JpegEncoder::new_with_quality(std::io::BufWriter::new(f), 90);
            enc.encode_image(&rgb)?;
        } else {
            img.save(path)?;
        }
        Ok(())
    }

    /// Renders warm-up frames plus one final frame offscreen and returns RGBA8 pixels.
    pub fn capture_rgba8(&mut self, camera: &Camera) -> anyhow::Result<(u32, u32, Vec<u8>)> {
        let warm = self.settings.capture_warmup_frames.max(1);
        let mut cam = *camera;
        for i in 0..warm {
            if i + 2 >= warm {
                self.instant_exposure = 0;
            } else {
                self.instant_exposure = self.instant_exposure.max(1);
            }
            let view = self.targets.final_offscreen.view.clone();
            self.render_to(&cam, &view, None)?;
            cam.cut = false;
        }
        self.read_final()
    }

    fn read_final(&mut self) -> anyhow::Result<(u32, u32, Vec<u8>)> {
        let d = &self.gpu.device;
        let (w, h) = (self.targets.out_w, self.targets.out_h);
        let bpr = (w * 4).next_multiple_of(256);
        let buf = gpu::buffer(
            d,
            "readback",
            (bpr * h) as u64,
            wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        );
        let mut enc = d.create_command_encoder(&Default::default());
        enc.copy_texture_to_buffer(
            self.targets.final_offscreen.texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buf,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bpr),
                    rows_per_image: Some(h),
                },
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        self.gpu.queue.submit([enc.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        buf.map_async(wgpu::MapMode::Read, .., move |r| {
            let _ = tx.send(r);
        });
        d.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        })
        .map_err(|e| anyhow!("poll: {e}"))?;
        rx.recv()?.map_err(|e| anyhow!("map: {e}"))?;
        let mut out = Vec::with_capacity((w * h * 4) as usize);
        {
            let view = buf.get_mapped_range(..).map_err(|e| anyhow!("{e:?}"))?;
            let bgra = matches!(
                self.targets.final_offscreen.texture.format(),
                wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
            );
            for y in 0..h {
                let row = &view[(y * bpr) as usize..(y * bpr + w * 4) as usize];
                if bgra {
                    for px in row.chunks_exact(4) {
                        out.extend_from_slice(&[px[2], px[1], px[0], px[3]]);
                    }
                } else {
                    out.extend_from_slice(row);
                }
            }
        }
        buf.unmap();
        Ok((w, h, out))
    }

    /// Blocks until the GPU has finished all submitted work.
    pub fn wait_idle(&self) {
        let _ = self.gpu.device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        });
    }

    fn upload_instances(&mut self) {
        if self.dirty_instances.is_empty() {
            return;
        }
        let mut dirty = std::mem::take(&mut self.dirty_instances);
        dirty.sort_unstable();
        dirty.dedup();
        let mut next_dirty = Vec::new();
        for &i in &dirty {
            let Some(Some(rec)) = self.instances.get_mut(i as usize) else {
                continue;
            };
            let nm = Mat3::from_mat4(rec.transform).inverse().transpose();
            let tint = if rec.visible { rec.tint } else { Vec4::ZERO };
            self.instance_cpu[i as usize] = InstanceGpu {
                model: rec.transform,
                prev_model: rec.rendered,
                normal: [
                    nm.x_axis.extend(0.0),
                    nm.y_axis.extend(0.0),
                    nm.z_axis.extend(0.0),
                ],
                tint,
                params: Vec4::new(rec.emissive, 0.0, 0.0, 0.0),
            };
            if rec.rendered != rec.transform {
                next_dirty.push(i);
            }
            rec.rendered = rec.transform;
        }
        let total = self.instance_cpu.len();
        let d = &self.gpu.device;
        let q = &self.gpu.queue;
        let grew = self
            .instance_buf
            .reserve(d, (total * std::mem::size_of::<InstanceGpu>()) as u64);
        if grew || dirty.len() * 4 > total {
            q.write_buffer(
                &self.instance_buf.buffer,
                0,
                bytemuck::cast_slice(&self.instance_cpu),
            );
        } else {
            // Coalesce runs of consecutive dirty instances.
            let sz = std::mem::size_of::<InstanceGpu>();
            let mut k = 0;
            while k < dirty.len() {
                let start = dirty[k] as usize;
                let mut end = start;
                while k + 1 < dirty.len() && dirty[k + 1] as usize == end + 1 {
                    k += 1;
                    end += 1;
                }
                q.write_buffer(
                    &self.instance_buf.buffer,
                    (start * sz) as u64,
                    bytemuck::cast_slice(&self.instance_cpu[start..=end]),
                );
                k += 1;
            }
        }
        self.dirty_instances = next_dirty;
    }

    /// Culls all instances against a frustum and appends sorted batches.
    fn cull(
        &self,
        frustum: &Frustum,
        shadow: bool,
        visible: &mut Vec<u32>,
        out: &mut Vec<Draw>,
        transparent: Option<(&mut Vec<(f32, u32)>, Vec3)>,
    ) {
        let mut keys: Vec<(u64, u32)> = Vec::new();
        let mut transp = transparent;
        for (i, rec) in self.instances.iter().enumerate() {
            let Some(rec) = rec else { continue };
            if !rec.visible {
                continue;
            }
            let Some(m) = self.materials.get(rec.material.0 as usize) else {
                continue;
            };
            let mat = &m.mat;
            if shadow && (!mat.cast_shadows || mat.is_forward()) {
                continue;
            }
            if !frustum.aabb(rec.lo, rec.hi) {
                continue;
            }
            if mat.is_forward() {
                if let Some((list, cam)) = transp.as_mut() {
                    let c = (rec.lo + rec.hi) * 0.5;
                    list.push(((c - *cam).length_squared(), i as u32));
                }
                continue;
            }
            let mask = matches!(mat.alpha_mode, AlphaMode::Mask(_));
            let variant = if shadow {
                mask as usize
            } else {
                gbuffer_variant(mat.double_sided, mask)
            } as u64;
            let key = (variant << 56) | ((rec.material.0 as u64) << 28) | rec.mesh.0 as u64;
            keys.push((key, i as u32));
        }
        keys.sort_unstable();
        let mut k = 0;
        while k < keys.len() {
            let key = keys[k].0;
            let first = visible.len() as u32;
            while k < keys.len() && keys[k].0 == key {
                visible.push(keys[k].1);
                k += 1;
            }
            out.push(Draw {
                variant: (key >> 56) as u8,
                material: ((key >> 28) & 0x0fff_ffff) as u32,
                mesh: (key & 0x0fff_ffff) as u32,
                first,
                count: visible.len() as u32 - first,
            });
        }
    }

    fn draw_list(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        draws: &[Draw],
        stats: &mut (u32, u32, u64),
    ) {
        let mut last_mat = u32::MAX;
        let mut last_mesh = u32::MAX;
        for dr in draws {
            let Some(Some(mesh)) = self.meshes.get(dr.mesh as usize) else {
                continue;
            };
            if dr.material != last_mat {
                pass.set_bind_group(2, &self.materials[dr.material as usize].bg, &[]);
                last_mat = dr.material;
            }
            if dr.mesh != last_mesh {
                pass.set_vertex_buffer(0, mesh.vb0.slice(..));
                pass.set_vertex_buffer(1, mesh.vb1.slice(..));
                pass.set_index_buffer(mesh.ib.slice(..), wgpu::IndexFormat::Uint32);
                last_mesh = dr.mesh;
            }
            pass.draw_indexed(0..mesh.index_count, 0, dr.first..dr.first + dr.count);
            stats.0 += 1;
            stats.1 += dr.count;
            stats.2 += (mesh.index_count / 3) as u64 * dr.count as u64;
        }
    }

    fn sky_needs_update(&mut self) -> bool {
        let e = &self.env;
        let key = [
            e.sun_direction.x,
            e.sun_direction.y,
            e.sun_direction.z,
            e.moon_direction.y,
            e.cloud_cover,
            e.turbidity,
            e.sky_intensity,
            e.sun_illuminance,
            e.night_glow.x + e.night_glow.y + e.night_glow.z,
            e.moon_illuminance,
            e.ground_albedo,
            e.stars,
        ];
        self.sky_ibl_age += 1;
        let changed = match &self.sky_key {
            None => true,
            Some(k) => {
                let dir_change = Vec3::new(k[0], k[1], k[2]).dot(e.sun_direction) < 0.99996;
                let other = k[3..]
                    .iter()
                    .zip(&key[3..])
                    .any(|(a, b)| (a - b).abs() > 0.002 * a.abs().max(b.abs()).max(1e-3));
                dir_change || other || self.sky_ibl_age > 600
            }
        };
        if changed {
            if self
                .sky_key
                .is_none_or(|k| k[5] != key[5] || k[10] != key[10])
            {
                self.sky.luts_valid = false;
            }
            self.sky_key = Some(key);
            self.sky_ibl_age = 0;
        }
        changed
    }

    #[allow(clippy::type_complexity)]
    fn render_to(
        &mut self,
        camera: &Camera,
        target_view: &wgpu::TextureView,
        overlay: Option<
            &mut dyn FnMut(
                &wgpu::Device,
                &wgpu::Queue,
                &mut wgpu::CommandEncoder,
                &wgpu::TextureView,
            ),
        >,
    ) -> anyhow::Result<()> {
        let cpu_start = Instant::now();
        let now = Instant::now();
        let dt = self
            .last_render
            .map(|t| (now - t).as_secs_f32())
            .unwrap_or(1.0 / 60.0)
            .clamp(1e-4, 0.25);
        self.last_render = Some(now);
        if camera.cut {
            self.reset_history();
        }
        if let Some(t) = self.timer.as_mut() {
            t.enabled = self.settings.gpu_timing;
            if let Some(total) = t.collect() {
                self.stats.gpu_total_ms = total;
                self.stats.gpu_pass_ms = t.results.clone();
            }
            t.begin_frame();
        }
        let _ = self.gpu.device.poll(wgpu::PollType::Poll);

        // ---------------- CPU preparation ----------------
        let s = self.settings.clone();
        let env = self.env;
        let (rw, rh) = (self.targets.render_w, self.targets.render_h);
        let (ow, oh) = (self.targets.out_w, self.targets.out_h);
        let aspect = rw as f32 / rh as f32;
        let near = camera.near.max(0.01);
        let view = camera.view_matrix();
        let proj_nj = mat::perspective_infinite_reverse(camera.fov_y, aspect, near);
        let taa_on = s.anti_aliasing == AntiAliasing::Taa;
        let jitter = if taa_on {
            let i = (self.frame_index % 8) as u32 + 1;
            Vec2::new(
                (halton(i, 2) - 0.5) * 2.0 / rw as f32,
                (halton(i, 3) - 0.5) * 2.0 / rh as f32,
            )
        } else {
            Vec2::ZERO
        };
        let proj = Mat4::from_translation(jitter.extend(0.0)) * proj_nj;
        let view_proj = proj * view;
        let view_proj_nj = proj_nj * view;
        let prev_vp = self.prev_view_proj.unwrap_or(view_proj_nj);

        self.upload_instances();

        // Primary directional light: the sun while it is up, otherwise the moon.
        let sun_dir = env.sun_direction.normalize_or(Vec3::Y);
        let moon_dir = env.moon_direction.normalize_or(Vec3::Y);
        let moon_primary = sun_dir.y < -0.02 && moon_dir.y > 0.0;
        let shadow_dir = if moon_primary { moon_dir } else { sun_dir };
        let shadows_on = s.shadow_cascades > 0
            && shadow_dir.y > 0.0
            && (if moon_primary {
                env.moon_illuminance
            } else {
                env.sun_illuminance
            }) > 0.0;
        let cascades = if shadows_on {
            fit_cascades(
                camera.position,
                camera.forward.normalize(),
                camera.fov_y,
                aspect,
                near,
                s.shadow_distance,
                s.shadow_cascades,
                self.shadow.res,
                shadow_dir,
            )
        } else {
            Vec::new()
        };

        // Lights: frustum cull, sort by importance, choose shadowed and volumetric lights.
        let cam_frustum = Frustum::from_matrix(view_proj_nj, false);
        let mut cand: Vec<(f32, usize)> = self
            .lights
            .iter()
            .enumerate()
            .filter(|(_, l)| {
                l.intensity > 0.0 && l.range > 0.0 && cam_frustum.sphere(l.position, l.range)
            })
            .map(|(i, l)| {
                let d = (l.position - camera.position).length().max(1.0);
                (-(l.intensity * l.range) / d, i)
            })
            .collect();
        cand.sort_by(|a, b| a.0.total_cmp(&b.0));
        cand.truncate(s.max_lights as usize);
        let mut gpu_lights: Vec<LightGpu> = Vec::with_capacity(cand.len());
        let mut spot_mats: Vec<Mat4> = Vec::new();
        let mut spot_views: Vec<(Mat4, Frustum)> = Vec::new();
        let mut vol: Vec<(f32, u32)> = Vec::new();
        for &(_, li) in &cand {
            let l = self.lights[li];
            let (cos_outer, cos_inner, kind) = match l.kind {
                LightKind::Point => (-2.0, 1.0, 0.0),
                LightKind::Spot {
                    inner_angle,
                    outer_angle,
                } => (
                    outer_angle.cos(),
                    inner_angle.min(outer_angle - 1e-3).cos(),
                    1.0,
                ),
            };
            let mut layer = -1.0;
            if let LightKind::Spot { outer_angle, .. } = l.kind {
                if l.cast_shadows
                    && (spot_views.len() as u32) < s.spot_shadows.min(MAX_SPOT_SHADOWS)
                {
                    let dir = l.direction.normalize_or(Vec3::NEG_Y);
                    let up = if dir.y.abs() > 0.99 { Vec3::Z } else { Vec3::Y };
                    let v = mat::look_to(l.position, dir, up);
                    let p =
                        mat::perspective((outer_angle * 2.0 + 0.1).min(3.0), 1.0, 0.05, l.range);
                    let vp = p * v;
                    layer = spot_views.len() as f32;
                    spot_mats.push(vp);
                    spot_views.push((vp, Frustum::from_matrix(vp, true)));
                }
            }
            let gi = gpu_lights.len() as u32;
            if l.volumetric > 0.0 && s.volumetric_lights > 0 {
                let d = (l.position - camera.position).length().max(3.0);
                vol.push((-(l.intensity * l.volumetric) / (d * d), gi));
            }
            gpu_lights.push(LightGpu {
                pos_range: l.position.extend(l.range),
                color: (l.color * l.intensity).extend(l.source_radius.max(0.01)),
                dir_cos: l.direction.normalize_or(Vec3::NEG_Y).extend(cos_outer),
                params: Vec4::new(cos_inner, layer, l.volumetric, kind),
            });
        }
        vol.sort_by(|a, b| a.0.total_cmp(&b.0));
        vol.truncate(s.volumetric_lights.min(32) as usize);
        let mut vol_lights = [[0u32; 4]; 8];
        for (k, (_, gi)) in vol.iter().enumerate() {
            vol_lights[k / 4][k % 4] = *gi;
        }

        // Height-field sky occlusion: re-render when the camera moved or periodically.
        let mut skyocc_update = false;
        if s.sky_occlusion {
            let snap = 8.0;
            let c = Vec3::new(
                (camera.position.x / snap).round() * snap,
                0.0,
                (camera.position.z / snap).round() * snap,
            );
            let needs = match self.skyocc_state {
                None => true,
                Some((pc, _, age)) => pc != c || age > 30,
            };
            if needs {
                let top = camera.position.y + SKYOCC_RANGE * 0.5;
                let v = mat::look_to(Vec3::new(c.x, top, c.z), Vec3::NEG_Y, Vec3::NEG_Z);
                let h = SKYOCC_SIZE * 0.5;
                let p = mat::orthographic(-h, h, -h, h, 0.0, SKYOCC_RANGE);
                self.skyocc_state = Some((c, p * v, 0));
                skyocc_update = true;
            } else if let Some(st) = self.skyocc_state.as_mut() {
                st.2 += 1;
            }
        }
        let skyocc_vp = self.skyocc_state.map(|s| s.1).unwrap_or(Mat4::IDENTITY);
        let skyocc_top = self
            .skyocc_state
            .map(|s| s.1.inverse().transform_point3(Vec3::ZERO).y)
            .unwrap_or(0.0);

        // Frame uniforms.
        let inv_view = view.inverse();
        let mut cascade_vp = [Mat4::IDENTITY; 4];
        let mut splits = [0.0f32; 4];
        let mut texel = [0.0f32; 4];
        for (i, c) in cascades.iter().enumerate() {
            cascade_vp[i] = c.view_proj;
            splits[i] = c.split_far;
            texel[i] = c.texel;
        }
        let max_cluster = s.max_lights_per_cluster.clamp(8, 512);
        let fu = FrameUniforms {
            view,
            proj,
            view_proj,
            inv_view,
            inv_proj: proj.inverse(),
            inv_view_proj: view_proj.inverse(),
            prev_view_proj: prev_vp,
            view_proj_nj,
            cascade_vp,
            camera_pos: camera.position.extend(env.time),
            viewport: Vec4::new(rw as f32, rh as f32, 1.0 / rw as f32, 1.0 / rh as f32),
            output: Vec4::new(ow as f32, oh as f32, 1.0 / ow as f32, 1.0 / oh as f32),
            jitter: Vec4::new(jitter.x, jitter.y, self.prev_jitter.x, self.prev_jitter.y),
            proj_params: Vec4::new(
                near,
                (camera.fov_y * 0.5).tan(),
                aspect,
                (self.frame_index % 1024) as f32,
            ),
            sun: sun_dir.extend(env.sun_illuminance),
            moon: moon_dir.extend(env.moon_illuminance),
            light_params: Vec4::new(moon_primary as u32 as f32, 0.004_65, 0.004_5 * 1.6, dt),
            sky: Vec4::new(
                env.sky_intensity,
                env.turbidity,
                env.ground_albedo,
                env.cloud_cover.clamp(0.0, 1.0),
            ),
            sky2: Vec4::new(
                env.stars,
                if env.hdri.is_some() {
                    env.hdri_blend.clamp(0.0, 1.0)
                } else {
                    0.0
                },
                env.hdri_rotation,
                env.hdri_intensity,
            ),
            sky3: Vec4::new(
                (env.hdri_background && env.hdri.is_some()) as u32 as f32,
                env.wind.x,
                env.wind.y,
                (camera.position.y / 1000.0).max(0.0005) + 0.0015,
            ),
            night_glow: env.night_glow.extend(0.0),
            fog: Vec4::new(
                env.fog.density,
                env.fog.height_falloff,
                env.fog.base_height,
                env.fog.anisotropy,
            ),
            fog2: Vec4::new(
                env.fog.start,
                env.fog.volumetric_scale,
                vol.len() as f32,
                0.0,
            ),
            weather: Vec4::new(
                env.wetness.clamp(0.0, 1.0),
                env.puddles.clamp(0.0, 1.0),
                env.rain.clamp(0.0, 1.0),
                env.time,
            ),
            cascade_splits: Vec4::from_array(splits),
            shadow: Vec4::new(
                cascades.len() as f32,
                self.shadow.res as f32,
                s.soft_shadows as u32 as f32,
                s.shadow_distance,
            ),
            cascade_texel: Vec4::from_array(texel),
            cluster_dims: [CLUSTER_X, CLUSTER_Y, CLUSTER_Z, max_cluster],
            cluster_z: Vec4::new(
                near.max(0.1),
                CLUSTER_FAR,
                (CLUSTER_FAR / near.max(0.1)).ln(),
                gpu_lights.len() as f32,
            ),
            post: Vec4::new(
                s.ssao as u32 as f32,
                (s.ssr && self.history_valid) as u32 as f32,
                spot_views.len() as f32,
                s.debug_view as u32 as f32,
            ),
            vol_lights,
            skyocc_vp: skyocc_vp,
            skyocc: Vec4::new(
                skyocc_top,
                SKYOCC_RANGE,
                (s.sky_occlusion && self.skyocc_state.is_some()) as u32 as f32,
                1.0,
            ),
            bounce: (Vec3::new(1.0, 0.94, 0.86) * env.ground_albedo).extend(0.0),
        };
        let d = &self.gpu.device;
        let q = &self.gpu.queue;
        q.write_buffer(&self.frame_buf, 0, bytemuck::bytes_of(&fu));

        // Exposure parameters.
        let manual = camera
            .exposure_ev100
            .unwrap_or(if s.auto_exposure { -1000.0 } else { 12.0 });
        let ep = ExposureParams {
            min_ev: s.min_ev100,
            max_ev: s.max_ev100,
            compensation: env.exposure_compensation,
            speed: s.exposure_speed,
            dt,
            manual_ev: manual,
            reset: (self.instant_exposure > 0) as u32 as f32,
            low_pct: 0.45,
            high_pct: 0.95,
            pad: [0.0; 3],
        };
        q.write_buffer(&self.exposure_params_buf, 0, bytemuck::bytes_of(&ep));
        if self.instant_exposure > 0 {
            self.instant_exposure -= 1;
        }

        // Dynamic buffers.
        let light_bytes: &[u8] = if gpu_lights.is_empty() {
            &[0u8; 64]
        } else {
            bytemuck::cast_slice(&gpu_lights)
        };
        self.light_buf.write(d, q, light_bytes);
        if !spot_mats.is_empty() {
            self.spot_mat_buf
                .write(d, q, bytemuck::cast_slice(&spot_mats));
        }
        // Particles: back to front.
        let mut parts: Vec<(f32, ParticleGpu)> = self
            .particles
            .iter()
            .filter(|p| cam_frustum.sphere(p.position, p.size * 2.0))
            .map(|p| {
                let kind = match p.kind {
                    ParticleKind::Smoke => 0.0,
                    ParticleKind::Spark => 1.0,
                    ParticleKind::Debris => 2.0,
                    ParticleKind::Splash => 3.0,
                    ParticleKind::Soft => 4.0,
                };
                let seed = (p.position.x * 12.9898 + p.position.z * 78.233 + p.size * 37.719)
                    .sin()
                    .fract()
                    .abs();
                (
                    (p.position - camera.position).length_squared(),
                    ParticleGpu {
                        pos_size: p.position.extend(p.size),
                        color: p.color,
                        vel_rot: p.velocity.extend(p.rotation),
                        params: Vec4::new(p.emissive, kind, seed, 0.0),
                    },
                )
            })
            .collect();
        parts.sort_by(|a, b| b.0.total_cmp(&a.0));
        let part_gpu: Vec<ParticleGpu> = parts.into_iter().map(|p| p.1).collect();
        if !part_gpu.is_empty() {
            self.particle_buf
                .write(d, q, bytemuck::cast_slice(&part_gpu));
        }
        // Decals.
        let decal_gpu: Vec<DecalGpu> = self
            .decals
            .iter()
            .flatten()
            .chain(self.dynamic_decals.iter())
            .enumerate()
            .map(|(i, dc)| {
                let (kind, dash, gap) = match dc.kind {
                    DecalKind::Paint => (0.0, 0.0, 0.0),
                    DecalKind::Dashed { dash, gap } => (1.0, dash, gap),
                    DecalKind::SkidMark => (2.0, 0.0, 0.0),
                    DecalKind::Stain => (3.0, 0.0, 0.0),
                };
                let len = dc.transform.z_axis.truncate().length();
                let wid = dc.transform.x_axis.truncate().length();
                DecalGpu {
                    model: dc.transform,
                    inv_model: dc.transform.inverse(),
                    color: dc.color,
                    params: Vec4::new(kind, dc.roughness, dc.wear, dash),
                    params2: Vec4::new(gap, (i as f32 * 0.618).fract() * 100.0, len, wid),
                }
            })
            .collect();
        if !decal_gpu.is_empty() {
            self.decal_buf.write(d, q, bytemuck::cast_slice(&decal_gpu));
        }
        let rain_count = (s.rain_streaks as f32 * env.rain.clamp(0.0, 1.0)) as u32;
        q.write_buffer(
            &self.rain_buf,
            0,
            bytemuck::bytes_of(&[rain_count, 0, 0, 0]),
        );
        let lines = std::mem::take(&mut self.lines);
        if !lines.is_empty() {
            self.line_buf.write(d, q, bytemuck::cast_slice(&lines));
        }

        // Culling.
        let mut visible: Vec<u32> = Vec::new();
        let mut main_draws = Vec::new();
        let mut transparent: Vec<(f32, u32)> = Vec::new();
        self.cull(
            &cam_frustum,
            false,
            &mut visible,
            &mut main_draws,
            Some((&mut transparent, camera.position)),
        );
        transparent.sort_by(|a, b| b.0.total_cmp(&a.0));
        let mut fwd_draws = Vec::new();
        for (_, i) in &transparent {
            let rec = self.instances[*i as usize].as_ref().unwrap();
            let first = visible.len() as u32;
            visible.push(*i);
            let ds = self.materials[rec.material.0 as usize].mat.double_sided;
            fwd_draws.push(Draw {
                variant: ds as u8,
                material: rec.material.0,
                mesh: rec.mesh.0,
                first,
                count: 1,
            });
        }
        let mut cascade_draws: Vec<Vec<Draw>> = Vec::new();
        for c in &cascades {
            let mut dl = Vec::new();
            // Extend the cascade frustum towards the light so off-screen casters are kept.
            self.cull(
                &Frustum::from_matrix(c.view_proj, true),
                true,
                &mut visible,
                &mut dl,
                None,
            );
            cascade_draws.push(dl);
        }
        let mut spot_draws: Vec<Vec<Draw>> = Vec::new();
        for (_, f) in &spot_views {
            let mut dl = Vec::new();
            self.cull(f, true, &mut visible, &mut dl, None);
            spot_draws.push(dl);
        }
        let mut skyocc_draws = Vec::new();
        if skyocc_update {
            self.cull(
                &Frustum::from_matrix(skyocc_vp, true),
                true,
                &mut visible,
                &mut skyocc_draws,
                None,
            );
        }
        if visible.is_empty() {
            visible.push(0);
        }
        self.visible_buf.write(d, q, bytemuck::cast_slice(&visible));

        // Shadow view uniforms (256-byte aligned for dynamic offsets).
        let mut sviews: Vec<ShadowViewGpu> = Vec::new();
        for i in 0..4 {
            let vp = cascades
                .get(i)
                .map(|c| c.view_proj)
                .unwrap_or(Mat4::IDENTITY);
            sviews.push(ShadowViewGpu {
                view_proj: vp,
                params: Vec4::ZERO,
                pad: [Vec4::ZERO; 11],
            });
        }
        for (vp, _) in &spot_views {
            sviews.push(ShadowViewGpu {
                view_proj: *vp,
                params: Vec4::ZERO,
                pad: [Vec4::ZERO; 11],
            });
        }
        while sviews.len() < SKYOCC_SLOT as usize {
            sviews.push(ShadowViewGpu {
                view_proj: Mat4::IDENTITY,
                params: Vec4::ZERO,
                pad: [Vec4::ZERO; 11],
            });
        }
        sviews.push(ShadowViewGpu {
            view_proj: skyocc_vp,
            params: Vec4::ZERO,
            pad: [Vec4::ZERO; 11],
        });
        q.write_buffer(&self.shadow_view_buf, 0, bytemuck::cast_slice(&sviews));

        // Small per-pass uniforms.
        let taa = TaaParams {
            blend: 0.1,
            reset: (!self.history_valid) as u32 as f32,
            enabled: taa_on as u32 as f32,
            pad: 0.0,
        };
        q.write_buffer(&self.taa_buf, 0, bytemuck::bytes_of(&taa));
        let ssr = SsrParams {
            steps: s.ssr_steps.clamp(8, 256),
            max_dist: 80.0,
            thickness: 0.35,
            enabled: (s.ssr && self.history_valid) as u32,
        };
        q.write_buffer(&self.ssr_buf, 0, bytemuck::bytes_of(&ssr));
        let bloom_levels = self.targets.bloom_views.len() as f32;
        let fxaa_on = s.anti_aliasing == AntiAliasing::Fxaa;
        let post = PostParams {
            bloom: if s.bloom { s.bloom_intensity } else { 0.0 },
            vignette: s.vignette,
            grain: s.film_grain,
            ca: s.chromatic_aberration,
            motion_blur: s.motion_blur,
            sharpen: if taa_on { s.sharpen } else { 0.0 },
            tonemapper: match s.tonemapper {
                Tonemapper::AgX => 0.0,
                Tonemapper::Aces => 1.0,
                Tonemapper::Neutral => 2.0,
            },
            ldr_out: fxaa_on as u32 as f32,
            bloom_norm: 1.0 / bloom_levels.max(1.0),
            pad: [0.0; 3],
        };
        q.write_buffer(&self.post_buf, 0, bytemuck::bytes_of(&post));

        // HDRI binding.
        let want_hdri = env
            .hdri
            .map(|h| h.0)
            .filter(|h| (*h as usize) < self.hdris.len());
        if want_hdri != self.bound_hdri {
            self.bound_hdri = want_hdri;
            if let Some(h) = want_hdri {
                let mut enc = d.create_command_encoder(&Default::default());
                enc.copy_buffer_to_buffer(
                    &self.hdris[h as usize].sh,
                    0,
                    &self.sh_buf,
                    9 * 16,
                    9 * 16,
                );
                q.submit([enc.finish()]);
            }
        }
        if self
            .bgs
            .as_ref()
            .is_none_or(|b| b.key != self.bind_groups_key())
        {
            self.rebuild_bind_groups();
        }
        let update_ibl = self.sky_needs_update();

        // ---------------- GPU frame graph ----------------
        let d = &self.gpu.device;
        let q = &self.gpu.queue;
        let bgs = self.bgs.as_ref().unwrap();
        let t = &self.targets;
        let p = &self.pipes;
        let hi = self.history_index;
        let mut enc = d.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("frame"),
        });
        let mut timer = self.timer.take();
        let qs = timer.as_ref().map(|t| t.query_set());
        let mut tsr = |name: &'static str| {
            let i = timer.as_mut().and_then(|t| t.slot(name))?;
            Some(wgpu::RenderPassTimestampWrites {
                query_set: qs.as_ref()?,
                beginning_of_pass_write_index: Some(2 * i),
                end_of_pass_write_index: Some(2 * i + 1),
            })
        };
        let mut stats = (0u32, 0u32, 0u64);
        let mut shadow_calls = 0u32;

        enc.clear_buffer(&self.histogram_buf, 0, None);
        {
            let mut cp = enc.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("exposure-begin"),
                timestamp_writes: None,
            });
            cp.set_pipeline(&p.exposure_begin);
            cp.set_bind_group(0, &self.empty_bg, &[]);
            cp.set_bind_group(1, &bgs.exposure[hi], &[]);
            cp.dispatch_workgroups(1, 1, 1);
        }
        self.sky
            .encode(&mut enc, &self.frame_bg, update_ibl, &self.sh_buf);
        {
            let mut cp = enc.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("clusters"),
                timestamp_writes: None,
            });
            cp.set_pipeline(&p.cluster);
            cp.set_bind_group(0, &self.frame_bg, &[]);
            cp.set_bind_group(1, &bgs.cluster, &[]);
            cp.dispatch_workgroups(CLUSTER_X / 4, CLUSTER_Y.div_ceil(4), CLUSTER_Z / 4);
        }

        // Shadows.
        let shadow_layers: Vec<(&wgpu::TextureView, &Vec<Draw>, u32)> = cascade_draws
            .iter()
            .enumerate()
            .map(|(i, dl)| (&self.shadow.cascade_layers[i], dl, i as u32))
            .chain(
                spot_draws
                    .iter()
                    .enumerate()
                    .map(|(i, dl)| (&self.shadow.spot_layers[i], dl, 4 + i as u32)),
            )
            .chain(skyocc_update.then_some((&self.skyocc_view, &skyocc_draws, SKYOCC_SLOT)))
            .collect();
        for (layer_view, dl, slot) in shadow_layers {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("shadow"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: layer_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: if slot == 0 { tsr("shadow-c0") } else { None },
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_bind_group(0, &self.shadow_frame_bg, &[slot * 256]);
            pass.set_bind_group(1, &bgs.instances, &[]);
            let mut cur_variant = u8::MAX;
            let mut st = (0, 0, 0);
            for dr in dl.iter() {
                if dr.variant != cur_variant {
                    pass.set_pipeline(&p.shadow[dr.variant as usize]);
                    cur_variant = dr.variant;
                }
                self.draw_list(&mut pass, std::slice::from_ref(dr), &mut st);
            }
            shadow_calls += st.0;
        }

        // G-buffer.
        {
            let clear0 = Some(wgpu::Color::TRANSPARENT);
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("gbuffer"),
                color_attachments: &[
                    gpu::color_attachment(&t.hdr.view, clear0),
                    gpu::color_attachment(&t.gb0.view, clear0),
                    gpu::color_attachment(&t.gb1.view, clear0),
                    gpu::color_attachment(&t.gb2.view, clear0),
                    gpu::color_attachment(&t.gb3.view, clear0),
                ],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &t.depth.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: tsr("gbuffer"),
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_bind_group(0, &self.frame_bg, &[]);
            pass.set_bind_group(1, &bgs.instances, &[]);
            let wire = if s.wireframe {
                p.gbuffer_wire.as_ref()
            } else {
                None
            };
            let mut cur_variant = u8::MAX;
            for dr in &main_draws {
                if dr.variant != cur_variant {
                    pass.set_pipeline(wire.unwrap_or(&p.gbuffer[dr.variant as usize]));
                    cur_variant = dr.variant;
                }
                self.draw_list(&mut pass, std::slice::from_ref(dr), &mut stats);
            }
        }
        // Decals.
        if !decal_gpu.is_empty() {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("decals"),
                color_attachments: &[
                    gpu::color_attachment(&t.gb0.view, None),
                    gpu::color_attachment(&t.gb1.view, None),
                ],
                depth_stencil_attachment: None,
                timestamp_writes: tsr("decals"),
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&p.decal);
            pass.set_bind_group(0, &self.frame_bg, &[]);
            pass.set_bind_group(1, &bgs.decal, &[]);
            pass.draw(0..36, 0..decal_gpu.len() as u32);
            stats.0 += 1;
        }
        gpu::fullscreen_pass(
            &mut enc,
            "copy-depth",
            &p.copy_depth,
            &[&self.frame_bg, &bgs.depth],
            &t.depth_copy.view,
            None,
            None,
        );
        if s.ssao {
            gpu::fullscreen_pass(
                &mut enc,
                "gtao",
                &p.gtao,
                &[&self.frame_bg, &bgs.gtao],
                &t.ao_raw.view,
                None,
                tsr("gtao"),
            );
            gpu::fullscreen_pass(
                &mut enc,
                "ao-blur",
                &p.ao_blur,
                &[&self.frame_bg, &bgs.ao_blur],
                &t.ao.view,
                None,
                None,
            );
        }
        if s.ssr && self.history_valid {
            gpu::fullscreen_pass(
                &mut enc,
                "ssr",
                &p.ssr,
                &[&self.frame_bg, &bgs.ssr[1 - hi]],
                &t.ssr.view,
                Some(wgpu::Color::TRANSPARENT),
                tsr("ssr"),
            );
        }
        // Deferred lighting.
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("lighting"),
                color_attachments: &[gpu::color_attachment(&t.hdr.view, None)],
                depth_stencil_attachment: None,
                timestamp_writes: tsr("lighting"),
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&p.deferred);
            pass.set_bind_group(0, &self.frame_bg, &[]);
            pass.set_bind_group(1, &bgs.gbuf_read, &[]);
            pass.set_bind_group(2, &self.empty_bg, &[]);
            pass.set_bind_group(3, &bgs.lighting, &[]);
            pass.draw(0..3, 0..1);
        }
        // Forward: glass, particles, rain.
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("forward"),
                color_attachments: &[gpu::color_attachment(&t.hdr.view, None)],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &t.depth.view,
                    depth_ops: None,
                    stencil_ops: None,
                }),
                timestamp_writes: tsr("forward"),
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_bind_group(0, &self.frame_bg, &[]);
            if !fwd_draws.is_empty() {
                pass.set_bind_group(1, &bgs.instances, &[]);
                pass.set_bind_group(3, &bgs.lighting, &[]);
                let mut cur = u8::MAX;
                for dr in &fwd_draws {
                    if dr.variant != cur {
                        pass.set_pipeline(&p.forward[dr.variant as usize]);
                        cur = dr.variant;
                    }
                    self.draw_list(&mut pass, std::slice::from_ref(dr), &mut stats);
                }
            }
            if !part_gpu.is_empty() || rain_count > 0 {
                pass.set_bind_group(1, &bgs.particles, &[]);
                pass.set_bind_group(2, &self.empty_bg, &[]);
                pass.set_bind_group(3, &bgs.lighting, &[]);
                if !part_gpu.is_empty() {
                    pass.set_pipeline(&p.particles);
                    pass.draw(0..6, 0..part_gpu.len() as u32);
                    stats.0 += 1;
                }
                if rain_count > 0 {
                    pass.set_pipeline(&p.rain);
                    pass.draw(0..6, 0..rain_count);
                    stats.0 += 1;
                }
            }
        }
        // TAA / upscale into history[hi].
        gpu::fullscreen_pass(
            &mut enc,
            "taa",
            &p.taa,
            &[&self.frame_bg, &bgs.taa[hi]],
            &t.history[hi].view,
            None,
            tsr("taa"),
        );
        // Auto exposure from the resolved image.
        {
            let mut cp = enc.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("exposure"),
                timestamp_writes: None,
            });
            cp.set_bind_group(0, &self.empty_bg, &[]);
            cp.set_bind_group(1, &bgs.exposure[hi], &[]);
            cp.set_pipeline(&p.histogram);
            cp.dispatch_workgroups(ow.div_ceil(32), oh.div_ceil(32), 1);
            cp.set_pipeline(&p.adapt);
            cp.dispatch_workgroups(1, 1, 1);
        }
        // Bloom.
        if s.bloom {
            let n = t.bloom_views.len();
            gpu::fullscreen_pass(
                &mut enc,
                "bloom-down0",
                &p.bloom_down_first,
                &[&self.frame_bg, &bgs.bloom_first[hi]],
                &t.bloom_views[0],
                None,
                tsr("bloom"),
            );
            for m in 1..n {
                gpu::fullscreen_pass(
                    &mut enc,
                    "bloom-down",
                    &p.bloom_down,
                    &[&self.frame_bg, &bgs.bloom_mips[m - 1]],
                    &t.bloom_views[m],
                    None,
                    None,
                );
            }
            for m in (0..n - 1).rev() {
                gpu::fullscreen_pass(
                    &mut enc,
                    "bloom-up",
                    &p.bloom_up,
                    &[&self.frame_bg, &bgs.bloom_mips[m + 1]],
                    &t.bloom_views[m],
                    None,
                    None,
                );
            }
        }
        let _ = BLOOM_MIPS;
        // Tonemap (+ FXAA).
        if fxaa_on {
            gpu::fullscreen_pass(
                &mut enc,
                "tonemap",
                &p.tonemap_ldr,
                &[&self.frame_bg, &bgs.post[hi]],
                &t.ldr.view,
                None,
                tsr("post"),
            );
            gpu::fullscreen_pass(
                &mut enc,
                "fxaa",
                &p.fxaa,
                &[&self.frame_bg, &bgs.fxaa],
                target_view,
                None,
                None,
            );
        } else {
            gpu::fullscreen_pass(
                &mut enc,
                "tonemap",
                &p.tonemap,
                &[&self.frame_bg, &bgs.post[hi]],
                target_view,
                None,
                tsr("post"),
            );
        }
        // Debug lines.
        if !lines.is_empty() {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("debug-lines"),
                color_attachments: &[gpu::color_attachment(target_view, None)],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&p.debug_lines);
            pass.set_bind_group(0, &self.frame_bg, &[]);
            pass.set_bind_group(1, &bgs.depth, &[]);
            pass.set_vertex_buffer(0, self.line_buf.buffer.slice(..));
            pass.draw(0..lines.len() as u32, 0..1);
        }
        drop(tsr);
        if let Some(cb) = overlay {
            cb(d, q, &mut enc, target_view);
        }
        if let Some(t) = timer.as_mut() {
            t.resolve(&mut enc);
        }
        q.submit([enc.finish()]);
        if let Some(t) = timer.as_mut() {
            t.after_submit();
        }
        self.timer = timer;

        // Bookkeeping for the next frame.
        self.prev_view_proj = Some(view_proj_nj);
        self.prev_jitter = jitter;
        self.history_index = 1 - self.history_index;
        self.history_valid = true;
        self.frame_index += 1;
        self.stats.frame = self.frame_index;
        self.stats.draw_calls = stats.0 + shadow_calls;
        self.stats.shadow_draw_calls = shadow_calls;
        self.stats.instances_drawn = stats.1;
        self.stats.triangles = stats.2;
        self.stats.lights = gpu_lights.len() as u32;
        self.stats.particles = part_gpu.len() as u32;
        self.stats.decals = decal_gpu.len() as u32;
        self.stats.render_width = rw;
        self.stats.render_height = rh;
        self.stats.cpu_ms = cpu_start.elapsed().as_secs_f32() * 1000.0;
        Ok(())
    }
}

fn frame_resources<'a>(
    frame: &'a wgpu::Buffer,
    exposure: &'a wgpu::Buffer,
    s: &'a Samplers,
    extra: Option<&'a wgpu::Buffer>,
) -> Vec<wgpu::BindingResource<'a>> {
    let mut r = vec![
        frame.as_entire_binding(),
        exposure.as_entire_binding(),
        wgpu::BindingResource::Sampler(&s.linear_clamp),
        wgpu::BindingResource::Sampler(&s.linear_repeat),
        wgpu::BindingResource::Sampler(&s.point_clamp),
        wgpu::BindingResource::Sampler(&s.shadow),
    ];
    if let Some(b) = extra {
        r.push(wgpu::BindingResource::Buffer(wgpu::BufferBinding {
            buffer: b,
            offset: 0,
            size: wgpu::BufferSize::new(256),
        }));
    }
    r
}

impl InstanceGpu {
    fn zeroed_default() -> Self {
        bytemuck::Zeroable::zeroed()
    }
}

pub(crate) fn transform_aabb(m: Mat4, lo: Vec3, hi: Vec3) -> (Vec3, Vec3) {
    let c = (lo + hi) * 0.5;
    let e = (hi - lo) * 0.5;
    let wc = m.transform_point3(c);
    let a = Mat3::from_mat4(m);
    let we = Vec3::new(
        a.x_axis.x.abs() * e.x + a.y_axis.x.abs() * e.y + a.z_axis.x.abs() * e.z,
        a.x_axis.y.abs() * e.x + a.y_axis.y.abs() * e.y + a.z_axis.y.abs() * e.z,
        a.x_axis.z.abs() * e.x + a.y_axis.z.abs() * e.y + a.z_axis.z.abs() * e.z,
    );
    (wc - we, wc + we)
}
