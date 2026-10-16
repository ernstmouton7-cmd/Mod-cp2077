import { Lane, Network, Node, Road } from "./network";

/** A step of a route: drive along `road` in the given direction, arriving at `to`. */
export interface RouteStep {
  road: Road;
  forward: boolean;
  to: Node;
}

interface OpenEntry {
  f: number;
  g: number;
  node: Node;
  /** Road we arrived by (for turn costs). */
  via: Road | undefined;
  parent: OpenEntry | undefined;
  stepRoad: Road | undefined;
  stepForward: boolean;
}

/**
 * Road-level A* with turn penalties and optional congestion costs.
 * Routes are lists of road steps after the vehicle's current lane.
 */
export class Router {
  /** Extra seconds per road from congestion; refreshed by the world periodically. */
  readonly congestion: Map<number, number> = new Map();
  private maxSpeed = 1;

  constructor(readonly net: Network) {
    for (const r of net.roads) this.maxSpeed = Math.max(this.maxSpeed, r.speedLimit);
  }

  private controlDelay(n: Node): number {
    switch (n.control) {
      case "signal":
        return 12;
      case "allway_stop":
        return 7;
      case "twoway_stop":
        return 4;
      default:
        return 0.5;
    }
  }

  /**
   * Route from the end of `startLane` (its `to` node) to `goalRoad`.
   * Returns undefined if unreachable.
   */
  route(startLane: Lane, goalRoad: Road, goalForward: boolean): RouteStep[] | undefined {
    const goalNodeFrom = goalForward ? goalRoad.a : goalRoad.b;
    const goalTo = goalForward ? goalRoad.b : goalRoad.a;
    const start = startLane.to;
    const open: OpenEntry[] = [];
    const best: Map<number, number> = new Map();
    const h = (n: Node) => Math.sqrt((n.x - goalNodeFrom.x) * (n.x - goalNodeFrom.x) + (n.y - goalNodeFrom.y) * (n.y - goalNodeFrom.y)) / this.maxSpeed;
    open.push({ f: h(start), g: 0, node: start, via: startLane.road, parent: undefined, stepRoad: undefined, stepForward: true });
    const key = (n: Node, via: Road | undefined) => n.id * 100000 + (via === undefined ? 99999 : via.id);
    let iterations = 0;
    while (open.length > 0 && iterations < 20000) {
      iterations++;
      // Pop lowest f (linear scan; graphs are small).
      let bi = 0;
      for (let i = 1; i < open.length; i++) {
        const a = open[i];
        const b = open[bi];
        if (a.f < b.f || (a.f === b.f && a.node.id < b.node.id)) bi = i;
      }
      const cur = open[bi];
      open[bi] = open[open.length - 1];
      open.pop();
      if (cur.node === goalNodeFrom && cur.via !== goalRoad) {
        // Final step along the goal road, if we may turn into it here.
        if (this.canTurn(cur.node, cur.via, goalRoad, goalForward)) {
          const steps: RouteStep[] = [{ road: goalRoad, forward: goalForward, to: goalTo }];
          let e: OpenEntry | undefined = cur;
          while (e !== undefined && e.stepRoad !== undefined) {
            steps.push({ road: e.stepRoad, forward: e.stepForward, to: e.node });
            e = e.parent;
          }
          steps.reverse();
          return steps;
        }
      }
      for (const r of cur.node.roads) {
        if (r === cur.via && cur.node.roads.length > 1) continue;
        const fwd = r.a === cur.node;
        if (!this.canTurn(cur.node, cur.via, r, fwd)) continue;
        const next = r.other(cur.node);
        const lanes = fwd ? r.lanesAB : r.lanesBA;
        const cost = r.centerPath.length / r.speedLimit + this.controlDelay(next) + (this.congestion.get(r.id) ?? 0) + this.turnCost(cur.node, cur.via, r) + 3 / lanes.length;
        const g = cur.g + cost;
        const k = key(next, r);
        const prev = best.get(k);
        if (prev !== undefined && prev <= g) continue;
        best.set(k, g);
        open.push({ f: g + h(next), g, node: next, via: r, parent: cur, stepRoad: r, stepForward: fwd });
      }
    }
    return undefined;
  }

  /** Is there any lane connection from road `from` into road `to` (direction) at node n? */
  canTurn(n: Node, from: Road | undefined, to: Road, forward: boolean): boolean {
    const outLanes = forward ? to.lanesAB : to.lanesBA;
    if (outLanes.length === 0) return false;
    if ((forward ? to.a : to.b) !== n) return false;
    if (from === undefined) return true;
    const inLanes = from.lanesInto(n);
    for (const l of inLanes) for (const c of l.outgoing) if (c.to.road === to) return true;
    return false;
  }

  private turnCost(n: Node, from: Road | undefined, to: Road): number {
    if (from === undefined) return 0;
    for (const l of from.lanesInto(n)) {
      for (const c of l.outgoing) {
        if (c.to.road === to) return c.turn === "left" ? 8 : c.turn === "right" ? 3 : 0;
      }
    }
    return 0;
  }
}
