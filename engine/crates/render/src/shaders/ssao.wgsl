// ---------------------------------------------------------------------------
// ssao.wgsl — ground-truth ambient occlusion (XeGTAO style) + depth-aware blur.

@group(1) @binding(0) var t_depth: texture_depth_2d;
@group(1) @binding(1) var t_gb1: texture_2d<f32>;
@group(1) @binding(2) var t_ao_in: texture_2d<f32>;

const AO_RADIUS: f32 = 1.2;
const AO_SLICES: u32 = 2u;
const AO_STEPS: u32 = 6u;

fn load_depth_uv(uv: vec2f) -> f32 {
    let dims = vec2f(textureDimensions(t_depth));
    let p = vec2i(clamp(uv * dims, vec2f(0.0), dims - 1.0));
    return textureLoad(t_depth, p, 0);
}

fn fast_acos(x: f32) -> f32 {
    let ax = abs(x);
    var r = (-0.156583 * ax + 0.5 * PI) * sqrt(1.0 - ax);
    return select(PI - r, r, x >= 0.0);
}

@fragment
fn fs_gtao(in: FsOut) -> @location(0) vec4f {
    let uv = in.uv;
    let d = load_depth_uv(uv);
    if (d <= 0.0) {
        return vec4f(1.0);
    }
    let p = view_from_depth(uv, d);
    let dims = vec2f(textureDimensions(t_depth));
    let gp = vec2i(clamp(uv * dims, vec2f(0.0), dims - 1.0));
    let wn = oct_decode(textureLoad(t_gb1, gp, 0).xy);
    let n = normalize((frame.view * vec4f(wn, 0.0)).xyz);
    let v = normalize(-p);
    let proj_scale = frame.viewport.y * 0.5 / frame.proj_params.y;
    let screen_radius = AO_RADIUS * proj_scale / max(-p.z, 1e-3);
    if (screen_radius < 1.5) {
        return vec4f(1.0);
    }
    let noise_a = ign(in.pos.xy, frame.proj_params.w);
    let noise_b = fract(noise_a * 7.31 + 0.37);
    let px = frame.viewport.zw;
    var vis = 0.0;
    for (var s = 0u; s < AO_SLICES; s++) {
        let phi = (f32(s) + noise_a) / f32(AO_SLICES) * PI;
        let dir = vec3f(cos(phi), sin(phi), 0.0);
        let omega = vec2f(cos(phi), -sin(phi)) * screen_radius;
        let ortho = dir - dot(dir, v) * v;
        let axis = normalize(cross(ortho, v));
        let pn = n - axis * dot(n, axis);
        let pn_len = length(pn);
        let sign_n = sign(dot(ortho, pn));
        let cos_n = saturate(dot(pn, v) / max(pn_len, 1e-5));
        let nang = sign_n * fast_acos(cos_n);
        let low0 = cos(nang + 0.5 * PI);
        let low1 = cos(nang - 0.5 * PI);
        var hc0 = low0;
        var hc1 = low1;
        for (var k = 0u; k < AO_STEPS; k++) {
            var t = (f32(k) + noise_b) / f32(AO_STEPS);
            t = t * t + 1.0 / screen_radius;
            let off = omega * t * px;
            let uv0 = uv + off;
            let uv1 = uv - off;
            let s0 = view_from_depth(uv0, load_depth_uv(uv0)) - p;
            let s1 = view_from_depth(uv1, load_depth_uv(uv1)) - p;
            let l0 = length(s0);
            let l1 = length(s1);
            let w0 = saturate(1.0 - sq(l0 / AO_RADIUS));
            let w1 = saturate(1.0 - sq(l1 / AO_RADIUS));
            hc0 = max(hc0, mix(low0, dot(s0 / max(l0, 1e-5), v), w0));
            hc1 = max(hc1, mix(low1, dot(s1 / max(l1, 1e-5), v), w1));
        }
        var h0 = -fast_acos(hc1);
        var h1 = fast_acos(hc0);
        h0 = nang + clamp(h0 - nang, -0.5 * PI, 0.5 * PI);
        h1 = nang + clamp(h1 - nang, -0.5 * PI, 0.5 * PI);
        let sn = sin(nang);
        let ia0 = (cos_n + 2.0 * h0 * sn - cos(2.0 * h0 - nang)) * 0.25;
        let ia1 = (cos_n + 2.0 * h1 * sn - cos(2.0 * h1 - nang)) * 0.25;
        vis += pn_len * (ia0 + ia1);
    }
    var ao = saturate(vis / f32(AO_SLICES));
    ao = pow(ao, 1.25);
    return vec4f(ao, linear_depth(d), 0.0, 1.0);
}

// 4x4 depth aware blur (input: ao in r, linear depth in g).
@fragment
fn fs_ao_blur(in: FsOut) -> @location(0) vec4f {
    let dims = vec2i(textureDimensions(t_ao_in));
    let c = vec2i(in.pos.xy);
    let center = textureLoad(t_ao_in, c, 0);
    var sum = 0.0;
    var w = 0.0;
    for (var y = -2; y <= 1; y++) {
        for (var x = -2; x <= 1; x++) {
            let p = clamp(c + vec2i(x, y), vec2i(0), dims - 1);
            let s = textureLoad(t_ao_in, p, 0);
            let dw = exp(-abs(s.g - center.g) / max(center.g * 0.04, 0.02));
            sum += s.r * dw;
            w += dw;
        }
    }
    let ao = sum / max(w, 1e-4);
    return vec4f(ao, center.g, 0.0, 1.0);
}
