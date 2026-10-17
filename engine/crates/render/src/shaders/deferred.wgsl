// ---------------------------------------------------------------------------
// deferred.wgsl — fullscreen lighting of the G-buffer + sky + fog.
// Output blends as  dst = src.rgb + dst.rgb * src.a  so fog also attenuates
// the emissive term written by the G-buffer pass.

@group(1) @binding(0) var t_depth: texture_depth_2d;
@group(1) @binding(1) var t_gb0: texture_2d<f32>;
@group(1) @binding(2) var t_gb1: texture_2d<f32>;
@group(1) @binding(3) var t_gb2: texture_2d<f32>;
@group(1) @binding(4) var t_gb3: texture_2d<f32>;

fn background(dir: vec3f) -> vec3f {
    if (frame.sky3.x > 0.5) {
        return textureSampleLevel(t_hdri_cube, samp_linear_clamp, rotate_y(dir, frame.sky2.z), 0.0).rgb * frame.sky2.w;
    }
    return sky_radiance(t_transmittance, t_skyview, samp_linear_clamp, samp_linear_repeat, dir, true);
}

fn build_surface(pos: vec3f, v: vec3f, g0: vec4f, g1: vec4f, g2: vec4f, g3: vec4f, ao: f32) -> Surface {
    var s: Surface;
    s.pos = pos;
    s.v = v;
    s.n = oct_decode(g1.xy);
    s.model = u32(g3.z + 0.5);
    let metallic = g1.w;
    let base = g0.rgb;
    s.roughness = max(g1.z, MIN_ROUGHNESS);
    s.a = s.roughness * s.roughness;
    s.albedo = base * (1.0 - metallic);
    let refl = g3.w;
    s.f0 = mix(vec3f(0.16 * refl * refl), base, metallic);
    s.base_n = s.n;
    s.clearcoat = 0.0;
    s.translucency = 0.0;
    s.cc_roughness = 0.0;
    s.cc_a = 0.0;
    if (s.model == 1u) {
        s.clearcoat = g2.x;
        s.cc_roughness = max(g2.y, MIN_ROUGHNESS);
        s.cc_a = s.cc_roughness * s.cc_roughness;
        s.f0 = mix(s.f0, f0_under_coat(s.f0), s.clearcoat);
        let off = g2.zw * 2.0 - 1.0;
        let b = onb(s.n);
        s.base_n = normalize(s.n + b[0] * off.x + b[1] * off.y);
    } else if (s.model == 2u) {
        s.translucency = g2.x;
    }
    s.nov = max(dot(s.n, v), 1e-4);
    s.ao = min(g0.a, ao);
    let lut = textureSampleLevel(t_brdf_lut, samp_linear_clamp, vec2f(max(dot(s.base_n, v), 1e-4), s.roughness), 0.0).rg;
    s.energy_comp = 1.0 + s.f0 * (1.0 / max(lut.x + lut.y, 1e-3) - 1.0);
    return s;
}

@fragment
fn fs_deferred(in: FsOut) -> @location(0) vec4f {
    let pix = vec2i(in.pos.xy);
    let uv = in.pos.xy * frame.viewport.zw;
    let d = textureLoad(t_depth, pix, 0);
    let pre = pre_exposure();
    let cam = frame.camera_pos.xyz;
    if (d <= 0.0) {
        let dir = view_dir_from_uv(uv);
        let vol = eval_volumetric_lights(cam, dir, 400.0, in.pos.xy);
        // Height fog also veils the sky near the horizon (keeps distant geometry and sky consistent).
        let fog = eval_fog_shadowed(cam, dir, 1500.0, fog_light_visibility(cam, dir, 1500.0, in.pos.xy));
        return vec4f(clamp_hdr((background(dir) * fog.transmittance + fog.inscatter + vol) * pre), 1.0);
    }
    let g0 = textureLoad(t_gb0, pix, 0);
    let g1 = textureLoad(t_gb1, pix, 0);
    let g2 = textureLoad(t_gb2, pix, 0);
    let g3 = textureLoad(t_gb3, pix, 0);
    if (g3.z > 2.5) {
        return vec4f(0.0, 0.0, 0.0, 1.0);
    }
    let world = world_from_depth(uv, d);
    let to_cam = cam - world;
    let dist = length(to_cam);
    let v = to_cam / max(dist, 1e-5);
    var ao = 1.0;
    if (frame.post.x > 0.5) {
        ao = textureSampleLevel(t_ao, samp_linear_clamp, uv, 0.0).r;
    }
    var s = build_surface(world, v, g0, g1, g2, g3, ao);
    s.pix = in.pos.xy;
    let h_km = camera_altitude_km();
    let view_z = dot(world - cam, -frame.inv_view[2].xyz);

    var col = vec3f(0.0);
    // Sun and moon.
    let shadow = sun_shadow(world, s.n, view_z, in.pos.xy);
    let moon_primary = frame.light_params.x > 0.5;
    let sun_e = sun_illuminance_at(h_km);
    if (frame.sun.y > -0.05 && frame.sun.w > 0.0) {
        let sh = select(shadow, 1.0, moon_primary) * cloud_shadow(world, frame.sun.xyz);
        col += shade_light(s, frame.sun.xyz, sun_e * sh, frame.light_params.y);
    }
    if (frame.moon.y > -0.05 && frame.moon.w > 0.0) {
        let sh = select(1.0, shadow, moon_primary) * cloud_shadow(world, frame.moon.xyz);
        col += shade_light(s, frame.moon.xyz, moon_illuminance_at(h_km) * sh, frame.light_params.z);
    }
    col += eval_local_lights(s, in.pos.xy, view_z, s.n);

    var ssr = vec4f(0.0);
    if (frame.post.y > 0.5) {
        ssr = textureSampleLevel(t_ssr, samp_linear_clamp, uv, 0.0);
        ssr = vec4f(ssr.rgb / max(pre, 1e-12), ssr.a);
    }
    let amb = eval_ambient(s, ssr);
    let direct = col;
    col += amb.diffuse + amb.specular;
    let dbg = u32(frame.post.w);
    if (dbg != 0u) {
        // Debug views: linear values scaled so ~1 maps to mid/high grey (exposure still applies).
        var c = vec3f(0.0);
        switch (dbg) {
            case 1u: { c = g0.rgb; }
            case 2u: { c = s.n * 0.5 + 0.5; }
            case 3u: { c = vec3f(g1.z); }
            case 4u: { c = vec3f(s.ao); }
            case 5u: { c = vec3f(sky_visibility(world, s.n, in.pos.xy)); }
            case 6u: { return vec4f(clamp_hdr((direct + amb.diffuse) * pre), 0.0); }
            case 7u: { return vec4f(clamp_hdr(amb.specular * pre), 0.0); }
            case 8u: { c = ssr.rgb * pre * ssr.a; }
            default: { c = vec3f(g1.w); }
        }
        // Encode for a fixed EV so the albedo-type views are readable.
        return vec4f(c * pre / max(exposure.current, 1e-12) * 0.35, 0.0);
    }

    let fog = eval_fog_shadowed(cam, -v, dist, fog_light_visibility(cam, -v, dist, in.pos.xy));
    let vol = eval_volumetric_lights(cam, -v, dist, in.pos.xy);
    let outc = (col * fog.transmittance + fog.inscatter + vol) * pre;
    return vec4f(clamp_hdr(outc), fog.transmittance);
}
