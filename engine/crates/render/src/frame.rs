//! GPU-side uniform/storage layouts. Must match the WGSL structs exactly.

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec4};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct FrameUniforms {
    pub view: Mat4,
    pub proj: Mat4,
    pub view_proj: Mat4,
    pub inv_view: Mat4,
    pub inv_proj: Mat4,
    pub inv_view_proj: Mat4,
    pub prev_view_proj: Mat4,
    pub view_proj_nj: Mat4,
    pub cascade_vp: [Mat4; 4],
    pub camera_pos: Vec4,
    pub viewport: Vec4,
    pub output: Vec4,
    pub jitter: Vec4,
    pub proj_params: Vec4,
    pub sun: Vec4,
    pub moon: Vec4,
    pub light_params: Vec4,
    pub sky: Vec4,
    pub sky2: Vec4,
    pub sky3: Vec4,
    pub night_glow: Vec4,
    pub fog: Vec4,
    pub fog2: Vec4,
    pub weather: Vec4,
    pub cascade_splits: Vec4,
    pub shadow: Vec4,
    pub cascade_texel: Vec4,
    pub cluster_dims: [u32; 4],
    pub cluster_z: Vec4,
    pub post: Vec4,
    pub vol_lights: [[u32; 4]; 8],
    pub skyocc_vp: Mat4,
    pub skyocc: Vec4,
    pub bounce: Vec4,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct ExposureGpu {
    pub pre: f32,
    pub current: f32,
    pub prev_pre: f32,
    pub ev100: f32,
    pub avg_lum: f32,
    pub target_ev: f32,
    pub pad: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct ExposureParams {
    pub min_ev: f32,
    pub max_ev: f32,
    pub compensation: f32,
    pub speed: f32,
    pub dt: f32,
    pub manual_ev: f32,
    pub reset: f32,
    pub low_pct: f32,
    pub high_pct: f32,
    pub pad: [f32; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct InstanceGpu {
    pub model: Mat4,
    pub prev_model: Mat4,
    pub normal: [Vec4; 3],
    pub tint: Vec4,
    pub params: Vec4,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct MaterialGpu {
    pub base_color: Vec4,
    pub emissive: Vec4,
    pub pbr: Vec4,
    pub coat: Vec4,
    pub extra: Vec4,
    pub uv: Vec4,
    pub flags: [u32; 4],
    pub extra2: Vec4,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct LightGpu {
    pub pos_range: Vec4,
    pub color: Vec4,
    pub dir_cos: Vec4,
    pub params: Vec4,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct ParticleGpu {
    pub pos_size: Vec4,
    pub color: Vec4,
    pub vel_rot: Vec4,
    pub params: Vec4,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct DecalGpu {
    pub model: Mat4,
    pub inv_model: Mat4,
    pub color: Vec4,
    pub params: Vec4,
    pub params2: Vec4,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct ShadowViewGpu {
    pub view_proj: Mat4,
    pub params: Vec4,
    pub pad: [Vec4; 11],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct TaaParams {
    pub blend: f32,
    pub reset: f32,
    pub enabled: f32,
    pub pad: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct SsrParams {
    pub steps: u32,
    pub max_dist: f32,
    pub thickness: f32,
    pub enabled: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct PostParams {
    pub bloom: f32,
    pub vignette: f32,
    pub grain: f32,
    pub ca: f32,
    pub motion_blur: f32,
    pub sharpen: f32,
    pub tonemapper: f32,
    pub ldr_out: f32,
    pub bloom_norm: f32,
    pub pad: [f32; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct PrefilterParams {
    pub roughness: f32,
    pub src_size: f32,
    pub samples: u32,
    pub pad: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct LineVertex {
    pub pos: [f32; 3],
    pub color: [f32; 4],
}

const _: () = assert!(std::mem::size_of::<ShadowViewGpu>() == 256);
