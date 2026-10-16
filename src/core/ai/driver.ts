import { angleDiff, clamp, DEG, lerp, mod } from "../math/scalar";
import { Rng } from "../math/rng";
import { addOccupant, Connector, Lane, Road } from "../road/network";
import { RouteStep } from "../road/router";
import { Segment } from "../road/types";
import { maxAccelAt } from "../vehicle/specs";
import { Vehicle } from "../vehicle/vehicle";
import type { SimContext } from "../world/context";
import { brakeToStop, driveToward } from "./control";
import { Emotion, Mood } from "./emotion";
import { idmAccel, idmFree, IdmParams } from "./idm";
import { approachStopDistance, connectorStopDistance, DriverView, JunctionState } from "./intersection";
import { Personality } from "./personality";
import { lineFor } from "./speech";

export type DriverState = "drive" | "crashed" | "recovering" | "chasing";

export type RageBehavior = "tailgate" | "brake_check" | "ram" | "chase";

export interface RageState {
  target: number;
  behavior: RageBehavior;
  time: number;
  lostTime: number;
  brakeCheck: number;
  cooldown: number;
  rams: number;
}

const params: IdmParams = { v0: 10, T: 1.4, a: 1.5, b: 2, s0: 2 };
const stopParams: IdmParams = { v0: 10, T: 0.2, a: 1.5, b: 2, s0: 0.6 };

/** Result of a leader search along the planned path. */
interface Leader {
  id: number;
  gap: number;
  speed: number;
  phantom: boolean;
  /** Segment & arc length where the leader was found. */
  seg: Segment | undefined;
  s: number;
}

/**
 * Brain of an AI vehicle. In lane mode it follows its route with IDM car
 * following, MOBIL-style lane changes and the junction rules in
 * intersection.ts. Emotions (emotion.ts) modulate parameters and trigger
 * horn/high-beam use, speech and road-rage episodes; physics mode handles
 * crashes, recovery and chases.
 */
export class AIDriver implements DriverView {
  readonly emo: Emotion;
  readonly js = new JunctionState();
  route: RouteStep[] = [];
  ri = 0;
  /** Connector planned from the current lane. */
  next: Connector | undefined = undefined;
  state: DriverState = "drive";
  emergency = false;
  rage: RageState | undefined = undefined;
  readonly leader: Leader = { id: -1, gap: Infinity, speed: 0, phantom: false, seg: undefined, s: 0 };
  private prevLeaderId = -1;
  private prevLeaderGap = Infinity;
  /** Distance to the current stop target (front bumper), Infinity if none. */
  stopDist = Infinity;
  /** Lane change. */
  private lcTimer: number;
  private lcIntent: Lane | undefined = undefined;
  private lcIntentTime = 0;
  /** The lane we are leaving during a lane change (registered as a shadow). */
  shadowLane: Lane | undefined = undefined;
  private latTarget = 0;
  /** Timers. */
  private hornCooldown = 0;
  private hornHold = 0;
  private flashTimer = 0;
  private flashCount = 0;
  speechCooldown = 0;
  private blockedTime = 0;
  private slowTime = 0;
  private distraction = 0;
  private distractCheck = 0;
  private reactTimer = 0;
  yieldEmergency = 0;
  /** Mood at the previous step (the world emits mood-change events). */
  lastMood: Mood = "calm";
  private lastAccel = 0;
  private crashTimer = 0;
  private recoverLane: Lane | undefined = undefined;
  private wanderTimer = 0;
  private stuckTime = 0;
  private readonly rng: Rng;
  /** Police override: chase/follow this vehicle id (physics). */
  followTarget = -1;
  followGap = 10;
  /** Police override: drive (on roads) toward this point. */
  headFor: { x: number; y: number } | undefined = undefined;

  constructor(
    readonly v: Vehicle,
    readonly p: Personality,
    seed: number,
  ) {
    this.emo = new Emotion(p);
    this.rng = new Rng(seed);
    this.lcTimer = this.rng.range(0.5, 2.5);
    this.distractCheck = this.rng.range(0, 1);
  }

  /** Mirrors of emotion state (plain fields: cheap in Lua). */
  mood: Mood = "calm";
  raging = false;

  private syncFlags(): void {
    this.mood = this.emo.mood;
    this.raging = this.rage !== undefined;
  }

  roll(): number {
    return this.rng.next();
  }

  /** Short description for debug overlays. */
  status(): string {
    if (this.state !== "drive") return this.state;
    if (this.rage !== undefined) return `RAGE:${this.rage.behavior}`;
    if (this.distraction > 0) return "distracted";
    if (this.yieldEmergency > 0) return "yield-emergency";
    if (this.js.reason !== "" && this.stopDist < 40) return this.js.reason;
    if (this.lcIntent !== undefined) return "changing lanes";
    return this.v.speed < 0.3 ? "stopped" : "driving";
  }

  // ---------------------------------------------------------------- routing

  private nextRoad(idx: number): RouteStep | undefined {
    return idx < this.route.length ? this.route[idx] : undefined;
  }

  /** Pick a new random destination and route there from `lane`. */
  newDestination(ctx: SimContext, lane: Lane): void {
    const net = ctx.net;
    for (let attempt = 0; attempt < 6; attempt++) {
      const goal = net.lanes[Math.floor(this.rng.next() * net.lanes.length)];
      if (goal.road === lane.road) continue;
      const r = ctx.router.route(lane, goal.road, goal.forward);
      if (r !== undefined && r.length > 0) {
        this.route = r;
        this.ri = 0;
        this.next = this.connectorFrom(lane);
        return;
      }
    }
    // Wander: no route, choose connectors at random.
    this.route = [];
    this.ri = 0;
    this.next = this.connectorFrom(lane);
  }

  /** Route to a specific road (police dispatch etc.). */
  routeTo(ctx: SimContext, road: Road, forward: boolean): boolean {
    const lane = this.currentLane();
    if (lane === undefined) return false;
    if (lane.road === road) return true;
    const r = ctx.router.route(lane, road, forward);
    if (r === undefined) return false;
    this.route = r;
    this.ri = 0;
    this.next = this.connectorFrom(lane);
    return true;
  }

  currentLane(): Lane | undefined {
    const seg = this.v.seg;
    if (seg === undefined) return undefined;
    return seg.kind === "lane" ? (seg as Lane) : (seg as Connector).to;
  }

  /** Connector to take out of `lane` according to the route (or a wander choice). */
  private connectorFrom(lane: Lane): Connector | undefined {
    const step = this.nextRoad(this.ri);
    if (step !== undefined) {
      const c = lane.connectorTo(step.road);
      if (c !== undefined) return c;
      // Wrong lane: plan with the straight-ish fallback until we change lanes.
    }
    if (lane.outgoing.length === 0) return undefined;
    if (step === undefined) return lane.outgoing[Math.floor(this.rng.next() * lane.outgoing.length)];
    for (const c of lane.outgoing) if (c.turn === "straight") return c;
    return lane.outgoing[0];
  }

  /** Lanes of the current road that lead where the route goes. */
  private routeLanes(lane: Lane): Lane[] {
    const step = this.nextRoad(this.ri);
    const out: Lane[] = [];
    if (step === undefined) return out;
    for (const l of lane.siblings()) if (l.connectorTo(step.road) !== undefined) out.push(l);
    return out;
  }

  // ------------------------------------------------------------- perception

  private scanLeader(ctx: SimContext): void {
    const v = this.v;
    const L = this.leader;
    L.id = -1;
    L.gap = Infinity;
    L.speed = 0;
    L.phantom = false;
    L.seg = undefined;
    const seg0 = v.seg;
    if (seg0 === undefined) return;
    const look = Math.max(55, v.speed * 4.5 + 25);
    let base = -v.s;
    let seg: Segment | undefined = seg0;
    let depth = 0;
    let bestD = Infinity;
    let nextConn = this.next;
    while (seg !== undefined && depth < 5) {
      for (let i = 0; i < seg.occCount; i++) {
        const e = seg.occ[i];
        if (e.occ.id === v.id) continue;
        if (depth === 0 && e.s <= v.s + 0.05) continue;
        // Lateral separation lets cars pass pulled-over vehicles.
        if (depth === 0 && Math.abs(e.lat - v.lat) > (e.occ.width + v.width) / 2 + 0.25) continue;
        const d = base + e.s - (e.occ.length + v.length) / 2;
        if (d < bestD) {
          bestD = d;
          L.id = e.occ.id;
          L.speed = e.v;
          L.phantom = e.phantom;
          L.seg = seg;
          L.s = e.s;
        }
      }
      if (seg.kind === "connector") {
        // Cars just entering a diverging connector still block the shared start.
        const conn = seg as Connector;
        const sib = conn.siblings;
        for (let k = 0; k < sib.length; k++) {
          const o = sib[k];
          const diverge = conn.siblingDiverge[k];
          for (let i = 0; i < o.occCount; i++) {
            const e = o.occ[i];
            if (e.occ.id === v.id || e.s - e.occ.length / 2 > diverge) continue;
            if (depth === 0 && e.s <= v.s) continue;
            const d = base + e.s - (e.occ.length + v.length) / 2;
            if (d < bestD) {
              bestD = d;
              L.id = e.occ.id;
              L.speed = e.v;
              L.phantom = e.phantom;
              L.seg = o;
              L.s = e.s;
            }
          }
        }
      }
      // Mid lane change: cars in the lane we are leaving still matter.
      if (depth === 0 && this.shadowLane !== undefined && seg.kind === "lane") {
        const sh = this.shadowLane;
        const sMe = (v.s * sh.length) / seg.length;
        for (let i = 0; i < sh.occCount; i++) {
          const e = sh.occ[i];
          if (e.occ.id === v.id || e.s <= sMe) continue;
          const d = e.s - sMe - (e.occ.length + v.length) / 2;
          if (d < bestD) {
            bestD = d;
            L.id = e.occ.id;
            L.speed = e.v;
            L.phantom = e.phantom;
            L.seg = sh;
            L.s = e.s;
          }
        }
      }
      if (bestD < Infinity) break;
      base += seg.length;
      if (base > look) break;
      // Advance along the plan.
      if (seg.kind === "lane") {
        const c: Connector | undefined = depth === 0 ? nextConn : (seg as Lane).outgoing.length > 0 ? this.peekConnector(seg as Lane, depth) : undefined;
        seg = c;
      } else {
        const lane: Lane = (seg as Connector).to;
        seg = lane;
        nextConn = undefined;
      }
      depth++;
    }
    L.gap = bestD;
  }

  private peekConnector(lane: Lane, depth: number): Connector | undefined {
    // Route index for lanes further ahead: one road step per lane passed.
    const idx = this.ri + (this.v.seg !== undefined && this.v.seg.kind === "connector" ? 0 : 1) + Math.floor((depth - 1) / 2);
    const step = this.nextRoad(idx);
    if (step !== undefined) {
      const c = lane.connectorTo(step.road);
      if (c !== undefined) return c;
    }
    for (const c of lane.outgoing) if (c.turn === "straight") return c;
    return lane.outgoing.length > 0 ? lane.outgoing[0] : undefined;
  }

  // ----------------------------------------------------------------- update

  update(dt: number, ctx: SimContext): void {
    const v = this.v;
    this.hornCooldown -= dt;
    this.speechCooldown -= dt;
    this.updateHornAndFlash(dt);
    this.emo.update(dt, ctx.time, this.congestion());
    this.syncFlags();
    v.lights.dark = ctx.env.dark;
    if (v.wrecked) {
      this.state = "crashed";
      v.lights.hazard = true;
    }
    if (v.mode === "physics") {
      this.physicsUpdate(dt, ctx);
      return;
    }
    this.laneUpdate(dt, ctx);
  }

  private congestion(): number {
    const lim = this.v.seg !== undefined ? this.v.seg.speedLimit : 10;
    return clamp(1 - this.v.speed / Math.max(lim, 1), 0, 1) * (this.leader.gap < 15 ? 1 : 0.3);
  }

  private laneUpdate(dt: number, ctx: SimContext): void {
    const v = this.v;
    const seg = v.seg;
    if (seg === undefined) return;
    const p = this.p;

    if (seg.kind === "lane" && (this.next === undefined || this.next.from !== seg)) this.next = this.connectorFrom(seg as Lane);
    v.intentConnector = seg.kind === "lane" && this.next !== undefined ? this.next.id : seg.kind === "connector" ? seg.id : -1;

    this.scanLeader(ctx);
    this.detectOffenses(dt, ctx);
    this.checkEmergencyBehind(dt, ctx);
    this.updateRage(dt, ctx);
    this.syncFlags();
    if (v.mode !== "lane") return; // rage may have switched us to physics

    // --- Parameters shaped by mood, weather and situation.
    const mood = this.emo.mood;
    const moodSpeed = mood === "angry" ? 1.1 : mood === "furious" ? 1.2 : mood === "irritated" ? 1.04 : 1;
    const moodT = mood === "angry" ? 0.7 : mood === "furious" ? 0.55 : mood === "irritated" ? 0.88 : 1;
    const wet = ctx.env.wetness;
    const vis = ctx.env.visibility;
    const weatherSpeed = (1 - wet * 0.15) * clamp(vis / 250, 0.55, 1);
    params.v0 = seg.speedLimit * p.speedFactor * moodSpeed * weatherSpeed;
    if (this.emergency) params.v0 = seg.speedLimit * 1.45;
    params.T = p.headway * moodT * (1 + wet * 0.3);
    params.a = Math.min(maxAccelAt(v.spec, v.speed), 2.6) * p.accelFactor;
    params.b = p.comfortDecel;
    params.s0 = p.minGap;
    const rage = this.rage;
    if (rage !== undefined && rage.target === this.leader.id && rage.behavior === "tailgate") {
      params.T = 0.3;
      params.s0 = 1.0;
      params.v0 *= 1.3;
    }

    // --- Car following.
    let acc = idmFree(v.speed, params);
    if (this.leader.gap < Infinity) acc = idmAccel(v.speed, this.leader.gap, v.speed - this.leader.speed, params);

    // --- Curves: slow for the next connector's comfortable speed.
    if (seg.kind === "lane" && this.next !== undefined) {
      const vc = this.next.speedLimit * lerp(0.9, 1.15, p.aggression);
      const dc = seg.length - v.s - v.halfLen;
      if (v.speed > vc) acc = Math.min(acc, -((v.speed * v.speed - vc * vc) / (2 * Math.max(dc, 2))) * 1.15);
    } else if (seg.kind === "connector") {
      const vc = seg.speedLimit * lerp(0.95, 1.15, p.aggression);
      if (v.speed > vc) acc = Math.min(acc, -1.5);
    }

    // --- Junction rules.
    this.stopDist = Infinity;
    if (seg.kind === "lane" && this.next !== undefined && !this.ragingThroughLights()) {
      this.stopDist = approachStopDistance(ctx, this, seg as Lane, this.next, dt);
      if (this.emergency && this.next.node.control === "signal" && seg.length - v.s < 120) ctx.signals[this.next.node.signalId].requestPreempt(this.next.approach);
    } else if (seg.kind === "connector") {
      this.stopDist = connectorStopDistance(ctx, this, seg as Connector, dt);
    }
    // End of a lane with no exit at all: stop.
    if (seg.kind === "lane" && this.next === undefined) this.stopDist = Math.min(this.stopDist, seg.length - v.s - v.halfLen);
    if (this.stopDist < Infinity) {
      stopParams.a = params.a;
      stopParams.b = params.b;
      stopParams.v0 = params.v0;
      acc = Math.min(acc, idmAccel(v.speed, this.stopDist, v.speed, stopParams));
    }

    // --- Pull over for emergency vehicles.
    if (this.yieldEmergency > 0) {
      const lane = seg.kind === "lane" ? (seg as Lane) : undefined;
      if (lane !== undefined && lane.right === undefined) this.latTarget = -(lane.width * 0.42);
      acc = Math.min(acc, v.speed > 2 ? -2.5 : -0.5);
    } else if (this.emergency && this.leader.gap < 30 && this.leader.speed < 3) {
      this.latTarget = 1.2; // pass on the left of yielding traffic
    } else this.latTarget = 0;

    // --- Brake check (rage).
    if (rage !== undefined && rage.brakeCheck > 0) acc = Math.min(acc, -7);

    // --- Human factors: reaction delay when starting, distraction.
    acc = this.humanFactors(dt, acc);

    const maxBrake = 8.5 * ctx.env.grip * v.spec.grip;
    acc = clamp(acc, -maxBrake, params.a * 1.05);
    this.lastAccel = acc;
    this.integrate(dt, acc, ctx);
    this.laneChanges(dt, ctx);
    this.lateral(dt);
    this.updateLights(dt, ctx);
    this.updateWaiting(dt, ctx);
  }

  private ragingThroughLights(): boolean {
    return this.rage !== undefined && (this.rage.behavior === "tailgate" || this.rage.behavior === "chase") && this.emo.anger > 85 && this.p.lawfulness < 0.3;
  }

  private humanFactors(dt: number, acc: number): number {
    const v = this.v;
    const p = this.p;
    // Starting from standstill takes a reaction time (queue discharge wave).
    if (v.speed < 0.15 && acc > 0.2) {
      this.reactTimer += dt;
      const reaction = lerp(1.4, 0.45, p.skill) + (this.distraction > 0 ? 2.5 : 0);
      if (this.reactTimer < reaction) acc = 0;
    } else if (v.speed > 0.5) this.reactTimer = 0;
    // Distraction (phone, radio...). The driver keeps the old command until a reflex kicks in.
    this.distractCheck -= dt;
    if (this.distractCheck <= 0) {
      this.distractCheck = 1;
      if (this.distraction <= 0 && this.rng.chance((1 - p.attentiveness) * 0.025)) this.distraction = this.rng.range(1.2, 3.5);
    }
    if (this.distraction > 0) {
      this.distraction -= dt;
      const ttc = this.leader.gap < Infinity && v.speed > this.leader.speed ? this.leader.gap / (v.speed - this.leader.speed) : Infinity;
      const stopTtc = this.stopDist < Infinity && v.speed > 0.5 ? this.stopDist / v.speed : Infinity;
      if (ttc < 1.3 || stopTtc < 1.0) {
        this.distraction = 0; // startled back to attention
        return Math.min(acc, -8);
      }
      return Math.min(this.lastAccel, Math.max(acc, this.lastAccel - 0.2));
    }
    return acc;
  }

  private integrate(dt: number, acc: number, ctx: SimContext): void {
    const v = this.v;
    let sp = v.speed + acc * dt;
    if (sp < 0) sp = 0;
    v.accel = (sp - v.speed) / dt;
    v.speed = sp;
    v.s += sp * dt;
    v.odometer += sp * dt;
    v.wheelSpin += (sp * dt) / v.spec.wheelRadius;
    // Segment transitions.
    let guard = 0;
    while (v.seg !== undefined && v.s > v.seg.length && guard < 4) {
      guard++;
      const seg: Segment = v.seg;
      const over = v.s - seg.length;
      if (seg.kind === "lane") {
        let c = this.next;
        if (c === undefined || c.from !== seg) c = this.connectorFrom(seg as Lane);
        if (c === undefined) {
          v.s = seg.length;
          v.speed = 0;
          break;
        }
        if (this.js.queued) ctx.allWay(c.node.id).arrivals.delete(v.id);
        this.js.queued = false;
        v.prevSeg = seg;
        v.seg = c;
        v.s = over;
        this.shadowLane = undefined;
      } else {
        const lane = (seg as Connector).to;
        v.prevSeg = seg;
        v.seg = lane;
        v.s = over;
        const step = this.nextRoad(this.ri);
        if (step !== undefined && step.road === lane.road) this.ri++;
        else if (step !== undefined) this.route = []; // went off-route
        if (this.headFor === undefined && this.ri >= this.route.length) this.newDestination(ctx, lane);
        this.next = this.connectorFrom(lane);
        this.js.reset(lane.id);
        if (this.lcIntent !== undefined) {
          this.lcIntent = undefined;
          v.lights.setIndicator("off");
        }
      }
    }
    v.syncLanePose();
    // Front wheel angle for rendering from path curvature.
    if (v.seg !== undefined) v.wheelAngle = clamp(v.seg.path.curvatureAt(v.s + 2, 2) * v.spec.wheelbase * 1.1 + this.v.latVel * 0.05, -0.6, 0.6);
  }

  // ----------------------------------------------------------- lane changes

  private laneChanges(dt: number, ctx: SimContext): void {
    const v = this.v;
    const seg = v.seg;
    if (seg === undefined || seg.kind !== "lane") return;
    const lane = seg as Lane;
    if (Math.abs(v.lat) > 0.6) return; // still executing one
    const toEnd = lane.length - v.s;
    // Keep a pending intent alive: signal first, then go when safe.
    if (this.lcIntent !== undefined) {
      this.lcIntentTime += dt;
      const signalTime = lerp(0.6, 2.2, this.p.signalDiscipline);
      const urgency = clamp(1 - (toEnd - 20) / 80, 0, 1);
      if (this.lcIntentTime >= signalTime * (1 - urgency * 0.7)) {
        const bSafe = lerp(2.5, 5, this.p.aggression) + urgency * 3 + (this.raging ? 2 : 0);
        if (this.tryLaneChange(ctx, lane, this.lcIntent, bSafe)) return;
      }
      if (this.lcIntentTime > 12) {
        this.lcIntent = undefined;
        v.lights.setIndicator("off");
      }
      return;
    }
    this.lcTimer -= dt;
    if (this.lcTimer > 0) return;
    this.lcTimer = this.rng.range(0.8, 1.8);
    if (toEnd < 12 || v.s < 4) return;
    // Mandatory: get into a lane that has our turn.
    const good = this.routeLanes(lane);
    if (good.length > 0 && good.indexOf(lane) < 0) {
      const dir = good[0].index < lane.index ? lane.left : lane.right;
      if (dir !== undefined) this.intend(dir);
      return;
    }
    // Emergency vehicle behind: move right.
    if (this.yieldEmergency > 0 && lane.right !== undefined) {
      this.intend(lane.right);
      return;
    }
    // Discretionary (MOBIL): only where both lanes still work for the route.
    if (v.speed < 1 && this.leader.gap > 8) return;
    const aOld = this.leader.gap < Infinity ? idmAccel(v.speed, this.leader.gap, v.speed - this.leader.speed, params) : idmFree(v.speed, params);
    let best: Lane | undefined = undefined;
    let bestGain = 0;
    const cands: (Lane | undefined)[] = [lane.left, lane.right];
    for (const cand of cands) {
      if (cand === undefined) continue;
      if (good.length > 0 && good.indexOf(cand) < 0 && toEnd < 150) continue;
      const g = this.evaluateLane(ctx, lane, cand);
      if (g === undefined) continue;
      const bias = cand === lane.right ? 0.12 : -0.05; // keep-right rule
      const politeness = this.raging ? 0 : this.p.politeness;
      const gain = g.aNew - aOld + politeness * g.followerDelta + bias;
      const threshold = lerp(0.45, 0.1, this.p.aggression);
      if (gain > threshold && gain > bestGain) {
        bestGain = gain;
        best = cand;
      }
    }
    if (best !== undefined) this.intend(best);
  }

  private intend(target: Lane): void {
    this.lcIntent = target;
    this.lcIntentTime = 0;
    const signals = this.rng.chance(this.p.signalDiscipline);
    if (signals) this.v.lights.setIndicator(target === (this.v.seg as Lane).left ? "left" : "right");
  }

  /** Neighbour lane situation at our position. */
  private evaluateLane(ctx: SimContext, lane: Lane, cand: Lane): { aNew: number; followerDelta: number; followerAcc: number; follower: number; gapAhead: number; gapBehind: number } | undefined {
    const v = this.v;
    const sT = (v.s * cand.length) / lane.length;
    let lead: { s: number; v: number; len: number } | undefined = undefined;
    let fol: { s: number; v: number; len: number; id: number } | undefined = undefined;
    for (let i = 0; i < cand.occCount; i++) {
      const e = cand.occ[i];
      if (e.occ.id === v.id) continue;
      if (e.s >= sT) {
        if (lead === undefined || e.s < lead.s) lead = { s: e.s, v: e.v, len: e.occ.length };
      } else if (fol === undefined || e.s > fol.s) fol = { s: e.s, v: e.v, len: e.occ.length, id: e.occ.id };
    }
    // Cars still on connectors feeding the target lane count as followers.
    if (fol === undefined && sT < 30) {
      for (const c of cand.incoming) {
        for (let i = 0; i < c.occCount; i++) {
          const e = c.occ[i];
          if (e.occ.id === v.id) continue;
          const s = e.s - c.length;
          if (fol === undefined || s > fol.s) fol = { s, v: e.v, len: e.occ.length, id: e.occ.id };
        }
      }
    }
    const gapAhead = lead === undefined ? Infinity : lead.s - sT - (lead.len + v.length) / 2;
    const gapBehind = fol === undefined ? Infinity : sT - fol.s - (fol.len + v.length) / 2;
    const minAhead = Math.max(2.5, v.speed * lerp(0.6, 0.3, this.p.aggression));
    const minBehind = fol === undefined ? 0 : Math.max(2.0, fol.v * lerp(0.5, 0.25, this.p.aggression));
    if (gapAhead < minAhead || gapBehind < minBehind) return undefined;
    const aNew = lead === undefined ? idmFree(v.speed, params) : idmAccel(v.speed, gapAhead, v.speed - lead.v, params);
    let followerDelta = 0;
    let followerAcc = 0;
    if (fol !== undefined) {
      const fp: IdmParams = { v0: cand.speedLimit, T: 1.3, a: 1.5, b: 2, s0: 2 };
      const before = lead === undefined ? idmFree(fol.v, fp) : idmAccel(fol.v, lead.s - fol.s - (lead.len + fol.len) / 2, fol.v - lead.v, fp);
      followerAcc = idmAccel(fol.v, gapBehind, fol.v - v.speed, fp);
      followerDelta = followerAcc - before;
    }
    return { aNew, followerDelta, followerAcc, follower: fol === undefined ? -1 : fol.id, gapAhead, gapBehind };
  }

  private tryLaneChange(ctx: SimContext, lane: Lane, target: Lane, bSafe: number): boolean {
    const v = this.v;
    // Never swerve across at the stop line unless crawling.
    if (lane.length - v.s - v.halfLen < 10 && v.speed > 2) return false;
    const ev = this.evaluateLane(ctx, lane, target);
    if (ev === undefined) return false;
    if (ev.followerAcc < -bSafe) return false;
    // Execute.
    const offset = target === lane.right ? (lane.width + target.width) / 2 : -(lane.width + target.width) / 2;
    const sT = (v.s * target.length) / lane.length;
    this.shadowLane = lane;
    v.seg = target;
    v.s = sT;
    v.lat += offset;
    // Visible immediately to vehicles updated later this step (no simultaneous merges).
    addOccupant(target, v, sT, v.speed, false, v.lat);
    this.lcIntent = undefined;
    this.next = this.connectorFrom(target);
    this.js.reset(target.id);
    // Forcing the new follower to brake hard is "cutting them off".
    if (ev.follower >= 0 && ev.followerAcc < -2.2) {
      const victim = ctx.vehicle(ev.follower);
      if (victim !== undefined) ctx.offend(v, victim, "cut_off", clamp(-ev.followerAcc / 3, 0.5, 2));
    }
    return true;
  }

  private lateral(dt: number): void {
    const v = this.v;
    const err = this.latTarget - v.lat;
    const maxV = lerp(0.9, 1.8, this.p.aggression) * (this.raging ? 1.4 : 1);
    const want = clamp(err * 1.1, -maxV, maxV);
    v.latVel += clamp(want - v.latVel, -2.5 * dt, 2.5 * dt);
    v.lat += v.latVel * dt;
    if (Math.abs(err) < 0.03 && Math.abs(v.latVel) < 0.05) {
      v.lat = this.latTarget;
      v.latVel = 0;
    }
    if (this.shadowLane !== undefined && Math.abs(v.lat) < v.width / 2) this.shadowLane = undefined;
    // Indicator off once the change is done (if no further intent).
    if (this.lcIntent === undefined && Math.abs(v.lat) < 0.2 && this.v.seg !== undefined && this.v.seg.kind === "lane") {
      const lights = v.lights;
      const turnSoon = this.turnSignalWanted();
      if (turnSoon === "off" && lights.indicator !== "off" && lights.indicatorTime > 1) lights.setIndicator("off");
    }
  }

  // ---------------------------------------------------------- lights, horn

  private turnSignalWanted(): "left" | "right" | "off" {
    const v = this.v;
    const seg = v.seg;
    if (seg === undefined) return "off";
    let c: Connector | undefined = undefined;
    let dist = 0;
    if (seg.kind === "lane") {
      c = this.next;
      dist = seg.length - v.s;
    } else c = seg as Connector;
    if (c === undefined || dist > 45) return "off";
    if (c.turn === "left" || c.turn === "uturn") return "left";
    if (c.turn === "right") return "right";
    return "off";
  }

  private signalDecided = -1;
  private willSignal = true;

  private updateLights(dt: number, ctx: SimContext): void {
    const v = this.v;
    const lights = v.lights;
    lights.brake = this.lastAccel < -0.6 || (v.speed < 0.2 && this.stopDist < 20) || (v.speed < 0.2 && this.leader.gap < 8) ? 1 : 0;
    lights.reverse = false;
    if (this.state === "drive" && !v.wrecked) lights.hazard = this.yieldEmergency > 0 && v.speed < 1 && this.p.lawfulness > 0.8;
    if (this.lcIntent === undefined) {
      const want = this.turnSignalWanted();
      const segId = v.seg !== undefined ? v.seg.id : -1;
      if (this.signalDecided !== segId) {
        this.signalDecided = segId;
        this.willSignal = this.rng.chance(this.p.signalDiscipline);
      }
      if (want !== "off" && this.willSignal) lights.setIndicator(want);
      else if (want === "off" && Math.abs(v.lat) < 0.2 && lights.indicatorTime > 1.5 && v.seg !== undefined && v.seg.kind === "lane") lights.setIndicator("off");
    }
    // Lights auto-cancel when the "steering" returns; lane-mode has no wheel, so drive it from curvature.
    lights.update(dt, v.wheelAngle);
    if (this.emergency) {
      lights.lightbar = true;
      if (lights.siren === "off") lights.siren = "wail";
    }
    // Night: some drivers forget their lights.
    if (ctx.env.dark && lights.headlightSwitch === "off" && this.rng.chance(0.002)) lights.headlightSwitch = "auto";
  }

  private updateHornAndFlash(dt: number): void {
    const l = this.v.lights;
    if (this.hornHold > 0) {
      this.hornHold -= dt;
      l.horn = true;
      if (this.hornHold <= 0) l.horn = false;
    }
    if (this.flashTimer > 0) {
      this.flashTimer -= dt;
      const phase = mod(this.flashTimer, 0.5);
      l.flash = phase > 0.25;
      if (this.flashTimer <= 0) l.flash = false;
    }
    void this.flashCount;
  }

  honk(ctx: SimContext, long: boolean): void {
    if (this.hornCooldown > 0) return;
    this.hornHold = long ? this.rng.range(1.2, 2.2) : this.rng.range(0.25, 0.45);
    this.hornCooldown = long ? 4 : 2.2;
    ctx.events.emit({ type: "horn", vehicle: this.v.id, long });
    // Vehicles in front hear it (handled by the world for consistency).
    hornVictims(ctx, this.v, long);
  }

  flashHighBeams(ctx: SimContext): void {
    if (this.flashTimer > 0) return;
    this.flashTimer = 1.0;
    this.flashCount++;
    ctx.events.emit({ type: "flash", vehicle: this.v.id });
    const leader = this.leader.id >= 0 ? ctx.vehicle(this.leader.id) : undefined;
    if (leader !== undefined && this.leader.gap < 40) ctx.offend(this.v, leader, "high_beams", 1);
  }

  say(ctx: SimContext, text: string): void {
    if (this.speechCooldown > 0) return;
    this.speechCooldown = this.rng.range(5, 9);
    ctx.events.emit({ type: "speech", vehicle: this.v.id, text, mood: this.emo.mood });
  }

  /** React to an offense (called by the world after anger was applied). */
  onOffended(ctx: SimContext, offender: Vehicle, kind: Parameters<Emotion["offend"]>[1]): void {
    const mood = this.emo.mood;
    const hornChance = this.p.hornHappiness * (mood === "calm" ? 0.4 : 1);
    if (kind !== "honked_at" && kind !== "long_honk" && this.rng.chance(hornChance)) this.honk(ctx, mood === "angry" || mood === "furious");
    if (mood !== "calm" || kind === "collision" || kind === "near_miss") {
      if (this.rng.chance(0.6)) this.say(ctx, lineFor(this.rng, mood, kind));
    }
    void offender;
  }

  // ------------------------------------------------------------- offenses

  private detectOffenses(dt: number, ctx: SimContext): void {
    const v = this.v;
    const L = this.leader;
    // Phantom (player / free body) cutting in close in front of us.
    if (L.id >= 0 && L.id !== this.prevLeaderId && L.phantom && v.speed > 4) {
      const thw = L.gap / Math.max(v.speed, 1);
      if (thw < 0.75 && L.gap < this.prevLeaderGap - 3) {
        const off = ctx.vehicle(L.id);
        if (off !== undefined) ctx.offend(off, v, "cut_off", clamp((0.9 - thw) * 2.5, 0.5, 2));
      }
    }
    this.prevLeaderId = L.id;
    this.prevLeaderGap = L.gap;
    // Blocked: stuck behind a stationary vehicle with no good reason.
    const leaderVeh = L.id >= 0 ? ctx.vehicle(L.id) : undefined;
    if (leaderVeh !== undefined && L.gap < 12 && L.speed < 0.3 && v.speed < 0.3 && !this.leaderHasReason(ctx, leaderVeh)) {
      this.blockedTime += dt;
      const limit = 2.5 + this.p.patience * 5;
      if (this.blockedTime > limit) {
        ctx.offend(leaderVeh, v, "blocked", dt * 2);
        if (this.hornCooldown <= 0 && this.rng.chance(0.4 + this.p.hornHappiness * 0.5)) this.honk(ctx, this.blockedTime > limit * 2 || this.emo.mood === "angry");
      }
    } else this.blockedTime = 0;
    // Slowpoke ahead.
    const lim = v.seg !== undefined ? v.seg.speedLimit : 10;
    if (leaderVeh !== undefined && L.gap < 25 && L.speed > 1 && L.speed < lim * 0.55 && this.stopDist === Infinity) {
      this.slowTime += dt;
      if (this.slowTime > 6 + this.p.patience * 10) ctx.offend(leaderVeh, v, "slow_driver", dt);
    } else this.slowTime = 0;
  }

  /** Is a stationary leader stopped for a legitimate reason (red light, queue, sign)? */
  private leaderHasReason(ctx: SimContext, lead: Vehicle): boolean {
    const d = ctx.driverOf(lead);
    if (d !== undefined) {
      if (d.stopDist < 8) return true;
      if (d.leader.gap < 8 && d.leader.speed < 0.5) return true;
      if (d.state !== "drive") return d.state === "crashed" ? false : true;
      return d.yieldEmergency > 0;
    }
    // Player/free body: near a stop line with a red or a sign?
    const seg = this.leader.seg;
    if (seg !== undefined && seg.kind === "lane") {
      const lane = seg as Lane;
      const toLine = lane.length - this.leader.s;
      if (toLine < 10) {
        const ap = lane.approach;
        if (ap !== undefined && (ap.sign === "stop" || ap.sign === "yield")) return true;
        if (ap !== undefined && ap.sign === "signal") {
          const sig = ctx.signals[ap.node.signalId];
          for (const c of lane.outgoing) if (sig.indication(c) === "green") return this.blockedTime < 4;
          return true;
        }
      }
      // Queued behind someone?
      for (let i = 0; i < lane.occCount; i++) {
        const e = lane.occ[i];
        if (e.occ.id !== lead.id && e.s > this.leader.s && e.s - this.leader.s < 10 && e.v < 0.5) return true;
      }
    }
    return false;
  }

  private checkEmergencyBehind(dt: number, ctx: SimContext): void {
    if (this.yieldEmergency > 0) this.yieldEmergency -= dt;
    if (this.emergency) return;
    const v = this.v;
    const hc = Math.cos(v.heading);
    const hs = Math.sin(v.heading);
    for (const e of ctx.vehicles) {
      if (!e.sirenOn() || e === v || !e.alive) continue;
      const dx = e.x - v.x;
      const dy = e.y - v.y;
      const d2 = dx * dx + dy * dy;
      if (d2 > 80 * 80) continue;
      const behind = dx * hc + dy * hs < -2;
      const sameDir = Math.abs(angleDiff(v.heading, e.heading)) < 50 * DEG;
      if (behind && sameDir) {
        this.yieldEmergency = 2.5;
        return;
      }
    }
  }

  private updateWaiting(dt: number, ctx: SimContext): void {
    const v = this.v;
    if (v.speed < 0.2) {
      this.stuckTime += dt;
      if (this.stuckTime > 30) this.emo.offend(-1, "long_wait", dt * 2, ctx.time);
    } else this.stuckTime = 0;
    // Angry drivers lean on the horn at their target.
    const t = this.emo.target;
    if (t >= 0 && (this.emo.mood === "angry" || this.emo.mood === "furious") && this.leader.id === t && this.leader.gap < 25) {
      if (this.rng.chance(dt * 0.4 * (0.4 + this.p.hornHappiness))) this.honk(ctx, true);
      if (this.rng.chance(dt * 0.3)) this.flashHighBeams(ctx);
    }
  }

  // ------------------------------------------------------------------ rage

  private updateRage(dt: number, ctx: SimContext): void {
    const v = this.v;
    const emo = this.emo;
    let rage = this.rage;
    if (rage === undefined) {
      if (!emo.canRage() || emo.target < 0 || this.emergency) return;
      const tgt = ctx.vehicle(emo.target);
      if (tgt === undefined || !tgt.alive) return;
      const dist = Math.sqrt((tgt.x - v.x) * (tgt.x - v.x) + (tgt.y - v.y) * (tgt.y - v.y));
      if (dist > 70) return;
      const police = ctx.policeNear(v.x, v.y, 60);
      if (police.length > 0) return;
      let behavior: RageBehavior = "tailgate";
      const tgtDriver = ctx.driverOf(tgt);
      const targetBehindMe = tgtDriver !== undefined ? tgtDriver.leader.id === v.id : this.isBehind(tgt);
      if (targetBehindMe) behavior = "brake_check";
      else if (this.leader.id !== tgt.id && this.p.aggression > 0.8) behavior = "chase";
      rage = { target: tgt.id, behavior, time: 0, lostTime: 0, brakeCheck: 0, cooldown: 0, rams: 0 };
      this.rage = rage;
      ctx.events.emit({ type: "rage_start", vehicle: v.id, target: tgt.id, behavior });
      this.say(ctx, lineFor(this.rng, "furious", undefined));
      if (behavior === "chase") this.startChase(ctx);
      return;
    }
    rage.time += dt;
    rage.cooldown -= dt;
    if (rage.brakeCheck > 0) rage.brakeCheck -= dt;
    const tgt = ctx.vehicle(rage.target);
    if (tgt === undefined || !tgt.alive) return this.endRage(ctx, "target gone");
    const dist = Math.sqrt((tgt.x - v.x) * (tgt.x - v.x) + (tgt.y - v.y) * (tgt.y - v.y));
    if (dist > 120) rage.lostTime += dt;
    else rage.lostTime = 0;
    if (rage.lostTime > 6) return this.endRage(ctx, "lost target");
    if (emo.anger < 35) return this.endRage(ctx, "calmed down");
    if (rage.time > 50) return this.endRage(ctx, "gave up");
    if (ctx.policeNear(v.x, v.y, 50).length > 0) {
      emo.anger = Math.min(emo.anger, 40);
      return this.endRage(ctx, "police nearby");
    }
    const tgtDriver = ctx.driverOf(tgt);
    const targetBehindMe = tgtDriver !== undefined ? tgtDriver.leader.id === v.id && tgtDriver.leader.gap < 18 : this.isBehind(tgt) && dist < 18;
    if (rage.behavior === "tailgate") {
      if (this.leader.id === rage.target && this.rng.chance(dt * 0.5)) this.flashHighBeams(ctx);
      if (this.leader.id === rage.target && this.rng.chance(dt * 0.35)) this.honk(ctx, true);
      // Escalate: tailgating for a while makes the worst drivers go for contact.
      if (rage.time > 8 && this.p.aggression > 0.82 && emo.anger > 88 && dist < 25) {
        rage.behavior = "ram";
        ctx.events.emit({ type: "rage_start", vehicle: v.id, target: rage.target, behavior: "ram" });
        this.startChase(ctx);
      } else if (targetBehindMe) rage.behavior = "brake_check";
    } else if (rage.behavior === "brake_check") {
      if (targetBehindMe && rage.cooldown <= 0 && v.speed > 5) {
        rage.brakeCheck = this.rng.range(0.8, 1.4);
        rage.cooldown = this.rng.range(6, 10);
        ctx.offend(v, tgt, "brake_check", 1);
        ctx.events.emit({ type: "rage_start", vehicle: v.id, target: rage.target, behavior: "brake_check" });
      }
      if (!targetBehindMe && rage.cooldown <= 0) rage.behavior = "tailgate";
    }
  }

  private endRage(ctx: SimContext, reason: string): void {
    this.rage = undefined;
    this.raging = false;
    ctx.events.emit({ type: "rage_end", vehicle: this.v.id, reason });
    if (this.state === "chasing") this.state = "recovering";
  }

  private isBehind(o: Vehicle): boolean {
    const v = this.v;
    const dx = o.x - v.x;
    const dy = o.y - v.y;
    return dx * Math.cos(v.heading) + dy * Math.sin(v.heading) < -3 && Math.abs(angleDiff(v.heading, o.heading)) < 0.8;
  }

  private startChase(ctx: SimContext): void {
    if (this.v.mode !== "physics") this.v.enterPhysics();
    this.state = "chasing";
    this.v.lights.setIndicator("off");
    void ctx;
  }

  // --------------------------------------------------------------- physics

  /** Called by the world when this vehicle was hit hard enough to leave its lane. */
  onCrash(ctx: SimContext, severity: number): void {
    if (this.state === "chasing" && severity < 6) return;
    this.state = "crashed";
    this.crashTimer = this.rng.range(2.5, 6) + severity * 0.3;
    this.v.lights.setIndicator("off");
    void ctx;
  }

  private physicsUpdate(dt: number, ctx: SimContext): void {
    const v = this.v;
    v.physicsTime += dt;
    const lights = v.lights;
    lights.brake = v.inputs.brake > 0.05 ? 1 : 0;
    lights.reverse = v.pt.gear < 0;
    lights.update(dt, v.dyn.steerAngle);
    if (this.followTarget >= 0) {
      this.followUpdate(dt, ctx);
      return;
    }
    switch (this.state) {
      case "crashed": {
        brakeToStop(v, true);
        if (v.physicsTime > 1) lights.hazard = true;
        if (v.wrecked) return;
        if (v.absSpeed() < 0.5) this.crashTimer -= dt;
        if (this.crashTimer <= 0) {
          if (this.emo.canRage() && this.emo.target >= 0 && this.rage === undefined) {
            this.updateRage(dt, ctx);
            if (this.rage !== undefined) {
              this.state = "chasing";
              return;
            }
          }
          this.state = "recovering";
          this.recoverLane = undefined;
        }
        return;
      }
      case "chasing": {
        const rage = this.rage;
        if (rage === undefined) {
          this.state = "recovering";
          return;
        }
        this.updateRage(dt, ctx);
        const tgt = ctx.vehicle(rage.target);
        if (tgt === undefined || this.rage === undefined) {
          this.state = "recovering";
          return;
        }
        // Aim slightly ahead of the target; ramming means aiming at its flank.
        const lead = rage.behavior === "ram" ? 0.35 : 0.8;
        const tx = tgt.x + tgt.vx() * lead;
        const ty = tgt.y + tgt.vy() * lead;
        const dist = Math.sqrt((tx - v.x) * (tx - v.x) + (ty - v.y) * (ty - v.y));
        const tSpeed = tgt.absSpeed();
        const want = rage.behavior === "ram" ? tSpeed + 6 : dist > 20 ? tSpeed + 8 : tSpeed;
        driveToward(v, tx, ty, clamp(want, 6, 40), true);
        if (this.rng.chance(dt * 0.6)) this.honk(ctx, true);
        if (this.rng.chance(dt * 0.4)) this.flashHighBeams(ctx);
        lights.hazard = false;
        return;
      }
      case "recovering":
        this.recoverUpdate(dt, ctx);
        return;
      default:
        this.state = "recovering";
    }
  }

  private recoverUpdate(dt: number, ctx: SimContext): void {
    const v = this.v;
    const d = v.dyn;
    v.lights.hazard = false;
    if (this.recoverLane === undefined || this.wanderTimer <= 0) {
      this.wanderTimer = 2;
      const hits = ctx.net.nearestLanes(d.x, d.y, 30, []);
      let best = undefined as (typeof hits)[number] | undefined;
      let bestScore = Infinity;
      for (const h of hits) {
        if (h.s > h.lane.length - 15) continue;
        const ang = Math.abs(angleDiff(d.yaw, h.lane.path.headingAt(h.s)));
        const score = h.dist + ang * 12;
        if (score < bestScore) {
          bestScore = score;
          best = h;
        }
      }
      this.recoverLane = best !== undefined ? best.lane : undefined;
      if (this.recoverLane === undefined) {
        brakeToStop(v, false);
        v.despawnTimer += dt * 20;
        return;
      }
    }
    this.wanderTimer -= dt;
    const lane = this.recoverLane;
    const pr = { s: 0, lateral: 0, d2: 0 };
    lane.path.project(d.x, d.y, pr);
    const target = { x: 0, y: 0, heading: 0 };
    lane.path.sample(Math.min(lane.length, pr.s + 9), target);
    driveToward(v, target.x, target.y, 6, true);
    const heading = lane.path.headingAt(pr.s);
    const herr = Math.abs(angleDiff(d.yaw, heading));
    if (Math.sqrt(pr.d2) < 0.9 && herr < 0.3 && d.u > 0.5 && pr.s < lane.length - 5) {
      v.enterLane(lane, pr.s, pr.lateral);
      v.latVel = 0;
      this.latTarget = 0;
      this.state = "drive";
      this.newDestination(ctx, lane);
      this.js.reset(lane.id);
    }
    if (v.physicsTime > 90) v.despawnTimer += dt;
  }

  /** Police: follow/pursue a vehicle in physics mode, keeping `followGap`. */
  private followUpdate(dt: number, ctx: SimContext): void {
    const v = this.v;
    const tgt = ctx.vehicle(this.followTarget);
    if (tgt === undefined) {
      this.followTarget = -1;
      this.state = "recovering";
      return;
    }
    const dx = tgt.x - v.x;
    const dy = tgt.y - v.y;
    const dist = Math.sqrt(dx * dx + dy * dy);
    const tSpeed = tgt.absSpeed();
    const gapErr = dist - this.followGap;
    let want = tSpeed + clamp(gapErr * 0.6, -tSpeed - 5, 25);
    if (this.followGap < 2) want = tSpeed + 10; // ram/PIT
    if (dist < this.followGap && tSpeed < 0.5) {
      brakeToStop(v, false);
      return;
    }
    // Aim behind the target when following, at it when pursuing.
    const back = this.followGap > 2 ? 0 : -0.5;
    const ax = tgt.x - Math.cos(tgt.heading) * back + tgt.vx() * 0.4;
    const ay = tgt.y - Math.sin(tgt.heading) * back + tgt.vy() * 0.4;
    driveToward(v, ax, ay, clamp(want, 0, 55), true);
    void dt;
    void ctx;
  }
}

/** Anger vehicles in front of a honking car. */
export function hornVictims(ctx: SimContext, honker: Vehicle, long: boolean): void {
  const hc = Math.cos(honker.heading);
  const hs = Math.sin(honker.heading);
  for (const o of ctx.vehicles) {
    if (o === honker || !o.alive) continue;
    const dx = o.x - honker.x;
    const dy = o.y - honker.y;
    const fwd = dx * hc + dy * hs;
    if (fwd < 0 || fwd > 30) continue;
    const side = Math.abs(-dx * hs + dy * hc);
    if (side > 4 + fwd * 0.2) continue;
    ctx.offend(honker, o, long ? "long_honk" : "honked_at", 1);
  }
}
