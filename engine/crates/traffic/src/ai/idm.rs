//! Intelligent Driver Model car following.

/// Parameters of the Intelligent Driver Model.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IdmParams {
    /// Desired speed (m/s).
    pub v0: f64,
    /// Time headway (s).
    pub t: f64,
    /// Max acceleration (m/s²).
    pub a: f64,
    /// Comfortable deceleration (m/s²).
    pub b: f64,
    /// Jam distance (m).
    pub s0: f64,
}

/// IDM free-road term (Treiber's improved variant above the desired speed).
pub fn idm_free(v: f64, p: &IdmParams) -> f64 {
    if p.v0 <= 0.1 {
        return -p.b * 2.0;
    }
    let r = v / p.v0;
    if r <= 1.0 {
        return p.a * (1.0 - r * r * r * r);
    }
    -p.b * (1.0 - 1.0 / (r * r))
}

/// Full IDM acceleration toward a leader: `gap` bumper to bumper (m), `dv`
/// approach rate v − v_leader (m/s).
pub fn idm_accel(v: f64, gap: f64, dv: f64, p: &IdmParams) -> f64 {
    let free = idm_free(v, p);
    let g = gap.max(0.05);
    let s_star = p.s0 + (v * p.t + (v * dv) / (2.0 * (p.a * p.b).sqrt())).max(0.0);
    let inter = p.a * (s_star / g) * (s_star / g);
    free - inter
}

/// Deceleration needed to stop within `dist` from speed `v`.
pub fn decel_to_stop(v: f64, dist: f64) -> f64 {
    if dist <= 0.05 {
        return if v > 0.1 { 99.0 } else { 0.0 };
    }
    (v * v) / (2.0 * dist)
}
