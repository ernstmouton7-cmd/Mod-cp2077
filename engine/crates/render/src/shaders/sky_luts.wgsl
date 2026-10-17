// ---------------------------------------------------------------------------
// sky_luts.wgsl — compute shaders for the atmosphere LUTs and the sky cubemap.
// Bindings for group 1 are injected per entry point (BINDINGS placeholder).

${BINDINGS}

struct Scatter {
    l: vec3f,
    ms: vec3f,
};

fn cube_dir(face: u32, uv: vec2f) -> vec3f {
    let s = uv.x * 2.0 - 1.0;
    let t = uv.y * 2.0 - 1.0;
    switch (face) {
        case 0u: { return normalize(vec3f(1.0, -t, -s)); }
        case 1u: { return normalize(vec3f(-1.0, -t, s)); }
        case 2u: { return normalize(vec3f(s, 1.0, t)); }
        case 3u: { return normalize(vec3f(s, -1.0, -t)); }
        case 4u: { return normalize(vec3f(s, -t, 1.0)); }
        default: { return normalize(vec3f(-s, -t, -1.0)); }
    }
}

fn planet_shadow(p: vec3f, l: vec3f) -> f32 {
    let t = ray_sphere(p, l, R_BOTTOM);
    return select(1.0, 0.0, t > 0.0);
}

#ifdef_TRANSMITTANCE
@compute @workgroup_size(8, 8, 1)
fn cs_transmittance(@builtin(global_invocation_id) id: vec3u) {
    let dims = textureDimensions(out_tex);
    if (any(id.xy >= dims)) {
        return;
    }
    let uv = (vec2f(id.xy) + 0.5) / vec2f(dims);
    let p = transmittance_params(uv);
    let ro = vec3f(0.0, p.x, 0.0);
    let rd = vec3f(sqrt(max(0.0, 1.0 - p.y * p.y)), p.y, 0.0);
    let t_max = ray_sphere(ro, rd, R_TOP);
    let steps = 40.0;
    var od = vec3f(0.0);
    let dt = max(t_max, 0.0) / steps;
    for (var i = 0.0; i < steps; i += 1.0) {
        let x = ro + rd * (i + 0.5) * dt;
        let m = medium_at(length(x) - R_BOTTOM, frame.sky.y);
        od += m.extinction * dt;
    }
    textureStore(out_tex, id.xy, vec4f(exp(-od), 1.0));
}
#endif

#ifdef_SCATTER
fn integrate(ro: vec3f, rd: vec3f, l: vec3f, steps: f32, ground: bool, real_phase: bool) -> Scatter {
    var o: Scatter;
    let t_bottom = ray_sphere(ro, rd, R_BOTTOM);
    let t_top = ray_sphere(ro, rd, R_TOP);
    var t_max = t_top;
    if (t_bottom > 0.0) {
        t_max = t_bottom;
    }
    t_max = min(max(t_max, 0.0), 9000.0);
    let c = dot(rd, l);
    var ph_r = 1.0 / (4.0 * PI);
    var ph_m = ph_r;
    if (real_phase) {
        ph_r = phase_rayleigh(c);
        ph_m = phase_cornette(c, MIE_G);
    }
    var throughput = vec3f(1.0);
    var lum = vec3f(0.0);
    var ms = vec3f(0.0);
    for (var i = 0.0; i < steps; i += 1.0) {
        let t0 = t_max * sq(i / steps);
        let t1 = t_max * sq((i + 1.0) / steps);
        let dt = t1 - t0;
        let ts = t0 + 0.3 * dt;
        let p = ro + rd * ts;
        let r = length(p);
        let up = p / r;
        let m = medium_at(r - R_BOTTOM, frame.sky.y);
        let st = exp(-m.extinction * dt);
        let mu_l = dot(l, up);
        let t_sun = sample_transmittance(t_trans, samp_linear_clamp, r, mu_l) * planet_shadow(p, l);
        let scat = m.rayleigh * ph_r + m.mie * ph_m;
        var multi = vec3f(0.0);
        if (real_phase) {
            multi = textureSampleLevel(t_ms, samp_linear_clamp, multiscatter_uv(r, mu_l), 0.0).rgb;
        }
        let s = t_sun * scat + multi * m.scattering;
        let ext = max(m.extinction, vec3f(1e-7));
        ms += throughput * (m.scattering - m.scattering * st) / ext;
        lum += throughput * (s - s * st) / ext;
        throughput *= st;
    }
    if (ground && t_bottom > 0.0) {
        let p = ro + rd * t_bottom;
        let up = normalize(p);
        let t_sun = sample_transmittance(t_trans, samp_linear_clamp, R_BOTTOM, dot(l, up));
        lum += t_sun * throughput * saturate(dot(up, l)) * frame.sky.z * INV_PI;
    }
    o.l = lum;
    o.ms = ms;
    return o;
}
#endif

#ifdef_MULTISCATTER
@compute @workgroup_size(8, 8, 1)
fn cs_multiscatter(@builtin(global_invocation_id) id: vec3u) {
    let dims = textureDimensions(out_tex);
    if (any(id.xy >= dims)) {
        return;
    }
    let uv = (vec2f(id.xy) + 0.5) / vec2f(dims);
    let mu_s = uv.x * 2.0 - 1.0;
    let h = R_BOTTOM + saturate(uv.y) * (R_TOP - R_BOTTOM) + 0.001;
    let ro = vec3f(0.0, h, 0.0);
    let l = vec3f(0.0, mu_s, sqrt(max(0.0, 1.0 - mu_s * mu_s)));
    var lsum = vec3f(0.0);
    var msum = vec3f(0.0);
    for (var i = 0u; i < 64u; i++) {
        let u = (f32(i % 8u) + 0.5) / 8.0;
        let v = (f32(i / 8u) + 0.5) / 8.0;
        let theta = acos(1.0 - 2.0 * v);
        let phi = TAU * u;
        let rd = vec3f(sin(theta) * cos(phi), cos(theta), sin(theta) * sin(phi));
        let r = integrate(ro, rd, l, 20.0, true, false);
        lsum += r.l;
        msum += r.ms;
    }
    // Uniform sphere average times isotropic phase (4pi / 64 * 1/(4pi) = 1/64).
    let lum = lsum / 64.0;
    let fms = msum / 64.0;
    let psi = lum / max(vec3f(1.0) - fms, vec3f(1e-4));
    textureStore(out_tex, id.xy, vec4f(psi, 1.0));
}
#endif

#ifdef_SKYVIEW
@compute @workgroup_size(8, 8, 1)
fn cs_skyview(@builtin(global_invocation_id) id: vec3u) {
    let dims = textureDimensions(out_tex);
    if (any(id.xy >= dims)) {
        return;
    }
    let uv = (vec2f(id.xy) + 0.5) / vec2f(dims);
    let rd = skyview_dir(uv);
    let ro = vec3f(0.0, R_BOTTOM + max(frame.sky3.w, 0.0005), 0.0);
    var col = vec3f(0.0);
    if (frame.sun.w > 0.0) {
        col += integrate(ro, rd, frame.sun.xyz, 30.0, false, true).l * frame.sun.w;
    }
    if (frame.moon.w > 0.0) {
        col += integrate(ro, rd, frame.moon.xyz, 16.0, false, true).l * frame.moon.w * vec3f(0.9, 0.95, 1.05);
    }
    textureStore(out_tex, id.xy, vec4f(clamp_hdr(col), 1.0));
}
#endif

#ifdef_SKYCUBE
@compute @workgroup_size(8, 8, 1)
fn cs_skycube(@builtin(global_invocation_id) id: vec3u) {
    let dims = textureDimensions(out_cube).xy;
    if (any(id.xy >= dims)) {
        return;
    }
    let uv = (vec2f(id.xy) + 0.5) / vec2f(dims);
    let dir = cube_dir(id.z, uv);
    let col = sky_radiance(t_trans, t_sv, samp_linear_clamp, samp_linear_repeat, dir, false);
    textureStore(out_cube, id.xy, id.z, vec4f(clamp_hdr(col), 1.0));
}
#endif
