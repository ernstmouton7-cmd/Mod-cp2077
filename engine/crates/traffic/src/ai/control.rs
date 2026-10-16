//! Low-level controller for physics-mode AI.

use crate::math::scalar::{clamp, wrap_angle};
use crate::vehicle::Vehicle;

/// Pure-pursuit steering toward a target point plus a proportional speed
/// controller (with reversing when the target is close behind). Writes the
/// vehicle's pedal inputs, steer target and gear.
pub fn drive_toward(v: &mut Vehicle, tx: f64, ty: f64, target_speed: f64, allow_reverse: bool) {
    let d = &v.dynamics;
    let dx = tx - d.x;
    let dy = ty - d.y;
    let c = d.yaw.cos();
    let s = d.yaw.sin();
    let lx = dx * c + dy * s;
    let ly = -dx * s + dy * c;
    let dist = (dx * dx + dy * dy).sqrt();
    let alpha = ly.atan2(lx);
    let reverse = allow_reverse && lx < -1.0 && dist < 12.0 && alpha.abs() > 2.2;
    let l = v.spec.wheelbase;
    let ld = dist.max(4.0);
    let mut steer = ((2.0 * l * alpha.sin()) / ld).atan();
    if reverse {
        steer = -((2.0 * l * wrap_angle(alpha + std::f64::consts::PI).sin()) / ld).atan();
    }
    v.steer_target = clamp(steer, -v.spec.max_steer, v.spec.max_steer);
    let sharp = if reverse {
        wrap_angle(alpha + std::f64::consts::PI)
    } else {
        alpha
    }
    .abs();
    let mut want = target_speed * clamp(1.2 - sharp / 1.6, 0.25, 1.0);
    if reverse {
        want = -target_speed.min(4.0);
    }
    let err = want - v.dynamics.u;
    if reverse {
        if v.pt.gear != -1 {
            v.pt.shift_to(-1);
        }
        v.inputs.throttle = clamp(-err * 0.35, 0.0, 1.0);
        v.inputs.brake = clamp(err * 0.5, 0.0, 1.0);
    } else {
        if v.pt.gear <= 0 {
            v.pt.shift_to(1);
        }
        v.inputs.throttle = clamp(err * 0.35, 0.0, 1.0);
        v.inputs.brake = clamp(-err * 0.45, 0.0, 1.0);
    }
    v.inputs.handbrake = 0.0;
}

/// Brake to a standstill (handbrake once stopped).
pub fn brake_to_stop(v: &mut Vehicle, hard: bool) {
    v.inputs.throttle = 0.0;
    v.inputs.brake = if hard { 1.0 } else { 0.6 };
    v.inputs.handbrake = if v.dynamics.u.abs() < 0.3 { 1.0 } else { 0.0 };
    v.steer_target = 0.0;
}
