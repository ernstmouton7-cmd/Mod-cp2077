import { clamp, KMH } from "../math/scalar";
import { Vehicle } from "../vehicle/vehicle";
import type { SimContext } from "../world/context";
import { ActionId, ControlFrame } from "./actions";

export interface PlayerSettings {
  /** Manual gearbox: automatic clutch (no stalling) when the player doesn't use the clutch. */
  clutchAssist: boolean;
  /** Automatic gearbox: holding brake at a standstill selects reverse. */
  autoReverse: boolean;
  /** Steering sensitivity multiplier. */
  steerSensitivity: number;
}

export interface CruiseState {
  on: boolean;
  /** Target speed (m/s). */
  target: number;
  integral: number;
}

/** Actions the simulation does not handle itself (cameras, menus): forwarded to the host. */
export const HOST_ACTIONS: ActionId[] = ["camera", "look_back", "look_left", "look_right", "pause", "help", "map", "debug", "time_forward", "weather_cycle", "reset_vehicle"];

/**
 * Turns the player's ControlFrame into vehicle inputs and operates every
 * piece of equipment: gearbox (auto/manual, clutch, stalls), cruise control,
 * speed limiter, ignition, lights, indicators, hazards, wipers, horn,
 * seatbelt and driver assists.
 */
export class PlayerDriver {
  readonly settings: PlayerSettings = { clutchAssist: true, autoReverse: true, steerSensitivity: 1 };
  readonly cruise: CruiseState = { on: false, target: 0, integral: 0 };
  limiterOn = false;
  seatbelt = true;
  /** Automatic gearbox in Park (parking pawl holds the car). */
  parked = true;
  private reverseTimer = 0;
  private seatbeltChime = 0;
  /** Host-side actions pressed this step. */
  readonly hostActions: ActionId[] = [];
  /** Gesture requested this step (handled by the world's social system). */
  gesture: "wave" | "rude" | undefined = undefined;
  /** Notifications for the HUD (e.g. "Cruise set 60 km/h"). */
  readonly notes: string[] = [];
  private wasStalled = false;

  constructor(readonly v: Vehicle) {}

  /** Gear selector text for the HUD: P R N D (automatic) or R N 1-6 (manual). */
  gearText(): string {
    const pt = this.v.pt;
    if (pt.mode === "automatic") {
      if (this.parked) return "P";
      const l = pt.gearLabel();
      return l === "R" || l === "N" ? l : `D${l}`;
    }
    return pt.gearLabel();
  }

  private note(s: string): void {
    this.notes.push(s);
    if (this.notes.length > 8) this.notes.shift();
  }

  update(dt: number, f: ControlFrame, ctx: SimContext, speedLimit: number): void {
    const v = this.v;
    const pt = v.pt;
    const lights = v.lights;
    this.gesture = undefined;
    for (const id of f.pressed) this.onPress(id, ctx, speedLimit);

    // Horn & flash are "hold" actions.
    const hornHeld = f.held.indexOf("horn") >= 0;
    if (hornHeld && !lights.horn) {
      lights.hornPresses++;
      ctx.events.emit({ type: "horn", vehicle: v.id, long: false });
    }
    lights.horn = hornHeld;
    const flashHeld = f.held.indexOf("flash") >= 0;
    if (flashHeld && !lights.flash) ctx.events.emit({ type: "flash", vehicle: v.id });
    lights.flash = flashHeld;
    for (const r of f.released) {
      if (r.id === "horn" && r.held > 0.9) ctx.events.emit({ type: "horn", vehicle: v.id, long: true });
    }

    // --- Pedals and gear selection.
    let throttle = f.throttle;
    let brake = f.brake;
    const u = v.dyn.u;
    if (pt.mode === "automatic" && this.parked) {
      // P: pawl holds the car; throttle selects D, brake selects R.
      if (pt.gear !== 0) pt.shiftTo(0);
      if (throttle > 0.15 && pt.running) {
        this.parked = false;
        pt.shiftTo(1);
      } else if (brake > 0.15 && throttle < 0.05 && pt.running && this.settings.autoReverse) {
        this.reverseTimer += dt;
        if (this.reverseTimer > 0.4) {
          this.parked = false;
          pt.shiftTo(-1);
          this.reverseTimer = 0;
        }
      }
    } else if (pt.mode === "automatic") {
      // Engine off at a standstill: park.
      if (!pt.running && Math.abs(u) < 0.3) this.parked = true;
      if (this.settings.autoReverse) {
        if (pt.gear >= 1) {
          if (brake > 0.15 && throttle < 0.05 && u < 0.4) {
            this.reverseTimer += dt;
            if (this.reverseTimer > 0.3 && Math.abs(u) < 0.3) {
              pt.shiftTo(-1);
              this.reverseTimer = 0;
            }
          } else this.reverseTimer = 0;
        } else if (pt.gear === -1) {
          if (throttle > 0.15 && brake < 0.05 && u > -0.4) {
            this.reverseTimer += dt;
            if (this.reverseTimer > 0.2 && Math.abs(u) < 0.3) {
              pt.shiftTo(1);
              this.reverseTimer = 0;
            }
          } else this.reverseTimer = 0;
          // In reverse the brake key drives backwards, throttle brakes.
          const t = throttle;
          throttle = brake;
          brake = t;
        }
      }
      pt.clutchPedal = 0;
    } else {
      // Manual gearbox.
      if (this.settings.clutchAssist && f.clutch < 0.05) {
        const wheelRpm = Math.abs(v.dyn.drivenOmega() * pt.ratio(pt.gear) * v.spec.finalDrive) * 9.549;
        const idle = v.spec.idleRpm;
        if (pt.shifting) pt.clutchPedal = 1;
        else if (pt.gear !== 0 && wheelRpm < idle * 1.15) pt.clutchPedal = clamp(1 - throttle * 1.6 - (wheelRpm / idle) * 0.5, 0, 1);
        else pt.clutchPedal = 0;
      } else pt.clutchPedal = f.clutch;
      if (pt.gear === -1 && u > 1.5 && throttle > 0.2) {
        // Grinding into reverse while rolling forward: the box refuses.
        pt.shiftTo(0);
      }
    }
    if (pt.stalled && !this.wasStalled) {
      ctx.events.emit({ type: "stall", vehicle: v.id });
      this.note("Engine stalled — press Start (I)");
      this.cruise.on = false;
    }
    this.wasStalled = pt.stalled;

    // --- Cruise control & limiter.
    const cr = this.cruise;
    if (cr.on) {
      if (f.brake > 0.08 || f.clutch > 0.5 || f.handbrake > 0.5 || !pt.running) {
        cr.on = false;
        this.note("Cruise cancelled");
      } else if (pt.gear >= 1) {
        const err = cr.target - u;
        cr.integral = clamp(cr.integral + err * dt, -8, 8);
        const cmd = clamp(0.18 * err + 0.06 * cr.integral + 0.15, 0, 1);
        throttle = Math.max(throttle, cmd);
      }
    }
    pt.limiter = this.limiterOn ? Math.max(speedLimit, 5) : 0;

    v.inputs.throttle = pt.running || pt.cranking > 0 ? throttle : 0;
    v.inputs.brake = brake;
    v.inputs.handbrake = this.parked && pt.mode === "automatic" ? 1 : f.handbrake;
    // Steering: full lock at parking speeds, reduced at speed for stability.
    const speedFactor = 1 / (1 + Math.abs(u) / 35);
    v.steerTarget = -f.steer * v.spec.maxSteer * speedFactor * this.settings.steerSensitivity;

    // Lamps.
    lights.brake = v.inputs.brake > 0.05 ? 1 : 0;
    lights.reverse = pt.gear === -1;
    lights.dark = ctx.env.dark;

    // Seatbelt chime.
    if (!this.seatbelt && Math.abs(u) > 3) {
      this.seatbeltChime -= dt;
      if (this.seatbeltChime <= 0) {
        this.seatbeltChime = 6;
        this.note("Fasten seatbelt (B)");
      }
    }
  }

  private onPress(id: ActionId, ctx: SimContext, speedLimit: number): void {
    const v = this.v;
    const pt = v.pt;
    const lights = v.lights;
    switch (id) {
      case "shift_up":
        if (pt.mode === "manual" && pt.shiftUp()) ctx.events.emit({ type: "shift", vehicle: v.id, gear: pt.gearLabel() });
        else if (pt.mode === "automatic" && pt.gear <= 0) {
          this.parked = false;
          pt.shiftTo(1);
        }
        break;
      case "shift_down":
        if (pt.mode === "manual" && pt.shiftDown()) ctx.events.emit({ type: "shift", vehicle: v.id, gear: pt.gearLabel() });
        else if (pt.mode === "automatic" && Math.abs(v.dyn.u) < 0.5) this.parked = true;
        break;
      case "gear_reverse":
        if (Math.abs(v.dyn.u) < 1.5) {
          this.parked = false;
          pt.shiftTo(-1);
        }
        break;
      case "gear_neutral":
        this.parked = false;
        pt.shiftTo(0);
        break;
      case "transmission_mode":
        pt.mode = pt.mode === "automatic" ? "manual" : "automatic";
        this.parked = false;
        if (pt.mode === "automatic" && pt.gear === 0) pt.shiftTo(1);
        this.note(pt.mode === "manual" ? "Manual transmission" : "Automatic transmission");
        break;
      case "indicator_left":
        lights.toggleIndicator("left");
        break;
      case "indicator_right":
        lights.toggleIndicator("right");
        break;
      case "hazards":
        lights.hazard = !lights.hazard;
        break;
      case "headlights":
        lights.cycleHeadlights();
        this.note(`Lights: ${lights.headlightSwitch.toUpperCase()}`);
        break;
      case "high_beam":
        lights.highBeam = !lights.highBeam;
        break;
      case "wipers":
        lights.cycleWipers();
        this.note(`Wipers: ${lights.wipers.toUpperCase()}`);
        break;
      case "siren":
        if (v.spec.emergency) lights.cycleSiren();
        break;
      case "cruise_toggle":
        if (this.cruise.on) {
          this.cruise.on = false;
          this.note("Cruise off");
        } else if (v.dyn.u > 30 * KMH * 0.5 && pt.running) {
          this.cruise.on = true;
          this.cruise.target = v.dyn.u;
          this.cruise.integral = 0;
          this.note(`Cruise set ${Math.floor(this.cruise.target * 3.6 + 0.5)} km/h`);
        } else this.note("Cruise needs at least 15 km/h");
        break;
      case "cruise_up":
        if (this.cruise.on) {
          this.cruise.target += 5 * KMH;
          this.note(`Cruise ${Math.floor(this.cruise.target * 3.6 + 0.5)} km/h`);
        }
        break;
      case "cruise_down":
        if (this.cruise.on) {
          this.cruise.target = Math.max(15 * KMH, this.cruise.target - 5 * KMH);
          this.note(`Cruise ${Math.floor(this.cruise.target * 3.6 + 0.5)} km/h`);
        }
        break;
      case "limiter":
        this.limiterOn = !this.limiterOn;
        this.note(this.limiterOn ? `Limiter ${Math.floor(speedLimit * 3.6 + 0.5)} km/h` : "Limiter off");
        break;
      case "engine":
        if (pt.running) {
          pt.stop();
          ctx.events.emit({ type: "engine", vehicle: v.id, on: false });
          this.note("Engine off");
        } else {
          pt.start();
          ctx.events.emit({ type: "engine", vehicle: v.id, on: true });
        }
        break;
      case "seatbelt":
        this.seatbelt = !this.seatbelt;
        this.note(this.seatbelt ? "Seatbelt fastened" : "Seatbelt unfastened");
        break;
      case "toggle_abs":
        v.assists.abs = !v.assists.abs;
        this.note(`ABS ${v.assists.abs ? "on" : "off"}`);
        break;
      case "toggle_tcs":
        v.assists.tcs = !v.assists.tcs;
        this.note(`Traction control ${v.assists.tcs ? "on" : "off"}`);
        break;
      case "toggle_esc":
        v.assists.esc = !v.assists.esc;
        this.note(`Stability control ${v.assists.esc ? "on" : "off"}`);
        break;
      case "apologize":
        this.gesture = "wave";
        break;
      case "rude_gesture":
        this.gesture = "rude";
        break;
      default:
        if (HOST_ACTIONS.indexOf(id) >= 0) this.hostActions.push(id);
    }
  }
}
