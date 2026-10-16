import { Vec2 } from "./vec";

/** Result of sampling a path: position, heading (radians) and the segment index used. */
export interface PathSample {
  x: number;
  y: number;
  heading: number;
}

/**
 * A polyline parameterised by arc length. All lane and connector geometry is
 * stored this way: sampling is a binary search plus a lerp, cheap enough for
 * hundreds of vehicles per frame.
 */
export class Path {
  readonly xs: number[] = [];
  readonly ys: number[] = [];
  /** Cumulative length at each vertex. */
  readonly cum: number[] = [];
  readonly length: number;

  constructor(points: ReadonlyArray<Vec2>) {
    let total = 0;
    for (let i = 0; i < points.length; i++) {
      const p = points[i];
      if (i > 0) {
        const dx = p.x - this.xs[this.xs.length - 1];
        const dy = p.y - this.ys[this.ys.length - 1];
        const d = Math.sqrt(dx * dx + dy * dy);
        if (d < 1e-6) continue;
        total += d;
      }
      this.xs.push(p.x);
      this.ys.push(p.y);
      this.cum.push(total);
    }
    if (this.xs.length === 1) {
      // Degenerate path: duplicate the point so sampling never breaks.
      this.xs.push(this.xs[0] + 1e-3);
      this.ys.push(this.ys[0]);
      this.cum.push(1e-3);
      total = 1e-3;
    }
    this.length = total;
  }

  /** Index i such that cum[i] <= s < cum[i+1]. */
  private seg(s: number): number {
    const cum = this.cum;
    let lo = 0;
    let hi = cum.length - 2;
    if (s <= 0) return 0;
    if (s >= this.length) return hi;
    while (lo < hi) {
      const mid = Math.floor((lo + hi + 1) / 2);
      if (cum[mid] <= s) lo = mid;
      else hi = mid - 1;
    }
    return lo;
  }

  sample(s: number, out: PathSample): PathSample {
    const i = this.seg(s);
    const x0 = this.xs[i];
    const y0 = this.ys[i];
    const x1 = this.xs[i + 1];
    const y1 = this.ys[i + 1];
    const segLen = this.cum[i + 1] - this.cum[i];
    let t = segLen > 0 ? (s - this.cum[i]) / segLen : 0;
    // Allow extrapolation past the ends along the end tangents.
    if (i === 0 && t < 0) t = (s - this.cum[0]) / segLen;
    out.x = x0 + (x1 - x0) * t;
    out.y = y0 + (y1 - y0) * t;
    out.heading = Math.atan2(y1 - y0, x1 - x0);
    return out;
  }

  headingAt(s: number): number {
    const i = this.seg(s);
    return Math.atan2(this.ys[i + 1] - this.ys[i], this.xs[i + 1] - this.xs[i]);
  }

  /** Curvature estimate (1/m) around s, from heading change over a window. */
  curvatureAt(s: number, window: number): number {
    const a = this.headingAt(s - window);
    const b = this.headingAt(s + window);
    let d = b - a;
    while (d > Math.PI) d -= Math.PI * 2;
    while (d < -Math.PI) d += Math.PI * 2;
    return d / (2 * window);
  }

  /**
   * Project a point onto the path. Returns arc length s, signed lateral offset
   * (positive = left of travel direction) and squared distance.
   */
  project(px: number, py: number, out: { s: number; lateral: number; d2: number }): { s: number; lateral: number; d2: number } {
    let best = Infinity;
    let bestS = 0;
    let bestLat = 0;
    const n = this.xs.length;
    for (let i = 0; i < n - 1; i++) {
      const x0 = this.xs[i];
      const y0 = this.ys[i];
      const dx = this.xs[i + 1] - x0;
      const dy = this.ys[i + 1] - y0;
      const l2 = dx * dx + dy * dy;
      let t = l2 > 0 ? ((px - x0) * dx + (py - y0) * dy) / l2 : 0;
      if (t < 0) t = 0;
      else if (t > 1) t = 1;
      const cx = x0 + dx * t;
      const cy = y0 + dy * t;
      const ex = px - cx;
      const ey = py - cy;
      const d2 = ex * ex + ey * ey;
      if (d2 < best) {
        best = d2;
        bestS = this.cum[i] + t * (this.cum[i + 1] - this.cum[i]);
        const l = Math.sqrt(l2);
        bestLat = l > 0 ? (dx * ey - dy * ex) / l : 0;
      }
    }
    out.s = bestS;
    out.lateral = bestLat;
    out.d2 = best;
    return out;
  }

  points(): Vec2[] {
    const r: Vec2[] = [];
    for (let i = 0; i < this.xs.length; i++) r.push({ x: this.xs[i], y: this.ys[i] });
    return r;
  }

  /** Points of a sub-range [s0, s1], useful for rendering trimmed geometry. */
  slice(s0: number, s1: number): Vec2[] {
    const tmp: PathSample = { x: 0, y: 0, heading: 0 };
    const r: Vec2[] = [];
    this.sample(s0, tmp);
    r.push({ x: tmp.x, y: tmp.y });
    for (let i = 0; i < this.cum.length; i++) {
      if (this.cum[i] > s0 && this.cum[i] < s1) r.push({ x: this.xs[i], y: this.ys[i] });
    }
    this.sample(s1, tmp);
    r.push({ x: tmp.x, y: tmp.y });
    return r;
  }
}

/** Offset a polyline sideways by `offset` metres (positive = left). */
export function offsetPolyline(points: ReadonlyArray<Vec2>, offset: number): Vec2[] {
  const n = points.length;
  const out: Vec2[] = [];
  for (let i = 0; i < n; i++) {
    let nx = 0;
    let ny = 0;
    if (i > 0) {
      const dx = points[i].x - points[i - 1].x;
      const dy = points[i].y - points[i - 1].y;
      const l = Math.sqrt(dx * dx + dy * dy);
      if (l > 0) {
        nx += -dy / l;
        ny += dx / l;
      }
    }
    if (i < n - 1) {
      const dx = points[i + 1].x - points[i].x;
      const dy = points[i + 1].y - points[i].y;
      const l = Math.sqrt(dx * dx + dy * dy);
      if (l > 0) {
        nx += -dy / l;
        ny += dx / l;
      }
    }
    const nl = Math.sqrt(nx * nx + ny * ny);
    if (nl > 0) {
      nx /= nl;
      ny /= nl;
    }
    // Miter correction so parallel offsets keep constant width at corners.
    let k = 1;
    if (i > 0 && i < n - 1) {
      const dx = points[i + 1].x - points[i].x;
      const dy = points[i + 1].y - points[i].y;
      const l = Math.sqrt(dx * dx + dy * dy);
      if (l > 0) {
        const c = (nx * -dy + ny * dx) / l;
        if (c > 0.3) k = 1 / c;
      }
    }
    out.push({ x: points[i].x + nx * offset * k, y: points[i].y + ny * offset * k });
  }
  return out;
}

/** Cubic Bezier flattened to a polyline. */
export function cubicBezier(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, steps: number): Vec2[] {
  const out: Vec2[] = [];
  for (let i = 0; i <= steps; i++) {
    const t = i / steps;
    const u = 1 - t;
    const a = u * u * u;
    const b = 3 * u * u * t;
    const c = 3 * u * t * t;
    const d = t * t * t;
    out.push({
      x: a * p0.x + b * p1.x + c * p2.x + d * p3.x,
      y: a * p0.y + b * p1.y + c * p2.y + d * p3.y,
    });
  }
  return out;
}

/**
 * Intersection of segments p1-p2 and p3-p4. Returns parameters (t on first,
 * u on second) or undefined when they don't cross.
 */
export function segmentIntersect(
  x1: number, y1: number, x2: number, y2: number,
  x3: number, y3: number, x4: number, y4: number,
): { t: number; u: number } | undefined {
  const d = (x2 - x1) * (y4 - y3) - (y2 - y1) * (x4 - x3);
  if (Math.abs(d) < 1e-12) return undefined;
  const t = ((x3 - x1) * (y4 - y3) - (y3 - y1) * (x4 - x3)) / d;
  const u = ((x3 - x1) * (y2 - y1) - (y3 - y1) * (x2 - x1)) / d;
  if (t < 0 || t > 1 || u < 0 || u > 1) return undefined;
  return { t, u };
}

/**
 * First crossing between two paths, as arc lengths on each, or undefined.
 * `skipStart`/`skipEnd` ignore crossings near the shared start/end points of
 * paths that merge or diverge (those are handled as merges, not crossings).
 */
export function pathCrossing(a: Path, b: Path): { sa: number; sb: number } | undefined {
  let best: { sa: number; sb: number } | undefined = undefined;
  for (let i = 0; i < a.xs.length - 1; i++) {
    for (let j = 0; j < b.xs.length - 1; j++) {
      const hit = segmentIntersect(
        a.xs[i], a.ys[i], a.xs[i + 1], a.ys[i + 1],
        b.xs[j], b.ys[j], b.xs[j + 1], b.ys[j + 1],
      );
      if (hit !== undefined) {
        const sa = a.cum[i] + hit.t * (a.cum[i + 1] - a.cum[i]);
        const sb = b.cum[j] + hit.u * (b.cum[j + 1] - b.cum[j]);
        if (best === undefined || sa < best.sa) best = { sa, sb };
      }
    }
  }
  return best;
}

/** Minimum distance between two paths with the arc lengths where it occurs (sampled). */
export function pathClosest(a: Path, b: Path, step: number): { sa: number; sb: number; d: number } {
  const tmp = { s: 0, lateral: 0, d2: 0 };
  const ps: PathSample = { x: 0, y: 0, heading: 0 };
  let best = { sa: 0, sb: 0, d: Infinity };
  const n = Math.max(2, Math.ceil(a.length / step));
  for (let i = 0; i <= n; i++) {
    const s = (a.length * i) / n;
    a.sample(s, ps);
    b.project(ps.x, ps.y, tmp);
    const d = Math.sqrt(tmp.d2);
    if (d < best.d) best = { sa: s, sb: tmp.s, d };
  }
  return best;
}
