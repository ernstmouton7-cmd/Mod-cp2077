import { ACTIONS, ActionCategory, ActionId, GamepadBinding } from "../core/player/actions";
import { SPECS, VehicleClass, topSpeed } from "../core/vehicle/specs";
import { Weather } from "../core/world/environment";
import { World } from "../core/world/world";

const PAD_NAMES: Record<number, string> = {
  0: "A", 1: "B", 2: "X", 3: "Y", 4: "LB", 5: "RB", 6: "LT", 7: "RT", 8: "View", 9: "Menu", 10: "L-stick click", 11: "R-stick click", 12: "D-pad ↑", 13: "D-pad ↓", 14: "D-pad ←", 15: "D-pad →",
};

export function keyLabel(code: string): string {
  if (code.startsWith("Key")) return code.slice(3);
  if (code.startsWith("Digit")) return code.slice(5);
  const map: Record<string, string> = {
    ArrowUp: "↑", ArrowDown: "↓", ArrowLeft: "←", ArrowRight: "→", Space: "Space", ShiftLeft: "L-Shift", ControlLeft: "L-Ctrl", Backquote: "`", Equal: "=", Minus: "−", NumpadAdd: "Num +", NumpadSubtract: "Num −", BracketLeft: "[", BracketRight: "]", Comma: ",", Period: ".", Escape: "Esc", Backspace: "Backspace", Tab: "Tab",
  };
  return map[code] ?? code;
}

function padLabel(b: GamepadBinding): string {
  if (b.button !== undefined) return PAD_NAMES[b.button] ?? `Button ${b.button}`;
  if (b.axis !== undefined) return b.axis === 0 ? (b.dir === -1 ? "L-stick ←" : "L-stick →") : `Axis ${b.axis}`;
  return "";
}

export interface Settings {
  unit: "kmh" | "mph";
  volume: number;
  bloom: boolean;
  quality: "high" | "low";
  vehicle: VehicleClass;
  mirror: boolean;
}

const STORE = "gridlock.settings.v1";

export function loadSettings(): Partial<Settings> & { keys?: Record<string, string[]> } {
  try {
    const raw = localStorage.getItem(STORE);
    return raw !== null ? JSON.parse(raw) : {};
  } catch {
    return {};
  }
}

export function saveSettings(s: Settings, world: World): void {
  try {
    const keys: Record<string, string[]> = {};
    world.input.bindings.forEach((b, id) => (keys[id] = b.keys));
    localStorage.setItem(STORE, JSON.stringify({ ...s, keys }));
  } catch {
    /* storage unavailable: settings last for this session only */
  }
}

/** Pause menu: resume, controls (rebindable), settings, about. */
export class Menu {
  open = false;
  private waitingFor: ActionId | undefined = undefined;
  onChange: () => void = () => {};
  onVehicle: (cls: VehicleClass) => void = () => {};

  constructor(
    readonly world: World,
    readonly settings: Settings,
  ) {
    document.querySelectorAll<HTMLButtonElement>("[data-tab]").forEach((b) => b.addEventListener("click", () => this.tab(b.dataset.tab as string)));
    (document.getElementById("resume") as HTMLButtonElement).addEventListener("click", () => this.hide());
    this.buildControls();
    this.buildSettings();
  }

  show(tab = "controls"): void {
    this.open = true;
    (document.getElementById("menu") as HTMLElement).hidden = false;
    this.tab(tab);
    this.refreshSettings();
  }

  hide(): void {
    this.open = false;
    this.waitingFor = undefined;
    (document.getElementById("menu") as HTMLElement).hidden = true;
  }

  /** Session statistics and the violation history. */
  renderReport(unit: "kmh" | "mph"): void {
    const w = this.world;
    const st = w.playerStats;
    const rec = w.record;
    const sp = (ms: number) => (unit === "kmh" ? `${Math.round(ms * 3.6)} km/h` : `${Math.round(ms * 2.23694)} mph`);
    const dist = unit === "kmh" ? `${(st.distance / 1000).toFixed(1)} km` : `${(st.distance / 1609.34).toFixed(1)} mi`;
    const mins = Math.floor(st.driveTime / 60);
    const tiles: [string, string][] = [
      ["Distance", dist],
      ["Time moving", `${mins} min`],
      ["Top speed", sp(st.maxSpeed)],
      ["Fines paid", `€$${Math.round(rec.totalFines).toLocaleString("en-US")}`],
      ["License points", `${rec.points}/${rec.maxPoints}`],
      ["Tickets", `${rec.ticketsIssued}`],
      ["Collisions", `${st.collisions}`],
      ["Near misses", `${st.nearMisses}`],
      ["Drivers you angered", `${st.angered}`],
      ["Apologies accepted", `${st.apologiesAccepted}`],
      ["Road-rage episodes", `${st.rageEpisodes}`],
      ["Pedestrians hit", `${st.pedestriansHit}`],
    ];
    const rows = rec.history
      .slice(-14)
      .reverse()
      .map((v) => {
        const t = Math.floor(v.time / 60);
        const witness = v.witness === "none" ? "Unseen" : v.witness === "camera" ? "Camera" : "Police";
        return `<tr><td class="mono">${t} min</td><td>${v.label}</td><td>${witness}</td><td class="mono num">${v.witness === "none" ? "—" : `€$${v.fine}`}</td><td class="mono num">${v.witness === "none" ? "—" : v.points}</td></tr>`;
      })
      .join("");
    const box = document.getElementById("report") as HTMLElement;
    box.innerHTML = `<div class="report-tiles">${tiles.map(([k, v]) => `<div class="tile"><span class="panel-label">${k}</span><b class="mono">${v}</b></div>`).join("")}</div>
      <h3 class="report-h">Violations</h3>
      ${rows === "" ? `<p class="muted">A clean record so far. Every red light, rolling stop and unsignalled turn will show up here, seen or not.</p>` : `<div class="table-wrap"><table class="report-table"><thead><tr><th>When</th><th>Violation</th><th>Seen by</th><th class="num">Fine</th><th class="num">Points</th></tr></thead><tbody>${rows}</tbody></table></div>`}`;
  }

  tab(name: string): void {
    if (name === "report") this.renderReport(this.settings.unit);
    document.querySelectorAll<HTMLElement>("[data-panel]").forEach((p) => (p.hidden = p.dataset.panel !== name));
    document.querySelectorAll<HTMLButtonElement>("[data-tab]").forEach((b) => b.setAttribute("aria-selected", b.dataset.tab === name ? "true" : "false"));
  }

  /** Called by the input layer: returns true when the key was consumed by rebinding. */
  rebind(code: string): boolean {
    if (this.waitingFor === undefined) return false;
    if (code !== "Escape") this.world.input.bindKeys(this.waitingFor, [code]);
    this.waitingFor = undefined;
    this.buildControls();
    saveSettings(this.settings, this.world);
    return true;
  }

  buildControls(): void {
    const box = document.getElementById("controlsList") as HTMLElement;
    const cats: ActionCategory[] = ["Driving", "Gearbox", "Signals & lights", "Comfort & assists", "Social", "Camera", "System"];
    const parts: string[] = [];
    for (const cat of cats) {
      parts.push(`<section class="ctl-cat"><h3>${cat}</h3><div class="ctl-grid">`);
      for (const a of ACTIONS) {
        if (a.category !== cat) continue;
        const b = this.world.input.bindings.get(a.id);
        const keys = b !== undefined && b.keys.length > 0 ? b.keys.map(keyLabel).map((k) => `<kbd>${k}</kbd>`).join(" ") : `<span class="unbound">unbound</span>`;
        const pad = b !== undefined && b.pad.length > 0 ? b.pad.map(padLabel).map((k) => `<span class="pad">${k}</span>`).join(" ") : "";
        const waiting = this.waitingFor === a.id;
        parts.push(`<div class="ctl"><div class="ctl-name">${a.label}<small>${a.help}</small></div><div class="ctl-keys"><button class="rebind${waiting ? " waiting" : ""}" data-rebind="${a.id}" id="rb-${a.id}">${waiting ? "Press a key…" : keys}</button>${pad}</div></div>`);
      }
      parts.push(`</div></section>`);
    }
    box.innerHTML = parts.join("");
    box.querySelectorAll<HTMLButtonElement>("[data-rebind]").forEach((btn) =>
      btn.addEventListener("click", () => {
        this.waitingFor = btn.dataset.rebind as ActionId;
        this.buildControls();
      }),
    );
  }

  private buildSettings(): void {
    const sel = document.getElementById("setVehicle") as HTMLSelectElement;
    sel.innerHTML = (Object.keys(SPECS) as VehicleClass[])
      .map((c) => {
        const s = SPECS[c];
        return `<option value="${c}">${s.name} · ${s.powerKw} kW · ${Math.round(topSpeed(s) * 3.6)} km/h</option>`;
      })
      .join("");
    sel.addEventListener("change", () => {
      this.settings.vehicle = sel.value as VehicleClass;
      this.onVehicle(this.settings.vehicle);
      this.onChange();
    });
    const bind = (id: string, ev: string, fn: (el: HTMLInputElement & HTMLSelectElement) => void) => {
      const el = document.getElementById(id) as HTMLInputElement & HTMLSelectElement;
      el.addEventListener(ev, () => {
        fn(el);
        this.onChange();
      });
    };
    const pd = () => this.world.playerDriver;
    bind("setTrans", "change", (el) => {
      const p = this.world.player;
      if (p !== undefined) p.pt.mode = el.value as "automatic" | "manual";
    });
    bind("setClutch", "change", (el) => {
      const d = pd();
      if (d !== undefined) d.settings.clutchAssist = el.checked;
    });
    bind("setAbs", "change", (el) => {
      const p = this.world.player;
      if (p !== undefined) p.assists.abs = el.checked;
    });
    bind("setTcs", "change", (el) => {
      const p = this.world.player;
      if (p !== undefined) p.assists.tcs = el.checked;
    });
    bind("setEsc", "change", (el) => {
      const p = this.world.player;
      if (p !== undefined) p.assists.esc = el.checked;
    });
    bind("setUnit", "change", (el) => (this.settings.unit = el.value as "kmh" | "mph"));
    bind("setHour", "input", (el) => (this.world.env.hour = Number(el.value)));
    bind("setTimeScale", "change", (el) => (this.world.env.timeScale = Number(el.value)));
    bind("setWeather", "change", (el) => {
      this.world.env.setWeather(el.value as Weather);
    });
    bind("setAutoWeather", "change", (el) => (this.world.env.autoWeather = el.checked));
    bind("setTraffic", "input", (el) => (this.world.spawner.opts.civilians = Number(el.value)));
    bind("setPolice", "input", (el) => (this.world.spawner.opts.police = Number(el.value)));
    bind("setVolume", "input", (el) => (this.settings.volume = Number(el.value)));
    bind("setIncidents", "change", (el) => {
      this.world.breakdownInterval = el.checked ? 80 : 0;
      this.world.outageInterval = el.checked ? 300 : 0;
    });
    bind("setBloom", "change", (el) => (this.settings.bloom = el.checked));
    bind("setMirror", "change", (el) => (this.settings.mirror = el.checked));
    bind("setQuality", "change", (el) => (this.settings.quality = el.value as "high" | "low"));
    bind("setSignals", "change", (el) => {
      for (const s of this.world.signals) s.setMode(el.value as "actuated");
    });
    bind("setGrip", "change", (el) => {
      const d = pd();
      if (d !== undefined) d.settings.autoReverse = el.checked;
    });
  }

  refreshSettings(): void {
    const p = this.world.player;
    const d = this.world.playerDriver;
    const set = (id: string, v: string | boolean) => {
      const el = document.getElementById(id) as HTMLInputElement;
      if (typeof v === "boolean") el.checked = v;
      else el.value = v;
    };
    set("setVehicle", this.settings.vehicle);
    if (p !== undefined) {
      set("setTrans", p.pt.mode);
      set("setAbs", p.assists.abs);
      set("setTcs", p.assists.tcs);
      set("setEsc", p.assists.esc);
    }
    if (d !== undefined) {
      set("setClutch", d.settings.clutchAssist);
      set("setGrip", d.settings.autoReverse);
    }
    set("setUnit", this.settings.unit);
    set("setHour", `${this.world.env.hour.toFixed(1)}`);
    set("setTimeScale", `${this.world.env.timeScale}`);
    set("setWeather", this.world.env.weather);
    set("setAutoWeather", this.world.env.autoWeather);
    set("setTraffic", `${this.world.spawner.opts.civilians}`);
    set("setPolice", `${this.world.spawner.opts.police}`);
    set("setVolume", `${this.settings.volume}`);
    set("setIncidents", this.world.breakdownInterval > 0);
    set("setBloom", this.settings.bloom);
    set("setMirror", this.settings.mirror);
    set("setQuality", this.settings.quality);
    const s0 = this.world.signals[0];
    if (s0 !== undefined) set("setSignals", s0.mode);
  }
}
