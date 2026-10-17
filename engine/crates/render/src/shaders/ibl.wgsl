// ---------------------------------------------------------------------------
// ibl.wgsl — GPU image based lighting precomputation:
// equirect -> cube, cube mip chain, GGX prefiltered specular, SH9 diffuse,
// split-sum BRDF LUT. Group 1 bindings are injected (BINDINGS placeholder).

struct PrefilterParams {
    roughness: f32,
    src_size: f32,
    samples: u32,
    pad: u32,
};

${BINDINGS}

override SH_BASE: u32 = 0u;

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

#ifdef_EQUIRECT
@compute @workgroup_size(8, 8, 1)
fn cs_equirect(@builtin(global_invocation_id) id: vec3u) {
    let dims = textureDimensions(out_cube).xy;
    if (any(id.xy >= dims)) {
        return;
    }
    // Supersample 2x2 to avoid aliasing from high resolution sources.
    var c = vec3f(0.0);
    for (var k = 0u; k < 4u; k++) {
        let o = vec2f(f32(k & 1u), f32(k >> 1u)) * 0.5 + 0.25;
        let d = cube_dir(id.z, (vec2f(id.xy) + o) / vec2f(dims));
        let uv = vec2f(atan2(d.x, -d.z) / TAU + 0.5, acos(clamp(d.y, -1.0, 1.0)) / PI);
        c += textureSampleLevel(t_src, samp_linear_repeat, uv, 0.0).rgb;
    }
    textureStore(out_cube, id.xy, id.z, vec4f(clamp_hdr(c * 0.25), 1.0));
}
#endif

#ifdef_DOWNSAMPLE
@compute @workgroup_size(8, 8, 1)
fn cs_downsample(@builtin(global_invocation_id) id: vec3u) {
    let dims = textureDimensions(out_cube).xy;
    if (any(id.xy >= dims)) {
        return;
    }
    let p = vec2i(id.xy) * 2;
    let l = i32(id.z);
    let c = textureLoad(t_src_mip, p, l, 0) + textureLoad(t_src_mip, p + vec2i(1, 0), l, 0)
        + textureLoad(t_src_mip, p + vec2i(0, 1), l, 0) + textureLoad(t_src_mip, p + vec2i(1, 1), l, 0);
    textureStore(out_cube, id.xy, id.z, c * 0.25);
}
#endif

#ifdef_PREFILTER
@compute @workgroup_size(8, 8, 1)
fn cs_prefilter(@builtin(global_invocation_id) id: vec3u) {
    let dims = textureDimensions(out_cube).xy;
    if (any(id.xy >= dims)) {
        return;
    }
    let n = cube_dir(id.z, (vec2f(id.xy) + 0.5) / vec2f(dims));
    let rough = params.roughness;
    if (rough <= 0.0) {
        textureStore(out_cube, id.xy, id.z, vec4f(textureSampleLevel(t_env, samp_linear_clamp, n, 0.0).rgb, 1.0));
        return;
    }
    let a = rough * rough;
    let b = onb(n);
    var sum = vec3f(0.0);
    var wsum = 0.0;
    let count = params.samples;
    let texel_sa = 4.0 * PI / (6.0 * params.src_size * params.src_size);
    let max_lod = f32(textureNumLevels(t_env) - 1u);
    for (var i = 0u; i < count; i++) {
        let xi = hammersley(i, count);
        let h = b * importance_sample_ggx(xi, a);
        let l = normalize(2.0 * dot(n, h) * h - n);
        let nol = dot(n, l);
        if (nol > 0.0) {
            // Mip-filtered importance sampling (Krivanek & Colbert).
            let noh = saturate(dot(n, h));
            let pdf = d_ggx(noh, a) * 0.25;
            let sample_sa = 1.0 / (f32(count) * pdf + 1e-6);
            let lod = clamp(0.5 * log2(sample_sa / texel_sa) + 1.0, 0.0, max_lod);
            sum += textureSampleLevel(t_env, samp_linear_clamp, l, lod).rgb * nol;
            wsum += nol;
        }
    }
    textureStore(out_cube, id.xy, id.z, vec4f(clamp_hdr(sum / max(wsum, 1e-4)), 1.0));
}
#endif

#ifdef_SH
var<workgroup> sh_acc: array<array<vec4f, 9>, 64>;

@compute @workgroup_size(64, 1, 1)
fn cs_sh(@builtin(local_invocation_index) li: u32) {
    let dims = textureDimensions(t_src_mip).xy;
    let n = dims.x;
    let total = n * n * 6u;
    var acc: array<vec3f, 9>;
    for (var k = 0u; k < 9u; k++) {
        acc[k] = vec3f(0.0);
    }
    var wsum = 0.0;
    for (var i = li; i < total; i += 64u) {
        let face = i / (n * n);
        let rem = i % (n * n);
        let xy = vec2u(rem % n, rem / n);
        let uv = (vec2f(xy) + 0.5) / f32(n);
        let st = uv * 2.0 - 1.0;
        let w = 4.0 / (f32(n * n) * pow(1.0 + dot(st, st), 1.5));
        let d = cube_dir(face, uv);
        let c = textureLoad(t_src_mip, vec2i(xy), i32(face), 0).rgb * w;
        acc[0] += c * 0.282095;
        acc[1] += c * 0.488603 * d.y;
        acc[2] += c * 0.488603 * d.z;
        acc[3] += c * 0.488603 * d.x;
        acc[4] += c * 1.092548 * d.x * d.y;
        acc[5] += c * 1.092548 * d.y * d.z;
        acc[6] += c * 0.315392 * (3.0 * d.z * d.z - 1.0);
        acc[7] += c * 1.092548 * d.x * d.z;
        acc[8] += c * 0.546274 * (d.x * d.x - d.y * d.y);
        wsum += w;
    }
    for (var k = 0u; k < 9u; k++) {
        sh_acc[li][k] = vec4f(acc[k], wsum);
    }
    workgroupBarrier();
    var stride = 32u;
    while (stride > 0u) {
        if (li < stride) {
            for (var k = 0u; k < 9u; k++) {
                sh_acc[li][k] += sh_acc[li + stride][k];
            }
        }
        workgroupBarrier();
        stride = stride / 2u;
    }
    if (li == 0u) {
        // Normalise the solid angle to exactly 4pi, convolve with the cosine lobe and divide by pi.
        let norm = 4.0 * PI / max(sh_acc[0][0].w, 1e-6);
        let band = array<f32, 9>(PI, 2.0 * PI / 3.0, 2.0 * PI / 3.0, 2.0 * PI / 3.0, PI / 4.0, PI / 4.0, PI / 4.0, PI / 4.0, PI / 4.0);
        for (var k = 0u; k < 9u; k++) {
            sh_out[SH_BASE + k] = vec4f(sh_acc[0][k].rgb * norm * band[k] * INV_PI, 0.0);
        }
    }
}
#endif

#ifdef_BRDF
@compute @workgroup_size(8, 8, 1)
fn cs_brdf_lut(@builtin(global_invocation_id) id: vec3u) {
    let dims = textureDimensions(out_tex);
    if (any(id.xy >= dims)) {
        return;
    }
    let nov = max((f32(id.x) + 0.5) / f32(dims.x), 1e-3);
    let rough = max((f32(id.y) + 0.5) / f32(dims.y), 0.02);
    let a = rough * rough;
    let v = vec3f(sqrt(1.0 - nov * nov), 0.0, nov);
    var ab = vec2f(0.0);
    let count = 512u;
    for (var i = 0u; i < count; i++) {
        let xi = hammersley(i, count);
        let h = importance_sample_ggx(xi, a);
        let l = normalize(2.0 * dot(v, h) * h - v);
        let nol = saturate(l.z);
        let noh = saturate(h.z);
        let voh = saturate(dot(v, h));
        if (nol > 0.0) {
            let vis = v_smith_ggx_correlated(nov, nol, a);
            let g = vis * 4.0 * nol * voh / max(noh, 1e-5);
            let fc = pow(1.0 - voh, 5.0);
            ab += vec2f((1.0 - fc) * g, fc * g);
        }
    }
    ab /= f32(count);
    textureStore(out_tex, id.xy, vec4f(ab, 0.0, 1.0));
}
#endif
