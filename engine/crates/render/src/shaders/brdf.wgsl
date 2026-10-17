// ---------------------------------------------------------------------------
// brdf.wgsl — GGX / Smith / Schlick microfacet model, clear coat and helpers.

const MIN_ROUGHNESS: f32 = 0.045;

fn f_schlick3(f0: vec3f, f90: f32, voh: f32) -> vec3f {
    let f = pow(1.0 - voh, 5.0);
    return f0 + (vec3f(f90) - f0) * f;
}

fn f_schlick(f0: f32, f90: f32, voh: f32) -> f32 {
    return f0 + (f90 - f0) * pow(1.0 - voh, 5.0);
}

fn f_schlick_roughness(f0: vec3f, nov: f32, roughness: f32) -> vec3f {
    return f0 + (max(vec3f(1.0 - roughness), f0) - f0) * pow(1.0 - nov, 5.0);
}

fn d_ggx(noh: f32, a: f32) -> f32 {
    let a2 = a * a;
    let f = (noh * a2 - noh) * noh + 1.0;
    return a2 / (PI * f * f + 1e-7);
}

// Height-correlated Smith visibility (includes 1/(4 NoL NoV)).
fn v_smith_ggx_correlated(nov: f32, nol: f32, a: f32) -> f32 {
    let a2 = a * a;
    let gv = nol * sqrt(nov * nov * (1.0 - a2) + a2);
    let gl = nov * sqrt(nol * nol * (1.0 - a2) + a2);
    return 0.5 / (gv + gl + 1e-6);
}

// Kelemen visibility for the clear coat layer.
fn v_kelemen(loh: f32) -> f32 {
    return 0.25 / max(loh * loh, 1e-4);
}

// Burley-ish energy conserving diffuse is overkill here; Lambert with a
// retro-reflection tweak keeps rough materials (asphalt) from looking flat.
fn fd_lambert() -> f32 {
    return INV_PI;
}

fn fd_burley(nov: f32, nol: f32, loh: f32, roughness: f32) -> f32 {
    let f90 = 0.5 + 2.0 * roughness * loh * loh;
    let light = f_schlick(1.0, f90, nol);
    let view = f_schlick(1.0, f90, nov);
    return light * view * INV_PI;
}

// Clear coat changes the base layer's F0 because the interface is now coat->base.
fn f0_under_coat(f0: vec3f) -> vec3f {
    return saturate3(f0 * (f0 * (0.941892 - 0.263008 * f0) + 0.346479) - 0.0285998);
}

// Roughness widening for spherical lights (Karis 2013) to keep highlights from
// small sources bounded and avoid fireflies.
fn sphere_roughness(a: f32, radius: f32, dist: f32) -> f32 {
    return saturate(a + radius / max(2.0 * dist, 1e-4) * 0.5);
}

struct Surface {
    pos: vec3f,
    n: vec3f,          // shading normal (clear coat normal when clear coated)
    base_n: vec3f,     // base layer normal (flakes)
    v: vec3f,
    nov: f32,
    albedo: vec3f,     // diffuse colour (already multiplied by 1-metallic)
    f0: vec3f,
    roughness: f32,    // perceptual
    a: f32,            // alpha = roughness^2
    ao: f32,
    clearcoat: f32,
    cc_roughness: f32,
    cc_a: f32,
    translucency: f32,
    energy_comp: vec3f,
    model: u32,
    pix: vec2f,        // pixel position (noise seeds)
};

// Shading for one light with radiance `light` arriving from direction l.
fn shade_light(s: Surface, l: vec3f, light: vec3f, source_a: f32) -> vec3f {
    let h = normalize(s.v + l);
    let nol_raw = dot(s.base_n, l);
    let nol = saturate(nol_raw);
    var col = vec3f(0.0);
    let loh = saturate(dot(l, h));
    if (nol > 0.0) {
        let noh = saturate(dot(s.base_n, h));
        let nov = max(abs(dot(s.base_n, s.v)), 1e-4);
        let a = max(s.a, source_a);
        let d = d_ggx(noh, a);
        let vis = v_smith_ggx_correlated(nov, nol, a);
        let f = f_schlick3(s.f0, 1.0, loh);
        let spec = d * vis * f * s.energy_comp;
        let diff = s.albedo * fd_burley(nov, nol, loh, s.roughness);
        col = (diff + spec) * nol;
    }
    if (s.clearcoat > 0.0) {
        let cnol = saturate(dot(s.n, l));
        let cnoh = saturate(dot(s.n, h));
        let fc = f_schlick(0.04, 1.0, loh) * s.clearcoat;
        let dc = d_ggx(cnoh, max(s.cc_a, source_a));
        let vc = v_kelemen(loh);
        col = col * (1.0 - fc) + vec3f(dc * vc * fc * cnol);
    }
    if (s.translucency > 0.0) {
        // Wrapped back-lighting for leaves/thin cloth.
        let back = saturate(-nol_raw * 0.6 + 0.4);
        col += s.albedo * s.translucency * back * INV_PI * 0.6;
    }
    return col * light;
}

// Hammersley & GGX importance sampling (used by IBL prefiltering and SSR).
fn radical_inverse(b: u32) -> f32 {
    var bits = b;
    bits = (bits << 16u) | (bits >> 16u);
    bits = ((bits & 0x55555555u) << 1u) | ((bits & 0xAAAAAAAAu) >> 1u);
    bits = ((bits & 0x33333333u) << 2u) | ((bits & 0xCCCCCCCCu) >> 2u);
    bits = ((bits & 0x0F0F0F0Fu) << 4u) | ((bits & 0xF0F0F0F0u) >> 4u);
    bits = ((bits & 0x00FF00FFu) << 8u) | ((bits & 0xFF00FF00u) >> 8u);
    return f32(bits) * 2.3283064365386963e-10;
}

fn hammersley(i: u32, n: u32) -> vec2f {
    return vec2f(f32(i) / f32(n), radical_inverse(i));
}

fn importance_sample_ggx(xi: vec2f, a: f32) -> vec3f {
    let phi = TAU * xi.x;
    let cos_t = sqrt((1.0 - xi.y) / (1.0 + (a * a - 1.0) * xi.y));
    let sin_t = sqrt(1.0 - cos_t * cos_t);
    return vec3f(sin_t * cos(phi), sin_t * sin(phi), cos_t);
}

// Visible normal sampling (Heitz 2018), view vector in tangent space.
fn sample_ggx_vndf(ve: vec3f, a: f32, u: vec2f) -> vec3f {
    let vh = normalize(vec3f(a * ve.x, a * ve.y, ve.z));
    let lensq = vh.x * vh.x + vh.y * vh.y;
    var t1 = vec3f(1.0, 0.0, 0.0);
    if (lensq > 0.0) {
        t1 = vec3f(-vh.y, vh.x, 0.0) * inverseSqrt(lensq);
    }
    let t2 = cross(vh, t1);
    let r = sqrt(u.x);
    let phi = TAU * u.y;
    let p1 = r * cos(phi);
    var p2 = r * sin(phi);
    let s = 0.5 * (1.0 + vh.z);
    p2 = (1.0 - s) * sqrt(1.0 - p1 * p1) + s * p2;
    let nh = p1 * t1 + p2 * t2 + sqrt(max(0.0, 1.0 - p1 * p1 - p2 * p2)) * vh;
    return normalize(vec3f(a * nh.x, a * nh.y, max(0.0, nh.z)));
}
