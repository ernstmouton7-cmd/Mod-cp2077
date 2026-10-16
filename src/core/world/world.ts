import { AIDriver, hornVictims } from "../ai/driver";
import { OffenseKind } from "../ai/emotion";
import { APOLOGY_ACCEPTED, APOLOGY_REJECTED } from "../ai/speech";
import { DrivingRecord, VIOLATIONS, ViolationKind, Witness } from "../law/violations";
import { Observation, PlayerMonitor } from "../law/playerMonitor";
import { PoliceSystem } from "../law/police";
import { Rng } from "../math/rng";
import { angleDiff, clamp, DEG, hypot, mod } from "../math/scalar";
import { Box, boxContact, Contact, resolveBodies, resolveStatic } from "../physics/collision";
import { InputMapper } from "../player/actions";
import { ActionId } from "../player/actions";
import { PlayerDriver } from "../player/playerDriver";
import { buildCity, City, CityOptions } from "../road/cityBuilder";
import { addOccupant, Connector, Lane, LaneHit, Network } from "../road/network";
import { Router } from "../road/router";
import { Segment } from "../road/types";
import { DemandFn, SignalController } from "../signals/signalController";
import { EventBus } from "../util/events";
import { SpatialHash } from "../util/spatialHash";
import { SPECS, VehicleClass } from "../vehicle/specs";
import { Vehicle } from "../vehicle/vehicle";
import { AllWayQueue, SimContext } from "./context";
import { Environment } from "./environment";
import { SimEvent } from "./events";
import { Spawner } from "./spawner";
import { Pedestrian, PedestrianSystem, PedHost } from "../ai/pedestrians";
import { buildCrosswalks, Crosswalk, CrosswalkCrossing } from "../road/crosswalks";

export interface WorldOptions {
  seed?: number;
  city?: CityOptions;
  /** Civilian vehicles at peak density. */
  traffic?: number;
  police?: number;
  startHour?: number;
  playerVehicle?: VehicleClass;
  /** Fixed simulation step (s). */
  fixedDt?: number;
  /** Disable the player entirely (pure traffic simulation). */
  noPlayer?: boolean;
}

/** The player's session statistics (driver report). */
export interface PlayerStats {
  distance: number;
  driveTime: number;
  maxSpeed: number;
  collisions: number;
  nearMisses: number;
  /** Drivers the player made at least irritated. */
  angered: number;
  apologiesAccepted: number;
  rageEpisodes: number;
  honks: number;
  pedestriansHit: number;
  redLightsRun: number;
}

export interface WorldStats {
  collisions: number;
  rageEpisodes: number;
  horns: number;
  redLightRunners: number;
  spawned: number;
}

const contact: Contact = { nx: 0, ny: 0, depth: 0, px: 0, py: 0 };
const boxA: Box = { x: 0, y: 0, heading: 0, hl: 0, hw: 0 };
const boxB: Box = { x: 0, y: 0, heading: 0, hl: 0, hw: 0 };

/**
 * The whole simulation. Hosts call update(realDt) every frame (it runs fixed
 * steps internally), feed input through `input`, and read vehicles, signals,
 * HUD state and drained events for rendering/audio.
 */
export class World implements SimContext {
  time = 0;
  readonly city: City;
  readonly net: Network;
  readonly router: Router;
  readonly signals: SignalController[] = [];
  readonly vehicles: Vehicle[] = [];
  readonly events = new EventBus<SimEvent>();
  readonly rng: Rng;
  readonly env: Environment;
  player: Vehicle | undefined = undefined;
  playerDriver: PlayerDriver | undefined = undefined;
  readonly input = new InputMapper();
  readonly monitor = new PlayerMonitor();
  readonly record = new DrivingRecord();
  readonly police = new PoliceSystem();
  readonly spawner: Spawner;
  readonly stats: WorldStats = { collisions: 0, rageEpisodes: 0, horns: 0, redLightRunners: 0, spawned: 0 };
  readonly playerStats: PlayerStats = { distance: 0, driveTime: 0, maxSpeed: 0, collisions: 0, nearMisses: 0, angered: 0, apologiesAccepted: 0, rageEpisodes: 0, honks: 0, pedestriansHit: 0, redLightsRun: 0 };
  private angeredIds: Map<number, boolean> = new Map();
  readonly fixedDt: number;
  /** Simulation speed multiplier (pause = 0). */
  timeScale = 1;
  /** Host-side actions pressed since the host last drained them. */
  readonly hostActions: ActionId[] = [];
  /** Static obstacles (buildings, boundary walls). */
  readonly obstacles: Box[] = [];
  readonly crosswalks: Crosswalk[];
  readonly crossings: Map<number, CrosswalkCrossing[]>;
  readonly peds: PedestrianSystem;
  /** Base pedestrian count at daytime peak. */
  pedestrianDensity = 45;

  private readonly drivers: Map<number, AIDriver> = new Map();
  private readonly byId: Map<number, Vehicle> = new Map();
  private readonly allWayQueues: Map<number, AllWayQueue> = new Map();
  private readonly obstacleHash = new SpatialHash<Box>(30);
  private vehicleHash = new SpatialHash<Vehicle>(12);
  private idCounter = 1;
  private accumulator = 0;
  private laneHits: LaneHit[] = [];
  private connHits: Connector[] = [];
  private queryBuf: Vehicle[] = [];
  private obstacleBuf: Box[] = [];
  private flashMemo: Map<number, number> = new Map();
  private ambulance: Vehicle | undefined = undefined;
  private ambulanceTarget: { x: number; y: number; t: number } | undefined = undefined;
  private playerTailgate: Map<number, number> = new Map();
  private nearMissMemo: Map<number, number> = new Map();
  private nightFlashing = false;
  private cleanupTimer = 0;
  /** Detector demand for actuated signals (created once: no per-step closures). */
  private readonly demandFn: DemandFn = (c: Connector): number => {
    const lane = c.from;
    let n = 0;
    for (let i = 0; i < lane.occCount; i++) {
      const e = lane.occ[i];
      if (e.s < lane.length - 60) continue;
      if (e.occ.intentConnector === c.id || (e.phantom && e.occ.intentConnector === -1)) n++;
    }
    for (let i = 0; i < c.occCount; i++) if (c.occ[i].v < 2) n++;
    return n;
  };

  constructor(opts: WorldOptions = {}) {
    const seed = opts.seed ?? 2077;
    this.rng = new Rng(seed);
    this.fixedDt = opts.fixedDt ?? 1 / 60;
    this.city = buildCity({ seed, ...(opts.city ?? {}) });
    this.net = this.city.network;
    this.router = new Router(this.net);
    this.env = new Environment(seed + 1, opts.startHour ?? 19.5);
    this.buildSignals();
    this.buildObstacles();
    const cw = buildCrosswalks(this.net);
    this.crosswalks = cw.crosswalks;
    this.crossings = cw.crossings;
    this.peds = new PedestrianSystem(this.crosswalks, this.rng.fork());
    const host = {
      rng: this.rng,
      vehicles: this.vehicles,
      lanes: this.net.lanes,
      playerPos: () => (this.player !== undefined ? { x: this.player.x, y: this.player.y } : undefined),
      add: (v: Vehicle, d: AIDriver) => this.addVehicle(v, d),
      remove: (v: Vehicle) => this.removeVehicle(v),
      nextId: () => this.idCounter++,
      driverOf: (v: Vehicle) => this.drivers.get(v.id),
      laneFree: (lane: Lane, s: number, clear: number) => this.laneFree(lane, s, clear),
      busyUnit: (v: Vehicle) => this.police.units.indexOf(v) >= 0 || v === this.ambulance,
    };
    this.spawner = new Spawner(host, { civilians: opts.traffic ?? 110, police: opts.police ?? 4, minDistance: 90 });
    if (opts.noPlayer !== true) this.spawnPlayer(opts.playerVehicle ?? "sports");
    this.spawner.populate(this.env.trafficDensity);
    this.police.spawnUnit = (x, y) => {
      const spot = this.spawner.findSpot(120, 400);
      void x;
      void y;
      if (spot === undefined) return undefined;
      return this.spawner.create("police", "police", spot.lane, spot.s, 8);
    };
    // Initial lane occupancy for the first frame.
    this.rebuildOccupancy();
    this.events.onAny((e) => this.trackPlayer(e));
  }

  // ----------------------------------------------------------- setup

  private buildSignals(): void {
    let coord = 0;
    for (const n of this.net.nodes) {
      if (n.control !== "signal") continue;
      // Coordinate along the x axis for a westbound/eastbound green wave.
      const offset = Math.max(0, (n.x - this.net.minX) / 13.9);
      const mode = this.rng.chance(0.75) ? "actuated" : "fixed";
      const sig = new SignalController(this.signals.length, n, { mode, offset: mode === "fixed" ? offset : this.rng.range(0, 30), allowRightOnRed: this.rng.chance(0.85) });
      sig.onChange = (c) => this.events.emit({ type: "signal", node: c.nodeId, phase: c.phase, state: c.state });
      n.signalId = this.signals.length;
      this.signals.push(sig);
      coord++;
    }
    void coord;
  }

  private buildObstacles(): void {
    for (const b of this.city.buildings) {
      if (b.kind === "park" || b.kind === "plaza") continue;
      const box: Box = { x: b.x, y: b.y, heading: 0, hl: b.w / 2, hw: b.h / 2 };
      this.obstacles.push(box);
    }
    // Boundary walls outside the perimeter road.
    const pad = 16;
    const x0 = this.net.minX - pad;
    const x1 = this.net.maxX + pad;
    const y0 = this.net.minY - pad;
    const y1 = this.net.maxY + pad;
    const cx = (x0 + x1) / 2;
    const cy = (y0 + y1) / 2;
    const t = 3;
    this.obstacles.push({ x: cx, y: y0 - t, heading: 0, hl: (x1 - x0) / 2 + t * 2, hw: t });
    this.obstacles.push({ x: cx, y: y1 + t, heading: 0, hl: (x1 - x0) / 2 + t * 2, hw: t });
    this.obstacles.push({ x: x0 - t, y: cy, heading: 0, hl: t, hw: (y1 - y0) / 2 + t * 2 });
    this.obstacles.push({ x: x1 + t, y: cy, heading: 0, hl: t, hw: (y1 - y0) / 2 + t * 2 });
    for (const o of this.obstacles) this.obstacleHash.insert(o, o.x, o.y, Math.max(o.hl, o.hw));
  }

  private spawnPlayer(cls: VehicleClass): void {
    // Start on a long lane near the centre.
    let best: Lane | undefined = undefined;
    let bd = Infinity;
    for (const l of this.net.lanes) {
      if (l.length < 60 || l.right !== undefined) continue;
      const pt = { x: 0, y: 0, heading: 0 };
      l.path.sample(l.length * 0.3, pt);
      const d = pt.x * pt.x + pt.y * pt.y;
      if (d < bd) {
        bd = d;
        best = l;
      }
    }
    const lane = best ?? this.net.lanes[0];
    const v = new Vehicle(this.idCounter++, SPECS[cls], 0xff2a6d, "player");
    const pt = { x: 0, y: 0, heading: 0 };
    lane.path.sample(lane.length * 0.3, pt);
    v.x = pt.x;
    v.y = pt.y;
    v.heading = pt.heading;
    v.dyn.reset(pt.x, pt.y, pt.heading, 0);
    v.mode = "physics";
    v.lights.headlightSwitch = "auto";
    this.player = v;
    this.playerDriver = new PlayerDriver(v);
    this.vehicles.push(v);
    this.byId.set(v.id, v);
  }

  /** Swap the player's car for another class (keeps position). */
  setPlayerVehicle(cls: VehicleClass): void {
    const old = this.player;
    if (old === undefined) return;
    const v = new Vehicle(this.idCounter++, SPECS[cls], old.color, "player");
    v.dyn.reset(old.x, old.y, old.heading, 0);
    v.x = old.x;
    v.y = old.y;
    v.heading = old.heading;
    v.mode = "physics";
    v.lights.headlightSwitch = old.lights.headlightSwitch;
    this.removeVehicle(old);
    this.player = v;
    const pd = new PlayerDriver(v);
    if (this.playerDriver !== undefined) {
      pd.seatbelt = this.playerDriver.seatbelt;
      pd.settings.clutchAssist = this.playerDriver.settings.clutchAssist;
    }
    this.playerDriver = pd;
    this.vehicles.push(v);
    this.byId.set(v.id, v);
  }

  private addVehicle(v: Vehicle, d: AIDriver): void {
    this.vehicles.push(v);
    this.byId.set(v.id, v);
    this.drivers.set(v.id, d);
    const lane = v.seg as Lane;
    d.newDestination(this, lane);
    this.stats.spawned++;
    this.events.emit({ type: "spawn", vehicle: v.id });
  }

  private removeVehicle(v: Vehicle): void {
    const i = this.vehicles.indexOf(v);
    if (i >= 0) this.vehicles.splice(i, 1);
    this.byId.delete(v.id);
    this.drivers.delete(v.id);
    v.alive = false;
    this.allWayQueues.forEach((q) => q.arrivals.delete(v.id));
    this.events.emit({ type: "despawn", vehicle: v.id });
  }

  private laneFree(lane: Lane, s: number, clear: number): boolean {
    for (let i = 0; i < lane.occCount; i++) if (Math.abs(lane.occ[i].s - s) < clear) return false;
    for (const v of this.vehicles) {
      if (v.seg === lane && Math.abs(v.s - s) < clear) return false;
    }
    return true;
  }

  private trackPlayer(e: SimEvent): void {
    const p = this.player;
    if (p === undefined) return;
    const st = this.playerStats;
    if (e.type === "collision" && (e.a === p.id || e.b === p.id) && e.severity > 0.8) st.collisions++;
    else if (e.type === "near_miss" && e.a === p.id) st.nearMisses++;
    else if (e.type === "offense" && e.offender === p.id && e.anger > 22 && this.angeredIds.get(e.victim) !== true) {
      this.angeredIds.set(e.victim, true);
      st.angered++;
    } else if (e.type === "apology" && e.calmed > 5) st.apologiesAccepted++;
    else if (e.type === "rage_start" && e.target === p.id) st.rageEpisodes++;
    else if (e.type === "horn" && e.vehicle === p.id && !e.long) st.honks++;
    else if (e.type === "ped_hit" && e.vehicle === p.id) st.pedestriansHit++;
    else if (e.type === "violation" && e.kind === "red_light") st.redLightsRun++;
  }

  // ----------------------------------------------------------- SimContext

  vehicle(id: number): Vehicle | undefined {
    return this.byId.get(id);
  }

  driverOf(v: Vehicle): AIDriver | undefined {
    return this.drivers.get(v.id);
  }

  allWay(nodeId: number): AllWayQueue {
    let q = this.allWayQueues.get(nodeId);
    if (q === undefined) {
      q = { arrivals: new Map() };
      this.allWayQueues.set(nodeId, q);
    }
    return q;
  }

  policeNear(x: number, y: number, range: number): Vehicle[] {
    const out: Vehicle[] = [];
    for (const v of this.vehicles) {
      if (v.role !== "police" || !v.alive || v.wrecked) continue;
      const dx = v.x - x;
      const dy = v.y - y;
      if (dx * dx + dy * dy > range * range) continue;
      if (this.los(v.x, v.y, x, y)) out.push(v);
    }
    return out;
  }

  los(ax: number, ay: number, bx: number, by: number): boolean {
    const mx = (ax + bx) / 2;
    const my = (ay + by) / 2;
    const r = hypot(bx - ax, by - ay) / 2 + 1;
    this.obstacleBuf = [];
    this.obstacleHash.query(mx, my, r, this.obstacleBuf);
    for (const o of this.obstacleBuf) {
      // Slab test against the axis-aligned box (shrunk slightly).
      const minX = o.x - o.hl + 0.5;
      const maxX = o.x + o.hl - 0.5;
      const minY = o.y - o.hw + 0.5;
      const maxY = o.y + o.hw - 0.5;
      let t0 = 0;
      let t1 = 1;
      const dx = bx - ax;
      const dy = by - ay;
      let hit = true;
      if (Math.abs(dx) < 1e-9) {
        if (ax < minX || ax > maxX) hit = false;
      } else {
        let ta = (minX - ax) / dx;
        let tb = (maxX - ax) / dx;
        if (ta > tb) {
          const t = ta;
          ta = tb;
          tb = t;
        }
        t0 = Math.max(t0, ta);
        t1 = Math.min(t1, tb);
        if (t0 > t1) hit = false;
      }
      if (hit) {
        if (Math.abs(dy) < 1e-9) {
          if (ay < minY || ay > maxY) hit = false;
        } else {
          let ta = (minY - ay) / dy;
          let tb = (maxY - ay) / dy;
          if (ta > tb) {
            const t = ta;
            ta = tb;
            tb = t;
          }
          t0 = Math.max(t0, ta);
          t1 = Math.min(t1, tb);
          if (t0 > t1) hit = false;
        }
      }
      if (hit) return false;
    }
    return true;
  }

  offend(offender: Vehicle, victim: Vehicle, kind: OffenseKind, scale: number): void {
    if (offender === victim) return;
    const d = this.drivers.get(victim.id);
    if (d === undefined) return;
    const gain = d.emo.offend(offender.id, kind, scale, this.time);
    const continuous = kind === "tailgating" || kind === "blocked" || kind === "slow_driver" || kind === "long_wait";
    if (!continuous && gain > 0.5) {
      this.events.emit({ type: "offense", offender: offender.id, victim: victim.id, kind, anger: d.emo.anger });
      d.onOffended(this, offender, kind);
    }
  }

  // ----------------------------------------------------------- main loop

  /** Advance by real time; returns the number of fixed steps taken. */
  update(realDt: number): number {
    this.accumulator += Math.min(realDt, 0.25) * this.timeScale;
    let steps = 0;
    while (this.accumulator >= this.fixedDt && steps < 8) {
      this.step(this.fixedDt);
      this.accumulator -= this.fixedDt;
      steps++;
    }
    if (steps === 8) this.accumulator = 0;
    return steps;
  }

  /** Interpolation alpha between the last two steps (for smooth rendering). */
  alpha(): number {
    return this.accumulator / this.fixedDt;
  }

  step(dt: number): void {
    this.time += dt;
    this.env.update(dt);
    this.updateSignals(dt);
    this.rebuildOccupancy();
    this.updatePlayer(dt);
    // AI.
    for (let i = 0; i < this.vehicles.length; i++) {
      const v = this.vehicles[i];
      const d = this.drivers.get(v.id);
      if (d === undefined) continue;
      d.update(dt, this);
      if (d.emo.mood !== d.lastMood) {
        this.events.emit({ type: "mood", vehicle: v.id, mood: d.emo.mood, target: d.emo.target });
        d.lastMood = d.emo.mood;
      }
    }
    this.updatePedestrians(dt);
    this.physics(dt);
    this.collisions();
    this.lawAndOrder(dt);
    this.ambulances(dt);
    this.spawner.update(dt, this.env.trafficDensity);
    // Stale all-way stop arrivals (every 2 s).
    this.cleanupTimer -= dt;
    if (this.cleanupTimer > 0) return;
    this.cleanupTimer = 2;
    this.allWayQueues.forEach((q) => {
      q.arrivals.forEach((t, id) => {
        if (this.time - t > 60 || !this.byId.has(id)) q.arrivals.delete(id);
      });
    });
  }

  private updateSignals(dt: number): void {
    const flash = this.env.nightFlash;
    if (flash !== this.nightFlashing) {
      this.nightFlashing = flash;
      for (const s of this.signals) {
        // Minor junctions go to flashing operation at night.
        const minor = s.node.roads.every((r) => r.kind !== "avenue" && r.kind !== "ring");
        if (minor) s.setMode(flash ? "flashing" : "actuated");
      }
    }
    for (const s of this.signals) s.update(dt, this.demandFn);
  }

  /** Register every vehicle on the segments it occupies (lanes, connectors, shadows). */
  rebuildOccupancy(): void {
    this.net.clearOccupancy();
    for (let i = 0; i < this.vehicles.length; i++) {
      const v = this.vehicles[i];
      if (!v.alive) continue;
      if (v.mode === "lane" && v.seg !== undefined) {
        const seg = v.seg;
        addOccupant(seg, v, v.s, v.speed, false, v.lat);
        // Tail still on the previous segment.
        const prev = v.prevSeg;
        if (prev !== undefined && v.s < v.halfLen + 0.5) addOccupant(prev, v, prev.length + v.s, v.speed, false, v.lat);
        // Lane change shadow.
        const d = this.drivers.get(v.id);
        if (d !== undefined && d.shadowLane !== undefined && seg.kind === "lane") {
          const sh = d.shadowLane;
          const sS = (v.s * sh.length) / seg.length;
          const offset = v.lat > 0 ? v.lat - (sh.width + (seg as Lane).width) / 2 : v.lat + (sh.width + (seg as Lane).width) / 2;
          addOccupant(sh, v, sS, v.speed, true, offset);
        }
      } else {
        this.registerFreeBody(v);
      }
    }
  }

  private registerFreeBody(v: Vehicle): void {
    this.laneHits = [];
    this.net.nearestLanes(v.x, v.y, 7, this.laneHits);
    const ch = Math.cos(v.heading);
    const sh = Math.sin(v.heading);
    // Effective half-extent across a lane depends on the body's orientation.
    for (const h of this.laneHits) {
      const lh = h.lane.path.headingAt(h.s);
      const rel = Math.abs(Math.sin(v.heading - lh));
      const halfAcross = v.halfWid * (1 - rel) + v.halfLen * rel;
      if (Math.abs(h.lateral) > h.lane.width / 2 + halfAcross - 0.3) continue;
      if (h.s < -v.halfLen || h.s > h.lane.length + v.halfLen) continue;
      const vAlong = v.vx() * Math.cos(lh) + v.vy() * Math.sin(lh);
      addOccupant(h.lane, v, h.s, vAlong, true, h.lateral);
    }
    this.connHits = [];
    this.net.nearestConnectors(v.x, v.y, 10, this.connHits);
    const pr = { s: 0, lateral: 0, d2: 0 };
    for (const c of this.connHits) {
      c.path.project(v.x, v.y, pr);
      if (pr.d2 > 2.6 * 2.6) continue;
      if (pr.s <= 0.01 && pr.d2 > 1) continue;
      const lh = c.path.headingAt(pr.s);
      const vAlong = v.vx() * Math.cos(lh) + v.vy() * Math.sin(lh);
      addOccupant(c, v, pr.s, vAlong, true, pr.lateral);
    }
    void ch;
    void sh;
  }

  private updatePlayer(dt: number): void {
    const p = this.player;
    const pd = this.playerDriver;
    if (p === undefined || pd === undefined) return;
    const frame = this.input.frame(dt, p.dyn.u);
    pd.update(dt, frame, this, this.monitor.info.speedLimit);
    for (const a of pd.hostActions) this.hostActions.push(a);
    pd.hostActions.splice(0, pd.hostActions.length);
    if (frame.pressed.indexOf("reset_vehicle") >= 0) this.resetPlayer();
    if (frame.pressed.indexOf("time_forward") >= 0) this.env.hour = mod(this.env.hour + 1, 24);
    if (frame.pressed.indexOf("weather_cycle") >= 0) {
      const order = ["clear", "rain", "storm", "fog", "cloudy"] as const;
      const i = order.indexOf(this.env.weather as (typeof order)[number]);
      this.env.setWeather(order[mod(i + 1, order.length)]);
    }
    this.monitor.seatbelt = pd.seatbelt;
    // Intent for AI yielding: from the indicator and current lane.
    const lane = this.monitor.info.lane;
    p.intentConnector = -1;
    if (lane !== undefined) {
      const want = p.lights.indicator === "left" ? "left" : p.lights.indicator === "right" ? "right" : "straight";
      for (const c of lane.outgoing) if (c.turn === want) p.intentConnector = c.id;
    }
    // Social actions.
    if (pd.gesture !== undefined) this.gesture(p, pd.gesture);
    if (frame.pressed.indexOf("hazards") >= 0 && p.lights.hazard) this.thankYouBlink(p);
    if (frame.pressed.indexOf("horn") >= 0) {
      this.stats.horns++;
      hornVictims(this, p, false);
    }
    for (const r of frame.released) if (r.id === "horn" && r.held > 0.9) hornVictims(this, p, true);
    if (frame.pressed.indexOf("flash") >= 0) this.flashAhead(p);
  }

  private gesture(p: Vehicle, kind: "wave" | "rude"): void {
    this.events.emit({ type: "gesture", from: p.id, kind });
    for (const v of this.vehicles) {
      const d = this.drivers.get(v.id);
      if (d === undefined) continue;
      const dist = hypot(v.x - p.x, v.y - p.y);
      if (kind === "wave" && dist < 45) {
        if (d.emo.grudgeAgainst(p.id) <= 0) continue;
        const calmed = d.emo.apologize(p.id, this.time);
        this.events.emit({ type: "apology", from: p.id, to: v.id, calmed });
        d.speechCooldown = 0;
        d.say(this, d.emo.anger < 35 ? this.rng.pick(APOLOGY_ACCEPTED) : this.rng.pick(APOLOGY_REJECTED));
      } else if (kind === "rude" && dist < 30) {
        if (d.p.archetype === "zen") {
          d.speechCooldown = 0;
          d.say(this, "Have a nice day!");
          continue;
        }
        this.offend(p, v, "rude_gesture", 1);
      }
    }
  }

  /** Hazards blink after merging: a softer apology to drivers behind. */
  private thankYouBlink(p: Vehicle): void {
    for (const v of this.vehicles) {
      const d = this.drivers.get(v.id);
      if (d === undefined || d.emo.grudgeAgainst(p.id) <= 0) continue;
      const dx = v.x - p.x;
      const dy = v.y - p.y;
      const behind = dx * Math.cos(p.heading) + dy * Math.sin(p.heading) < 0;
      if (!behind || dx * dx + dy * dy > 50 * 50) continue;
      const calmed = d.emo.apologize(p.id, this.time) * 0.6;
      this.events.emit({ type: "apology", from: p.id, to: v.id, calmed });
    }
  }

  private flashAhead(p: Vehicle): void {
    for (const v of this.vehicles) {
      if (v === p) continue;
      const dx = v.x - p.x;
      const dy = v.y - p.y;
      const fwd = dx * Math.cos(p.heading) + dy * Math.sin(p.heading);
      if (fwd < 0 || fwd > 45) continue;
      const side = Math.abs(-dx * Math.sin(p.heading) + dy * Math.cos(p.heading));
      if (side > 3 + fwd * 0.15) continue;
      // Oncoming drivers get dazzled, drivers ahead feel pushed.
      const last = this.flashMemo.get(v.id) ?? -10;
      if (this.time - last < 1.5) continue;
      this.flashMemo.set(v.id, this.time);
      this.offend(p, v, "high_beams", 1);
    }
  }

  private physics(dt: number): void {
    const grip = this.env.grip;
    for (let i = 0; i < this.vehicles.length; i++) {
      const v = this.vehicles[i];
      if (v.mode !== "physics") continue;
      const pt = v.pt;
      const dyn = v.dyn;
      const isPlayer = v.isPlayer;
      if (v.wrecked) {
        pt.running = false;
        v.inputs.throttle = 0;
      }
      // Traction/stability control cut torque, not the pedal (keeps shift logic stable).
      const cut = (dyn.tcsActive ? 0.4 : 1) * (dyn.escActive ? 0.6 : 1);
      const torque = pt.update(dt, dyn.drivenOmega(), v.inputs.throttle, dyn.u, cut);
      const onRoad = isPlayer ? this.monitor.info.onRoad : true;
      dyn.surfaceGrip = grip * (onRoad ? 1 : 0.82);
      dyn.surfaceDrag = onRoad ? 0 : 1.5;
      dyn.step(dt, v.inputs, torque, v.steerTarget, v.assists);
      v.syncFromDynamics();
      v.wheelSpin += dyn.drivenOmega() * dt;
      v.odometer += dyn.absSpeed() * dt;
      if (isPlayer) {
        const sp = dyn.absSpeed();
        this.playerStats.distance += sp * dt;
        if (sp > 0.5) this.playerStats.driveTime += dt;
        if (sp > this.playerStats.maxSpeed) this.playerStats.maxSpeed = sp;
        v.lights.update(dt, dyn.steerAngle);
        if (dyn.absSpeed() > 0.5) this.record.driveClean(dyn.absSpeed() * dt);
      }
    }
    // Lane-mode rpm estimates for audio.
    for (let i = 0; i < this.vehicles.length; i++) {
      const v = this.vehicles[i];
      if (v.mode !== "lane") continue;
      const spec = v.spec;
      const wheel = (v.speed / spec.wheelRadius) * spec.finalDrive * 9.549;
      let g = v.gearEstimate;
      if (wheel * spec.gears[g - 1] > spec.redline * 0.55 && g < spec.gears.length) g++;
      else if (g > 1 && wheel * spec.gears[g - 2] < spec.redline * 0.42) g--;
      v.gearEstimate = g;
      v.rpmEstimate = Math.max(spec.idleRpm, wheel * spec.gears[g - 1]) + Math.max(0, v.accel) * 300;
    }
  }

  private collisions(): void {
    this.vehicleHash = new SpatialHash<Vehicle>(12);
    for (const v of this.vehicles) if (v.alive) this.vehicleHash.insert(v, v.x, v.y, v.halfLen);
    for (let i = 0; i < this.vehicles.length; i++) {
      const a = this.vehicles[i];
      if (!a.alive) continue;
      this.queryBuf = [];
      this.vehicleHash.query(a.x, a.y, a.halfLen + 2, this.queryBuf);
      for (const b of this.queryBuf) {
        if (b.id <= a.id || !b.alive) continue;
        // Lane-mode pairs only collide when something went wrong (distraction,
        // red-light runners): rear-ends on the same path or crossing movements.
        if (a.mode === "lane" && b.mode === "lane") {
          if (!lanePairCanCollide(a, b)) continue;
          const shrink = 0.85;
          setBox(boxA, a, shrink);
          setBox(boxB, b, shrink);
        } else {
          setBox(boxA, a, 1);
          setBox(boxB, b, 1);
        }
        if (!boxContact(boxA, boxB, contact)) continue;
        this.resolveVehicles(a, b);
      }
      if (a.mode === "physics") this.staticCollisions(a);
    }
  }

  private resolveVehicles(a: Vehicle, b: Vehicle): void {
    const vn = (b.vx() - a.vx()) * contact.nx + (b.vy() - a.vy()) * contact.ny;
    const closing = -vn;
    const bothLane = a.mode === "lane" && b.mode === "lane";
    if (bothLane && closing < 2.0) return;
    // Light pushes against a lane-mode car: treat it as a wall unless it's a real impact.
    if (a.mode === "physics" && b.mode === "lane" && closing < 1.2) {
      resolveStatic(a.dyn, contact, 0.1, 0.3);
      a.syncFromDynamics();
      this.bumped(a, b);
      return;
    }
    if (b.mode === "physics" && a.mode === "lane" && closing < 1.2) {
      const c2: Contact = { nx: -contact.nx, ny: -contact.ny, depth: contact.depth, px: contact.px, py: contact.py };
      resolveStatic(b.dyn, c2, 0.1, 0.3);
      b.syncFromDynamics();
      this.bumped(b, a);
      return;
    }
    const aWasLane = a.mode === "lane";
    const bWasLane = b.mode === "lane";
    if (aWasLane) a.enterPhysics();
    if (bWasLane) b.enterPhysics();
    // Fault: who drove into whom.
    const aInto = a.vx() * contact.nx + a.vy() * contact.ny;
    const bInto = -(b.vx() * contact.nx + b.vy() * contact.ny);
    const atFault = aInto >= bInto ? a : b;
    const j = resolveBodies(a.dyn, b.dyn, contact, 0.15, 0.4);
    a.syncFromDynamics();
    b.syncFromDynamics();
    if (j <= 0) return;
    const sevA = j / a.dyn.mass;
    const sevB = j / b.dyn.mass;
    this.damage(a, sevA);
    this.damage(b, sevB);
    const severity = Math.max(sevA, sevB);
    const da = this.drivers.get(a.id);
    const db = this.drivers.get(b.id);
    if (da !== undefined && (aWasLane || severity > 2)) da.onCrash(this, sevA);
    if (db !== undefined && (bWasLane || severity > 2)) db.onCrash(this, sevB);
    const victim = atFault === a ? b : a;
    a.lastHitTime = this.time;
    b.lastHitTime = this.time;
    a.lastHitBy = b.id;
    b.lastHitBy = a.id;
    if (severity > 0.8) {
      this.stats.collisions++;
      this.events.emit({ type: "collision", a: a.id, b: b.id, impulse: j, severity, x: contact.px, y: contact.py, atFault: atFault.id });
      this.offend(atFault, victim, "collision", clamp(severity / 4, 0.5, 2.5));
      if (victim !== atFault) this.offend(victim, atFault, "collision", 0.3);
    }
    const p = this.player;
    if (p !== undefined && atFault === p && severity > 1.5) {
      this.monitor.hitRun = { x: p.x, y: p.y, time: this.time, stoppedFor: 0, victim: victim.id };
      this.observe({ kind: "collision" });
    }
    if (severity > 7) this.requestAmbulance(contact.px, contact.py);
  }

  /** Gentle contact with a lane car: annoys its driver. */
  private bumped(phys: Vehicle, laneCar: Vehicle): void {
    const last = this.nearMissMemo.get(laneCar.id) ?? -10;
    if (this.time - last < 2) return;
    this.nearMissMemo.set(laneCar.id, this.time);
    this.offend(phys, laneCar, "near_miss", 0.6);
  }

  private staticCollisions(v: Vehicle): void {
    this.obstacleBuf = [];
    this.obstacleHash.query(v.x, v.y, v.halfLen + 1, this.obstacleBuf);
    for (const o of this.obstacleBuf) {
      setBox(boxA, v, 1);
      if (!boxContact(boxA, o, contact)) continue;
      const j = resolveStatic(v.dyn, contact, 0.12, 0.5);
      v.syncFromDynamics();
      if (j > 0) {
        const sev = j / v.dyn.mass;
        this.damage(v, sev);
        if (sev > 1.5) this.events.emit({ type: "crash_static", vehicle: v.id, severity: sev, x: contact.px, y: contact.py });
        const d = this.drivers.get(v.id);
        if (d !== undefined && sev > 3 && d.state === "chasing") d.onCrash(this, sev);
      }
    }
  }

  private damage(v: Vehicle, severity: number): void {
    const c = Math.cos(v.heading);
    const s = Math.sin(v.heading);
    const lx = (contact.px - v.x) * c + (contact.py - v.y) * s;
    const ly = -(contact.px - v.x) * s + (contact.py - v.y) * c;
    const hit = v.damage.hit(lx, ly, v.halfLen, v.halfWid, severity);
    if (hit.lost > 0) {
      if (hit.zone === "front") {
        v.pt.health = Math.min(v.pt.health, 0.25 + v.damage.front * 0.75);
        if (v.damage.front < 0.5) {
          if (ly > 0) v.lights.leftHeadlightBroken = true;
          else v.lights.rightHeadlightBroken = true;
        }
        if (v.damage.front < 0.15) v.pt.health = 0;
      }
      v.dyn.tireHealth = Math.min(v.dyn.tireHealth, 0.5 + v.damage.body() * 0.5);
      if (v.damage.totaled() && !v.wrecked) {
        v.wrecked = true;
        v.lights.hazard = !v.isPlayer;
        if (v.isPlayer) this.events.emit({ type: "info", text: "Your vehicle is wrecked. Press ` to reset." });
      }
    }
  }

  /** Put the player back on the nearest road (repairs cost money). */
  resetPlayer(): void {
    const p = this.player;
    if (p === undefined) return;
    const hits = this.net.nearestLanes(p.x, p.y, 200, []);
    if (hits.length === 0) return;
    const h = hits[0];
    const s = clamp(h.s, 5, h.lane.length - 5);
    const pt = { x: 0, y: 0, heading: 0 };
    h.lane.path.sample(s, pt);
    p.dyn.reset(pt.x, pt.y, pt.heading, 0);
    p.syncFromDynamics();
    const cost = Math.floor((1 - p.damage.body()) * 1500 + (1 - p.pt.health) * 800);
    if (cost > 0) {
      this.record.money -= cost;
      this.events.emit({ type: "info", text: `Vehicle repaired: -€$${cost}` });
    }
    p.damage.repair();
    p.pt.health = 1;
    p.pt.running = true;
    p.pt.stalled = false;
    p.wrecked = false;
    p.dyn.tireHealth = 1;
    p.lights.leftHeadlightBroken = false;
    p.lights.rightHeadlightBroken = false;
    if (p.pt.fuel < 5) p.pt.fuel = p.spec.fuelCapacity * 0.5;
  }

  // ----------------------------------------------------------- law

  private observations: Observation[] = [];

  private observe(o: Observation): void {
    this.observations.push(o);
  }

  private lawAndOrder(dt: number): void {
    const p = this.player;
    if (p !== undefined) {
      this.monitor.update(dt, p, this, this.observations);
      for (const o of this.observations) this.handleViolation(p, o.kind, o.cameraNode !== undefined, o.speedCamera === true);
      this.observations = [];
      this.playerSocial(dt, p);
    }
    this.police.update(dt, this, this.record);
  }

  private handleViolation(p: Vehicle, kind: ViolationKind, redCamera: boolean, speedCamera: boolean): void {
    if (!this.record.canCharge(kind, this.time)) return;
    const cops = this.policeNear(p.x, p.y, 110);
    let witness: Witness = "none";
    if (cops.length > 0) witness = "police";
    else if (redCamera || speedCamera) witness = "camera";
    const rec = this.record.add(kind, this.time, witness, p.x, p.y);
    this.events.emit({ type: "violation", vehicle: p.id, kind, label: rec.label, fine: rec.fine, points: rec.points, witnessed: witness, x: p.x, y: p.y });
    if (witness === "camera") {
      this.events.emit({ type: "camera_flash", x: p.x, y: p.y, kind: redCamera ? "red_light" : "speed" });
      this.events.emit({ type: "ticket", vehicle: p.id, total: rec.fine, points: rec.points, reason: `${rec.label} (camera)` });
    } else if (witness === "police") {
      this.police.report(this, cops[0], this.record, VIOLATIONS[kind].heat);
    }
  }

  /** Player behaviour that upsets AI drivers: tailgating, near misses. */
  private playerSocial(dt: number, p: Vehicle): void {
    const info = this.monitor.info;
    const lane = info.lane;
    if (lane !== undefined && info.headway < 0.7 && p.absSpeed() > 6 && info.leaderGap < 20) {
      // Find the car ahead in our lane.
      let leader: Vehicle | undefined = undefined;
      let best = Infinity;
      for (let i = 0; i < lane.occCount; i++) {
        const e = lane.occ[i];
        if (e.occ.id === p.id) continue;
        const v = this.byId.get(e.occ.id);
        if (v === undefined) continue;
        const d = (v.x - p.x) * Math.cos(p.heading) + (v.y - p.y) * Math.sin(p.heading);
        if (d > 0 && d < best) {
          best = d;
          leader = v;
        }
      }
      if (leader !== undefined) {
        const t = (this.playerTailgate.get(leader.id) ?? 0) + dt;
        this.playerTailgate.set(leader.id, t);
        if (t > 2) this.offend(p, leader, "tailgating", dt * 2.5);
      }
    } else this.playerTailgate = new Map();
    // Near misses: fast passes within a metre or two.
    if (p.absSpeed() > 8) {
      this.queryBuf = [];
      this.vehicleHash.query(p.x, p.y, 6, this.queryBuf);
      for (const v of this.queryBuf) {
        if (v === p) continue;
        const d = hypot(v.x - p.x, v.y - p.y) - (v.halfWid + p.halfWid);
        const rel = hypot(v.vx() - p.vx(), v.vy() - p.vy());
        if (d < 1.2 && rel > 7) {
          const last = this.nearMissMemo.get(v.id) ?? -10;
          if (this.time - last < 3) continue;
          this.nearMissMemo.set(v.id, this.time);
          this.events.emit({ type: "near_miss", a: p.id, b: v.id });
          this.offend(p, v, "near_miss", clamp(rel / 12, 0.5, 2));
        }
      }
    }
  }

  // ----------------------------------------------------------- pedestrians

  private readonly pedHost: PedHost = {
    signals: this.signals,
    trafficApproaching: (x: number, y: number, radius: number, seconds: number): boolean => {
      for (const v of this.vehicles) {
        const dx = x - v.x;
        const dy = y - v.y;
        const d = Math.sqrt(dx * dx + dy * dy);
        if (d > radius + v.absSpeed() * seconds + 3) continue;
        if (d < radius + 2) return true;
        // Heading toward the crossing?
        const toward = (dx * Math.cos(v.heading) + dy * Math.sin(v.heading)) / Math.max(d, 0.1);
        if (toward > 0.6 && v.absSpeed() > 1) return true;
      }
      return false;
    },
    vehicleAt: (x: number, y: number, margin: number): boolean => {
      for (const v of this.vehicles) {
        const dx = x - v.x;
        const dy = y - v.y;
        if (Math.abs(dx) > v.halfLen + margin + 1 || Math.abs(dy) > v.halfLen + margin + 1) continue;
        const c = Math.cos(v.heading);
        const s = Math.sin(v.heading);
        const lx = dx * c + dy * s;
        const ly = -dx * s + dy * c;
        if (Math.abs(lx) < v.halfLen + margin && Math.abs(ly) < v.halfWid + margin) return true;
      }
      return false;
    },
    onShout: (p: Pedestrian): void => {
      this.events.emit({ type: "ped_speech", ped: p.id, text: this.rng.pick(PED_SHOUTS), x: p.x, y: p.y });
    },
  };

  private updatePedestrians(dt: number): void {
    const night = 1 - this.env.daylight;
    this.peds.target = Math.round(this.pedestrianDensity * (1 - night * 0.6) * (0.5 + this.env.trafficDensity * 0.5) * (1 - this.env.rain * 0.5));
    this.peds.update(dt, this.pedHost);
    // Vehicles touching pedestrians.
    for (const ped of this.peds.peds) {
      if (ped.state !== "crossing" && ped.state !== "waiting") continue;
      this.queryBuf = [];
      this.vehicleHash.query(ped.x, ped.y, 4, this.queryBuf);
      for (const v of this.queryBuf) {
        // Pedestrian position in the vehicle frame.
        const c = Math.cos(v.heading);
        const s = Math.sin(v.heading);
        const lx = (ped.x - v.x) * c + (ped.y - v.y) * s;
        const ly = -(ped.x - v.x) * s + (ped.y - v.y) * c;
        const inside = Math.abs(lx) < v.halfLen + 0.3 && Math.abs(ly) < v.halfWid + 0.3;
        const speed = v.absSpeed();
        // Lane-bound cars can't mount the sidewalk: only people in the road count for them.
        if (inside && speed > 1.2 && (v.mode === "physics" || ped.state === "crossing")) {
          ped.state = "down";
          ped.ttl = 40;
          this.events.emit({ type: "ped_hit", ped: ped.id, vehicle: v.id, severity: speed, x: ped.x, y: ped.y });
          if (speed > 4) this.requestAmbulance(ped.x, ped.y);
          if (v.isPlayer) {
            this.observe({ kind: "hit_pedestrian" });
            this.monitor.hitRun = { x: v.x, y: v.y, time: this.time, stoppedFor: 0, victim: -1 };
          }
          break;
        }
        // Close pass in front of someone crossing: they shout, the player is charged.
        const near = Math.abs(ly) < v.halfWid + 1.6 && lx > -v.halfLen && lx < v.halfLen + 2.5;
        if (near && ped.state === "crossing" && speed > 2.5 && ped.shoutCooldown <= 0) {
          ped.shoutCooldown = 5;
          this.pedHost.onShout(ped);
          if (v.isPlayer) this.observe({ kind: "fail_yield_pedestrian" });
        }
      }
    }
  }

  // ----------------------------------------------------------- ambulances

  private requestAmbulance(x: number, y: number): void {
    if (this.ambulance !== undefined && this.ambulance.alive) return;
    const spot = this.spawner.findSpot(200, 700);
    if (spot === undefined) return;
    const v = this.spawner.create("ambulance", "ambulance", spot.lane, spot.s, 8);
    const d = this.drivers.get(v.id);
    if (d === undefined) return;
    d.emergency = true;
    v.lights.siren = "wail";
    v.lights.lightbar = true;
    const hits = this.net.nearestLanes(x, y, 30, []);
    if (hits.length > 0) d.routeTo(this, hits[0].lane.road, hits[0].lane.forward);
    this.ambulance = v;
    this.ambulanceTarget = { x, y, t: 0 };
    this.events.emit({ type: "dispatch", unit: v.id, kind: "ambulance", x, y, reason: "collision" });
  }

  private ambulances(dt: number): void {
    const a = this.ambulance;
    const t = this.ambulanceTarget;
    if (a === undefined || t === undefined) return;
    if (!a.alive) {
      this.ambulance = undefined;
      return;
    }
    const d = this.drivers.get(a.id);
    const dist = hypot(a.x - t.x, a.y - t.y);
    t.t += dt;
    if ((dist < 30 || t.t > 120) && d !== undefined && d.emergency) {
      d.emergency = false;
      a.lights.siren = "off";
      t.t = 0;
    }
    if (d !== undefined && !d.emergency && t.t > 30) {
      a.lights.lightbar = false;
      this.ambulance = undefined;
      this.ambulanceTarget = undefined;
    }
  }

  // ----------------------------------------------------------- queries

  /** Signal controller at a node (or undefined). */
  signalAt(nodeId: number): SignalController | undefined {
    const n = this.net.nodes[nodeId];
    return n.signalId >= 0 ? this.signals[n.signalId] : undefined;
  }

  /** Vehicles in a radius (for renderers with LOD). */
  vehiclesNear(x: number, y: number, r: number): Vehicle[] {
    const out: Vehicle[] = [];
    for (const v of this.vehicles) if ((v.x - x) * (v.x - x) + (v.y - y) * (v.y - y) <= r * r) out.push(v);
    return out;
  }

  /** Drain host actions (camera, pause, ...). */
  drainHostActions(): ActionId[] {
    return this.hostActions.splice(0, this.hostActions.length);
  }

  /** Debug: is segment occupied? */
  segment(id: number): Segment {
    return this.net.segments[id];
  }

  /** Is the given vehicle's driver angry at the player? (HUD helper) */
  angryAtPlayer(): { vehicle: Vehicle; anger: number; raging: boolean }[] {
    const out: { vehicle: Vehicle; anger: number; raging: boolean }[] = [];
    const p = this.player;
    if (p === undefined) return out;
    for (const v of this.vehicles) {
      const d = this.drivers.get(v.id);
      if (d === undefined) continue;
      const g = d.emo.grudgeAgainst(p.id);
      if (g > 10 && hypot(v.x - p.x, v.y - p.y) < 120) out.push({ vehicle: v, anger: d.emo.anger, raging: d.rage !== undefined && d.rage.target === p.id });
    }
    out.sort((a, b) => (b.anger !== a.anger ? b.anger - a.anger : a.vehicle.id - b.vehicle.id));
    return out;
  }

  /** Vehicles that are close behind & aligned (police HUD helper). */
  isAngleAligned(a: Vehicle, b: Vehicle): boolean {
    return Math.abs(angleDiff(a.heading, b.heading)) < 30 * DEG;
  }
}

const PED_SHOUTS = ["Hey! I'm walking here!", "Watch it!", "It's a crosswalk!", "Slow down!", "Are you blind?!", "Jeez!"];

function lanePairCanCollide(a: Vehicle, b: Vehicle): boolean {
  const sa = a.seg;
  const sb = b.seg;
  if (sa === undefined || sb === undefined) return false;
  if (sa === sb || a.prevSeg === sb || b.prevSeg === sa) return Math.abs(angleDiff(a.heading, b.heading)) < 35 * DEG;
  if (sa.kind === "connector" && sb.kind === "connector") {
    const ca = sa as Connector;
    for (const z of ca.conflicts) if (z.other === sb) return true;
  }
  return false;
}

function setBox(b: Box, v: Vehicle, shrink: number): void {
  b.x = v.x;
  b.y = v.y;
  b.heading = v.heading;
  b.hl = v.halfLen * shrink;
  b.hw = v.halfWid * shrink;
}
