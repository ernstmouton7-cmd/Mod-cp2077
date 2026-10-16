import { segmentIntersect } from "../math/path";
import { Connector, Network, Node, Road } from "./network";

/** A marked pedestrian crossing across one road at a junction. */
export interface Crosswalk {
  readonly id: number;
  readonly node: Node;
  readonly road: Road;
  /** End points (sim coords): a = right curb of traffic leaving the node, b = left curb. */
  readonly ax: number;
  readonly ay: number;
  readonly bx: number;
  readonly by: number;
  readonly length: number;
  /** Heading of the road leaving the node (traffic on it crosses this walk). */
  readonly roadHeading: number;
}

/** Where a connector path crosses a crosswalk. */
export interface CrosswalkCrossing {
  cw: Crosswalk;
  /** Arc length on the connector. */
  s: number;
  /** Position along the crosswalk 0 (a) .. 1 (b). */
  t: number;
}

/**
 * Builds crosswalks at every controlled junction (signals and signs) and the
 * points where each connector's path crosses them. Crosswalk geometry matches
 * the zebra band drawn by renderers: 2.6 m inside the junction box radius.
 */
export function buildCrosswalks(net: Network): { crosswalks: Crosswalk[]; crossings: Map<number, CrosswalkCrossing[]> } {
  const crosswalks: Crosswalk[] = [];
  const crossings = new Map<number, CrosswalkCrossing[]>();
  for (const n of net.nodes) {
    if (n.roads.length < 3 || n.control === "none") continue;
    for (const r of n.roads) {
      const h = r.headingFrom(n);
      const dx = Math.cos(h);
      const dy = Math.sin(h);
      const px = -dy;
      const py = dx;
      const mid = n.radius - 2.6;
      const half = r.width / 2 + 0.6;
      const cx = n.x + dx * mid;
      const cy = n.y + dy * mid;
      crosswalks.push({
        id: crosswalks.length,
        node: n,
        road: r,
        ax: cx - px * half,
        ay: cy - py * half,
        bx: cx + px * half,
        by: cy + py * half,
        length: half * 2,
        roadHeading: h,
      });
    }
  }
  for (const c of net.connectors) {
    const list: CrosswalkCrossing[] = [];
    for (const cw of crosswalks) {
      if (cw.node !== c.node) continue;
      const hit = firstCrossing(c, cw);
      if (hit !== undefined) list.push(hit);
    }
    list.sort((a, b) => (a.s !== b.s ? a.s - b.s : a.cw.id - b.cw.id));
    if (list.length > 0) crossings.set(c.id, list);
  }
  return { crosswalks, crossings };
}

function firstCrossing(c: Connector, cw: Crosswalk): CrosswalkCrossing | undefined {
  const p = c.path;
  for (let i = 0; i < p.xs.length - 1; i++) {
    const hit = segmentIntersect(p.xs[i], p.ys[i], p.xs[i + 1], p.ys[i + 1], cw.ax, cw.ay, cw.bx, cw.by);
    if (hit !== undefined) return { cw, s: p.cum[i] + hit.t * (p.cum[i + 1] - p.cum[i]), t: hit.u };
  }
  return undefined;
}
