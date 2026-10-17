// ---------------------------------------------------------------------------
// forward.wgsl — alpha blended / thin-glass surfaces (premultiplied output).

@fragment
fn fs_forward(i: VsOut, @builtin(front_facing) front: bool) -> @location(0) vec4f {
    let m = sample_material(i, front);
    let pre = pre_exposure();
    let cam = frame.camera_pos.xyz;
    let to_cam = cam - i.world;
    let dist = length(to_cam);
    let v = to_cam / max(dist, 1e-5);
    let transmission = mat.extra.x;
    let opacity = m.albedo.a;

    var s: Surface;
    s.pos = i.world;
    s.v = v;
    s.n = m.n;
    if (dot(s.n, v) < 0.0) {
        s.n = -s.n;
    }
    s.base_n = s.n;
    s.model = 0u;
    s.pix = i.clip.xy;
    s.roughness = max(m.roughness, MIN_ROUGHNESS);
    s.a = s.roughness * s.roughness;
    let ior = mat.extra.y;
    var f0s = sq((ior - 1.0) / (ior + 1.0));
    if (transmission <= 0.0) {
        f0s = 0.16 * sq(mat.extra2.x);
    }
    s.f0 = mix(vec3f(f0s), m.albedo.rgb, m.metallic);
    // Glass: the "diffuse" part is the tint/dirt fraction given by opacity.
    s.albedo = m.albedo.rgb * (1.0 - m.metallic) * select(1.0, opacity, transmission > 0.0);
    s.ao = m.ao;
    s.clearcoat = 0.0;
    s.cc_roughness = 0.0;
    s.cc_a = 0.0;
    s.translucency = 0.0;
    s.nov = max(dot(s.n, v), 1e-4);
    let lut = textureSampleLevel(t_brdf_lut, samp_linear_clamp, vec2f(s.nov, s.roughness), 0.0).rg;
    s.energy_comp = 1.0 + s.f0 * (1.0 / max(lut.x + lut.y, 1e-3) - 1.0);

    let h_km = camera_altitude_km();
    let view_z = dot(i.world - cam, -frame.inv_view[2].xyz);
    var col = vec3f(0.0);
    let shadow = sun_shadow(i.world, s.n, view_z, i.clip.xy);
    let moon_primary = frame.light_params.x > 0.5;
    if (frame.sun.y > -0.05) {
        let sh = select(shadow, 1.0, moon_primary) * cloud_shadow(i.world, frame.sun.xyz);
        col += shade_light(s, frame.sun.xyz, sun_illuminance_at(h_km) * sh, frame.light_params.y);
    }
    if (frame.moon.y > -0.05) {
        let sh = select(1.0, shadow, moon_primary);
        col += shade_light(s, frame.moon.xyz, moon_illuminance_at(h_km) * sh, frame.light_params.z);
    }
    col += eval_local_lights(s, i.clip.xy, view_z, s.n);
    let amb = eval_ambient(s, vec4f(0.0));
    col += amb.specular + amb.diffuse;
    col += m.emissive;

    var alpha = opacity;
    if (transmission > 0.0) {
        let f = f_schlick(f0s, 1.0, s.nov);
        let t = (1.0 - f) * (1.0 - opacity) * transmission;
        alpha = 1.0 - t;
    } else {
        col *= opacity;
    }
    let fog = eval_fog(cam, -v, dist);
    let outc = (col * fog.transmittance + fog.inscatter * alpha) * pre;
    return vec4f(clamp_hdr(outc), saturate(alpha));
}
