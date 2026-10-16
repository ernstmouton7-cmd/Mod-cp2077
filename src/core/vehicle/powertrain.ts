import { clamp, curve } from "../math/scalar";
import { VehicleSpec } from "./specs";

/** Gear index: -1 = reverse, 0 = neutral, 1..n forward. */
export type Gear = number;

export type TransmissionMode = "automatic" | "manual";

const RPM_PER_RADS = 60 / (2 * Math.PI);

/**
 * Engine, clutch and gearbox. Produces drive torque at the wheels from pedal
 * inputs and the current wheel speed, and keeps engine rpm for HUD/audio.
 */
export class Powertrain {
  rpm: number;
  gear: Gear = 1;
  mode: TransmissionMode = "automatic";
  running = true;
  /** Engine stalled (manual: dropped the clutch at low rpm). */
  stalled = false;
  /** 0 = clutch pedal released (engaged), 1 = fully pressed. */
  clutchPedal = 0;
  fuel: number;
  /** 0..1 engine health; reduces torque, can kill the engine. */
  health = 1;
  /** Seconds left of an in-progress gear change (torque is cut). */
  private shiftTimer = 0;
  private pendingGear: Gear = 1;
  /** Limiter cuts fuel briefly when hitting redline. */
  limiterActive = false;
  private limiterTimer = 0;
  /** Last produced engine torque (N·m) and load 0..1 for audio. */
  engineTorque = 0;
  load = 0;
  /** Seconds the starter has been cranking (ignition sequence). */
  cranking = 0;
  /** Accumulated fuel used (L) and distance-free consumption info. */
  fuelUsed = 0;
  /** Speed limiter target (m/s); 0 = off. */
  limiter = 0;

  constructor(readonly spec: VehicleSpec) {
    this.rpm = spec.idleRpm;
    this.fuel = spec.fuelCapacity * 0.75;
  }

  get maxGear(): number {
    return this.spec.gears.length;
  }

  ratio(g: Gear): number {
    if (g === 0) return 0;
    if (g < 0) return -this.spec.reverseRatio;
    return this.spec.gears[g - 1];
  }

  get shifting(): boolean {
    return this.shiftTimer > 0;
  }

  /** Request a gear (manual shifting or automatic logic). */
  shiftTo(g: Gear): boolean {
    const target = clamp(Math.floor(g), -1, this.maxGear);
    if (target === this.gear && this.shiftTimer <= 0) return false;
    this.pendingGear = target;
    this.shiftTimer = this.spec.shiftTime;
    return true;
  }

  shiftUp(): boolean {
    const base = this.shiftTimer > 0 ? this.pendingGear : this.gear;
    if (base >= this.maxGear) return false;
    return this.shiftTo(base + 1);
  }

  shiftDown(): boolean {
    const base = this.shiftTimer > 0 ? this.pendingGear : this.gear;
    if (base <= -1) return false;
    return this.shiftTo(base - 1);
  }

  /** Displayed gear text: R, N, 1..6 (P when parked and stopped is handled by HUD). */
  gearLabel(): string {
    const g = this.shiftTimer > 0 ? this.pendingGear : this.gear;
    if (g < 0) return "R";
    if (g === 0) return "N";
    return `${g}`;
  }

  start(): void {
    if (this.health <= 0.02 || this.fuel <= 0) {
      this.cranking = 1.2;
      return;
    }
    if (!this.running) this.cranking = 0.8;
  }

  stop(): void {
    this.running = false;
    this.cranking = 0;
  }

  torqueAt(rpm: number, throttle: number): number {
    const full = curve(this.spec.torqueCurve, rpm);
    const healthFactor = 0.35 + 0.65 * clamp(this.health, 0, 1);
    // Engine braking grows with rpm above idle (none at idle).
    const drag = this.spec.engineBrake * clamp((rpm - this.spec.idleRpm * 1.05) / (this.spec.redline - this.spec.idleRpm), 0, 1.2);
    return throttle * full * healthFactor - (1 - throttle) * drag;
  }

  /**
   * Advance the powertrain and return drive torque at the driven wheels (N·m).
   * @param wheelOmega  angular speed of driven wheels (rad/s, signed)
   * @param throttle    0..1
   * @param speed       vehicle forward speed (m/s, signed)
   */
  update(dt: number, wheelOmega: number, throttle: number, speed: number): number {
    const spec = this.spec;
    if (this.cranking > 0) {
      this.cranking -= dt;
      // Starter motor cranking speed with a little wobble.
      this.rpm = 280 + 60 * Math.sin(this.cranking * 47);
      if (this.cranking <= 0) {
        if (this.health > 0.02 && this.fuel > 0) {
          this.running = true;
          this.stalled = false;
          this.rpm = spec.idleRpm * 1.4;
        }
      }
      return 0;
    }
    if (this.fuel <= 0 || this.health <= 0.02) this.running = false;

    if (this.shiftTimer > 0) {
      this.shiftTimer -= dt;
      if (this.shiftTimer <= 0) this.gear = this.pendingGear;
    }
    if (this.mode === "automatic" && this.running) this.autoShift(throttle, speed);

    // Speed limiter / cruise use this to cap throttle.
    if (this.limiter > 0 && Math.abs(speed) > this.limiter) throttle = 0;

    const ratio = this.ratio(this.gear) * spec.finalDrive;
    const engaged = this.gear !== 0 && this.shiftTimer <= 0;
    // Automatic: torque converter lets the engine slip at low speed.
    // Manual: clutch pedal controls slip.
    let couple = engaged ? 1 - this.clutchPedal : 0;
    if (!this.running) {
      this.rpm = Math.max(0, this.rpm - 3000 * dt);
      this.engineTorque = 0;
      this.load = 0;
      // A stopped engine still resists wheel motion through the gearbox.
      return engaged ? -Math.sign(wheelOmega) * Math.min(Math.abs(wheelOmega) * 4, 40) * Math.abs(ratio) * couple : 0;
    }
    const wheelRpm = Math.abs(wheelOmega * ratio) * RPM_PER_RADS;
    const freeRev = spec.idleRpm + throttle * (spec.redline - spec.idleRpm);
    let targetRpm: number;
    if (couple > 0.05) {
      if (this.mode === "automatic") {
        // Torque converter "stall speed": engine can run up to ~2500 rpm while wheels are slow.
        const stall = spec.idleRpm + throttle * Math.min(2400, (spec.redline - spec.idleRpm) * 0.45);
        targetRpm = Math.max(wheelRpm, stall, spec.idleRpm);
      } else {
        const slip = 1 - couple;
        targetRpm = wheelRpm * couple + freeRev * slip;
        if (wheelRpm < spec.idleRpm * 0.55 && couple > 0.9 && throttle < 0.6 && Math.abs(speed) < 2.5 && this.gear !== 0) {
          // Lugging below idle with the clutch out: stall.
          this.running = false;
          this.stalled = true;
          return 0;
        }
        targetRpm = Math.max(targetRpm, spec.idleRpm * (couple < 1 ? 1 : 0.6));
      }
    } else {
      targetRpm = freeRev;
      couple = 0;
    }
    // Engine inertia smoothing.
    const rate = couple > 0.05 ? 25 : 8;
    this.rpm += (targetRpm - this.rpm) * Math.min(1, rate * dt);

    // Rev limiter.
    if (this.rpm >= spec.redline) {
      this.limiterTimer = 0.08;
    }
    let effThrottle = throttle;
    if (this.limiterTimer > 0) {
      this.limiterTimer -= dt;
      effThrottle = 0;
      this.limiterActive = true;
    } else this.limiterActive = false;
    this.rpm = Math.min(this.rpm, spec.redline + 150);

    let tq = this.torqueAt(this.rpm, effThrottle);
    // Automatic creep: the converter pulls gently at idle in gear.
    if (this.mode === "automatic" && couple > 0 && effThrottle < 0.05 && wheelRpm < spec.idleRpm * 1.3) tq = Math.max(tq, curve(spec.torqueCurve, spec.idleRpm) * 0.12);
    // Torque multiplication of an automatic's converter at low speed.
    if (this.mode === "automatic" && couple > 0 && wheelRpm < this.rpm * 0.85) tq *= 1 + 0.6 * (1 - wheelRpm / Math.max(this.rpm, 1));
    this.engineTorque = tq;
    this.load = clamp(effThrottle, 0, 1);

    // Fuel: ~0.33 L per kWh at load plus idle flow.
    const powerKw = Math.max(0, (tq * this.rpm) / RPM_PER_RADS / 1000);
    const used = (powerKw * 0.33 + 0.8) * (dt / 3600);
    this.fuel = Math.max(0, this.fuel - used);
    this.fuelUsed += used;

    return tq * ratio * 0.9 * couple;
  }

  private autoShift(throttle: number, speed: number): void {
    if (this.shiftTimer > 0) return;
    const spec = this.spec;
    if (this.gear <= 0) return; // reverse/neutral selected explicitly
    const ratio = this.ratio(this.gear) * spec.finalDrive;
    const rpm = (Math.abs(speed) / spec.wheelRadius) * ratio * RPM_PER_RADS;
    const upAt = spec.idleRpm + (spec.redline - spec.idleRpm) * (0.35 + 0.6 * throttle);
    const downAt = spec.idleRpm * 1.25 + (spec.redline - spec.idleRpm) * 0.28 * throttle;
    if (rpm > upAt && this.gear < this.maxGear) {
      this.shiftTo(this.gear + 1);
    } else if (this.gear > 1) {
      const lowerRatio = this.ratio(this.gear - 1) * spec.finalDrive;
      const rpmLower = (Math.abs(speed) / spec.wheelRadius) * lowerRatio * RPM_PER_RADS;
      // Kickdown when the lower gear would still be below redline.
      if (rpm < downAt || (throttle > 0.9 && rpmLower < spec.redline * 0.82 && rpm < spec.redline * 0.55)) this.shiftTo(this.gear - 1);
    }
  }
}
