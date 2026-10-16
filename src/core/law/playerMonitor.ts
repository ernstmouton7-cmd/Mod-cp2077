import { angleDiff, DEG } from "../math/scalar";
import { Connector, Lane, LaneHit, Node } from "../road/network";
import { Indication, HeadState } from "../signals/signalController";
import { Vehicle } from "../vehicle/vehicle";
import type { SimContext } from "../world/context";
import { ViolationKind } from "./violations";

/** What the HUD shows about the road ahead of the player. */
export interface RoadInfo {
  roadName: string;
  speedLimit: number;
  onRoad: boolean;
  wrongWay: boolean;
  inJunction: boolean;
  lane: Lane | undefined;
  /** Next controlled junction ahead. */
  ahead: {
    distance: number;
    control: "signal" | "stop" | "yield" | "none";
    head: HeadState | undefined;
    /** Indication for the movement implied by the player's indicator. */
    indication: Indication | undefined;
    nodeName: string;
    camera: boolean;
  } | undefined;
  /** Gap & time headway to the vehicle ahead (for tailgating warnings). */
  leaderGap: number;
  headway: number;
}

export interface Observation {
  kind: ViolationKind;
  /** Node with a red-light camera (for camera witness). */
  cameraNode?: Node;
  speedCamera?: boolean;
}

/**
 * Watches the player's vehicle against the road network: which lane it is
 * in, whether it is going the wrong way, what it did at each junction (red
 * lights, stop signs, signalling, illegal turns), speeding and more. Emits
 * raw observations; the world decides who witnessed them.
 */
export class PlayerMonitor {
  readonly info: RoadInfo = {
    roadName: "",
    speedLimit: 13.9,
    onRoad: true,
    wrongWay: false,
    inJunction: false,
    lane: undefined,
    ahead: undefined,
    leaderGap: Infinity,
    headway: Infinity,
  };
  private hits: LaneHit[] = [];
  private lastLane: Lane | undefined = undefined;
  private wrongWayTime = 0;
  private offRoadTime = 0;
  private speedingTime = 0;
  private darkNoLightsTime = 0;
  private unbeltedTime = 0;
  private tailgateTime = 0;
  private skidTime = 0;
  private emergencyBehindTime = 0;
  private hornHeld = 0;
  private boxStopTime = 0;
  /** Junction traversal tracking. */
  private entryLane: Lane | undefined = undefined;
  private entrySnapshot: Map<number, Indication> = new Map();
  private entryNode: Node | undefined = undefined;
  private minSpeedNearLine = Infinity;
  private stoppedNearLine = false;
  private signalLeftAt = -100;
  private signalRightAt = -100;
  private hazardAt = -100;
  /** Collision bookkeeping for hit-and-run. */
  hitRun: { x: number; y: number; time: number; stoppedFor: number; victim: number } | undefined = undefined;
  seatbelt = true;

  update(dt: number, p: Vehicle, ctx: SimContext, out: Observation[]): void {
    const now = ctx.time;
    const info = this.info;
    const speed = p.absSpeed();
    const lights = p.lights;
    if (lights.indicator === "left" || lights.hazard) this.signalLeftAt = now;
    if (lights.indicator === "right" || lights.hazard) this.signalRightAt = now;
    if (lights.hazard) this.hazardAt = now;

    // --- Which lane are we in?
    this.hits = [];
    ctx.net.nearestLanes(p.x, p.y, 7, this.hits);
    let aligned: LaneHit | undefined = undefined;
    let opposite: LaneHit | undefined = undefined;
    for (const h of this.hits) {
      const half = h.lane.width / 2 + 0.4;
      if (Math.abs(h.lateral) > half || h.s <= 0.01 || h.s >= h.lane.length - 0.01) continue;
      const d = Math.abs(angleDiff(p.heading, h.lane.path.headingAt(h.s)));
      if (d < 70 * DEG && aligned === undefined) aligned = h;
      if (d > 110 * DEG && opposite === undefined) opposite = h;
    }
    const node = ctx.net.nodeAt(p.x, p.y);
    info.inJunction = node !== undefined;
    const lane = aligned !== undefined ? aligned.lane : opposite !== undefined ? opposite.lane : undefined;
    info.lane = aligned !== undefined ? aligned.lane : undefined;
    const nearRoad = this.hits.length > 0 && this.hits[0].dist < this.hits[0].lane.width / 2 + 1.3;
    info.onRoad = lane !== undefined || node !== undefined || nearRoad;
    if (lane !== undefined) {
      info.roadName = lane.road.name;
      info.speedLimit = lane.speedLimit;
    }
    // Wrong way: in a lane only in the opposite direction while moving forward.
    const wrong = aligned === undefined && opposite !== undefined && p.speed > 2 && node === undefined;
    info.wrongWay = wrong;
    if (wrong) {
      this.wrongWayTime += dt;
      if (this.wrongWayTime > 1.5) out.push({ kind: "wrong_way" });
    } else this.wrongWayTime = 0;

    // Sidewalk / off-road driving.
    if (!info.onRoad && speed > 4) {
      this.offRoadTime += dt;
      if (this.offRoadTime > 1.0) out.push({ kind: "sidewalk" });
    } else this.offRoadTime = 0;

    // --- Lane changes without signalling.
    const cur = aligned !== undefined ? aligned.lane : undefined;
    if (cur !== undefined && this.lastLane !== undefined && cur !== this.lastLane && cur.road === this.lastLane.road && cur.forward === this.lastLane.forward && speed > 3) {
      const toLeft = cur.index < this.lastLane.index;
      const last = toLeft ? this.signalLeftAt : this.signalRightAt;
      if (now - last > 3) out.push({ kind: "no_signal_lane" });
    }

    // --- Junction traversal.
    if (cur !== undefined) {
      const toLine = cur.length - (aligned as LaneHit).s;
      if (toLine < 18) {
        this.minSpeedNearLine = Math.min(this.minSpeedNearLine, speed);
        if (speed < 0.4 && toLine < 7) this.stoppedNearLine = true;
      }
      if (cur !== this.lastLane && cur.road !== (this.lastLane !== undefined ? this.lastLane.road : undefined)) {
        // Left a junction into a new road: evaluate the movement.
        if (this.entryLane !== undefined && this.entryNode !== undefined && cur.from === this.entryNode) this.evaluateMovement(ctx, cur, out);
        this.entryLane = undefined;
        this.entryNode = undefined;
        this.minSpeedNearLine = Infinity;
        this.stoppedNearLine = false;
      }
      this.lastLane = cur;
    }
    if (node !== undefined && this.entryLane === undefined && this.lastLane !== undefined && this.lastLane.to === node) {
      this.entryLane = this.lastLane;
      this.entryNode = node;
      this.entrySnapshot = new Map();
      if (node.control === "signal") {
        const sig = ctx.signals[node.signalId];
        for (const ap of node.approaches) for (const l of ap.lanes) for (const c of l.outgoing) this.entrySnapshot.set(c.id, sig.indication(c));
      }
    }
    // Stopped inside the box while our movement is red.
    if (node !== undefined && speed < 0.5 && this.entryLane !== undefined && node.control === "signal") {
      const sig = ctx.signals[node.signalId];
      let anyGreen = false;
      for (const c of this.entryLane.outgoing) if (sig.indication(c) === "green" || sig.indication(c) === "permissive") anyGreen = true;
      if (!anyGreen) {
        this.boxStopTime += dt;
        if (this.boxStopTime > 4) out.push({ kind: "blocking_box" });
      } else this.boxStopTime = 0;
    } else this.boxStopTime = 0;

    // --- Speeding.
    if (lane !== undefined && node === undefined) {
      const lim = lane.speedLimit;
      const over = (speed - lim) / lim;
      if (over > 0.1 && speed - lim > 1) {
        this.speedingTime += dt;
        if (this.speedingTime > 2) {
          const kind: ViolationKind = over > 0.5 ? "speeding_extreme" : over > 0.25 ? "speeding_major" : "speeding_minor";
          out.push({ kind, speedCamera: lane.road.speedCamera && this.nearSpeedCamera(lane, aligned !== undefined ? aligned.s : 0) });
        }
      } else this.speedingTime = 0;
    }

    // --- Equipment.
    if (ctx.env.dark && !lights.anyHeadlight() && speed > 3) {
      this.darkNoLightsTime += dt;
      if (this.darkNoLightsTime > 10) out.push({ kind: "no_headlights" });
    } else this.darkNoLightsTime = 0;
    if (!this.seatbelt && speed > 5.5) {
      this.unbeltedTime += dt;
      if (this.unbeltedTime > 5) out.push({ kind: "seatbelt" });
    } else this.unbeltedTime = 0;
    if (lights.horn) {
      this.hornHeld += dt;
      if (this.hornHeld > 4) out.push({ kind: "horn_abuse" });
    } else this.hornHeld = Math.max(0, this.hornHeld - dt * 0.5);

    // --- Tailgating (time headway < 0.6 s at speed).
    info.leaderGap = Infinity;
    info.headway = Infinity;
    if (cur !== undefined && aligned !== undefined) {
      let gap = Infinity;
      for (let i = 0; i < cur.occCount; i++) {
        const e = cur.occ[i];
        if (e.occ.id === p.id) continue;
        const d = e.s - aligned.s - (e.occ.length + p.length) / 2;
        if (d > -0.5 && d < gap && Math.abs(e.lat - aligned.lateral) < 2.2) gap = d;
      }
      info.leaderGap = gap;
      info.headway = speed > 1 ? gap / speed : Infinity;
      if (speed > 12 && info.headway < 0.6) {
        this.tailgateTime += dt;
        if (this.tailgateTime > 4) out.push({ kind: "tailgating" });
      } else this.tailgateTime = 0;
    }

    // --- Reckless: sustained sliding at speed (drifting, donuts, burnouts).
    if (p.dyn.skid > 0.6 && speed > 6) {
      this.skidTime += dt;
      if (this.skidTime > 2.0) out.push({ kind: "reckless" });
    } else this.skidTime = Math.max(0, this.skidTime - dt);

    // --- Emergency vehicles behind with siren.
    let emergencyBehind = false;
    for (const e of ctx.vehicles) {
      if (!e.sirenOn() || e === p) continue;
      const dx = e.x - p.x;
      const dy = e.y - p.y;
      if (dx * dx + dy * dy > 45 * 45) continue;
      const behind = dx * Math.cos(p.heading) + dy * Math.sin(p.heading) < -3;
      if (behind && Math.abs(angleDiff(p.heading, e.heading)) < 45 * DEG && e.absSpeed() > 4) emergencyBehind = true;
    }
    if (emergencyBehind && speed > 4) {
      this.emergencyBehindTime += dt;
      if (this.emergencyBehindTime > 7) out.push({ kind: "fail_yield_emergency" });
    } else this.emergencyBehindTime = Math.max(0, this.emergencyBehindTime - dt * 2);

    // --- Hit and run.
    const hr = this.hitRun;
    if (hr !== undefined) {
      if (speed < 0.5) hr.stoppedFor += dt;
      const dx = p.x - hr.x;
      const dy = p.y - hr.y;
      if (hr.stoppedFor > 3 || now - hr.time > 40) this.hitRun = undefined;
      else if (dx * dx + dy * dy > 90 * 90) {
        out.push({ kind: "hit_and_run" });
        this.hitRun = undefined;
      }
    }

    this.updateAhead(ctx, p, cur, aligned);
  }

  /** Recently used the hazards (a "thank you" blink)? */
  hazardThanks(now: number): boolean {
    return now - this.hazardAt < 1.5;
  }

  private nearSpeedCamera(lane: Lane, s: number): boolean {
    const mid = lane.length / 2;
    return Math.abs(s - mid) < 30;
  }

  private evaluateMovement(ctx: SimContext, exitLane: Lane, out: Observation[]): void {
    const entry = this.entryLane as Lane;
    const node = this.entryNode as Node;
    let c: Connector | undefined = entry.connectorTo(exitLane.road);
    if (c === undefined) {
      // Not allowed from our lane: allowed from another lane of the approach?
      let any: Connector | undefined = undefined;
      const ap = entry.approach;
      if (ap !== undefined) for (const l of ap.lanes) {
        const k = l.connectorTo(exitLane.road);
        if (k !== undefined) any = k;
      }
      if (any === undefined) {
        out.push({ kind: "illegal_turn" });
        return;
      }
      c = any;
    }
    const now = ctx.time;
    if (c.turn === "left" && now - this.signalLeftAt > 6) out.push({ kind: "no_signal_turn" });
    if (c.turn === "right" && now - this.signalRightAt > 6) out.push({ kind: "no_signal_turn" });
    const ap = entry.approach;
    if (node.control === "signal") {
      const ind = this.entrySnapshot.get(c.id);
      if (ind === "red") {
        const sig = ctx.signals[node.signalId];
        if (c.turn === "right" && sig.allowRightOnRed) {
          if (!this.stoppedNearLine) out.push({ kind: "right_on_red_no_stop" });
        } else out.push({ kind: "red_light", cameraNode: node.redLightCamera ? node : undefined });
      } else if ((ind === "flash_red" || ind === "dark") && this.minSpeedNearLine > 1.5) out.push({ kind: "stop_sign" });
    } else if (ap !== undefined && ap.sign === "stop" && this.minSpeedNearLine > 1.4) {
      out.push({ kind: "stop_sign" });
    }
  }

  private updateAhead(ctx: SimContext, p: Vehicle, cur: Lane | undefined, hit: LaneHit | undefined): void {
    const info = this.info;
    if (cur === undefined || hit === undefined) {
      if (!info.inJunction) info.ahead = undefined;
      return;
    }
    const dist = cur.length - hit.s;
    const ap = cur.approach;
    if (ap === undefined || dist > 160) {
      info.ahead = undefined;
      return;
    }
    const node = ap.node;
    let control: "signal" | "stop" | "yield" | "none" = "none";
    let head: HeadState | undefined = undefined;
    let indication: Indication | undefined = undefined;
    if (node.control === "signal") {
      control = "signal";
      const sig = ctx.signals[node.signalId];
      head = sig.head(ap);
      const want = p.lights.indicator === "left" ? "left" : p.lights.indicator === "right" ? "right" : "straight";
      let c: Connector | undefined = undefined;
      for (const k of cur.outgoing) if (k.turn === want) c = k;
      if (c === undefined && cur.outgoing.length > 0) c = cur.outgoing[0];
      if (c !== undefined) indication = sig.indication(c);
    } else if (ap.sign === "stop") control = "stop";
    else if (ap.sign === "yield") control = "yield";
    info.ahead = { distance: dist, control, head, indication, nodeName: node.name, camera: node.redLightCamera };
  }
}
