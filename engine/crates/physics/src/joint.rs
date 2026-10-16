//! Joints: ball, weld (fixed, optionally springy), hinge (limits, motor, spring, friction) and distance.
//! Every joint can break when its constraint force or torque exceeds a threshold.

use crate::body::BodyHandle;
use crate::math::{inverse_or_zero, skew, Pose};
use crate::solver::{body_pair, Softness, SolverBody};
use glam::{Mat2, Mat3, Quat, Vec2, Vec3};

#[derive(Clone, Copy, Debug)]
pub struct HingeLimits {
    pub lower: f32,
    pub upper: f32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct HingeMotor {
    /// Target angular speed (rad/s).
    pub speed: f32,
    /// Max torque (N·m). Zero speed with a small torque acts as hinge friction.
    pub max_torque: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct HingeSpring {
    pub target: f32,
    pub hertz: f32,
    pub damping_ratio: f32,
}

#[derive(Clone, Debug)]
pub enum JointKind {
    /// Shared point; free rotation.
    Ball,
    /// Locks all relative motion. Non-zero hertz makes it a spring (bendable poles, flexible mounts).
    Weld {
        linear_hertz: f32,
        angular_hertz: f32,
        damping_ratio: f32,
    },
    /// Rotation about the frame X axis.
    Hinge {
        limits: Option<HingeLimits>,
        motor: Option<HingeMotor>,
        spring: Option<HingeSpring>,
    },
    /// Keeps anchor distance within [min, max]; with hertz > 0 it is a spring toward `rest`.
    Distance {
        rest: f32,
        min: f32,
        max: f32,
        hertz: f32,
        damping_ratio: f32,
    },
}

#[derive(Clone, Debug)]
pub struct JointDesc {
    pub body_a: BodyHandle,
    pub body_b: BodyHandle,
    /// Joint frame in body A's origin frame.
    pub frame_a: Pose,
    /// Joint frame in body B's origin frame.
    pub frame_b: Pose,
    pub kind: JointKind,
    pub collide_connected: bool,
    /// Break when the constraint force exceeds this (N). Infinite by default.
    pub break_force: f32,
    /// Break when the constraint torque exceeds this (N·m).
    pub break_torque: f32,
    pub user_data: u64,
}

impl JointDesc {
    pub fn new(
        body_a: BodyHandle,
        body_b: BodyHandle,
        frame_a: Pose,
        frame_b: Pose,
        kind: JointKind,
    ) -> Self {
        Self {
            body_a,
            body_b,
            frame_a,
            frame_b,
            kind,
            collide_connected: false,
            break_force: f32::INFINITY,
            break_torque: f32::INFINITY,
            user_data: 0,
        }
    }
    pub fn breakable(mut self, force: f32, torque: f32) -> Self {
        self.break_force = force;
        self.break_torque = torque;
        self
    }
}

#[derive(Clone, Debug)]
pub struct Joint {
    pub desc: JointDesc,
    pub broken: bool,
    /// Last step's constraint force and torque magnitudes (N, N·m), for stress readouts.
    pub force: f32,
    pub torque: f32,
    pub(crate) state: JointState,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct JointState {
    pub a: u32,
    pub b: u32,
    pub la: Vec3,
    pub lb: Vec3,
    pub lin_impulse: Vec3,
    pub ang_impulse: Vec3,
    pub axial_impulse: f32,
    pub limit_impulse: f32,
    pub upper_impulse: f32,
    pub motor_impulse: f32,
    pub spring_impulse: f32,
    pub hinge_impulse: Vec2,
    pub soft: Softness,
    pub lin_soft: Softness,
    pub ang_soft: Softness,
    pub spring_soft: Softness,
    pub peak_lin: f32,
    pub peak_ang: f32,
    /// Low-passed force/torque used for breaking (failure needs sustained load, not a spike).
    pub stress_lin: f32,
    pub stress_ang: f32,
}

impl Joint {
    pub(crate) fn new(desc: JointDesc) -> Self {
        Joint {
            desc,
            broken: false,
            force: 0.0,
            torque: 0.0,
            state: JointState::default(),
        }
    }

    /// Current hinge angle (rad) given the two body orientations.
    pub fn hinge_angle(&self, qa: Quat, qb: Quat) -> f32 {
        let fa = qa * self.desc.frame_a.q;
        let fb = qb * self.desc.frame_b.q;
        let axis = fa * Vec3::X;
        let ya = fa * Vec3::Y;
        let yb = fb * Vec3::Y;
        ya.cross(yb).dot(axis).atan2(ya.dot(yb))
    }

    pub(crate) fn prepare(
        &mut self,
        a: u32,
        b: u32,
        bodies: &[SolverBody],
        h: f32,
        joint_soft: Softness,
    ) {
        let s = &mut self.state;
        s.a = a;
        s.b = b;
        let ba = &bodies[a as usize];
        let bb = &bodies[b as usize];
        s.la = self.desc.frame_a.p - ba.local_com;
        s.lb = self.desc.frame_b.p - bb.local_com;
        s.soft = joint_soft;
        s.peak_lin = 0.0;
        s.peak_ang = 0.0;
        match &self.desc.kind {
            JointKind::Weld {
                linear_hertz,
                angular_hertz,
                damping_ratio,
            } => {
                s.lin_soft = if *linear_hertz > 0.0 {
                    Softness::new(*linear_hertz, *damping_ratio, h)
                } else {
                    joint_soft
                };
                s.ang_soft = if *angular_hertz > 0.0 {
                    Softness::new(*angular_hertz, *damping_ratio, h)
                } else {
                    joint_soft
                };
            }
            JointKind::Hinge {
                spring: Some(sp), ..
            } => {
                s.spring_soft = Softness::new(sp.hertz, sp.damping_ratio, h);
            }
            JointKind::Distance {
                hertz,
                damping_ratio,
                ..
            } => {
                s.spring_soft = Softness::new(*hertz, *damping_ratio, h);
            }
            _ => {}
        }
    }

    pub(crate) fn warm_start(&self, bodies: &mut [SolverBody]) {
        let s = &self.state;
        let (ba, bb) = body_pair(bodies, s.a, s.b);
        let ra = ba.q * s.la;
        let rb = bb.q * s.lb;
        let mut ang = s.ang_impulse;
        let fa = ba.q * self.desc.frame_a.q;
        if let JointKind::Hinge { .. } = self.desc.kind {
            let axis = fa * Vec3::X;
            let (y, z) = (fa * Vec3::Y, fa * Vec3::Z);
            ang = y * s.hinge_impulse.x
                + z * s.hinge_impulse.y
                + axis * (s.limit_impulse - s.upper_impulse + s.motor_impulse + s.spring_impulse);
        }
        let mut lin = s.lin_impulse;
        if let JointKind::Distance { .. } = self.desc.kind {
            let d = (bb.c + rb) - (ba.c + ra);
            lin = d.normalize_or_zero() * s.axial_impulse;
        }
        ba.apply_impulse(-lin, ra);
        bb.apply_impulse(lin, rb);
        ba.apply_angular_impulse(-ang);
        bb.apply_angular_impulse(ang);
    }

    pub(crate) fn solve(&mut self, bodies: &mut [SolverBody], h: f32, inv_h: f32, use_bias: bool) {
        let kind = self.desc.kind.clone();
        let fa_q = self.desc.frame_a.q;
        let fb_q = self.desc.frame_b.q;
        let s = &mut self.state;
        let (ba, bb) = body_pair(bodies, s.a, s.b);
        let ra = ba.q * s.la;
        let rb = bb.q * s.lb;
        let fa = ba.q * fa_q;
        let fb = bb.q * fb_q;
        let inv_i_sum = ba.inv_i + bb.inv_i;

        // Angular parts first (they are usually the stiffest), then the point constraint.
        match &kind {
            JointKind::Weld { .. } => {
                let mut qe = fb * fa.conjugate();
                if qe.w < 0.0 {
                    qe = -qe;
                }
                let c = Vec3::new(qe.x, qe.y, qe.z) * 2.0;
                let (bias, ms, is) = if use_bias {
                    (
                        c * s.ang_soft.bias_rate,
                        s.ang_soft.mass_scale,
                        s.ang_soft.impulse_scale,
                    )
                } else {
                    (Vec3::ZERO, 1.0, 0.0)
                };
                let cdot = bb.w - ba.w;
                let k = inverse_or_zero(inv_i_sum);
                let impulse = -(k * (cdot + bias)) * ms - s.ang_impulse * is;
                s.ang_impulse += impulse;
                s.peak_ang = s.peak_ang.max(s.ang_impulse.length());
                ba.apply_angular_impulse(-impulse);
                bb.apply_angular_impulse(impulse);
            }
            JointKind::Hinge {
                limits,
                motor,
                spring,
            } => {
                let axis = fa * Vec3::X;
                let axis_b = fb * Vec3::X;
                let k_axial = axis.dot(inv_i_sum * axis);
                let m_axial = if k_axial > 0.0 { 1.0 / k_axial } else { 0.0 };
                let ya = fa * Vec3::Y;
                let yb = fb * Vec3::Y;
                let angle = ya.cross(yb).dot(axis).atan2(ya.dot(yb));
                // Spring toward a target angle.
                if let Some(sp) = spring {
                    if sp.hertz > 0.0 {
                        let c = angle - sp.target;
                        let bias = c * s.spring_soft.bias_rate;
                        let cdot = axis.dot(bb.w - ba.w);
                        let impulse = -m_axial * s.spring_soft.mass_scale * (cdot + bias)
                            - s.spring_soft.impulse_scale * s.spring_impulse;
                        s.spring_impulse += impulse;
                        ba.apply_angular_impulse(-axis * impulse);
                        bb.apply_angular_impulse(axis * impulse);
                    }
                }
                if let Some(m) = motor {
                    let cdot = axis.dot(bb.w - ba.w) - m.speed;
                    let max = m.max_torque * h;
                    let impulse = -m_axial * cdot;
                    let old = s.motor_impulse;
                    s.motor_impulse = (old + impulse).clamp(-max, max);
                    let applied = s.motor_impulse - old;
                    ba.apply_angular_impulse(-axis * applied);
                    bb.apply_angular_impulse(axis * applied);
                }
                if let Some(l) = limits {
                    // Lower: angle - lower >= 0 (impulse along +axis); upper: upper - angle >= 0 (along -axis).
                    for (c, sign) in [(angle - l.lower, 1.0f32), (l.upper - angle, -1.0f32)] {
                        let (bias, ms, is) = if c > 0.0 {
                            (c * inv_h, 1.0, 0.0)
                        } else if use_bias {
                            (
                                c * s.soft.bias_rate,
                                s.soft.mass_scale,
                                s.soft.impulse_scale,
                            )
                        } else {
                            (0.0, 1.0, 0.0)
                        };
                        let acc = if sign > 0.0 {
                            &mut s.limit_impulse
                        } else {
                            &mut s.upper_impulse
                        };
                        let cdot = sign * axis.dot(bb.w - ba.w);
                        let impulse = -m_axial * ms * (cdot + bias) - is * *acc;
                        let new = (*acc + impulse).max(0.0);
                        let applied = new - *acc;
                        *acc = new;
                        let l = axis * (applied * sign);
                        ba.apply_angular_impulse(-l);
                        bb.apply_angular_impulse(l);
                    }
                }
                // Keep the hinge axes aligned (2 rows).
                let y = fa * Vec3::Y;
                let z = fa * Vec3::Z;
                let err = axis.cross(axis_b);
                let c = Vec2::new(y.dot(err), z.dot(err));
                let (bias, ms, is) = if use_bias {
                    (
                        c * s.soft.bias_rate,
                        s.soft.mass_scale,
                        s.soft.impulse_scale,
                    )
                } else {
                    (Vec2::ZERO, 1.0, 0.0)
                };
                let dw = bb.w - ba.w;
                let cdot = Vec2::new(y.dot(dw), z.dot(dw));
                let k = Mat2::from_cols(
                    Vec2::new(y.dot(inv_i_sum * y), z.dot(inv_i_sum * y)),
                    Vec2::new(y.dot(inv_i_sum * z), z.dot(inv_i_sum * z)),
                );
                let det = k.determinant();
                if det.abs() > 1e-12 {
                    let impulse = -(k.inverse() * (cdot + bias)) * ms - s.hinge_impulse * is;
                    s.hinge_impulse += impulse;
                    let l = y * impulse.x + z * impulse.y;
                    ba.apply_angular_impulse(-l);
                    bb.apply_angular_impulse(l);
                }
                s.peak_ang = s.peak_ang.max(s.hinge_impulse.length());
            }
            _ => {}
        }

        match &kind {
            JointKind::Distance {
                rest,
                min,
                max,
                hertz,
                ..
            } => {
                let pa = ba.c + ra;
                let pb = bb.c + rb;
                let d = pb - pa;
                let len = d.length();
                if len < 1e-6 {
                    return;
                }
                let u = d / len;
                let rna = ra.cross(u);
                let rnb = rb.cross(u);
                let k =
                    ba.inv_mass + bb.inv_mass + rna.dot(ba.inv_i * rna) + rnb.dot(bb.inv_i * rnb);
                let m = if k > 0.0 { 1.0 / k } else { 0.0 };
                let vr = u.dot(bb.velocity_at(rb) - ba.velocity_at(ra));
                let mut impulse = 0.0;
                if *hertz > 0.0 {
                    let c = len - rest;
                    impulse += -m * s.spring_soft.mass_scale * (vr + c * s.spring_soft.bias_rate)
                        - s.spring_soft.impulse_scale * s.axial_impulse;
                }
                // Hard limits.
                if len < *min || len > *max {
                    let c = if len < *min { len - min } else { len - max };
                    let bias = if use_bias { c * s.soft.bias_rate } else { 0.0 };
                    let lim = -m * (vr + bias);
                    impulse += if len < *min {
                        lim.max(0.0)
                    } else {
                        lim.min(0.0)
                    };
                } else if *hertz <= 0.0 {
                    return;
                }
                s.axial_impulse += impulse;
                s.peak_lin = s.peak_lin.max(s.axial_impulse.abs());
                ba.apply_impulse(-u * impulse, ra);
                bb.apply_impulse(u * impulse, rb);
            }
            _ => {
                let soft = if let JointKind::Weld { .. } = kind {
                    s.lin_soft
                } else {
                    s.soft
                };
                let c = (bb.c + rb) - (ba.c + ra);
                let (bias, ms, is) = if use_bias {
                    (c * soft.bias_rate, soft.mass_scale, soft.impulse_scale)
                } else {
                    (Vec3::ZERO, 1.0, 0.0)
                };
                let cdot = bb.velocity_at(rb) - ba.velocity_at(ra);
                let sa = skew(ra);
                let sb = skew(rb);
                let k = Mat3::from_diagonal(Vec3::splat(ba.inv_mass + bb.inv_mass))
                    - sa * ba.inv_i * sa
                    - sb * bb.inv_i * sb;
                let impulse = -(inverse_or_zero(k) * (cdot + bias)) * ms - s.lin_impulse * is;
                s.lin_impulse += impulse;
                s.peak_lin = s.peak_lin.max(s.lin_impulse.length());
                ba.apply_impulse(-impulse, ra);
                bb.apply_impulse(impulse, rb);
            }
        }
    }

    /// Check break thresholds after a step (h = substep length).
    pub(crate) fn finish(&mut self, h: f32, dt: f32) -> bool {
        let f = self.state.peak_lin / h;
        let t = self.state.peak_ang / h;
        self.force = f;
        self.torque = t;
        // ~50 ms time constant.
        let k = 1.0 - (-dt / 0.05).exp();
        let s = &mut self.state;
        s.stress_lin += (f - s.stress_lin) * k;
        s.stress_ang += (t - s.stress_ang) * k;
        s.stress_lin > self.desc.break_force
            || s.stress_ang > self.desc.break_torque
            || f > self.desc.break_force * 4.0
            || t > self.desc.break_torque * 4.0
    }
}
