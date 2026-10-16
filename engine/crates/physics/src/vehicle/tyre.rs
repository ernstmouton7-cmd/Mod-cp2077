//! Combined-slip tyre model (normalised-slip "similarity" method with a magic-formula shape).

use crate::body::Surface;

#[derive(Clone, Copy, Debug)]
pub struct TyreDesc {
    /// Peak friction coefficient on dry asphalt.
    pub mu: f32,
    /// Sliding friction as a fraction of peak (0.6–0.85). Sets the magic-formula C.
    pub slide_ratio: f32,
    /// Slip ratio at peak longitudinal force.
    pub peak_slip_ratio: f32,
    /// Slip angle at peak lateral force (rad).
    pub peak_slip_angle: f32,
    /// Grip lost per unit of load above nominal (0 = none).
    pub load_sensitivity: f32,
    /// Static load the tyre is designed for (N).
    pub nominal_load: f32,
    pub rolling_resistance: f32,
}

impl Default for TyreDesc {
    fn default() -> Self {
        Self {
            mu: 1.0,
            slide_ratio: 0.76,
            peak_slip_ratio: 0.1,
            peak_slip_angle: 0.14,
            load_sensitivity: 0.12,
            nominal_load: 4000.0,
            rolling_resistance: 0.012,
        }
    }
}

/// Grip multiplier for a surface (dry).
pub fn surface_grip(s: Surface) -> f32 {
    match s {
        Surface::Asphalt => 1.0,
        Surface::Concrete => 0.95,
        Surface::Cobblestone => 0.82,
        Surface::Grass => 0.5,
        Surface::Dirt => 0.62,
        Surface::Gravel => 0.58,
        Surface::Metal => 0.7,
        Surface::Wood => 0.65,
        Surface::Glass => 0.55,
        Surface::Rubber => 1.05,
        Surface::Water => 0.35,
    }
}

/// How much a surface loses when wet (multiplier at wetness 1).
pub fn wet_factor(s: Surface) -> f32 {
    match s {
        Surface::Asphalt | Surface::Concrete => 0.72,
        Surface::Cobblestone => 0.6,
        Surface::Grass => 0.7,
        Surface::Dirt => 0.6,
        Surface::Metal | Surface::Wood | Surface::Glass => 0.55,
        _ => 0.8,
    }
}

/// Extra rolling drag on soft surfaces (fraction of load).
pub fn surface_drag(s: Surface) -> f32 {
    match s {
        Surface::Grass => 0.05,
        Surface::Dirt => 0.04,
        Surface::Gravel => 0.07,
        Surface::Water => 0.1,
        _ => 0.0,
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TyreForce {
    /// Longitudinal force along the wheel heading (N).
    pub fx: f32,
    /// Lateral force along the wheel's right axis (N).
    pub fy: f32,
    /// Combined normalised slip (1 = at the grip peak, >1 sliding).
    pub slip: f32,
}

/// Forces for slip ratio `kappa`, lateral slip `tan_alpha` (positive = sliding toward +right),
/// normal load `load` and an overall grip multiplier.
pub fn tyre_force(t: &TyreDesc, kappa: f32, tan_alpha: f32, load: f32, grip: f32) -> TyreForce {
    if load <= 0.0 {
        return TyreForce::default();
    }
    let sx = kappa / t.peak_slip_ratio;
    let sy = tan_alpha / t.peak_slip_angle.tan();
    let s = (sx * sx + sy * sy).sqrt();
    if s < 1e-6 {
        return TyreForce::default();
    }
    let c = 2.0 - 2.0 * t.slide_ratio.clamp(0.3, 0.99).asin() / std::f32::consts::PI;
    let b = (std::f32::consts::FRAC_PI_2 / c).tan();
    let shape = (c * (b * s).atan()).sin();
    let load_ratio = load / t.nominal_load.max(1.0);
    let mu = t.mu * grip * (1.0 - t.load_sensitivity * (load_ratio - 1.0)).clamp(0.5, 1.3);
    let f = mu * load * shape;
    TyreForce {
        fx: f * sx / s,
        fy: -f * sy / s,
        slip: s,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peak_and_slide() {
        let t = TyreDesc {
            load_sensitivity: 0.0,
            ..Default::default()
        };
        let peak = tyre_force(&t, t.peak_slip_ratio, 0.0, 4000.0, 1.0);
        assert!((peak.fx - 4000.0).abs() < 5.0, "{}", peak.fx);
        let slide = tyre_force(&t, 2.0, 0.0, 4000.0, 1.0);
        assert!((slide.fx / 4000.0 - 0.76).abs() < 0.05, "{}", slide.fx);
        let lat = tyre_force(&t, 0.0, t.peak_slip_angle.tan(), 4000.0, 1.0);
        assert!((lat.fy + 4000.0).abs() < 5.0);
        // Combined: never exceeds the friction circle.
        let comb = tyre_force(&t, 0.1, 0.14f32.tan(), 4000.0, 1.0);
        assert!((comb.fx * comb.fx + comb.fy * comb.fy).sqrt() <= 4000.0 + 1.0);
    }
}
