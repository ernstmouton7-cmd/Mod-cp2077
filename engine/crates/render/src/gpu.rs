//! Device creation and small wgpu helpers that keep pass code compact.

use std::borrow::Cow;

use anyhow::{anyhow, Context};

pub(crate) struct GpuContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub adapter_info: wgpu::AdapterInfo,
    pub features: wgpu::Features,
}

pub(crate) fn wanted_features(adapter: &wgpu::Adapter) -> wgpu::Features {
    let want = wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::POLYGON_MODE_LINE;
    adapter.features() & want
}

pub(crate) fn create_device(adapter: &wgpu::Adapter) -> anyhow::Result<GpuContext> {
    let features = wanted_features(adapter);
    let alimits = adapter.limits();
    let mut limits = wgpu::Limits::default();
    // Use what the adapter offers for the few limits we care about.
    limits.max_storage_buffers_per_shader_stage =
        alimits.max_storage_buffers_per_shader_stage.min(16);
    limits.max_sampled_textures_per_shader_stage =
        alimits.max_sampled_textures_per_shader_stage.min(32);
    limits.max_storage_buffer_binding_size = alimits.max_storage_buffer_binding_size.min(1 << 30);
    limits.max_buffer_size = alimits.max_buffer_size.min(1 << 30);
    limits.max_texture_dimension_2d = alimits.max_texture_dimension_2d.min(16384);
    limits.max_color_attachment_bytes_per_sample =
        alimits.max_color_attachment_bytes_per_sample.max(32);
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("gridlock-render"),
        required_features: features,
        required_limits: limits,
        experimental_features: wgpu::ExperimentalFeatures::disabled(),
        memory_hints: wgpu::MemoryHints::Performance,
        trace: wgpu::Trace::Off,
    }))
    .context("request_device")?;
    device.on_uncaptured_error(std::sync::Arc::new(|e: wgpu::Error| {
        log::error!("wgpu error: {e}");
        eprintln!("wgpu error: {e}");
    }));
    Ok(GpuContext {
        device,
        queue,
        adapter_info: adapter.get_info(),
        features,
    })
}

pub(crate) fn headless_adapter() -> anyhow::Result<(wgpu::Instance, wgpu::Adapter)> {
    let mut desc = wgpu::InstanceDescriptor::new_without_display_handle_from_env();
    if std::env::var("WGPU_BACKEND").is_err() {
        desc.backends = wgpu::Backends::VULKAN | wgpu::Backends::DX12 | wgpu::Backends::METAL;
    }
    let instance = wgpu::Instance::new(desc);
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter: false,
        compatible_surface: None,
        apply_limit_buckets: false,
    }))
    .map_err(|e| anyhow!("no GPU adapter available: {e}"))?;
    Ok((instance, adapter))
}

// ---------------------------------------------------------------------------
// Shader assembly. WGSL has no #include, so shaders are built by concatenating
// modules and substituting `${NAME}` constants.

pub(crate) mod src {
    pub const COMMON: &str = include_str!("shaders/common.wgsl");
    pub const BRDF: &str = include_str!("shaders/brdf.wgsl");
    pub const ATMOSPHERE: &str = include_str!("shaders/atmosphere.wgsl");
    pub const LIGHTING: &str = include_str!("shaders/lighting.wgsl");
    pub const MATERIAL: &str = include_str!("shaders/material.wgsl");
    pub const GBUFFER: &str = include_str!("shaders/gbuffer.wgsl");
    pub const SHADOW: &str = include_str!("shaders/shadow.wgsl");
    pub const SKY_LUTS: &str = include_str!("shaders/sky_luts.wgsl");
    pub const IBL: &str = include_str!("shaders/ibl.wgsl");
    pub const CLUSTER: &str = include_str!("shaders/cluster.wgsl");
    pub const SSAO: &str = include_str!("shaders/ssao.wgsl");
    pub const SSR: &str = include_str!("shaders/ssr.wgsl");
    pub const DEFERRED: &str = include_str!("shaders/deferred.wgsl");
    pub const FORWARD: &str = include_str!("shaders/forward.wgsl");
    pub const PARTICLES: &str = include_str!("shaders/particles.wgsl");
    pub const DECAL: &str = include_str!("shaders/decal.wgsl");
    pub const TAA: &str = include_str!("shaders/taa.wgsl");
    pub const BLOOM: &str = include_str!("shaders/bloom.wgsl");
    pub const EXPOSURE: &str = include_str!("shaders/exposure.wgsl");
    pub const POST: &str = include_str!("shaders/post.wgsl");
    pub const DEBUG: &str = include_str!("shaders/debug.wgsl");
    pub const WET: &str = include_str!("shaders/wet.wgsl");
    pub const UTIL: &str = include_str!("shaders/util.wgsl");
}

pub(crate) fn shader(
    device: &wgpu::Device,
    label: &str,
    modules: &[&str],
    defines: &[(&str, String)],
) -> wgpu::ShaderModule {
    let mut raw = String::new();
    for m in modules {
        raw.push_str(m);
        raw.push('\n');
    }
    for (k, v) in defines {
        raw = raw.replace(&format!("${{{k}}}"), v);
    }
    // `#ifdef_NAME` ... `#endif` sections are kept only when NAME is defined (as a define key).
    let mut s = String::with_capacity(raw.len());
    let mut skipping = false;
    for line in raw.lines() {
        let t = line.trim();
        if let Some(name) = t.strip_prefix("#ifdef_") {
            skipping = !defines.iter().any(|(k, _)| *k == name);
            continue;
        }
        if t == "#endif" {
            skipping = false;
            continue;
        }
        if !skipping {
            s.push_str(line);
            s.push('\n');
        }
    }
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(Cow::Owned(s)),
    })
}

// ---------------------------------------------------------------------------
// Bind group layout helpers.

pub(crate) mod bt {
    use wgpu::*;
    pub fn uniform() -> BindingType {
        BindingType::Buffer {
            ty: BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        }
    }
    pub fn uniform_dyn() -> BindingType {
        BindingType::Buffer {
            ty: BufferBindingType::Uniform,
            has_dynamic_offset: true,
            min_binding_size: None,
        }
    }
    pub fn storage(read_only: bool) -> BindingType {
        BindingType::Buffer {
            ty: BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: None,
        }
    }
    pub fn tex(dim: TextureViewDimension) -> BindingType {
        BindingType::Texture {
            sample_type: TextureSampleType::Float { filterable: true },
            view_dimension: dim,
            multisampled: false,
        }
    }
    pub fn tex2d() -> BindingType {
        tex(TextureViewDimension::D2)
    }
    pub fn tex2d_nf() -> BindingType {
        BindingType::Texture {
            sample_type: TextureSampleType::Float { filterable: false },
            view_dimension: TextureViewDimension::D2,
            multisampled: false,
        }
    }
    pub fn tex_cube() -> BindingType {
        tex(TextureViewDimension::Cube)
    }
    pub fn tex_depth() -> BindingType {
        BindingType::Texture {
            sample_type: TextureSampleType::Depth,
            view_dimension: TextureViewDimension::D2,
            multisampled: false,
        }
    }
    pub fn tex_depth_array() -> BindingType {
        BindingType::Texture {
            sample_type: TextureSampleType::Depth,
            view_dimension: TextureViewDimension::D2Array,
            multisampled: false,
        }
    }
    pub fn sampler() -> BindingType {
        BindingType::Sampler(SamplerBindingType::Filtering)
    }
    pub fn sampler_nf() -> BindingType {
        BindingType::Sampler(SamplerBindingType::NonFiltering)
    }
    pub fn sampler_cmp() -> BindingType {
        BindingType::Sampler(SamplerBindingType::Comparison)
    }
    pub fn storage_tex(format: TextureFormat, dim: TextureViewDimension) -> BindingType {
        BindingType::StorageTexture {
            access: StorageTextureAccess::WriteOnly,
            format,
            view_dimension: dim,
        }
    }
}

pub(crate) fn bgl(
    device: &wgpu::Device,
    label: &str,
    entries: &[wgpu::BindingType],
) -> wgpu::BindGroupLayout {
    let vis =
        wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::COMPUTE;
    bgl_vis(device, label, entries, vis)
}

pub(crate) fn bgl_vis(
    device: &wgpu::Device,
    label: &str,
    entries: &[wgpu::BindingType],
    vis: wgpu::ShaderStages,
) -> wgpu::BindGroupLayout {
    let e: Vec<_> = entries
        .iter()
        .enumerate()
        .map(|(i, ty)| {
            // Storage textures and writable storage buffers are not allowed in vertex shaders.
            let mut v = vis;
            let writable = matches!(
                ty,
                wgpu::BindingType::StorageTexture { .. }
                    | wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        ..
                    }
            );
            if writable {
                v -= wgpu::ShaderStages::VERTEX;
            }
            wgpu::BindGroupLayoutEntry {
                binding: i as u32,
                visibility: v,
                ty: *ty,
                count: None,
            }
        })
        .collect();
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some(label),
        entries: &e,
    })
}

pub(crate) fn bind_group(
    device: &wgpu::Device,
    label: &str,
    layout: &wgpu::BindGroupLayout,
    res: &[wgpu::BindingResource],
) -> wgpu::BindGroup {
    let e: Vec<_> = res
        .iter()
        .enumerate()
        .map(|(i, r)| wgpu::BindGroupEntry {
            binding: i as u32,
            resource: r.clone(),
        })
        .collect();
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(label),
        layout,
        entries: &e,
    })
}

pub(crate) fn pipeline_layout(
    device: &wgpu::Device,
    label: &str,
    layouts: &[&wgpu::BindGroupLayout],
) -> wgpu::PipelineLayout {
    let l: Vec<Option<&wgpu::BindGroupLayout>> = layouts.iter().map(|x| Some(*x)).collect();
    device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(label),
        bind_group_layouts: &l,
        immediate_size: 0,
    })
}

pub(crate) fn compute_pipeline(
    device: &wgpu::Device,
    label: &str,
    module: &wgpu::ShaderModule,
    entry: &str,
    layouts: &[&wgpu::BindGroupLayout],
) -> wgpu::ComputePipeline {
    let layout = pipeline_layout(device, label, layouts);
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some(label),
        layout: Some(&layout),
        module,
        entry_point: Some(entry),
        compilation_options: Default::default(),
        cache: None,
    })
}

/// Fullscreen-triangle pipeline (vertex entry `vs_fullscreen` must exist in the module).
pub(crate) fn fullscreen_pipeline(
    device: &wgpu::Device,
    label: &str,
    module: &wgpu::ShaderModule,
    fs_entry: &str,
    layouts: &[&wgpu::BindGroupLayout],
    targets: &[Option<wgpu::ColorTargetState>],
) -> wgpu::RenderPipeline {
    let layout = pipeline_layout(device, label, layouts);
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module,
            entry_point: Some("vs_fullscreen"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module,
            entry_point: Some(fs_entry),
            compilation_options: Default::default(),
            targets,
        }),
        multiview_mask: None,
        cache: None,
    })
}

pub(crate) fn target(format: wgpu::TextureFormat) -> Option<wgpu::ColorTargetState> {
    Some(wgpu::ColorTargetState {
        format,
        blend: None,
        write_mask: wgpu::ColorWrites::ALL,
    })
}

pub(crate) fn target_blend(
    format: wgpu::TextureFormat,
    blend: wgpu::BlendState,
) -> Option<wgpu::ColorTargetState> {
    Some(wgpu::ColorTargetState {
        format,
        blend: Some(blend),
        write_mask: wgpu::ColorWrites::ALL,
    })
}

pub(crate) const ADDITIVE: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
};

pub(crate) const PREMULTIPLIED: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
        operation: wgpu::BlendOperation::Add,
    },
};

/// `dst = src.rgb + dst.rgb * src.a` — used to apply fog transmittance to emissive already in the target.
pub(crate) const ADD_WITH_TRANSMITTANCE: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::SrcAlpha,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::Zero,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
};

pub(crate) struct RenderTexture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
}

impl RenderTexture {
    pub fn new(
        device: &wgpu::Device,
        label: &str,
        w: u32,
        h: u32,
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
    ) -> Self {
        Self::with_mips(device, label, w, h, 1, format, usage)
    }

    pub fn with_mips(
        device: &wgpu::Device,
        label: &str,
        w: u32,
        h: u32,
        mips: u32,
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
    ) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: w.max(1),
                height: h.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: mips,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        Self { texture, view }
    }

    pub fn mip_view(&self, mip: u32) -> wgpu::TextureView {
        self.texture.create_view(&wgpu::TextureViewDescriptor {
            base_mip_level: mip,
            mip_level_count: Some(1),
            ..Default::default()
        })
    }
}

pub(crate) fn color_attachment(
    view: &wgpu::TextureView,
    clear: Option<wgpu::Color>,
) -> Option<wgpu::RenderPassColorAttachment<'_>> {
    Some(wgpu::RenderPassColorAttachment {
        view,
        depth_slice: None,
        resolve_target: None,
        ops: wgpu::Operations {
            load: match clear {
                Some(c) => wgpu::LoadOp::Clear(c),
                None => wgpu::LoadOp::Load,
            },
            store: wgpu::StoreOp::Store,
        },
    })
}

/// Runs a fullscreen pass with a single colour target.
#[allow(clippy::too_many_arguments)]
pub(crate) fn fullscreen_pass(
    enc: &mut wgpu::CommandEncoder,
    label: &str,
    pipeline: &wgpu::RenderPipeline,
    groups: &[&wgpu::BindGroup],
    target: &wgpu::TextureView,
    clear: Option<wgpu::Color>,
    timestamps: Option<wgpu::RenderPassTimestampWrites>,
) {
    let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some(label),
        color_attachments: &[color_attachment(target, clear)],
        depth_stencil_attachment: None,
        timestamp_writes: timestamps,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    pass.set_pipeline(pipeline);
    for (i, g) in groups.iter().enumerate() {
        pass.set_bind_group(i as u32, *g, &[]);
    }
    pass.draw(0..3, 0..1);
}

pub(crate) fn buffer(
    device: &wgpu::Device,
    label: &str,
    size: u64,
    usage: wgpu::BufferUsages,
) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: size.max(16),
        usage,
        mapped_at_creation: false,
    })
}

pub(crate) fn buffer_init(
    device: &wgpu::Device,
    label: &str,
    data: &[u8],
    usage: wgpu::BufferUsages,
) -> wgpu::Buffer {
    let size = (data.len() as u64).max(16).next_multiple_of(4);
    let b = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size,
        usage,
        mapped_at_creation: true,
    });
    {
        let mut view = b.get_mapped_range_mut(..).expect("mapped at creation");
        view.slice(..data.len()).copy_from_slice(data);
    }
    b.unmap();
    b
}

/// A GPU buffer that grows (doubling) when more space is required.
pub(crate) struct GrowBuffer {
    pub buffer: wgpu::Buffer,
    pub label: &'static str,
    pub usage: wgpu::BufferUsages,
    /// Incremented when the buffer is recreated, so dependent bind groups can be rebuilt.
    pub generation: u64,
}

impl GrowBuffer {
    pub fn new(
        device: &wgpu::Device,
        label: &'static str,
        size: u64,
        usage: wgpu::BufferUsages,
    ) -> Self {
        let usage = usage | wgpu::BufferUsages::COPY_DST;
        Self {
            buffer: buffer(device, label, size, usage),
            label,
            usage,
            generation: 0,
        }
    }

    /// Ensures capacity; returns true when the buffer was recreated.
    pub fn reserve(&mut self, device: &wgpu::Device, size: u64) -> bool {
        if size <= self.buffer.size() {
            return false;
        }
        let new = size.next_power_of_two().max(256);
        self.buffer = buffer(device, self.label, new, self.usage);
        self.generation += 1;
        true
    }

    pub fn write(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, data: &[u8]) -> bool {
        let grew = self.reserve(device, data.len() as u64);
        if !data.is_empty() {
            queue.write_buffer(&self.buffer, 0, data);
        }
        grew
    }
}
