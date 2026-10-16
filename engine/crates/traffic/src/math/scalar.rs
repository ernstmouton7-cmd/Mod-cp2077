//! Scalar helpers shared by every system.
//!
//! These mirror `math/scalar.ts` exactly: modulo is floored (the sign follows
//! the divisor), `sign(0) == 0` (unlike [`f64::signum`]) and `clamp` never
//! panics when the bounds are inverted.

pub const PI: f64 = std::f64::consts::PI;
pub const TAU: f64 = std::f64::consts::PI * 2.0;
/// One degree in radians.
pub const DEG: f64 = std::f64::consts::PI / 180.0;
/// Gravity (m/s²).
pub const G: f64 = 9.81;
pub const EPS: f64 = 1e-9;
/// km/h → m/s factor.
pub const KMH: f64 = 1.0 / 3.6;

/// Floored modulo: the result has the sign of `b`.
#[inline]
pub fn fmod(a: f64, b: f64) -> f64 {
    a - (a / b).floor() * b
}

/// Clamp without the `lo <= hi` assertion of [`f64::clamp`].
#[inline]
pub fn clamp(v: f64, lo: f64, hi: f64) -> f64 {
    if v < lo {
        lo
    } else if v > hi {
        hi
    } else {
        v
    }
}

#[inline]
pub fn clamp01(v: f64) -> f64 {
    clamp(v, 0.0, 1.0)
}

#[inline]
pub fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

#[inline]
pub fn inv_lerp(a: f64, b: f64, v: f64) -> f64 {
    if b == a {
        0.0
    } else {
        (v - a) / (b - a)
    }
}

/// -1, 0 or 1 (zero maps to zero).
#[inline]
pub fn sign(v: f64) -> f64 {
    if v > 0.0 {
        1.0
    } else if v < 0.0 {
        -1.0
    } else {
        0.0
    }
}

/// Wrap an angle to (-π, π].
#[inline]
pub fn wrap_angle(a: f64) -> f64 {
    let w = fmod(a + PI, TAU) - PI;
    if w == -PI {
        PI
    } else {
        w
    }
}

/// Signed smallest difference `b - a` in (-π, π].
#[inline]
pub fn angle_diff(a: f64, b: f64) -> f64 {
    wrap_angle(b - a)
}

/// Move `current` toward `target` by at most `max_delta`.
#[inline]
pub fn approach(current: f64, target: f64, max_delta: f64) -> f64 {
    if current < target {
        (current + max_delta).min(target)
    } else {
        (current - max_delta).max(target)
    }
}

/// Frame-rate independent exponential smoothing factor.
#[inline]
pub fn smooth_factor(rate: f64, dt: f64) -> f64 {
    1.0 - (-rate * dt).exp()
}

#[inline]
pub fn smoothstep(e0: f64, e1: f64, x: f64) -> f64 {
    let t = clamp01((x - e0) / (e1 - e0));
    t * t * (3.0 - 2.0 * t)
}

/// Round half up to `decimals` places.
#[inline]
pub fn round_to(v: f64, decimals: i32) -> f64 {
    let f = 10f64.powi(decimals);
    (v * f + 0.5).floor() / f
}

/// JavaScript `Math.round`: halves round toward +∞.
#[inline]
pub fn js_round(v: f64) -> f64 {
    (v + 0.5).floor()
}

/// `sqrt(x² + y²)` (plain formula, matching the TypeScript helper).
#[inline]
pub fn hypot(x: f64, y: f64) -> f64 {
    (x * x + y * y).sqrt()
}

/// Piecewise-linear lookup in `(x, y)` pairs sorted by x.
pub fn curve(points: &[(f64, f64)], x: f64) -> f64 {
    let n = points.len();
    if x <= points[0].0 {
        return points[0].1;
    }
    if x >= points[n - 1].0 {
        return points[n - 1].1;
    }
    for i in 1..n {
        let p1 = points[i];
        if x <= p1.0 {
            let p0 = points[i - 1];
            return lerp(p0.1, p1.1, (x - p0.0) / (p1.0 - p0.0));
        }
    }
    points[n - 1].1
}

#[inline]
pub fn to_kmh(ms: f64) -> f64 {
    ms * 3.6
}

#[inline]
pub fn dist2(ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    let dx = ax - bx;
    let dy = ay - by;
    dx * dx + dy * dy
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helpers_match_ts_semantics() {
        assert_eq!(fmod(-1.0, 3.0), 2.0);
        assert_eq!(sign(0.0), 0.0);
        assert_eq!(sign(-0.0), 0.0);
        assert!((wrap_angle(3.0 * PI) - PI).abs() < 1e-12);
        assert_eq!(clamp(5.0, 10.0, 0.0), 10.0);
        assert_eq!(js_round(2.5), 3.0);
        assert_eq!(js_round(-2.5), -2.0);
    }
}
