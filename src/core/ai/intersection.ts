import { angleDiff, clamp, DEG, lerp } from "../math/scalar";
import { Connector, Lane, Node } from "../road/network";
import { TurnType } from "../road/types";
import { Indication } from "../signals/signalController";
import { Vehicle } from "../vehicle/vehicle";
import type { SimContext } from "../world/context";
import { Mood } from "./emotion";
import { Personality } from "./personality";

/** How a driver must treat the junction ahead. */
export type EntryMode = "go" | "stop" | "stop_then_yield" | "yield" | "allway";

/** Per-approach latched decisions (reset whenever the driver enters a new lane). */
export class JunctionState {
  laneId = -1;
  yellow: "stop" | "go" | undefined = undefined;
  /** Decided to run this red light. */
  redRun = false;
  /** The run-or-stop decision for this red has been made (rolled once). */
  redDecided = false;
  /** Came to a complete stop at the line (stop signs, right on red). */
  hasStopped = false;
  /** Registered in the all-way-stop queue. */
  queued = false;
  /** Ignores don't-block-the-box this time. */
  boxIgnore = false;
  /** Seconds waiting at this junction for yield reasons. */
  waitTime = 0;
  /** Last reason we were held (for debug HUD). */
  reason = "";
  /** Seconds held inside the junction by stationary vehicles only (deadlock breaker). */
  connWait = 0;
  /** Deadlock broken in our favour: ignore stationary higher-id blockers until we leave. */
  tieBreak = false;
  lastIndication: Indication | undefined = undefined;

  reset(laneId: number): void {
    this.laneId = laneId;
    this.yellow = undefined;
    this.redRun = false;
    this.redDecided = false;
    this.hasStopped = false;
    this.queued = false;
    this.boxIgnore = false;
    this.waitTime = 0;
    this.reason = "";
    this.lastIndication = undefined;
    this.connWait = 0;
    this.tieBreak = false;
  }
}

/** The subset of a driver the junction logic needs. */
export interface DriverView {
  v: Vehicle;
  p: Personality;
  mood: Mood;
  js: JunctionState;
  /** Emergency vehicle running with siren: treats reds as yields. */
  emergency: boolean;
  /** In a road-rage episode: ignores some rules. */
  raging: boolean;
  /** Deterministic per-driver random in [0,1) for latched choices. */
  roll(): number;
}

const MOVE_RANK: Record<TurnType, number> = { straight: 3, right: 2, left: 1, uturn: 0 };

function indRank(i: Indication): number {
  switch (i) {
    case "green":
      return 4;
    case "yellow":
      return 3;
    case "permissive":
    case "flash_yellow":
      return 2;
    case "flash_red":
    case "dark":
      return 1;
    default:
      return 0;
  }
}

export function indicationFor(ctx: SimContext, c: Connector): Indication | undefined {
  const n = c.node;
  if (n.control !== "signal" || n.signalId < 0) return undefined;
  return ctx.signals[n.signalId].indication(c);
}

/** Does movement `o` have right of way over movement `c` right now? */
export function hasPriority(ctx: SimContext, o: Connector, c: Connector): boolean {
  const io = indicationFor(ctx, o);
  const ic = indicationFor(ctx, c);
  if (io !== undefined && ic !== undefined) {
    const ro = indRank(io);
    const rc = indRank(ic);
    if (ro !== rc) return ro > rc;
  } else {
    const mo = o.approach.major;
    const mc = c.approach.major;
    if (mo !== mc) return mo;
  }
  const a = MOVE_RANK[o.turn];
  const b = MOVE_RANK[c.turn];
  if (a !== b) return a > b;
  // Same movement class: traffic from the right goes first.
  const d = angleDiff(c.approach.heading, o.approach.heading);
  return d > 45 * DEG && d < 135 * DEG;
}

/** Resolve the entry mode for connector c from signals and signs. */
export function entryMode(ctx: SimContext, d: DriverView, c: Connector, distToLine: number): EntryMode {
  const n = c.node;
  const ap = c.approach;
  const v = d.v;
  const ind = indicationFor(ctx, c);
  if (ind !== undefined) {
    d.js.lastIndication = ind;
    switch (ind) {
      case "green":
        d.js.yellow = undefined;
        return "go";
      case "permissive":
        return "yield";
      case "flash_yellow":
        return "yield";
      case "flash_red":
      case "dark":
        return "allway";
      case "yellow": {
        if (d.emergency) return "yield";
        if (d.js.yellow === undefined) {
          const sp = Math.abs(v.speed);
          const need = (sp * sp) / (2 * Math.max(distToLine, 0.1));
          // Willingness to brake: cautious drivers stop for harder decel, aggressive ones gamble.
          const threshold = lerp(4.6, 2.0, d.p.aggression) * lerp(0.85, 1.1, d.p.lawfulness);
          const tooClose = distToLine < sp * 0.8;
          d.js.yellow = need <= threshold && !tooClose ? "stop" : "go";
          if (d.mood === "furious" || d.raging) d.js.yellow = need < 1.2 ? "stop" : "go";
        }
        return d.js.yellow === "go" ? "go" : "stop";
      }
      case "red": {
        if (d.emergency) return "stop_then_yield";
        if (d.js.yellow === "go" && distToLine < 3) return "go"; // committed at the change
        // Red-light running: rare, needs low lawfulness or rage; decided once.
        if (!d.js.redDecided && distToLine > 25 && Math.abs(v.speed) > 8) {
          d.js.redDecided = true;
          const chance = (1 - d.p.lawfulness) * 0.08 + (d.raging ? 0.5 : 0) + (d.mood === "furious" ? 0.15 : 0);
          if (d.roll() < chance) d.js.redRun = true;
        } else if (!d.js.redDecided && distToLine <= 25) d.js.redDecided = true;
        if (d.js.redRun) return "yield";
        const sig = ctx.signals[n.signalId];
        if (c.turn === "right" && sig.allowRightOnRed) return "stop_then_yield";
        return "stop";
      }
      default:
        return "stop";
    }
  }
  switch (ap.sign) {
    case "stop":
      return n.control === "allway_stop" ? "allway" : "stop_then_yield";
    case "yield":
      return "yield";
    default:
      return n.roads.length <= 2 ? "go" : "yield";
  }
}

/** Seconds needed for `v` to travel `dist` from its current speed (accelerating with `a`, capped at vmax). */
export function timeToCover(speed: number, dist: number, a: number, vmax: number): number {
  if (dist <= 0) return 0;
  const sp = Math.max(0, speed);
  const vm = Math.max(vmax, sp, 1);
  const tAcc = (vm - sp) / a;
  const dAcc = sp * tAcc + 0.5 * a * tAcc * tAcc;
  if (dAcc >= dist) return (-sp + Math.sqrt(sp * sp + 2 * a * dist)) / a;
  return tAcc + (dist - dAcc) / vm;
}

/**
 * Vehicles already inside the junction on conflicting paths that would still
 * be in a shared zone when we arrive. Applies regardless of priority (you
 * don't T-bone a red-light runner just because you have green).
 */
function committedBlock(ctx: SimContext, c: Connector, me: Vehicle, sMe: number, speed: number, strict: boolean): boolean {
  for (let k = 0; k < c.conflicts.length; k++) {
    const z = c.conflicts[k];
    if (z.s1 < sMe - me.halfLen) continue; // zone behind us
    const o = z.other;
    for (let i = 0; i < o.occCount; i++) {
      const e = o.occ[i];
      const occ = e.occ;
      if (occ.id === me.id) continue;
      const half = occ.length / 2;
      if (e.s - half > z.os1) continue; // already cleared the zone
      const inside = e.s + half > z.os0;
      if (!inside && e.v < 0.5) {
        // Stationary before the zone: they are waiting. Ignore them when they
        // must yield to us; otherwise an id tiebreak prevents mutual deadlock.
        if (!hasPriority(ctx, o, c)) continue;
        if (!strict || me.id < occ.id) continue;
      }
      const tClear = (z.os1 - (e.s - half)) / Math.max(e.v, 1.0);
      const tEnter = Math.max(0, z.s0 - sMe - me.halfLen) / Math.max(speed, 3);
      if (tClear > tEnter - 0.3) return true;
    }
  }
  return false;
}

/**
 * Gap acceptance against movements with priority: vehicles on, or heading for,
 * conflicting connectors that arrive within our critical gap.
 */
function priorityBlock(ctx: SimContext, d: DriverView, c: Connector, distToLine: number, gapScale: number): boolean {
  const me = d.v;
  const a = 2.0;
  const vmax = c.speedLimit;
  for (let k = 0; k < c.conflicts.length; k++) {
    const z = c.conflicts[k];
    const o = z.other;
    if (!hasPriority(ctx, o, c)) continue;
    const tMe = timeToCover(me.speed, distToLine + z.s1 + me.length, a, vmax);
    const need = Math.max(d.p.criticalGap * gapScale, tMe + 1.2);
    // Vehicles on the conflicting connector.
    for (let i = 0; i < o.occCount; i++) {
      const e = o.occ[i];
      if (e.occ.id === me.id) continue;
      if (e.s - e.occ.length / 2 > z.os1) continue;
      const dist = z.os0 - e.s - e.occ.length / 2;
      const t = dist <= 0 ? 0 : dist / Math.max(e.v, 0.5);
      if (e.v < 0.5 && dist > 2) continue;
      if (t < need) return true;
    }
    // Vehicles approaching it on its lane that intend to take it.
    const lane = o.from;
    for (let i = 0; i < lane.occCount; i++) {
      const e = lane.occ[i];
      const occ = e.occ;
      if (occ.id === me.id) continue;
      if (occ.intentConnector !== o.id && occ.intentConnector !== -1) continue;
      if (e.v < 1.0) continue; // stopped (red light / queue): not coming yet
      const dist = lane.length - e.s - occ.length / 2 + z.os0;
      const t = dist / Math.max(e.v, 0.5);
      if (t < need) return true;
    }
  }
  return false;
}

/** Enough room on the exit lane to clear the junction? */
function boxBlocked(c: Connector, me: Vehicle): boolean {
  const lane = c.to;
  let minS = Infinity;
  let minV = 0;
  for (let i = 0; i < lane.occCount; i++) {
    const e = lane.occ[i];
    if (e.occ.id === me.id) continue;
    const tail = e.s - e.occ.length / 2;
    if (tail < minS) {
      minS = tail;
      minV = e.v;
    }
  }
  // Vehicles on our own connector ahead also consume exit space.
  let onC = 0;
  for (let i = 0; i < c.occCount; i++) {
    const e = c.occ[i];
    if (e.occ.id !== me.id) onC += e.occ.length + 2;
  }
  if (minS === Infinity) return onC > c.length + 4;
  return minS - onC < me.length + 2.5 && minV < 3;
}

/** Another car is already waiting inside the box on our movement (one left-turner at a time). */
function connectorHasWaiter(c: Connector, me: Vehicle): boolean {
  for (let i = 0; i < c.occCount; i++) {
    const e = c.occ[i];
    if (e.occ.id !== me.id && e.v < 1.5) return true;
  }
  return false;
}

function firstConflictStart(c: Connector): number {
  let s = c.length;
  for (const z of c.conflicts) if (z.kind === "cross" && z.s0 < s) s = z.s0;
  return s;
}

/** All-way stop: is it our turn among vehicles waiting at conflicting approaches? */
function allWayTurn(ctx: SimContext, d: DriverView, c: Connector): boolean {
  const q = ctx.allWay(c.node.id);
  const mine = q.arrivals.get(d.v.id);
  if (mine === undefined) return false;
  for (const z of c.conflicts) {
    const lane = z.other.from;
    for (let i = 0; i < lane.occCount; i++) {
      const e = lane.occ[i];
      if (e.occ.id === d.v.id) continue;
      const t = q.arrivals.get(e.occ.id);
      if (t === undefined) continue;
      if (e.occ.intentConnector !== z.other.id && e.occ.intentConnector !== -1) continue;
      if (t < mine - 0.05 || (Math.abs(t - mine) <= 0.05 && e.occ.id < d.v.id)) return false;
    }
  }
  return true;
}

/**
 * Distance (from our front bumper) at which we must stop for the junction
 * reached via connector `c`, or Infinity to proceed. Called while on the lane
 * leading into c.
 */
export function approachStopDistance(ctx: SimContext, d: DriverView, lane: Lane, c: Connector, dt: number): number {
  const me = d.v;
  if (d.js.laneId !== lane.id) d.js.reset(lane.id);
  const distToLine = lane.length - me.s - me.halfLen;
  const sp = Math.max(0, me.speed);
  const range = Math.max(35, (sp * sp) / 3 + 20);
  if (distToLine > range) return Infinity;

  let mode = entryMode(ctx, d, c, distToLine);
  const stopped = sp < 0.4 && distToLine < 3.0;
  const rainScale = 1 + ctx.env.wetness * 0.25 + (ctx.env.dark ? 0.1 : 0);
  const impatience = clamp(d.js.waitTime / 30, 0, 0.5);
  const gapScale = rainScale * (d.raging ? 0.6 : 1) * (1 - impatience);

  if (mode === "stop") {
    d.js.reason = "red";
    return distToLine;
  }
  if (mode === "stop_then_yield" || mode === "allway") {
    if (!d.js.hasStopped) {
      // Lawless drivers do a "rolling stop".
      const rolling = d.p.lawfulness < 0.35 && sp < 3 && distToLine < 2;
      if (stopped || rolling) d.js.hasStopped = true;
      else {
        d.js.reason = "stop sign";
        return distToLine;
      }
    }
    if (mode === "allway") {
      const q = ctx.allWay(c.node.id);
      if (!d.js.queued) {
        q.arrivals.set(me.id, ctx.time);
        d.js.queued = true;
      }
      if (!allWayTurn(ctx, d, c) && d.js.waitTime < 12) {
        d.js.waitTime += dt;
        d.js.reason = "all-way order";
        return distToLine;
      }
    } else mode = "yield";
  }

  if (committedBlock(ctx, c, me, -me.halfLen - distToLine, Math.max(sp, 3), d.js.waitTime < 15)) {
    d.js.waitTime += dt;
    d.js.reason = "junction occupied";
    return distToLine;
  }
  if (mode === "yield" || mode === "allway") {
    if (priorityBlock(ctx, d, c, distToLine, gapScale)) {
      // Permissive left: pull into the box and wait at the conflict point.
      const ind = d.js.lastIndication;
      if (c.turn === "left" && (ind === "permissive" || ind === "flash_yellow") && !boxBlocked(c, me) && !connectorHasWaiter(c, me)) {
        d.js.reason = "waiting in box";
        return distToLine + firstConflictStart(c) - 1.0;
      }
      d.js.waitTime += dt;
      d.js.reason = "yield";
      return distToLine;
    }
  }
  // Don't block the box (unless this driver doesn't care today).
  if (d.js.laneId === lane.id && !d.js.boxIgnore && d.js.waitTime === 0 && d.p.lawfulness < 0.3 && d.roll() < 0.3) d.js.boxIgnore = true;
  if (!d.js.boxIgnore && boxBlocked(c, me)) {
    d.js.reason = "exit blocked";
    return distToLine;
  }
  d.js.reason = "";
  d.js.waitTime = 0;
  return Infinity;
}

/**
 * While on a connector: distance to stop before a conflict zone that is still
 * occupied (or contested by a priority movement for permissive lefts).
 */
export function connectorStopDistance(ctx: SimContext, d: DriverView, c: Connector, dt: number): number {
  const me = d.v;
  const front = me.s + me.halfLen;
  const sp = Math.max(0, me.speed);
  let stopAt = Infinity;
  // Deadlock breaker: after waiting a while on stationary vehicles only, the
  // lower id proceeds (the other keeps waiting for it to clear).
  if (d.js.connWait > 4) d.js.tieBreak = true;
  const breakTies = d.js.tieBreak;
  let onlyStationary = true;
  for (const z of c.conflicts) {
    if (z.s0 < front - 0.5) continue; // already in or past the zone
    const distToZone = z.s0 - front - 0.5;
    const tArrive = distToZone / Math.max(sp, 2);
    const o = z.other;
    const prio = hasPriority(ctx, o, c);
    let blocked = false;
    for (let i = 0; i < o.occCount && !blocked; i++) {
      const e = o.occ[i];
      if (e.occ.id === me.id) continue;
      const half = e.occ.length / 2;
      if (e.s - half > z.os1) continue;
      const inside = e.s + half > z.os0;
      if (!inside && e.v < 0.5 && me.id < e.occ.id) continue;
      if (!inside && !prio && e.v < 3) continue;
      if (e.v < 0.5 && breakTies && me.id < e.occ.id) continue;
      const tClear = (z.os1 - (e.s - half)) / Math.max(e.v, 1.0);
      const tThem = inside ? 0 : Math.max(0, z.os0 - e.s - half) / Math.max(e.v, 0.5);
      if (inside || (prio && tThem < tArrive + 2.0) || tClear > tArrive - 0.2) {
        blocked = true;
        if (e.v >= 0.5) onlyStationary = false;
      }
    }
    // Permissive lefts also wait for oncoming traffic still on its approach lane.
    if (!blocked && c.turn === "left" && prio) {
      const lane = o.from;
      const tMe = timeToCover(sp, distToZone + (z.s1 - z.s0) + me.length, 2.0, c.speedLimit);
      for (let i = 0; i < lane.occCount; i++) {
        const e = lane.occ[i];
        if (e.occ.id === me.id || e.v < 1) continue;
        if (e.occ.intentConnector !== o.id && e.occ.intentConnector !== -1) continue;
        const t = (lane.length - e.s - e.occ.length / 2 + z.os0) / Math.max(e.v, 0.5);
        if (t < tMe + 1.0) {
          blocked = true;
          onlyStationary = false;
          break;
        }
      }
    }
    if (blocked) {
      stopAt = Math.min(stopAt, distToZone);
      break;
    }
  }
  if (stopAt < Infinity && sp < 0.3 && onlyStationary) d.js.connWait += dt;
  return stopAt;
}

export function nodeOf(c: Connector): Node {
  return c.node;
}
