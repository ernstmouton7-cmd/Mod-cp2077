// ---------------------------------------------------------------------------
// shadow.wgsl — depth-only rendering for cascades and spot shadow maps.

override ALPHA_MASK: bool = false;

struct ShadowView {
    view_proj: mat4x4f,
    params: vec4f,
};

@group(0) @binding(6) var<uniform> sview: ShadowView;

struct ShadowOut {
    @builtin(position) clip: vec4f,
    @location(0) uv: vec2f,
};

@vertex
fn vs_shadow(v: VsIn) -> ShadowOut {
    let inst = instances[visible[v.instance]];
    var o: ShadowOut;
    let world = inst.model * vec4f(v.position, 1.0);
    o.clip = sview.view_proj * world;
    o.uv = v.uv * mat.uv.xy + mat.uv.zw;
    return o;
}

@fragment
fn fs_shadow(i: ShadowOut) {
    if (ALPHA_MASK) {
        let a = textureSample(t_albedo, s_mat, i.uv).a * mat.base_color.a;
        if (a < mat.pbr.w) {
            discard;
        }
    }
}
