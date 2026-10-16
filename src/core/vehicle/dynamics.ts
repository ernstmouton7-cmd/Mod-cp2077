import { clamp, G, lerp } from "../math/scalar";
import { VehicleSpec } from "./specs";

export interface DriveInputs {
  /** 0..1 */
  throttle: number;
  /** 0..1 */
  brake: number;
  /** -1 (full left) .. 1 (full right), already shaped by the driver layer. */
  steer: number;
  /** 0..1 */
  handbrake: number;
}

export interface Assists {
  abs: boolean;
  tcs: boolean;
  esc: boolean;
}

const RHO = 1.2;

/**
 * Planar rigid-body vehicle model ("bicycle" model with longitudinal load
 * transfer). Each axle is a lumped tyre with a magic-formula lateral curve and
 * a traction-limited longitudinal force, combined through a friction circle.
 * Wheel lock (no ABS) and wheelspin (no TCS) degrade lateral grip, so drifts,
 * burnouts and lock-up understeer emerge naturally.
 */
export class VehicleDynamics {
  x = 0;
  y = 0;
  yaw = 0;
  /** Body-frame velocity: forward (u) and leftward (v), m/s. */
  u = 0;
  v = 0;
  /** Yaw rate, rad/s (positive = turning left). */
  r = 0;
  /** Actual front wheel angle, rad (positive = left). */
  steerAngle = 0;
  /** Longitudinal / lateral acceleration, smoothed (m/s^2), for load transfer & visuals. */
  ax = 0;
  ay = 0;
  /** Wheel angular speeds (rad/s) used for rendering and engine rpm. */
  omegaF = 0;
  omegaR = 0;
  /** Extra slip velocity of the driven wheels during wheelspin (m/s). */
  private spinSlip = 0;
  /** Status for HUD and audio. */
  absActive = false;
  tcsActive = false;
  escActive = false;
  frontLocked = false;
  rearLocked = false;
  wheelspin = false;
  /** 0..1 how hard the tyres are sliding (skid sound & marks). */
  skid = 0;
  slipAngleF = 0;
  slipAngleR = 0;

  readonly mass: number;
  readonly iz: number;
  /** CG to front axle / rear axle. */
  readonly a: number;
  readonly b: number;
  readonly wheelbase: number;
  /** Grip multiplier from surface & weather (1 = dry asphalt). */
  surfaceGrip = 1;
  /** Extra rolling resistance (off-road). */
  surfaceDrag = 0;
  /** 0..1, reduces grip on damaged corners. */
  tireHealth = 1;

  constructor(readonly spec: VehicleSpec) {
    this.mass = spec.mass;
    this.iz = (spec.mass * (spec.length * spec.length + spec.width * spec.width)) / 12 * 0.85;
    this.wheelbase = spec.wheelbase;
    this.a = spec.wheelbase * (1 - spec.frontWeight);
    this.b = spec.wheelbase * spec.frontWeight;
  }

  absSpeed(): number {
    return Math.sqrt(this.u * this.u + this.v * this.v);
  }

  vx(): number {
    return this.u * Math.cos(this.yaw) - this.v * Math.sin(this.yaw);
  }

  vy(): number {
    return this.u * Math.sin(this.yaw) + this.v * Math.cos(this.yaw);
  }

  setWorldVelocity(vx: number, vy: number): void {
    const c = Math.cos(this.yaw);
    const s = Math.sin(this.yaw);
    this.u = vx * c + vy * s;
    this.v = -vx * s + vy * c;
  }

  /** Place the body with a forward speed and no slip. */
  reset(x: number, y: number, yaw: number, forwardSpeed: number): void {
    this.x = x;
    this.y = y;
    this.yaw = yaw;
    this.u = forwardSpeed;
    this.v = 0;
    this.r = 0;
    this.ax = 0;
    this.ay = 0;
    this.spinSlip = 0;
    this.omegaF = forwardSpeed / this.spec.wheelRadius;
    this.omegaR = this.omegaF;
  }

  /** Angular speed of the driven wheels (for the powertrain). */
  drivenOmega(): number {
    const d = this.spec.drive;
    if (d === "fwd") return this.omegaF;
    if (d === "rwd") return this.omegaR;
    return (this.omegaF + this.omegaR) / 2;
  }

  /** Apply an impulse (N·s) at world point (px, py). */
  applyImpulse(jx: number, jy: number, px: number, py: number): void {
    const vx = this.vx() + jx / this.mass;
    const vy = this.vy() + jy / this.mass;
    const rx = px - this.x;
    const ry = py - this.y;
    this.r += (rx * jy - ry * jx) / this.iz;
    this.setWorldVelocity(vx, vy);
  }

  /** Tyre lateral force curve (magic formula), returns force opposing slip. */
  private lateral(alpha: number, fz: number, mu: number): number {
    const B = this.spec.tireB;
    const C = this.spec.tireC;
    return -mu * fz * Math.sin(C * Math.atan(B * alpha));
  }

  /**
   * Advance by dt (internally sub-stepped).
   * @param driveTorque  torque at the driven wheels (N·m) from the powertrain
   * @param steerTarget  desired wheel angle (rad, positive = left)
   */
  step(dt: number, inputs: DriveInputs, driveTorque: number, steerTarget: number, assists: Assists): void {
    const sub = dt > 1 / 90 ? Math.ceil(dt / (1 / 120)) : 1;
    const h = dt / sub;
    for (let i = 0; i < sub; i++) this.substep(h, inputs, driveTorque, steerTarget, assists);
  }

  private substep(h: number, inp: DriveInputs, driveTorque: number, steerTarget: number, assists: Assists): void {
    const spec = this.spec;
    const m = this.mass;
    const L = this.wheelbase;
    const R = spec.wheelRadius;
    const mu = spec.grip * this.surfaceGrip * (0.6 + 0.4 * this.tireHealth);
    const muK = mu * 0.78;

    // Steering actuator (rate limited).
    const steerRate = 3.2;
    const ds = clamp(steerTarget - this.steerAngle, -steerRate * h, steerRate * h);
    this.steerAngle += ds;
    const delta = this.steerAngle;
    const cd = Math.cos(delta);
    const sd = Math.sin(delta);

    // Vertical loads with longitudinal weight transfer.
    const wt = (m * this.ax * spec.cgHeight) / L;
    const fzF = clamp((m * G * this.b) / L - wt, 0.15 * m * G, 0.85 * m * G);
    const fzR = clamp((m * G * this.a) / L + wt, 0.15 * m * G, 0.85 * m * G);

    const u = this.u;
    const v = this.v;
    const r = this.r;
    // Axle velocities in body frame, then front in wheel frame.
    const vyF = v + this.a * r;
    const vyR = v - this.b * r;
    const longF = u * cd + vyF * sd;
    const latF = -u * sd + vyF * cd;
    const longR = u;
    const latR = vyR;
    const vMin = 1.5;
    const alphaF = Math.atan2(latF, Math.max(Math.abs(longF), vMin));
    const alphaR = Math.atan2(latR, Math.max(Math.abs(longR), vMin));
    this.slipAngleF = alphaF;
    this.slipAngleR = alphaR;

    // Drive torque split.
    let tqF = 0;
    let tqR = 0;
    if (spec.drive === "fwd") tqF = driveTorque;
    else if (spec.drive === "rwd") tqR = driveTorque;
    else {
      tqF = driveTorque * 0.4;
      tqR = driveTorque * 0.6;
    }
    // Brakes.
    const brakeT = inp.brake * spec.brakeTorque;
    const bF = (brakeT * spec.brakeBias) / R;
    let bR = (brakeT * (1 - spec.brakeBias)) / R;
    const hb = inp.handbrake * spec.brakeTorque * 0.9;
    bR += hb / R;
    const rollF = spec.rollResist * fzF * (1 + this.surfaceDrag);
    const rollR = spec.rollResist * fzR * (1 + this.surfaceDrag);

    const res = this.axle(tqF / R, bF, rollF, longF, latF, alphaF, fzF, mu, muK, assists, false, inp.handbrake);
    const fxF = res.fx;
    const fyF = res.fy;
    this.frontLocked = res.locked;
    let spinF = res.spin;
    const absF = res.abs;
    const tcsF = res.tcs;
    const resR = this.axle(tqR / R, bR, rollR, longR, latR, alphaR, fzR, mu, muK, assists, true, inp.handbrake);
    const fxR = resR.fx;
    const fyR = resR.fy;
    this.rearLocked = resR.locked;
    spinF = spinF || resR.spin;
    this.absActive = absF || resR.abs;
    this.tcsActive = tcsF || resR.tcs;
    this.wheelspin = spinF;

    // Body-frame forces.
    const fxFb = fxF * cd - fyF * sd;
    const fyFb = fxF * sd + fyF * cd;
    const drag = 0.5 * RHO * spec.dragArea * u * Math.abs(u);
    const latDrag = 0.5 * RHO * spec.dragArea * 3 * v * Math.abs(v);
    let fx = fxFb + fxR - drag;
    let fy = fyFb + fyR - latDrag;
    let mz = this.a * fyFb - this.b * fyR;

    // Electronic stability control: limit yaw rate error with a corrective moment.
    this.escActive = false;
    if (assists.esc && Math.abs(u) > 6 && inp.handbrake < 0.1) {
      const rDes = (u * Math.tan(delta)) / (L * (1 + 0.0025 * u * u));
      const rMax = (mu * G * 0.9) / Math.abs(u);
      const target = clamp(rDes, -rMax, rMax);
      const err = r - target;
      if (Math.abs(err) > 0.12) {
        mz -= clamp(err * this.iz * 4, -m * G * 1.2, m * G * 1.2);
        // Braking an individual wheel also scrubs speed.
        fx -= Math.sign(u) * Math.min(Math.abs(err), 1) * m * 1.5;
        this.escActive = true;
      }
    }

    // Integrate (semi-implicit Euler in the body frame).
    let du = fx / m + v * r;
    let dv = fy / m - u * r;
    const dr = mz / this.iz;
    let uN = u + du * h;
    let vN = v + dv * h;
    let rN = r + dr * h;

    // Brakes and rolling resistance cannot push the car backwards: hold at rest.
    const driveF = Math.abs((tqF + tqR) / R);
    const resist = bF + bR + rollF + rollR;
    if ((u > 0 && uN < 0) || (u < 0 && uN > 0)) {
      if (driveF < resist || Math.sign(driveTorque) === Math.sign(u)) uN = 0;
    }
    if (Math.abs(uN) < 0.05 && driveF <= resist + 1) {
      uN = 0;
      if (Math.abs(vN) < 0.3) vN *= 0.5;
    }

    // Low-speed blend toward kinematic rolling (no tyre slip when crawling).
    const spd = Math.abs(uN);
    const k = clamp(1 - spd / 3, 0, 1);
    if (k > 0) {
      const rKin = (uN * Math.tan(delta)) / L;
      const vKin = rKin * this.b;
      const blend = Math.min(1, k * 12 * h);
      rN = lerp(rN, rKin, blend);
      vN = lerp(vN, vKin, blend);
    }
    du = (uN - u) / h;
    dv = (vN - v) / h;

    this.u = uN;
    this.v = vN;
    this.r = rN;
    const cy = Math.cos(this.yaw);
    const sy = Math.sin(this.yaw);
    this.x += (uN * cy - vN * sy) * h;
    this.y += (uN * sy + vN * cy) * h;
    this.yaw += rN * h;
    // Smoothed accelerations (for load transfer stability and body sway visuals).
    const accLong = du - vN * rN;
    const accLat = dv + uN * rN;
    const sm = Math.min(1, 10 * h);
    this.ax += (clamp(accLong, -15, 15) - this.ax) * sm;
    this.ay += (clamp(accLat, -15, 15) - this.ay) * sm;

    // Wheel speeds.
    if (spinF) this.spinSlip = Math.min(this.spinSlip + 40 * h, 25);
    else this.spinSlip = Math.max(0, this.spinSlip - 60 * h);
    const sign = driveTorque >= 0 ? 1 : -1;
    const freeF = longF / R;
    const freeR = longR / R;
    const driven = spec.drive;
    this.omegaF = this.frontLocked ? 0 : freeF + (driven !== "rwd" ? (sign * this.spinSlip) / R : 0);
    this.omegaR = this.rearLocked ? 0 : freeR + (driven !== "fwd" ? (sign * this.spinSlip) / R : 0);

    // Skid intensity: lateral slip past the tyre peak, or locked/spinning wheels.
    const peak = 0.12;
    const latSkid = Math.max(Math.abs(alphaF), Math.abs(alphaR)) - peak;
    let skid = clamp(latSkid * 5, 0, 1) * clamp(this.absSpeed() / 6, 0, 1);
    if ((this.frontLocked || this.rearLocked) && this.absSpeed() > 1) skid = Math.max(skid, 0.9);
    if (spinF) skid = Math.max(skid, 0.7);
    this.skid = skid;
  }

  /** Forces of one axle in its wheel frame. */
  private axle(
    driveF: number,
    brakeF: number,
    rollF: number,
    vLong: number,
    vLat: number,
    alpha: number,
    fz: number,
    mu: number,
    muK: number,
    assists: Assists,
    rear: boolean,
    handbrake: number,
  ): { fx: number; fy: number; locked: boolean; spin: boolean; abs: boolean; tcs: boolean } {
    const fMax = mu * fz;
    const dir = vLong > 0.2 ? 1 : vLong < -0.2 ? -1 : 0;
    let fyPure = this.lateral(alpha, fz, mu);
    // At a standstill, lateral force only resists sliding (avoid jitter).
    if (Math.abs(vLat) < 0.05 && Math.abs(vLong) < 0.2) fyPure = 0;
    let fx = driveF;
    let locked = false;
    let spin = false;
    let abs = false;
    let tcs = false;
    // Brake & rolling resistance oppose motion.
    const resist = brakeF + rollF;
    if (dir !== 0) fx -= dir * resist;
    else {
      // At rest: resistance cancels drive up to its magnitude.
      if (Math.abs(fx) <= resist) fx = 0;
      else fx -= Math.sign(fx) * resist;
    }
    const brakeDominant = brakeF > Math.abs(driveF) && dir !== 0;
    if (Math.abs(fx) > fMax) {
      if (brakeDominant) {
        const handbrakeLock = rear && handbrake > 0.3;
        if (assists.abs && !handbrakeLock) {
          fx = Math.sign(fx) * fMax * 0.97;
          abs = true;
        } else {
          locked = true;
        }
      } else if (assists.tcs) {
        fx = Math.sign(fx) * fMax * 0.95;
        tcs = true;
      } else {
        spin = true;
      }
    }
    if (locked) {
      // Sliding tyre: friction opposes the full sliding velocity.
      const sp = Math.sqrt(vLong * vLong + vLat * vLat);
      if (sp > 0.01) return { fx: (-muK * fz * vLong) / sp, fy: (-muK * fz * vLat) / sp, locked, spin, abs, tcs };
      return { fx: 0, fy: 0, locked, spin, abs, tcs };
    }
    if (spin) {
      fx = Math.sign(fx) * muK * fz;
      const fyMax = Math.sqrt(Math.max(0, fMax * fMax - fx * fx)) + fMax * 0.15;
      return { fx, fy: clamp(fyPure * 0.55, -fyMax, fyMax), locked, spin, abs, tcs };
    }
    // Friction circle: longitudinal has priority, lateral gets what remains.
    const fyMax = Math.sqrt(Math.max(0, fMax * fMax - fx * fx));
    const fy = clamp(fyPure, -fyMax, fyMax);
    return { fx, fy, locked, spin, abs, tcs };
  }
}
