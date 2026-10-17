// ---------------------------------------------------------------------------
// debug.wgsl — debug lines drawn on the final image; hidden parts are dimmed.

override OUTPUT_ENCODE_SRGB: bool = false;

@group(1) @binding(0) var t_scene_depth: texture_depth_2d;

struct LineVs {
    @builtin(position) clip: vec4f,
    @location(0) color: vec4f,
    @location(1) ndc_z: f32,
};

@vertex
fn vs_line(@location(0) pos: vec3f, @location(1) color: vec4f) -> LineVs {
    var o: LineVs;
    o.clip = frame.view_proj_nj * vec4f(pos, 1.0);
    o.color = color;
    o.ndc_z = 0.0;
    return o;
}

@fragment
fn fs_line(i: LineVs) -> @location(0) vec4f {
    let uv = i.clip.xy * frame.output.zw;
    let dims = vec2f(textureDimensions(t_scene_depth));
    let sd = textureLoad(t_scene_depth, vec2i(clamp(uv * dims, vec2f(0.0), dims - 1.0)), 0);
    var c = i.color;
    if (i.clip.z < sd * 0.999) {
        c.a *= 0.3;
    }
    var rgb = c.rgb;
    if (OUTPUT_ENCODE_SRGB) {
        rgb = pow(max(rgb, vec3f(0.0)), vec3f(1.0 / 2.2));
    }
    return vec4f(rgb * c.a, c.a);
}
