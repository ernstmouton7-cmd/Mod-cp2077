import * as THREE from "three";
import { HeadState, Lamp } from "../core/signals/signalController";
import { Vehicle } from "../core/vehicle/vehicle";
import { World } from "../core/world/world";
import { SimEvent } from "../core/world/events";
import { Mood } from "../core/ai/emotion";

const $ = <T extends HTMLElement = HTMLElement>(id: string): T => document.getElementById(id) as T;

export type SpeedUnit = "kmh" | "mph";

const LAMP_CSS: Record<Lamp, string> = { red: "var(--sig-red)", yellow: "var(--sig-amber)", green: "var(--sig-green)", off: "transparent" };

const MOOD_LABEL: Record<Mood, string> = { calm: "Calm", irritated: "Irritated", angry: "Angry", furious: "Furious" };

interface Bubble {
  el: HTMLDivElement;
  vehicle: number;
  ttl: number;
}

function fmtMoney(n: number): string {
  const s = Math.abs(Math.round(n)).toLocaleString("en-US");
  return `${n < 0 ? "−" : ""}€$${s}`;
}

/** The DOM heads-up display. */
export class Hud {
  unit: SpeedUnit = "kmh";
  private cluster: HTMLCanvasElement;
  private cctx: CanvasRenderingContext2D;
  private mini: HTMLCanvasElement;
  private mctx: CanvasRenderingContext2D;
  private mapImage: HTMLCanvasElement | undefined;
  private mapScale = 1;
  private bubbles: Bubble[] = [];
  private toastBox: HTMLElement;
  private textTimer = 0;
  private bigMap = false;
  debug = false;
  private debugLayer: HTMLElement;
  private debugLabels: Map<number, HTMLDivElement> = new Map();
  private lastBanner = "";
  private flashEl: HTMLElement;

  constructor(readonly world: World) {
    this.cluster = $<HTMLCanvasElement>("cluster");
    this.cctx = this.cluster.getContext("2d") as CanvasRenderingContext2D;
    this.mini = $<HTMLCanvasElement>("minimap");
    this.mctx = this.mini.getContext("2d") as CanvasRenderingContext2D;
    this.toastBox = $("toasts");
    this.debugLayer = $("debugLayer");
    this.flashEl = $("flash");
    this.buildMapImage();
  }

  private buildMapImage(): void {
    const net = this.world.net;
    const pad = 30;
    const w = net.maxX - net.minX + pad * 2;
    const h = net.maxY - net.minY + pad * 2;
    const scale = 2;
    const c = document.createElement("canvas");
    c.width = Math.ceil(w * scale);
    c.height = Math.ceil(h * scale);
    const g = c.getContext("2d") as CanvasRenderingContext2D;
    g.fillStyle = "#15161c";
    g.fillRect(0, 0, c.width, c.height);
    for (const b of this.world.city.buildings) {
      g.fillStyle = b.kind === "park" ? "#1d3324" : "#23242c";
      g.fillRect((b.x - b.w / 2 - net.minX + pad) * scale, (net.maxY + pad - (b.y + b.h / 2)) * scale, b.w * scale, b.h * scale);
    }
    g.lineCap = "round";
    for (const r of net.roads) {
      g.strokeStyle = r.kind === "avenue" || r.kind === "ring" ? "#77787f" : "#5a5b62";
      g.lineWidth = r.width * scale;
      g.beginPath();
      r.centerline.forEach((p, i) => {
        const x = (p.x - net.minX + pad) * scale;
        const y = (net.maxY + pad - p.y) * scale;
        if (i === 0) g.moveTo(x, y);
        else g.lineTo(x, y);
      });
      g.stroke();
    }
    this.mapImage = c;
    this.mapScale = scale;
    this.mapPad = pad;
  }

  private mapPad = 30;

  toggleBigMap(): boolean {
    this.bigMap = !this.bigMap;
    $("mapWrap").classList.toggle("big", this.bigMap);
    return this.bigMap;
  }

  speedText(ms: number): string {
    return `${Math.round(this.unit === "kmh" ? ms * 3.6 : ms * 2.23694)}`;
  }

  toast(text: string, kind: "violation" | "ticket" | "info" | "rage" | "police" | "good", sub?: string): void {
    const el = document.createElement("div");
    el.className = `toast ${kind}`;
    const t = document.createElement("div");
    t.className = "toast-title";
    t.textContent = text;
    el.appendChild(t);
    if (sub !== undefined) {
      const s = document.createElement("div");
      s.className = "toast-sub";
      s.textContent = sub;
      el.appendChild(s);
    }
    this.toastBox.prepend(el);
    while (this.toastBox.children.length > 5) this.toastBox.lastElementChild?.remove();
    setTimeout(() => el.classList.add("out"), 4200);
    setTimeout(() => el.remove(), 4800);
  }

  cameraFlash(): void {
    this.flashEl.classList.remove("on");
    void this.flashEl.offsetWidth;
    this.flashEl.classList.add("on");
  }

  speech(vehicle: number, text: string, mood: Mood): void {
    const el = document.createElement("div");
    el.className = `bubble mood-${mood}`;
    el.textContent = text;
    $("bubbles").appendChild(el);
    this.bubbles.push({ el, vehicle, ttl: 3.4 });
    if (this.bubbles.length > 8) {
      const b = this.bubbles.shift();
      b?.el.remove();
    }
  }

  onEvent(e: SimEvent): void {
    const w = this.world;
    const p = w.player;
    const me = p !== undefined ? p.id : -1;
    switch (e.type) {
      case "violation":
        if (e.witnessed === "none") this.toast(e.label, "info", "Nobody saw that");
        else this.toast(e.label, "violation", e.witnessed === "police" ? `Police saw it · ${fmtMoney(e.fine)} · ${e.points} pts pending` : `Camera · ${fmtMoney(e.fine)}`);
        break;
      case "ticket":
        this.toast(`Ticket: ${fmtMoney(e.total)}`, "ticket", `${e.reason} · +${e.points} points`);
        break;
      case "camera_flash":
        this.cameraFlash();
        break;
      case "speech": {
        const v = w.vehicle(e.vehicle);
        if (v !== undefined && p !== undefined && Math.hypot(v.x - p.x, v.y - p.y) < 70) this.speech(e.vehicle, e.text, e.mood);
        break;
      }
      case "rage_start":
        if (e.target === me) {
          const v = w.vehicle(e.vehicle);
          const what = e.behavior === "ram" ? "is trying to ram you" : e.behavior === "brake_check" ? "is brake-checking you" : e.behavior === "chase" ? "is chasing you" : "is tailgating you";
          this.toast(`Road rage: ${v !== undefined ? v.spec.name : "a driver"} ${what}`, "rage", "Wave (G) to apologise, or get away");
        }
        break;
      case "rage_end":
        if (e.reason === "police nearby") this.toast("The angry driver backed off", "good", "Police nearby");
        break;
      case "apology":
        if (e.calmed > 5) this.toast("Apology accepted", "good", `Anger −${Math.round(e.calmed)}`);
        break;
      case "pullover":
        if (e.stage === "request") this.toast("Police: pull over", "police", "Slow down and stop at the curb");
        else if (e.stage === "fled") this.toast("You fled from police", "violation", "Lose them: break line of sight");
        else if (e.stage === "escaped") this.toast("You lost the police", "good");
        else if (e.stage === "issued") this.toast("Officer: drive safely", "police");
        break;
      case "dispatch":
        if (e.kind === "ambulance") this.toast("Ambulance dispatched", "info", "Pull right and let it pass");
        break;
      case "collision":
        if (e.a === me || e.b === me) {
          if (e.severity > 2) this.toast(e.atFault === me ? "You caused a collision" : "You were hit", e.atFault === me ? "violation" : "info", `Impact ${(e.severity * 3.6).toFixed(0)} km/h Δv`);
        }
        break;
      case "info":
        this.toast(e.text, "info");
        break;
      default:
        break;
    }
  }

  private setText(id: string, text: string): void {
    const el = $(id);
    if (el.textContent !== text) el.textContent = text;
  }

  private setLamp(id: string, on: boolean): void {
    $(id).classList.toggle("on", on);
  }

  private miniSignal(head: HeadState | undefined, control: string, camera: boolean, distance: number, nodeName: string): void {
    const box = $("nextJunction");
    box.hidden = control === "none";
    if (control === "none") return;
    this.setText("njKind", control === "signal" ? "Signal" : control === "stop" ? "Stop sign" : "Yield");
    this.setText("njDist", `${Math.round(distance)} m`);
    this.setText("njName", nodeName);
    $("njCam").hidden = !camera;
    const sig = $("njSignal");
    sig.hidden = control !== "signal";
    $("njSign").hidden = control === "signal";
    $("njSign").className = `mini-sign ${control}`;
    if (head !== undefined) {
      const set = (id: string, lamp: Lamp, color: Lamp) => {
        const el = $(id);
        el.style.background = lamp === color ? LAMP_CSS[color] : "";
        el.classList.toggle("lit", lamp === color);
      };
      set("lr", head.ball, "red");
      set("ly", head.ball, "yellow");
      set("lg", head.ball, "green");
      const arrow = $("larrow");
      arrow.hidden = head.arrow === undefined;
      if (head.arrow !== undefined) {
        const a = head.arrow;
        arrow.dataset.lamp = a === "off" && head.ball === "green" ? "flash" : a;
      }
      const ped = $("ped");
      ped.dataset.state = head.ped;
      this.setText("pedCount", head.ped === "flash" ? `${head.pedCountdown}` : head.ped === "walk" ? "WALK" : head.ped === "off" ? "" : "WAIT");
    }
  }

  update(dt: number, camera: THREE.PerspectiveCamera, gearText: string, cruise: { on: boolean; target: number }, limiter: boolean, seatbelt: boolean, notes: string[]): void {
    const w = this.world;
    const p = w.player;
    if (p === undefined) return;
    this.drawCluster(p, gearText);
    this.drawMinimap(p);
    this.updateBubbles(dt, camera);
    if (this.debug) this.updateDebug(camera);
    // Blinking indicator arrows every frame.
    this.setLamp("indL", p.lights.leftLampOn());
    this.setLamp("indR", p.lights.rightLampOn());
    this.textTimer -= dt;
    if (this.textTimer > 0) return;
    this.textTimer = 0.1;
    const info = w.monitor.info;
    const lights = p.lights;
    const pt = p.pt;
    const dyn = p.dyn;
    // Tell-tales.
    this.setLamp("ttLow", lights.lowBeamOn() && !lights.highBeamOn());
    this.setLamp("ttHigh", lights.highBeamOn());
    this.setLamp("ttHaz", lights.hazard && lights.blinkOn());
    this.setLamp("ttAbs", dyn.absActive || !p.assists.abs);
    this.setText("ttAbs", p.assists.abs ? "ABS" : "ABS OFF");
    this.setLamp("ttTcs", dyn.tcsActive || !p.assists.tcs);
    this.setText("ttTcs", p.assists.tcs ? "TCS" : "TCS OFF");
    this.setLamp("ttEsc", dyn.escActive || !p.assists.esc);
    this.setText("ttEsc", p.assists.esc ? "ESC" : "ESC OFF");
    this.setLamp("ttBelt", !seatbelt);
    this.setLamp("ttEng", pt.health < 0.5 || pt.stalled || !pt.running);
    this.setLamp("ttFuel", pt.fuel < p.spec.fuelCapacity * 0.12);
    this.setLamp("ttBrake", p.inputs.handbrake > 0.5);
    this.setLamp("ttCruise", cruise.on);
    this.setText("ttCruise", cruise.on ? `CRUISE ${this.speedText(cruise.target)}` : "CRUISE");
    this.setLamp("ttLim", limiter);
    this.setText("ttLim", limiter ? `LIM ${this.speedText(info.speedLimit)}` : "LIM");
    this.setLamp("ttWipe", lights.wipers !== "off");
    this.setText("ttWipe", lights.wipers === "off" ? "WIPE" : `WIPE ${lights.wipers === "intermittent" ? "INT" : lights.wipers.toUpperCase()}`);
    this.setText("lightSwitch", `LIGHTS ${lights.headlightSwitch.toUpperCase()}`);
    this.setText("transMode", pt.mode === "manual" ? "MANUAL" : "AUTO");
    ($("fuelBar") as HTMLElement).style.width = `${(pt.fuel / p.spec.fuelCapacity) * 100}%`;
    ($("dmgBar") as HTMLElement).style.width = `${Math.min(p.damage.body(), pt.health) * 100}%`;
    // Road panel.
    this.setText("roadName", info.onRoad ? info.roadName : "Off road");
    const lim = this.unit === "kmh" ? Math.round(info.speedLimit * 3.6) : Math.round((info.speedLimit * 2.23694) / 5) * 5;
    this.setText("limitValue", `${lim}`);
    this.setText("limitUnit", this.unit === "kmh" ? "km/h" : "mph");
    const over = p.absSpeed() > info.speedLimit * 1.1 + 0.3;
    $("limitSign").classList.toggle("over", over);
    const a = info.ahead;
    if (a !== undefined) this.miniSignal(a.head, a.control, a.camera, a.distance, a.nodeName);
    else $("nextJunction").hidden = true;
    // Status panel.
    const env = w.env;
    const hh = Math.floor(env.hour);
    const mm = Math.floor((env.hour - hh) * 60);
    this.setText("clock", `${String(hh).padStart(2, "0")}:${String(mm).padStart(2, "0")}`);
    this.setText("weather", env.weather === "clear" ? (env.dark ? "Clear night" : "Clear") : env.weather[0].toUpperCase() + env.weather.slice(1));
    const rec = w.record;
    this.setText("wallet", fmtMoney(rec.money));
    this.setText("points", `${rec.points}/${rec.maxPoints}`);
    $("points").classList.toggle("warn", rec.points >= 8);
    const heat = Math.floor(rec.heat);
    const stars = $("heat").children;
    for (let i = 0; i < stars.length; i++) stars[i].classList.toggle("on", i < heat);
    const stage = w.police.stage;
    this.setText("policeState", stage === "none" ? "" : stage === "responding" ? "Police responding" : stage === "pullover" ? "Pull over" : stage === "stopped" ? "Stopped by police" : stage === "pursuit" ? "Pursuit" : "Police searching");
    $("policeState").hidden = stage === "none";
    this.updateBanner(p, info.wrongWay);
    this.updateMoods(p);
    // Notes from the player's controls (cruise set, lights...).
    while (notes.length > 0) this.toast(notes.shift() as string, "info");
  }

  private updateBanner(p: Vehicle, wrongWay: boolean): void {
    const w = this.world;
    let text = "";
    let kind = "warn";
    const stage = w.police.stage;
    if (w.record.suspended()) text = "License suspended: 12 points";
    if (stage === "pullover") {
      text = "Police: pull over and stop";
      kind = "police";
    } else if (stage === "stopped") {
      text = `Officer writing your ticket… ${Math.max(0, Math.ceil(w.police.ticketCountdown))}`;
      kind = "police";
    } else if (stage === "pursuit") {
      text = "Evading police: break line of sight";
      kind = "danger";
    } else if (stage === "searching") {
      text = "Police searching: stay hidden";
      kind = "police";
    }
    if (p.wrecked) {
      text = "Vehicle wrecked: press ` to reset";
      kind = "danger";
    } else if (p.pt.stalled) text = "Stalled: press I to restart";
    else if (!p.pt.running && p.pt.fuel <= 0) text = "Out of fuel: press ` to call a tow";
    else if (wrongWay) {
      text = "Wrong way";
      kind = "danger";
    }
    if (text !== this.lastBanner) {
      this.lastBanner = text;
      const b = $("banner");
      b.textContent = text;
      b.hidden = text === "";
      b.dataset.kind = kind;
    }
  }

  private updateMoods(p: Vehicle): void {
    const list = this.world.angryAtPlayer().slice(0, 4);
    const box = $("moods");
    box.hidden = list.length === 0;
    const rows: string[] = [];
    for (const e of list) {
      const d = this.world.driverOf(e.vehicle);
      if (d === undefined) continue;
      const dist = Math.round(Math.hypot(e.vehicle.x - p.x, e.vehicle.y - p.y));
      const st = d.rage !== undefined && d.rage.target === p.id ? (d.rage.behavior === "ram" ? "Ramming you" : d.rage.behavior === "brake_check" ? "Brake-checking" : d.rage.behavior === "chase" ? "Chasing you" : "Tailgating you") : MOOD_LABEL[d.emo.mood];
      rows.push(`<div class="mood-row mood-${d.emo.mood}"><div class="mood-head"><span>${d.p.label}</span><span class="mood-dist">${dist} m</span></div><div class="mood-bar"><i style="width:${Math.round(e.anger)}%"></i></div><div class="mood-state">${st}</div></div>`);
    }
    const html = `<div class="panel-label">Drivers angry at you</div>${rows.join("")}`;
    if (box.innerHTML !== html) box.innerHTML = html;
  }

  private drawCluster(p: Vehicle, gearText: string): void {
    const c = this.cluster;
    const dpr = Math.min(window.devicePixelRatio > 0 ? window.devicePixelRatio : 1, 2);
    const cw = c.clientWidth;
    const ch = c.clientHeight;
    if (c.width !== Math.round(cw * dpr)) {
      c.width = Math.round(cw * dpr);
      c.height = Math.round(ch * dpr);
    }
    const g = this.cctx;
    g.setTransform(dpr, 0, 0, dpr, 0, 0);
    g.clearRect(0, 0, cw, ch);
    const spec = p.spec;
    const pt = p.pt;
    const cx = cw / 2;
    const cy = ch * 0.62;
    const R = Math.min(cw * 0.42, ch * 0.56);
    const maxRpm = Math.ceil((spec.redline + 600) / 1000) * 1000;
    const a0 = Math.PI * 0.8;
    const a1 = Math.PI * 2.2;
    const ang = (rpm: number) => a0 + (a1 - a0) * Math.min(1, rpm / maxRpm);
    // Track.
    g.lineWidth = 7;
    g.lineCap = "butt";
    g.strokeStyle = "rgba(233,230,220,0.12)";
    g.beginPath();
    g.arc(cx, cy, R, a0, a1);
    g.stroke();
    g.strokeStyle = "rgba(255,77,61,0.75)";
    g.beginPath();
    g.arc(cx, cy, R, ang(spec.redline), a1);
    g.stroke();
    // Ticks.
    g.fillStyle = "rgba(233,230,220,0.75)";
    g.font = "600 10px 'Overpass Mono', monospace";
    g.textAlign = "center";
    g.textBaseline = "middle";
    for (let r = 0; r <= maxRpm; r += 1000) {
      const a = ang(r);
      g.strokeStyle = r >= spec.redline ? "#ff4d3d" : "rgba(233,230,220,0.6)";
      g.lineWidth = 2;
      g.beginPath();
      g.moveTo(cx + Math.cos(a) * (R - 10), cy + Math.sin(a) * (R - 10));
      g.lineTo(cx + Math.cos(a) * (R - 2), cy + Math.sin(a) * (R - 2));
      g.stroke();
      g.fillText(`${r / 1000}`, cx + Math.cos(a) * (R - 20), cy + Math.sin(a) * (R - 20));
    }
    // Rpm fill.
    const rpm = pt.running || pt.cranking > 0 ? pt.rpm : 0;
    g.lineWidth = 7;
    g.strokeStyle = rpm > spec.redline * 0.92 ? "#ff4d3d" : "#ffb35c";
    g.beginPath();
    g.arc(cx, cy, R, a0, ang(rpm));
    g.stroke();
    // Speed digits.
    g.fillStyle = "#e9e6dc";
    g.font = `800 ${Math.round(R * 0.62)}px 'Overpass Mono', monospace`;
    g.fillText(this.speedText(p.absSpeed()), cx, cy - R * 0.08);
    g.font = "600 11px 'Overpass', sans-serif";
    g.fillStyle = "rgba(233,230,220,0.6)";
    g.fillText(this.unit === "kmh" ? "KM/H" : "MPH", cx, cy + R * 0.3);
    // Gear.
    g.font = `800 ${Math.round(R * 0.34)}px 'Overpass Mono', monospace`;
    g.fillStyle = gearText === "R" ? "#ff4d3d" : gearText === "P" || gearText === "N" ? "#9a978f" : "#ffb35c";
    g.fillText(gearText, cx, cy + R * 0.62);
    g.font = "600 9px 'Overpass Mono', monospace";
    g.fillStyle = "rgba(233,230,220,0.5)";
    g.fillText("RPM ×1000", cx, cy - R * 0.58);
  }

  private drawMinimap(p: Vehicle): void {
    const c = this.mini;
    const dpr = Math.min(window.devicePixelRatio > 0 ? window.devicePixelRatio : 1, 2);
    const size = c.clientWidth;
    if (c.width !== Math.round(size * dpr)) {
      c.width = Math.round(size * dpr);
      c.height = Math.round(c.clientHeight * dpr);
    }
    const g = this.mctx;
    const W = c.clientWidth;
    const H = c.clientHeight;
    g.setTransform(dpr, 0, 0, dpr, 0, 0);
    g.clearRect(0, 0, W, H);
    const net = this.world.net;
    const img = this.mapImage;
    if (img === undefined) return;
    const pad = this.mapPad;
    const sc = this.mapScale;
    const big = this.bigMap;
    const zoom = big ? Math.min(W / (img.width / sc), H / (img.height / sc)) : 1.25;
    g.save();
    if (!big) {
      g.beginPath();
      g.arc(W / 2, H / 2, W / 2, 0, Math.PI * 2);
      g.clip();
    }
    g.fillStyle = "#101116";
    g.fillRect(0, 0, W, H);
    const toMap = (x: number, y: number) => ({ x: x - net.minX + pad, y: net.maxY + pad - y });
    const pm = toMap(p.x, p.y);
    g.translate(W / 2, H / 2);
    if (!big) g.rotate(p.heading - Math.PI / 2);
    g.scale(zoom, zoom);
    if (big) g.translate(-(img.width / sc) / 2, -(img.height / sc) / 2);
    else g.translate(-pm.x, -pm.y);
    g.drawImage(img, 0, 0, img.width / sc, img.height / sc);
    // Signals.
    for (const s of this.world.signals) {
      const n = s.node;
      const m = toMap(n.x, n.y);
      const st = s.phaseState();
      g.fillStyle = s.mode === "flashing" ? "#ffc233" : s.mode === "off" ? "#555" : st === "green" ? "#36e39a" : st === "yellow" ? "#ffc233" : "#ff4d3d";
      g.beginPath();
      g.arc(m.x, m.y, big ? 3 : 2.4 / zoom * 1.5, 0, Math.PI * 2);
      g.fill();
    }
    // Vehicles.
    const angry = new Set(this.world.angryAtPlayer().map((e) => e.vehicle.id));
    for (const v of this.world.vehicles) {
      if (v === p) continue;
      const m = toMap(v.x, v.y);
      const r = (big ? 2.2 : 2) / (big ? 1 : zoom) * (big ? 1 : 1.2);
      if (v.role === "police") g.fillStyle = v.lights.lightbar ? (v.lights.lightbarPhase() === 0 ? "#ff4d3d" : "#4d8bff") : "#4d8bff";
      else if (v.role === "ambulance") g.fillStyle = "#ffffff";
      else if (angry.has(v.id)) g.fillStyle = "#ff2a6d";
      else g.fillStyle = "rgba(233,230,220,0.55)";
      g.beginPath();
      g.arc(m.x, m.y, r, 0, Math.PI * 2);
      g.fill();
    }
    // Player arrow.
    g.save();
    g.translate(pm.x, pm.y);
    g.rotate(-p.heading + Math.PI / 2);
    const s = (big ? 7 : 6) / zoom;
    g.fillStyle = "#ffb35c";
    g.strokeStyle = "#0e0f14";
    g.lineWidth = 1 / zoom;
    g.beginPath();
    g.moveTo(0, -s * 1.4);
    g.lineTo(s, s);
    g.lineTo(0, s * 0.5);
    g.lineTo(-s, s);
    g.closePath();
    g.fill();
    g.stroke();
    g.restore();
    g.restore();
    if (!big) {
      g.strokeStyle = "rgba(233,230,220,0.25)";
      g.lineWidth = 1.5;
      g.beginPath();
      g.arc(W / 2, H / 2, W / 2 - 1, 0, Math.PI * 2);
      g.stroke();
      // North marker.
      const na = -(p.heading - Math.PI / 2) - Math.PI / 2;
      g.fillStyle = "#e9e6dc";
      g.font = "700 10px 'Overpass', sans-serif";
      g.textAlign = "center";
      g.fillText("N", W / 2 + Math.cos(na) * (W / 2 - 10), H / 2 + Math.sin(na) * (W / 2 - 10) + 3);
    }
  }

  private project(camera: THREE.PerspectiveCamera, x: number, y: number, h: number): { x: number; y: number; visible: boolean } {
    const v = new THREE.Vector3(x, h, -y).project(camera);
    return { x: (v.x * 0.5 + 0.5) * window.innerWidth, y: (-v.y * 0.5 + 0.5) * window.innerHeight, visible: v.z < 1 && v.z > -1 };
  }

  private updateBubbles(dt: number, camera: THREE.PerspectiveCamera): void {
    for (let i = this.bubbles.length - 1; i >= 0; i--) {
      const b = this.bubbles[i];
      b.ttl -= dt;
      const v = this.world.vehicle(b.vehicle);
      if (b.ttl <= 0 || v === undefined) {
        b.el.remove();
        this.bubbles.splice(i, 1);
        continue;
      }
      const s = this.project(camera, v.x, v.y, v.spec.height + 1.6);
      b.el.style.transform = `translate(${s.x}px, ${s.y}px) translate(-50%, -100%)`;
      b.el.style.opacity = s.visible ? `${Math.min(1, b.ttl * 2)}` : "0";
    }
  }

  setDebug(on: boolean): void {
    this.debug = on;
    this.debugLayer.hidden = !on;
    if (!on) {
      this.debugLabels.forEach((el) => el.remove());
      this.debugLabels.clear();
    }
  }

  private updateDebug(camera: THREE.PerspectiveCamera): void {
    const p = this.world.player;
    if (p === undefined) return;
    const seen = new Set<number>();
    for (const v of this.world.vehicles) {
      if (v === p || Math.hypot(v.x - p.x, v.y - p.y) > 70) continue;
      const d = this.world.driverOf(v);
      if (d === undefined) continue;
      const s = this.project(camera, v.x, v.y, v.spec.height + 0.6);
      if (!s.visible) continue;
      seen.add(v.id);
      let el = this.debugLabels.get(v.id);
      if (el === undefined) {
        el = document.createElement("div");
        el.className = "dbg";
        this.debugLayer.appendChild(el);
        this.debugLabels.set(v.id, el);
      }
      el.textContent = `#${v.id} ${d.p.label} · ${d.emo.mood} ${Math.round(d.emo.anger)} · ${d.status()} · ${Math.round(v.absSpeed() * 3.6)} km/h`;
      el.style.transform = `translate(${s.x}px, ${s.y}px) translate(-50%, -100%)`;
    }
    this.debugLabels.forEach((el, id) => {
      if (!seen.has(id)) {
        el.remove();
        this.debugLabels.delete(id);
      }
    });
  }
}
