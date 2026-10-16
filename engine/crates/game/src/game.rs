//! The game loop that couples the traffic simulation with the 3D physics world.
//!
//! - Lane-mode traffic cars are "puppets": dynamic bodies with their real mass whose velocity is
//!   set every step to follow the lane pose. A hit therefore transfers momentum properly; the
//!   traffic sim is told about the collision and switches the car to physics mode, at which
//!   point it becomes a full wheeled `Vehicle` driven by its AI's controls.
//! - Physics-mode cars (and the player) feed their poses back to the traffic sim, so law
//!   enforcement, AI perception, road rage and pedestrians all see the 3D motion.
//! - Impacts drive body deformation, part loss, prop breakage and destruction.

use crate::carmesh::{build_car, CarModel};
use crate::city::StaticCity;
use crate::coords::*;
use glam::Vec3;
use gridlock_input::{
    ActionEvent, ControlFrame, GamepadState, InputMapper, KeyboardState, Rumble, RumbleInputs,
    RumbleMixer,
};
use gridlock_physics::deform::{DamageEvent, VehicleDamage};
use gridlock_physics::fracture::DestructionSystem;
use gridlock_physics::props::{PropEvent, PropSystem};
use gridlock_physics::vehicle::powertrain::Selector;
use gridlock_physics::vehicle::presets::VehicleClass as PClass;
use gridlock_physics::water::{PuddleField, SprayParticles};
use gridlock_physics::{
    layers, BodyDesc, BodyHandle, ColliderDesc, PhysicsEvent, PhysicsWorld, Pose, Shape,
    VehicleControls, VehicleHandle,
};
use gridlock_traffic::{
    action_def, ActionId, Event, ExternalPose, MotionMode, VehicleClass as TClass, VehicleId,
    World, WorldOptions,
};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct GameConfig {
    pub seed: i64,
    pub traffic: usize,
    pub police: usize,
    pub start_hour: f64,
    pub player_class: TClass,
    pub props: bool,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            seed: 2077,
            traffic: 70,
            police: 2,
            start_hour: 17.5,
            player_class: TClass::Sedan,
            props: true,
        }
    }
}

pub enum CarState {
    /// Kinematically following its lane (dynamic body with velocity control).
    Lane { body: BodyHandle },
    /// Fully simulated wheeled vehicle.
    Physics {
        vehicle: VehicleHandle,
        damage: Box<VehicleDamage>,
    },
}

impl CarState {
    pub fn is_lane(&self) -> bool {
        matches!(self, CarState::Lane { .. })
    }
}

pub struct CarLink {
    pub state: CarState,
    pub class: PClass,
    /// Last chassis pose (for kinematic velocity targets and interpolation).
    pub last_pose: Pose,
}

/// Things the presentation layer reacts to (sounds, particles, HUD toasts, rumble).
#[derive(Clone, Debug)]
pub enum GameEvent {
    Traffic(Event),
    Damage {
        vehicle: VehicleId,
        event: DamageEvent,
    },
    Prop(PropEvent),
    Crash {
        a: Option<VehicleId>,
        b: Option<VehicleId>,
        point: Vec3,
        impulse: f32,
        speed: f32,
    },
    Shattered {
        point: Vec3,
        pieces: usize,
    },
}

pub struct Game {
    pub config: GameConfig,
    pub traffic: World,
    pub physics: PhysicsWorld,
    pub city: StaticCity,
    pub props: PropSystem,
    pub destruction: DestructionSystem,
    pub links: HashMap<VehicleId, CarLink>,
    body_owner: HashMap<BodyHandle, VehicleId>,
    static_bodies: usize,
    pub input: InputMapper,
    pub rumble: RumbleMixer,
    pub last_rumble: Rumble,
    pub spray: SprayParticles,
    pub events: Vec<GameEvent>,
    models: HashMap<PClass, CarModel>,
    action_ids: HashMap<&'static str, ActionId>,
    pub fixed_dt: f32,
    accumulator: f64,
    pub time: f64,
    last_gear: i32,
    frame_impact: f32,
    /// Impacts taken by puppets this step, replayed into the damage model if they convert.
    pending_hits: HashMap<VehicleId, Vec<(Vec3, Vec3, f32)>>,
}

fn pclass(c: TClass) -> PClass {
    PClass::ALL[c as usize]
}

impl Game {
    pub fn new(config: GameConfig) -> Self {
        let opts = WorldOptions {
            seed: config.seed,
            traffic: config.traffic,
            police: config.police,
            start_hour: config.start_hour,
            player_vehicle: config.player_class,
            external_physics: true,
            ..Default::default()
        };
        let traffic = World::new(opts);
        let city = StaticCity::build(&traffic);
        let mut physics = PhysicsWorld::default();
        city.add_to_physics(&mut physics);
        let static_bodies = physics.body_count();
        let mut props = PropSystem::new();
        if config.props {
            for (kind, pose) in &city.props {
                props.spawn(&mut physics, *kind, *pose);
            }
        }
        // Puddles collect along gutters (near curbs) and in random dips.
        let blocks = city.blocks.clone();
        let puddles = PuddleField::new(city.min, city.max - city.min, 1.0, move |x, z| {
            let mut near_curb = f32::MAX;
            for b in &blocks {
                let dx = (b.min.x - x).max(x - b.max.x).max(0.0);
                let dz = (b.min.y - z).max(z - b.max.y).max(0.0);
                let inside = x > b.min.x && x < b.max.x && z > b.min.y && z < b.max.y;
                if inside {
                    return 0.0;
                }
                near_curb = near_curb.min((dx * dx + dz * dz).sqrt());
            }
            let dip = ((x * 0.37).sin() * (z * 0.29).cos() * 0.5 + 0.5).powi(6) * 0.008;
            let gutter = if near_curb < 0.8 {
                0.006 * (1.0 - near_curb / 0.8)
            } else {
                0.0
            };
            dip + gutter
        });
        physics.puddles = Some(puddles);
        let mut action_ids = HashMap::new();
        for a in ActionId::ALL {
            action_ids.insert(action_def(*a).name, *a);
        }
        let mut g = Self {
            config,
            traffic,
            physics,
            city,
            props,
            destruction: DestructionSystem::new(),
            links: HashMap::new(),
            body_owner: HashMap::new(),
            static_bodies,
            input: InputMapper::default(),
            rumble: RumbleMixer::default(),
            last_rumble: Rumble::default(),
            spray: SprayParticles::new(4000),
            events: Vec::new(),
            models: HashMap::new(),
            action_ids,
            fixed_dt: 1.0 / 60.0,
            accumulator: 0.0,
            time: 0.0,
            last_gear: 0,
            frame_impact: 0.0,
            pending_hits: HashMap::new(),
        };
        g.sync_vehicles(1.0 / 60.0);
        g
    }

    pub fn player_id(&self) -> Option<VehicleId> {
        self.traffic.player_id()
    }

    pub fn player_vehicle(&self) -> Option<VehicleHandle> {
        let id = self.player_id()?;
        match &self.links.get(&id)?.state {
            CarState::Physics { vehicle, .. } => Some(*vehicle),
            _ => None,
        }
    }

    pub fn damage_of(&self, id: VehicleId) -> Option<&VehicleDamage> {
        match &self.links.get(&id)?.state {
            CarState::Physics { damage, .. } => Some(damage),
            _ => None,
        }
    }

    pub fn model(&mut self, class: PClass) -> &CarModel {
        self.models
            .entry(class)
            .or_insert_with(|| build_car(class, &class.desc()))
    }

    /// Advance by real time with a fixed step. Returns the number of steps taken.
    pub fn update(&mut self, real_dt: f64, pad: &GamepadState, kb: &mut KeyboardState) -> usize {
        self.accumulator += real_dt.min(0.25);
        let mut steps = 0;
        let speed = self
            .player_vehicle()
            .and_then(|v| self.physics.vehicle(v))
            .map_or(0.0, |v| v.telemetry.speed);
        let frame = self.input.update(real_dt as f32, pad, kb, speed);
        let mut first = true;
        while self.accumulator >= self.fixed_dt as f64 {
            self.accumulator -= self.fixed_dt as f64;
            let f = if first {
                frame.clone()
            } else {
                ControlFrame {
                    events: Vec::new(),
                    ..frame.clone()
                }
            };
            first = false;
            self.step(&f);
            steps += 1;
        }
        steps
    }

    /// Interpolation factor between the last two physics states.
    pub fn alpha(&self) -> f32 {
        (self.accumulator / self.fixed_dt as f64) as f32
    }

    /// One fixed step with the given controls.
    pub fn step(&mut self, frame: &ControlFrame) {
        let dt = self.fixed_dt;
        self.events.clear();
        self.frame_impact = 0.0;
        self.feed_input(frame);
        // Physics poses -> traffic.
        let mut poses = Vec::new();
        for (id, link) in &self.links {
            if let CarState::Physics { vehicle, .. } = &link.state {
                if let Some(v) = self.physics.vehicle(*vehicle) {
                    if let Some(b) = self.physics.body(v.chassis) {
                        let (vx, vy) = sim_velocity(b.lin_vel);
                        let (x, y) = to_sim(b.pose.p);
                        poses.push((
                            *id,
                            ExternalPose {
                                x: x as f32,
                                y: y as f32,
                                heading: heading_of(b.pose.q),
                                vx,
                                vy,
                                yaw_rate: b.ang_vel.y,
                            },
                        ));
                    }
                }
            }
        }
        for (id, p) in poses {
            self.traffic.set_external_pose(id, p);
        }
        self.traffic.step(dt as f64);
        for e in self.traffic.drain_events() {
            self.events.push(GameEvent::Traffic(e));
        }
        self.sync_vehicles(dt);
        self.apply_controls(frame);
        // Weather onto the physics side.
        let env = self.traffic.env();
        let (wet, rain) = (env.wetness as f32, env.rain as f32);
        for (_, link) in self.links.iter() {
            if let CarState::Physics { vehicle, .. } = &link.state {
                if let Some(v) = self.physics.vehicle_mut(*vehicle) {
                    v.wetness = wet;
                }
            }
        }
        if let Some(p) = self.physics.puddles.as_mut() {
            p.update(dt, rain * 12.0, 0.4 + (1.0 - wet) * 2.0);
        }
        self.physics.step(dt);
        self.handle_physics_events();
        self.update_spray(dt);
        self.update_rumble(dt, frame);
        self.time += dt as f64;
    }

    fn feed_input(&mut self, frame: &ControlFrame) {
        let inp = &mut self.traffic.input;
        inp.set(ActionId::Throttle, frame.throttle as f64);
        inp.set(ActionId::Brake, frame.brake as f64);
        inp.set(ActionId::SteerLeft, (-frame.steer).max(0.0) as f64);
        inp.set(ActionId::SteerRight, frame.steer.max(0.0) as f64);
        inp.set(ActionId::Handbrake, frame.handbrake as f64);
        inp.set(ActionId::Clutch, frame.clutch as f64);
        for a in gridlock_input::ACTIONS {
            let Some(&tid) = self.action_ids.get(a.id) else {
                continue;
            };
            if matches!(
                tid,
                ActionId::Throttle
                    | ActionId::Brake
                    | ActionId::SteerLeft
                    | ActionId::SteerRight
                    | ActionId::Handbrake
                    | ActionId::Clutch
            ) {
                continue;
            }
            match a.kind {
                gridlock_input::ActionKind::Hold => {
                    inp.set(tid, if frame.is_held(a.action) { 1.0 } else { 0.0 })
                }
                _ => {
                    if frame.events.contains(&ActionEvent::Pressed(a.action)) {
                        inp.press(tid);
                    }
                }
            }
        }
    }

    fn spawn_puppet(&mut self, id: VehicleId, class: PClass, pose: Pose, vel: Vec3) -> BodyHandle {
        let d = class.desc();
        let body = self.physics.add_body(BodyDesc {
            gravity_scale: 0.0,
            can_sleep: true,
            lin_vel: vel,
            ..BodyDesc::dynamic(pose)
        });
        let hull = Shape::convex(&d.hull)
            .unwrap_or_else(|| Shape::cuboid(Vec3::new(d.size.y, d.size.z, d.size.x) * 0.5));
        // Puppets follow the lane graph exactly; contacts with kerbs or buildings would only be
        // geometry mismatches, so they collide with cars, props and debris only.
        self.physics.add_collider(
            body,
            ColliderDesc::new(hull).friction(0.4).filter(
                layers::VEHICLE,
                layers::ALL & !layers::WHEEL_RAY & !layers::STATIC,
            ),
        );
        self.physics.set_mass_properties(
            body,
            gridlock_physics::MassProperties {
                mass: d.mass,
                com: d.com,
                inertia: glam::Mat3::from_diagonal(d.inertia),
            },
        );
        self.body_owner.insert(body, id);
        body
    }

    fn spawn_physics_car(
        &mut self,
        id: VehicleId,
        class: PClass,
        pose: Pose,
        lin: Vec3,
        ang: Vec3,
        player: bool,
    ) -> (VehicleHandle, Box<VehicleDamage>) {
        let desc = class.desc();
        let radius = desc.wheels[0].radius;
        let vh = self.physics.add_vehicle(desc, pose);
        let chassis = self.physics.vehicle(vh).unwrap().chassis;
        self.physics.set_velocity(chassis, lin, ang);
        let fwd = lin.dot(pose.q * -Vec3::Z);
        if let Some(v) = self.physics.vehicle_mut(vh) {
            for w in &mut v.wheels {
                w.omega = fwd / radius;
            }
            v.assists.steering = if player { 0.55 } else { 0.0 };
        }
        self.body_owner.insert(chassis, id);
        let model = self.model(class);
        let (body_mesh, parts) = model.damage_parts();
        let wheels = 4;
        (
            vh,
            Box::new(VehicleDamage::new(vh, body_mesh, parts, wheels)),
        )
    }

    fn remove_link(&mut self, id: VehicleId) {
        if let Some(link) = self.links.remove(&id) {
            match link.state {
                CarState::Lane { body } => {
                    self.body_owner.remove(&body);
                    self.physics.remove_body(body);
                }
                CarState::Physics {
                    vehicle,
                    mut damage,
                } => {
                    if let Some(v) = self.physics.vehicle(vehicle) {
                        self.body_owner.remove(&v.chassis);
                    }
                    damage.repair(&mut self.physics);
                    self.physics.remove_vehicle(vehicle);
                }
            }
        }
    }

    /// Create, convert and drive physics representations for every traffic vehicle.
    fn sync_vehicles(&mut self, dt: f32) {
        let player = self.traffic.player_id();
        let snapshot: Vec<(VehicleId, TClass, MotionMode, f64, f64, f64, f64)> = self
            .traffic
            .vehicles()
            .iter()
            .filter(|v| v.alive)
            .map(|v| (v.id, v.spec.id, v.mode, v.x, v.y, v.heading, v.speed))
            .collect();
        // Despawned.
        let alive: std::collections::HashSet<VehicleId> = snapshot.iter().map(|s| s.0).collect();
        let gone: Vec<VehicleId> = self
            .links
            .keys()
            .copied()
            .filter(|k| !alive.contains(k))
            .collect();
        for id in gone {
            self.remove_link(id);
        }
        for (id, tclass, mode, x, y, heading, speed) in snapshot {
            let class = pclass(tclass);
            let is_player = Some(id) == player;
            let want_physics = is_player || mode == MotionMode::Physics;
            let target = Pose::new(to_world(x, y, 0.0), rotation_for_heading(heading));
            let fwd = target.q * -Vec3::Z;
            match self
                .links
                .get(&id)
                .map(|l| matches!(l.state, CarState::Physics { .. }))
            {
                None => {
                    if want_physics {
                        let (vh, dmg) = self.spawn_physics_car(
                            id,
                            class,
                            target,
                            fwd * speed as f32,
                            Vec3::ZERO,
                            is_player,
                        );
                        self.links.insert(
                            id,
                            CarLink {
                                state: CarState::Physics {
                                    vehicle: vh,
                                    damage: dmg,
                                },
                                class,
                                last_pose: target,
                            },
                        );
                    } else {
                        let b = self.spawn_puppet(id, class, target, fwd * speed as f32);
                        self.links.insert(
                            id,
                            CarLink {
                                state: CarState::Lane { body: b },
                                class,
                                last_pose: target,
                            },
                        );
                    }
                    self.traffic.set_external(id, true);
                }
                Some(false) if want_physics => {
                    // Lane -> physics: keep the post-collision motion of the puppet.
                    let CarState::Lane { body } = self.links[&id].state else {
                        unreachable!()
                    };
                    let (pose, lin, ang) = self
                        .physics
                        .body(body)
                        .map(|b| (b.pose, b.lin_vel, b.ang_vel))
                        .unwrap_or((target, Vec3::ZERO, Vec3::ZERO));
                    self.body_owner.remove(&body);
                    self.physics.remove_body(body);
                    let upright =
                        Pose::new(Vec3::new(pose.p.x, pose.p.y.max(0.0), pose.p.z), pose.q);
                    let (vh, mut dmg) = self.spawn_physics_car(
                        id,
                        class,
                        upright,
                        Vec3::new(lin.x, 0.0, lin.z),
                        Vec3::new(0.0, ang.y, 0.0),
                        is_player,
                    );
                    // The hit that caused the switch happened to the puppet: replay it.
                    if let Some(hits) = self.pending_hits.remove(&id) {
                        let mut evs = Vec::new();
                        for (point, dir, impulse) in hits {
                            let excess = impulse - dmg.tuning.threshold;
                            if excess > 0.0 {
                                dmg.apply_hit(&mut self.physics, point, dir, excess, &mut evs);
                            }
                        }
                        for ev in evs {
                            self.events.push(GameEvent::Damage {
                                vehicle: id,
                                event: ev,
                            });
                        }
                    }
                    self.links.insert(
                        id,
                        CarLink {
                            state: CarState::Physics {
                                vehicle: vh,
                                damage: dmg,
                            },
                            class,
                            last_pose: pose,
                        },
                    );
                }
                Some(true) if !want_physics => {
                    // Recovered to lane following: back to a puppet at the lane pose.
                    self.remove_link(id);
                    let b = self.spawn_puppet(id, class, target, fwd * speed as f32);
                    self.links.insert(
                        id,
                        CarLink {
                            state: CarState::Lane { body: b },
                            class,
                            last_pose: target,
                        },
                    );
                }
                Some(false) => {
                    // Drive the puppet toward the lane pose.
                    let CarState::Lane { body } = self.links[&id].state else {
                        unreachable!()
                    };
                    if let Some(b) = self.physics.body(body) {
                        let dp = target.p - b.pose.p;
                        let dq = target.q * b.pose.q.conjugate();
                        let (axis, angle) = dq.to_axis_angle();
                        let angle = if angle > std::f32::consts::PI {
                            angle - std::f32::consts::TAU
                        } else {
                            angle
                        };
                        let lin = dp / dt;
                        let ang = axis * (angle / dt);
                        if dp.length_squared() > 1e-8
                            || angle.abs() > 1e-5
                            || b.lin_vel.length_squared() > 1e-6
                        {
                            self.physics.set_velocity(body, lin, ang);
                        }
                    }
                    if let Some(l) = self.links.get_mut(&id) {
                        l.last_pose = target;
                    }
                }
                Some(true) => {}
            }
        }
    }

    fn apply_controls(&mut self, frame: &ControlFrame) {
        let player = self.traffic.player_id();
        let ids: Vec<VehicleId> = self.links.keys().copied().collect();
        for id in ids {
            let CarState::Physics { vehicle, .. } = self.links[&id].state else {
                continue;
            };
            let is_player = Some(id) == player;
            let c = if is_player {
                self.traffic.player_controls()
            } else {
                self.traffic.ai_controls(id)
            };
            let Some(c) = c else {
                // Wrecked or driverless: hold the brakes.
                self.physics.set_vehicle_controls(
                    vehicle,
                    VehicleControls {
                        brake: 0.3,
                        ..Default::default()
                    },
                );
                continue;
            };
            let steer = if is_player { -frame.steer } else { c.steer };
            let controls = VehicleControls {
                throttle: c.throttle,
                brake: c.brake,
                steer,
                handbrake: c
                    .handbrake
                    .max(if is_player { frame.handbrake } else { 0.0 }),
                clutch: if is_player { frame.clutch } else { 0.0 },
            };
            let speed = self
                .physics
                .vehicle(vehicle)
                .map_or(0.0, |v| v.telemetry.speed);
            if let Some(v) = self.physics.vehicle_mut(vehicle) {
                let sel = match c.gear_dir {
                    d if d < 0 => Selector::Reverse,
                    0 if speed.abs() < 0.3 => Selector::Park,
                    0 => Selector::Neutral,
                    _ => Selector::Drive,
                };
                if v.powertrain.selector != sel {
                    v.set_selector(sel);
                }
            }
            self.physics.set_vehicle_controls(vehicle, controls);
        }
    }

    fn handle_physics_events(&mut self) {
        let events: Vec<PhysicsEvent> = self.physics.events().to_vec();
        self.pending_hits.clear();
        let mut reported: std::collections::HashSet<(Option<VehicleId>, Option<VehicleId>)> =
            Default::default();
        let player = self.traffic.player_id();
        for e in &events {
            if let PhysicsEvent::Impact {
                body_a,
                body_b,
                point,
                impulse,
                speed,
                ..
            } = *e
            {
                let a = self.body_owner.get(&body_a).copied();
                let b = self.body_owner.get(&body_b).copied();
                if a.is_none() && b.is_none() {
                    continue;
                }
                let static_hit = |h: BodyHandle| (h.0.index as usize) < self.static_bodies;
                let (x, y) = to_sim(point);
                match (a, b) {
                    (Some(a), Some(b)) => {
                        if reported.insert((Some(a.min(b)), Some(a.max(b)))) {
                            self.traffic.report_collision(
                                a,
                                Some(b),
                                impulse,
                                (x as f32, y as f32),
                            );
                        }
                    }
                    (Some(v), None) | (None, Some(v)) => {
                        let other = if a.is_some() { body_b } else { body_a };
                        let threshold = if static_hit(other) { 4.0 } else { 2.0 };
                        if speed > threshold && reported.insert((Some(v), None)) {
                            self.traffic
                                .report_collision(v, None, impulse, (x as f32, y as f32));
                        }
                    }
                    _ => {}
                }
                // Remember hits on puppets (their damage model only exists after conversion).
                if let PhysicsEvent::Impact { normal, .. } = *e {
                    for (body, owner, dir) in [(body_a, a, -normal), (body_b, b, normal)] {
                        if let Some(owner) = owner {
                            if self.links.get(&owner).is_some_and(
                                |l| matches!(l.state, CarState::Lane { body: pb } if pb == body),
                            ) {
                                self.pending_hits
                                    .entry(owner)
                                    .or_default()
                                    .push((point, dir, impulse));
                            }
                        }
                    }
                }
                if Some(a) == Some(player) || Some(b) == Some(player) {
                    self.frame_impact = self.frame_impact.max(impulse);
                }
                if speed > 2.0 {
                    self.events.push(GameEvent::Crash {
                        a,
                        b,
                        point,
                        impulse,
                        speed,
                    });
                }
            }
        }
        // Damage models.
        let ids: Vec<VehicleId> = self.links.keys().copied().collect();
        for id in ids {
            let link = self.links.get_mut(&id).unwrap();
            if let CarState::Physics { damage, .. } = &mut link.state {
                let mut evs = damage.process(&mut self.physics);
                evs.extend(damage.update(&self.physics));
                for ev in evs {
                    self.events.push(GameEvent::Damage {
                        vehicle: id,
                        event: ev,
                    });
                }
            }
        }
        for ev in self.props.update(&self.physics) {
            if let PropEvent::HydrantBurst { position, .. } = ev {
                self.spray.emit(position, Vec3::Y, 9.0, 0.25, 60, 0.06);
            }
            self.events.push(GameEvent::Prop(ev));
        }
        let pieces = self.destruction.update(&mut self.physics);
        if !pieces.is_empty() {
            let p = self
                .physics
                .body(pieces[0])
                .map_or(Vec3::ZERO, |b| b.pose.p);
            self.events.push(GameEvent::Shattered {
                point: p,
                pieces: pieces.len(),
            });
        }
    }

    fn update_spray(&mut self, dt: f32) {
        // Broken hydrants keep gushing.
        for p in &self.props.props {
            if p.broken && p.kind == gridlock_physics::props::PropKind::FireHydrant {
                self.spray
                    .emit(p.home.p + Vec3::Y * 0.4, Vec3::Y, 10.0, 0.18, 12, 0.07);
            }
        }
        // Tyre spray from wheels in standing water.
        let mut emits = Vec::new();
        for link in self.links.values() {
            if let CarState::Physics { vehicle, .. } = &link.state {
                if let Some(v) = self.physics.vehicle(*vehicle) {
                    let speed = v.telemetry.speed.abs();
                    for w in &v.wheels {
                        if w.in_contact && w.water_depth > 0.001 && speed > 4.0 {
                            let n = ((w.water_depth * 1000.0) * speed * 0.08) as usize;
                            emits.push((w.contact_point, speed, n.min(10)));
                        }
                    }
                }
            }
        }
        for (p, s, n) in emits {
            self.spray
                .emit(p + Vec3::Y * 0.05, Vec3::Y * 0.6, s * 0.25, 0.9, n, 0.05);
        }
        self.spray.update(dt, self.physics.gravity, |_| 0.0);
    }

    fn update_rumble(&mut self, dt: f32, frame: &ControlFrame) {
        let Some(vh) = self.player_vehicle() else {
            return;
        };
        let Some(v) = self.physics.vehicle(vh) else {
            return;
        };
        let gear = v.powertrain.gear;
        let shifted = gear != self.last_gear;
        self.last_gear = gear;
        let mut slide: f32 = 0.0;
        let mut spin: f32 = 0.0;
        let mut rough: f32 = 0.0;
        let mut jolt: f32 = 0.0;
        let mut locked = false;
        for (w, d) in v.wheels.iter().zip(&v.desc.wheels) {
            if !w.in_contact {
                continue;
            }
            slide = slide.max(w.slide_speed);
            if d.driven {
                spin = spin.max(w.slip_ratio);
            }
            rough = rough.max(match w.surface {
                gridlock_physics::Surface::Cobblestone => 0.35,
                gridlock_physics::Surface::Gravel => 0.6,
                gridlock_physics::Surface::Grass | gridlock_physics::Surface::Dirt => 0.5,
                _ => 0.05,
            });
            jolt = jolt.max(w.compression_velocity.abs());
            locked |= w.omega.abs() < 0.5 && v.telemetry.speed.abs() > 3.0;
        }
        let inputs = RumbleInputs {
            speed: v.telemetry.speed,
            rpm_fraction: v.powertrain.rpm() / v.desc.engine.redline,
            limiter: v.powertrain.limiter,
            engine_running: v.powertrain.running,
            slide_speed: slide,
            wheelspin: spin,
            abs_active: v.telemetry.abs_active,
            tcs_active: v.telemetry.tcs_active,
            wheel_locked: locked,
            roughness: rough,
            suspension_jolt: jolt,
            impact: self.frame_impact,
            mass: v.desc.mass,
            shifted,
            throttle: frame.throttle,
            brake: frame.brake,
        };
        self.last_rumble = self.rumble.update(dt, &inputs);
    }
}
