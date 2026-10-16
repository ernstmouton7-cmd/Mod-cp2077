import { mod } from "../math/scalar";

export type Indicator = "off" | "left" | "right";
/** Rotary headlight switch. */
export type HeadlightSwitch = "off" | "auto" | "low";
export type WiperMode = "off" | "intermittent" | "low" | "high";
export type SirenMode = "off" | "wail" | "yelp";

/**
 * Lamps, signals and other driver-operated equipment: indicators with
 * steering-column auto-cancel, hazards, low/high beam, flash-to-pass, brake
 * and reverse lamps, horn, wipers and emergency lightbar/siren.
 */
export class VehicleLights {
  headlightSwitch: HeadlightSwitch = "auto";
  highBeam = false;
  /** Momentary flash-to-pass (held). */
  flash = false;
  indicator: Indicator = "off";
  hazard = false;
  brake = 0;
  reverse = false;
  horn = false;
  /** Duration of the current horn press (s); 0 when not honking. */
  hornTime = 0;
  /** Count of horn presses (for "honk honk" patterns). */
  hornPresses = 0;
  wipers: WiperMode = "off";
  /** Wiper arm position 0..1..0 (sweep). */
  wiperPos = 0;
  private wiperTimer = 0;
  private wiperDir = 0;
  siren: SirenMode = "off";
  lightbar = false;
  /** Lamps broken by collision damage. */
  leftHeadlightBroken = false;
  rightHeadlightBroken = false;
  /** Global clock for blink phases. */
  private clock = 0;
  /** Steering-column auto-cancel state. */
  private cancelArmed = false;
  /** Is it dark enough for "auto" lights to be on (set by environment). */
  dark = false;
  /** Seconds the current indicator has been on. */
  indicatorTime = 0;
  /** Phase offset so AI cars don't blink in sync. */
  readonly phase: number;

  constructor(seed: number) {
    this.phase = mod(seed * 0.618, 1) * 0.8;
  }

  /** Relay state of the turn signals (~85 blinks/min). */
  blinkOn(): boolean {
    return mod(this.clock + this.phase, 0.7) < 0.38;
  }

  leftLampOn(): boolean {
    return (this.hazard || this.indicator === "left") && this.blinkOn();
  }

  rightLampOn(): boolean {
    return (this.hazard || this.indicator === "right") && this.blinkOn();
  }

  lowBeamOn(): boolean {
    if (this.headlightSwitch === "low") return true;
    return this.headlightSwitch === "auto" && this.dark;
  }

  /** High beam lit (switched or flashing). Flash works even with lights off. */
  highBeamOn(): boolean {
    return this.flash || (this.highBeam && this.lowBeamOn());
  }

  anyHeadlight(): boolean {
    return this.lowBeamOn() || this.highBeamOn();
  }

  /** Lightbar colour phase for emergency vehicles: 0 = red/left, 1 = blue/right. */
  lightbarPhase(): number {
    return mod(this.clock * 3, 1) < 0.5 ? 0 : 1;
  }

  setIndicator(dir: Indicator): void {
    if (dir === this.indicator) return;
    this.indicator = dir;
    this.cancelArmed = false;
    this.indicatorTime = 0;
  }

  /** Toggle like a real stalk: pressing the active side cancels. */
  toggleIndicator(dir: "left" | "right"): void {
    this.setIndicator(this.indicator === dir ? "off" : dir);
  }

  cycleHeadlights(): void {
    this.headlightSwitch = this.headlightSwitch === "off" ? "auto" : this.headlightSwitch === "auto" ? "low" : "off";
  }

  cycleWipers(): void {
    this.wipers = this.wipers === "off" ? "intermittent" : this.wipers === "intermittent" ? "low" : this.wipers === "low" ? "high" : "off";
  }

  cycleSiren(): void {
    this.siren = this.siren === "off" ? "wail" : this.siren === "wail" ? "yelp" : "off";
    this.lightbar = this.siren !== "off" || this.lightbar;
    if (this.siren === "off") this.lightbar = false;
  }

  /**
   * @param steer current front wheel angle (rad, + = left), used for auto-cancel.
   */
  update(dt: number, steer: number): void {
    this.clock += dt;
    if (this.horn) this.hornTime += dt;
    else this.hornTime = 0;
    if (this.indicator !== "off") {
      this.indicatorTime += dt;
      // Arm once the wheel is turned meaningfully in the signalled direction,
      // cancel when it returns toward centre.
      const toward = this.indicator === "left" ? steer : -steer;
      if (toward > 0.18) this.cancelArmed = true;
      else if (this.cancelArmed && toward < 0.04) {
        this.indicator = "off";
        this.cancelArmed = false;
        this.indicatorTime = 0;
      }
    }
    // Wipers: a sweep takes ~1.1 s (low), 0.7 s (high), intermittent pauses 3 s.
    const sweep = this.wipers === "high" ? 0.7 : 1.1;
    if (this.wipers === "off" && this.wiperDir === 0) {
      this.wiperPos = 0;
      return;
    }
    if (this.wiperDir === 0) {
      this.wiperTimer -= dt;
      if (this.wiperTimer <= 0 && this.wipers !== "off") this.wiperDir = 1;
    } else {
      this.wiperPos += ((this.wiperDir * 2) / sweep) * dt;
      if (this.wiperPos >= 1) {
        this.wiperPos = 1;
        this.wiperDir = -1;
      } else if (this.wiperPos <= 0) {
        this.wiperPos = 0;
        this.wiperDir = 0;
        this.wiperTimer = this.wipers === "intermittent" ? 3 : 0;
      }
    }
  }
}
