// ---------------------------------------------------------------------------
// common.wgsl — frame uniforms, samplers and utility functions shared by all
// scene passes. Must match `FrameUniforms` in src/frame.rs.

const PI: f32 = 3.14159265359;
const TAU: f32 = 6.28318530718;
const INV_PI: f32 = 0.31830988618;
const MAX_HALF: f32 = 60000.0;

struct Frame {
    view: mat4x4f,
    proj: mat4x4f,
    view_proj: mat4x4f,
    inv_view: mat4x4f,
    inv_proj: mat4x4f,
    inv_view_proj: mat4x4f,
    prev_view_proj: mat4x4f,
    view_proj_nj: mat4x4f,
    cascade_vp: array<mat4x4f, 4>,
    camera_pos: vec4f,      // xyz, w = time (s)
    viewport: vec4f,        // render w, h, 1/w, 1/h
    output: vec4f,          // output w, h, 1/w, 1/h
    jitter: vec4f,          // current jitter (ndc) xy, previous jitter xy
    proj_params: vec4f,     // near, tan(fov/2), aspect, frame index
    sun: vec4f,             // dir to sun, TOA illuminance (lux)
    moon: vec4f,            // dir to moon, illuminance (lux)
    light_params: vec4f,    // primary shadow light is moon (0/1), sun angular radius, moon angular radius, dt
    sky: vec4f,             // sky intensity, mie scale, ground albedo, cloud cover
    sky2: vec4f,            // stars, hdri blend, hdri rotation, hdri intensity
    sky3: vec4f,            // hdri background, wind x, wind z, camera altitude (km)
    night_glow: vec4f,      // rgb nits
    fog: vec4f,             // density, height falloff, base height, anisotropy
    fog2: vec4f,            // start, volumetric scale, volumetric light count, 0
    weather: vec4f,         // wetness, puddles, rain, time
    cascade_splits: vec4f,  // far view distance of each cascade
    shadow: vec4f,          // cascade count, resolution, soft (0/1), shadow distance
    cascade_texel: vec4f,   // world-space texel size per cascade
    cluster_dims: vec4u,    // x, y, z, max lights per cluster
    cluster_z: vec4f,       // near, far, log(far/near), light count
    post: vec4f,            // ssao on, ssr on, spot shadow count, debug mode
    vol_lights: array<vec4u, 8>,
    skyocc_vp: mat4x4f,     // top-down height map view-projection
    skyocc: vec4f,          // top y, depth range, enabled, strength
    bounce: vec4f,          // city albedo rgb (bounce light), 0
};

struct Exposure {
    pre: f32,       // exposure the HDR targets of this frame are scaled by
    current: f32,   // adapted exposure (after this frame's histogram)
    prev_pre: f32,  // pre-exposure of the previous frame (history rescale)
    ev100: f32,
    avg_lum: f32,
    target_ev: f32,
    pad0: f32,
    pad1: f32,
};

@group(0) @binding(0) var<uniform> frame: Frame;
@group(0) @binding(1) var<storage, read> exposure: Exposure;
@group(0) @binding(2) var samp_linear_clamp: sampler;
@group(0) @binding(3) var samp_linear_repeat: sampler;
@group(0) @binding(4) var samp_point_clamp: sampler;
@group(0) @binding(5) var samp_shadow: sampler_comparison;

fn pre_exposure() -> f32 {
    return exposure.pre;
}

fn saturate(x: f32) -> f32 { return clamp(x, 0.0, 1.0); }
fn saturate3(x: vec3f) -> vec3f { return clamp(x, vec3f(0.0), vec3f(1.0)); }
fn sq(x: f32) -> f32 { return x * x; }
fn luminance(c: vec3f) -> f32 { return dot(c, vec3f(0.2126, 0.7152, 0.0722)); }

fn clamp_hdr(c: vec3f) -> vec3f {
    // Removes NaN/Inf and keeps values within f16 range.
    let m = select(c, vec3f(0.0), c != c);
    return clamp(m, vec3f(0.0), vec3f(MAX_HALF));
}

// --- Encoding --------------------------------------------------------------

fn oct_wrap(v: vec2f) -> vec2f {
    return (1.0 - abs(v.yx)) * select(vec2f(-1.0), vec2f(1.0), v.xy >= vec2f(0.0));
}

fn oct_encode(n: vec3f) -> vec2f {
    var p = n.xy / (abs(n.x) + abs(n.y) + abs(n.z));
    if (n.z < 0.0) {
        p = oct_wrap(p);
    }
    return p;
}

fn oct_decode(e: vec2f) -> vec3f {
    var n = vec3f(e.x, e.y, 1.0 - abs(e.x) - abs(e.y));
    let t = saturate(-n.z);
    n.x += select(t, -t, n.x >= 0.0);
    n.y += select(t, -t, n.y >= 0.0);
    return normalize(n);
}

// Duff et al. 2017 branchless orthonormal basis.
fn onb(n: vec3f) -> mat3x3f {
    let s = select(-1.0, 1.0, n.z >= 0.0);
    let a = -1.0 / (s + n.z);
    let b = n.x * n.y * a;
    let t = vec3f(1.0 + s * n.x * n.x * a, s * b, -s * n.x);
    let bt = vec3f(b, s + n.y * n.y * a, -n.y);
    return mat3x3f(t, bt, n);
}

// --- Hashes / noise ---------------------------------------------------------

fn pcg(v: u32) -> u32 {
    let state = v * 747796405u + 2891336453u;
    let word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word;
}

fn hash11(p: f32) -> f32 {
    return f32(pcg(bitcast<u32>(p))) / 4294967295.0;
}

fn hash21(p: vec2f) -> f32 {
    let q = vec2u(vec2i(floor(p)) + vec2i(32768));
    return f32(pcg(q.x + pcg(q.y))) / 4294967295.0;
}

fn hash22(p: vec2f) -> vec2f {
    let q = vec2u(vec2i(floor(p)) + vec2i(32768));
    let h = pcg(q.x + pcg(q.y));
    return vec2f(f32(h & 0xffffu), f32(h >> 16u)) / 65535.0;
}

fn hash31(p: vec3f) -> f32 {
    let q = vec3u(vec3i(floor(p)) + vec3i(32768));
    return f32(pcg(q.x + pcg(q.y + pcg(q.z)))) / 4294967295.0;
}

fn hash33(p: vec3f) -> vec3f {
    let q = vec3u(vec3i(floor(p)) + vec3i(32768));
    let h = pcg(q.x + pcg(q.y + pcg(q.z)));
    let h2 = pcg(h);
    return vec3f(f32(h & 0xffffu) / 65535.0, f32(h >> 16u) / 65535.0, f32(h2 & 0xffffu) / 65535.0);
}

// Interleaved gradient noise (Jimenez), animated per frame.
fn ign(pix: vec2f, frame_index: f32) -> f32 {
    let p = pix + 5.588238 * (frame_index % 64.0);
    return fract(52.9829189 * fract(dot(p, vec2f(0.06711056, 0.00583715))));
}

fn value_noise2(p: vec2f) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash21(i);
    let b = hash21(i + vec2f(1.0, 0.0));
    let c = hash21(i + vec2f(0.0, 1.0));
    let d = hash21(i + vec2f(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

fn value_noise3(p: vec3f) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let n000 = hash31(i);
    let n100 = hash31(i + vec3f(1.0, 0.0, 0.0));
    let n010 = hash31(i + vec3f(0.0, 1.0, 0.0));
    let n110 = hash31(i + vec3f(1.0, 1.0, 0.0));
    let n001 = hash31(i + vec3f(0.0, 0.0, 1.0));
    let n101 = hash31(i + vec3f(1.0, 0.0, 1.0));
    let n011 = hash31(i + vec3f(0.0, 1.0, 1.0));
    let n111 = hash31(i + vec3f(1.0, 1.0, 1.0));
    return mix(
        mix(mix(n000, n100, u.x), mix(n010, n110, u.x), u.y),
        mix(mix(n001, n101, u.x), mix(n011, n111, u.x), u.y),
        u.z,
    );
}

fn fbm2(p: vec2f, octaves: i32) -> f32 {
    var s = 0.0;
    var a = 0.5;
    var q = p;
    for (var i = 0; i < octaves; i++) {
        s += a * value_noise2(q);
        q = mat2x2f(1.6, 1.2, -1.2, 1.6) * q + vec2f(17.1, 3.7);
        a *= 0.5;
    }
    return s;
}

// --- Geometry ---------------------------------------------------------------

// Reversed infinite projection: view distance along -Z from device depth.
fn linear_depth(d: f32) -> f32 {
    return frame.proj_params.x / max(d, 1e-7);
}

fn world_from_depth(uv: vec2f, d: f32) -> vec3f {
    let ndc = vec4f(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, max(d, 1e-7), 1.0);
    let w = frame.inv_view_proj * ndc;
    return w.xyz / w.w;
}

fn view_from_depth(uv: vec2f, d: f32) -> vec3f {
    let ndc = vec4f(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, max(d, 1e-7), 1.0);
    let v = frame.inv_proj * ndc;
    return v.xyz / v.w;
}

fn view_dir_from_uv(uv: vec2f) -> vec3f {
    let ndc = vec4f(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, 1.0, 1.0);
    let w = frame.inv_view_proj * ndc;
    return normalize(w.xyz / w.w - frame.camera_pos.xyz);
}

fn ndc_to_uv(ndc: vec2f) -> vec2f {
    return vec2f(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
}

// --- Fullscreen triangle ----------------------------------------------------

struct FsOut {
    @builtin(position) pos: vec4f,
    @location(0) uv: vec2f,
};

@vertex
fn vs_fullscreen(@builtin(vertex_index) vi: u32) -> FsOut {
    var o: FsOut;
    let x = f32((vi << 1u) & 2u);
    let y = f32(vi & 2u);
    o.pos = vec4f(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
    o.uv = vec2f(x, y);
    return o;
}
