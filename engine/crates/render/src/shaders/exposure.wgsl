// ---------------------------------------------------------------------------
// exposure.wgsl — luminance histogram + adaptation (all on the GPU, no readback).
// Histogram covers log2(luminance) in [MIN_LOG, MIN_LOG + RANGE_LOG].

struct ExposureParams {
    min_ev: f32,
    max_ev: f32,
    compensation: f32,
    speed: f32,
    dt: f32,
    manual_ev: f32,   // < -100 = auto
    reset: f32,
    low_pct: f32,
    high_pct: f32,
    pad0: f32,
    pad1: f32,
    pad2: f32,
};

const MIN_LOG: f32 = -12.0;
const RANGE_LOG: f32 = 30.0;

@group(1) @binding(0) var<storage, read_write> exp_rw: Exposure;
@group(1) @binding(1) var<storage, read_write> hist: array<atomic<u32>, 256>;
@group(1) @binding(2) var<uniform> ep: ExposureParams;
@group(1) @binding(3) var t_lum_src: texture_2d<f32>;

@compute @workgroup_size(1, 1, 1)
fn cs_begin() {
    exp_rw.prev_pre = exp_rw.pre;
    exp_rw.pre = exp_rw.current;
}

var<workgroup> local_hist: array<atomic<u32>, 256>;

@compute @workgroup_size(16, 16, 1)
fn cs_histogram(@builtin(global_invocation_id) gid: vec3u, @builtin(local_invocation_index) li: u32) {
    atomicStore(&local_hist[li], 0u);
    workgroupBarrier();
    let dims = textureDimensions(t_lum_src);
    let p = gid.xy * 2u;
    if (all(p < dims)) {
        let c = textureLoad(t_lum_src, p, 0).rgb;
        let lum = luminance(c) / max(exp_rw.pre, 1e-12);
        var bin = 0u;
        if (lum > 1e-9) {
            let t = saturate((log2(lum) - MIN_LOG) / RANGE_LOG);
            bin = u32(t * 254.0) + 1u;
        }
        // Centre-weighted metering.
        let uv = vec2f(p) / vec2f(dims) * 2.0 - 1.0;
        let w = u32(mix(4.0, 1.0, saturate(length(uv * vec2f(0.8, 1.0)))));
        atomicAdd(&local_hist[bin], w);
    }
    workgroupBarrier();
    atomicAdd(&hist[li], atomicLoad(&local_hist[li]));
}

var<workgroup> wg_counts: array<f32, 256>;

@compute @workgroup_size(256, 1, 1)
fn cs_adapt(@builtin(local_invocation_index) li: u32) {
    wg_counts[li] = f32(atomicLoad(&hist[li]));
    atomicStore(&hist[li], 0u);
    workgroupBarrier();
    if (li != 0u) {
        return;
    }
    var total = 0.0;
    for (var i = 0u; i < 256u; i++) {
        total += wg_counts[i];
    }
    let lo = total * ep.low_pct;
    let hi = total * ep.high_pct;
    var acc = 0.0;
    var sum = 0.0;
    var wsum = 0.0;
    for (var i = 0u; i < 256u; i++) {
        let c = wg_counts[i];
        let a0 = acc;
        let a1 = acc + c;
        acc = a1;
        let inside = max(0.0, min(a1, hi) - max(a0, lo));
        if (inside > 0.0) {
            let lg = select(MIN_LOG, MIN_LOG + (f32(i - 1u) + 0.5) / 254.0 * RANGE_LOG, i > 0u);
            sum += lg * inside;
            wsum += inside;
        }
    }
    let avg_log = select(-3.0, sum / max(wsum, 1e-6), wsum > 0.0);
    let avg_lum = exp2(avg_log);
    // EV100 for which the average maps to middle grey (K = 12.5).
    var target_ev = log2(avg_lum * 100.0 / 12.5) - ep.compensation;
    target_ev = clamp(target_ev, ep.min_ev, ep.max_ev);
    if (ep.manual_ev > -100.0) {
        target_ev = ep.manual_ev - ep.compensation;
    }
    var ev = exp_rw.ev100;
    if (ep.reset > 0.5 || ep.manual_ev > -100.0) {
        ev = target_ev;
    } else {
        // Asymmetric adaptation: eyes adapt faster to bright.
        let speed = select(ep.speed * 0.6, ep.speed, target_ev > ev);
        ev = ev + (target_ev - ev) * (1.0 - exp(-ep.dt * speed));
    }
    exp_rw.ev100 = ev;
    exp_rw.target_ev = target_ev;
    exp_rw.avg_lum = avg_lum;
    exp_rw.current = 1.0 / (1.2 * exp2(ev));
}
