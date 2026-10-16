//! Mapping between the traffic simulation's 2D frame and the 3D world.
//!
//! Traffic: x east, y north, heading CCW from +x (radians).
//! World: X east, Y up, Z south (so north is -Z); vehicles face -Z in their own frame.

use glam::{Quat, Vec2, Vec3};
use std::f32::consts::FRAC_PI_2;

#[inline]
pub fn to_world(x: f64, y: f64, height: f32) -> Vec3 {
    Vec3::new(x as f32, height, -(y as f32))
}

#[inline]
pub fn to_world2(x: f64, y: f64) -> Vec2 {
    Vec2::new(x as f32, -(y as f32))
}

#[inline]
pub fn to_sim(p: Vec3) -> (f64, f64) {
    (p.x as f64, -(p.z as f64))
}

/// Body rotation for a traffic heading.
#[inline]
pub fn rotation_for_heading(heading: f64) -> Quat {
    Quat::from_rotation_y(heading as f32 - FRAC_PI_2)
}

/// Traffic heading of a body rotation (from its forward axis, ignoring pitch/roll).
#[inline]
pub fn heading_of(q: Quat) -> f32 {
    let f = q * -Vec3::Z;
    (-f.z).atan2(f.x)
}

/// World velocity -> traffic (vx, vy).
#[inline]
pub fn sim_velocity(v: Vec3) -> (f32, f32) {
    (v.x, -v.z)
}

/// Traffic (vx, vy) -> world velocity.
#[inline]
pub fn world_velocity(vx: f64, vy: f64) -> Vec3 {
    Vec3::new(vx as f32, 0.0, -(vy as f32))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heading_roundtrip() {
        for h in [0.0, 0.5, 1.57, 3.0, -2.0] {
            let q = rotation_for_heading(h);
            let back = heading_of(q) as f64;
            let d = (back - h + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU)
                - std::f64::consts::PI;
            assert!(d.abs() < 1e-4, "{h} -> {back}");
        }
        // Heading 0 (east) faces world +X.
        let f = rotation_for_heading(0.0) * -Vec3::Z;
        assert!((f - Vec3::X).length() < 1e-5);
        // Heading 90° (north) faces world -Z.
        let f = rotation_for_heading(std::f64::consts::FRAC_PI_2) * -Vec3::Z;
        assert!((f + Vec3::Z).length() < 1e-5);
    }
}
