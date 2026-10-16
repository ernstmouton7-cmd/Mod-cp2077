import { ACTIONS, ActionId } from "../core/player/actions";
import { World } from "../core/world/world";

/** UI actions handled by the page itself (work while paused too). */
export const UI_ACTIONS: ActionId[] = ["pause", "help", "map", "debug"];

export interface InputCallbacks {
  ui(action: ActionId): void;
  /** Mouse drag / right stick orbit for chase cameras (radians). */
  orbit(dx: number, dy: number): void;
  /** Any user gesture (to unlock audio). */
  gesture(): void;
  /** A key was pressed while waiting for a rebinding. */
  rebind?(code: string): boolean;
}

/** Keyboard, gamepad, mouse and touch wiring for the sandbox. */
export class BrowserInput {
  private padPrev: number[] = [];
  padName = "";
  private dragging = false;
  private lastX = 0;
  private lastY = 0;

  constructor(
    readonly world: World,
    readonly cb: InputCallbacks,
    canvas: HTMLElement,
  ) {
    window.addEventListener("keydown", (e) => {
      cb.gesture();
      if (cb.rebind !== undefined && cb.rebind(e.code)) {
        e.preventDefault();
        return;
      }
      if (e.repeat) {
        if (this.boundTo(e.code) !== undefined) e.preventDefault();
        return;
      }
      const ui = this.uiFor(e.code);
      if (ui !== undefined) {
        e.preventDefault();
        cb.ui(ui);
        return;
      }
      if (world.input.key(e.code, true)) e.preventDefault();
    });
    window.addEventListener("keyup", (e) => {
      if (world.input.key(e.code, false)) e.preventDefault();
    });
    window.addEventListener("blur", () => world.input.releaseAll());
    canvas.addEventListener("pointerdown", (e) => {
      cb.gesture();
      if (e.pointerType === "mouse") {
        this.dragging = true;
        this.lastX = e.clientX;
        this.lastY = e.clientY;
      }
    });
    window.addEventListener("pointerup", () => (this.dragging = false));
    window.addEventListener("pointermove", (e) => {
      if (!this.dragging) return;
      cb.orbit((e.clientX - this.lastX) * 0.006, (e.clientY - this.lastY) * 0.004);
      this.lastX = e.clientX;
      this.lastY = e.clientY;
    });
  }

  private boundTo(code: string): ActionId | undefined {
    for (const a of ACTIONS) {
      const b = this.world.input.bindings.get(a.id);
      if (b !== undefined && b.keys.includes(code)) return a.id;
    }
    return undefined;
  }

  private uiFor(code: string): ActionId | undefined {
    const id = this.boundTo(code);
    return id !== undefined && UI_ACTIONS.includes(id) ? id : undefined;
  }

  /** Poll the first connected gamepad (call once per frame). */
  pollGamepad(): void {
    const pads = typeof navigator.getGamepads === "function" ? navigator.getGamepads() : [];
    let pad: Gamepad | null = null;
    for (const p of pads) if (p !== null && p.connected) {
      pad = p;
      break;
    }
    if (pad === null) {
      this.padName = "";
      return;
    }
    this.padName = pad.id;
    const buttons = pad.buttons.map((b) => b.value);
    const axes = pad.axes.slice();
    // UI buttons edge-detected here so they work while paused.
    for (const id of UI_ACTIONS) {
      const b = this.world.input.bindings.get(id);
      if (b === undefined) continue;
      for (const pb of b.pad) {
        if (pb.button === undefined) continue;
        const now = buttons[pb.button] ?? 0;
        const before = this.padPrev[pb.button] ?? 0;
        if (now > 0.5 && before <= 0.5) {
          this.cb.gesture();
          this.cb.ui(id);
        }
      }
    }
    this.padPrev = buttons;
    // Remove UI buttons from what the sim sees.
    const simButtons = buttons.slice();
    for (const id of UI_ACTIONS) {
      const b = this.world.input.bindings.get(id);
      if (b !== undefined) for (const pb of b.pad) if (pb.button !== undefined) simButtons[pb.button] = 0;
    }
    this.world.input.gamepad(simButtons, axes);
    // Right stick orbits the camera.
    const rx = axes[2] ?? 0;
    const ry = axes[3] ?? 0;
    if (Math.abs(rx) > 0.2 || Math.abs(ry) > 0.2) this.cb.orbit(rx * 0.04, ry * 0.02);
  }
}

/** On-screen controls for touch devices. */
export function setupTouch(world: World, cb: InputCallbacks): void {
  const root = document.getElementById("touch");
  if (root === null) return;
  const coarse = window.matchMedia("(pointer: coarse)").matches || "ontouchstart" in window;
  root.hidden = !coarse;
  root.querySelectorAll<HTMLElement>("[data-hold]").forEach((el) => {
    const id = el.dataset.hold as ActionId;
    const down = (e: PointerEvent) => {
      e.preventDefault();
      cb.gesture();
      el.setPointerCapture(e.pointerId);
      el.classList.add("down");
      world.input.set(id, 1);
    };
    const up = (e: PointerEvent) => {
      e.preventDefault();
      el.classList.remove("down");
      world.input.set(id, 0);
    };
    el.addEventListener("pointerdown", down);
    el.addEventListener("pointerup", up);
    el.addEventListener("pointercancel", up);
    el.addEventListener("lostpointercapture", up);
  });
  root.querySelectorAll<HTMLElement>("[data-press]").forEach((el) => {
    const id = el.dataset.press as ActionId;
    el.addEventListener("pointerdown", (e) => {
      e.preventDefault();
      cb.gesture();
      if (UI_ACTIONS.includes(id)) cb.ui(id);
      else world.input.press(id);
      el.classList.add("down");
      setTimeout(() => el.classList.remove("down"), 150);
    });
  });
}
