// ---------------------------------------------------------------------------
// atmosphere.wgsl — Hillaire 2020 style physically based sky.
// Distances in km. LUTs: transmittance (256x64), multi-scattering (32x32),
// sky-view (256x128, absolute azimuth, sun + moon).

const R_BOTTOM: f32 = 6360.0;
const R_TOP: f32 = 6460.0;
const RAYLEIGH_SCATTER: vec3f = vec3f(5.802e-3, 13.558e-3, 33.1e-3);
const MIE_SCATTER: f32 = 3.996e-3;
const MIE_EXTINCT: f32 = 4.44e-3;
const OZONE_ABSORB: vec3f = vec3f(0.650e-3, 1.881e-3, 0.085e-3);
const MIE_G: f32 = 0.8;

struct Medium {
    scattering: vec3f,
    extinction: vec3f,
    rayleigh: vec3f,
    mie: vec3f,
};

fn medium_at(h: f32, mie_scale: f32) -> Medium {
    var m: Medium;
    let rd = exp(-h / 8.0);
    let md = exp(-h / 1.2) * mie_scale;
    let od = max(0.0, 1.0 - abs(h - 25.0) / 15.0);
    m.rayleigh = RAYLEIGH_SCATTER * rd;
    m.mie = vec3f(MIE_SCATTER * md);
    m.scattering = m.rayleigh + m.mie;
    m.extinction = m.rayleigh + vec3f(MIE_EXTINCT * md) + OZONE_ABSORB * od;
    return m;
}

fn ray_sphere(ro: vec3f, rd: vec3f, r: f32) -> f32 {
    // Nearest positive hit of a sphere centred at the origin; -1 if none.
    let b = dot(ro, rd);
    let c = dot(ro, ro) - r * r;
    let disc = b * b - c;
    if (disc < 0.0) {
        return -1.0;
    }
    let s = sqrt(disc);
    let t0 = -b - s;
    let t1 = -b + s;
    if (t0 > 0.0) {
        return t0;
    }
    if (t1 > 0.0) {
        return t1;
    }
    return -1.0;
}

fn phase_rayleigh(c: f32) -> f32 {
    return 3.0 / (16.0 * PI) * (1.0 + c * c);
}

fn phase_hg(c: f32, g: f32) -> f32 {
    let g2 = g * g;
    return (1.0 - g2) / (4.0 * PI * pow(max(1.0 + g2 - 2.0 * g * c, 1e-4), 1.5));
}

fn phase_cornette(c: f32, g: f32) -> f32 {
    let g2 = g * g;
    let k = 3.0 / (8.0 * PI) * (1.0 - g2) / (2.0 + g2);
    return k * (1.0 + c * c) / pow(max(1.0 + g2 - 2.0 * g * c, 1e-4), 1.5);
}

fn transmittance_uv(view_height: f32, mu: f32) -> vec2f {
    let h = sqrt(R_TOP * R_TOP - R_BOTTOM * R_BOTTOM);
    let rho = sqrt(max(0.0, view_height * view_height - R_BOTTOM * R_BOTTOM));
    let disc = view_height * view_height * (mu * mu - 1.0) + R_TOP * R_TOP;
    let d = max(0.0, -view_height * mu + sqrt(max(disc, 0.0)));
    let d_min = R_TOP - view_height;
    let d_max = rho + h;
    return vec2f((d - d_min) / (d_max - d_min), rho / h);
}

fn transmittance_params(uv: vec2f) -> vec2f {
    let h = sqrt(R_TOP * R_TOP - R_BOTTOM * R_BOTTOM);
    let rho = h * uv.y;
    let vh = sqrt(rho * rho + R_BOTTOM * R_BOTTOM);
    let d_min = R_TOP - vh;
    let d_max = rho + h;
    let d = d_min + uv.x * (d_max - d_min);
    var mu = 1.0;
    if (d > 0.0) {
        mu = (h * h - rho * rho - d * d) / (2.0 * vh * d);
    }
    return vec2f(vh, clamp(mu, -1.0, 1.0));
}

fn sample_transmittance(lut: texture_2d<f32>, s: sampler, view_height: f32, mu: f32) -> vec3f {
    let uv = transmittance_uv(view_height, mu);
    return textureSampleLevel(lut, s, uv, 0.0).rgb;
}

// Transmittance from altitude `h_km` towards direction with cosine `mu`,
// including a soft planet shadow so sunsets fade instead of popping.
fn light_transmittance(lut: texture_2d<f32>, s: sampler, h_km: f32, mu: f32) -> vec3f {
    let r = R_BOTTOM + max(h_km, 0.0005);
    let t = sample_transmittance(lut, s, r, mu);
    let horizon = -sqrt(max(0.0, 1.0 - (R_BOTTOM * R_BOTTOM) / (r * r)));
    return t * smoothstep(horizon - 0.012, horizon + 0.004, mu);
}

fn multiscatter_uv(view_height: f32, mu: f32) -> vec2f {
    return vec2f(mu * 0.5 + 0.5, saturate((view_height - R_BOTTOM) / (R_TOP - R_BOTTOM)));
}

// Sky view LUT parametrisation: absolute azimuth on u, warped elevation on v.
fn skyview_uv(dir: vec3f) -> vec2f {
    let az = atan2(dir.z, dir.x);
    let el = asin(clamp(dir.y, -1.0, 1.0));
    let s = sign(el) * sqrt(abs(el) / (0.5 * PI));
    return vec2f(az / TAU + 0.5, 0.5 - 0.5 * s);
}

fn skyview_dir(uv: vec2f) -> vec3f {
    let az = (uv.x - 0.5) * TAU;
    let s = (0.5 - uv.y) * 2.0;
    let el = sign(s) * s * s * 0.5 * PI;
    return vec3f(cos(el) * cos(az), sin(el), cos(el) * sin(az));
}

// --- Clouds -----------------------------------------------------------------

const CLOUD_HEIGHT: f32 = 1800.0;

fn cloud_density_at(xz: vec2f, cover: f32, time: f32, wind: vec2f) -> f32 {
    let p = (xz + wind * time * 8.0) / 2600.0;
    var n = fbm2(p, 5);
    n = n * 0.8 + 0.2 * value_noise2(p * 7.0 + vec2f(time * 0.002));
    let threshold = 1.0 - cover;
    let d = saturate((n - threshold * 0.85 - 0.15 * (1.0 - cover)) / max(0.22 + 0.1 * cover, 1e-3));
    return select(d, 1.0, cover >= 0.999);
}

// Fraction of direct light blocked by clouds at a world position.
fn cloud_shadow(world: vec3f, l: vec3f) -> f32 {
    let cover = frame.sky.w;
    if (cover <= 0.01) {
        return 1.0;
    }
    let t = (CLOUD_HEIGHT - world.y) / max(l.y, 0.05);
    let xz = world.xz + l.xz * t;
    let d = cloud_density_at(xz, cover, frame.camera_pos.w, frame.sky3.yz);
    // Thick overcast blocks almost everything.
    let thick = mix(0.75, 0.985, smoothstep(0.6, 1.0, cover));
    return 1.0 - d * thick;
}

// Overcast (CIE) sky luminance per unit of horizontal direct illuminance.
fn overcast_luminance(dir_y: f32, e_horizontal: vec3f) -> vec3f {
    return e_horizontal * 0.2 * INV_PI * (1.0 + 2.0 * max(dir_y, 0.0)) / 3.0 * 1.6;
}

// --- Full sky radiance (absolute nits, not pre-exposed) ----------------------

fn star_field(dir: vec3f) -> f32 {
    let p = dir * 420.0;
    let cell = floor(p);
    let h = hash31(cell);
    if (h < 0.965) {
        return 0.0;
    }
    let jitter = hash33(cell + vec3f(7.0)) * 0.7 + 0.15;
    let d = length(fract(p) - jitter);
    let mag = pow((h - 0.965) / 0.035, 6.0);
    let tw = 0.75 + 0.25 * sin(frame.camera_pos.w * (2.0 + 6.0 * h) + h * 100.0);
    return mag * tw * exp(-d * d * 60.0) * 0.04;
}

fn sky_radiance(tl: texture_2d<f32>, sv: texture_2d<f32>, sc: sampler, sr: sampler, dir_in: vec3f, disks: bool) -> vec3f {
    let h_km = frame.sky3.w;
    let sun = frame.sun.xyz;
    let moon = frame.moon.xyz;
    let cover = frame.sky.w;
    let dir = normalize(dir_in);
    let r = R_BOTTOM + max(h_km, 0.0005);

    var col = textureSampleLevel(sv, sr, skyview_uv(vec3f(dir.x, max(dir.y, -0.02), dir.z)), 0.0).rgb;
    let t_view = sample_transmittance(tl, sc, r, max(dir.y, 0.0));
    let e_sun = frame.sun.w * light_transmittance(tl, sc, h_km, sun.y);
    let e_moon = frame.moon.w * light_transmittance(tl, sc, h_km, moon.y);

    if (dir.y > -0.01) {
        // Celestial bodies and stars (behind clouds).
        var space = vec3f(0.0);
        if (disks) {
            let sun_r = frame.light_params.y;
            let cs = dot(dir, sun);
            if (cs > cos(sun_r)) {
                let mu = sqrt(saturate(1.0 - sq(acos(min(cs, 1.0)) / sun_r)));
                let limb = 1.0 - 0.6 * (1.0 - pow(mu, 0.8));
                space += vec3f(frame.sun.w / (PI * sun_r * sun_r)) * limb;
            }
            let moon_r = frame.light_params.z;
            let cm = dot(dir, moon);
            if (cm > cos(moon_r)) {
                let b = onb(moon);
                let lp = vec2f(dot(dir, b[0]), dot(dir, b[1])) / moon_r;
                let maria = 0.75 + 0.25 * fbm2(lp * 3.0 + vec2f(3.1, 1.7), 3);
                space += vec3f(frame.moon.w / (PI * moon_r * moon_r)) * maria * vec3f(1.0, 0.97, 0.92);
            }
        }
        space += vec3f(star_field(dir) * frame.sky2.x) * vec3f(0.9, 0.95, 1.0);
        col += space * t_view;
        // Light pollution: warm glow near the horizon.
        col += frame.night_glow.rgb * (0.15 + 0.85 * pow(1.0 - saturate(dir.y), 5.0));
    }

    // Cloud layer.
    if (dir.y > 0.0 && cover > 0.01) {
        let t = (CLOUD_HEIGHT - frame.camera_pos.y) / max(dir.y, 0.015);
        let xz = frame.camera_pos.xz + dir.xz * t;
        let dens = cloud_density_at(xz, cover, frame.camera_pos.w, frame.sky3.yz);
        let fade = exp(-t / 45000.0);
        let e_h = e_sun * saturate(sun.y + 0.02) + e_moon * saturate(moon.y + 0.02);
        let base = overcast_luminance(dir.y, e_h);
        let silver = e_sun * phase_hg(dot(dir, sun), 0.65) * 0.25 * (1.0 - dens) * dens;
        let glow = frame.night_glow.rgb * (2.5 + 2.0 * cover);
        let cloud = base * (1.25 - 0.55 * dens) + silver + glow;
        let alpha = dens * mix(fade, 1.0, pow(cover, 4.0));
        col = mix(col, cloud, alpha);
    }

    if (dir.y < 0.0) {
        // Ground below the horizon (only visible in reflections / IBL).
        let up = textureSampleLevel(sv, sr, skyview_uv(vec3f(0.0, 1.0, 0.0)), 0.0).rgb;
        let e_dir = (e_sun * saturate(sun.y) + e_moon * saturate(moon.y)) * (1.0 - 0.97 * smoothstep(0.5, 1.0, cover));
        let e_sky = mix(PI * up * 1.3, PI * overcast_luminance(0.5, e_sun * saturate(sun.y + 0.02) + e_moon * saturate(moon.y + 0.02)), cover);
        let ground = frame.sky.z * INV_PI * (e_dir + e_sky) + frame.night_glow.rgb * 0.6;
        col = mix(col, ground, 1.0 - smoothstep(-0.06, 0.0, dir.y));
    }
    return col * frame.sky.x;
}
