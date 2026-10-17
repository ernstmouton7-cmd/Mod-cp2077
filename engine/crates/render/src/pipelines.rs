//! Bind group layouts and render/compute pipelines of the frame graph.

use wgpu::{TextureFormat as F, TextureViewDimension as D};

use crate::gpu::{self, bt, src};

pub(crate) const HDR_FORMAT: F = F::Rgba16Float;
pub(crate) const DEPTH_FORMAT: F = F::Depth32Float;
pub(crate) const GB0_FORMAT: F = F::Rgba8UnormSrgb;
pub(crate) const GB1_FORMAT: F = F::Rgba16Float;
pub(crate) const GB2_FORMAT: F = F::Rgba8Unorm;
pub(crate) const GB3_FORMAT: F = F::Rgba16Float;
pub(crate) const AO_FORMAT: F = F::Rg16Float;
pub(crate) const SHADOW_FORMAT: F = F::Depth32Float;
pub(crate) const LDR_FORMAT: F = F::Rgba8Unorm;

pub(crate) struct Layouts {
    pub empty: wgpu::BindGroupLayout,
    pub frame: wgpu::BindGroupLayout,
    pub shadow_frame: wgpu::BindGroupLayout,
    pub instances: wgpu::BindGroupLayout,
    pub material: wgpu::BindGroupLayout,
    pub lighting: wgpu::BindGroupLayout,
    pub gbuf_read: wgpu::BindGroupLayout,
    pub decal: wgpu::BindGroupLayout,
    pub ssao: wgpu::BindGroupLayout,
    pub ssr: wgpu::BindGroupLayout,
    pub particles: wgpu::BindGroupLayout,
    pub taa: wgpu::BindGroupLayout,
    pub single_tex: wgpu::BindGroupLayout,
    pub post: wgpu::BindGroupLayout,
    pub exposure: wgpu::BindGroupLayout,
    pub depth_only: wgpu::BindGroupLayout,
    pub cluster: wgpu::BindGroupLayout,
}

impl Layouts {
    pub fn new(d: &wgpu::Device) -> Self {
        let frame_entries = [
            bt::uniform(),
            bt::storage(true),
            bt::sampler(),
            bt::sampler(),
            bt::sampler(),
            bt::sampler_cmp(),
        ];
        let mut shadow_entries = frame_entries.to_vec();
        shadow_entries.push(bt::uniform_dyn());
        Self {
            empty: gpu::bgl(d, "empty", &[]),
            frame: gpu::bgl(d, "frame", &frame_entries),
            shadow_frame: gpu::bgl(d, "shadow-frame", &shadow_entries),
            instances: gpu::bgl(d, "instances", &[bt::storage(true), bt::storage(true)]),
            material: gpu::bgl(
                d,
                "material",
                &[
                    bt::uniform(),
                    bt::tex2d(),
                    bt::tex2d(),
                    bt::tex2d(),
                    bt::tex2d(),
                    bt::tex2d(),
                    bt::tex2d(),
                    bt::sampler(),
                ],
            ),
            lighting: gpu::bgl(
                d,
                "lighting",
                &[
                    bt::tex_depth_array(),
                    bt::tex_depth_array(),
                    bt::tex_cube(),
                    bt::tex_cube(),
                    bt::tex2d(),
                    bt::tex2d(),
                    bt::tex2d(),
                    bt::tex2d(),
                    bt::tex2d(),
                    bt::storage(true),
                    bt::storage(true),
                    bt::storage(true),
                    bt::storage(true),
                    bt::tex_depth(),
                ],
            ),
            gbuf_read: gpu::bgl(
                d,
                "gbuf-read",
                &[
                    bt::tex_depth(),
                    bt::tex2d(),
                    bt::tex2d(),
                    bt::tex2d(),
                    bt::tex2d(),
                ],
            ),
            decal: gpu::bgl(d, "decal", &[bt::storage(true), bt::tex_depth()]),
            ssao: gpu::bgl(d, "ssao", &[bt::tex_depth(), bt::tex2d(), bt::tex2d()]),
            ssr: gpu::bgl(
                d,
                "ssr",
                &[
                    bt::tex_depth(),
                    bt::tex2d(),
                    bt::tex2d(),
                    bt::tex2d(),
                    bt::tex2d(),
                    bt::uniform(),
                ],
            ),
            particles: gpu::bgl(
                d,
                "particles",
                &[bt::storage(true), bt::tex2d_nf(), bt::uniform()],
            ),
            taa: gpu::bgl(
                d,
                "taa",
                &[
                    bt::tex2d(),
                    bt::tex_depth(),
                    bt::tex2d(),
                    bt::tex2d(),
                    bt::uniform(),
                ],
            ),
            single_tex: gpu::bgl(d, "single-tex", &[bt::tex2d()]),
            post: gpu::bgl(
                d,
                "post",
                &[
                    bt::tex2d(),
                    bt::tex2d(),
                    bt::tex2d(),
                    bt::uniform(),
                    bt::tex2d(),
                ],
            ),
            exposure: gpu::bgl(
                d,
                "exposure",
                &[
                    bt::storage(false),
                    bt::storage(false),
                    bt::uniform(),
                    bt::tex2d(),
                ],
            ),
            depth_only: gpu::bgl(d, "depth-only", &[bt::tex_depth()]),
            cluster: gpu::bgl(d, "cluster", &[bt::storage(true), bt::storage(false)]),
        }
    }
}

pub(crate) fn scene_vertex_layouts() -> [wgpu::VertexBufferLayout<'static>; 2] {
    const A0: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];
    const A1: [wgpu::VertexAttribute; 4] =
        wgpu::vertex_attr_array![2 => Float32x4, 3 => Float32x2, 4 => Float32x2, 5 => Float32x4];
    [
        wgpu::VertexBufferLayout {
            array_stride: 24,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &A0,
        },
        wgpu::VertexBufferLayout {
            array_stride: 48,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &A1,
        },
    ]
}

/// G-buffer pipeline variant index: bit0 = double sided, bit1 = alpha mask.
pub(crate) fn gbuffer_variant(double_sided: bool, mask: bool) -> usize {
    (double_sided as usize) | ((mask as usize) << 1)
}

pub(crate) struct Pipelines {
    pub gbuffer: Vec<wgpu::RenderPipeline>,
    pub gbuffer_wire: Option<wgpu::RenderPipeline>,
    pub shadow: Vec<wgpu::RenderPipeline>,
    pub forward: Vec<wgpu::RenderPipeline>,
    pub decal: wgpu::RenderPipeline,
    pub copy_depth: wgpu::RenderPipeline,
    pub gtao: wgpu::RenderPipeline,
    pub ao_blur: wgpu::RenderPipeline,
    pub ssr: wgpu::RenderPipeline,
    pub deferred: wgpu::RenderPipeline,
    pub particles: wgpu::RenderPipeline,
    pub rain: wgpu::RenderPipeline,
    pub taa: wgpu::RenderPipeline,
    pub bloom_down_first: wgpu::RenderPipeline,
    pub bloom_down: wgpu::RenderPipeline,
    pub bloom_up: wgpu::RenderPipeline,
    pub tonemap: wgpu::RenderPipeline,
    pub tonemap_ldr: wgpu::RenderPipeline,
    pub fxaa: wgpu::RenderPipeline,
    pub debug_lines: wgpu::RenderPipeline,
    pub cluster: wgpu::ComputePipeline,
    pub exposure_begin: wgpu::ComputePipeline,
    pub histogram: wgpu::ComputePipeline,
    pub adapt: wgpu::ComputePipeline,
}

fn opts<'a>(constants: &'a [(&'a str, f64)]) -> wgpu::PipelineCompilationOptions<'a> {
    wgpu::PipelineCompilationOptions {
        constants,
        zero_initialize_workgroup_memory: true,
    }
}

impl Pipelines {
    pub fn new(d: &wgpu::Device, l: &Layouts, output_format: F, polygon_line: bool) -> Self {
        let scene_mods: &[&str] = &[
            src::COMMON,
            src::BRDF,
            src::ATMOSPHERE,
            src::LIGHTING,
            src::MATERIAL,
            src::WET,
        ];
        let gb_mod = gpu::shader(d, "gbuffer", &[scene_mods, &[src::GBUFFER]].concat(), &[]);
        let sh_mod = gpu::shader(d, "shadow", &[src::COMMON, src::MATERIAL, src::SHADOW], &[]);
        let fw_mod = gpu::shader(d, "forward", &[scene_mods, &[src::FORWARD]].concat(), &[]);
        let light_mods: &[&str] = &[src::COMMON, src::BRDF, src::ATMOSPHERE, src::LIGHTING];
        let def_mod = gpu::shader(d, "deferred", &[light_mods, &[src::DEFERRED]].concat(), &[]);
        let part_mod = gpu::shader(
            d,
            "particles",
            &[light_mods, &[src::PARTICLES]].concat(),
            &[],
        );
        let cl_mod = gpu::shader(d, "cluster", &[light_mods, &[src::CLUSTER]].concat(), &[]);
        let decal_mod = gpu::shader(d, "decal", &[src::COMMON, src::WET, src::DECAL], &[]);
        let ssao_mod = gpu::shader(d, "ssao", &[src::COMMON, src::SSAO], &[]);
        let ssr_mod = gpu::shader(d, "ssr", &[src::COMMON, src::BRDF, src::SSR], &[]);
        let taa_mod = gpu::shader(d, "taa", &[src::COMMON, src::TAA], &[]);
        let bloom_mod = gpu::shader(d, "bloom", &[src::COMMON, src::BLOOM], &[]);
        let post_mod = gpu::shader(d, "post", &[src::COMMON, src::POST], &[]);
        let exp_mod = gpu::shader(d, "exposure", &[src::COMMON, src::EXPOSURE], &[]);
        let dbg_mod = gpu::shader(d, "debug", &[src::COMMON, src::DEBUG], &[]);
        let util_mod = gpu::shader(d, "util", &[src::COMMON, src::UTIL], &[]);

        let vlayouts = scene_vertex_layouts();
        let vbufs = [Some(vlayouts[0].clone()), Some(vlayouts[1].clone())];

        let gb_layout = gpu::pipeline_layout(d, "gbuffer", &[&l.frame, &l.instances, &l.material]);
        let gb_targets = [
            gpu::target(HDR_FORMAT),
            gpu::target(GB0_FORMAT),
            gpu::target(GB1_FORMAT),
            gpu::target(GB2_FORMAT),
            gpu::target(GB3_FORMAT),
        ];
        let depth_rw = Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Greater),
            stencil: Default::default(),
            bias: Default::default(),
        });
        let make_gb = |double_sided: bool, mask: bool, wire: bool| {
            let consts = [("ALPHA_MASK", if mask { 1.0 } else { 0.0 })];
            d.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("gbuffer"),
                layout: Some(&gb_layout),
                vertex: wgpu::VertexState {
                    module: &gb_mod,
                    entry_point: Some("vs_scene"),
                    compilation_options: opts(&[]),
                    buffers: &vbufs,
                },
                primitive: wgpu::PrimitiveState {
                    cull_mode: if double_sided || wire {
                        None
                    } else {
                        Some(wgpu::Face::Back)
                    },
                    polygon_mode: if wire {
                        wgpu::PolygonMode::Line
                    } else {
                        wgpu::PolygonMode::Fill
                    },
                    ..Default::default()
                },
                depth_stencil: depth_rw.clone(),
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &gb_mod,
                    entry_point: Some(if wire { "fs_wire" } else { "fs_gbuffer" }),
                    compilation_options: opts(if wire { &[] } else { &consts }),
                    targets: &gb_targets,
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let gbuffer = (0..4)
            .map(|v| make_gb(v & 1 != 0, v & 2 != 0, false))
            .collect();
        let gbuffer_wire = polygon_line.then(|| make_gb(true, false, true));

        let sh_layout =
            gpu::pipeline_layout(d, "shadow", &[&l.shadow_frame, &l.instances, &l.material]);
        let shadow = (0..2)
            .map(|v| {
                let mask = v == 1;
                let consts = [("ALPHA_MASK", if mask { 1.0 } else { 0.0 })];
                d.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("shadow"),
                    layout: Some(&sh_layout),
                    vertex: wgpu::VertexState {
                        module: &sh_mod,
                        entry_point: Some("vs_shadow"),
                        compilation_options: opts(&[]),
                        buffers: &vbufs,
                    },
                    primitive: wgpu::PrimitiveState {
                        cull_mode: None,
                        ..Default::default()
                    },
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: SHADOW_FORMAT,
                        depth_write_enabled: Some(true),
                        depth_compare: Some(wgpu::CompareFunction::LessEqual),
                        stencil: Default::default(),
                        bias: wgpu::DepthBiasState {
                            constant: 2,
                            slope_scale: 2.0,
                            clamp: 0.0,
                        },
                    }),
                    multisample: Default::default(),
                    fragment: mask.then(|| wgpu::FragmentState {
                        module: &sh_mod,
                        entry_point: Some("fs_shadow"),
                        compilation_options: opts(&consts),
                        targets: &[],
                    }),
                    multiview_mask: None,
                    cache: None,
                })
            })
            .collect();

        let depth_ro = Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(false),
            depth_compare: Some(wgpu::CompareFunction::GreaterEqual),
            stencil: Default::default(),
            bias: Default::default(),
        });
        let fw_layout = gpu::pipeline_layout(
            d,
            "forward",
            &[&l.frame, &l.instances, &l.material, &l.lighting],
        );
        let fw_targets = [gpu::target_blend(HDR_FORMAT, gpu::PREMULTIPLIED)];
        let forward = (0..2)
            .map(|v| {
                d.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("forward"),
                    layout: Some(&fw_layout),
                    vertex: wgpu::VertexState {
                        module: &fw_mod,
                        entry_point: Some("vs_scene"),
                        compilation_options: opts(&[]),
                        buffers: &vbufs,
                    },
                    primitive: wgpu::PrimitiveState {
                        cull_mode: if v == 1 { None } else { Some(wgpu::Face::Back) },
                        ..Default::default()
                    },
                    depth_stencil: depth_ro.clone(),
                    multisample: Default::default(),
                    fragment: Some(wgpu::FragmentState {
                        module: &fw_mod,
                        entry_point: Some("fs_forward"),
                        compilation_options: opts(&[]),
                        targets: &fw_targets,
                    }),
                    multiview_mask: None,
                    cache: None,
                })
            })
            .collect();

        let decal_layout = gpu::pipeline_layout(d, "decal", &[&l.frame, &l.decal]);
        let decal = d.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("decal"),
            layout: Some(&decal_layout),
            vertex: wgpu::VertexState {
                module: &decal_mod,
                entry_point: Some("vs_decal"),
                compilation_options: opts(&[]),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState {
                cull_mode: Some(wgpu::Face::Front),
                ..Default::default()
            },
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &decal_mod,
                entry_point: Some("fs_decal"),
                compilation_options: opts(&[]),
                targets: &[
                    Some(wgpu::ColorTargetState {
                        format: GB0_FORMAT,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::COLOR,
                    }),
                    Some(wgpu::ColorTargetState {
                        format: GB1_FORMAT,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::BLUE,
                    }),
                ],
            }),
            multiview_mask: None,
            cache: None,
        });

        let copy_depth = gpu::fullscreen_pipeline(
            d,
            "copy-depth",
            &util_mod,
            "fs_copy_depth",
            &[&l.frame, &l.depth_only],
            &[gpu::target(F::R32Float)],
        );
        let gtao = gpu::fullscreen_pipeline(
            d,
            "gtao",
            &ssao_mod,
            "fs_gtao",
            &[&l.frame, &l.ssao],
            &[gpu::target(AO_FORMAT)],
        );
        let ao_blur = gpu::fullscreen_pipeline(
            d,
            "ao-blur",
            &ssao_mod,
            "fs_ao_blur",
            &[&l.frame, &l.ssao],
            &[gpu::target(AO_FORMAT)],
        );
        let ssr = gpu::fullscreen_pipeline(
            d,
            "ssr",
            &ssr_mod,
            "fs_ssr",
            &[&l.frame, &l.ssr],
            &[gpu::target(HDR_FORMAT)],
        );
        let deferred = gpu::fullscreen_pipeline(
            d,
            "deferred",
            &def_mod,
            "fs_deferred",
            &[&l.frame, &l.gbuf_read, &l.empty, &l.lighting],
            &[gpu::target_blend(HDR_FORMAT, gpu::ADD_WITH_TRANSMITTANCE)],
        );

        let part_layout = gpu::pipeline_layout(
            d,
            "particles",
            &[&l.frame, &l.particles, &l.empty, &l.lighting],
        );
        let make_part = |vs: &str, fs: &str| {
            d.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(fs),
                layout: Some(&part_layout),
                vertex: wgpu::VertexState {
                    module: &part_mod,
                    entry_point: Some(vs),
                    compilation_options: opts(&[]),
                    buffers: &[],
                },
                primitive: Default::default(),
                depth_stencil: depth_ro.clone(),
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &part_mod,
                    entry_point: Some(fs),
                    compilation_options: opts(&[]),
                    targets: &[gpu::target_blend(HDR_FORMAT, gpu::PREMULTIPLIED)],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let particles = make_part("vs_particle", "fs_particle");
        let rain = make_part("vs_rain", "fs_rain");

        let taa = gpu::fullscreen_pipeline(
            d,
            "taa",
            &taa_mod,
            "fs_taa",
            &[&l.frame, &l.taa],
            &[gpu::target(HDR_FORMAT)],
        );
        let bl = [&l.frame, &l.single_tex];
        let bloom_down_first = gpu::fullscreen_pipeline(
            d,
            "bloom-down0",
            &bloom_mod,
            "fs_down_first",
            &bl,
            &[gpu::target(HDR_FORMAT)],
        );
        let bloom_down = gpu::fullscreen_pipeline(
            d,
            "bloom-down",
            &bloom_mod,
            "fs_down",
            &bl,
            &[gpu::target(HDR_FORMAT)],
        );
        let bloom_up = gpu::fullscreen_pipeline(
            d,
            "bloom-up",
            &bloom_mod,
            "fs_up",
            &bl,
            &[gpu::target_blend(HDR_FORMAT, gpu::ADDITIVE)],
        );

        let encode = if output_format.is_srgb() { 0.0 } else { 1.0 };
        let post_layout = gpu::pipeline_layout(d, "post", &[&l.frame, &l.post]);
        let make_post = |fs: &str, fmt: F, enc: f64| {
            let consts = [("OUTPUT_ENCODE_SRGB", enc)];
            d.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(fs),
                layout: Some(&post_layout),
                vertex: wgpu::VertexState {
                    module: &post_mod,
                    entry_point: Some("vs_fullscreen"),
                    compilation_options: opts(&[]),
                    buffers: &[],
                },
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &post_mod,
                    entry_point: Some(fs),
                    compilation_options: opts(&consts),
                    targets: &[gpu::target(fmt)],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let tonemap = make_post("fs_tonemap", output_format, encode);
        let tonemap_ldr = make_post("fs_tonemap", LDR_FORMAT, 1.0);
        let fxaa = make_post("fs_fxaa", output_format, encode);

        let dbg_layout = gpu::pipeline_layout(d, "debug", &[&l.frame, &l.depth_only]);
        const LA: [wgpu::VertexAttribute; 2] =
            wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4];
        let dbg_consts = [("OUTPUT_ENCODE_SRGB", encode)];
        let debug_lines = d.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("debug-lines"),
            layout: Some(&dbg_layout),
            vertex: wgpu::VertexState {
                module: &dbg_mod,
                entry_point: Some("vs_line"),
                compilation_options: opts(&[]),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: 28,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &LA,
                })],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &dbg_mod,
                entry_point: Some("fs_line"),
                compilation_options: opts(&dbg_consts),
                targets: &[gpu::target_blend(output_format, gpu::PREMULTIPLIED)],
            }),
            multiview_mask: None,
            cache: None,
        });

        let cluster =
            gpu::compute_pipeline(d, "cluster", &cl_mod, "cs_cluster", &[&l.frame, &l.cluster]);
        let exposure_begin = gpu::compute_pipeline(
            d,
            "exposure-begin",
            &exp_mod,
            "cs_begin",
            &[&l.empty, &l.exposure],
        );
        let histogram = gpu::compute_pipeline(
            d,
            "histogram",
            &exp_mod,
            "cs_histogram",
            &[&l.empty, &l.exposure],
        );
        let adapt =
            gpu::compute_pipeline(d, "adapt", &exp_mod, "cs_adapt", &[&l.empty, &l.exposure]);

        Self {
            gbuffer,
            gbuffer_wire,
            shadow,
            forward,
            decal,
            copy_depth,
            gtao,
            ao_blur,
            ssr,
            deferred,
            particles,
            rain,
            taa,
            bloom_down_first,
            bloom_down,
            bloom_up,
            tonemap,
            tonemap_ldr,
            fxaa,
            debug_lines,
            cluster,
            exposure_begin,
            histogram,
            adapt,
        }
    }
}

pub(crate) fn storage_tex_2d() -> wgpu::BindingType {
    bt::storage_tex(F::Rgba16Float, D::D2)
}

pub(crate) fn storage_tex_array() -> wgpu::BindingType {
    bt::storage_tex(F::Rgba16Float, D::D2Array)
}
