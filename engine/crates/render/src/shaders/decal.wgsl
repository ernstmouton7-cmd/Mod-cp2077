// ---------------------------------------------------------------------------
// decal.wgsl — deferred projected box decals (road paint, skid marks, stains).
// Blends albedo into GB0 and roughness into GB1.z.

struct GpuDecal {
    model: mat4x4f,
    inv_model: mat4x4f,
    color: vec4f,
    params: vec4f,   // kind, roughness, wear, dash
    params2: vec4f,  // gap, seed, length (m), width (m)
};

@group(1) @binding(0) var<storage, read> decals: array<GpuDecal>;
@group(1) @binding(1) var t_depth: texture_depth_2d;

struct DecalVs {
    @builtin(position) clip: vec4f,
    @location(0) @interpolate(flat) id: u32,
};

@vertex
fn vs_decal(@builtin(vertex_index) vi: u32, @builtin(instance_index) ii: u32) -> DecalVs {
    // 36-vertex unit cube.
    let idx = array<u32, 36>(0u, 2u, 1u, 1u, 2u, 3u, 4u, 5u, 6u, 5u, 7u, 6u, 0u, 1u, 4u, 1u, 5u, 4u,
                             2u, 6u, 3u, 3u, 6u, 7u, 0u, 4u, 2u, 2u, 4u, 6u, 1u, 3u, 5u, 3u, 7u, 5u);
    let c = idx[vi];
    let p = vec3f(f32(c & 1u), f32((c >> 1u) & 1u), f32((c >> 2u) & 1u)) - 0.5;
    var o: DecalVs;
    o.clip = frame.view_proj * (decals[ii].model * vec4f(p, 1.0));
    o.id = ii;
    return o;
}

struct DecalOut {
    @location(0) gb0: vec4f,
    @location(1) gb1: vec4f,
};

@fragment
fn fs_decal(i: DecalVs) -> DecalOut {
    let pix = vec2i(i.clip.xy);
    let d = textureLoad(t_depth, pix, 0);
    if (d <= 0.0) {
        discard;
    }
    let dc = decals[i.id];
    let uv = i.clip.xy * frame.viewport.zw;
    let world = world_from_depth(uv, d);
    let local = (dc.inv_model * vec4f(world, 1.0)).xyz;
    if (any(abs(local) > vec3f(0.5))) {
        discard;
    }
    let kind = u32(dc.params.x);
    let len = dc.params2.z;
    let wid = dc.params2.w;
    let zm = (local.z + 0.5) * len;
    let xm = (local.x + 0.5) * wid;
    // Anti-aliased edges in metres.
    let fw = max(fwidth(xm), 0.004);
    let fz = max(fwidth(zm), 0.004);
    var alpha = smoothstep(0.0, fw * 1.5, xm) * smoothstep(0.0, fw * 1.5, wid - xm);
    alpha *= smoothstep(0.0, fz * 1.5, zm) * smoothstep(0.0, fz * 1.5, len - zm);
    var albedo = dc.color.rgb;
    var rough = dc.params.y;
    let seed = dc.params2.y;
    let wear = dc.params.z;
    if (kind == 1u) {
        let period = dc.params.w + dc.params2.x;
        let ph = zm % period;
        alpha *= smoothstep(0.0, fz * 1.5, ph) * smoothstep(0.0, fz * 1.5, dc.params.w - ph);
    } else if (kind == 2u) {
        // Skid mark: streaks along the length, fading at the ends.
        let streak = value_noise2(vec2f(xm * 40.0, zm * 0.4 + seed)) * 0.6 + value_noise2(vec2f(xm * 120.0, zm * 2.0)) * 0.4;
        let ends = smoothstep(0.0, 1.5, zm) * smoothstep(0.0, 1.0, len - zm);
        alpha *= saturate(streak * 1.4 - 0.15) * ends;
        rough = mix(rough, 0.45, 0.5);
    } else if (kind == 3u) {
        let n = fbm2(world.xz * 1.3 + vec2f(seed), 4);
        let r = length(local.xz) * 2.0;
        alpha *= saturate((n - r * 0.6) * 2.5);
    }
    if (kind != 2u && kind != 3u) {
        // Worn paint: aggregate shows through, small cracks.
        let n = fbm2(world.xz * 6.0 + vec2f(seed * 13.0), 4);
        let speck = value_noise2(world.xz * 90.0);
        alpha *= saturate(1.0 - wear * (smoothstep(0.35, 0.75, n) * 0.9 + speck * 0.35));
        albedo *= 0.9 + 0.1 * speck;
    }
    alpha *= dc.color.a;
    if (alpha <= 0.001) {
        discard;
    }
    let wet = apply_wetness(world, vec3f(0.0, 1.0, 0.0), albedo, rough, 0.0, vec3f(0.0, 1.0, 0.0), 1.0);
    var o: DecalOut;
    o.gb0 = vec4f(saturate3(wet.albedo), alpha);
    o.gb1 = vec4f(0.0, 0.0, wet.roughness, alpha);
    return o;
}
