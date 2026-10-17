// ---------------------------------------------------------------------------
// gbuffer.wgsl — opaque/alpha-tested geometry into the G-buffer.
//  target 0: HDR colour (pre-exposed emissive)
//  target 1: rgba8 srgb   albedo.rgb, material AO
//  target 2: rgba16f      oct normal.xy, roughness, metallic
//  target 3: rgba8        clearcoat|translucency, coat roughness, flake offset.xy
//  target 4: rgba16f      velocity.xy (uv delta), shading model, reflectance

override ALPHA_MASK: bool = false;

struct GOut {
    @location(0) emissive: vec4f,
    @location(1) gb0: vec4f,
    @location(2) gb1: vec4f,
    @location(3) gb2: vec4f,
    @location(4) gb3: vec4f,
};

@fragment
fn fs_gbuffer(i: VsOut, @builtin(front_facing) front: bool) -> GOut {
    let m = sample_material(i, front);
    if (ALPHA_MASK) {
        if (m.albedo.a < mat.pbr.w) {
            discard;
        }
    }
    let inst = instances[i.inst];
    let geo_n = select(-1.0, 1.0, front) * normalize(i.normal);
    let wet = apply_wetness(i.world, geo_n, m.albedo.rgb, m.roughness, m.metallic, m.n, mat.extra.z);

    var model = 0.0;
    var gb2 = vec4f(0.0, 0.0, 0.5, 0.5);
    if (mat.coat.x > 0.0) {
        model = 1.0;
        let off = flake_offset(i.obj_pos, wet.n, inst);
        let b = onb(wet.n);
        let o2 = clamp(vec2f(dot(off, b[0]), dot(off, b[1])), vec2f(-1.0), vec2f(1.0));
        gb2 = vec4f(mat.coat.x, mat.coat.y, o2 * 0.5 + 0.5);
    } else if (mat.extra.w > 0.0) {
        model = 2.0;
        gb2.x = mat.extra.w;
    }

    let cur = i.cur_clip.xy / i.cur_clip.w;
    let prev = i.prev_clip.xy / i.prev_clip.w;
    let vel = ndc_to_uv(cur) - ndc_to_uv(prev);

    var o: GOut;
    o.emissive = vec4f(clamp_hdr(m.emissive * pre_exposure()), 0.0);
    o.gb0 = vec4f(saturate3(wet.albedo), m.ao);
    o.gb1 = vec4f(oct_encode(wet.n), clamp(wet.roughness, 0.02, 1.0), saturate(wet.metallic));
    o.gb2 = gb2;
    o.gb3 = vec4f(vel, model, mat.extra2.x);
    return o;
}

// Wireframe debug: flat colour.
@fragment
fn fs_wire(i: VsOut) -> GOut {
    var o: GOut;
    o.emissive = vec4f(vec3f(0.2, 1.0, 0.3) * 2.0, 0.0);
    o.gb0 = vec4f(0.0, 0.0, 0.0, 1.0);
    o.gb1 = vec4f(oct_encode(normalize(i.normal)), 1.0, 0.0);
    o.gb2 = vec4f(0.0, 0.0, 0.5, 0.5);
    o.gb3 = vec4f(0.0, 0.0, 3.0, 0.5);
    return o;
}
