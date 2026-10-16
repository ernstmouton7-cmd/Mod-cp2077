/** Parameters of the Intelligent Driver Model. */
export interface IdmParams {
  /** Desired speed (m/s). */
  v0: number;
  /** Time headway (s). */
  T: number;
  /** Max acceleration (m/s^2). */
  a: number;
  /** Comfortable deceleration (m/s^2). */
  b: number;
  /** Jam distance (m). */
  s0: number;
}

/** IDM free-road term. */
export function idmFree(v: number, p: IdmParams): number {
  if (p.v0 <= 0.1) return -p.b * 2;
  const r = v / p.v0;
  if (r <= 1) return p.a * (1 - r * r * r * r);
  // Above desired speed: brake smoothly (Treiber's improved variant).
  return -p.b * (1 - 1 / (r * r));
}

/**
 * Full IDM acceleration toward a leader.
 * @param gap  bumper-to-bumper distance (m)
 * @param dv   approach rate v - vLeader (m/s)
 */
export function idmAccel(v: number, gap: number, dv: number, p: IdmParams): number {
  const free = idmFree(v, p);
  const g = Math.max(gap, 0.05);
  const sStar = p.s0 + Math.max(0, v * p.T + (v * dv) / (2 * Math.sqrt(p.a * p.b)));
  const inter = p.a * (sStar / g) * (sStar / g);
  return free - inter;
}

/** Deceleration needed to stop within `dist` from speed v. */
export function decelToStop(v: number, dist: number): number {
  if (dist <= 0.05) return v > 0.1 ? 99 : 0;
  return (v * v) / (2 * dist);
}
