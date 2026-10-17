// ---------------------------------------------------------------------------
// ssr.wgsl — screen space reflections. Linear screen-space march with
// binary refinement against the current depth buffer; the hit is reprojected
// into the previous frame's lit HDR result. Roughness-aware via VNDF jitter
// (resolved by TAA). Output: rgb = pre-exposed radiance, a = confidence.

struct SsrParams {
    steps: u32,
    max_dist: f32,
    thickness: f32,
    enabled: u32,
};

@group(1) @binding(0) var t_depth: texture_depth_2d;
@group(1) @binding(1) var t_gb1: texture_2d<f32>;
@group(1) @binding(2) var t_gb2: texture_2d<f32>;
@group(1) @binding(3) var t_gb3: texture_2d<f32>;
@group(1) @binding(4) var t_history: texture_2d<f32>;
@group(1) @binding(5) var<uniform> ssr: SsrParams;

fn depth_at(uv: vec2f) -> f32 {
    let dims = vec2f(textureDimensions(t_depth));
    let p = vec2i(clamp(uv * dims, vec2f(0.0), dims - 1.0));
    return textureLoad(t_depth, p, 0);
}

fn project_view(p: vec3f) -> vec3f {
    let c = frame.proj * vec4f(p, 1.0);
    let ndc = c.xyz / c.w;
    return vec3f(ndc_to_uv(ndc.xy), -p.z);
}

@fragment
fn fs_ssr(in: FsOut) -> @location(0) vec4f {
    if (ssr.enabled == 0u) {
        return vec4f(0.0);
    }
    let uv = in.uv;
    let d = depth_at(uv);
    if (d <= 0.0) {
        return vec4f(0.0);
    }
    let dims = vec2f(textureDimensions(t_depth));
    let gp = vec2i(clamp(uv * dims, vec2f(0.0), dims - 1.0));
    let g1 = textureLoad(t_gb1, gp, 0);
    let model = textureLoad(t_gb3, gp, 0).z;
    var rough = g1.z;
    if (model > 0.5 && model < 1.5) {
        rough = min(rough, textureLoad(t_gb2, gp, 0).y);
    }
    if (rough > 0.62 || model > 2.5) {
        return vec4f(0.0);
    }
    let wn = oct_decode(g1.xy);
    let n = normalize((frame.view * vec4f(wn, 0.0)).xyz);
    let p = view_from_depth(uv, d);
    let v = normalize(-p);
    var r = reflect(-v, n);
    let noise = vec2f(ign(in.pos.xy, frame.proj_params.w), ign(in.pos.xy + vec2f(37.0, 11.0), frame.proj_params.w));
    if (rough > 0.04) {
        let a = rough * rough;
        let b = onb(n);
        let ve = vec3f(dot(v, b[0]), dot(v, b[1]), dot(v, b[2]));
        let h = sample_ggx_vndf(ve, a, noise * vec2f(0.9, 1.0));
        let hw = b * h;
        r = reflect(-v, hw);
        if (dot(r, n) <= 0.0) {
            r = reflect(-v, n);
        }
    }
    // Clip the ray against the near plane.
    var len = ssr.max_dist;
    if (r.z > 0.0) {
        len = min(len, (-frame.proj_params.x * 1.5 - p.z) / r.z);
    }
    if (len <= 0.05) {
        return vec4f(0.0);
    }
    let s0 = project_view(p);
    let s1 = project_view(p + r * len);
    let steps = ssr.steps;
    let inv_z0 = 1.0 / s0.z;
    let inv_z1 = 1.0 / s1.z;
    var hit = false;
    var t_prev = 0.0;
    var t_hit = 0.0;
    let jitter = noise.x;
    for (var i = 1u; i <= steps; i++) {
        var t = (f32(i) - 1.0 + jitter) / f32(steps);
        t = t * t * 0.5 + t * 0.5;
        let suv = mix(s0.xy, s1.xy, t);
        if (any(suv < vec2f(0.0)) || any(suv > vec2f(1.0))) {
            break;
        }
        let ray_d = 1.0 / mix(inv_z0, inv_z1, t);
        let sd = depth_at(suv);
        let scene_d = linear_depth(sd);
        let thick = max(ssr.thickness, scene_d * 0.04);
        if (ray_d > scene_d && ray_d < scene_d + thick && sd > 0.0) {
            hit = true;
            t_hit = t;
            break;
        }
        t_prev = t;
    }
    if (!hit) {
        return vec4f(0.0);
    }
    // Binary refinement.
    var lo = t_prev;
    var hi = t_hit;
    for (var k = 0u; k < 6u; k++) {
        let mid = 0.5 * (lo + hi);
        let suv = mix(s0.xy, s1.xy, mid);
        let ray_d = 1.0 / mix(inv_z0, inv_z1, mid);
        let scene_d = linear_depth(depth_at(suv));
        if (ray_d > scene_d) {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    let huv = mix(s0.xy, s1.xy, hi);
    let hd = depth_at(huv);
    let hw = world_from_depth(huv, hd);
    let pc = frame.prev_view_proj * vec4f(hw, 1.0);
    let puv = ndc_to_uv(pc.xy / pc.w);
    if (pc.w <= 0.0 || any(puv < vec2f(0.0)) || any(puv > vec2f(1.0))) {
        return vec4f(0.0);
    }
    let col = textureSampleLevel(t_history, samp_linear_clamp, puv, 0.0).rgb * (exposure.pre / max(exposure.prev_pre, 1e-12));
    let edge = vec2f(1.0) - smoothstep(vec2f(0.82), vec2f(1.0), abs(huv * 2.0 - 1.0));
    var conf = edge.x * edge.y;
    conf *= 1.0 - smoothstep(0.3, 0.62, rough);
    conf *= 1.0 - smoothstep(0.55, 0.95, r.z);
    conf *= 1.0 - smoothstep(0.8, 1.0, hi);
    return vec4f(clamp_hdr(col), saturate(conf));
}
