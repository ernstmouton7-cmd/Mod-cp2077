// ---------------------------------------------------------------------------
// material.wgsl — instance + material bindings, vertex transform, texture
// sampling (mesh UV / triplanar / world XZ), rain wetness and car-paint flakes.

struct Instance {
    model: mat4x4f,
    prev_model: mat4x4f,
    normal0: vec4f,
    normal1: vec4f,
    normal2: vec4f,
    tint: vec4f,
    params: vec4f,   // emissive scale, 0, 0, 0
};

struct MaterialU {
    base_color: vec4f,
    emissive: vec4f,   // rgb nits, w = normal scale
    pbr: vec4f,        // metallic, roughness, ao strength, alpha cutoff
    coat: vec4f,       // clearcoat, clearcoat roughness, flake strength, flake scale
    extra: vec4f,      // transmission, ior, wet factor, translucency
    uv: vec4f,         // scale xy, offset xy
    flags: vec4u,      // uv mode, texture mask, packed metal-rough, alpha mode
    extra2: vec4f,     // reflectance, double sided, 0, 0
};

@group(1) @binding(0) var<storage, read> instances: array<Instance>;
@group(1) @binding(1) var<storage, read> visible: array<u32>;

@group(2) @binding(0) var<uniform> mat: MaterialU;
@group(2) @binding(1) var t_albedo: texture_2d<f32>;
@group(2) @binding(2) var t_normal: texture_2d<f32>;
@group(2) @binding(3) var t_rough: texture_2d<f32>;
@group(2) @binding(4) var t_metal: texture_2d<f32>;
@group(2) @binding(5) var t_occl: texture_2d<f32>;
@group(2) @binding(6) var t_emis: texture_2d<f32>;
@group(2) @binding(7) var s_mat: sampler;

const TEX_ALBEDO: u32 = 1u;
const TEX_NORMAL: u32 = 2u;
const TEX_ROUGH: u32 = 4u;
const TEX_METAL: u32 = 8u;
const TEX_AO: u32 = 16u;
const TEX_EMISSIVE: u32 = 32u;

const UV_MESH: u32 = 0u;
const UV_MESH2: u32 = 1u;
const UV_TRIPLANAR: u32 = 2u;
const UV_WORLDXZ: u32 = 3u;

struct VsIn {
    @builtin(instance_index) instance: u32,
    @location(0) position: vec3f,
    @location(1) normal: vec3f,
    @location(2) tangent: vec4f,
    @location(3) uv: vec2f,
    @location(4) uv2: vec2f,
    @location(5) color: vec4f,
};

struct VsOut {
    @builtin(position) clip: vec4f,
    @location(0) world: vec3f,
    @location(1) normal: vec3f,
    @location(2) tangent: vec4f,
    @location(3) uv: vec4f,
    @location(4) color: vec4f,
    @location(5) obj_pos: vec3f,
    @location(6) obj_normal: vec3f,
    @location(7) cur_clip: vec4f,
    @location(8) prev_clip: vec4f,
    @location(9) @interpolate(flat) inst: u32,
};

fn instance_normal_matrix(inst: Instance) -> mat3x3f {
    return mat3x3f(inst.normal0.xyz, inst.normal1.xyz, inst.normal2.xyz);
}

@vertex
fn vs_scene(v: VsIn) -> VsOut {
    let idx = visible[v.instance];
    let inst = instances[idx];
    var o: VsOut;
    let world = inst.model * vec4f(v.position, 1.0);
    let nm = instance_normal_matrix(inst);
    o.world = world.xyz;
    o.normal = normalize(nm * v.normal);
    o.tangent = vec4f(normalize((inst.model * vec4f(v.tangent.xyz, 0.0)).xyz), v.tangent.w);
    o.uv = vec4f(v.uv, v.uv2);
    o.color = v.color;
    o.obj_pos = v.position;
    o.obj_normal = v.normal;
    o.clip = frame.view_proj * world;
    o.cur_clip = frame.view_proj_nj * world;
    o.prev_clip = frame.prev_view_proj * (inst.prev_model * vec4f(v.position, 1.0));
    o.inst = idx;
    return o;
}

struct MatSample {
    albedo: vec4f,
    n: vec3f,         // world shading normal
    roughness: f32,
    metallic: f32,
    ao: f32,
    emissive: vec3f,
};

fn tex_uv(i: VsOut) -> vec2f {
    var uv = i.uv.xy;
    if (mat.flags.x == UV_MESH2) {
        uv = i.uv.zw;
    } else if (mat.flags.x == UV_WORLDXZ) {
        uv = i.world.xz;
    }
    return uv * mat.uv.xy + mat.uv.zw;
}

fn unpack_normal(t: vec4f) -> vec3f {
    let xy = t.xy * 2.0 - 1.0;
    let z = sqrt(saturate(1.0 - dot(xy, xy)));
    return vec3f(xy * mat.emissive.w, z);
}

fn sample_planar(uv: vec2f, mask: u32) -> array<vec4f, 6> {
    var r: array<vec4f, 6>;
    r[0] = vec4f(1.0);
    r[1] = vec4f(0.5, 0.5, 1.0, 1.0);
    r[2] = vec4f(1.0);
    r[3] = vec4f(1.0);
    r[4] = vec4f(1.0);
    r[5] = vec4f(1.0);
    if ((mask & TEX_ALBEDO) != 0u) { r[0] = textureSample(t_albedo, s_mat, uv); }
    if ((mask & TEX_NORMAL) != 0u) { r[1] = textureSample(t_normal, s_mat, uv); }
    if ((mask & TEX_ROUGH) != 0u) { r[2] = textureSample(t_rough, s_mat, uv); }
    if ((mask & TEX_METAL) != 0u) { r[3] = textureSample(t_metal, s_mat, uv); }
    if ((mask & TEX_AO) != 0u) { r[4] = textureSample(t_occl, s_mat, uv); }
    if ((mask & TEX_EMISSIVE) != 0u) { r[5] = textureSample(t_emis, s_mat, uv); }
    return r;
}

fn sample_material(i: VsOut, front: bool) -> MatSample {
    var m: MatSample;
    let inst = instances[i.inst];
    let mask = mat.flags.y;
    let geo_n = select(-1.0, 1.0, front) * normalize(i.normal);
    var t: array<vec4f, 6>;
    var n = geo_n;
    if (mat.flags.x == UV_TRIPLANAR) {
        // Object space triplanar with whiteout normal blending.
        let p = i.obj_pos * mat.uv.x + vec3f(mat.uv.z, mat.uv.w, 0.0);
        let on = select(-1.0, 1.0, front) * normalize(i.obj_normal);
        var w = pow(abs(on), vec3f(4.0));
        w /= (w.x + w.y + w.z);
        let tx = sample_planar(vec2f(p.z * sign(on.x), -p.y), mask);
        let ty = sample_planar(vec2f(p.x, p.z * sign(on.y)), mask);
        let tz = sample_planar(vec2f(-p.x * sign(on.z), -p.y), mask);
        for (var k = 0; k < 6; k++) {
            t[k] = tx[k] * w.x + ty[k] * w.y + tz[k] * w.z;
        }
        if ((mask & TEX_NORMAL) != 0u) {
            let nx = unpack_normal(tx[1]);
            let ny = unpack_normal(ty[1]);
            let nz = unpack_normal(tz[1]);
            let bx = vec3f(nx.xy + on.zy, abs(nx.z) * on.x);
            let by = vec3f(ny.xy + on.xz, abs(ny.z) * on.y);
            let bz = vec3f(nz.xy + on.xy, abs(nz.z) * on.z);
            let obj_n = normalize(bx.zyx * w.x + by.xzy * w.y + bz.xyz * w.z);
            n = normalize(instance_normal_matrix(inst) * obj_n);
        }
    } else {
        let uv = tex_uv(i);
        t = sample_planar(uv, mask);
        if ((mask & TEX_NORMAL) != 0u) {
            let tn = unpack_normal(t[1]);
            var tg: vec3f;
            var bt: vec3f;
            if (mat.flags.x == UV_WORLDXZ) {
                tg = vec3f(1.0, 0.0, 0.0);
                bt = vec3f(0.0, 0.0, -1.0);
            } else {
                tg = normalize(i.tangent.xyz - geo_n * dot(geo_n, i.tangent.xyz));
                bt = cross(geo_n, tg) * i.tangent.w;
            }
            n = normalize(tg * tn.x + bt * tn.y + geo_n * tn.z);
        }
    }
    m.albedo = mat.base_color * t[0] * i.color * inst.tint;
    if (mat.flags.z != 0u) {
        m.roughness = mat.pbr.y * t[2].g;
        m.metallic = mat.pbr.x * t[2].b;
    } else {
        m.roughness = mat.pbr.y * t[2].r;
        m.metallic = mat.pbr.x * t[3].r;
    }
    m.ao = mix(1.0, t[4].r, mat.pbr.z);
    m.emissive = mat.emissive.rgb * t[5].rgb * i.color.rgb * inst.params.x;
    m.n = n;
    return m;
}

// Metallic flake perturbation (object space hash cells, faded by pixel footprint).
fn flake_offset(obj_pos: vec3f, world_n: vec3f, inst: Instance) -> vec3f {
    let strength = mat.coat.z;
    if (strength <= 0.0) {
        return vec3f(0.0);
    }
    let scale = max(mat.coat.w, 1e-4);
    let footprint = length(fwidth(obj_pos));
    let fade = saturate(scale * 1.5 / max(footprint, 1e-6));
    let h = hash33(floor(obj_pos / scale)) * 2.0 - 1.0;
    let wv = instance_normal_matrix(inst) * h;
    let tangential = wv - world_n * dot(world_n, wv);
    return tangential * strength * fade;
}
