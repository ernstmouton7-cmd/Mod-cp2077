// ---------------------------------------------------------------------------
// util.wgsl — copies the depth buffer into a sampleable R32F texture
// (soft particles read it while the depth buffer is attached).

@group(1) @binding(0) var t_depth_src: texture_depth_2d;

@fragment
fn fs_copy_depth(in: FsOut) -> @location(0) vec4f {
    return vec4f(textureLoad(t_depth_src, vec2i(in.pos.xy), 0), 0.0, 0.0, 0.0);
}
