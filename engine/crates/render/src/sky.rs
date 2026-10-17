//! Atmosphere LUTs, procedural sky cubemap and image based lighting precomputation.

use wgpu::TextureViewDimension as D;

use crate::frame::PrefilterParams;
use crate::gpu::{self, bt, src};
use crate::pipelines::{storage_tex_2d, storage_tex_array, Layouts};

const LUT_FMT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

pub(crate) const SPEC_MIPS: u32 = 6;

fn storage_texture(
    d: &wgpu::Device,
    label: &str,
    w: u32,
    h: u32,
    layers: u32,
    mips: u32,
) -> wgpu::Texture {
    d.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: layers,
        },
        mip_level_count: mips,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: LUT_FMT,
        usage: wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::STORAGE_BINDING
            | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    })
}

fn view(t: &wgpu::Texture, dim: D, mip: u32, mips: Option<u32>) -> wgpu::TextureView {
    t.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(dim),
        base_mip_level: mip,
        mip_level_count: mips,
        ..Default::default()
    })
}

/// Radiance cube + GGX prefiltered cube.
pub(crate) struct IblSet {
    pub env: wgpu::Texture,
    pub spec: wgpu::Texture,
    pub spec_view: wgpu::TextureView,
    pub res: u32,
    /// Per-set SH coefficients (9 x vec4), copied into the shared SH buffer when bound.
    pub sh: wgpu::Buffer,
    env_mip_bgs: Vec<wgpu::BindGroup>,
    prefilter_bgs: Vec<wgpu::BindGroup>,
    sh_bg: wgpu::BindGroup,
    pub env_out_bg: wgpu::BindGroup,
}

pub(crate) struct IblLayouts {
    l_skycube: wgpu::BindGroupLayout,
    l_equirect: wgpu::BindGroupLayout,
    l_downsample: wgpu::BindGroupLayout,
    l_prefilter: wgpu::BindGroupLayout,
    l_sh: wgpu::BindGroupLayout,
}

pub(crate) struct Sky {
    pub transmittance: wgpu::Texture,
    pub transmittance_view: wgpu::TextureView,
    pub multiscatter: wgpu::Texture,
    pub skyview: wgpu::Texture,
    pub skyview_view: wgpu::TextureView,
    pub brdf_lut_view: wgpu::TextureView,
    _brdf_lut: wgpu::Texture,
    p_transmittance: wgpu::ComputePipeline,
    p_multiscatter: wgpu::ComputePipeline,
    p_skyview: wgpu::ComputePipeline,
    p_skycube: wgpu::ComputePipeline,
    p_equirect: wgpu::ComputePipeline,
    p_downsample: wgpu::ComputePipeline,
    p_prefilter: wgpu::ComputePipeline,
    p_sh: wgpu::ComputePipeline,
    bg_transmittance: wgpu::BindGroup,
    bg_multiscatter: wgpu::BindGroup,
    bg_skyview: wgpu::BindGroup,
    il: IblLayouts,
    pub sky_ibl: IblSet,
    pub luts_valid: bool,
}

const SKY_BINDINGS_T: &str =
    "@group(1) @binding(0) var out_tex: texture_storage_2d<rgba16float, write>;";
const SKY_BINDINGS_MS: &str = "@group(1) @binding(0) var t_trans: texture_2d<f32>;\n@group(1) @binding(1) var t_ms: texture_2d<f32>;\n@group(1) @binding(2) var out_tex: texture_storage_2d<rgba16float, write>;";
const SKY_BINDINGS_CUBE: &str = "@group(1) @binding(0) var t_trans: texture_2d<f32>;\n@group(1) @binding(1) var t_sv: texture_2d<f32>;\n@group(1) @binding(2) var out_cube: texture_storage_2d_array<rgba16float, write>;";
const IBL_BINDINGS_EQ: &str = "@group(1) @binding(0) var t_src: texture_2d<f32>;\n@group(1) @binding(1) var out_cube: texture_storage_2d_array<rgba16float, write>;";
const IBL_BINDINGS_DOWN: &str = "@group(1) @binding(0) var t_src_mip: texture_2d_array<f32>;\n@group(1) @binding(1) var out_cube: texture_storage_2d_array<rgba16float, write>;";
const IBL_BINDINGS_PRE: &str = "@group(1) @binding(0) var t_env: texture_cube<f32>;\n@group(1) @binding(1) var out_cube: texture_storage_2d_array<rgba16float, write>;\n@group(1) @binding(2) var<uniform> params: PrefilterParams;";
const IBL_BINDINGS_SH: &str = "@group(1) @binding(0) var t_src_mip: texture_2d_array<f32>;\n@group(1) @binding(1) var<storage, read_write> sh_out: array<vec4f, 9>;";
const IBL_BINDINGS_BRDF: &str =
    "@group(1) @binding(0) var out_tex: texture_storage_2d<rgba16float, write>;";

fn groups(n: u32) -> u32 {
    n.div_ceil(8)
}

impl Sky {
    pub fn new(
        d: &wgpu::Device,
        q: &wgpu::Queue,
        layouts: &Layouts,
        frame_bg: &wgpu::BindGroup,
        ibl_res: u32,
    ) -> Self {
        let sky_mods = [src::COMMON, src::ATMOSPHERE, src::SKY_LUTS];
        let mk = |label: &str, bindings: &str, flags: &[&str]| {
            let mut defs: Vec<(&str, String)> = vec![("BINDINGS", bindings.to_string())];
            for f in flags {
                defs.push((f, String::new()));
            }
            gpu::shader(d, label, &sky_mods, &defs)
        };
        let m_t = mk("sky-transmittance", SKY_BINDINGS_T, &["TRANSMITTANCE"]);
        let m_ms = mk(
            "sky-multiscatter",
            SKY_BINDINGS_MS,
            &["SCATTER", "MULTISCATTER"],
        );
        let m_sv = mk("sky-view", SKY_BINDINGS_MS, &["SCATTER", "SKYVIEW"]);
        let m_cube = mk("sky-cube", SKY_BINDINGS_CUBE, &["SKYCUBE"]);

        let ibl_mods = [src::COMMON, src::BRDF, src::IBL];
        let mi = |label: &str, bindings: &str, flag: &str| {
            gpu::shader(
                d,
                label,
                &ibl_mods,
                &[("BINDINGS", bindings.to_string()), (flag, String::new())],
            )
        };
        let m_eq = mi("ibl-equirect", IBL_BINDINGS_EQ, "EQUIRECT");
        let m_down = mi("ibl-down", IBL_BINDINGS_DOWN, "DOWNSAMPLE");
        let m_pre = mi("ibl-prefilter", IBL_BINDINGS_PRE, "PREFILTER");
        let m_sh = mi("ibl-sh", IBL_BINDINGS_SH, "SH");
        let m_brdf = mi("ibl-brdf", IBL_BINDINGS_BRDF, "BRDF");

        let l_t = gpu::bgl(d, "sky-t", &[storage_tex_2d()]);
        let l_ms = gpu::bgl(d, "sky-ms", &[bt::tex2d(), bt::tex2d(), storage_tex_2d()]);
        let l_skycube = gpu::bgl(
            d,
            "sky-cube",
            &[bt::tex2d(), bt::tex2d(), storage_tex_array()],
        );
        let l_equirect = gpu::bgl(d, "ibl-eq", &[bt::tex2d(), storage_tex_array()]);
        let l_downsample = gpu::bgl(d, "ibl-down", &[bt::tex(D::D2Array), storage_tex_array()]);
        let l_prefilter = gpu::bgl(
            d,
            "ibl-pre",
            &[bt::tex_cube(), storage_tex_array(), bt::uniform()],
        );
        let l_sh = gpu::bgl(d, "ibl-sh", &[bt::tex(D::D2Array), bt::storage(false)]);

        let f = &layouts.frame;
        let p_transmittance =
            gpu::compute_pipeline(d, "transmittance", &m_t, "cs_transmittance", &[f, &l_t]);
        let p_multiscatter =
            gpu::compute_pipeline(d, "multiscatter", &m_ms, "cs_multiscatter", &[f, &l_ms]);
        let p_skyview = gpu::compute_pipeline(d, "skyview", &m_sv, "cs_skyview", &[f, &l_ms]);
        let p_skycube =
            gpu::compute_pipeline(d, "skycube", &m_cube, "cs_skycube", &[f, &l_skycube]);
        let p_equirect =
            gpu::compute_pipeline(d, "equirect", &m_eq, "cs_equirect", &[f, &l_equirect]);
        let p_downsample = gpu::compute_pipeline(
            d,
            "cube-down",
            &m_down,
            "cs_downsample",
            &[f, &l_downsample],
        );
        let p_prefilter =
            gpu::compute_pipeline(d, "prefilter", &m_pre, "cs_prefilter", &[f, &l_prefilter]);
        let p_sh = gpu::compute_pipeline(d, "sh", &m_sh, "cs_sh", &[f, &l_sh]);
        let l_brdf = gpu::bgl(d, "brdf", &[storage_tex_2d()]);
        let p_brdf = gpu::compute_pipeline(d, "brdf-lut", &m_brdf, "cs_brdf_lut", &[f, &l_brdf]);

        let transmittance = storage_texture(d, "transmittance-lut", 256, 64, 1, 1);
        let multiscatter = storage_texture(d, "multiscatter-lut", 32, 32, 1, 1);
        let skyview = storage_texture(d, "skyview-lut", 256, 128, 1, 1);
        let brdf = storage_texture(d, "brdf-lut", 128, 128, 1, 1);
        let dummy = storage_texture(d, "sky-dummy", 1, 1, 1, 1);
        let tv = transmittance.create_view(&Default::default());
        let msv = multiscatter.create_view(&Default::default());
        let svv = skyview.create_view(&Default::default());
        let brdf_view = brdf.create_view(&Default::default());
        let dv = dummy.create_view(&Default::default());

        let bg_transmittance =
            gpu::bind_group(d, "sky-t", &l_t, &[wgpu::BindingResource::TextureView(&tv)]);
        let bg_multiscatter = gpu::bind_group(
            d,
            "sky-ms",
            &l_ms,
            &[
                wgpu::BindingResource::TextureView(&tv),
                wgpu::BindingResource::TextureView(&dv),
                wgpu::BindingResource::TextureView(&msv),
            ],
        );
        let bg_skyview = gpu::bind_group(
            d,
            "sky-sv",
            &l_ms,
            &[
                wgpu::BindingResource::TextureView(&tv),
                wgpu::BindingResource::TextureView(&msv),
                wgpu::BindingResource::TextureView(&svv),
            ],
        );
        let bg_brdf = gpu::bind_group(
            d,
            "brdf",
            &l_brdf,
            &[wgpu::BindingResource::TextureView(&brdf_view)],
        );

        let il = IblLayouts {
            l_skycube,
            l_equirect,
            l_downsample,
            l_prefilter,
            l_sh,
        };
        let sky_ibl = create_ibl_set(d, &il, &tv, &svv, ibl_res, true);
        let sky = Self {
            transmittance,
            transmittance_view: tv,
            multiscatter,
            skyview,
            skyview_view: svv,
            brdf_lut_view: brdf_view,
            _brdf_lut: brdf,
            p_transmittance,
            p_multiscatter,
            p_skyview,
            p_skycube,
            p_equirect,
            p_downsample,
            p_prefilter,
            p_sh,
            bg_transmittance,
            bg_multiscatter,
            bg_skyview,
            sky_ibl,
            il,
            luts_valid: false,
        };

        // BRDF LUT once.
        let mut enc = d.create_command_encoder(&Default::default());
        {
            let mut p = enc.begin_compute_pass(&Default::default());
            p.set_pipeline(&p_brdf);
            p.set_bind_group(0, frame_bg, &[]);
            p.set_bind_group(1, &bg_brdf, &[]);
            p.dispatch_workgroups(16, 16, 1);
        }
        q.submit([enc.finish()]);
        sky
    }

    /// Creates an IBL set from an equirect RGBA16F texture and records its precomputation.
    pub fn hdri_set(
        &self,
        d: &wgpu::Device,
        enc: &mut wgpu::CommandEncoder,
        frame_bg: &wgpu::BindGroup,
        equirect: &wgpu::TextureView,
        res: u32,
    ) -> IblSet {
        let mut set = create_ibl_set(
            d,
            &self.il,
            &self.transmittance_view,
            &self.skyview_view,
            res,
            false,
        );
        let out0 = view(&set.env, D::D2Array, 0, Some(1));
        set.env_out_bg = gpu::bind_group(
            d,
            "eq",
            &self.il.l_equirect,
            &[
                wgpu::BindingResource::TextureView(equirect),
                wgpu::BindingResource::TextureView(&out0),
            ],
        );
        {
            let mut p = enc.begin_compute_pass(&Default::default());
            p.set_pipeline(&self.p_equirect);
            p.set_bind_group(0, frame_bg, &[]);
            p.set_bind_group(1, &set.env_out_bg, &[]);
            p.dispatch_workgroups(groups(res), groups(res), 6);
        }
        self.filter_ibl(enc, frame_bg, &set);
        set
    }

    fn filter_ibl(&self, enc: &mut wgpu::CommandEncoder, frame_bg: &wgpu::BindGroup, set: &IblSet) {
        let mut p = enc.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("ibl-filter"),
            timestamp_writes: None,
        });
        p.set_bind_group(0, frame_bg, &[]);
        p.set_pipeline(&self.p_downsample);
        let mut size = set.res;
        for bg in &set.env_mip_bgs {
            size = (size / 2).max(1);
            p.set_bind_group(1, bg, &[]);
            p.dispatch_workgroups(groups(size), groups(size), 6);
        }
        p.set_pipeline(&self.p_prefilter);
        let mut size = set.res;
        for bg in &set.prefilter_bgs {
            p.set_bind_group(1, bg, &[]);
            p.dispatch_workgroups(groups(size), groups(size), 6);
            size = (size / 2).max(1);
        }
        p.set_pipeline(&self.p_sh);
        p.set_bind_group(1, &set.sh_bg, &[]);
        p.dispatch_workgroups(1, 1, 1);
    }

    /// Records the per-frame sky work: LUTs when invalid, sky view every frame,
    /// and the sky IBL when `update_ibl` is set.
    pub fn encode(
        &mut self,
        enc: &mut wgpu::CommandEncoder,
        frame_bg: &wgpu::BindGroup,
        update_ibl: bool,
        sh_target: &wgpu::Buffer,
    ) {
        {
            let mut p = enc.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("sky-luts"),
                timestamp_writes: None,
            });
            p.set_bind_group(0, frame_bg, &[]);
            if !self.luts_valid {
                p.set_pipeline(&self.p_transmittance);
                p.set_bind_group(1, &self.bg_transmittance, &[]);
                p.dispatch_workgroups(32, 8, 1);
            }
        }
        if !self.luts_valid {
            let mut p = enc.begin_compute_pass(&Default::default());
            p.set_bind_group(0, frame_bg, &[]);
            p.set_pipeline(&self.p_multiscatter);
            p.set_bind_group(1, &self.bg_multiscatter, &[]);
            p.dispatch_workgroups(4, 4, 1);
            self.luts_valid = true;
        }
        {
            let mut p = enc.begin_compute_pass(&Default::default());
            p.set_bind_group(0, frame_bg, &[]);
            p.set_pipeline(&self.p_skyview);
            p.set_bind_group(1, &self.bg_skyview, &[]);
            p.dispatch_workgroups(32, 16, 1);
        }
        if update_ibl {
            let res = self.sky_ibl.res;
            {
                let mut p = enc.begin_compute_pass(&Default::default());
                p.set_bind_group(0, frame_bg, &[]);
                p.set_pipeline(&self.p_skycube);
                p.set_bind_group(1, &self.sky_ibl.env_out_bg, &[]);
                p.dispatch_workgroups(groups(res), groups(res), 6);
            }
            self.filter_ibl(enc, frame_bg, &self.sky_ibl);
            enc.copy_buffer_to_buffer(&self.sky_ibl.sh, 0, sh_target, 0, 9 * 16);
        }
    }
}

/// Creates the textures and bind groups of an IBL set. `sky` sets use the sky LUTs as input.
pub(crate) fn create_ibl_set(
    d: &wgpu::Device,
    il: &IblLayouts,
    trans_view: &wgpu::TextureView,
    skyview_view: &wgpu::TextureView,
    res: u32,
    sky: bool,
) -> IblSet {
    let env_mips = crate::texture::mip_count(res, res);
    let env = storage_texture(d, "ibl-env", res, res, 6, env_mips);
    let spec = storage_texture(d, "ibl-spec", res, res, 6, SPEC_MIPS);
    let spec_view = view(&spec, D::Cube, 0, None);
    let env_cube = view(&env, D::Cube, 0, None);
    let sh = gpu::buffer(
        d,
        "ibl-sh",
        9 * 16,
        wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
    );
    let env_mip_bgs = (1..env_mips)
        .map(|m| {
            let src_v = view(&env, D::D2Array, m - 1, Some(1));
            let dst_v = view(&env, D::D2Array, m, Some(1));
            gpu::bind_group(
                d,
                "ibl-down",
                &il.l_downsample,
                &[
                    wgpu::BindingResource::TextureView(&src_v),
                    wgpu::BindingResource::TextureView(&dst_v),
                ],
            )
        })
        .collect();
    let prefilter_bgs = (0..SPEC_MIPS)
        .map(|m| {
            let dst_v = view(&spec, D::D2Array, m, Some(1));
            let rough = m as f32 / (SPEC_MIPS - 1) as f32;
            let samples = if m == 0 { 1 } else { 48 + 16 * m };
            let p = PrefilterParams {
                roughness: rough,
                src_size: res as f32,
                samples,
                pad: 0,
            };
            let ub = gpu::buffer_init(
                d,
                "prefilter-params",
                bytemuck::bytes_of(&p),
                wgpu::BufferUsages::UNIFORM,
            );
            gpu::bind_group(
                d,
                "ibl-pre",
                &il.l_prefilter,
                &[
                    wgpu::BindingResource::TextureView(&env_cube),
                    wgpu::BindingResource::TextureView(&dst_v),
                    ub.as_entire_binding(),
                ],
            )
        })
        .collect();
    // SH from the mip that is ~16 texels (or the smallest available).
    let sh_mip = env_mips.saturating_sub(5).min(env_mips - 1);
    let sh_src = view(&env, D::D2Array, sh_mip, Some(1));
    let sh_bg = gpu::bind_group(
        d,
        "ibl-sh",
        &il.l_sh,
        &[
            wgpu::BindingResource::TextureView(&sh_src),
            sh.as_entire_binding(),
        ],
    );
    let out0 = view(&env, D::D2Array, 0, Some(1));
    let env_out_bg = if sky {
        gpu::bind_group(
            d,
            "sky-cube",
            &il.l_skycube,
            &[
                wgpu::BindingResource::TextureView(trans_view),
                wgpu::BindingResource::TextureView(skyview_view),
                wgpu::BindingResource::TextureView(&out0),
            ],
        )
    } else {
        // Placeholder; replaced by the equirect bind group in `upload_hdri`.
        let dummy = d.create_texture(&wgpu::TextureDescriptor {
            label: Some("eq-dummy"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let dv = dummy.create_view(&Default::default());
        gpu::bind_group(
            d,
            "eq",
            &il.l_equirect,
            &[
                wgpu::BindingResource::TextureView(&dv),
                wgpu::BindingResource::TextureView(&out0),
            ],
        )
    };
    IblSet {
        env,
        spec,
        spec_view,
        res,
        sh,
        env_mip_bgs,
        prefilter_bgs,
        sh_bg,
        env_out_bg,
    }
}
