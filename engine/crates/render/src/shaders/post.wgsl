// ---------------------------------------------------------------------------
// post.wgsl — final composite: sharpen, motion blur, bloom, exposure,
// vignette, chromatic aberration, tonemapping (AgX / ACES / Neutral), grain,
// dithering; plus an FXAA fallback pass.

override OUTPUT_ENCODE_SRGB: bool = false;

struct PostParams {
    bloom: f32,
    vignette: f32,
    grain: f32,
    ca: f32,
    motion_blur: f32,
    sharpen: f32,
    tonemapper: f32,
    ldr_out: f32,      // 1 = write gamma-encoded + luma (FXAA input)
    bloom_norm: f32,
    pad0: f32,
    pad1: f32,
    pad2: f32,
};

@group(1) @binding(0) var t_hdr: texture_2d<f32>;
@group(1) @binding(1) var t_bloom: texture_2d<f32>;
@group(1) @binding(2) var t_vel: texture_2d<f32>;
@group(1) @binding(3) var<uniform> pp: PostParams;
@group(1) @binding(4) var t_ldr: texture_2d<f32>;

fn srgb_encode(c: vec3f) -> vec3f {
    let lo = c * 12.92;
    let hi = 1.055 * pow(max(c, vec3f(0.0)), vec3f(1.0 / 2.4)) - 0.055;
    return select(hi, lo, c <= vec3f(0.0031308));
}

fn srgb_decode(c: vec3f) -> vec3f {
    let lo = c / 12.92;
    let hi = pow((max(c, vec3f(0.0)) + 0.055) / 1.055, vec3f(2.4));
    return select(hi, lo, c <= vec3f(0.04045));
}

// --- AgX (Sobotka; polynomial fit by Wrensch) -------------------------------
fn agx_contrast(x: vec3f) -> vec3f {
    let x2 = x * x;
    let x4 = x2 * x2;
    return 15.5 * x4 * x2 - 40.14 * x4 * x + 31.96 * x4 - 6.868 * x2 * x + 0.4298 * x2 + 0.1191 * x - 0.00232;
}

fn agx(c: vec3f) -> vec3f {
    let m = mat3x3f(
        vec3f(0.842479062253094, 0.0423282422610123, 0.0423756549057051),
        vec3f(0.0784335999999992, 0.878468636469772, 0.0784336),
        vec3f(0.0792237451477643, 0.0791661274605434, 0.879142973793104),
    );
    let min_ev = -12.47393;
    let max_ev = 4.026069;
    var v = m * max(c, vec3f(1e-10));
    v = clamp(log2(v), vec3f(min_ev), vec3f(max_ev));
    v = (v - min_ev) / (max_ev - min_ev);
    v = agx_contrast(v);
    // Gentle look: a touch more contrast and saturation than base AgX.
    let luma = dot(v, vec3f(0.2126, 0.7152, 0.0722));
    v = pow(max(v, vec3f(0.0)), vec3f(1.08));
    v = luma + 1.03 * (v - luma);
    let mi = mat3x3f(
        vec3f(1.19687900512017, -0.0528968517574562, -0.0529716355144438),
        vec3f(-0.0980208811401368, 1.15190312990417, -0.0980434501171241),
        vec3f(-0.0990297440797205, -0.0989611768448433, 1.15107367264116),
    );
    v = mi * v;
    return pow(max(v, vec3f(0.0)), vec3f(2.2));
}

fn aces(c: vec3f) -> vec3f {
    let i = mat3x3f(vec3f(0.59719, 0.07600, 0.02840), vec3f(0.35458, 0.90834, 0.13383), vec3f(0.04823, 0.01566, 0.83777));
    let o = mat3x3f(vec3f(1.60475, -0.10208, -0.00327), vec3f(-0.53108, 1.10813, -0.07276), vec3f(-0.07367, -0.00605, 1.07602));
    let v = i * c;
    let a = v * (v + 0.0245786) - 0.000090537;
    let b = v * (0.983729 * v + 0.4329510) + 0.238081;
    return saturate3(o * (a / b));
}

fn neutral(c_in: vec3f) -> vec3f {
    let start = 0.8 - 0.04;
    let desat = 0.15;
    let x = min(c_in.r, min(c_in.g, c_in.b));
    let off = select(0.04, x - 6.25 * x * x, x < 0.08);
    var c = c_in - off;
    let peak = max(c.r, max(c.g, c.b));
    if (peak < start) {
        return c;
    }
    let d = 1.0 - start;
    let np = 1.0 - d * d / (peak + d - start);
    c *= np / peak;
    let g = 1.0 - 1.0 / (desat * (peak - np) + 1.0);
    return mix(c, vec3f(np), g);
}

fn hdr_at(uv: vec2f) -> vec3f {
    return textureSampleLevel(t_hdr, samp_linear_clamp, uv, 0.0).rgb;
}

@fragment
fn fs_tonemap(in: FsOut) -> @location(0) vec4f {
    let uv = in.uv;
    let ts = frame.output.zw;
    var c = hdr_at(uv);
    if (pp.ca > 0.0) {
        let dvec = (uv - 0.5) * pp.ca * 0.012;
        c = vec3f(hdr_at(uv + dvec).r, c.g, hdr_at(uv - dvec).b);
    }
    if (pp.sharpen > 0.0) {
        let n = hdr_at(uv + vec2f(0.0, -ts.y));
        let s = hdr_at(uv + vec2f(0.0, ts.y));
        let e = hdr_at(uv + vec2f(ts.x, 0.0));
        let w = hdr_at(uv + vec2f(-ts.x, 0.0));
        let avg = (n + s + e + w) * 0.25;
        // Sharpen in a perceptual domain to avoid ringing on highlights.
        let lc = luminance(c);
        let la = luminance(avg);
        let k = pp.sharpen * saturate(1.0 - abs(lc - la) / max(lc + la, 1e-4) * 2.0);
        c = max(c + (c - avg) * k, vec3f(0.0));
    }
    if (pp.motion_blur > 0.0) {
        let vel = textureSampleLevel(t_vel, samp_point_clamp, uv, 0.0).xy * pp.motion_blur;
        if (dot(vel, vel) * dot(frame.output.xy, frame.output.xy) > 1.0) {
            var acc = c;
            let noise = ign(in.pos.xy, frame.proj_params.w) - 0.5;
            for (var i = 1; i <= 8; i++) {
                let t = (f32(i) + noise) / 8.0 - 0.5;
                acc += hdr_at(uv + vel * t);
            }
            c = acc / 9.0;
        }
    }
    if (pp.bloom > 0.0) {
        let b = textureSampleLevel(t_bloom, samp_linear_clamp, uv, 0.0).rgb * pp.bloom_norm;
        c = mix(c, b, pp.bloom);
    }
    c *= exposure.current / max(exposure.pre, 1e-12);
    if (pp.vignette > 0.0) {
        let d = (uv - 0.5) * vec2f(frame.output.x * frame.output.w, 1.0);
        let r2 = dot(d, d) * 1.6;
        let cos4 = 1.0 / sq(1.0 + r2);
        c *= mix(1.0, cos4, pp.vignette);
    }
    var o: vec3f;
    let tmo = u32(pp.tonemapper);
    if (tmo == 1u) {
        o = aces(c * 1.6);
    } else if (tmo == 2u) {
        o = neutral(c * 1.6);
    } else {
        o = agx(c * 1.6);
    }
    o = saturate3(o);
    var enc = srgb_encode(o);
    if (pp.grain > 0.0) {
        let g = hash21(in.pos.xy + vec2f(frame.proj_params.w * 13.0, frame.proj_params.w * 7.0)) - 0.5;
        let l = luminance(o);
        enc += g * pp.grain * (1.0 - l * 0.7);
    }
    // Triangular dither to avoid banding in skies and fog.
    let dn = hash21(in.pos.xy * 1.37 + vec2f(frame.proj_params.w)) + hash21(in.pos.xy * 0.71 + vec2f(91.0, frame.proj_params.w)) - 1.0;
    enc += dn / 255.0;
    enc = saturate3(enc);
    if (pp.ldr_out > 0.5) {
        return vec4f(enc, dot(enc, vec3f(0.299, 0.587, 0.114)));
    }
    if (OUTPUT_ENCODE_SRGB) {
        return vec4f(enc, 1.0);
    }
    return vec4f(srgb_decode(enc), 1.0);
}

// FXAA 3.11 "console" variant on gamma encoded input (luma in alpha).
@fragment
fn fs_fxaa(in: FsOut) -> @location(0) vec4f {
    let uv = in.uv;
    let ts = frame.output.zw;
    let m = textureSampleLevel(t_ldr, samp_linear_clamp, uv, 0.0);
    let nw = textureSampleLevel(t_ldr, samp_linear_clamp, uv + vec2f(-0.5, -0.5) * ts, 0.0).a;
    let ne = textureSampleLevel(t_ldr, samp_linear_clamp, uv + vec2f(0.5, -0.5) * ts, 0.0).a;
    let sw = textureSampleLevel(t_ldr, samp_linear_clamp, uv + vec2f(-0.5, 0.5) * ts, 0.0).a;
    let se = textureSampleLevel(t_ldr, samp_linear_clamp, uv + vec2f(0.5, 0.5) * ts, 0.0).a;
    let lmax = max(max(nw, ne), max(sw, se));
    let lmin = min(min(nw, ne), min(sw, se));
    var outc = m.rgb;
    if (max(lmax, m.a) - min(lmin, m.a) > max(0.05, lmax * 0.125)) {
        var dir = vec2f(-((nw + ne) - (sw + se)), (nw + sw) - (ne + se));
        let red = max((nw + ne + sw + se) * 0.03125, 1.0 / 128.0);
        let rmin = 1.0 / (min(abs(dir.x), abs(dir.y)) + red);
        dir = clamp(dir * rmin, vec2f(-8.0), vec2f(8.0)) * ts;
        let a = 0.5 * (textureSampleLevel(t_ldr, samp_linear_clamp, uv - dir / 6.0, 0.0).rgb
            + textureSampleLevel(t_ldr, samp_linear_clamp, uv + dir / 6.0, 0.0).rgb);
        let b = a * 0.5 + 0.25 * (textureSampleLevel(t_ldr, samp_linear_clamp, uv - dir * 0.5, 0.0).rgb
            + textureSampleLevel(t_ldr, samp_linear_clamp, uv + dir * 0.5, 0.0).rgb);
        let lb = dot(b, vec3f(0.299, 0.587, 0.114));
        outc = select(b, a, lb < min(lmin, m.a) || lb > max(lmax, m.a));
    }
    if (OUTPUT_ENCODE_SRGB) {
        return vec4f(outc, 1.0);
    }
    return vec4f(srgb_decode(outc), 1.0);
}
