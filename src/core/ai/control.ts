import { clamp, wrapAngle } from "../math/scalar";
import { Vehicle } from "../vehicle/vehicle";

/**
 * Low-level controller for physics-mode AI: pure-pursuit steering toward a
 * target point plus a proportional speed controller. Writes vehicle.inputs.
 */
export function driveToward(v: Vehicle, tx: number, ty: number, targetSpeed: number, allowReverse: boolean): void {
  const d = v.dyn;
  const dx = tx - d.x;
  const dy = ty - d.y;
  const c = Math.cos(d.yaw);
  const s = Math.sin(d.yaw);
  const lx = dx * c + dy * s;
  const ly = -dx * s + dy * c;
  const dist = Math.sqrt(dx * dx + dy * dy);
  const alpha = Math.atan2(ly, lx);
  const reverse = allowReverse && lx < -1 && dist < 12 && Math.abs(alpha) > 2.2;
  const L = v.spec.wheelbase;
  const ld = Math.max(dist, 4);
  let steer = Math.atan((2 * L * Math.sin(alpha)) / ld);
  if (reverse) steer = -Math.atan((2 * L * Math.sin(wrapAngle(alpha + Math.PI))) / ld);
  v.steerTarget = clamp(steer, -v.spec.maxSteer, v.spec.maxSteer);
  // Slow down for sharp heading errors.
  const sharp = Math.abs(reverse ? wrapAngle(alpha + Math.PI) : alpha);
  let want = targetSpeed * clamp(1.2 - sharp / 1.6, 0.25, 1);
  if (reverse) want = -Math.min(4, targetSpeed);
  const err = want - d.u;
  if (reverse) {
    if (v.pt.gear !== -1) v.pt.shiftTo(-1);
    v.inputs.throttle = clamp(-err * 0.35, 0, 1);
    v.inputs.brake = clamp(err * 0.5, 0, 1);
  } else {
    if (v.pt.gear <= 0) v.pt.shiftTo(1);
    v.inputs.throttle = clamp(err * 0.35, 0, 1);
    v.inputs.brake = clamp(-err * 0.45, 0, 1);
  }
  v.inputs.handbrake = 0;
}

export function brakeToStop(v: Vehicle, hard: boolean): void {
  v.inputs.throttle = 0;
  v.inputs.brake = hard ? 1 : 0.6;
  v.inputs.handbrake = Math.abs(v.dyn.u) < 0.3 ? 1 : 0;
  v.steerTarget = 0;
}
