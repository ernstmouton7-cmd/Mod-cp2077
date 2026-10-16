/**
 * Scalar helpers. Everything in src/core must behave identically when compiled
 * to JavaScript and to Lua (via TypeScriptToLua), so modulo and rounding go
 * through these helpers instead of operators whose semantics differ.
 */

export const PI = Math.PI;
export const TAU = Math.PI * 2;
export const DEG = Math.PI / 180;
export const G = 9.81;
export const EPS = 1e-9;

/** Floored modulo: result always has the sign of `b` (Lua semantics, also what we want for angles). */
export function mod(a: number, b: number): number {
  return a - Math.floor(a / b) * b;
}

export function clamp(v: number, lo: number, hi: number): number {
  return v < lo ? lo : v > hi ? hi : v;
}

export function clamp01(v: number): number {
  return v < 0 ? 0 : v > 1 ? 1 : v;
}

export function lerp(a: number, b: number, t: number): number {
  return a + (b - a) * t;
}

export function invLerp(a: number, b: number, v: number): number {
  return b === a ? 0 : (v - a) / (b - a);
}

export function sign(v: number): number {
  return v > 0 ? 1 : v < 0 ? -1 : 0;
}

/** Wrap angle to (-PI, PI]. */
export function wrapAngle(a: number): number {
  const w = mod(a + PI, TAU) - PI;
  return w === -PI ? PI : w;
}

/** Signed smallest difference b - a in (-PI, PI]. */
export function angleDiff(a: number, b: number): number {
  return wrapAngle(b - a);
}

/** Move `current` toward `target` by at most `maxDelta`. */
export function approach(current: number, target: number, maxDelta: number): number {
  if (current < target) return Math.min(current + maxDelta, target);
  return Math.max(current - maxDelta, target);
}

/** Frame-rate independent exponential smoothing factor. */
export function smoothFactor(rate: number, dt: number): number {
  return 1 - Math.exp(-rate * dt);
}

export function smoothstep(e0: number, e1: number, x: number): number {
  const t = clamp01((x - e0) / (e1 - e0));
  return t * t * (3 - 2 * t);
}

export function round(v: number, decimals: number): number {
  const f = Math.pow(10, decimals);
  return Math.floor(v * f + 0.5) / f;
}

export function hypot(x: number, y: number): number {
  return Math.sqrt(x * x + y * y);
}

/** Piecewise-linear lookup in a table of [x, y] pairs sorted by x. */
export function curve(points: ReadonlyArray<readonly [number, number]>, x: number): number {
  const n = points.length;
  if (x <= points[0][0]) return points[0][1];
  if (x >= points[n - 1][0]) return points[n - 1][1];
  for (let i = 1; i < n; i++) {
    const p1 = points[i];
    if (x <= p1[0]) {
      const p0 = points[i - 1];
      return lerp(p0[1], p1[1], (x - p0[0]) / (p1[0] - p0[0]));
    }
  }
  return points[n - 1][1];
}

/** km/h <-> m/s */
export const KMH = 1 / 3.6;
export function toKmh(ms: number): number {
  return ms * 3.6;
}
