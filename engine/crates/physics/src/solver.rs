//! Soft-step contact solver (sub-stepped, soft constraints with relaxation), after Box2D v3.
//!
//! Each step is split into `substeps`. Every substep integrates velocities, warm starts,
//! solves with a soft position bias, integrates positions and then relaxes without bias.
//! Contact anchors and normals are fixed for the step; separation is re-evaluated from the
//! current poses every substep so deep penetration resolves smoothly and stacks stay calm.

use crate::math::{integrate_rotation, orthonormal_basis};
use glam::{Mat3, Quat, Vec3};
use smallvec::SmallVec;

/// Spring-damper softness for a constraint at a given substep length (Box2D v3 `b2Softness`).
#[derive(Clone, Copy, Debug, Default)]
pub struct Softness {
    pub bias_rate: f32,
    pub mass_scale: f32,
    pub impulse_scale: f32,
}

impl Softness {
    pub fn new(hertz: f32, zeta: f32, h: f32) -> Self {
        if hertz <= 0.0 {
            return Softness {
                bias_rate: 0.0,
                mass_scale: 1.0,
                impulse_scale: 0.0,
            };
        }
        let omega = 2.0 * std::f32::consts::PI * hertz;
        let a1 = 2.0 * zeta + h * omega;
        let a2 = h * omega * a1;
        let a3 = 1.0 / (1.0 + a2);
        Softness {
            bias_rate: omega / a1,
            mass_scale: a2 * a3,
            impulse_scale: a3,
        }
    }
    pub const RIGID: Softness = Softness {
        bias_rate: 0.0,
        mass_scale: 1.0,
        impulse_scale: 0.0,
    };
}

/// Body state used during the solve.
#[derive(Clone, Copy, Debug)]
pub struct SolverBody {
    pub v: Vec3,
    pub w: Vec3,
    /// World centre of mass.
    pub c: Vec3,
    pub q: Quat,
    pub inv_mass: f32,
    pub inv_i: Mat3,
    pub inv_i_local: Mat3,
    pub local_com: Vec3,
    pub force: Vec3,
    pub torque: Vec3,
    pub gravity_scale: f32,
    pub linear_damping: f32,
    pub angular_damping: f32,
    pub dynamic: bool,
    pub kinematic: bool,
    /// Index of the owning body slot.
    pub body_index: u32,
}

impl SolverBody {
    #[inline]
    pub fn apply_impulse(&mut self, p: Vec3, r: Vec3) {
        self.v += p * self.inv_mass;
        self.w += self.inv_i * r.cross(p);
    }
    #[inline]
    pub fn apply_angular_impulse(&mut self, l: Vec3) {
        self.w += self.inv_i * l;
    }
    #[inline]
    pub fn velocity_at(&self, r: Vec3) -> Vec3 {
        self.v + self.w.cross(r)
    }
    /// World position of the body origin.
    #[inline]
    pub fn origin(&self) -> Vec3 {
        self.c - self.q * self.local_com
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ConstraintPoint {
    pub ra: Vec3,
    pub rb: Vec3,
    /// Anchors relative to the centres of mass, in body frames (for separation updates).
    pub la: Vec3,
    pub lb: Vec3,
    pub base_separation: f32,
    pub normal_mass: f32,
    pub tangent_mass: [f32; 2],
    pub normal_impulse: f32,
    pub tangent_impulse: [f32; 2],
    pub total_normal_impulse: f32,
    pub max_normal_impulse: f32,
    pub relative_velocity: f32,
}

#[derive(Clone, Debug)]
pub struct ContactConstraint {
    pub a: u32,
    pub b: u32,
    pub normal: Vec3,
    pub tangents: [Vec3; 2],
    pub friction: f32,
    pub restitution: f32,
    pub softness: Softness,
    pub points: SmallVec<[ConstraintPoint; 4]>,
    /// Rolling resistance lever arm (m); 0 disables.
    pub rolling: f32,
    pub rolling_mass: Mat3,
    pub rolling_impulse: Vec3,
    /// (contact index, manifold index) for impulse write-back.
    pub source: (u32, u32),
}

/// Shared solver tuning.
#[derive(Clone, Copy, Debug)]
pub struct SolverConfig {
    pub substeps: u32,
    pub contact_hertz: f32,
    pub contact_damping_ratio: f32,
    pub joint_hertz: f32,
    pub joint_damping_ratio: f32,
    /// Max velocity used to push bodies apart when penetrating (m/s).
    pub max_push_velocity: f32,
    pub linear_slop: f32,
    pub restitution_threshold: f32,
    pub max_linear_speed: f32,
    pub max_angular_speed: f32,
}

impl Default for SolverConfig {
    fn default() -> Self {
        Self {
            substeps: 4,
            contact_hertz: 30.0,
            contact_damping_ratio: 10.0,
            joint_hertz: 60.0,
            joint_damping_ratio: 2.0,
            max_push_velocity: 3.0,
            linear_slop: 0.005,
            restitution_threshold: 1.0,
            max_linear_speed: 400.0,
            max_angular_speed: 120.0,
        }
    }
}

pub fn integrate_velocities(bodies: &mut [SolverBody], gravity: Vec3, h: f32, cfg: &SolverConfig) {
    for b in bodies.iter_mut() {
        if !b.dynamic {
            continue;
        }
        let v = b.v + (gravity * b.gravity_scale + b.force * b.inv_mass) * h;
        let w = b.w + b.inv_i * b.torque * h;
        let mut v = v * (1.0 / (1.0 + h * b.linear_damping));
        let mut w = w * (1.0 / (1.0 + h * b.angular_damping));
        let vl = v.length_squared();
        if vl > cfg.max_linear_speed * cfg.max_linear_speed {
            v *= cfg.max_linear_speed / vl.sqrt();
        }
        let wl = w.length_squared();
        if wl > cfg.max_angular_speed * cfg.max_angular_speed {
            w *= cfg.max_angular_speed / wl.sqrt();
        }
        b.v = v;
        b.w = w;
    }
}

pub fn integrate_positions(bodies: &mut [SolverBody], h: f32) {
    for b in bodies.iter_mut() {
        if !b.dynamic && !b.kinematic {
            continue;
        }
        b.c += b.v * h;
        if b.w != Vec3::ZERO {
            b.q = integrate_rotation(b.q, b.w, h);
            let r = Mat3::from_quat(b.q);
            b.inv_i = r * b.inv_i_local * r.transpose();
        }
    }
}

/// Fill effective masses and anchors. `normal_impulse`/`tangent_impulse` must already hold warm-start values.
pub fn prepare_contact(
    c: &mut ContactConstraint,
    bodies: &[SolverBody],
    world_points: &[(Vec3, f32)],
    h: f32,
    cfg: &SolverConfig,
    static_soft: Softness,
    soft: Softness,
) {
    let ba = &bodies[c.a as usize];
    let bb = &bodies[c.b as usize];
    let n = c.normal;
    let (t1, t2) = orthonormal_basis(n);
    c.tangents = [t1, t2];
    c.softness = if ba.inv_mass == 0.0 || bb.inv_mass == 0.0 {
        static_soft
    } else {
        soft
    };
    if c.rolling > 0.0 {
        c.rolling_mass = crate::math::inverse_or_zero(ba.inv_i + bb.inv_i);
    }
    c.rolling_impulse = Vec3::ZERO;
    let _ = (h, cfg);
    for (cp, (p, sep)) in c.points.iter_mut().zip(world_points) {
        let ra = *p - ba.c;
        let rb = *p - bb.c;
        cp.ra = ra;
        cp.rb = rb;
        cp.la = ba.q.conjugate() * ra;
        cp.lb = bb.q.conjugate() * rb;
        cp.base_separation = *sep;
        let k = |dir: Vec3| {
            let rna = ra.cross(dir);
            let rnb = rb.cross(dir);
            ba.inv_mass + bb.inv_mass + rna.dot(ba.inv_i * rna) + rnb.dot(bb.inv_i * rnb)
        };
        let kn = k(n);
        cp.normal_mass = if kn > 0.0 { 1.0 / kn } else { 0.0 };
        let k1 = k(t1);
        let k2 = k(t2);
        cp.tangent_mass = [
            if k1 > 0.0 { 1.0 / k1 } else { 0.0 },
            if k2 > 0.0 { 1.0 / k2 } else { 0.0 },
        ];
        let vrel = bb.velocity_at(rb) - ba.velocity_at(ra);
        cp.relative_velocity = n.dot(vrel);
        cp.total_normal_impulse = 0.0;
        cp.max_normal_impulse = 0.0;
    }
}

#[inline]
fn pair(bodies: &mut [SolverBody], a: u32, b: u32) -> (&mut SolverBody, &mut SolverBody) {
    let (a, b) = (a as usize, b as usize);
    debug_assert_ne!(a, b);
    if a < b {
        let (l, r) = bodies.split_at_mut(b);
        (&mut l[a], &mut r[0])
    } else {
        let (l, r) = bodies.split_at_mut(a);
        (&mut r[0], &mut l[b])
    }
}

pub(crate) fn body_pair(
    bodies: &mut [SolverBody],
    a: u32,
    b: u32,
) -> (&mut SolverBody, &mut SolverBody) {
    pair(bodies, a, b)
}

pub fn warm_start_contact(c: &ContactConstraint, bodies: &mut [SolverBody]) {
    let (ba, bb) = pair(bodies, c.a, c.b);
    for cp in &c.points {
        let p = c.normal * cp.normal_impulse
            + c.tangents[0] * cp.tangent_impulse[0]
            + c.tangents[1] * cp.tangent_impulse[1];
        ba.apply_impulse(-p, cp.ra);
        bb.apply_impulse(p, cp.rb);
    }
    ba.apply_angular_impulse(-c.rolling_impulse);
    bb.apply_angular_impulse(c.rolling_impulse);
}

pub fn solve_contact(
    c: &mut ContactConstraint,
    bodies: &mut [SolverBody],
    inv_h: f32,
    use_bias: bool,
    cfg: &SolverConfig,
) {
    let (ba, bb) = pair(bodies, c.a, c.b);
    let n = c.normal;
    let mut total_normal = 0.0;
    for cp in c.points.iter_mut() {
        // Current separation from the moved anchors.
        let pa = ba.c + ba.q * cp.la;
        let pb = bb.c + bb.q * cp.lb;
        let s = n.dot(pb - pa) + cp.base_separation;
        let (bias, mass_scale, impulse_scale) = if s > 0.0 {
            // Speculative: allow closing the gap this substep, no more.
            (s * inv_h, 1.0, 0.0)
        } else if use_bias {
            (
                (c.softness.bias_rate * (s + cfg.linear_slop).min(0.0)).max(-cfg.max_push_velocity),
                c.softness.mass_scale,
                c.softness.impulse_scale,
            )
        } else {
            (0.0, 1.0, 0.0)
        };
        let vrel = bb.velocity_at(cp.rb) - ba.velocity_at(cp.ra);
        let vn = n.dot(vrel);
        let impulse =
            -cp.normal_mass * mass_scale * (vn + bias) - impulse_scale * cp.normal_impulse;
        let new_impulse = (cp.normal_impulse + impulse).max(0.0);
        let applied = new_impulse - cp.normal_impulse;
        cp.normal_impulse = new_impulse;
        cp.max_normal_impulse = cp.max_normal_impulse.max(new_impulse);
        total_normal += new_impulse;
        let p = n * applied;
        ba.apply_impulse(-p, cp.ra);
        bb.apply_impulse(p, cp.rb);
    }
    if c.rolling > 0.0 {
        let max = c.rolling * total_normal;
        let dw = bb.w - ba.w;
        let impulse = -(c.rolling_mass * dw);
        let mut new = c.rolling_impulse + impulse;
        let l = new.length();
        if l > max {
            new *= max / l;
        }
        let applied = new - c.rolling_impulse;
        c.rolling_impulse = new;
        ba.apply_angular_impulse(-applied);
        bb.apply_angular_impulse(applied);
    }
    for cp in c.points.iter_mut() {
        let limit = c.friction * cp.normal_impulse;
        for k in 0..2 {
            let t = c.tangents[k];
            let vrel = bb.velocity_at(cp.rb) - ba.velocity_at(cp.ra);
            let vt = t.dot(vrel);
            let impulse = -cp.tangent_mass[k] * vt;
            let new_impulse = (cp.tangent_impulse[k] + impulse).clamp(-limit, limit);
            let applied = new_impulse - cp.tangent_impulse[k];
            cp.tangent_impulse[k] = new_impulse;
            let p = t * applied;
            ba.apply_impulse(-p, cp.ra);
            bb.apply_impulse(p, cp.rb);
        }
    }
}

pub fn apply_restitution(c: &mut ContactConstraint, bodies: &mut [SolverBody], cfg: &SolverConfig) {
    if c.restitution == 0.0 {
        return;
    }
    let (ba, bb) = pair(bodies, c.a, c.b);
    let n = c.normal;
    for cp in c.points.iter_mut() {
        if cp.relative_velocity > -cfg.restitution_threshold || cp.max_normal_impulse == 0.0 {
            continue;
        }
        let vrel = bb.velocity_at(cp.rb) - ba.velocity_at(cp.ra);
        let vn = n.dot(vrel);
        let impulse = -cp.normal_mass * (vn + c.restitution * cp.relative_velocity);
        let new_impulse = (cp.normal_impulse + impulse).max(0.0);
        let applied = new_impulse - cp.normal_impulse;
        cp.normal_impulse = new_impulse;
        cp.max_normal_impulse = cp.max_normal_impulse.max(new_impulse);
        cp.total_normal_impulse += applied;
        let p = n * applied;
        ba.apply_impulse(-p, cp.ra);
        bb.apply_impulse(p, cp.rb);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn softness_limits() {
        let s = Softness::new(30.0, 10.0, 1.0 / 240.0);
        assert!(s.mass_scale > 0.0 && s.mass_scale < 1.0);
        assert!((s.mass_scale + s.impulse_scale - 1.0).abs() < 1e-5);
        let r = Softness::new(0.0, 1.0, 0.01);
        assert_eq!(r.mass_scale, 1.0);
    }
}
