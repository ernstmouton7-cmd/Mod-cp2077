// ---------------------------------------------------------------------------
// lighting.wgsl — shared lighting resources (bind group 3) and evaluation:
// cascaded sun shadows, clustered point/spot lights, IBL, fog, volumetrics.

struct GpuLight {
    pos_range: vec4f,  // position, range
    color: vec4f,      // rgb * candela, source radius
    dir_cos: vec4f,    // spot direction, cos(outer) (-2 for point lights)
    params: vec4f,     // cos(inner), shadow layer (-1 = none), volumetric, 0
};

@group(3) @binding(0) var t_shadow: texture_depth_2d_array;
@group(3) @binding(1) var t_spot_shadow: texture_depth_2d_array;
@group(3) @binding(2) var t_sky_cube: texture_cube<f32>;
@group(3) @binding(3) var t_hdri_cube: texture_cube<f32>;
@group(3) @binding(4) var t_brdf_lut: texture_2d<f32>;
@group(3) @binding(5) var t_transmittance: texture_2d<f32>;
@group(3) @binding(6) var t_skyview: texture_2d<f32>;
@group(3) @binding(7) var t_ao: texture_2d<f32>;
@group(3) @binding(8) var t_ssr: texture_2d<f32>;
@group(3) @binding(9) var<storage, read> lights: array<GpuLight>;
@group(3) @binding(10) var<storage, read> clusters: array<u32>;
@group(3) @binding(11) var<storage, read> sh: array<vec4f, 18>;
@group(3) @binding(12) var<storage, read> spot_mats: array<mat4x4f>;
@group(3) @binding(13) var t_skyocc: texture_depth_2d;

const SHADOW_DEPTH_RANGE: f32 = 800.0;

const POISSON16: array<vec2f, 16> = array<vec2f, 16>(
    vec2f(-0.94201624, -0.39906216), vec2f(0.94558609, -0.76890725),
    vec2f(-0.09418410, -0.92938870), vec2f(0.34495938, 0.29387760),
    vec2f(-0.91588581, 0.45771432), vec2f(-0.81544232, -0.87912464),
    vec2f(-0.38277543, 0.27676845), vec2f(0.97484398, 0.75648379),
    vec2f(0.44323325, -0.97511554), vec2f(0.53742981, -0.47373420),
    vec2f(-0.26496911, -0.41893023), vec2f(0.79197514, 0.19090188),
    vec2f(-0.24188840, 0.99706507), vec2f(-0.81409955, 0.91437590),
    vec2f(0.19984126, 0.78641367), vec2f(0.14383161, -0.14100790),
);

fn camera_altitude_km() -> f32 {
    return frame.sky3.w;
}

// Direction and illuminance of the shadow-casting directional light (sun or moon).
fn primary_light_dir() -> vec3f {
    return select(frame.sun.xyz, frame.moon.xyz, frame.light_params.x > 0.5);
}

fn sun_illuminance_at(h_km: f32) -> vec3f {
    return frame.sun.w * light_transmittance(t_transmittance, samp_linear_clamp, h_km, frame.sun.y);
}

fn moon_illuminance_at(h_km: f32) -> vec3f {
    return frame.moon.w * light_transmittance(t_transmittance, samp_linear_clamp, h_km, frame.moon.y) * vec3f(0.9, 0.95, 1.05);
}

// --- Shadows ------------------------------------------------------------------

fn cascade_lookup(c: u32, world: vec3f) -> vec3f {
    let p = frame.cascade_vp[c] * vec4f(world, 1.0);
    return vec3f(ndc_to_uv(p.xy), p.z);
}

fn pcf_cascade(c: u32, world: vec3f, radius_texels: f32, rot: vec2f) -> f32 {
    let lp = cascade_lookup(c, world);
    if (any(lp.xy < vec2f(0.0)) || any(lp.xy > vec2f(1.0)) || lp.z > 1.0) {
        return 1.0;
    }
    let texel = 1.0 / frame.shadow.y;
    var radius = radius_texels;
    if (frame.shadow.z > 0.5) {
        // PCSS-lite: average blocker depth widens the kernel.
        var blocker = 0.0;
        var nb = 0.0;
        let dims = vec2f(frame.shadow.y);
        for (var i = 0u; i < 8u; i++) {
            let o = POISSON16[i * 2u];
            let off = vec2f(o.x * rot.x - o.y * rot.y, o.x * rot.y + o.y * rot.x) * 6.0 * texel;
            let tc = vec2i(clamp((lp.xy + off) * dims, vec2f(0.0), dims - 1.0));
            let d = textureLoad(t_shadow, tc, i32(c), 0);
            if (d < lp.z - 0.0005) {
                blocker += d;
                nb += 1.0;
            }
        }
        if (nb < 0.5) {
            return 1.0;
        }
        blocker /= nb;
        // Penumbra from the sun's angular diameter (0.53 deg) in texels.
        let pen = (lp.z - blocker) * SHADOW_DEPTH_RANGE * 0.0093 / max(frame.cascade_texel[c], 1e-4);
        radius = clamp(radius_texels + pen, radius_texels, 10.0);
    }
    var sum = 0.0;
    for (var i = 0u; i < 16u; i++) {
        let o = POISSON16[i];
        let off = vec2f(o.x * rot.x - o.y * rot.y, o.x * rot.y + o.y * rot.x) * radius * texel;
        sum += textureSampleCompareLevel(t_shadow, samp_shadow, lp.xy + off, i32(c), lp.z);
    }
    return sum / 16.0;
}

fn sun_shadow(world: vec3f, n: vec3f, view_dist: f32, pix: vec2f) -> f32 {
    let count = u32(frame.shadow.x);
    if (count == 0u || view_dist > frame.shadow.w) {
        return 1.0;
    }
    let l = primary_light_dir();
    var c = 0u;
    for (var i = 0u; i < count - 1u; i++) {
        if (view_dist > frame.cascade_splits[i]) {
            c = i + 1u;
        }
    }
    let noise = ign(pix, frame.proj_params.w);
    // Dithered transition into the next cascade.
    let split = frame.cascade_splits[c];
    let prev_split = select(0.0, frame.cascade_splits[max(c, 1u) - 1u], c > 0u);
    let blend_zone = (split - prev_split) * 0.12;
    if (c + 1u < count && view_dist > split - blend_zone) {
        if (noise < (view_dist - (split - blend_zone)) / blend_zone) {
            c += 1u;
        }
    }
    let texel = frame.cascade_texel[c];
    let nol = dot(n, l);
    // Normal offset scaled by texel size and slope.
    let offset = n * texel * (1.2 + 1.8 * (1.0 - saturate(nol))) + l * texel * 0.5;
    let angle = noise * TAU;
    let rot = vec2f(cos(angle), sin(angle));
    var s = pcf_cascade(c, world + offset, 1.6, rot);
    // Fade out at the shadow distance.
    let fade = saturate((frame.shadow.w - view_dist) / (frame.shadow.w * 0.1));
    return mix(1.0, s, fade);
}

fn spot_shadow(layer: i32, world: vec3f, n: vec3f, l: vec3f, dist: f32, pix: vec2f) -> f32 {
    let p = spot_mats[layer] * vec4f(world + n * (0.01 + dist * 0.004), 1.0);
    if (p.w <= 0.0) {
        return 1.0;
    }
    let ndc = p.xyz / p.w;
    let uv = ndc_to_uv(ndc.xy);
    if (any(uv < vec2f(0.0)) || any(uv > vec2f(1.0))) {
        return 1.0;
    }
    let dims = vec2f(textureDimensions(t_spot_shadow).xy);
    let angle = ign(pix, frame.proj_params.w) * TAU;
    let rot = vec2f(cos(angle), sin(angle));
    var sum = 0.0;
    for (var i = 0u; i < 8u; i++) {
        let o = POISSON16[i * 2u + 1u];
        let off = vec2f(o.x * rot.x - o.y * rot.y, o.x * rot.y + o.y * rot.x) * 1.5 / dims;
        sum += textureSampleCompareLevel(t_spot_shadow, samp_shadow, uv + off, layer, ndc.z - 0.00002);
    }
    return sum / 8.0;
}

// --- Clustered local lights -----------------------------------------------------

fn cluster_of(frag_xy: vec2f, view_dist: f32) -> u32 {
    let dims = frame.cluster_dims;
    let tile = vec2u(clamp(frag_xy * frame.viewport.zw * vec2f(dims.xy), vec2f(0.0), vec2f(dims.xy) - 1.0));
    let zs = log(max(view_dist, frame.cluster_z.x) / frame.cluster_z.x) / frame.cluster_z.z;
    let slice = u32(clamp(zs * f32(dims.z), 0.0, f32(dims.z) - 1.0));
    return (slice * dims.y + tile.y) * dims.x + tile.x;
}

fn light_attenuation(d2: f32, range: f32, radius: f32) -> f32 {
    let f = saturate(1.0 - sq(d2 / (range * range)));
    return f * f / max(d2, radius * radius + 1e-4);
}

fn spot_cone(light: GpuLight, l: vec3f) -> f32 {
    if (light.dir_cos.w < -1.5) {
        return 1.0;
    }
    let cd = dot(-l, light.dir_cos.xyz);
    let t = saturate((cd - light.dir_cos.w) / max(light.params.x - light.dir_cos.w, 1e-4));
    return t * t;
}

fn eval_local_lights(s: Surface, frag_xy: vec2f, view_dist: f32, geo_n: vec3f) -> vec3f {
    let stride = frame.cluster_dims.w + 1u;
    let base = cluster_of(frag_xy, view_dist) * stride;
    let count = min(clusters[base], frame.cluster_dims.w);
    var sum = vec3f(0.0);
    for (var i = 0u; i < count; i++) {
        let light = lights[clusters[base + 1u + i]];
        let dv = light.pos_range.xyz - s.pos;
        let d2 = dot(dv, dv);
        let range = light.pos_range.w;
        if (d2 > range * range) {
            continue;
        }
        let dist = sqrt(d2);
        let l = dv / max(dist, 1e-4);
        var att = light_attenuation(d2, range, light.color.w) * spot_cone(light, l);
        if (att <= 0.0) {
            continue;
        }
        let shadow_layer = i32(light.params.y);
        if (shadow_layer >= 0) {
            att *= spot_shadow(shadow_layer, s.pos, geo_n, l, dist, frag_xy);
        }
        let src_a = saturate(light.color.w / max(dist, 1e-3));
        sum += shade_light(s, l, light.color.rgb * att, src_a);
    }
    return sum;
}

// --- Image based lighting ---------------------------------------------------------

fn rotate_y(v: vec3f, a: f32) -> vec3f {
    let c = cos(a);
    let s = sin(a);
    return vec3f(c * v.x + s * v.z, v.y, -s * v.x + c * v.z);
}

fn sh_eval(base: u32, n: vec3f) -> vec3f {
    // Coefficients are pre-convolved with the clamped cosine lobe and divided by PI.
    var r = sh[base + 0u].rgb * 0.282095;
    r += sh[base + 1u].rgb * 0.488603 * n.y;
    r += sh[base + 2u].rgb * 0.488603 * n.z;
    r += sh[base + 3u].rgb * 0.488603 * n.x;
    r += sh[base + 4u].rgb * 1.092548 * n.x * n.y;
    r += sh[base + 5u].rgb * 1.092548 * n.y * n.z;
    r += sh[base + 6u].rgb * 0.315392 * (3.0 * n.z * n.z - 1.0);
    r += sh[base + 7u].rgb * 1.092548 * n.x * n.z;
    r += sh[base + 8u].rgb * 0.546274 * (n.x * n.x - n.y * n.y);
    return max(r, vec3f(0.0));
}

// Diffuse irradiance / PI in absolute nits (not pre-exposed).
fn ibl_diffuse(n: vec3f) -> vec3f {
    let blend = frame.sky2.y;
    var c = vec3f(0.0);
    if (blend < 0.999) {
        c = sh_eval(0u, n) * (1.0 - blend);
    }
    if (blend > 0.001) {
        c += sh_eval(9u, rotate_y(n, frame.sky2.z)) * frame.sky2.w * blend;
    }
    return c;
}

fn ibl_specular(r: vec3f, roughness: f32) -> vec3f {
    let blend = frame.sky2.y;
    var c = vec3f(0.0);
    if (blend < 0.999) {
        let lod = roughness * f32(textureNumLevels(t_sky_cube) - 1u);
        c = textureSampleLevel(t_sky_cube, samp_linear_clamp, r, lod).rgb * (1.0 - blend);
    }
    if (blend > 0.001) {
        let lod = roughness * f32(textureNumLevels(t_hdri_cube) - 1u);
        c += textureSampleLevel(t_hdri_cube, samp_linear_clamp, rotate_y(r, frame.sky2.z), lod).rgb * frame.sky2.w * blend;
    }
    return c;
}

// Off-specular dominant direction (Frostbite) for rough surfaces.
fn specular_dominant_dir(n: vec3f, r: vec3f, roughness: f32) -> vec3f {
    let s = saturate(1.0 - roughness);
    let lerp_f = s * (sqrt(s) + roughness);
    return normalize(mix(n, r, lerp_f));
}

fn specular_occlusion(nov: f32, ao: f32, roughness: f32) -> f32 {
    return saturate(pow(nov + ao, exp2(-16.0 * roughness - 1.0)) - 1.0 + ao);
}

// Multi-bounce AO approximation (Jimenez 2016).
fn gtao_multibounce(ao: f32, albedo: vec3f) -> vec3f {
    let a = 2.0404 * albedo - 0.3324;
    let b = -4.7951 * albedo + 0.6417;
    let c = 2.7552 * albedo + 0.6903;
    return max(vec3f(ao), ((ao * a + b) * ao + c) * ao);
}

// --- Height-field sky occlusion --------------------------------------------------
// A top-down depth map of the surroundings gives the horizon elevation in a few
// directions; occluded sky is replaced by light bounced off the occluders.

fn skyocc_height(xz: vec2f) -> f32 {
    let c = frame.skyocc_vp * vec4f(xz.x, 0.0, xz.y, 1.0);
    let uv = ndc_to_uv(c.xy);
    if (any(uv <= vec2f(0.0)) || any(uv >= vec2f(1.0))) {
        return -1e4;
    }
    let dims = vec2f(textureDimensions(t_skyocc));
    let d = textureLoad(t_skyocc, vec2i(uv * dims), 0);
    return frame.skyocc.x - d * frame.skyocc.y;
}

fn sky_visibility(p: vec3f, n: vec3f, pix: vec2f) -> f32 {
    if (frame.skyocc.z < 0.5) {
        return 1.0;
    }
    let rot = ign(pix, frame.proj_params.w) * (TAU / 8.0);
    let base_y = p.y + 0.05;
    let nh_len = length(n.xz);
    let nh = n.xz / max(nh_len, 1e-4);
    let wall = saturate(nh_len * 1.2);
    var vis = 0.0;
    var wsum = 0.0;
    for (var i = 0u; i < 8u; i++) {
        let a = rot + f32(i) * (TAU / 8.0);
        let dir = vec2f(cos(a), sin(a));
        var t = 0.0;
        var dist = 0.7;
        for (var k = 0; k < 7; k++) {
            let h = skyocc_height(p.xz + dir * dist);
            t = max(t, (h - base_y) / dist);
            dist *= 2.0;
        }
        let w = mix(1.0, saturate(dot(dir, nh) + 0.25), wall);
        vis += w / (1.0 + t * t);
        wsum += w;
    }
    return mix(1.0, vis / max(wsum, 1e-4), frame.skyocc.w);
}

// Is the reflection ray blocked by the height field? 1 = sky visible.
fn reflection_visibility(p: vec3f, r: vec3f) -> f32 {
    if (frame.skyocc.z < 0.5 || r.y < -0.2) {
        return 1.0;
    }
    var dist = 1.0;
    var vis = 1.0;
    for (var k = 0; k < 8; k++) {
        let q = p + r * dist;
        let h = skyocc_height(q.xz);
        vis = min(vis, saturate((q.y - h) / (0.3 + dist * 0.08)));
        dist *= 2.0;
    }
    return vis;
}

// Radiance of the occluding city (facades lit by sun + sky).
fn bounce_radiance() -> vec3f {
    let h_km = camera_altitude_km();
    let cover_att = 1.0 - 0.95 * smoothstep(0.4, 1.0, frame.sky.w);
    let e_sun = sun_illuminance_at(h_km) * saturate(frame.sun.y) * cover_att + moon_illuminance_at(h_km) * saturate(frame.moon.y);
    let sky_up = ibl_diffuse(vec3f(0.0, 1.0, 0.0));
    return frame.bounce.rgb * (0.45 * e_sun * INV_PI + 0.6 * sky_up) + frame.night_glow.rgb * 0.3;
}

struct Ambient {
    diffuse: vec3f,
    specular: vec3f,
};

// Ambient (IBL + SSR) lighting with multiple-scattering energy compensation.
fn eval_ambient(s: Surface, ssr: vec4f) -> Ambient {
    var o: Ambient;
    let nov = max(dot(s.base_n, s.v), 1e-4);
    let lut = textureSampleLevel(t_brdf_lut, samp_linear_clamp, vec2f(nov, s.roughness), 0.0).rg;
    let fss_ess = s.f0 * lut.x + lut.y;
    let ems = 1.0 - (lut.x + lut.y);
    let f_avg = s.f0 + (1.0 - s.f0) / 21.0;
    let fms_ems = ems * fss_ess * f_avg / (1.0 - f_avg * ems);
    let kd = s.albedo * (1.0 - fss_ess - fms_ems);

    let svis = sky_visibility(s.pos, s.base_n, s.pix);
    let bounce = bounce_radiance();
    let irr = ibl_diffuse(s.base_n) * svis + bounce * (1.0 - svis);
    let r = reflect(-s.v, s.base_n);
    let dom = specular_dominant_dir(s.base_n, r, s.roughness);
    let rvis = mix(reflection_visibility(s.pos, dom), svis, saturate(s.roughness * 1.5));
    var rad = mix(bounce, ibl_specular(dom, s.roughness), rvis);
    rad = mix(rad, ssr.rgb, ssr.a);

    let ao_col = gtao_multibounce(s.ao, s.albedo);
    let spec_ao = specular_occlusion(nov, s.ao, s.roughness);
    o.diffuse = (fms_ems + kd) * irr * ao_col;
    o.specular = fss_ess * rad * spec_ao;

    if (s.clearcoat > 0.0) {
        let cnov = max(dot(s.n, s.v), 1e-4);
        let fc = f_schlick(0.04, 1.0, cnov) * s.clearcoat;
        let att = 1.0 - fc;
        let cr = reflect(-s.v, s.n);
        var crad = mix(bounce, ibl_specular(cr, s.cc_roughness), reflection_visibility(s.pos, cr));
        // SSR is traced with the coat normal for clear coated pixels.
        crad = mix(crad, ssr.rgb, ssr.a);
        o.diffuse *= att;
        o.specular = o.specular * att * att + vec3f(fc) * crad * spec_ao;
    }
    if (s.translucency > 0.0) {
        o.diffuse += s.albedo * s.translucency * (ibl_diffuse(-s.base_n) * svis + bounce * (1.0 - svis)) * 0.5 * s.ao;
    }
    return o;
}

// --- Fog / aerial perspective -------------------------------------------------------

struct FogResult {
    inscatter: vec3f,   // absolute nits
    transmittance: f32,
};

fn fog_optical_depth(ro: vec3f, dir: vec3f, dist: f32) -> f32 {
    let d0 = frame.fog.x;
    let b = max(frame.fog.y, 1e-5);
    let start = frame.fog2.x;
    let l = max(dist - start, 0.0);
    if (l <= 0.0 || d0 <= 0.0) {
        return 0.0;
    }
    let y0 = ro.y + dir.y * min(start, dist) - frame.fog.z;
    let dy = dir.y * l;
    var k = 1.0;
    if (abs(b * dy) > 1e-4) {
        k = (1.0 - exp(-b * dy)) / (b * dy);
    }
    return d0 * exp(-b * max(y0, -50.0)) * l * k;
}

// Single-tap cascade lookup for participating media.
fn shadow_1tap(p: vec3f, view_z: f32) -> f32 {
    let count = u32(frame.shadow.x);
    if (count == 0u || view_z > frame.shadow.w) {
        return 1.0;
    }
    var c = 0u;
    for (var i = 0u; i < count - 1u; i++) {
        if (view_z > frame.cascade_splits[i]) {
            c = i + 1u;
        }
    }
    let lp = cascade_lookup(c, p);
    if (any(lp.xy < vec2f(0.0)) || any(lp.xy > vec2f(1.0)) || lp.z > 1.0) {
        return 1.0;
    }
    return textureSampleCompareLevel(t_shadow, samp_shadow, lp.xy, i32(c), lp.z);
}

// Fraction of the view ray (weighted by in-scattering) that sees the shadow light: light shafts.
fn fog_light_visibility(ro: vec3f, dir: vec3f, dist: f32, pix: vec2f) -> f32 {
    if (u32(frame.shadow.x) == 0u) {
        return 1.0;
    }
    let maxd = min(dist, frame.shadow.w);
    let fwd = -frame.inv_view[2].xyz;
    let cz = dot(dir, fwd);
    let j = ign(pix + vec2f(13.0, 7.0), frame.proj_params.w);
    let n = 12.0;
    var sum = 0.0;
    var wsum = 0.0;
    var tail = 0.0;
    var tail_n = 0.0;
    let d0 = frame.fog.x;
    for (var i = 0.0; i < n; i += 1.0) {
        // Denser samples near the camera where most in-scattering happens.
        let u = (i + j) / n;
        let t = maxd * u * u;
        let dt = maxd * (2.0 * u) / n;
        let w = exp(-d0 * t) * dt;
        let v = shadow_1tap(ro + dir * t, t * cz);
        sum += v * w;
        wsum += w;
        if (u > 0.6) {
            tail += v;
            tail_n += 1.0;
        }
    }
    // Beyond the shadowed range assume the trend of the farthest samples continues.
    let far_vis = mix(1.0, tail / max(tail_n, 1.0), 0.75);
    let far = max(dist - maxd, 0.0) * exp(-d0 * maxd);
    return (sum + far * far_vis) / max(wsum + far, 1e-4);
}

fn eval_fog(ro: vec3f, dir: vec3f, dist: f32) -> FogResult {
    return eval_fog_shadowed(ro, dir, dist, 1.0);
}

fn eval_fog_shadowed(ro: vec3f, dir: vec3f, dist: f32, light_vis: f32) -> FogResult {
    var o: FogResult;
    let tau = fog_optical_depth(ro, dir, dist);
    o.transmittance = exp(-tau);
    if (tau <= 0.0) {
        o.inscatter = vec3f(0.0);
        return o;
    }
    let h_km = camera_altitude_km();
    let hdir = normalize(vec3f(dir.x, max(dir.y, 0.03), dir.z));
    let horizon = textureSampleLevel(t_skyview, samp_linear_repeat, skyview_uv(hdir), 0.0).rgb * frame.sky.x;
    let amb = mix(horizon, ibl_diffuse(vec3f(0.0, 1.0, 0.0)) * 1.1, 0.5 + 0.5 * frame.sky.w);
    let g = frame.fog.w;
    let cover_att = 1.0 - 0.95 * smoothstep(0.4, 1.0, frame.sky.w);
    let moon_primary = frame.light_params.x > 0.5;
    let sun = sun_illuminance_at(h_km) * phase_hg(dot(dir, frame.sun.xyz), g) * cover_att * select(light_vis, 1.0, moon_primary);
    let moon = moon_illuminance_at(h_km) * phase_hg(dot(dir, frame.moon.xyz), g) * cover_att * select(1.0, light_vis, moon_primary);
    o.inscatter = (amb + sun + moon + frame.night_glow.rgb * 0.5) * (1.0 - o.transmittance);
    return o;
}

// Light scattered towards the camera by participating media around local lights.
fn eval_volumetric_lights(ro: vec3f, dir: vec3f, max_t: f32, pix: vec2f) -> vec3f {
    let count = u32(frame.fog2.z);
    if (count == 0u) {
        return vec3f(0.0);
    }
    // Scattering coefficient near the ground: fog plus rain droplets.
    let sigma = (frame.fog.x * exp(-frame.fog.y * max(ro.y - frame.fog.z, 0.0)) + frame.weather.z * 0.004) * frame.fog2.y;
    if (sigma <= 0.0) {
        return vec3f(0.0);
    }
    let jitter = ign(pix, frame.proj_params.w);
    var sum = vec3f(0.0);
    for (var i = 0u; i < count; i++) {
        let idx = frame.vol_lights[i / 4u][i % 4u];
        let light = lights[idx];
        let range = light.pos_range.w;
        let p = light.pos_range.xyz;
        let t0 = dot(p - ro, dir);
        let closest = ro + dir * t0;
        let h2 = dot(closest - p, closest - p);
        if (h2 > range * range) {
            continue;
        }
        let half_chord = sqrt(range * range - h2);
        let ta = max(t0 - half_chord, 0.0);
        let tb = min(t0 + half_chord, max_t);
        if (tb <= ta) {
            continue;
        }
        let vscale = light.params.z * sigma * (1.0 / (4.0 * PI));
        if (light.dir_cos.w < -1.5) {
            // Point light: analytic integral of 1/d^2 along the ray.
            let h = sqrt(max(h2, 0.0025 + light.color.w * light.color.w));
            let integral = (atan((tb - t0) / h) - atan((ta - t0) / h)) / h;
            sum += light.color.rgb * vscale * integral;
        } else {
            // Spot light: short jittered ray march through the cone.
            let steps = 10u;
            let dt = (tb - ta) / f32(steps);
            var acc = 0.0;
            for (var k = 0u; k < steps; k++) {
                let t = ta + (f32(k) + jitter) * dt;
                let x = ro + dir * t;
                let dv = p - x;
                let d2 = max(dot(dv, dv), 0.01);
                let l = dv * inverseSqrt(d2);
                var att = spot_cone(light, l) * light_attenuation(d2, range, max(light.color.w, 0.1));
                acc += att;
            }
            sum += light.color.rgb * vscale * acc * dt;
        }
    }
    return sum;
}
