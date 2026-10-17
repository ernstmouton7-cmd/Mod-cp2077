// ---------------------------------------------------------------------------
// particles.wgsl — camera-facing billboards (soft, lit or emissive) and
// procedural rain streaks around the camera. Premultiplied alpha output.

struct GpuParticle {
    pos_size: vec4f,
    color: vec4f,
    vel_rot: vec4f,
    params: vec4f,   // emissive nits, kind, seed, 0
};

struct RainParams {
    count: u32,
    pad0: u32,
    pad1: u32,
    pad2: u32,
};

@group(1) @binding(0) var<storage, read> particles: array<GpuParticle>;
@group(1) @binding(1) var t_depth_copy: texture_2d<f32>;
@group(1) @binding(2) var<uniform> rain: RainParams;

struct PVs {
    @builtin(position) clip: vec4f,
    @location(0) local: vec2f,
    @location(1) world: vec3f,
    @location(2) @interpolate(flat) id: u32,
    @location(3) lit: vec3f,
};

const QUAD: array<vec2f, 6> = array<vec2f, 6>(
    vec2f(-1.0, -1.0), vec2f(1.0, -1.0), vec2f(1.0, 1.0),
    vec2f(-1.0, -1.0), vec2f(1.0, 1.0), vec2f(-1.0, 1.0),
);

// Simple diffuse lighting at a point (sun with 1-tap shadow, sky, local lights).
fn point_lighting(p: vec3f, frag_xy: vec2f, view_z: f32) -> vec3f {
    let h_km = camera_altitude_km();
    let svis = sky_visibility(p, vec3f(0.0, 1.0, 0.0), frag_xy);
    var c = (ibl_diffuse(vec3f(0.0, 1.0, 0.0)) * 0.7 + ibl_diffuse(vec3f(0.0, -1.0, 0.0)) * 0.3) * svis + bounce_radiance() * (1.0 - svis);
    let sh = sun_shadow(p, primary_light_dir(), view_z, frag_xy);
    let moon_primary = frame.light_params.x > 0.5;
    c += sun_illuminance_at(h_km) * select(sh, 1.0, moon_primary) * cloud_shadow(p, frame.sun.xyz) * INV_PI * saturate(frame.sun.y * 4.0 + 0.2);
    c += moon_illuminance_at(h_km) * select(1.0, sh, moon_primary) * INV_PI * saturate(frame.moon.y * 4.0 + 0.2);
    let stride = frame.cluster_dims.w + 1u;
    let base = cluster_of(frag_xy, view_z) * stride;
    let count = min(clusters[base], frame.cluster_dims.w);
    for (var i = 0u; i < count; i++) {
        let light = lights[clusters[base + 1u + i]];
        let dv = light.pos_range.xyz - p;
        let d2 = dot(dv, dv);
        let l = dv * inverseSqrt(max(d2, 1e-6));
        c += light.color.rgb * light_attenuation(d2, light.pos_range.w, max(light.color.w, 0.1)) * spot_cone(light, l) * INV_PI;
    }
    return c;
}

@vertex
fn vs_particle(@builtin(vertex_index) vi: u32, @builtin(instance_index) ii: u32) -> PVs {
    let pt = particles[ii];
    let q = QUAD[vi];
    let right = frame.inv_view[0].xyz;
    let up = frame.inv_view[1].xyz;
    let fwd = -frame.inv_view[2].xyz;
    let kind = u32(pt.params.y);
    var ax = right;
    var ay = up;
    var sx = pt.pos_size.w;
    var sy = pt.pos_size.w;
    let vel = pt.vel_rot.xyz;
    let vlen = length(vel - fwd * dot(vel, fwd));
    if ((kind == 1u || kind == 3u) && vlen > 0.01) {
        // Stretch along the screen-projected velocity (1/30 s exposure).
        ay = normalize(vel - fwd * dot(vel, fwd));
        ax = normalize(cross(ay, fwd));
        sy = pt.pos_size.w + vlen * 0.033;
    } else {
        let r = pt.vel_rot.w;
        let c = cos(r);
        let s = sin(r);
        ax = right * c + up * s;
        ay = -right * s + up * c;
    }
    let world = pt.pos_size.xyz + ax * q.x * sx + ay * q.y * sy;
    var o: PVs;
    o.clip = frame.view_proj * vec4f(world, 1.0);
    o.local = q;
    o.world = world;
    o.id = ii;
    o.lit = vec3f(0.0);
    return o;
}

fn soft_fade(clip: vec4f, softness: f32) -> f32 {
    let sd = textureLoad(t_depth_copy, vec2i(clip.xy), 0).r;
    let scene = select(1e6, linear_depth(sd), sd > 0.0);
    return saturate((scene - clip.w) / max(softness, 1e-3));
}

@fragment
fn fs_particle(i: PVs) -> @location(0) vec4f {
    let pt = particles[i.id];
    let kind = u32(pt.params.y);
    let seed = pt.params.z;
    let r = length(i.local);
    if (r > 1.0) {
        discard;
    }
    let pre = pre_exposure();
    let cam = frame.camera_pos.xyz;
    let dist = length(cam - i.world);
    let view_z = i.clip.w;
    var alpha = 0.0;
    var emis = vec3f(0.0);
    var lit_scale = 1.0;
    if (kind == 0u) {
        let n = fbm2(i.local * 1.7 + vec2f(seed * 31.0, seed * 17.0 + frame.camera_pos.w * 0.05), 4);
        alpha = smoothstep(1.0, 0.15, r) * saturate(n * 1.6 - 0.15);
    } else if (kind == 1u) {
        let core = exp(-sq(i.local.x) * 18.0) * smoothstep(1.0, 0.3, abs(i.local.y));
        emis = pt.color.rgb * pt.params.x * core;
        alpha = 0.0;
        lit_scale = 0.0;
    } else if (kind == 2u) {
        let ang = atan2(i.local.y, i.local.x);
        let edge = 0.65 + 0.25 * value_noise2(vec2f(ang * 2.0 + seed * 40.0, seed));
        alpha = 1.0 - smoothstep(edge - 0.06, edge, r);
    } else if (kind == 3u) {
        let n = value_noise2(i.local * 4.0 + vec2f(seed * 19.0));
        alpha = smoothstep(1.0, 0.2, r) * smoothstep(0.35, 0.7, n);
    } else {
        alpha = smoothstep(1.0, 0.0, r);
    }
    alpha *= pt.color.a * soft_fade(i.clip, pt.pos_size.w * 0.7);
    if (kind == 1u) {
        emis *= soft_fade(i.clip, 0.05);
    }
    var col = vec3f(0.0);
    if (lit_scale > 0.0) {
        col = pt.color.rgb * point_lighting(i.world, i.clip.xy, view_z) * alpha;
    }
    col += emis + pt.color.rgb * pt.params.x * alpha * select(1.0, 0.0, kind == 1u);
    let fog = eval_fog(cam, normalize(i.world - cam), dist);
    let outc = (col * fog.transmittance + fog.inscatter * alpha) * pre;
    return vec4f(clamp_hdr(outc), saturate(alpha));
}

// --- Rain streaks ---------------------------------------------------------------

const RAIN_BOX: vec3f = vec3f(22.0, 14.0, 22.0);

@vertex
fn vs_rain(@builtin(vertex_index) vi: u32, @builtin(instance_index) ii: u32) -> PVs {
    let q = QUAD[vi];
    let h = vec3f(f32(pcg(ii * 3u)), f32(pcg(ii * 3u + 1u)), f32(pcg(ii * 3u + 2u))) / 4294967295.0;
    let speed = 7.5 + h.x * 2.5;
    let wind = vec3f(frame.sky3.y, 0.0, frame.sky3.z) * 0.6;
    let vel = vec3f(0.0, -speed, 0.0) + wind;
    let t = frame.weather.w;
    let cam = frame.camera_pos.xyz;
    var p = h * RAIN_BOX + vel * t;
    p = cam + (fract((p - cam) / RAIN_BOX + 0.5) - 0.5) * RAIN_BOX;
    let fwd = -frame.inv_view[2].xyz;
    let dir = normalize(vel);
    var side = cross(dir, normalize(p - cam));
    side = normalize(side + vec3f(1e-5));
    let dist = length(p - cam);
    // Width at least ~1.2 px to avoid aliasing.
    let px_size = 2.0 * frame.proj_params.y * frame.viewport.w * max(dist, 0.1);
    let w = max(0.0015, px_size * 0.6);
    let len = speed * 0.03;
    let world = p + side * q.x * w + dir * q.y * len * 0.5;
    var o: PVs;
    o.clip = frame.view_proj * vec4f(world, 1.0);
    o.local = vec2f(q.x, q.y);
    o.world = p;
    o.id = ii;
    // Fade near the camera (would be huge) and with distance.
    let fade = smoothstep(0.6, 2.0, dist) * (1.0 - smoothstep(8.0, 11.0, dist));
    o.lit = vec3f(fade * 0.0015 / max(w, 0.0015), 0.0, 0.0);
    return o;
}

@fragment
fn fs_rain(i: PVs) -> @location(0) vec4f {
    let pre = pre_exposure();
    let prof = exp(-i.local.x * i.local.x * 3.0) * smoothstep(1.0, 0.6, abs(i.local.y));
    let alpha = prof * 0.22 * frame.weather.z * min(i.lit.x, 1.0) * soft_fade(i.clip, 0.2);
    if (alpha <= 0.0005) {
        discard;
    }
    // Droplets refract the sky and strongly forward-scatter nearby lights.
    let l = point_lighting(i.world, i.clip.xy, i.clip.w);
    let col = l * 0.9 * alpha;
    return vec4f(clamp_hdr(col * pre), alpha * 0.4);
}
