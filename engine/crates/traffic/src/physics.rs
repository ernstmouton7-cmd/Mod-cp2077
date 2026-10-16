//! 2D oriented-box contacts and impulse response for the internal physics.

use crate::math::scalar::clamp;
use crate::vehicle::VehicleDynamics;

/// Oriented box (centre, heading, half extents).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BoxShape {
    pub x: f64,
    pub y: f64,
    pub heading: f64,
    pub hl: f64,
    pub hw: f64,
}

/// Contact result: normal points from A to B.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Contact {
    pub nx: f64,
    pub ny: f64,
    pub depth: f64,
    pub px: f64,
    pub py: f64,
}

fn support(b: &BoxShape, dx: f64, dy: f64) -> (f64, f64) {
    let c = b.heading.cos();
    let s = b.heading.sin();
    let fx = dx * c + dy * s;
    let fy = -dx * s + dy * c;
    let lx = if fx >= 0.0 { b.hl } else { -b.hl };
    let ly = if fy >= 0.0 { b.hw } else { -b.hw };
    (b.x + lx * c - ly * s, b.y + lx * s + ly * c)
}

/// Separating-axis test for two oriented boxes.
pub fn box_contact(a: &BoxShape, b: &BoxShape) -> Option<Contact> {
    let ca = a.heading.cos();
    let sa = a.heading.sin();
    let cb = b.heading.cos();
    let sb = b.heading.sin();
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let mut best = f64::INFINITY;
    let mut bnx = 0.0;
    let mut bny = 0.0;
    for (nx, ny) in [(ca, sa), (-sa, ca), (cb, sb), (-sb, cb)] {
        let ra = a.hl * (ca * nx + sa * ny).abs() + a.hw * (-sa * nx + ca * ny).abs();
        let rb = b.hl * (cb * nx + sb * ny).abs() + b.hw * (-sb * nx + cb * ny).abs();
        let d = dx * nx + dy * ny;
        let overlap = ra + rb - d.abs();
        if overlap <= 0.0 {
            return None;
        }
        if overlap < best {
            best = overlap;
            if d < 0.0 {
                bnx = -nx;
                bny = -ny;
            } else {
                bnx = nx;
                bny = ny;
            }
        }
    }
    let pa = support(a, bnx, bny);
    let pb = support(b, -bnx, -bny);
    Some(Contact {
        nx: bnx,
        ny: bny,
        depth: best,
        px: (pa.0 + pb.0) / 2.0,
        py: (pa.1 + pb.1) / 2.0,
    })
}

/// Impulse response between two rigid bodies; separates them along the
/// normal and returns the normal impulse (N·s).
pub fn resolve_bodies(
    a: &mut VehicleDynamics,
    b: &mut VehicleDynamics,
    c: &Contact,
    restitution: f64,
    friction: f64,
) -> f64 {
    let rax = c.px - a.x;
    let ray = c.py - a.y;
    let rbx = c.px - b.x;
    let rby = c.py - b.y;
    let vax = a.vx() - a.r * ray;
    let vay = a.vy() + a.r * rax;
    let vbx = b.vx() - b.r * rby;
    let vby = b.vy() + b.r * rbx;
    let rvx = vbx - vax;
    let rvy = vby - vay;
    let vn = rvx * c.nx + rvy * c.ny;
    let inv_a = 1.0 / a.mass;
    let inv_b = 1.0 / b.mass;
    let corr = ((c.depth - 0.01).max(0.0) * 0.8) / (inv_a + inv_b);
    a.x -= c.nx * corr * inv_a;
    a.y -= c.ny * corr * inv_a;
    b.x += c.nx * corr * inv_b;
    b.y += c.ny * corr * inv_b;
    if vn >= 0.0 {
        return 0.0;
    }
    let ra_n = rax * c.ny - ray * c.nx;
    let rb_n = rbx * c.ny - rby * c.nx;
    let denom = inv_a + inv_b + (ra_n * ra_n) / a.iz + (rb_n * rb_n) / b.iz;
    let j = (-(1.0 + restitution) * vn) / denom;
    a.apply_impulse(-c.nx * j, -c.ny * j, c.px, c.py);
    b.apply_impulse(c.nx * j, c.ny * j, c.px, c.py);
    let tx = -c.ny;
    let ty = c.nx;
    let vt = rvx * tx + rvy * ty;
    let ra_t = rax * ty - ray * tx;
    let rb_t = rbx * ty - rby * tx;
    let denom_t = inv_a + inv_b + (ra_t * ra_t) / a.iz + (rb_t * rb_t) / b.iz;
    let max_f = friction * j;
    let jt = clamp(-vt / denom_t, -max_f, max_f);
    a.apply_impulse(-tx * jt, -ty * jt, c.px, c.py);
    b.apply_impulse(tx * jt, ty * jt, c.px, c.py);
    j
}

/// Impulse response against an immovable obstacle (normal points from the
/// body into the obstacle). Returns the normal impulse.
pub fn resolve_static(
    a: &mut VehicleDynamics,
    c: &Contact,
    restitution: f64,
    friction: f64,
) -> f64 {
    let rax = c.px - a.x;
    let ray = c.py - a.y;
    a.x -= c.nx * (c.depth - 0.005).max(0.0);
    a.y -= c.ny * (c.depth - 0.005).max(0.0);
    let vax = a.vx() - a.r * ray;
    let vay = a.vy() + a.r * rax;
    let vn = -(vax * c.nx + vay * c.ny);
    if vn >= 0.0 {
        return 0.0;
    }
    let inv_a = 1.0 / a.mass;
    let ra_n = rax * c.ny - ray * c.nx;
    let j = (-(1.0 + restitution) * vn) / (inv_a + (ra_n * ra_n) / a.iz);
    a.apply_impulse(-c.nx * j, -c.ny * j, c.px, c.py);
    let tx = -c.ny;
    let ty = c.nx;
    let vt = vax * tx + vay * ty;
    let ra_t = rax * ty - ray * tx;
    let max_f = friction * j;
    let jt = clamp(-vt / (inv_a + (ra_t * ra_t) / a.iz), -max_f, max_f);
    a.apply_impulse(tx * jt, ty * jt, c.px, c.py);
    j
}
