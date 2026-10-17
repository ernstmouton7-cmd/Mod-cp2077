// ---------------------------------------------------------------------------
// bloom.wgsl — physically based bloom: 13-tap downsample chain (Karis average
// on the first level) and 9-tap tent upsample with additive blending.

@group(1) @binding(0) var t_src: texture_2d<f32>;

fn karis(c: vec3f) -> f32 {
    return 1.0 / (1.0 + luminance(c));
}

fn down13(uv: vec2f, karis_avg: bool) -> vec3f {
    let ts = 1.0 / vec2f(textureDimensions(t_src));
    let a = textureSampleLevel(t_src, samp_linear_clamp, uv + ts * vec2f(-2.0, -2.0), 0.0).rgb;
    let b = textureSampleLevel(t_src, samp_linear_clamp, uv + ts * vec2f(0.0, -2.0), 0.0).rgb;
    let c = textureSampleLevel(t_src, samp_linear_clamp, uv + ts * vec2f(2.0, -2.0), 0.0).rgb;
    let d = textureSampleLevel(t_src, samp_linear_clamp, uv + ts * vec2f(-2.0, 0.0), 0.0).rgb;
    let e = textureSampleLevel(t_src, samp_linear_clamp, uv, 0.0).rgb;
    let f = textureSampleLevel(t_src, samp_linear_clamp, uv + ts * vec2f(2.0, 0.0), 0.0).rgb;
    let g = textureSampleLevel(t_src, samp_linear_clamp, uv + ts * vec2f(-2.0, 2.0), 0.0).rgb;
    let h = textureSampleLevel(t_src, samp_linear_clamp, uv + ts * vec2f(0.0, 2.0), 0.0).rgb;
    let i = textureSampleLevel(t_src, samp_linear_clamp, uv + ts * vec2f(2.0, 2.0), 0.0).rgb;
    let j = textureSampleLevel(t_src, samp_linear_clamp, uv + ts * vec2f(-1.0, -1.0), 0.0).rgb;
    let k = textureSampleLevel(t_src, samp_linear_clamp, uv + ts * vec2f(1.0, -1.0), 0.0).rgb;
    let l = textureSampleLevel(t_src, samp_linear_clamp, uv + ts * vec2f(-1.0, 1.0), 0.0).rgb;
    let m = textureSampleLevel(t_src, samp_linear_clamp, uv + ts * vec2f(1.0, 1.0), 0.0).rgb;
    if (karis_avg) {
        let g0 = (a + b + d + e) * 0.25;
        let g1 = (b + c + e + f) * 0.25;
        let g2 = (d + e + g + h) * 0.25;
        let g3 = (e + f + h + i) * 0.25;
        let g4 = (j + k + l + m) * 0.25;
        let w0 = karis(g0) * 0.125;
        let w1 = karis(g1) * 0.125;
        let w2 = karis(g2) * 0.125;
        let w3 = karis(g3) * 0.125;
        let w4 = karis(g4) * 0.5;
        return (g0 * w0 + g1 * w1 + g2 * w2 + g3 * w3 + g4 * w4) / (w0 + w1 + w2 + w3 + w4);
    }
    var o = e * 0.125;
    o += (a + c + g + i) * 0.03125;
    o += (b + d + f + h) * 0.0625;
    o += (j + k + l + m) * 0.125;
    return o;
}

@fragment
fn fs_down_first(in: FsOut) -> @location(0) vec4f {
    return vec4f(clamp_hdr(down13(in.uv, true)), 1.0);
}

@fragment
fn fs_down(in: FsOut) -> @location(0) vec4f {
    return vec4f(down13(in.uv, false), 1.0);
}

@fragment
fn fs_up(in: FsOut) -> @location(0) vec4f {
    let ts = 1.0 / vec2f(textureDimensions(t_src));
    let r = 1.0;
    var o = textureSampleLevel(t_src, samp_linear_clamp, in.uv, 0.0).rgb * 4.0;
    o += textureSampleLevel(t_src, samp_linear_clamp, in.uv + ts * vec2f(-r, 0.0), 0.0).rgb * 2.0;
    o += textureSampleLevel(t_src, samp_linear_clamp, in.uv + ts * vec2f(r, 0.0), 0.0).rgb * 2.0;
    o += textureSampleLevel(t_src, samp_linear_clamp, in.uv + ts * vec2f(0.0, -r), 0.0).rgb * 2.0;
    o += textureSampleLevel(t_src, samp_linear_clamp, in.uv + ts * vec2f(0.0, r), 0.0).rgb * 2.0;
    o += textureSampleLevel(t_src, samp_linear_clamp, in.uv + ts * vec2f(-r, -r), 0.0).rgb;
    o += textureSampleLevel(t_src, samp_linear_clamp, in.uv + ts * vec2f(r, -r), 0.0).rgb;
    o += textureSampleLevel(t_src, samp_linear_clamp, in.uv + ts * vec2f(-r, r), 0.0).rgb;
    o += textureSampleLevel(t_src, samp_linear_clamp, in.uv + ts * vec2f(r, r), 0.0).rgb;
    return vec4f(o / 16.0, 1.0);
}
