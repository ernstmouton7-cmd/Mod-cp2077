import { Path, PathSample, cubicBezier, offsetPolyline } from "../math/path";
import { Vec2 } from "../math/vec";
import { angleDiff, clamp, DEG, KMH } from "../math/scalar";
import { SpatialHash } from "../util/spatialHash";
import { ApproachSign, ControlType, OccEntry, RoadKind, Segment, TurnType } from "./types";

/** Region where two connectors come too close to be used at the same time. */
export interface ConflictZone {
  /** Connector we conflict with. */
  other: Connector;
  /** Zone on this connector [s0, s1]. */
  s0: number;
  s1: number;
  /** Zone on the other connector. */
  os0: number;
  os1: number;
  /** "merge": both end in the same lane; "cross": paths intersect. */
  kind: "merge" | "cross";
}

export interface RoadOptions {
  /** Lanes travelling from node A to node B. */
  lanesFwd: number;
  /** Lanes travelling from node B to node A (0 = one-way road). */
  lanesBack: number;
  laneWidth?: number;
  /** km/h */
  speedLimitKmh: number;
  kind: RoadKind;
  name: string;
  medianWidth?: number;
  sidewalkWidth?: number;
  /** Optional interior points for curved roads (between A and B). */
  via?: Vec2[];
}

export class Node {
  readonly roads: Road[] = [];
  readonly approaches: Approach[] = [];
  readonly connectors: Connector[] = [];
  /** Junction "box" radius: incoming lanes end this far from the centre. */
  radius = 0;
  /** Id of the signal controller (if control === "signal"). */
  signalId = -1;
  /** Optional enforcement cameras. */
  redLightCamera = false;
  name = "";

  constructor(
    readonly id: number,
    readonly x: number,
    readonly y: number,
    public control: ControlType,
  ) {}

  /** True for a plain bend/merge with no crossing traffic. */
  isTrivial(): boolean {
    return this.roads.length <= 2;
  }
}

export class Road {
  readonly lanesAB: Lane[] = [];
  readonly lanesBA: Lane[] = [];
  /** Full centreline polyline from A to B (untrimmed). */
  readonly centerline: Vec2[];
  readonly centerPath: Path;
  readonly laneWidth: number;
  readonly medianWidth: number;
  readonly sidewalkWidth: number;
  speedLimit: number;
  trimA = 0;
  trimB = 0;
  /** Has a speed camera somewhere along it. */
  speedCamera = false;
  readonly kind: RoadKind;
  readonly name: string;
  /** Total carriageway width. */
  readonly width: number;

  constructor(
    readonly id: number,
    readonly a: Node,
    readonly b: Node,
    readonly opts: RoadOptions,
  ) {
    const pts: Vec2[] = [{ x: a.x, y: a.y }];
    if (opts.via !== undefined) for (const p of opts.via) pts.push({ x: p.x, y: p.y });
    pts.push({ x: b.x, y: b.y });
    this.centerline = pts;
    this.centerPath = new Path(pts);
    this.laneWidth = opts.laneWidth ?? 3.5;
    this.medianWidth = opts.medianWidth ?? (opts.lanesBack > 0 && opts.lanesFwd > 0 ? 0.3 : 0);
    this.sidewalkWidth = opts.sidewalkWidth ?? 3;
    this.speedLimit = opts.speedLimitKmh * KMH;
    this.kind = opts.kind;
    this.name = opts.name;
    const n = opts.lanesFwd + opts.lanesBack;
    const m = opts.lanesBack === 0 || opts.lanesFwd === 0 ? 0 : this.medianWidth;
    this.width = n * this.laneWidth + m;
  }

  /** Lanes arriving at node n. */
  lanesInto(n: Node): Lane[] {
    return n === this.b ? this.lanesAB : this.lanesBA;
  }

  /** Lanes departing from node n. */
  lanesOutOf(n: Node): Lane[] {
    return n === this.a ? this.lanesAB : this.lanesBA;
  }

  other(n: Node): Node {
    return n === this.a ? this.b : this.a;
  }

  /** Heading of the road when leaving node n. */
  headingFrom(n: Node): number {
    const pts = this.centerline;
    if (n === this.a) return Math.atan2(pts[1].y - pts[0].y, pts[1].x - pts[0].x);
    const k = pts.length - 1;
    return Math.atan2(pts[k - 1].y - pts[k].y, pts[k - 1].x - pts[k].x);
  }
}

export class Lane implements Segment {
  readonly kind = "lane" as const;
  readonly outgoing: Connector[] = [];
  readonly incoming: Connector[] = [];
  /** Adjacent lane closer to the road centre (driver's left). */
  left: Lane | undefined = undefined;
  /** Adjacent lane closer to the curb (driver's right). */
  right: Lane | undefined = undefined;
  approach: Approach | undefined = undefined;
  /** Allowed turns at the end of the lane. */
  readonly turns: TurnType[] = [];
  occ: OccEntry[] = [];
  occCount = 0;
  speedLimit: number;
  readonly length: number;

  constructor(
    readonly id: number,
    readonly road: Road,
    /** true = travels A->B */
    readonly forward: boolean,
    /** 0 = innermost (next to the centre line / leftmost) */
    readonly index: number,
    readonly from: Node,
    readonly to: Node,
    readonly path: Path,
    readonly width: number,
  ) {
    this.speedLimit = road.speedLimit;
    this.length = path.length;
  }

  /** Lanes of the same road & direction, index 0 = leftmost. */
  siblings(): Lane[] {
    return this.forward ? this.road.lanesAB : this.road.lanesBA;
  }

  hasTurn(t: TurnType): boolean {
    return this.turns.indexOf(t) >= 0;
  }

  connectorTo(road: Road): Connector | undefined {
    for (const c of this.outgoing) if (c.to.road === road) return c;
    return undefined;
  }
}

export class Connector implements Segment {
  readonly kind = "connector" as const;
  readonly conflicts: ConflictZone[] = [];
  readonly approach: Approach;
  /** Connectors starting from the same lane (diverging). */
  readonly siblings: Connector[] = [];
  /** For each sibling: arc length (on this connector) where the paths have separated. */
  readonly siblingDiverge: number[] = [];
  occ: OccEntry[] = [];
  occCount = 0;
  speedLimit: number;
  readonly length: number;

  constructor(
    readonly id: number,
    readonly node: Node,
    readonly from: Lane,
    readonly to: Lane,
    readonly turn: TurnType,
    readonly path: Path,
  ) {
    this.length = path.length;
    this.approach = from.approach as Approach;
    // Comfortable lateral acceleration ~2.5 m/s^2 through the turn.
    let maxCurv = 0;
    const steps = Math.max(2, Math.floor(path.length / 2));
    for (let i = 1; i < steps; i++) {
      const c = Math.abs(path.curvatureAt((path.length * i) / steps, 1.5));
      if (c > maxCurv) maxCurv = c;
    }
    const turnSpeed = maxCurv > 1e-4 ? Math.sqrt(2.6 / maxCurv) : 99;
    this.speedLimit = Math.min(Math.max(from.speedLimit, to.speedLimit), Math.max(3.5, turnSpeed));
  }

}

/** The set of lanes entering a node from one road. Signals and signs attach here. */
export class Approach {
  readonly lanes: Lane[] = [];
  sign: ApproachSign = "none";
  /** Major road in a two-way-stop/yield intersection. */
  major = false;
  /** Signal group index within the node's controller (-1 = none). */
  group = -1;
  /** Stop line endpoints (curb side and centre side). */
  stopA: Vec2 = { x: 0, y: 0 };
  stopB: Vec2 = { x: 0, y: 0 };
  /** Direction of travel when entering the node. */
  heading = 0;

  constructor(
    readonly id: number,
    readonly node: Node,
    readonly road: Road,
  ) {}
}

export interface LaneHit {
  lane: Lane;
  s: number;
  lateral: number;
  dist: number;
}

const scratch: PathSample = { x: 0, y: 0, heading: 0 };

/**
 * The drivable network: nodes, roads, lanes and intersection connectors.
 * Build with addNode/addRoad, then call finalize() once.
 */
export class Network {
  readonly nodes: Node[] = [];
  readonly roads: Road[] = [];
  readonly lanes: Lane[] = [];
  readonly connectors: Connector[] = [];
  readonly approaches: Approach[] = [];
  /** lanes and connectors in id order (ids are indices). */
  readonly segments: Segment[] = [];
  private laneHash: SpatialHash<Lane> = new SpatialHash<Lane>(40);
  private connectorHash: SpatialHash<Connector> = new SpatialHash<Connector>(40);
  finalized = false;
  minX = Infinity;
  minY = Infinity;
  maxX = -Infinity;
  maxY = -Infinity;

  addNode(x: number, y: number, control: ControlType): Node {
    const n = new Node(this.nodes.length, x, y, control);
    this.nodes.push(n);
    return n;
  }

  addRoad(a: Node, b: Node, opts: RoadOptions): Road {
    const r = new Road(this.roads.length, a, b, opts);
    this.roads.push(r);
    a.roads.push(r);
    b.roads.push(r);
    return r;
  }

  finalize(): void {
    if (this.finalized) return;
    this.computeNodeRadii();
    for (const r of this.roads) this.buildLanes(r);
    for (const n of this.nodes) this.buildApproaches(n);
    for (const n of this.nodes) this.buildConnectors(n);
    for (const n of this.nodes) this.buildConflicts(n);
    for (const l of this.lanes) {
      const pts = l.path.points();
      for (let i = 0; i < pts.length; i++) {
        const p = pts[i];
        this.minX = Math.min(this.minX, p.x);
        this.minY = Math.min(this.minY, p.y);
        this.maxX = Math.max(this.maxX, p.x);
        this.maxY = Math.max(this.maxY, p.y);
      }
      this.hashPath(l.path, l, this.laneHash);
    }
    for (const c of this.connectors) this.hashPath(c.path, c, this.connectorHash);
    this.finalized = true;
  }

  private hashPath<T>(path: Path, item: T, hash: SpatialHash<T>): void {
    const step = 20;
    const n = Math.max(1, Math.ceil(path.length / step));
    for (let i = 0; i <= n; i++) {
      path.sample((path.length * i) / n, scratch);
      hash.insert(item, scratch.x, scratch.y, 12);
    }
  }

  private computeNodeRadii(): void {
    for (const n of this.nodes) {
      let maxHalf = 0;
      for (const r of n.roads) maxHalf = Math.max(maxHalf, r.width / 2);
      if (n.roads.length <= 2) {
        // Bends: small box so the curve stays close to the corner.
        n.radius = n.roads.length === 2 ? Math.max(4, maxHalf * 0.9) : 2;
      } else {
        // Room for the crosswalk band between stop line and conflict area.
        n.radius = maxHalf + 4.5;
      }
    }
    for (const r of this.roads) {
      r.trimA = r.a.radius;
      r.trimB = r.b.radius;
      // Never trim more than the road allows.
      const maxTrim = r.centerPath.length * 0.45;
      if (r.trimA > maxTrim) r.trimA = maxTrim;
      if (r.trimB > maxTrim) r.trimB = maxTrim;
    }
  }

  private buildLanes(r: Road): void {
    const nF = r.opts.lanesFwd;
    const nB = r.opts.lanesBack;
    const w = r.laneWidth;
    const oneWay = nF === 0 || nB === 0;
    const halfMedian = oneWay ? 0 : r.medianWidth / 2;
    const fullLen = r.centerPath.length;
    // Forward lanes: offset to the right of A->B (negative = right).
    for (let i = 0; i < nF; i++) {
      const off = oneWay ? -(i + 0.5) * w + (nF * w) / 2 : -(halfMedian + (i + 0.5) * w);
      const pts = offsetPolyline(r.centerline, off);
      const full = new Path(pts);
      const trimmed = new Path(full.slice(r.trimA * (full.length / fullLen), full.length - r.trimB * (full.length / fullLen)));
      const lane = new Lane(this.segments.length, r, true, i, r.a, r.b, trimmed, w);
      this.segments.push(lane);
      this.lanes.push(lane);
      r.lanesAB.push(lane);
    }
    const rev: Vec2[] = [];
    for (let i = r.centerline.length - 1; i >= 0; i--) rev.push(r.centerline[i]);
    for (let i = 0; i < nB; i++) {
      const off = oneWay ? -(i + 0.5) * w + (nB * w) / 2 : -(halfMedian + (i + 0.5) * w);
      const pts = offsetPolyline(rev, off);
      const full = new Path(pts);
      const trimmed = new Path(full.slice(r.trimB * (full.length / fullLen), full.length - r.trimA * (full.length / fullLen)));
      const lane = new Lane(this.segments.length, r, false, i, r.b, r.a, trimmed, w);
      this.segments.push(lane);
      this.lanes.push(lane);
      r.lanesBA.push(lane);
    }
    for (const group of [r.lanesAB, r.lanesBA]) {
      for (let i = 0; i < group.length; i++) {
        group[i].left = i > 0 ? group[i - 1] : undefined;
        group[i].right = i < group.length - 1 ? group[i + 1] : undefined;
      }
    }
  }

  private buildApproaches(n: Node): void {
    for (const r of n.roads) {
      const lanes = r.lanesInto(n);
      if (lanes.length === 0) continue;
      const ap = new Approach(this.approaches.length, n, r);
      this.approaches.push(ap);
      n.approaches.push(ap);
      for (const l of lanes) {
        ap.lanes.push(l);
        l.approach = ap;
      }
      const first = lanes[0];
      const last = lanes[lanes.length - 1];
      first.path.sample(first.length, scratch);
      ap.heading = scratch.heading;
      const nx = -Math.sin(ap.heading);
      const ny = Math.cos(ap.heading);
      ap.stopB = { x: scratch.x + nx * first.width * 0.5, y: scratch.y + ny * first.width * 0.5 };
      last.path.sample(last.length, scratch);
      ap.stopA = { x: scratch.x - nx * last.width * 0.5, y: scratch.y - ny * last.width * 0.5 };
    }
  }

  private turnBetween(inHeading: number, outHeading: number): TurnType {
    const d = angleDiff(inHeading, outHeading);
    if (Math.abs(d) > 150 * DEG) return "uturn";
    if (d > 35 * DEG) return "left";
    if (d < -35 * DEG) return "right";
    return "straight";
  }

  private buildConnectors(n: Node): void {
    for (const ap of n.approaches) {
      const inLanes = ap.lanes;
      const k = inLanes.length;
      // Classify outgoing roads by turn type.
      const outs: { road: Road; turn: TurnType }[] = [];
      for (const r of n.roads) {
        if (r === ap.road && n.roads.length > 1) continue;
        if (r.lanesOutOf(n).length === 0) continue;
        const t = this.turnBetween(ap.heading, r.headingFrom(n));
        if (t === "uturn" && n.roads.length > 1) continue;
        outs.push({ road: r, turn: t });
      }
      let hasL = false;
      let hasS = false;
      let hasR = false;
      for (const o of outs) {
        if (o.turn === "left") hasL = true;
        else if (o.turn === "straight") hasS = true;
        else if (o.turn === "right") hasR = true;
      }
      // Lane-use assignment (right-hand traffic).
      for (let i = 0; i < k; i++) {
        const t = inLanes[i].turns;
        if (hasS) {
          const dedicatedLeft = k >= 3 && hasL;
          if (!(dedicatedLeft && i === 0)) t.push("straight");
          if (hasL && i === 0) t.push("left");
          if (hasR && i === k - 1) t.push("right");
        } else if (hasL && hasR) {
          if (k === 1) {
            t.push("left");
            t.push("right");
          } else if (i < Math.ceil(k / 2)) t.push("left");
          else t.push("right");
        } else if (hasL) t.push("left");
        else if (hasR) t.push("right");
        for (const o of outs) if (o.turn === "uturn") t.push("uturn");
      }
      // Create connectors.
      for (const o of outs) {
        const outLanes = o.road.lanesOutOf(n);
        const m = outLanes.length;
        const users: Lane[] = [];
        for (const l of inLanes) if (l.hasTurn(o.turn)) users.push(l);
        for (let u = 0; u < users.length; u++) {
          const from = users[u];
          let target: Lane;
          if (o.turn === "left" || o.turn === "uturn") target = outLanes[clamp(u, 0, m - 1)];
          else if (o.turn === "right") target = outLanes[clamp(m - users.length + u, 0, m - 1)];
          else {
            // Straight: keep lane index; when lanes drop, the inner ones merge.
            if (m >= k) target = outLanes[clamp(from.index, 0, m - 1)];
            else target = outLanes[clamp(from.index - (k - m), 0, m - 1)];
          }
          const c = this.makeConnector(n, from, target, o.turn);
          from.outgoing.push(c);
          target.incoming.push(c);
          n.connectors.push(c);
        }
      }
    }
    const tmp = { s: 0, lateral: 0, d2: 0 };
    for (const c of n.connectors) {
      for (const d of c.from.outgoing) {
        if (d === c) continue;
        c.siblings.push(d);
        // Walk along c until d is more than a car width away.
        let sep = c.length;
        const steps = Math.max(4, Math.ceil(c.length / 0.5));
        for (let k = 1; k <= steps; k++) {
          const sk = (c.length * k) / steps;
          c.path.sample(sk, scratch);
          d.path.project(scratch.x, scratch.y, tmp);
          if (tmp.d2 > 2.3 * 2.3) {
            sep = sk;
            break;
          }
        }
        c.siblingDiverge.push(sep);
      }
    }
  }

  private makeConnector(n: Node, from: Lane, to: Lane, turn: TurnType): Connector {
    from.path.sample(from.length, scratch);
    const p0 = { x: scratch.x, y: scratch.y };
    const h0 = scratch.heading;
    to.path.sample(0, scratch);
    const p3 = { x: scratch.x, y: scratch.y };
    const h3 = scratch.heading;
    const dx = p3.x - p0.x;
    const dy = p3.y - p0.y;
    const d = Math.sqrt(dx * dx + dy * dy);
    let pts: Vec2[];
    if (turn === "straight" && Math.abs(angleDiff(h0, h3)) < 5 * DEG) {
      pts = [p0, p3];
    } else {
      const k = turn === "uturn" ? Math.max(4, d * 1.2) : d * 0.45;
      const p1 = { x: p0.x + Math.cos(h0) * k, y: p0.y + Math.sin(h0) * k };
      const p2 = { x: p3.x - Math.cos(h3) * k, y: p3.y - Math.sin(h3) * k };
      pts = cubicBezier(p0, p1, p2, p3, 16);
    }
    const c = new Connector(this.segments.length, n, from, to, turn, new Path(pts));
    this.segments.push(c);
    this.connectors.push(c);
    return c;
  }

  private buildConflicts(n: Node): void {
    const cs = n.connectors;
    const tmp = { s: 0, lateral: 0, d2: 0 };
    for (let i = 0; i < cs.length; i++) {
      for (let j = i + 1; j < cs.length; j++) {
        const a = cs[i];
        const b = cs[j];
        if (a.from === b.from) continue; // diverging: handled as siblings
        const merge = a.to === b.to;
        // Sample a, find the zone where b is closer than the clearance.
        const clearance = 2.4;
        let s0 = -1;
        let s1 = -1;
        let os0 = Infinity;
        let os1 = -Infinity;
        const steps = Math.max(4, Math.ceil(a.length / 0.75));
        for (let k = 0; k <= steps; k++) {
          const s = (a.length * k) / steps;
          a.path.sample(s, scratch);
          b.path.project(scratch.x, scratch.y, tmp);
          if (tmp.d2 < clearance * clearance && tmp.s > 0.01 && tmp.s < b.length + 0.01) {
            if (s0 < 0) s0 = s;
            s1 = s;
            if (tmp.s < os0) os0 = tmp.s;
            if (tmp.s > os1) os1 = tmp.s;
          }
        }
        if (s0 < 0) continue;
        // Paths that only share their final point were caught as merges; make
        // sure merge zones extend to the end of both connectors.
        const kind: "merge" | "cross" = merge ? "merge" : "cross";
        if (merge) {
          s1 = a.length;
          os1 = b.length;
        }
        a.conflicts.push({ other: b, s0: Math.max(0, s0 - 1), s1: Math.min(a.length, s1 + 1), os0: Math.max(0, os0 - 1), os1: Math.min(b.length, os1 + 1), kind });
        b.conflicts.push({ other: a, s0: Math.max(0, os0 - 1), s1: Math.min(b.length, os1 + 1), os0: Math.max(0, s0 - 1), os1: Math.min(a.length, s1 + 1), kind });
      }
    }
  }

  /** Lanes near a point, ordered by distance. */
  nearestLanes(x: number, y: number, radius: number, out: LaneHit[]): LaneHit[] {
    const cands: Lane[] = [];
    this.laneHash.query(x, y, radius, cands);
    const tmp = { s: 0, lateral: 0, d2: 0 };
    for (const l of cands) {
      l.path.project(x, y, tmp);
      const d = Math.sqrt(tmp.d2);
      if (d <= radius) out.push({ lane: l, s: tmp.s, lateral: tmp.lateral, dist: d });
    }
    // Insertion sort: tiny arrays, and no closure allocation (LuaJIT-friendly).
    for (let i = 1; i < out.length; i++) {
      const h = out[i];
      let j = i - 1;
      while (j >= 0 && (out[j].dist > h.dist || (out[j].dist === h.dist && out[j].lane.id > h.lane.id))) {
        out[j + 1] = out[j];
        j--;
      }
      out[j + 1] = h;
    }
    return out;
  }

  nearestConnectors(x: number, y: number, radius: number, out: Connector[]): Connector[] {
    return this.connectorHash.query(x, y, radius, out);
  }

  /** Node whose junction box contains the point, if any. */
  nodeAt(x: number, y: number): Node | undefined {
    let best: Node | undefined = undefined;
    let bestD = Infinity;
    for (const n of this.nodes) {
      if (n.roads.length < 3) continue;
      const dx = x - n.x;
      const dy = y - n.y;
      const d = dx * dx + dy * dy;
      const r = n.radius - 1;
      if (d < r * r && d < bestD) {
        bestD = d;
        best = n;
      }
    }
    return best;
  }

  /** Reset occupancy of all segments (start of a simulation step). */
  clearOccupancy(): void {
    const segs = this.segments;
    for (let i = 0; i < segs.length; i++) segs[i].occCount = 0;
  }
}

/** Append an occupancy entry to a segment, reusing pooled records. */
export function addOccupant(seg: Segment, occ: OccEntry["occ"], s: number, v: number, phantom: boolean, lat: number): void {
  const i = seg.occCount;
  if (i < seg.occ.length) {
    const e = seg.occ[i];
    e.occ = occ;
    e.s = s;
    e.v = v;
    e.phantom = phantom;
    e.lat = lat;
  } else {
    seg.occ.push({ occ, s, v, phantom, lat });
  }
  seg.occCount = i + 1;
}
