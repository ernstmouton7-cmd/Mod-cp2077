// ---------------------------------------------------------------------------
// taa.wgsl — temporal anti-aliasing + upsampling (render res -> output res).
// Gaussian reconstruction of the jittered current frame, velocity dilation,
// Catmull-Rom history, YCoCg variance clipping.

struct TaaParams {
    blend: f32,
    reset: f32,
    enabled: f32,
    pad: f32,
};

@group(1) @binding(0) var t_cur: texture_2d<f32>;
@group(1) @binding(1) var t_depth: texture_depth_2d;
@group(1) @binding(2) var t_gb3: texture_2d<f32>;
@group(1) @binding(3) var t_hist: texture_2d<f32>;
@group(1) @binding(4) var<uniform> taa: TaaParams;

fn rgb_to_ycocg(c: vec3f) -> vec3f {
    return vec3f(0.25 * c.r + 0.5 * c.g + 0.25 * c.b, 0.5 * c.r - 0.5 * c.b, -0.25 * c.r + 0.5 * c.g - 0.25 * c.b);
}

fn ycocg_to_rgb(c: vec3f) -> vec3f {
    return vec3f(c.x + c.y - c.z, c.x + c.z, c.x - c.y - c.z);
}

fn tm(c: vec3f) -> vec3f {
    return c / (1.0 + luminance(c));
}

fn tm_inv(c: vec3f) -> vec3f {
    return c / max(1.0 - luminance(c), 1e-4);
}

fn sample_history(uv: vec2f) -> vec3f {
    // 5-tap Catmull-Rom (Jimenez).
    let dims = vec2f(textureDimensions(t_hist));
    let pos = uv * dims;
    let tc = floor(pos - 0.5) + 0.5;
    let f = pos - tc;
    let w0 = f * (-0.5 + f * (1.0 - 0.5 * f));
    let w1 = 1.0 + f * f * (-2.5 + 1.5 * f);
    let w2 = f * (0.5 + f * (2.0 - 1.5 * f));
    let w3 = f * f * (-0.5 + 0.5 * f);
    let w12 = w1 + w2;
    let tc0 = (tc - 1.0) / dims;
    let tc3 = (tc + 2.0) / dims;
    let tc12 = (tc + w2 / w12) / dims;
    var c = textureSampleLevel(t_hist, samp_linear_clamp, vec2f(tc12.x, tc0.y), 0.0).rgb * (w12.x * w0.y);
    c += textureSampleLevel(t_hist, samp_linear_clamp, vec2f(tc0.x, tc12.y), 0.0).rgb * (w0.x * w12.y);
    c += textureSampleLevel(t_hist, samp_linear_clamp, vec2f(tc12.x, tc12.y), 0.0).rgb * (w12.x * w12.y);
    c += textureSampleLevel(t_hist, samp_linear_clamp, vec2f(tc3.x, tc12.y), 0.0).rgb * (w3.x * w12.y);
    c += textureSampleLevel(t_hist, samp_linear_clamp, vec2f(tc12.x, tc3.y), 0.0).rgb * (w12.x * w3.y);
    let wsum = w12.x * w0.y + w0.x * w12.y + w12.x * w12.y + w3.x * w12.y + w12.x * w3.y;
    return max(c / wsum, vec3f(0.0));
}

@fragment
fn fs_taa(in: FsOut) -> @location(0) vec4f {
    let uv = in.uv;
    let rdims = vec2f(textureDimensions(t_cur));
    let rdi = vec2i(rdims);
    // Position of the output pixel centre in the jittered render image.
    let jit_uv = vec2f(frame.jitter.x * 0.5, -frame.jitter.y * 0.5) * taa.enabled;
    let rp = (uv + jit_uv) * rdims - 0.5;
    let base = vec2i(floor(rp + 0.5));
    var sum = vec3f(0.0);
    var wsum = 0.0;
    var m1 = vec3f(0.0);
    var m2 = vec3f(0.0);
    var best_d = 0.0;
    var best_p = clamp(base, vec2i(0), rdi - 1);
    var wmax = 0.0;
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            let p = clamp(base + vec2i(x, y), vec2i(0), rdi - 1);
            let c = clamp_hdr(textureLoad(t_cur, p, 0).rgb);
            let dv = vec2f(p) - rp;
            let w = exp(-2.29 * dot(dv, dv));
            let tc = tm(c);
            sum += tc * w;
            wsum += w;
            wmax = max(wmax, w);
            let yc = rgb_to_ycocg(tc);
            m1 += yc;
            m2 += yc * yc;
            let d = textureLoad(t_depth, p, 0);
            if (d > best_d) {
                best_d = d;
                best_p = p;
            }
        }
    }
    let cur = sum / max(wsum, 1e-5);
    if (taa.enabled < 0.5 || taa.reset > 0.5) {
        let c = select(tm_inv(cur), textureSampleLevel(t_cur, samp_linear_clamp, uv, 0.0).rgb, taa.enabled < 0.5);
        return vec4f(clamp_hdr(c), 1.0);
    }
    var vel = textureLoad(t_gb3, best_p, 0).xy;
    if (best_d <= 0.0) {
        // Sky: reproject the view direction with camera rotation only.
        let dir = view_dir_from_uv(uv);
        let pc = frame.prev_view_proj * vec4f(dir, 0.0);
        let cc = frame.view_proj_nj * vec4f(dir, 0.0);
        vel = ndc_to_uv(cc.xy / cc.w) - ndc_to_uv(pc.xy / pc.w);
    }
    let puv = uv - vel;
    let mean = m1 / 9.0;
    let sigma = sqrt(max(m2 / 9.0 - mean * mean, vec3f(0.0)));
    let gamma = 1.1;
    let lo = mean - sigma * gamma;
    let hi = mean + sigma * gamma;
    var blend = taa.blend;
    if (any(puv < vec2f(0.0)) || any(puv > vec2f(1.0))) {
        blend = 1.0;
    }
    let hist_rgb = sample_history(puv) * (exposure.pre / max(exposure.prev_pre, 1e-12));
    var hist = rgb_to_ycocg(tm(clamp_hdr(hist_rgb)));
    // Clip towards the mean (Karis / Salvi variance clipping).
    let center = (lo + hi) * 0.5;
    let ext = max((hi - lo) * 0.5, vec3f(1e-5));
    let off = hist - center;
    let ts = abs(off / ext);
    let mx = max(ts.x, max(ts.y, ts.z));
    if (mx > 1.0) {
        hist = center + off / mx;
    }
    // Faster convergence when the current sample is close to this output pixel (upsampling).
    let speed = length(vel * frame.output.xy);
    blend = max(blend * mix(0.6, 1.4, saturate(wmax)), saturate(speed * 0.02) * 0.2 + blend * 0.5);
    let res = mix(ycocg_to_rgb(hist), cur, saturate(blend));
    return vec4f(clamp_hdr(tm_inv(res)), 1.0);
}
