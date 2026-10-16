//! The physics world: owns bodies, colliders, joints, contacts and vehicles, and steps them.

use crate::aabb::Aabb;
use crate::arena::{Arena, Handle};
use crate::body::*;
use crate::broadphase::DynamicTree;
use crate::joint::{Joint, JointDesc};
use crate::math::Pose;
use crate::narrowphase::{collide, ManifoldGeom};
use crate::query::{QueryFilter, QueryView, RayHit};
use crate::shape::MassProperties;
use crate::solver::*;
use glam::{Mat3, Vec3};
use rustc_hash::{FxHashMap, FxHashSet};
use smallvec::SmallVec;

const AABB_MARGIN: f32 = 0.1;
const SPECULATIVE_DISTANCE: f32 = 0.02;
const SLEEP_LINEAR: f32 = 0.05;
const SLEEP_ANGULAR: f32 = 0.05;
const TIME_TO_SLEEP: f32 = 0.5;

/// A persistent contact point (warm-starting state).
#[derive(Clone, Copy, Debug, Default)]
pub struct ManifoldPoint {
    /// Point on A in A's origin frame.
    pub local_a: Vec3,
    /// Point on B in B's origin frame.
    pub local_b: Vec3,
    /// World midpoint between the surfaces.
    pub point: Vec3,
    pub separation: f32,
    pub normal_impulse: f32,
    pub tangent_impulse: [f32; 2],
    /// Total normal impulse delivered during the last step (N·s).
    pub total_normal_impulse: f32,
    /// Normal approach speed before the solve (negative = closing).
    pub relative_velocity: f32,
}

#[derive(Clone, Debug, Default)]
pub struct Manifold {
    /// From A to B.
    pub normal: Vec3,
    pub points: SmallVec<[ManifoldPoint; 4]>,
    pub key: u32,
}

#[derive(Clone, Debug)]
pub struct Contact {
    pub collider_a: ColliderHandle,
    pub collider_b: ColliderHandle,
    pub body_a: BodyHandle,
    pub body_b: BodyHandle,
    pub manifolds: SmallVec<[Manifold; 1]>,
    pub friction: f32,
    pub restitution: f32,
    /// Rolling resistance lever arm (m).
    pub rolling: f32,
    pub touching: bool,
    pub sensor: bool,
}

/// Something that happened during the last step.
#[derive(Clone, Copy, Debug)]
pub enum PhysicsEvent {
    ContactBegin { a: ColliderHandle, b: ColliderHandle, sensor: bool },
    ContactEnd { a: ColliderHandle, b: ColliderHandle, sensor: bool },
    /// A collision with a noticeable approach speed (for damage, sounds, rumble, particles).
    Impact { a: ColliderHandle, b: ColliderHandle, body_a: BodyHandle, body_b: BodyHandle, point: Vec3, normal: Vec3, impulse: f32, speed: f32 },
    JointBroken { joint: JointHandle, body_a: BodyHandle, body_b: BodyHandle },
}

#[derive(Clone, Copy, Debug, Default)]
pub struct StepStats {
    pub bodies_awake: usize,
    pub contacts: usize,
    pub touching: usize,
    pub constraints: usize,
    pub joints: usize,
    pub broadphase_us: f32,
    pub narrowphase_us: f32,
    pub solver_us: f32,
    pub total_us: f32,
}

/// Per-substep hook used by vehicles and other force generators.
pub struct SubstepContext<'a> {
    pub query: QueryView<'a>,
    pub bodies: &'a mut [SolverBody],
    /// Body slot index -> solver index (u32::MAX if not in the solve).
    pub solver_of: &'a [u32],
    pub h: f32,
    pub gravity: Vec3,
}

impl SubstepContext<'_> {
    #[inline]
    pub fn solver_index(&self, b: BodyHandle) -> Option<usize> {
        let i = *self.solver_of.get(b.0.index as usize)?;
        if i == u32::MAX {
            None
        } else {
            Some(i as usize)
        }
    }
}

pub struct PhysicsWorld {
    pub gravity: Vec3,
    pub config: SolverConfig,
    /// Minimum approach speed for `PhysicsEvent::Impact` (m/s).
    pub impact_speed_threshold: f32,
    pub(crate) bodies: Arena<RigidBody>,
    pub(crate) colliders: Arena<Collider>,
    pub(crate) joints: Arena<Joint>,
    pub(crate) contacts: Arena<Contact>,
    pair_map: FxHashMap<(u32, u32), Handle>,
    static_tree: DynamicTree,
    dynamic_tree: DynamicTree,
    body_contacts: Vec<SmallVec<[Handle; 8]>>,
    body_joints: Vec<SmallVec<[Handle; 4]>>,
    no_collide_pairs: FxHashMap<(u32, u32), u32>,
    events: Vec<PhysicsEvent>,
    moved: Vec<u32>,
    pub vehicles: Arena<crate::vehicle::Vehicle>,
    pub stats: StepStats,
    pub time: f64,
    pub step_count: u64,
    // Scratch.
    solver_bodies: Vec<SolverBody>,
    solver_of: Vec<u32>,
    constraints: Vec<ContactConstraint>,
    geom: Vec<ManifoldGeom>,
}

impl Default for PhysicsWorld {
    fn default() -> Self {
        Self::new(Vec3::new(0.0, -9.81, 0.0))
    }
}

#[inline]
fn pair_key(a: u32, b: u32) -> (u32, u32) {
    if a < b {
        (a, b)
    } else {
        (b, a)
    }
}

impl PhysicsWorld {
    pub fn new(gravity: Vec3) -> Self {
        Self {
            gravity,
            config: SolverConfig::default(),
            impact_speed_threshold: 0.6,
            bodies: Arena::new(),
            colliders: Arena::new(),
            joints: Arena::new(),
            contacts: Arena::new(),
            pair_map: FxHashMap::default(),
            static_tree: DynamicTree::new(),
            dynamic_tree: DynamicTree::new(),
            body_contacts: Vec::new(),
            body_joints: Vec::new(),
            no_collide_pairs: FxHashMap::default(),
            events: Vec::new(),
            moved: Vec::new(),
            vehicles: Arena::new(),
            stats: StepStats::default(),
            time: 0.0,
            step_count: 0,
            solver_bodies: Vec::new(),
            solver_of: Vec::new(),
            constraints: Vec::new(),
            geom: Vec::new(),
        }
    }

    // ---------------------------------------------------------------- bodies

    pub fn add_body(&mut self, desc: BodyDesc) -> BodyHandle {
        let h = BodyHandle(self.bodies.insert(RigidBody::from_desc(&desc)));
        let idx = h.0.index as usize;
        if self.body_contacts.len() <= idx {
            self.body_contacts.resize_with(idx + 1, SmallVec::new);
            self.body_joints.resize_with(idx + 1, SmallVec::new);
        }
        self.body_contacts[idx].clear();
        self.body_joints[idx].clear();
        h
    }

    /// Convenience: a body with one collider.
    pub fn add_body_with(&mut self, desc: BodyDesc, collider: ColliderDesc) -> (BodyHandle, ColliderHandle) {
        let b = self.add_body(desc);
        let c = self.add_collider(b, collider);
        (b, c)
    }

    pub fn body(&self, h: BodyHandle) -> Option<&RigidBody> {
        self.bodies.get(h.0)
    }
    /// Mutable access. Changing pose/velocity here does not wake the body; prefer the setters.
    pub fn body_mut(&mut self, h: BodyHandle) -> Option<&mut RigidBody> {
        self.bodies.get_mut(h.0)
    }
    pub fn bodies(&self) -> impl Iterator<Item = (BodyHandle, &RigidBody)> {
        self.bodies.iter().map(|(h, b)| (BodyHandle(h), b))
    }
    pub fn body_count(&self) -> usize {
        self.bodies.len()
    }

    pub fn remove_body(&mut self, h: BodyHandle) {
        let Some(body) = self.bodies.get(h.0) else { return };
        let colliders = body.colliders.clone();
        self.wake_neighbours(h);
        for c in colliders {
            self.remove_collider_inner(c, false);
        }
        let joints: SmallVec<[Handle; 4]> = self.body_joints[h.0.index as usize].clone();
        for j in joints {
            self.remove_joint(JointHandle(j));
        }
        self.bodies.remove(h.0);
    }

    pub fn set_pose(&mut self, h: BodyHandle, pose: Pose) {
        if let Some(b) = self.bodies.get_mut(h.0) {
            b.pose = pose;
            b.prev_pose = pose;
            let cols = b.colliders.clone();
            for c in cols {
                self.refresh_collider(c, true);
            }
            self.wake(h);
        }
    }
    pub fn set_velocity(&mut self, h: BodyHandle, lin: Vec3, ang: Vec3) {
        if let Some(b) = self.bodies.get_mut(h.0) {
            b.lin_vel = lin;
            b.ang_vel = ang;
        }
        self.wake(h);
    }
    pub fn apply_force(&mut self, h: BodyHandle, f: Vec3, at: Option<Vec3>) {
        if let Some(b) = self.bodies.get_mut(h.0) {
            b.force += f;
            if let Some(p) = at {
                b.torque += (p - b.com()).cross(f);
            }
        }
        self.wake(h);
    }
    pub fn apply_torque(&mut self, h: BodyHandle, t: Vec3) {
        if let Some(b) = self.bodies.get_mut(h.0) {
            b.torque += t;
        }
        self.wake(h);
    }
    pub fn apply_impulse(&mut self, h: BodyHandle, j: Vec3, at: Vec3) {
        if let Some(b) = self.bodies.get_mut(h.0) {
            if b.is_dynamic() {
                b.lin_vel += j * b.inv_mass;
                let r = at - b.com();
                b.ang_vel += b.inv_inertia_world() * r.cross(j);
            }
        }
        self.wake(h);
    }

    /// Fix the mass properties instead of deriving them from collider densities.
    pub fn set_mass_properties(&mut self, h: BodyHandle, mp: MassProperties) {
        if let Some(b) = self.bodies.get_mut(h.0) {
            b.mass_override = Some(mp);
        }
        self.update_mass(h);
    }

    fn update_mass(&mut self, h: BodyHandle) {
        let Some(b) = self.bodies.get(h.0) else { return };
        let mp = if let Some(m) = b.mass_override {
            m
        } else {
            let parts: SmallVec<[MassProperties; 4]> = b
                .colliders
                .iter()
                .filter_map(|c| self.colliders.get(c.0))
                .filter(|c| !c.sensor)
                .map(|c| c.shape.mass_properties(c.density).transformed(&c.local))
                .collect();
            MassProperties::combine(&parts)
        };
        let b = self.bodies.get_mut(h.0).unwrap();
        if b.body_type != BodyType::Dynamic || mp.mass <= 0.0 {
            b.mass = if b.body_type == BodyType::Dynamic { 1.0 } else { 0.0 };
            b.inv_mass = if b.body_type == BodyType::Dynamic { 1.0 } else { 0.0 };
            b.local_com = if b.body_type == BodyType::Dynamic { mp.com } else { Vec3::ZERO };
            b.inertia = if b.body_type == BodyType::Dynamic { Mat3::IDENTITY } else { Mat3::ZERO };
            b.inv_inertia_local = b.inertia;
            return;
        }
        b.mass = mp.mass;
        b.inv_mass = 1.0 / mp.mass;
        b.local_com = mp.com;
        b.inertia = mp.inertia;
        b.inv_inertia_local = crate::math::inverse_or_zero(mp.inertia);
    }

    // ---------------------------------------------------------------- colliders

    pub fn add_collider(&mut self, body: BodyHandle, d: ColliderDesc) -> ColliderHandle {
        let b = &self.bodies[body.0];
        let world = b.pose.mul(&d.local);
        let is_static = b.is_static();
        let aabb = d.shape.aabb(&world);
        let c = Collider {
            body,
            shape: d.shape,
            local: d.local,
            density: d.density,
            friction: d.friction,
            restitution: d.restitution,
            rolling_resistance: d.rolling_resistance,
            filter: d.filter,
            sensor: d.sensor,
            surface: d.surface,
            user_data: d.user_data,
            world,
            aabb,
            proxy: u32::MAX,
            is_static_tree: is_static,
        };
        let h = ColliderHandle(self.colliders.insert(c));
        let fat = aabb.expanded(if is_static { 0.0 } else { AABB_MARGIN });
        let tree = if is_static { &mut self.static_tree } else { &mut self.dynamic_tree };
        let proxy = tree.insert(fat, h.0.index);
        self.colliders[h.0].proxy = proxy;
        self.bodies[body.0].colliders.push(h);
        self.moved.push(h.0.index);
        self.update_mass(body);
        self.wake(body);
        h
    }

    pub fn collider(&self, h: ColliderHandle) -> Option<&Collider> {
        self.colliders.get(h.0)
    }
    pub fn collider_mut(&mut self, h: ColliderHandle) -> Option<&mut Collider> {
        self.colliders.get_mut(h.0)
    }
    pub fn colliders(&self) -> impl Iterator<Item = (ColliderHandle, &Collider)> {
        self.colliders.iter().map(|(h, c)| (ColliderHandle(h), c))
    }

    pub fn remove_collider(&mut self, h: ColliderHandle) {
        self.remove_collider_inner(h, true);
    }

    fn remove_collider_inner(&mut self, h: ColliderHandle, update_body: bool) {
        let Some(c) = self.colliders.get(h.0) else { return };
        let (proxy, st, body) = (c.proxy, c.is_static_tree, c.body);
        if st {
            self.static_tree.remove(proxy);
        } else {
            self.dynamic_tree.remove(proxy);
        }
        // Destroy contacts involving this collider.
        let contacts: Vec<Handle> = self.body_contacts.get(body.0.index as usize).map(|v| v.to_vec()).unwrap_or_default();
        for ch in contacts {
            if let Some(ct) = self.contacts.get(ch) {
                if ct.collider_a == h || ct.collider_b == h {
                    self.destroy_contact(ch);
                }
            }
        }
        self.colliders.remove(h.0);
        if let Some(b) = self.bodies.get_mut(body.0) {
            b.colliders.retain(|x| *x != h);
        }
        if update_body {
            self.update_mass(body);
            self.wake(body);
        }
    }

    fn refresh_collider(&mut self, h: ColliderHandle, force_move: bool) {
        let Some(c) = self.colliders.get(h.0) else { return };
        let b = &self.bodies[c.body.0];
        let world = b.pose.mul(&c.local);
        let aabb = c.shape.aabb(&world);
        let (proxy, st) = (c.proxy, c.is_static_tree);
        let c = &mut self.colliders[h.0];
        c.world = world;
        c.aabb = aabb;
        if force_move {
            if st {
                self.static_tree.update(proxy, aabb);
            } else {
                self.dynamic_tree.update(proxy, aabb.expanded(AABB_MARGIN));
            }
            self.moved.push(h.0.index);
        }
    }

    // ---------------------------------------------------------------- joints

    pub fn add_joint(&mut self, desc: JointDesc) -> JointHandle {
        let (a, b) = (desc.body_a, desc.body_b);
        let cc = desc.collide_connected;
        let h = JointHandle(self.joints.insert(Joint::new(desc)));
        self.body_joints[a.0.index as usize].push(h.0);
        self.body_joints[b.0.index as usize].push(h.0);
        if !cc {
            *self.no_collide_pairs.entry(pair_key(a.0.index, b.0.index)).or_insert(0) += 1;
            // Drop existing contacts between the two bodies.
            let cs: Vec<Handle> = self.body_contacts[a.0.index as usize].to_vec();
            for ch in cs {
                if let Some(ct) = self.contacts.get(ch) {
                    if (ct.body_a == a && ct.body_b == b) || (ct.body_a == b && ct.body_b == a) {
                        self.destroy_contact(ch);
                    }
                }
            }
        }
        self.wake(a);
        self.wake(b);
        h
    }

    pub fn joint(&self, h: JointHandle) -> Option<&Joint> {
        self.joints.get(h.0)
    }
    pub fn joint_mut(&mut self, h: JointHandle) -> Option<&mut Joint> {
        self.joints.get_mut(h.0)
    }

    pub fn remove_joint(&mut self, h: JointHandle) {
        let Some(j) = self.joints.remove(h.0) else { return };
        let (a, b) = (j.desc.body_a, j.desc.body_b);
        for body in [a, b] {
            if let Some(list) = self.body_joints.get_mut(body.0.index as usize) {
                list.retain(|x| *x != h.0);
            }
        }
        if !j.desc.collide_connected {
            let k = pair_key(a.0.index, b.0.index);
            if let Some(n) = self.no_collide_pairs.get_mut(&k) {
                *n -= 1;
                if *n == 0 {
                    self.no_collide_pairs.remove(&k);
                }
            }
        }
        self.wake(a);
        self.wake(b);
    }

    // ---------------------------------------------------------------- sleeping

    /// Wake a body and everything resting on or jointed to it.
    pub fn wake(&mut self, h: BodyHandle) {
        let Some(b) = self.bodies.get_mut(h.0) else { return };
        if b.is_static() {
            return;
        }
        b.sleep_time = 0.0;
        if !b.sleeping {
            return;
        }
        let mut stack: SmallVec<[BodyHandle; 16]> = SmallVec::new();
        stack.push(h);
        while let Some(x) = stack.pop() {
            let Some(b) = self.bodies.get_mut(x.0) else { continue };
            if !b.sleeping {
                continue;
            }
            b.sleeping = false;
            b.sleep_time = 0.0;
            for ch in &self.body_contacts[x.0.index as usize] {
                if let Some(c) = self.contacts.get(*ch) {
                    if c.touching {
                        let o = if c.body_a == x { c.body_b } else { c.body_a };
                        if self.bodies.get(o.0).is_some_and(|ob| ob.sleeping) {
                            stack.push(o);
                        }
                    }
                }
            }
            for jh in &self.body_joints[x.0.index as usize] {
                if let Some(j) = self.joints.get(*jh) {
                    let o = if j.desc.body_a == x { j.desc.body_b } else { j.desc.body_a };
                    if self.bodies.get(o.0).is_some_and(|ob| ob.sleeping) {
                        stack.push(o);
                    }
                }
            }
        }
    }

    fn wake_neighbours(&mut self, h: BodyHandle) {
        let list: SmallVec<[Handle; 8]> = self.body_contacts[h.0.index as usize].clone();
        for ch in list {
            if let Some(c) = self.contacts.get(ch) {
                let o = if c.body_a == h { c.body_b } else { c.body_a };
                self.wake(o);
            }
        }
    }

    // ---------------------------------------------------------------- queries

    pub fn query(&self) -> QueryView<'_> {
        QueryView { colliders: &self.colliders, bodies: &self.bodies, static_tree: &self.static_tree, dynamic_tree: &self.dynamic_tree }
    }
    pub fn ray_cast(&self, origin: Vec3, dir: Vec3, max_t: f32, filter: &QueryFilter) -> Option<RayHit> {
        self.query().ray_cast(origin, dir, max_t, filter)
    }
    pub fn sphere_cast(&self, center: Vec3, radius: f32, dir: Vec3, max_t: f32, filter: &QueryFilter) -> Option<RayHit> {
        self.query().sphere_cast(center, radius, dir, max_t, filter)
    }

    /// Events produced by the last `step`.
    pub fn events(&self) -> &[PhysicsEvent] {
        &self.events
    }
    pub fn contacts(&self) -> impl Iterator<Item = &Contact> {
        self.contacts.iter().map(|(_, c)| c)
    }
    pub fn contacts_of(&self, b: BodyHandle) -> impl Iterator<Item = &Contact> {
        self.body_contacts.get(b.0.index as usize).into_iter().flatten().filter_map(|h| self.contacts.get(*h))
    }

    // ---------------------------------------------------------------- step

    pub fn step(&mut self, dt: f32) {
        if dt <= 0.0 {
            return;
        }
        let t0 = std::time::Instant::now();
        self.events.clear();
        for (_, b) in self.bodies.iter_mut() {
            b.prev_pose = b.pose;
        }
        self.update_broadphase(dt);
        self.find_pairs();
        let t1 = std::time::Instant::now();
        self.narrowphase(dt);
        let t2 = std::time::Instant::now();
        self.solve(dt);
        self.finish_step(dt);
        let t3 = std::time::Instant::now();
        self.stats.broadphase_us = (t1 - t0).as_secs_f32() * 1e6;
        self.stats.narrowphase_us = (t2 - t1).as_secs_f32() * 1e6;
        self.stats.solver_us = (t3 - t2).as_secs_f32() * 1e6;
        self.stats.total_us = (t3 - t0).as_secs_f32() * 1e6;
        self.time += dt as f64;
        self.step_count += 1;
    }

    fn update_broadphase(&mut self, dt: f32) {
        for (bh, b) in self.bodies.iter() {
            if b.is_static() || b.sleeping {
                continue;
            }
            let disp = b.lin_vel * dt;
            for &ch in &b.colliders {
                let c = &mut self.colliders[ch.0];
                c.world = b.pose.mul(&c.local);
                c.aabb = c.shape.aabb(&c.world);
                let predicted = c.aabb.swept(disp);
                if !self.dynamic_tree.fat_aabb(c.proxy).contains(&predicted) {
                    let fat = predicted.expanded(AABB_MARGIN).swept(disp * 0.5);
                    self.dynamic_tree.update(c.proxy, fat);
                    self.moved.push(ch.0.index);
                }
            }
            let _ = bh;
        }
    }

    fn find_pairs(&mut self) {
        let moved = std::mem::take(&mut self.moved);
        let mut seen: FxHashSet<u32> = FxHashSet::default();
        let mut new_pairs: Vec<(u32, u32)> = Vec::new();
        for ci in moved {
            if !seen.insert(ci) {
                continue;
            }
            let Some((_, c)) = self.colliders.get_index(ci) else { continue };
            let fat = if c.is_static_tree { self.static_tree.fat_aabb(c.proxy) } else { self.dynamic_tree.fat_aabb(c.proxy) };
            let trees: &[&DynamicTree] = if c.is_static_tree { &[&self.dynamic_tree] } else { &[&self.static_tree, &self.dynamic_tree] };
            for tree in trees {
                tree.query(&fat, |proxy| {
                    let other = tree.data(proxy);
                    if other != ci {
                        new_pairs.push(pair_key(ci, other));
                    }
                    true
                });
            }
        }
        new_pairs.sort_unstable();
        new_pairs.dedup();
        for (a, b) in new_pairs {
            if self.pair_map.contains_key(&(a, b)) {
                continue;
            }
            let (Some((ha, ca)), Some((hb, cb))) = (self.colliders.get_index(a), self.colliders.get_index(b)) else { continue };
            if ca.body == cb.body || !ca.filter.test(&cb.filter) {
                continue;
            }
            let (ba, bb) = (&self.bodies[ca.body.0], &self.bodies[cb.body.0]);
            // At least one side must be dynamic (or a sensor watching a moving body).
            let sensor = ca.sensor || cb.sensor;
            if !ba.is_dynamic() && !bb.is_dynamic() && !(sensor && (!ba.is_static() || !bb.is_static())) {
                continue;
            }
            if self.no_collide_pairs.contains_key(&pair_key(ca.body.0.index, cb.body.0.index)) {
                continue;
            }
            let contact = Contact {
                collider_a: ColliderHandle(ha),
                collider_b: ColliderHandle(hb),
                body_a: ca.body,
                body_b: cb.body,
                manifolds: SmallVec::new(),
                friction: (ca.friction * cb.friction).sqrt(),
                restitution: ca.restitution.max(cb.restitution),
                rolling: ca.rolling_resistance.max(cb.rolling_resistance) * ca.shape.bounding_radius().min(cb.shape.bounding_radius()).min(1.0),
                touching: false,
                sensor,
            };
            let (ia, ib) = (ca.body.0.index as usize, cb.body.0.index as usize);
            let h = self.contacts.insert(contact);
            self.pair_map.insert((a, b), h);
            self.body_contacts[ia].push(h);
            self.body_contacts[ib].push(h);
        }
    }

    fn destroy_contact(&mut self, h: Handle) {
        let Some(c) = self.contacts.remove(h) else { return };
        let key = pair_key(c.collider_a.0.index, c.collider_b.0.index);
        self.pair_map.remove(&key);
        for b in [c.body_a, c.body_b] {
            if let Some(list) = self.body_contacts.get_mut(b.0.index as usize) {
                if let Some(pos) = list.iter().position(|x| *x == h) {
                    list.swap_remove(pos);
                }
            }
        }
        if c.touching {
            self.events.push(PhysicsEvent::ContactEnd { a: c.collider_a, b: c.collider_b, sensor: c.sensor });
        }
    }

    fn narrowphase(&mut self, dt: f32) {
        let handles = self.contacts.handles();
        let mut to_destroy: Vec<Handle> = Vec::new();
        let mut to_wake: Vec<BodyHandle> = Vec::new();
        let mut geom = std::mem::take(&mut self.geom);
        for h in handles {
            let c = &self.contacts[h];
            let (Some(ca), Some(cb)) = (self.colliders.get(c.collider_a.0), self.colliders.get(c.collider_b.0)) else {
                to_destroy.push(h);
                continue;
            };
            let ba = &self.bodies[c.body_a.0];
            let bb = &self.bodies[c.body_b.0];
            let a_active = !ba.is_static() && !ba.sleeping;
            let b_active = !bb.is_static() && !bb.sleeping;
            if !a_active && !b_active {
                continue;
            }
            let fat_a = if ca.is_static_tree { self.static_tree.fat_aabb(ca.proxy) } else { self.dynamic_tree.fat_aabb(ca.proxy) };
            let fat_b = if cb.is_static_tree { self.static_tree.fat_aabb(cb.proxy) } else { self.dynamic_tree.fat_aabb(cb.proxy) };
            if !fat_a.overlaps(&fat_b) {
                to_destroy.push(h);
                continue;
            }
            // Speculative margin grows with the closing speed so fast bodies do not tunnel.
            let rel = (ba.lin_vel - bb.lin_vel).length()
                + ba.ang_vel.length() * ca.shape.bounding_radius().min(4.0)
                + bb.ang_vel.length() * cb.shape.bounding_radius().min(4.0);
            let margin = (SPECULATIVE_DISTANCE + rel * dt).min(2.0);
            geom.clear();
            if c.sensor {
                collide(&ca.shape, &ca.world, &cb.shape, &cb.world, 0.0, &mut geom);
            } else {
                collide(&ca.shape, &ca.world, &cb.shape, &cb.world, margin, &mut geom);
            }
            let (pose_a, pose_b) = (ba.pose, bb.pose);
            let was_touching = c.touching;
            let sensor = c.sensor;
            let c = &mut self.contacts[h];
            let old = std::mem::take(&mut c.manifolds);
            let mut touching = false;
            for g in &geom {
                let mut m = Manifold { normal: g.normal, points: SmallVec::new(), key: g.key };
                let prev = old.iter().find(|o| o.key == g.key);
                for p in &g.points {
                    let la = pose_a.inverse_transform_point(p.pa);
                    let lb = pose_b.inverse_transform_point(p.pb);
                    let mut mp = ManifoldPoint { local_a: la, local_b: lb, point: (p.pa + p.pb) * 0.5, separation: p.separation, ..Default::default() };
                    if p.separation < 0.0 + 0.005 {
                        touching = true;
                    }
                    if let Some(prev) = prev {
                        // Warm start from the nearest matching old point.
                        let mut best = 0.05f32 * 0.05;
                        for op in &prev.points {
                            let d = op.local_a.distance_squared(la).min(op.local_b.distance_squared(lb));
                            if d < best {
                                best = d;
                                mp.normal_impulse = op.normal_impulse;
                                mp.tangent_impulse = op.tangent_impulse;
                            }
                        }
                    }
                    m.points.push(mp);
                }
                if !m.points.is_empty() {
                    c.manifolds.push(m);
                }
            }
            if sensor {
                c.manifolds.clear();
            }
            c.touching = touching;
            if touching != was_touching {
                let (a, b) = (c.collider_a, c.collider_b);
                self.events.push(if touching { PhysicsEvent::ContactBegin { a, b, sensor } } else { PhysicsEvent::ContactEnd { a, b, sensor } });
                if touching && !sensor {
                    if !a_active {
                        to_wake.push(c.body_a);
                    }
                    if !b_active {
                        to_wake.push(c.body_b);
                    }
                }
            }
        }
        self.geom = geom;
        for h in to_destroy {
            self.destroy_contact(h);
        }
        for b in to_wake {
            self.wake(b);
        }
    }

    fn solve(&mut self, dt: f32) {
        let n_sub = self.config.substeps.max(1);
        let h = dt / n_sub as f32;
        let inv_h = 1.0 / h;
        let cfg = self.config;

        // Gather solver bodies.
        let mut sb = std::mem::take(&mut self.solver_bodies);
        sb.clear();
        let cap = self.bodies.capacity_index();
        self.solver_of.clear();
        self.solver_of.resize(cap, u32::MAX);
        let make = |b: &RigidBody, index: u32, active: bool| {
            let dynamic = active && b.is_dynamic();
            let kinematic = active && b.body_type == BodyType::Kinematic;
            let r = Mat3::from_quat(b.pose.q);
            SolverBody {
                v: if active { b.lin_vel } else { Vec3::ZERO },
                w: if active { b.ang_vel } else { Vec3::ZERO },
                c: b.com(),
                q: b.pose.q,
                inv_mass: if dynamic { b.inv_mass } else { 0.0 },
                inv_i: if dynamic { r * b.inv_inertia_local * r.transpose() } else { Mat3::ZERO },
                inv_i_local: if dynamic { b.inv_inertia_local } else { Mat3::ZERO },
                local_com: b.local_com,
                force: b.force,
                torque: b.torque,
                gravity_scale: b.gravity_scale,
                linear_damping: b.linear_damping,
                angular_damping: b.angular_damping,
                dynamic,
                kinematic,
                body_index: index,
            }
        };
        let mut awake = 0;
        for (bh, b) in self.bodies.iter() {
            if b.is_static() || b.sleeping {
                continue;
            }
            self.solver_of[bh.index as usize] = sb.len() as u32;
            sb.push(make(b, bh.index, true));
            awake += 1;
        }
        let ensure = |sb: &mut Vec<SolverBody>, solver_of: &mut Vec<u32>, bh: BodyHandle, bodies: &Arena<RigidBody>| -> u32 {
            let i = solver_of[bh.0.index as usize];
            if i != u32::MAX {
                return i;
            }
            let b = &bodies[bh.0];
            let idx = sb.len() as u32;
            sb.push(make(b, bh.0.index, false));
            solver_of[bh.0.index as usize] = idx;
            idx
        };

        // Contact constraints.
        let mut constraints = std::mem::take(&mut self.constraints);
        constraints.clear();
        let soft = Softness::new(cfg.contact_hertz.min(0.25 / h), cfg.contact_damping_ratio, h);
        let static_soft = Softness::new((2.0 * cfg.contact_hertz).min(0.25 / h), cfg.contact_damping_ratio, h);
        let joint_soft = Softness::new((2.0 * cfg.contact_hertz).min(cfg.joint_hertz).min(0.25 / h), cfg.joint_damping_ratio, h);
        let mut world_pts: SmallVec<[(Vec3, f32); 4]> = SmallVec::new();
        let mut touching = 0;
        for (chh, c) in self.contacts.iter() {
            if c.sensor || c.manifolds.is_empty() {
                continue;
            }
            if c.touching {
                touching += 1;
            }
            let ba = &self.bodies[c.body_a.0];
            let bb = &self.bodies[c.body_b.0];
            let a_dyn = ba.is_dynamic() && !ba.sleeping;
            let b_dyn = bb.is_dynamic() && !bb.sleeping;
            if !a_dyn && !b_dyn {
                continue;
            }
            let ia = ensure(&mut sb, &mut self.solver_of, c.body_a, &self.bodies);
            let ib = ensure(&mut sb, &mut self.solver_of, c.body_b, &self.bodies);
            for (mi, m) in c.manifolds.iter().enumerate() {
                let mut cc = ContactConstraint {
                    a: ia,
                    b: ib,
                    normal: m.normal,
                    tangents: [Vec3::X, Vec3::Z],
                    friction: c.friction,
                    restitution: c.restitution,
                    softness: soft,
                    points: SmallVec::new(),
                    rolling: c.rolling,
                    rolling_mass: Mat3::ZERO,
                    rolling_impulse: Vec3::ZERO,
                    source: (chh.index, mi as u32),
                };
                world_pts.clear();
                for p in &m.points {
                    cc.points.push(ConstraintPoint { normal_impulse: p.normal_impulse, tangent_impulse: p.tangent_impulse, ..Default::default() });
                    world_pts.push((p.point, p.separation));
                }
                prepare_contact(&mut cc, &sb, &world_pts, h, &cfg, static_soft, soft);
                constraints.push(cc);
            }
        }

        // Joints.
        let mut active_joints: SmallVec<[Handle; 32]> = SmallVec::new();
        for (jh, j) in self.joints.iter() {
            if j.broken {
                continue;
            }
            let ba = &self.bodies[j.desc.body_a.0];
            let bb = &self.bodies[j.desc.body_b.0];
            if !(ba.is_dynamic() && !ba.sleeping) && !(bb.is_dynamic() && !bb.sleeping) {
                continue;
            }
            active_joints.push(jh);
        }
        for &jh in &active_joints {
            let (a, b) = {
                let j = &self.joints[jh];
                (j.desc.body_a, j.desc.body_b)
            };
            let ia = ensure(&mut sb, &mut self.solver_of, a, &self.bodies);
            let ib = ensure(&mut sb, &mut self.solver_of, b, &self.bodies);
            self.joints[jh].prepare(ia, ib, &sb, h, joint_soft);
        }

        // Substeps.
        for _ in 0..n_sub {
            integrate_velocities(&mut sb, self.gravity, h, &cfg);
            if !self.vehicles.is_empty() {
                let mut ctx = SubstepContext {
                    query: QueryView { colliders: &self.colliders, bodies: &self.bodies, static_tree: &self.static_tree, dynamic_tree: &self.dynamic_tree },
                    bodies: &mut sb,
                    solver_of: &self.solver_of,
                    h,
                    gravity: self.gravity,
                };
                crate::vehicle::substep_vehicles(&mut self.vehicles, &mut ctx);
            }
            for &jh in &active_joints {
                self.joints[jh].warm_start(&mut sb);
            }
            for c in &constraints {
                warm_start_contact(c, &mut sb);
            }
            for &jh in &active_joints {
                self.joints[jh].solve(&mut sb, h, inv_h, true);
            }
            for c in constraints.iter_mut() {
                solve_contact(c, &mut sb, inv_h, true, &cfg);
            }
            integrate_positions(&mut sb, h);
            for &jh in &active_joints {
                self.joints[jh].solve(&mut sb, h, inv_h, false);
            }
            for c in constraints.iter_mut() {
                solve_contact(c, &mut sb, inv_h, false, &cfg);
                for p in c.points.iter_mut() {
                    p.total_normal_impulse += p.normal_impulse;
                }
            }
        }
        for c in constraints.iter_mut() {
            apply_restitution(c, &mut sb, &cfg);
        }

        // Store impulses and emit impacts.
        for cc in &constraints {
            let Some(c) = self.contacts.get_index(cc.source.0).map(|(h, _)| h) else { continue };
            let contact = &mut self.contacts[c];
            let m = &mut contact.manifolds[cc.source.1 as usize];
            let mut max_speed = 0.0f32;
            let mut impulse = 0.0f32;
            let mut point = Vec3::ZERO;
            for (mp, cp) in m.points.iter_mut().zip(&cc.points) {
                mp.normal_impulse = cp.normal_impulse;
                mp.tangent_impulse = cp.tangent_impulse;
                mp.total_normal_impulse = cp.total_normal_impulse;
                mp.relative_velocity = cp.relative_velocity;
                if -cp.relative_velocity > max_speed {
                    max_speed = -cp.relative_velocity;
                    point = mp.point;
                }
                impulse += cp.total_normal_impulse;
            }
            if max_speed > self.impact_speed_threshold && impulse > 0.0 {
                self.events.push(PhysicsEvent::Impact {
                    a: contact.collider_a,
                    b: contact.collider_b,
                    body_a: contact.body_a,
                    body_b: contact.body_b,
                    point,
                    normal: m.normal,
                    impulse,
                    speed: max_speed,
                });
            }
        }

        // Joint breaking.
        for &jh in &active_joints {
            let j = &mut self.joints[jh];
            if j.finish(h) {
                j.broken = true;
                let (a, b) = (j.desc.body_a, j.desc.body_b);
                self.events.push(PhysicsEvent::JointBroken { joint: JointHandle(jh), body_a: a, body_b: b });
            }
        }
        let broken: SmallVec<[Handle; 4]> = active_joints.iter().copied().filter(|j| self.joints[*j].broken).collect();
        for j in broken {
            self.remove_joint(JointHandle(j));
        }

        // Write back.
        for s in &sb {
            if !s.dynamic && !s.kinematic {
                continue;
            }
            let Some((bh, _)) = self.bodies.get_index(s.body_index) else { continue };
            let b = &mut self.bodies[bh];
            b.lin_vel = s.v;
            b.ang_vel = s.w;
            b.pose.q = s.q;
            b.pose.p = s.c - s.q * b.local_com;
        }
        self.stats.bodies_awake = awake;
        self.stats.contacts = self.contacts.len();
        self.stats.touching = touching;
        self.stats.constraints = constraints.len();
        self.stats.joints = active_joints.len();
        self.constraints = constraints;
        self.solver_bodies = sb;
    }

    fn finish_step(&mut self, dt: f32) {
        // Refresh collider world poses for queries.
        for (_, b) in self.bodies.iter() {
            if b.is_static() || b.sleeping {
                continue;
            }
            for &ch in &b.colliders {
                let c = &mut self.colliders[ch.0];
                c.world = b.pose.mul(&c.local);
                c.aabb = c.shape.aabb(&c.world);
            }
        }
        // Clear forces, accumulate sleep timers.
        for (_, b) in self.bodies.iter_mut() {
            b.force = Vec3::ZERO;
            b.torque = Vec3::ZERO;
            if !b.is_dynamic() || b.sleeping {
                continue;
            }
            if b.can_sleep && b.lin_vel.length_squared() < SLEEP_LINEAR * SLEEP_LINEAR && b.ang_vel.length_squared() < SLEEP_ANGULAR * SLEEP_ANGULAR {
                b.sleep_time += dt;
            } else {
                b.sleep_time = 0.0;
            }
        }
        self.update_islands();
    }

    /// Union-find over awake dynamic bodies; islands that have been still long enough go to sleep.
    fn update_islands(&mut self) {
        let cap = self.bodies.capacity_index();
        let mut parent: Vec<u32> = (0..cap as u32).collect();
        fn find(p: &mut [u32], mut x: u32) -> u32 {
            while p[x as usize] != x {
                p[x as usize] = p[p[x as usize] as usize];
                x = p[x as usize];
            }
            x
        }
        let mut keep_awake = vec![false; cap];
        for (_, c) in self.contacts.iter() {
            if !c.touching || c.sensor {
                continue;
            }
            let ba = &self.bodies[c.body_a.0];
            let bb = &self.bodies[c.body_b.0];
            if ba.is_dynamic() && bb.is_dynamic() {
                let (ra, rb) = (find(&mut parent, c.body_a.0.index), find(&mut parent, c.body_b.0.index));
                parent[ra as usize] = rb;
            } else {
                // Touching a moving kinematic body prevents sleep.
                for (me, other) in [(ba, bb), (bb, ba)] {
                    let _ = me;
                    if other.body_type == BodyType::Kinematic && (other.lin_vel != Vec3::ZERO || other.ang_vel != Vec3::ZERO) {
                        keep_awake[c.body_a.0.index as usize] = true;
                        keep_awake[c.body_b.0.index as usize] = true;
                    }
                }
            }
        }
        for (_, j) in self.joints.iter() {
            let (a, b) = (j.desc.body_a.0.index, j.desc.body_b.0.index);
            if self.bodies[j.desc.body_a.0].is_dynamic() && self.bodies[j.desc.body_b.0].is_dynamic() {
                let (ra, rb) = (find(&mut parent, a), find(&mut parent, b));
                parent[ra as usize] = rb;
            }
        }
        let mut min_sleep = vec![f32::MAX; cap];
        for (bh, b) in self.bodies.iter() {
            if !b.is_dynamic() || b.sleeping {
                continue;
            }
            let r = find(&mut parent, bh.index) as usize;
            let t = if !b.can_sleep || keep_awake[bh.index as usize] { 0.0 } else { b.sleep_time };
            min_sleep[r] = min_sleep[r].min(t);
        }
        // Vehicles with throttle input never sleep.
        for (_, v) in self.vehicles.iter() {
            if !v.can_sleep() {
                let r = find(&mut parent, v.chassis.0.index) as usize;
                min_sleep[r] = 0.0;
            }
        }
        for (bh, b) in self.bodies.iter_mut() {
            if !b.is_dynamic() || b.sleeping {
                continue;
            }
            let r = find(&mut parent, bh.index) as usize;
            if min_sleep[r] >= TIME_TO_SLEEP {
                b.sleeping = true;
                b.lin_vel = Vec3::ZERO;
                b.ang_vel = Vec3::ZERO;
            }
        }
    }

    /// Bounds of all colliders (debug/camera framing).
    pub fn bounds(&self) -> Aabb {
        let mut a = Aabb::EMPTY;
        for (_, c) in self.colliders.iter() {
            a = a.union(&c.aabb);
        }
        a
    }
}
