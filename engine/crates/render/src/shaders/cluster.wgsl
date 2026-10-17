// ---------------------------------------------------------------------------
// cluster.wgsl — assigns local lights to view-space froxel clusters.
// Layout of `c_clusters`: per cluster [count, index * max_per_cluster].

@group(1) @binding(0) var<storage, read> c_lights: array<GpuLight>;
@group(1) @binding(1) var<storage, read_write> c_clusters: array<u32>;

fn tile_corner(ndc: vec2f, dist: f32) -> vec3f {
    let th = frame.proj_params.y;
    let asp = frame.proj_params.z;
    return vec3f(ndc.x * th * asp * dist, ndc.y * th * dist, -dist);
}

@compute @workgroup_size(4, 4, 4)
fn cs_cluster(@builtin(global_invocation_id) id: vec3u) {
    let dims = frame.cluster_dims;
    if (any(id >= dims.xyz)) {
        return;
    }
    let near = frame.cluster_z.x;
    let lg = frame.cluster_z.z;
    let z0 = near * exp(f32(id.z) / f32(dims.z) * lg);
    var z1 = near * exp(f32(id.z + 1u) / f32(dims.z) * lg);
    if (id.z + 1u == dims.z) {
        z1 = 1e6;
    }
    let x0 = f32(id.x) / f32(dims.x) * 2.0 - 1.0;
    let x1 = f32(id.x + 1u) / f32(dims.x) * 2.0 - 1.0;
    let y0 = 1.0 - f32(id.y + 1u) / f32(dims.y) * 2.0;
    let y1 = 1.0 - f32(id.y) / f32(dims.y) * 2.0;
    var lo = vec3f(1e9);
    var hi = vec3f(-1e9);
    for (var k = 0u; k < 8u; k++) {
        let nx = select(x0, x1, (k & 1u) != 0u);
        let ny = select(y0, y1, (k & 2u) != 0u);
        let dz = select(z0, z1, (k & 4u) != 0u);
        let c = tile_corner(vec2f(nx, ny), dz);
        lo = min(lo, c);
        hi = max(hi, c);
    }
    let maxc = dims.w;
    let stride = maxc + 1u;
    let base = ((id.z * dims.y + id.y) * dims.x + id.x) * stride;
    let count = u32(frame.cluster_z.w);
    var n = 0u;
    for (var i = 0u; i < count; i++) {
        let l = c_lights[i];
        let c = (frame.view * vec4f(l.pos_range.xyz, 1.0)).xyz;
        let r = l.pos_range.w;
        let q = clamp(c, lo, hi) - c;
        if (dot(q, q) <= r * r) {
            if (l.dir_cos.w > -1.5) {
                // Cheap cone test: reject clusters fully behind the spot.
                let cc = (lo + hi) * 0.5;
                let rad = length(hi - lo) * 0.5;
                let dv = (frame.view * vec4f(l.dir_cos.xyz, 0.0)).xyz;
                let to = cc - c;
                if (dot(to, dv) < -rad) {
                    continue;
                }
            }
            if (n < maxc) {
                c_clusters[base + 1u + n] = i;
                n += 1u;
            }
        }
    }
    c_clusters[base] = n;
}
