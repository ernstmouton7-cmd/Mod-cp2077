/**
 * Every driver control as an abstract action with default bindings.
 * Keyboard codes follow KeyboardEvent.code; gamepad indices follow the W3C
 * "standard" mapping. Hosts (browser, CET, any engine) feed raw input via
 * InputMapper or set action values directly.
 */
export type ActionId =
  | "throttle"
  | "brake"
  | "steer_left"
  | "steer_right"
  | "handbrake"
  | "clutch"
  | "shift_up"
  | "shift_down"
  | "gear_reverse"
  | "gear_neutral"
  | "horn"
  | "indicator_left"
  | "indicator_right"
  | "hazards"
  | "headlights"
  | "high_beam"
  | "flash"
  | "wipers"
  | "cruise_toggle"
  | "cruise_up"
  | "cruise_down"
  | "limiter"
  | "engine"
  | "transmission_mode"
  | "seatbelt"
  | "apologize"
  | "rude_gesture"
  | "siren"
  | "camera"
  | "look_back"
  | "look_left"
  | "look_right"
  | "reset_vehicle"
  | "toggle_abs"
  | "toggle_tcs"
  | "toggle_esc"
  | "pause"
  | "help"
  | "map"
  | "debug"
  | "time_forward"
  | "weather_cycle";

export type ActionKind = "analog" | "hold" | "press";

export type ActionCategory = "Driving" | "Gearbox" | "Signals & lights" | "Comfort & assists" | "Social" | "Camera" | "System";

export interface GamepadBinding {
  button?: number;
  /** Axis index and direction (+1 / -1). */
  axis?: number;
  dir?: number;
}

export interface ActionDef {
  id: ActionId;
  label: string;
  category: ActionCategory;
  kind: ActionKind;
  keys: string[];
  pad: GamepadBinding[];
  help: string;
}

const A = (id: ActionId, label: string, category: ActionCategory, kind: ActionKind, keys: string[], pad: GamepadBinding[], help: string): ActionDef => ({ id, label, category, kind, keys, pad, help });

export const ACTIONS: ActionDef[] = [
  A("throttle", "Throttle", "Driving", "analog", ["KeyW", "ArrowUp"], [{ button: 7 }], "Accelerate (in reverse with automatic: brake)."),
  A("brake", "Brake / reverse", "Driving", "analog", ["KeyS", "ArrowDown"], [{ button: 6 }], "Brake. With automatic gearbox, hold at a standstill to reverse."),
  A("steer_left", "Steer left", "Driving", "analog", ["KeyA", "ArrowLeft"], [{ axis: 0, dir: -1 }], "Steer left."),
  A("steer_right", "Steer right", "Driving", "analog", ["KeyD", "ArrowRight"], [{ axis: 0, dir: 1 }], "Steer right."),
  A("handbrake", "Handbrake", "Driving", "hold", ["Space"], [{ button: 0 }], "Locks the rear wheels: parking, hairpins and drifts."),
  A("clutch", "Clutch", "Gearbox", "analog", ["KeyZ"], [{ button: 4 }], "Manual gearbox: press to shift or launch without stalling."),
  A("shift_up", "Shift up", "Gearbox", "press", ["ShiftLeft", "KeyR"], [{ button: 1 }], "Next gear (manual)."),
  A("shift_down", "Shift down", "Gearbox", "press", ["ControlLeft", "KeyF"], [{ button: 2 }], "Previous gear (manual)."),
  A("gear_reverse", "Select reverse", "Gearbox", "press", ["Backspace"], [], "Select R directly."),
  A("gear_neutral", "Select neutral", "Gearbox", "press", ["KeyN"], [], "Select N directly."),
  A("transmission_mode", "Auto / manual", "Gearbox", "press", ["KeyT"], [], "Toggle automatic and manual transmission."),
  A("horn", "Horn", "Signals & lights", "hold", ["KeyH"], [{ button: 10 }], "Tap for a beep, hold for a long blast (other drivers notice)."),
  A("indicator_left", "Left indicator", "Signals & lights", "press", ["KeyQ"], [{ button: 14 }], "Toggle left turn signal (auto-cancels after the turn)."),
  A("indicator_right", "Right indicator", "Signals & lights", "press", ["KeyE"], [{ button: 15 }], "Toggle right turn signal (auto-cancels after the turn)."),
  A("hazards", "Hazard lights", "Signals & lights", "press", ["KeyX"], [{ button: 13 }], "All four indicators. A quick blink after merging says thanks."),
  A("headlights", "Headlights", "Signals & lights", "press", ["KeyL"], [{ button: 12 }], "Cycle the light switch: OFF, AUTO, ON."),
  A("high_beam", "High beam", "Signals & lights", "press", ["KeyK"], [], "Toggle high beams (dazzles oncoming drivers)."),
  A("flash", "Flash high beams", "Signals & lights", "hold", ["KeyJ"], [{ button: 5 }], "Flash to pass / warn. Drivers ahead take it personally."),
  A("wipers", "Wipers", "Signals & lights", "press", ["KeyP"], [], "Cycle wipers: OFF, INT, LOW, HIGH."),
  A("siren", "Siren", "Signals & lights", "press", ["KeyO"], [], "Emergency vehicles only: cycle siren."),
  A("cruise_toggle", "Cruise control", "Comfort & assists", "press", ["KeyC"], [], "Set / cancel cruise at the current speed. Brake also cancels."),
  A("cruise_up", "Cruise +5", "Comfort & assists", "press", ["Equal", "NumpadAdd"], [], "Raise cruise speed by 5 km/h."),
  A("cruise_down", "Cruise -5", "Comfort & assists", "press", ["Minus", "NumpadSubtract"], [], "Lower cruise speed by 5 km/h."),
  A("limiter", "Speed limiter", "Comfort & assists", "press", ["KeyU"], [], "Cap speed at the posted limit."),
  A("engine", "Engine start / stop", "Comfort & assists", "press", ["KeyI"], [], "Turn the ignition. Needed after a stall."),
  A("seatbelt", "Seatbelt", "Comfort & assists", "press", ["KeyB"], [], "Buckle up. Police fine unbelted drivers."),
  A("toggle_abs", "ABS on/off", "Comfort & assists", "press", ["Digit1"], [], "Anti-lock brakes."),
  A("toggle_tcs", "Traction control on/off", "Comfort & assists", "press", ["Digit2"], [], "Traction control (off = burnouts)."),
  A("toggle_esc", "Stability control on/off", "Comfort & assists", "press", ["Digit3"], [], "Electronic stability control."),
  A("apologize", "Wave / sorry", "Social", "press", ["KeyG"], [{ button: 3 }], "Apologise to nearby drivers. Calms people you just upset."),
  A("rude_gesture", "Rude gesture", "Social", "press", ["KeyY"], [], "Insult nearby drivers. Expect consequences."),
  A("camera", "Change camera", "Camera", "press", ["KeyV"], [{ button: 8 }], "Chase, far chase, hood, cockpit, top-down, cinematic."),
  A("look_back", "Look back", "Camera", "hold", ["KeyM"], [{ button: 11 }], "Hold to look behind."),
  A("look_left", "Look left", "Camera", "hold", ["Comma"], [], "Hold to look left."),
  A("look_right", "Look right", "Camera", "hold", ["Period"], [], "Hold to look right."),
  A("reset_vehicle", "Reset vehicle", "System", "press", ["Backquote"], [], "Put the car back on the nearest road (repair costs money)."),
  A("pause", "Pause / menu", "System", "press", ["Escape"], [{ button: 9 }], "Pause and open the menu."),
  A("help", "Controls help", "System", "press", ["F1"], [], "Show all controls."),
  A("map", "Map", "System", "press", ["Tab"], [], "Toggle the big map."),
  A("debug", "Debug overlay", "System", "press", ["F3"], [], "Show AI moods, intentions and signal phases."),
  A("time_forward", "Skip time", "System", "press", ["BracketRight"], [], "Advance the clock by one hour."),
  A("weather_cycle", "Change weather", "System", "press", ["BracketLeft"], [], "Cycle clear, rain, storm, fog."),
];

export function actionDef(id: ActionId): ActionDef {
  for (const a of ACTIONS) if (a.id === id) return a;
  return ACTIONS[0];
}

/** Snapshot of driver controls for one simulation step. */
export interface ControlFrame {
  throttle: number;
  brake: number;
  /** -1 left .. +1 right */
  steer: number;
  handbrake: number;
  clutch: number;
  /** Actions pressed since the previous frame. */
  pressed: ActionId[];
  /** Actions released since the previous frame (with how long they were held). */
  released: { id: ActionId; held: number }[];
  /** Actions currently held. */
  held: ActionId[];
}

/**
 * Collects raw keyboard/gamepad/direct input and turns it into ControlFrames
 * with keyboard smoothing (throttle ramps, speed-sensitive steering).
 */
export class InputMapper {
  /** Current raw value of each action (0..1). */
  private raw: Map<ActionId, number> = new Map();
  private prev: Map<ActionId, number> = new Map();
  private heldTime: Map<ActionId, number> = new Map();
  private keysDown: Map<string, boolean> = new Map();
  private padValues: Map<ActionId, number> = new Map();
  private direct: Map<ActionId, number> = new Map();
  /** Edge-triggered presses that happened between frames (so taps aren't lost). */
  private pendingPress: ActionId[] = [];
  private keyIndex: Map<string, ActionId[]> = new Map();
  /** Smoothed outputs. */
  private sThrottle = 0;
  private sBrake = 0;
  private sSteer = 0;
  private sClutch = 0;
  /** True if the last analog input came from a gamepad (no smoothing). */
  analogFromPad = false;
  readonly bindings: Map<ActionId, { keys: string[]; pad: GamepadBinding[] }> = new Map();

  constructor() {
    for (const a of ACTIONS) this.bindings.set(a.id, { keys: a.keys.slice(), pad: a.pad.slice() });
    this.rebuildIndex();
  }

  rebuildIndex(): void {
    this.keyIndex = new Map();
    this.bindings.forEach((b, id) => {
      for (const k of b.keys) {
        let l = this.keyIndex.get(k);
        if (l === undefined) {
          l = [];
          this.keyIndex.set(k, l);
        }
        l.push(id);
      }
    });
  }

  /** Rebind an action's keyboard keys. */
  bindKeys(id: ActionId, keys: string[]): void {
    const b = this.bindings.get(id);
    if (b === undefined) return;
    // A key can only drive one action.
    this.bindings.forEach((o) => {
      for (const k of keys) {
        const i = o.keys.indexOf(k);
        if (i >= 0) o.keys.splice(i, 1);
      }
    });
    b.keys = keys.slice();
    this.rebuildIndex();
  }

  bindPad(id: ActionId, pad: GamepadBinding[]): void {
    const b = this.bindings.get(id);
    if (b !== undefined) b.pad = pad.slice();
  }

  /** Returns true if the key is bound (so hosts can preventDefault). */
  key(code: string, down: boolean): boolean {
    const ids = this.keyIndex.get(code);
    const was = this.keysDown.get(code) === true;
    this.keysDown.set(code, down);
    if (ids === undefined) return false;
    if (down && !was) for (const id of ids) this.pendingPress.push(id);
    if (down) this.analogFromPad = false;
    return true;
  }

  /** Feed a gamepad state (standard mapping). */
  gamepad(buttons: ReadonlyArray<number>, axes: ReadonlyArray<number>): void {
    let any = false;
    for (const def of ACTIONS) {
      const id = def.id;
      const b = this.bindings.get(id);
      if (b === undefined) continue;
      let v = 0;
      for (const p of b.pad) {
        if (p.button !== undefined && p.button < buttons.length) v = Math.max(v, buttons[p.button]);
        if (p.axis !== undefined && p.axis < axes.length) {
          const a = axes[p.axis] * (p.dir ?? 1);
          const dz = 0.12;
          if (a > dz) v = Math.max(v, (a - dz) / (1 - dz));
        }
      }
      const before = this.padValues.get(id) ?? 0;
      if (v > 0.5 && before <= 0.5) this.pendingPress.push(id);
      if (v > 0.05) any = true;
      this.padValues.set(id, v);
    }
    if (any) this.analogFromPad = true;
  }

  /** Set an action directly (touch UI, mod hotkeys, tests). */
  set(id: ActionId, value: number): void {
    const before = this.direct.get(id) ?? 0;
    if (value > 0.5 && before <= 0.5) this.pendingPress.push(id);
    this.direct.set(id, value);
  }

  /** Fire a one-shot press. */
  press(id: ActionId): void {
    this.pendingPress.push(id);
  }

  releaseAll(): void {
    this.keysDown = new Map();
    this.direct = new Map();
    this.padValues = new Map();
  }

  private value(id: ActionId): number {
    let v = this.direct.get(id) ?? 0;
    v = Math.max(v, this.padValues.get(id) ?? 0);
    const b = this.bindings.get(id);
    if (b !== undefined) for (const k of b.keys) if (this.keysDown.get(k) === true) v = 1;
    return v;
  }

  /**
   * Build the control frame for this step.
   * @param speed current vehicle speed (m/s), for steering sensitivity.
   */
  frame(dt: number, speed: number): ControlFrame {
    const pressed: ActionId[] = [];
    const released: { id: ActionId; held: number }[] = [];
    const held: ActionId[] = [];
    // Keep repeats: two quick taps between frames are two presses.
    for (const id of this.pendingPress) pressed.push(id);
    this.pendingPress = [];
    for (const a of ACTIONS) {
      const v = this.value(a.id);
      const before = this.prev.get(a.id) ?? 0;
      this.raw.set(a.id, v);
      if (v > 0.5) {
        held.push(a.id);
        this.heldTime.set(a.id, (this.heldTime.get(a.id) ?? 0) + dt);
      } else if (before > 0.5) {
        released.push({ id: a.id, held: this.heldTime.get(a.id) ?? 0 });
        this.heldTime.set(a.id, 0);
      }
      this.prev.set(a.id, v);
    }
    const thr = this.raw.get("throttle") ?? 0;
    const brk = this.raw.get("brake") ?? 0;
    const steer = (this.raw.get("steer_right") ?? 0) - (this.raw.get("steer_left") ?? 0);
    const clutch = this.raw.get("clutch") ?? 0;
    if (this.analogFromPad) {
      this.sThrottle = thr;
      this.sBrake = brk;
      this.sSteer += (steer - this.sSteer) * Math.min(1, dt * 20);
      this.sClutch = clutch;
    } else {
      // Keyboard: ramp pedals, steer faster when returning to centre, slower at speed.
      this.sThrottle = ramp(this.sThrottle, thr, dt * (thr > this.sThrottle ? 4 : 8));
      this.sBrake = ramp(this.sBrake, brk, dt * (brk > this.sBrake ? 5 : 10));
      this.sClutch = ramp(this.sClutch, clutch, dt * 6);
      const speedFactor = 1 / (1 + Math.abs(speed) / 18);
      const target = steer * (0.35 + 0.65 * speedFactor);
      const returning = Math.abs(target) < Math.abs(this.sSteer) || target * this.sSteer < 0;
      this.sSteer = ramp(this.sSteer, target, dt * (returning ? 5 : 2.2 + 1.8 * speedFactor));
    }
    return {
      throttle: this.sThrottle,
      brake: this.sBrake,
      steer: this.sSteer,
      handbrake: this.raw.get("handbrake") ?? 0,
      clutch: this.sClutch,
      pressed,
      released,
      held,
    };
  }

  isHeld(id: ActionId): boolean {
    return (this.raw.get(id) ?? 0) > 0.5;
  }

  heldFor(id: ActionId): number {
    return this.heldTime.get(id) ?? 0;
  }
}

function ramp(cur: number, target: number, maxDelta: number): number {
  if (cur < target) return Math.min(target, cur + maxDelta);
  return Math.max(target, cur - maxDelta);
}
