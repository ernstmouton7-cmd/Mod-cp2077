//! Screen-size render targets (recreated on resize / render-scale change).

use wgpu::TextureUsages as U;

use crate::gpu::RenderTexture;
use crate::pipelines::*;

pub(crate) const BLOOM_MIPS: u32 = 6;

pub(crate) struct Targets {
    pub render_w: u32,
    pub render_h: u32,
    pub out_w: u32,
    pub out_h: u32,
    pub depth: RenderTexture,
    pub depth_copy: RenderTexture,
    pub hdr: RenderTexture,
    pub gb0: RenderTexture,
    pub gb1: RenderTexture,
    pub gb2: RenderTexture,
    pub gb3: RenderTexture,
    pub ao_raw: RenderTexture,
    pub ao: RenderTexture,
    pub ssr: RenderTexture,
    pub history: [RenderTexture; 2],
    pub bloom: RenderTexture,
    pub bloom_views: Vec<wgpu::TextureView>,
    pub ldr: RenderTexture,
    /// Offscreen final image (headless rendering / captures).
    pub final_offscreen: RenderTexture,
}

impl Targets {
    pub fn new(
        d: &wgpu::Device,
        out_w: u32,
        out_h: u32,
        scale: f32,
        ao_half: bool,
        ssr_half: bool,
        final_format: wgpu::TextureFormat,
    ) -> Self {
        let rw = ((out_w as f32 * scale).round() as u32).max(16);
        let rh = ((out_h as f32 * scale).round() as u32).max(16);
        let rt = U::RENDER_ATTACHMENT | U::TEXTURE_BINDING;
        let mk = |l: &str, w: u32, h: u32, f| RenderTexture::new(d, l, w, h, f, rt);
        let (aw, ah) = if ao_half {
            (rw.div_ceil(2), rh.div_ceil(2))
        } else {
            (rw, rh)
        };
        let (sw, sh) = if ssr_half {
            (rw.div_ceil(2), rh.div_ceil(2))
        } else {
            (rw, rh)
        };
        let bw = out_w.div_ceil(2).max(1);
        let bh = out_h.div_ceil(2).max(1);
        let mips = BLOOM_MIPS.min(crate::texture::mip_count(bw, bh));
        let bloom = RenderTexture::with_mips(d, "bloom", bw, bh, mips, HDR_FORMAT, rt);
        let bloom_views = (0..mips).map(|m| bloom.mip_view(m)).collect();
        Self {
            render_w: rw,
            render_h: rh,
            out_w,
            out_h,
            depth: RenderTexture::new(d, "depth", rw, rh, DEPTH_FORMAT, rt | U::COPY_SRC),
            depth_copy: mk("depth-copy", rw, rh, wgpu::TextureFormat::R32Float),
            hdr: mk("hdr", rw, rh, HDR_FORMAT),
            gb0: mk("gb0", rw, rh, GB0_FORMAT),
            gb1: mk("gb1", rw, rh, GB1_FORMAT),
            gb2: mk("gb2", rw, rh, GB2_FORMAT),
            gb3: mk("gb3", rw, rh, GB3_FORMAT),
            ao_raw: mk("ao-raw", aw, ah, AO_FORMAT),
            ao: mk("ao", aw, ah, AO_FORMAT),
            ssr: mk("ssr", sw, sh, HDR_FORMAT),
            history: [
                mk("history0", out_w, out_h, HDR_FORMAT),
                mk("history1", out_w, out_h, HDR_FORMAT),
            ],
            bloom,
            bloom_views,
            ldr: mk("ldr", out_w, out_h, LDR_FORMAT),
            final_offscreen: RenderTexture::new(
                d,
                "final",
                out_w,
                out_h,
                final_format,
                rt | U::COPY_SRC,
            ),
        }
    }
}
