//! Soft-body style crash deformation and detachable parts for vehicles.
//!
//! The car's visual mesh is split into the main body and separate part meshes (bumpers, hood,
//! doors, trunk, mirrors). Impacts reported by the solver dent the nearest vertices (plastic
//! deformation with crumple noise and a crush limit) and wear down part health. Damaged doors,
//! hood and trunk unlatch and swing on hinge joints; heavier hits tear parts off entirely, which
//! then become free rigid bodies. Wheels can be knocked off and roll away. The engine, steering
//! and tyres pick up mechanical damage that the vehicle dynamics use.

use crate::body::{layers, BodyDesc, BodyHandle, ColliderDesc};
use crate::joint::{HingeLimits, HingeMotor, JointDesc, JointKind};
use crate::math::Pose;
use crate::shape::Shape;
use crate::vehicle::VehicleHandle;
use crate::world::{PhysicsEvent, PhysicsWorld};
use crate::JointHandle;
use glam::{Quat, Vec3};

/// A deformable triangle mesh in some local frame (chassis or part).
#[derive(Clone, Debug)]
pub struct DeformMesh {
    pub rest: Vec<Vec3>,
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub indices: Vec<u32>,
    /// Bumped whenever positions change (renderers re-upload when it moves).
    pub revision: u64,
}

impl DeformMesh {
    pub fn new(positions: Vec<Vec3>, indices: Vec<u32>) -> Self {
        let mut m = Self {
            rest: positions.clone(),
            normals: vec![Vec3::Y; positions.len()],
            positions,
            indices,
            revision: 0,
        };
        m.recompute_normals();
        m
    }

    /// A subdivided box surface (useful as a placeholder body or in tests).
    pub fn grid_box(center: Vec3, half: Vec3, n: usize) -> Self {
        let mut pos = Vec::new();
        let mut idx = Vec::new();
        let axes = [
            (Vec3::X, Vec3::Y, Vec3::Z),
            (Vec3::Y, Vec3::Z, Vec3::X),
            (Vec3::Z, Vec3::X, Vec3::Y),
        ];
        for (nrm, u, v) in axes {
            for sign in [-1.0f32, 1.0] {
                let base = pos.len() as u32;
                let nn = nrm * sign;
                for j in 0..=n {
                    for i in 0..=n {
                        let a = i as f32 / n as f32 * 2.0 - 1.0;
                        let b = j as f32 / n as f32 * 2.0 - 1.0;
                        pos.push(center + (nn + u * a + v * b) * half);
                    }
                }
                for j in 0..n as u32 {
                    for i in 0..n as u32 {
                        let k = base + j * (n as u32 + 1) + i;
                        let r = n as u32 + 1;
                        if sign > 0.0 {
                            idx.extend_from_slice(&[k, k + 1, k + r + 1, k, k + r + 1, k + r]);
                        } else {
                            idx.extend_from_slice(&[k, k + r + 1, k + 1, k, k + r, k + r + 1]);
                        }
                    }
                }
            }
        }
        Self::new(pos, idx)
    }

    pub fn recompute_normals(&mut self) {
        for n in &mut self.normals {
            *n = Vec3::ZERO;
        }
        for t in self.indices.chunks_exact(3) {
            let (a, b, c) = (t[0] as usize, t[1] as usize, t[2] as usize);
            let n = (self.positions[b] - self.positions[a])
                .cross(self.positions[c] - self.positions[a]);
            self.normals[a] += n;
            self.normals[b] += n;
            self.normals[c] += n;
        }
        for n in &mut self.normals {
            *n = n.normalize_or(Vec3::Y);
        }
    }

    /// Push vertices near `point` along `dir` (unit, into the surface) by up to `depth` metres
    /// within `radius`. No vertex moves more than `max_crush` from its rest position.
    /// Returns the largest displacement applied.
    pub fn dent(&mut self, point: Vec3, dir: Vec3, depth: f32, radius: f32, max_crush: f32) -> f32 {
        let mut moved: f32 = 0.0;
        let r2 = radius * radius;
        for (i, p) in self.positions.iter_mut().enumerate() {
            let d2 = p.distance_squared(point);
            if d2 >= r2 {
                continue;
            }
            let f = 1.0 - d2 / r2;
            let f = f * f;
            // Crumple noise from a hash of the rest position: wrinkles instead of a smooth bowl.
            let rp = self.rest[i];
            let h = ((rp.x * 12.9898 + rp.y * 78.233 + rp.z * 37.719).sin() * 43758.547)
                .fract()
                .abs();
            let crumple = 0.65 + 0.7 * h;
            let mut np = *p + dir * (depth * f * crumple);
            let off = np - rp;
            let l = off.length();
            if l > max_crush {
                np = rp + off * (max_crush / l);
            }
            moved = moved.max((np - *p).length());
            *p = np;
        }
        if moved > 1e-4 {
            self.revision += 1;
        }
        moved
    }

    /// Restore the rest shape (repair).
    pub fn repair(&mut self) {
        self.positions.clone_from(&self.rest);
        self.recompute_normals();
        self.revision += 1;
    }

    pub fn bounds(&self) -> (Vec3, Vec3) {
        let mut lo = Vec3::splat(f32::MAX);
        let mut hi = Vec3::splat(f32::MIN);
        for p in &self.positions {
            lo = lo.min(*p);
            hi = hi.max(*p);
        }
        (lo, hi)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartKind {
    FrontBumper,
    RearBumper,
    Hood,
    Trunk,
    Door,
    Mirror,
    /// Lights and windows shatter rather than detach.
    Glass,
    Light,
}

#[derive(Clone, Copy, Debug)]
pub enum PartMount {
    /// Bolted or clipped on: comes off in one go.
    Fixed,
    /// Hinged (doors, hood, trunk): latch breaks first, then the hinge.
    /// Axis and pivot are in the chassis frame; limits in radians.
    Hinge {
        pivot: Vec3,
        axis: Vec3,
        lower: f32,
        upper: f32,
    },
}

#[derive(Clone, Debug)]
pub struct PartDesc {
    pub name: String,
    pub kind: PartKind,
    pub mount: PartMount,
    /// Part mesh in the chassis frame.
    pub mesh: DeformMesh,
    pub mass: f32,
    /// Impulse (N·s) the part absorbs before failing.
    pub strength: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartState {
    Attached,
    /// Latch broken; swinging on its hinge.
    Loose,
    Detached,
    Shattered,
}

#[derive(Clone, Debug)]
pub struct Part {
    pub desc: PartDesc,
    pub health: f32,
    pub state: PartState,
    /// Body once loose/detached.
    pub body: Option<BodyHandle>,
    pub joint: Option<JointHandle>,
    /// Pose of the part mesh frame relative to its body (identity for chassis-attached parts).
    pub mesh_offset: Pose,
}

#[derive(Clone, Copy, Debug)]
pub enum DamageEvent {
    Dent { point: Vec3, depth: f32 },
    PartLoose { part: usize },
    PartDetached { part: usize, body: BodyHandle },
    Shattered { part: usize, point: Vec3 },
    WheelDetached { wheel: usize, body: BodyHandle },
    TyrePuncture { wheel: usize },
    EngineDamaged { health: f32 },
}

/// Tuning for how easily a vehicle deforms.
#[derive(Clone, Copy, Debug)]
pub struct DamageTuning {
    /// Impulse (N·s) per metre of dent depth.
    pub stiffness: f32,
    pub max_crush: f32,
    /// Ignore impacts below this impulse (N·s).
    pub threshold: f32,
    pub engine_strength: f32,
    pub wheel_strength: f32,
}

impl Default for DamageTuning {
    fn default() -> Self {
        Self {
            stiffness: 60000.0,
            max_crush: 0.45,
            threshold: 900.0,
            engine_strength: 90000.0,
            wheel_strength: 40000.0,
        }
    }
}

/// Damage state for one vehicle: deformable body, parts and mechanical consequences.
#[derive(Clone, Debug)]
pub struct VehicleDamage {
    pub vehicle: VehicleHandle,
    pub body: DeformMesh,
    pub parts: Vec<Part>,
    pub tuning: DamageTuning,
    /// Accumulated impulse per wheel (for punctures and knocked-off wheels).
    pub wheel_hits: Vec<f32>,
    /// Total absorbed impulse (overall wreck level).
    pub total: f32,
}

impl VehicleDamage {
    pub fn new(
        vehicle: VehicleHandle,
        body: DeformMesh,
        parts: Vec<PartDesc>,
        wheels: usize,
    ) -> Self {
        Self {
            vehicle,
            body,
            parts: parts
                .into_iter()
                .map(|desc| Part {
                    health: 1.0,
                    state: PartState::Attached,
                    body: None,
                    joint: None,
                    mesh_offset: Pose::IDENTITY,
                    desc,
                })
                .collect(),
            tuning: DamageTuning::default(),
            wheel_hits: vec![0.0; wheels],
            total: 0.0,
        }
    }

    /// Consume the world's events from the last step. Returns what broke.
    pub fn process(&mut self, world: &mut PhysicsWorld) -> Vec<DamageEvent> {
        let Some(v) = world.vehicle(self.vehicle) else {
            return Vec::new();
        };
        let chassis = v.chassis;
        let mut hits: Vec<(Vec3, Vec3, f32)> = Vec::new();
        for e in world.events() {
            if let PhysicsEvent::Impact {
                body_a,
                body_b,
                point,
                normal,
                impulse,
                ..
            } = *e
            {
                // Normal points A -> B; the dent goes into whichever side is ours.
                let dir = if body_a == chassis {
                    -normal
                } else if body_b == chassis {
                    normal
                } else {
                    continue;
                };
                // Remove the resting/steady part: only the impact's excess counts.
                if impulse > self.tuning.threshold {
                    hits.push((point, dir, impulse - self.tuning.threshold));
                }
            }
        }
        let mut out = Vec::new();
        for (point, dir, impulse) in hits {
            self.apply_hit(world, point, dir, impulse, &mut out);
        }
        out
    }

    /// Apply one impact in world space (also usable for scripted damage: bullets, explosions).
    pub fn apply_hit(
        &mut self,
        world: &mut PhysicsWorld,
        point: Vec3,
        dir: Vec3,
        impulse: f32,
        out: &mut Vec<DamageEvent>,
    ) {
        let Some(v) = world.vehicle(self.vehicle) else {
            return;
        };
        let chassis = v.chassis;
        let Some(body) = world.body(chassis) else {
            return;
        };
        let pose = body.pose;
        let lp = pose.inverse_transform_point(point);
        // Direction into the car, in chassis space.
        let ld = pose.inverse_transform_vector(dir).normalize_or_zero();
        let t = self.tuning;
        self.total += impulse;
        let depth = (impulse / t.stiffness).min(t.max_crush);
        let radius = (0.35 + impulse / 60000.0).min(1.3);
        let moved = self.body.dent(lp, ld, depth, radius, t.max_crush);
        if moved > 0.005 {
            self.body.recompute_normals();
            out.push(DamageEvent::Dent {
                point,
                depth: moved,
            });
        }
        // Parts near the hit.
        for (pi, part) in self.parts.iter_mut().enumerate() {
            if matches!(part.state, PartState::Detached | PartState::Shattered) {
                continue;
            }
            let (lo, hi) = part.desc.mesh.bounds();
            let closest = lp.clamp(lo, hi);
            let dist = closest.distance(lp);
            if dist > radius {
                continue;
            }
            let falloff = 1.0 - dist / radius;
            if part.state == PartState::Attached {
                part.desc.mesh.dent(lp, ld, depth, radius, t.max_crush);
                part.desc.mesh.recompute_normals();
            }
            part.health -= impulse * falloff / part.desc.strength.max(1.0);
            match part.desc.kind {
                PartKind::Glass | PartKind::Light => {
                    if part.health <= 0.0 {
                        part.state = PartState::Shattered;
                        out.push(DamageEvent::Shattered { part: pi, point });
                    }
                }
                _ => {}
            }
        }
        // Engine (front third of the car).
        let len = world
            .vehicle(self.vehicle)
            .map(|v| v.desc.size.z)
            .unwrap_or(4.5);
        if lp.z < -len * 0.2 {
            let dmg = impulse / t.engine_strength;
            if let Some(v) = world.vehicle_mut(self.vehicle) {
                v.damage.engine = (v.damage.engine - dmg).max(0.0);
                if v.damage.engine < 0.6 {
                    v.damage.radiator_leak = true;
                }
                out.push(DamageEvent::EngineDamaged {
                    health: v.damage.engine,
                });
            }
        }
        // Wheels close to the hit.
        let wheel_info: Vec<(Vec3, f32)> = world
            .vehicle(self.vehicle)
            .map(|v| {
                v.desc
                    .wheels
                    .iter()
                    .map(|w| (w.hardpoint, w.radius))
                    .collect()
            })
            .unwrap_or_default();
        for (wi, (hp, r)) in wheel_info.iter().enumerate() {
            let d =
                Vec3::new(lp.x - hp.x, (lp.y - (hp.y - r)).max(0.0) * 0.5, lp.z - hp.z).length();
            if d > 0.9 {
                continue;
            }
            let share = impulse * (1.0 - d / 0.9);
            self.wheel_hits[wi] += share;
            let hits = self.wheel_hits[wi];
            let Some(v) = world.vehicle_mut(self.vehicle) else {
                break;
            };
            let w = &mut v.wheels[wi];
            if w.detached {
                continue;
            }
            let was = w.damage;
            w.damage = (hits / t.wheel_strength).min(1.0);
            if was < 0.5 && w.damage >= 0.5 {
                out.push(DamageEvent::TyrePuncture { wheel: wi });
            }
            // Bent steering on the front wheels.
            if v.desc.wheels[wi].steer != 0.0 {
                let pull = (share / t.wheel_strength) * 0.04 * if hp.x > 0.0 { 1.0 } else { -1.0 };
                v.damage.steering_offset = (v.damage.steering_offset + pull).clamp(-0.12, 0.12);
            }
            if hits > t.wheel_strength * 2.2 {
                if let Some(b) = knock_off_wheel(world, self.vehicle, wi) {
                    out.push(DamageEvent::WheelDetached { wheel: wi, body: b });
                }
            }
        }
        // Part failures that spawn bodies.
        for pi in 0..self.parts.len() {
            let (state, health) = (self.parts[pi].state, self.parts[pi].health);
            if matches!(self.parts[pi].desc.kind, PartKind::Glass | PartKind::Light) {
                continue;
            }
            match (state, self.parts[pi].desc.mount) {
                (PartState::Attached, PartMount::Hinge { .. }) if health < 0.5 => {
                    if self.unlatch(world, chassis, pi) {
                        out.push(DamageEvent::PartLoose { part: pi });
                    }
                }
                (PartState::Attached, PartMount::Fixed) if health < 0.0 => {
                    if let Some(b) = self.detach(world, chassis, pi) {
                        out.push(DamageEvent::PartDetached { part: pi, body: b });
                    }
                }
                (PartState::Loose, _) if health < -0.4 => {
                    if let Some(j) = self.parts[pi].joint.take() {
                        world.remove_joint(j);
                    }
                    self.parts[pi].state = PartState::Detached;
                    if let Some(b) = self.parts[pi].body {
                        out.push(DamageEvent::PartDetached { part: pi, body: b });
                    }
                }
                _ => {}
            }
        }
    }

    fn spawn_part_body(
        world: &mut PhysicsWorld,
        chassis: BodyHandle,
        part: &Part,
    ) -> Option<(BodyHandle, Pose)> {
        let cb = world.body(chassis)?;
        let (pose, lin, ang) = (cb.pose, cb.lin_vel, cb.ang_vel);
        let mesh = &part.desc.mesh;
        let (lo, hi) = mesh.bounds();
        let center = (lo + hi) * 0.5;
        // Hull of a decimated vertex set, in the part body frame (centred on the part).
        let step = (mesh.positions.len() / 48).max(1);
        let pts: Vec<Vec3> = mesh
            .positions
            .iter()
            .step_by(step)
            .map(|p| *p - center)
            .collect();
        let shape = Shape::convex(&pts)
            .unwrap_or_else(|| Shape::cuboid(((hi - lo) * 0.5).max(Vec3::splat(0.02))));
        let body_pose = pose.mul(&Pose::from_translation(center));
        let world_center = body_pose.p;
        let b = world.add_body(BodyDesc {
            lin_vel: lin + ang.cross(world_center - cb.com()),
            ang_vel: ang,
            ..BodyDesc::dynamic(body_pose)
        });
        let vol = shape.mass_properties(1.0).mass.max(1e-4);
        world.add_collider(
            b,
            ColliderDesc::new(shape)
                .density(part.desc.mass / vol)
                .friction(0.5)
                .filter(layers::DEBRIS, layers::ALL & !layers::WHEEL_RAY),
        );
        Some((b, Pose::from_translation(-center)))
    }

    fn unlatch(&mut self, world: &mut PhysicsWorld, chassis: BodyHandle, pi: usize) -> bool {
        let PartMount::Hinge {
            pivot,
            axis,
            lower,
            upper,
        } = self.parts[pi].desc.mount
        else {
            return false;
        };
        let Some((b, offset)) = Self::spawn_part_body(world, chassis, &self.parts[pi]) else {
            return false;
        };
        // Hinge frame: X along the axis, at the pivot, in both bodies.
        let q = Quat::from_rotation_arc(Vec3::X, axis.normalize_or(Vec3::Y));
        let fa = Pose::new(pivot, q);
        let center = -offset.p;
        let fb = Pose::new(pivot - center, q);
        let j = world.add_joint(
            JointDesc::new(
                chassis,
                b,
                fa,
                fb,
                JointKind::Hinge {
                    limits: Some(HingeLimits { lower, upper }),
                    motor: Some(HingeMotor {
                        speed: 0.0,
                        max_torque: 4.0,
                    }),
                    spring: None,
                },
            )
            .breakable(60000.0, 9000.0),
        );
        let part = &mut self.parts[pi];
        part.state = PartState::Loose;
        part.body = Some(b);
        part.joint = Some(j);
        part.mesh_offset = offset;
        true
    }

    fn detach(
        &mut self,
        world: &mut PhysicsWorld,
        chassis: BodyHandle,
        pi: usize,
    ) -> Option<BodyHandle> {
        let (b, offset) = Self::spawn_part_body(world, chassis, &self.parts[pi])?;
        let part = &mut self.parts[pi];
        part.state = PartState::Detached;
        part.body = Some(b);
        part.mesh_offset = offset;
        Some(b)
    }

    /// World pose to draw a part's mesh with (chassis pose while attached).
    pub fn part_pose(&self, world: &PhysicsWorld, pi: usize) -> Option<Pose> {
        let part = self.parts.get(pi)?;
        match part.body {
            Some(b) => world.body(b).map(|bb| bb.pose.mul(&part.mesh_offset)),
            None => world
                .vehicle(self.vehicle)
                .and_then(|v| world.body(v.chassis))
                .map(|c| c.pose),
        }
    }

    /// Watch loose parts: broken hinge joints mean the part is gone.
    pub fn update(&mut self, world: &PhysicsWorld) -> Vec<DamageEvent> {
        let mut out = Vec::new();
        for (pi, part) in self.parts.iter_mut().enumerate() {
            if part.state == PartState::Loose {
                if let Some(j) = part.joint {
                    if world.joint(j).is_none() {
                        part.joint = None;
                        part.state = PartState::Detached;
                        if let Some(b) = part.body {
                            out.push(DamageEvent::PartDetached { part: pi, body: b });
                        }
                    }
                }
            }
        }
        out
    }

    /// Full repair: restore meshes and mechanical state, delete loose part bodies.
    pub fn repair(&mut self, world: &mut PhysicsWorld) {
        self.body.repair();
        for p in &mut self.parts {
            if let Some(j) = p.joint.take() {
                world.remove_joint(j);
            }
            if let Some(b) = p.body.take() {
                world.remove_body(b);
            }
            p.desc.mesh.repair();
            p.health = 1.0;
            p.state = PartState::Attached;
            p.mesh_offset = Pose::IDENTITY;
        }
        self.wheel_hits.iter_mut().for_each(|w| *w = 0.0);
        self.total = 0.0;
        if let Some(v) = world.vehicle_mut(self.vehicle) {
            v.damage = Default::default();
            for w in &mut v.wheels {
                w.damage = 0.0;
                w.detached = false;
            }
        }
    }
}

/// Detach a wheel: the vehicle loses it and a free wheel body rolls away. Returns that body.
pub fn knock_off_wheel(
    world: &mut PhysicsWorld,
    vh: VehicleHandle,
    wi: usize,
) -> Option<BodyHandle> {
    let v = world.vehicle(vh)?;
    if v.wheels.get(wi)?.detached {
        return None;
    }
    let chassis = world.body(v.chassis)?;
    let cpose = chassis.pose;
    let (lin, ang, com) = (chassis.lin_vel, chassis.ang_vel, chassis.com());
    let d = v.desc.wheels[wi].clone();
    let wp = v.wheel_pose(wi, &cpose);
    let omega = v.wheels[wi].omega;
    // Cylinder hull (axis along local X).
    let mut pts = Vec::new();
    for k in 0..16 {
        let a = k as f32 / 16.0 * std::f32::consts::TAU;
        for x in [-d.width * 0.5, d.width * 0.5] {
            pts.push(Vec3::new(x, a.cos() * d.radius, a.sin() * d.radius));
        }
    }
    let shape = Shape::convex(&pts)?;
    let axle = wp.q * Vec3::X;
    let b = world.add_body(BodyDesc {
        lin_vel: lin + ang.cross(wp.p - com),
        ang_vel: ang - axle * omega,
        ..BodyDesc::dynamic(Pose::new(wp.p, wp.q))
    });
    let vol = shape.mass_properties(1.0).mass;
    world.add_collider(
        b,
        ColliderDesc::new(shape)
            .density(22.0 / vol)
            .friction(0.9)
            .rolling_resistance(0.01)
            .filter(layers::DEBRIS, layers::ALL & !layers::WHEEL_RAY),
    );
    let v = world.vehicle_mut(vh)?;
    v.wheels[wi].detached = true;
    v.wheels[wi].in_contact = false;
    Some(b)
}
