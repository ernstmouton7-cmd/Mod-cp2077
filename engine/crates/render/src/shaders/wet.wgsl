// ---------------------------------------------------------------------------
// wet.wgsl — rain wetness, puddles and ripples (no bindings; used by G-buffer and decals).

// --- Rain ---------------------------------------------------------------------

fn ripple_layer(p: vec2f, t: f32, cell: f32, seed: f32) -> vec2f {
    let q = p / cell + vec2f(seed * 17.0, seed * 31.0);
    let id = floor(q);
    let f = fract(q);
    let h = hash22(id + vec2f(seed * 3.0, 0.0));
    let center = 0.3 + 0.4 * h;
    let phase = fract(t * (0.8 + 0.5 * h.x) + h.y);
    let r = phase * 0.32;
    let dv = f - center;
    let d = length(dv);
    let x = (d - r) / 0.045;
    // Damped ring wave: derivative gives the normal perturbation.
    let env = exp(-x * x) * (1.0 - phase) * (1.0 - phase);
    let dw = -2.0 * x * env * cos(x * 3.0) - 3.0 * env * sin(x * 3.0);
    return dv / max(d, 1e-4) * dw;
}

fn rain_ripples(p: vec2f, t: f32, intensity: f32) -> vec2f {
    var g = ripple_layer(p, t, 0.42, 0.0);
    g += ripple_layer(p, t * 1.13 + 0.37, 0.33, 1.0);
    g += ripple_layer(p, t * 0.91 + 0.71, 0.51, 2.0) * step(0.4, intensity);
    return g * intensity * 0.12;
}

struct WetResult {
    albedo: vec3f,
    roughness: f32,
    metallic: f32,
    n: vec3f,
    puddle: f32,
};

fn apply_wetness(world: vec3f, geo_n: vec3f, albedo: vec3f, roughness: f32, metallic: f32, n: vec3f, wet_factor: f32) -> WetResult {
    var o: WetResult;
    o.albedo = albedo;
    o.roughness = roughness;
    o.metallic = metallic;
    o.n = n;
    o.puddle = 0.0;
    let w = frame.weather.x * wet_factor;
    if (w <= 0.001) {
        return o;
    }
    let up = smoothstep(0.65, 0.95, geo_n.y);
    let porosity = saturate(roughness * 1.3 - 0.3) * (1.0 - metallic);
    let wet = w * mix(0.6, 1.0, up);
    // Darkening from water filling pores (Lagarde).
    o.albedo = albedo * mix(1.0, mix(0.85, 0.42, porosity), wet);
    let streak = value_noise2(world.xz * vec2f(1.7, 0.6) + vec2f(world.y)) * 0.12;
    o.roughness = mix(roughness, mix(0.08, 0.26, porosity) + streak, wet * 0.92);

    // Standing water in low areas: world space noise mask so it works anywhere.
    let p = frame.weather.y * wet_factor;
    if (p > 0.001 && up > 0.0) {
        let nz = fbm2(world.xz * 0.11 + vec2f(13.0, 7.0), 4) * 0.75 + value_noise2(world.xz * 0.8) * 0.25;
        let thr = 1.0 - p * 0.75;
        let puddle = smoothstep(thr - 0.02, thr + 0.05, nz) * up * saturate(w * 1.5);
        o.puddle = puddle;
        o.albedo = o.albedo * mix(1.0, 0.75, puddle);
        o.roughness = mix(o.roughness, 0.025, puddle);
        o.metallic = mix(o.metallic, 0.0, puddle);
        var flat_n = geo_n;
        let rain = frame.weather.z;
        if (rain > 0.001) {
            let g = rain_ripples(world.xz, frame.weather.w, rain);
            flat_n = normalize(geo_n + vec3f(g.x, 0.0, g.y));
        }
        // Thin water film on non-puddle wet areas still smooths small bumps.
        o.n = normalize(mix(mix(n, geo_n, wet * 0.35 * up), flat_n, puddle));
    }
    return o;
}

