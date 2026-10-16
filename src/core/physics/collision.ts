import { VehicleDynamics } from "../vehicle/dynamics";

/** Oriented box description. */
export interface Box {
  x: number;
  y: number;
  heading: number;
  hl: number;
  hw: number;
}

/** Contact result: normal points from A to B. */
export interface Contact {
  nx: number;
  ny: number;
  depth: number;
  px: number;
  py: number;
}

function support(b: Box, dx: number, dy: number, out: { x: number; y: number }): void {
  const c = Math.cos(b.heading);
  const s = Math.sin(b.heading);
  const fx = dx * c + dy * s;
  const fy = -dx * s + dy * c;
  const lx = fx >= 0 ? b.hl : -b.hl;
  const ly = fy >= 0 ? b.hw : -b.hw;
  out.x = b.x + lx * c - ly * s;
  out.y = b.y + lx * s + ly * c;
}

const sa = { x: 0, y: 0 };
const sb = { x: 0, y: 0 };

/** Separating-axis test for two oriented boxes. Returns true and fills `out` on overlap. */
export function boxContact(a: Box, b: Box, out: Contact): boolean {
  const ca = Math.cos(a.heading);
  const sA = Math.sin(a.heading);
  const cb = Math.cos(b.heading);
  const sB = Math.sin(b.heading);
  const dx = b.x - a.x;
  const dy = b.y - a.y;
  let best = Infinity;
  let bnx = 0;
  let bny = 0;
  for (let i = 0; i < 4; i++) {
    let nx: number;
    let ny: number;
    if (i === 0) {
      nx = ca;
      ny = sA;
    } else if (i === 1) {
      nx = -sA;
      ny = ca;
    } else if (i === 2) {
      nx = cb;
      ny = sB;
    } else {
      nx = -sB;
      ny = cb;
    }
    const rA = a.hl * Math.abs(ca * nx + sA * ny) + a.hw * Math.abs(-sA * nx + ca * ny);
    const rB = b.hl * Math.abs(cb * nx + sB * ny) + b.hw * Math.abs(-sB * nx + cb * ny);
    const d = dx * nx + dy * ny;
    const overlap = rA + rB - Math.abs(d);
    if (overlap <= 0) return false;
    if (overlap < best) {
      best = overlap;
      if (d < 0) {
        bnx = -nx;
        bny = -ny;
      } else {
        bnx = nx;
        bny = ny;
      }
    }
  }
  support(a, bnx, bny, sa);
  support(b, -bnx, -bny, sb);
  out.nx = bnx;
  out.ny = bny;
  out.depth = best;
  out.px = (sa.x + sb.x) / 2;
  out.py = (sa.y + sb.y) / 2;
  return true;
}

/**
 * Impulse response between two rigid bodies. Returns the normal impulse (N·s).
 * Also separates the bodies along the normal.
 */
export function resolveBodies(a: VehicleDynamics, b: VehicleDynamics, c: Contact, restitution: number, friction: number): number {
  const rax = c.px - a.x;
  const ray = c.py - a.y;
  const rbx = c.px - b.x;
  const rby = c.py - b.y;
  const vax = a.vx() - a.r * ray;
  const vay = a.vy() + a.r * rax;
  const vbx = b.vx() - b.r * rby;
  const vby = b.vy() + b.r * rbx;
  const rvx = vbx - vax;
  const rvy = vby - vay;
  const vn = rvx * c.nx + rvy * c.ny;
  const invA = 1 / a.mass;
  const invB = 1 / b.mass;
  // Positional correction (split by inverse mass).
  const corr = (Math.max(0, c.depth - 0.01) * 0.8) / (invA + invB);
  a.x -= c.nx * corr * invA;
  a.y -= c.ny * corr * invA;
  b.x += c.nx * corr * invB;
  b.y += c.ny * corr * invB;
  if (vn >= 0) return 0;
  const raN = rax * c.ny - ray * c.nx;
  const rbN = rbx * c.ny - rby * c.nx;
  const denom = invA + invB + (raN * raN) / a.iz + (rbN * rbN) / b.iz;
  const j = (-(1 + restitution) * vn) / denom;
  a.applyImpulse(-c.nx * j, -c.ny * j, c.px, c.py);
  b.applyImpulse(c.nx * j, c.ny * j, c.px, c.py);
  // Coulomb friction along the tangent.
  const tx = -c.ny;
  const ty = c.nx;
  const vt = rvx * tx + rvy * ty;
  const raT = rax * ty - ray * tx;
  const rbT = rbx * ty - rby * tx;
  const denomT = invA + invB + (raT * raT) / a.iz + (rbT * rbT) / b.iz;
  let jt = -vt / denomT;
  const maxF = friction * j;
  if (jt > maxF) jt = maxF;
  if (jt < -maxF) jt = -maxF;
  a.applyImpulse(-tx * jt, -ty * jt, c.px, c.py);
  b.applyImpulse(tx * jt, ty * jt, c.px, c.py);
  return j;
}

/** Impulse response against an immovable obstacle (normal points from body into obstacle). */
export function resolveStatic(a: VehicleDynamics, c: Contact, restitution: number, friction: number): number {
  const rax = c.px - a.x;
  const ray = c.py - a.y;
  a.x -= c.nx * Math.max(0, c.depth - 0.005);
  a.y -= c.ny * Math.max(0, c.depth - 0.005);
  const vax = a.vx() - a.r * ray;
  const vay = a.vy() + a.r * rax;
  const vn = -(vax * c.nx + vay * c.ny);
  if (vn >= 0) return 0;
  const invA = 1 / a.mass;
  const raN = rax * c.ny - ray * c.nx;
  const j = (-(1 + restitution) * vn) / (invA + (raN * raN) / a.iz);
  a.applyImpulse(-c.nx * j, -c.ny * j, c.px, c.py);
  const tx = -c.ny;
  const ty = c.nx;
  const vt = vax * tx + vay * ty;
  const raT = rax * ty - ray * tx;
  let jt = -vt / (invA + (raT * raT) / a.iz);
  const maxF = friction * j;
  if (jt > maxF) jt = maxF;
  if (jt < -maxF) jt = -maxF;
  a.applyImpulse(tx * jt, ty * jt, c.px, c.py);
  return j;
}
