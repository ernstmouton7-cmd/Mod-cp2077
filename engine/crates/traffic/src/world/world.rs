//! The whole simulation: construction, the fixed step and host queries.

use super::environment::{Environment, Weather};
use super::events::{Event, GestureKind, IncidentKind, PlayerStats};
use super::sim::{Sim, WorldStats};
use super::spawner::{PopulationOptions, Spawner};
use crate::ai::driver::{AIDriver, DriverState, HoldKind};
use crate::ai::emotion::OffenseKind;
use crate::ai::pedestrians::{PedHost, PedState, Pedestrian, PedestrianSystem};
use crate::ai::personality::Archetype;
use crate::ai::speech::{APOLOGY_ACCEPTED, APOLOGY_REJECTED};
use crate::ids::{ConnectorId, LaneId, NodeId, SegRef, SignalId, VehicleId};
use crate::law::player_monitor::{HitRun, Observation, PlayerMonitor};
use crate::law::police::PoliceSystem;
use crate::law::violations::{DrivingRecord, ViolationKind, Witness};
use crate::math::scalar::{angle_diff, clamp, fmod, hypot, js_round, DEG};
use crate::math::Rng;
use crate::physics::{box_contact, resolve_bodies, resolve_static, BoxShape, Contact};
use crate::player::actions::{ActionId, InputMapper};
use crate::player::player_driver::PlayerDriver;
use crate::road::{
    build_city, build_crosswalks, Building, BuildingKind, CityOptions, Network, RoadKind, Router,
    SpeedCamera, TurnType,
};
use crate::signals::{SignalController, SignalMode, SignalOptions};
use crate::util::SpatialHash;
use crate::vehicle::{spec, Indicator, MotionMode, SirenMode, Vehicle, VehicleClass, VehicleRole};
use std::collections::HashMap;

/// World construction options.
#[derive(Clone, Copy, Debug)]
pub struct WorldOptions {
    pub seed: i64,
    /// City generation (its seed defaults to the world seed).
    pub city: CityOptions,
    /// Civilian vehicles at peak density.
    pub traffic: usize,
    pub police: usize,
    pub start_hour: f64,
    pub player_vehicle: VehicleClass,
    /// Fixed simulation step (s) used by [`World::update`].
    pub fixed_dt: f64,
    /// Disable the player entirely (pure traffic simulation).
    pub no_player: bool,
    /// Every vehicle is host-simulated (`Vehicle::external`) and the World
    /// never runs its own 2D collision detection.
    pub external_physics: bool,
}

impl Default for WorldOptions {
    fn default() -> Self {
        WorldOptions {
            seed: 2077,
            city: CityOptions::default(),
            traffic: 110,
            police: 4,
            start_hour: 19.5,
            player_vehicle: VehicleClass::Sports,
            fixed_dt: 1.0 / 60.0,
            no_player: false,
            external_physics: false,
        }
    }
}

/// Pose of a host-simulated vehicle (metres, radians CCW from +x, m/s, rad/s).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ExternalPose {
    pub x: f32,
    pub y: f32,
    pub heading: f32,
    pub vx: f32,
    pub vy: f32,
    pub yaw_rate: f32,
}

/// Driving controls the World's driver (AI or player) wants applied to a
/// host-simulated car.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DriveControls {
    /// Normalised steering, -1 (full right) .. +1 (full left) of the spec's max lock.
    pub steer: f32,
    /// Target front-wheel angle (rad, + = left).
    pub steer_angle: f32,
    /// 0..1
    pub throttle: f32,
    /// 0..1
    pub brake: f32,
    /// 0..1
    pub handbrake: f32,
    /// -1 reverse, 0 neutral, +1 forward (the gear being selected).
    pub gear_dir: i8,
}

struct WorldPedHost<'a> {
    net: &'a Network,
    signals: &'a [SignalController],
    vehicles: &'a [Vehicle],
    events: &'a mut super::events::EventQueue,
    rng: &'a mut Rng,
}

const PED_SHOUTS: [&str; 6] = [
    "Hey! I'm walking here!",
    "Watch it!",
    "It's a crosswalk!",
    "Slow down!",
    "Are you blind?!",
    "Jeez!",
];

impl PedHost for WorldPedHost<'_> {
    fn net(&self) -> &Network {
        self.net
    }
    fn signals(&self) -> &[SignalController] {
        self.signals
    }
    fn traffic_approaching(&self, x: f64, y: f64, radius: f64, seconds: f64) -> bool {
        traffic_approaching(self.vehicles, x, y, radius, seconds)
    }
    fn vehicle_at(&self, x: f64, y: f64, margin: f64) -> bool {
        self.vehicles.iter().any(|v| {
            let dx = x - v.x;
            let dy = y - v.y;
            if dx.abs() > v.half_len + margin + 1.0 || dy.abs() > v.half_len + margin + 1.0 {
                return false;
            }
            let c = v.heading.cos();
            let s = v.heading.sin();
            let lx = dx * c + dy * s;
            let ly = -dx * s + dy * c;
            lx.abs() < v.half_len + margin && ly.abs() < v.half_wid + margin
        })
    }
    fn on_shout(&mut self, p: &Pedestrian) {
        let text = self.rng.pick(&PED_SHOUTS);
        self.events.emit(Event::PedSpeech {
            ped: p.id,
            text,
            x: p.x,
            y: p.y,
        });
    }
}

fn traffic_approaching(vehicles: &[Vehicle], x: f64, y: f64, radius: f64, seconds: f64) -> bool {
    for v in vehicles {
        let dx = x - v.x;
        let dy = y - v.y;
        let d = (dx * dx + dy * dy).sqrt();
        if d > radius + v.abs_speed() * seconds + 3.0 {
            continue;
        }
        if d < radius + 2.0 {
            return true;
        }
        let toward = (dx * v.heading.cos() + dy * v.heading.sin()) / d.max(0.1);
        if toward > 0.6 && v.abs_speed() > 1.0 {
            return true;
        }
    }
    false
}

fn set_box(v: &Vehicle, shrink: f64) -> BoxShape {
    BoxShape {
        x: v.x,
        y: v.y,
        heading: v.heading,
        hl: v.half_len * shrink,
        hw: v.half_wid * shrink,
    }
}

fn lane_pair_can_collide(net: &Network, a: &Vehicle, b: &Vehicle) -> bool {
    let (Some(sa), Some(sb)) = (a.seg, b.seg) else {
        return false;
    };
    if sa == sb || a.prev_seg == Some(sb) || b.prev_seg == Some(sa) {
        return angle_diff(a.heading, b.heading).abs() < 35.0 * DEG;
    }
    if let (SegRef::Connector(ca), SegRef::Connector(cb)) = (sa, sb) {
        return net.connector(ca).conflicts.iter().any(|z| z.other == cb);
    }
    false
}

/// The whole simulation. Hosts call [`World::update`] every frame (it runs
/// fixed steps) or [`World::step`] directly, feed input through `input`,
/// read vehicles, signals and HUD state, and drain events with
/// [`World::drain_events`].
pub struct World {
    /// Shared simulation state (network, signals, vehicles, events...).
    pub sim: Sim,
    pub buildings: Vec<Building>,
    pub speed_cameras: Vec<SpeedCamera>,
    pub grid_xs: Vec<f64>,
    pub grid_ys: Vec<f64>,
    pub city_name: String,
    pub player_driver: Option<PlayerDriver>,
    pub input: InputMapper,
    pub monitor: PlayerMonitor,
    pub record: DrivingRecord,
    pub police: PoliceSystem,
    pub spawner: Spawner,
    pub fixed_dt: f64,
    /// Simulation speed multiplier for `update` (pause = 0).
    pub time_scale: f64,
    /// Host-side actions pressed since the host last drained them.
    pub host_actions: Vec<ActionId>,
    /// Base pedestrian count at daytime peak.
    pub pedestrian_density: f64,
    /// Seconds between random breakdowns / power cuts (0 disables).
    pub breakdown_interval: f64,
    pub outage_interval: f64,
    /// All vehicles are host-simulated; no internal 2D collision detection.
    pub external_physics: bool,
    accumulator: f64,
    vehicle_hash: SpatialHash<VehicleId>,
    flash_memo: HashMap<VehicleId, f64>,
    ambulance: Option<VehicleId>,
    ambulance_target: Option<(f64, f64, f64)>,
    player_tailgate: HashMap<VehicleId, f64>,
    near_miss_memo: HashMap<VehicleId, f64>,
    night_flashing: bool,
    cleanup_timer: f64,
    breakdown_timer: f64,
    outage_timer: f64,
    outages: Vec<(SignalId, f64)>,
    observations: Vec<Observation>,
}

impl World {
    pub fn new(opts: WorldOptions) -> Self {
        let seed = opts.seed;
        let mut rng = Rng::new(seed);
        let mut city_opts = opts.city;
        city_opts.seed = Some(city_opts.seed.unwrap_or(seed));
        let city = build_city(city_opts);
        let mut net = city.network;
        let router = Router::new(&net);
        let env = Environment::new(seed + 1, opts.start_hour);
        // Signals.
        let mut signals = Vec::new();
        for n in 0..net.nodes.len() {
            let node = &net.nodes[n];
            if node.control != crate::road::ControlType::Signal {
                continue;
            }
            // Coordinate along the x axis for a green wave.
            let offset = ((node.x - net.min_x) / 13.9).max(0.0);
            let mode = if rng.chance(0.75) {
                SignalMode::Actuated
            } else {
                SignalMode::Fixed
            };
            let offset = if mode == SignalMode::Fixed {
                offset
            } else {
                rng.range(0.0, 30.0)
            };
            let allow_right_on_red = rng.chance(0.85);
            let sid = SignalId::from(signals.len());
            let sig = SignalController::new(
                sid,
                &mut net,
                NodeId::from(n),
                SignalOptions {
                    mode,
                    offset,
                    allow_right_on_red,
                    cycle_scale: 1.0,
                },
            );
            net.nodes[n].signal = Some(sid);
            signals.push(sig);
        }
        // Obstacles: buildings and boundary walls.
        let mut obstacles = Vec::new();
        for b in &city.buildings {
            if b.kind == BuildingKind::Park || b.kind == BuildingKind::Plaza {
                continue;
            }
            obstacles.push(BoxShape {
                x: b.x,
                y: b.y,
                heading: 0.0,
                hl: b.w / 2.0,
                hw: b.h / 2.0,
            });
        }
        let pad = 16.0;
        let (x0, x1, y0, y1) = (
            net.min_x - pad,
            net.max_x + pad,
            net.min_y - pad,
            net.max_y + pad,
        );
        let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
        let t = 3.0;
        obstacles.push(BoxShape {
            x: cx,
            y: y0 - t,
            heading: 0.0,
            hl: (x1 - x0) / 2.0 + t * 2.0,
            hw: t,
        });
        obstacles.push(BoxShape {
            x: cx,
            y: y1 + t,
            heading: 0.0,
            hl: (x1 - x0) / 2.0 + t * 2.0,
            hw: t,
        });
        obstacles.push(BoxShape {
            x: x0 - t,
            y: cy,
            heading: 0.0,
            hl: t,
            hw: (y1 - y0) / 2.0 + t * 2.0,
        });
        obstacles.push(BoxShape {
            x: x1 + t,
            y: cy,
            heading: 0.0,
            hl: t,
            hw: (y1 - y0) / 2.0 + t * 2.0,
        });
        let mut obstacle_hash = SpatialHash::new(30.0);
        for (i, o) in obstacles.iter().enumerate() {
            obstacle_hash.insert(i as u32, o.x, o.y, o.hl.max(o.hw));
        }
        let (crosswalks, crossings) = build_crosswalks(&net);
        let peds = PedestrianSystem::new(crosswalks, rng.fork());
        let mut sim = Sim::new(
            net,
            router,
            rng,
            env,
            peds,
            crossings,
            obstacles,
            obstacle_hash,
        );
        sim.signals = signals;
        let mut spawner = Spawner::new(PopulationOptions {
            civilians: opts.traffic,
            police: opts.police,
            min_distance: 90.0,
        });
        spawner.external = opts.external_physics;
        let mut w = World {
            sim,
            buildings: city.buildings,
            speed_cameras: city.speed_cameras,
            grid_xs: city.xs,
            grid_ys: city.ys,
            city_name: city.name,
            player_driver: None,
            input: InputMapper::new(),
            monitor: PlayerMonitor::new(),
            record: DrivingRecord::new(),
            police: PoliceSystem::new(),
            spawner,
            fixed_dt: opts.fixed_dt,
            time_scale: 1.0,
            host_actions: Vec::new(),
            pedestrian_density: 45.0,
            breakdown_interval: 80.0,
            outage_interval: 300.0,
            external_physics: opts.external_physics,
            accumulator: 0.0,
            vehicle_hash: SpatialHash::new(12.0),
            flash_memo: HashMap::new(),
            ambulance: None,
            ambulance_target: None,
            player_tailgate: HashMap::new(),
            near_miss_memo: HashMap::new(),
            night_flashing: false,
            cleanup_timer: 0.0,
            breakdown_timer: 45.0,
            outage_timer: 150.0,
            outages: Vec::new(),
            observations: Vec::new(),
        };
        if !opts.no_player {
            w.spawn_player(opts.player_vehicle);
        }
        let density = w.sim.env.traffic_density;
        w.spawner.populate(&mut w.sim, density);
        w.sim.rebuild_occupancy();
        w
    }

    // ----------------------------------------------------------- setup

    fn spawn_player(&mut self, cls: VehicleClass) {
        let net = &self.sim.net;
        let mut best = None;
        let mut bd = f64::INFINITY;
        for l in &net.lanes {
            if l.length < 60.0 || l.right.is_some() {
                continue;
            }
            let pt = l.path.sample(l.length * 0.3);
            let d = pt.x * pt.x + pt.y * pt.y;
            if d < bd {
                bd = d;
                best = Some(l.id);
            }
        }
        let lane = net.lane(best.unwrap_or(LaneId(0)));
        let pt = lane.path.sample(lane.length * 0.3);
        let id = self.sim.next_vehicle_id();
        let mut v = Vehicle::new(id, spec(cls), 0xff2a6d, VehicleRole::Player);
        v.x = pt.x;
        v.y = pt.y;
        v.heading = pt.heading;
        v.dynamics.reset(pt.x, pt.y, pt.heading, 0.0);
        v.mode = MotionMode::Physics;
        v.lights.headlight_switch = crate::vehicle::HeadlightSwitch::Auto;
        v.external = self.external_physics;
        self.sim.player = Some(id);
        self.sim.events.player = Some(id);
        self.player_driver = Some(PlayerDriver::new());
        self.sim.insert_vehicle(v);
    }

    /// Swap the player's car for another class (keeps position).
    pub fn set_player_vehicle(&mut self, cls: VehicleClass) {
        let Some(old) = self.sim.player.and_then(|p| self.sim.vehicle(p)) else {
            return;
        };
        let (ox, oy, oh, color, hl, ext) = (
            old.x,
            old.y,
            old.heading,
            old.color,
            old.lights.headlight_switch,
            old.external,
        );
        let old_id = old.id;
        let id = self.sim.next_vehicle_id();
        let mut v = Vehicle::new(id, spec(cls), color, VehicleRole::Player);
        v.dynamics.reset(ox, oy, oh, 0.0);
        v.x = ox;
        v.y = oy;
        v.heading = oh;
        v.mode = MotionMode::Physics;
        v.lights.headlight_switch = hl;
        v.external = ext;
        self.sim.remove_vehicle(old_id);
        self.sim.player = Some(id);
        self.sim.events.player = Some(id);
        let mut pd = PlayerDriver::new();
        if let Some(old_pd) = &self.player_driver {
            pd.seatbelt = old_pd.seatbelt;
            pd.settings.clutch_assist = old_pd.settings.clutch_assist;
        }
        self.player_driver = Some(pd);
        self.sim.insert_vehicle(v);
    }

    /// Spawn an AI vehicle on a lane (scripted scenarios, tests).
    #[allow(clippy::too_many_arguments)]
    pub fn spawn_vehicle(
        &mut self,
        cls: VehicleClass,
        role: VehicleRole,
        lane: LaneId,
        s: f64,
        speed: f64,
        archetype: Option<Archetype>,
    ) -> VehicleId {
        self.spawner
            .create(&mut self.sim, cls, role, lane, s, speed, archetype)
    }

    /// Remove a vehicle.
    pub fn remove_vehicle(&mut self, id: VehicleId) {
        self.sim.remove_vehicle(id);
    }

    // ----------------------------------------------------------- queries

    pub fn time(&self) -> f64 {
        self.sim.time
    }
    pub fn net(&self) -> &Network {
        &self.sim.net
    }
    pub fn net_mut(&mut self) -> &mut Network {
        &mut self.sim.net
    }
    pub fn vehicles(&self) -> &[Vehicle] {
        &self.sim.vehicles
    }
    pub fn vehicle(&self, id: VehicleId) -> Option<&Vehicle> {
        self.sim.vehicle(id)
    }
    pub fn vehicle_mut(&mut self, id: VehicleId) -> Option<&mut Vehicle> {
        self.sim.vehicle_mut(id)
    }
    pub fn driver_of(&self, id: VehicleId) -> Option<&AIDriver> {
        self.sim.driver(id)
    }
    pub fn driver_of_mut(&mut self, id: VehicleId) -> Option<&mut AIDriver> {
        self.sim.driver_mut(id)
    }
    pub fn player(&self) -> Option<&Vehicle> {
        self.sim.player.and_then(|p| self.sim.vehicle(p))
    }
    pub fn player_mut(&mut self) -> Option<&mut Vehicle> {
        let p = self.sim.player?;
        self.sim.vehicle_mut(p)
    }
    pub fn player_id(&self) -> Option<VehicleId> {
        self.sim.player
    }
    pub fn signals(&self) -> &[SignalController] {
        &self.sim.signals
    }
    pub fn signals_mut(&mut self) -> &mut [SignalController] {
        &mut self.sim.signals
    }
    pub fn env(&self) -> &Environment {
        &self.sim.env
    }
    pub fn env_mut(&mut self) -> &mut Environment {
        &mut self.sim.env
    }
    pub fn peds(&self) -> &PedestrianSystem {
        &self.sim.peds
    }
    pub fn stats(&self) -> &WorldStats {
        &self.sim.stats
    }
    pub fn player_stats(&self) -> &PlayerStats {
        &self.sim.events.player_stats
    }

    /// Returns and clears the events emitted since the last drain.
    pub fn drain_events(&mut self) -> Vec<Event> {
        self.sim.events.drain()
    }

    /// Signal controller at a node.
    pub fn signal_at(&self, node: NodeId) -> Option<&SignalController> {
        self.sim
            .net
            .node(node)
            .signal
            .map(|s| &self.sim.signals[s.idx()])
    }

    /// Is walking allowed across a crosswalk right now?
    pub fn walk_allowed(&self, cw: crate::ids::CrosswalkId, reckless: bool) -> bool {
        self.sim
            .peds
            .walk_allowed(&self.sim.net, &self.sim.signals, cw, reckless)
    }

    /// Vehicles within `r` of a point (renderer LOD).
    pub fn vehicles_near(&self, x: f64, y: f64, r: f64) -> Vec<VehicleId> {
        self.sim
            .vehicles
            .iter()
            .filter(|v| (v.x - x) * (v.x - x) + (v.y - y) * (v.y - y) <= r * r)
            .map(|v| v.id)
            .collect()
    }

    /// Drain host actions (camera, pause...).
    pub fn drain_host_actions(&mut self) -> Vec<ActionId> {
        std::mem::take(&mut self.host_actions)
    }

    /// Drivers angry at the player within 120 m: (vehicle, anger, raging at player), angriest first.
    pub fn angry_at_player(&self) -> Vec<(VehicleId, f64, bool)> {
        let Some(p) = self.player() else {
            return Vec::new();
        };
        let mut out: Vec<(VehicleId, f64, bool)> = self
            .sim
            .vehicles
            .iter()
            .filter_map(|v| {
                let d = v.driver()?;
                let g = d.emo.grudge_against(p.id);
                (g > 10.0 && hypot(v.x - p.x, v.y - p.y) < 120.0)
                    .then(|| (v.id, d.emo.anger, d.rage.is_some_and(|r| r.target == p.id)))
            })
            .collect();
        out.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.0.cmp(&b.0))
        });
        out
    }

    /// Heading alignment helper (police HUD).
    pub fn is_angle_aligned(a: &Vehicle, b: &Vehicle) -> bool {
        angle_diff(a.heading, b.heading).abs() < 30.0 * DEG
    }

    /// Hash of the simulation state (determinism checks).
    pub fn state_hash(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mut mix = |x: u64| {
            h ^= x;
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        };
        mix(self.sim.time.to_bits());
        mix(self.sim.events.emitted);
        for v in &self.sim.vehicles {
            mix(v.id.0 as u64);
            mix(v.x.to_bits());
            mix(v.y.to_bits());
            mix(v.heading.to_bits());
            mix(v.speed.to_bits());
            mix(v.odometer.to_bits());
            if let Some(d) = v.driver() {
                mix(d.emo.anger.to_bits());
            }
        }
        for s in &self.sim.signals {
            mix(s.current_phase_index() as u64);
            mix(s.state_time().to_bits());
        }
        for p in &self.sim.peds.peds {
            mix(p.id.0 as u64);
            mix(p.x.to_bits());
            mix(p.y.to_bits());
        }
        mix(self.record.money.to_bits());
        h
    }

    // ----------------------------------------------------------- main loop

    /// Advance by real time; returns the number of fixed steps taken.
    pub fn update(&mut self, real_dt: f64) -> usize {
        self.accumulator += real_dt.min(0.25) * self.time_scale;
        let mut steps = 0;
        while self.accumulator >= self.fixed_dt && steps < 8 {
            self.step(self.fixed_dt);
            self.accumulator -= self.fixed_dt;
            steps += 1;
        }
        if steps == 8 {
            self.accumulator = 0.0;
        }
        steps
    }

    /// Interpolation alpha between the last two steps.
    pub fn alpha(&self) -> f64 {
        self.accumulator / self.fixed_dt
    }

    /// One fixed simulation step.
    pub fn step(&mut self, dt: f64) {
        self.sim.time += dt;
        self.sim.env.update(dt);
        self.update_signals(dt);
        self.sim.rebuild_occupancy();
        self.update_player(dt);
        // AI.
        let mut i = 0;
        while i < self.sim.vehicles.len() {
            if let Some(mut d) = self.sim.vehicles[i].driver.take() {
                d.update(dt, &mut self.sim, i);
                let changed = d.emo.mood != d.last_mood;
                let (mood, target) = (d.emo.mood, d.emo.target);
                d.last_mood = mood;
                let vid = self.sim.vehicles[i].id;
                self.sim.vehicles[i].driver = Some(d);
                self.sim.flush_deferred();
                if changed {
                    self.sim.emit(Event::Mood {
                        vehicle: vid,
                        mood,
                        target,
                    });
                }
            }
            i += 1;
        }
        self.update_pedestrians(dt);
        self.update_incidents(dt);
        self.physics(dt);
        self.collisions();
        self.law_and_order(dt);
        self.ambulances(dt);
        let busy: Vec<VehicleId> = self
            .police
            .units
            .iter()
            .copied()
            .chain(self.ambulance)
            .collect();
        let density = self.sim.env.traffic_density;
        self.spawner.update(dt, density, &mut self.sim, &busy);
        // Stale all-way stop arrivals (every 2 s).
        self.cleanup_timer -= dt;
        if self.cleanup_timer > 0.0 {
            return;
        }
        self.cleanup_timer = 2.0;
        let now = self.sim.time;
        let Sim {
            all_way, vehicles, ..
        } = &mut self.sim;
        for q in all_way.values_mut() {
            q.retain(|id, t| {
                now - *t <= 60.0 && vehicles.binary_search_by_key(id, |v| v.id).is_ok()
            });
        }
    }

    fn update_signals(&mut self, dt: f64) {
        let flash = self.sim.env.night_flash;
        if flash != self.night_flashing {
            self.night_flashing = flash;
            for i in 0..self.sim.signals.len() {
                let node = self.sim.signals[i].node;
                let minor = self.sim.net.node(node).roads.iter().all(|&r| {
                    let k = self.sim.net.road(r).kind;
                    k != RoadKind::Avenue && k != RoadKind::Ring
                });
                if minor {
                    self.sim.signals[i].set_mode(if flash {
                        SignalMode::Flashing
                    } else {
                        SignalMode::Actuated
                    });
                }
            }
        }
        let mut signals = std::mem::take(&mut self.sim.signals);
        let mut changes = Vec::new();
        {
            let sim = &self.sim;
            let demand = |c: ConnectorId| -> f64 {
                let conn = sim.net.connector(c);
                let lane = sim.net.lane(conn.from);
                let mut n = 0.0;
                for e in sim.occ.lane(conn.from) {
                    if e.s < lane.length - 60.0 {
                        continue;
                    }
                    let intent = sim.intent_of(e);
                    if intent == Some(c) || (e.phantom && intent.is_none()) {
                        n += 1.0;
                    }
                }
                n + sim.occ.connector(c).iter().filter(|e| e.v < 2.0).count() as f64
            };
            for s in signals.iter_mut() {
                if let Some(ch) = s.update(dt, Some(&demand)) {
                    changes.push(ch);
                }
            }
        }
        self.sim.signals = signals;
        for ch in changes {
            self.sim.emit(Event::Signal {
                node: ch.node,
                phase: ch.phase,
                state: ch.state,
            });
        }
    }

    fn update_player(&mut self, dt: f64) {
        let (Some(pi), Some(pd)) = (self.sim.player_idx(), self.player_driver.as_mut()) else {
            return;
        };
        let u = self.sim.vehicles[pi].dynamics.u;
        let frame = self.input.frame(dt, u);
        let dark = self.sim.env.dark;
        let Sim {
            vehicles, events, ..
        } = &mut self.sim;
        pd.update(
            dt,
            &frame,
            &mut vehicles[pi],
            events,
            dark,
            self.monitor.info.speed_limit,
        );
        self.host_actions.append(&mut pd.host_actions);
        let seatbelt = pd.seatbelt;
        let gesture = pd.gesture;
        if frame.pressed.contains(&ActionId::ResetVehicle) {
            self.reset_player();
        }
        if frame.pressed.contains(&ActionId::TimeForward) {
            self.sim.env.hour = fmod(self.sim.env.hour + 1.0, 24.0);
        }
        if frame.pressed.contains(&ActionId::WeatherCycle) {
            let order = [
                Weather::Clear,
                Weather::Rain,
                Weather::Storm,
                Weather::Fog,
                Weather::Cloudy,
            ];
            let i = order
                .iter()
                .position(|&w| w == self.sim.env.weather)
                .unwrap_or(0);
            self.sim.env.set_weather(order[(i + 1) % order.len()]);
        }
        self.monitor.seatbelt = seatbelt;
        let Some(pi) = self.sim.player_idx() else {
            return;
        };
        // Intent for AI yielding: from the indicator and current lane.
        let mut intent = None;
        if let Some(lane) = self.monitor.info.lane {
            let want = match self.sim.vehicles[pi].lights.indicator {
                Indicator::Left => TurnType::Left,
                Indicator::Right => TurnType::Right,
                Indicator::Off => TurnType::Straight,
            };
            for &c in &self.sim.net.lane(lane).outgoing {
                if self.sim.net.connector(c).turn == want {
                    intent = Some(c);
                }
            }
        }
        self.sim.vehicles[pi].intent_connector = intent;
        let pid = self.sim.vehicles[pi].id;
        if let Some(g) = gesture {
            self.gesture(pid, g);
        }
        if frame.pressed.contains(&ActionId::Hazards)
            && self.sim.vehicle(pid).is_some_and(|p| p.lights.hazard)
        {
            self.thank_you_blink(pid);
        }
        if frame.pressed.contains(&ActionId::Horn) {
            self.sim.stats.horns += 1;
            self.sim.horn_victims(pid, false);
        }
        for &(id, held) in &frame.released {
            if id == ActionId::Horn && held > 0.9 {
                self.sim.horn_victims(pid, true);
            }
        }
        if frame.pressed.contains(&ActionId::Flash) {
            self.flash_ahead(pid);
        }
    }

    fn gesture(&mut self, pid: VehicleId, kind: GestureKind) {
        self.sim.emit(Event::Gesture { from: pid, kind });
        let Some(p) = self.sim.vehicle(pid) else {
            return;
        };
        let (px, py) = (p.x, p.y);
        let mut i = 0;
        while i < self.sim.vehicles.len() {
            let v = &self.sim.vehicles[i];
            let vid = v.id;
            let dist = hypot(v.x - px, v.y - py);
            let Some(d) = v.driver() else {
                i += 1;
                continue;
            };
            let zen = d.p.archetype == Archetype::Zen;
            let grudge = d.emo.grudge_against(pid);
            if kind == GestureKind::Wave && dist < 45.0 {
                if grudge > 0.0 {
                    let now = self.sim.time;
                    let Sim {
                        vehicles,
                        events,
                        rng,
                        ..
                    } = &mut self.sim;
                    let d = vehicles[i].driver_mut().expect("driver");
                    let calmed = d.emo.apologize(pid, now);
                    events.emit(Event::Apology {
                        from: pid,
                        to: vid,
                        calmed,
                    });
                    d.speech_cooldown = 0.0;
                    let line = if d.emo.anger < 35.0 {
                        rng.pick(&APOLOGY_ACCEPTED)
                    } else {
                        rng.pick(&APOLOGY_REJECTED)
                    };
                    d.say(events, line);
                }
            } else if kind == GestureKind::Rude && dist < 30.0 {
                if zen {
                    let Sim {
                        vehicles, events, ..
                    } = &mut self.sim;
                    let d = vehicles[i].driver_mut().expect("driver");
                    d.speech_cooldown = 0.0;
                    d.say(events, "Have a nice day!");
                } else {
                    self.sim.offend(pid, vid, OffenseKind::RudeGesture, 1.0);
                }
            }
            i += 1;
        }
    }

    /// Hazards blink after merging: a softer apology to drivers behind.
    fn thank_you_blink(&mut self, pid: VehicleId) {
        let Some(p) = self.sim.vehicle(pid) else {
            return;
        };
        let (px, py, ph) = (p.x, p.y, p.heading);
        let now = self.sim.time;
        let Sim {
            vehicles, events, ..
        } = &mut self.sim;
        for v in vehicles.iter_mut() {
            let (dx, dy, vid) = (v.x - px, v.y - py, v.id);
            let Some(d) = v.driver_mut() else { continue };
            if d.emo.grudge_against(pid) <= 0.0 {
                continue;
            }
            let behind = dx * ph.cos() + dy * ph.sin() < 0.0;
            if !behind || dx * dx + dy * dy > 50.0 * 50.0 {
                continue;
            }
            let calmed = d.emo.apologize(pid, now) * 0.6;
            events.emit(Event::Apology {
                from: pid,
                to: vid,
                calmed,
            });
        }
    }

    fn flash_ahead(&mut self, pid: VehicleId) {
        let Some(p) = self.sim.vehicle(pid) else {
            return;
        };
        let (px, py, ph) = (p.x, p.y, p.heading);
        let now = self.sim.time;
        let targets: Vec<VehicleId> = self
            .sim
            .vehicles
            .iter()
            .filter(|v| {
                if v.id == pid {
                    return false;
                }
                let dx = v.x - px;
                let dy = v.y - py;
                let fwd = dx * ph.cos() + dy * ph.sin();
                if !(0.0..=45.0).contains(&fwd) {
                    return false;
                }
                let side = (-dx * ph.sin() + dy * ph.cos()).abs();
                side <= 3.0 + fwd * 0.15
            })
            .map(|v| v.id)
            .collect();
        for vid in targets {
            let last = self.flash_memo.get(&vid).copied().unwrap_or(-10.0);
            if now - last < 1.5 {
                continue;
            }
            self.flash_memo.insert(vid, now);
            self.sim.offend(pid, vid, OffenseKind::HighBeams, 1.0);
        }
    }

    fn physics(&mut self, dt: f64) {
        let grip = self.sim.env.grip;
        let on_road_player = self.monitor.info.on_road;
        for v in self.sim.vehicles.iter_mut() {
            if v.mode != MotionMode::Physics {
                continue;
            }
            if v.wrecked {
                v.pt.running = false;
                v.inputs.throttle = 0.0;
            }
            let dynm = &mut v.dynamics;
            let cut = (if dynm.tcs_active { 0.4 } else { 1.0 })
                * (if dynm.esc_active { 0.6 } else { 1.0 });
            let torque =
                v.pt.update(dt, dynm.driven_omega(), v.inputs.throttle, dynm.u, cut);
            if !v.external {
                let on_road = if v.is_player { on_road_player } else { true };
                dynm.surface_grip = grip * if on_road { 1.0 } else { 0.82 };
                dynm.surface_drag = if on_road { 0.0 } else { 1.5 };
                dynm.step(dt, &v.inputs, torque, v.steer_target, &v.assists);
                v.sync_from_dynamics();
            }
            let dynm = &v.dynamics;
            v.wheel_spin += dynm.driven_omega() * dt;
            v.odometer += dynm.abs_speed() * dt;
            if v.is_player {
                let sp = dynm.abs_speed();
                let st = &mut self.sim.events.player_stats;
                st.distance += sp * dt;
                if sp > 0.5 {
                    st.drive_time += dt;
                }
                if sp > st.max_speed {
                    st.max_speed = sp;
                }
                let sa = dynm.steer_angle;
                v.lights.update(dt, sa);
                if sp > 0.5 {
                    self.record.drive_clean(sp * dt);
                }
            }
        }
        // Lane-mode rpm estimates for audio.
        for v in self.sim.vehicles.iter_mut() {
            if v.mode != MotionMode::Lane {
                continue;
            }
            let sp = v.spec;
            let wheel = (v.speed / sp.wheel_radius) * sp.final_drive * 9.549;
            let mut g = v.gear_estimate;
            if wheel * sp.gears[g - 1] > sp.redline * 0.55 && g < sp.gears.len() {
                g += 1;
            } else if g > 1 && wheel * sp.gears[g - 2] < sp.redline * 0.42 {
                g -= 1;
            }
            v.gear_estimate = g;
            v.rpm_estimate = sp.idle_rpm.max(wheel * sp.gears[g - 1]) + v.accel.max(0.0) * 300.0;
        }
    }

    fn rebuild_vehicle_hash(&mut self) {
        self.vehicle_hash = SpatialHash::new(12.0);
        for v in &self.sim.vehicles {
            if v.alive {
                self.vehicle_hash.insert(v.id, v.x, v.y, v.half_len);
            }
        }
    }

    fn collisions(&mut self) {
        self.rebuild_vehicle_hash();
        if self.external_physics {
            return;
        }
        let mut i = 0;
        while i < self.sim.vehicles.len() {
            let (aid, ax, ay, ahl, a_alive) = {
                let a = &self.sim.vehicles[i];
                (a.id, a.x, a.y, a.half_len, a.alive)
            };
            if !a_alive {
                i += 1;
                continue;
            }
            let mut buf = Vec::new();
            self.vehicle_hash.query(ax, ay, ahl + 2.0, &mut buf);
            for bid in buf {
                if bid <= aid {
                    continue;
                }
                let Some(bi) = self.sim.vidx(bid) else {
                    continue;
                };
                let a = &self.sim.vehicles[i];
                let b = &self.sim.vehicles[bi];
                if !b.alive || a.external || b.external {
                    continue;
                }
                let (ba, bb) = if a.mode == MotionMode::Lane && b.mode == MotionMode::Lane {
                    if !lane_pair_can_collide(&self.sim.net, a, b) {
                        continue;
                    }
                    (set_box(a, 0.85), set_box(b, 0.85))
                } else {
                    (set_box(a, 1.0), set_box(b, 1.0))
                };
                let Some(contact) = box_contact(&ba, &bb) else {
                    continue;
                };
                self.resolve_vehicles(i, bi, contact);
            }
            let a = &self.sim.vehicles[i];
            if a.mode == MotionMode::Physics && !a.external {
                self.static_collisions(i);
            }
            i += 1;
        }
    }

    fn resolve_vehicles(&mut self, ai: usize, bi: usize, contact: Contact) {
        let (a, b) = (&self.sim.vehicles[ai], &self.sim.vehicles[bi]);
        let vn = (b.vx() - a.vx()) * contact.nx + (b.vy() - a.vy()) * contact.ny;
        let closing = -vn;
        let (a_lane, b_lane) = (a.mode == MotionMode::Lane, b.mode == MotionMode::Lane);
        let (aid, bid) = (a.id, b.id);
        if a_lane && b_lane && closing < 2.0 {
            return;
        }
        // Light pushes against a lane-mode car: treat it as a wall.
        if !a_lane && b_lane && closing < 1.2 {
            let a = &mut self.sim.vehicles[ai];
            resolve_static(&mut a.dynamics, &contact, 0.1, 0.3);
            a.sync_from_dynamics();
            self.bumped(aid, bid);
            return;
        }
        if !b_lane && a_lane && closing < 1.2 {
            let c2 = Contact {
                nx: -contact.nx,
                ny: -contact.ny,
                ..contact
            };
            let b = &mut self.sim.vehicles[bi];
            resolve_static(&mut b.dynamics, &c2, 0.1, 0.3);
            b.sync_from_dynamics();
            self.bumped(bid, aid);
            return;
        }
        if a_lane {
            self.sim.vehicles[ai].enter_physics();
        }
        if b_lane {
            self.sim.vehicles[bi].enter_physics();
        }
        let (a, b) = (&self.sim.vehicles[ai], &self.sim.vehicles[bi]);
        // Fault: who drove into whom.
        let a_into = a.vx() * contact.nx + a.vy() * contact.ny;
        let b_into = -(b.vx() * contact.nx + b.vy() * contact.ny);
        let at_fault = if a_into >= b_into { aid } else { bid };
        let j = {
            let (lo, hi) = self.sim.vehicles.split_at_mut(bi.max(ai));
            let (va, vb) = if ai < bi {
                (&mut lo[ai], &mut hi[0])
            } else {
                (&mut hi[0], &mut lo[bi])
            };
            let j = resolve_bodies(&mut va.dynamics, &mut vb.dynamics, &contact, 0.15, 0.4);
            va.sync_from_dynamics();
            vb.sync_from_dynamics();
            j
        };
        if j <= 0.0 {
            return;
        }
        self.collision_consequences(ai, bi, j, contact.px, contact.py, at_fault, a_lane, b_lane);
    }

    /// Damage, crash reactions, events, anger and law after an impact of `j` N·s.
    #[allow(clippy::too_many_arguments)]
    fn collision_consequences(
        &mut self,
        ai: usize,
        bi: usize,
        j: f64,
        px: f64,
        py: f64,
        at_fault: VehicleId,
        a_was_lane: bool,
        b_was_lane: bool,
    ) {
        let sev_a = j / self.sim.vehicles[ai].dynamics.mass;
        let sev_b = j / self.sim.vehicles[bi].dynamics.mass;
        self.damage(ai, sev_a, px, py);
        self.damage(bi, sev_b, px, py);
        let severity = sev_a.max(sev_b);
        if a_was_lane || severity > 2.0 {
            self.sim
                .with_driver(ai, |d, sim| d.on_crash(sim, ai, sev_a));
        }
        if b_was_lane || severity > 2.0 {
            self.sim
                .with_driver(bi, |d, sim| d.on_crash(sim, bi, sev_b));
        }
        let (aid, bid) = (self.sim.vehicles[ai].id, self.sim.vehicles[bi].id);
        let victim = if at_fault == aid { bid } else { aid };
        let now = self.sim.time;
        {
            let a = &mut self.sim.vehicles[ai];
            a.last_hit_time = now;
            a.last_hit_by = Some(bid);
        }
        {
            let b = &mut self.sim.vehicles[bi];
            b.last_hit_time = now;
            b.last_hit_by = Some(aid);
        }
        if severity > 0.8 {
            self.sim.stats.collisions += 1;
            self.sim.emit(Event::Collision {
                a: aid,
                b: bid,
                impulse: j,
                severity,
                x: px,
                y: py,
                at_fault,
            });
            self.sim.offend(
                at_fault,
                victim,
                OffenseKind::Collision,
                clamp(severity / 4.0, 0.5, 2.5),
            );
            if victim != at_fault {
                self.sim
                    .offend(victim, at_fault, OffenseKind::Collision, 0.3);
            }
        }
        if let Some(p) = self.sim.player.and_then(|p| self.sim.vehicle(p)) {
            if p.id == at_fault && severity > 1.5 {
                self.monitor.hit_run = Some(HitRun {
                    x: p.x,
                    y: p.y,
                    time: now,
                    stopped_for: 0.0,
                    victim: Some(victim),
                });
                self.observations
                    .push(Observation::new(ViolationKind::Collision));
            }
        }
        if severity > 7.0 {
            self.request_ambulance(px, py);
        }
    }

    /// Gentle contact with a lane car: annoys its driver.
    fn bumped(&mut self, phys: VehicleId, lane_car: VehicleId) {
        let now = self.sim.time;
        let last = self.near_miss_memo.get(&lane_car).copied().unwrap_or(-10.0);
        if now - last < 2.0 {
            return;
        }
        self.near_miss_memo.insert(lane_car, now);
        self.sim.offend(phys, lane_car, OffenseKind::NearMiss, 0.6);
    }

    fn static_collisions(&mut self, vi: usize) {
        let mut buf = Vec::new();
        {
            let v = &self.sim.vehicles[vi];
            self.sim
                .obstacle_hash
                .query(v.x, v.y, v.half_len + 1.0, &mut buf);
        }
        for oi in buf {
            let o = self.sim.obstacles[oi as usize];
            let v = &mut self.sim.vehicles[vi];
            let Some(contact) = box_contact(&set_box(v, 1.0), &o) else {
                continue;
            };
            let j = resolve_static(&mut v.dynamics, &contact, 0.12, 0.5);
            v.sync_from_dynamics();
            if j > 0.0 {
                let sev = j / v.dynamics.mass;
                let vid = v.id;
                self.damage(vi, sev, contact.px, contact.py);
                if sev > 1.5 {
                    self.sim.emit(Event::CrashStatic {
                        vehicle: vid,
                        severity: sev,
                        x: contact.px,
                        y: contact.py,
                    });
                }
                let chasing = self.sim.vehicles[vi]
                    .driver()
                    .is_some_and(|d| d.state == DriverState::Chasing);
                if sev > 3.0 && chasing {
                    self.sim.with_driver(vi, |d, sim| d.on_crash(sim, vi, sev));
                }
            }
        }
    }

    fn damage(&mut self, vi: usize, severity: f64, px: f64, py: f64) {
        let v = &mut self.sim.vehicles[vi];
        let c = v.heading.cos();
        let s = v.heading.sin();
        let lx = (px - v.x) * c + (py - v.y) * s;
        let ly = -(px - v.x) * s + (py - v.y) * c;
        let (zone, lost) = v.damage.hit(lx, ly, v.half_len, v.half_wid, severity);
        if lost > 0.0 {
            if zone == crate::vehicle::DamageZone::Front {
                v.pt.health = v.pt.health.min(0.25 + v.damage.front * 0.75);
                if v.damage.front < 0.5 {
                    if ly > 0.0 {
                        v.lights.left_headlight_broken = true;
                    } else {
                        v.lights.right_headlight_broken = true;
                    }
                }
                if v.damage.front < 0.15 {
                    v.pt.health = 0.0;
                }
            }
            v.dynamics.tire_health = v.dynamics.tire_health.min(0.5 + v.damage.body() * 0.5);
            if v.damage.totaled() && !v.wrecked {
                v.wrecked = true;
                v.lights.hazard = !v.is_player;
                if v.is_player {
                    self.sim.emit(Event::Info {
                        text: "Your vehicle is wrecked. Press ` to reset.".into(),
                    });
                }
            }
        }
    }

    /// Put the player back on the nearest road (repairs cost money).
    pub fn reset_player(&mut self) {
        let Some(pi) = self.sim.player_idx() else {
            return;
        };
        let (px, py) = (self.sim.vehicles[pi].x, self.sim.vehicles[pi].y);
        let hits = self.sim.net.nearest_lanes(px, py, 200.0);
        let Some(h) = hits.first() else { return };
        let lane = self.sim.net.lane(h.lane);
        let s = clamp(h.s, 5.0, lane.length - 5.0);
        let pt = lane.path.sample(s);
        let p = &mut self.sim.vehicles[pi];
        p.dynamics.reset(pt.x, pt.y, pt.heading, 0.0);
        p.sync_from_dynamics();
        let cost = ((1.0 - p.damage.body()) * 1500.0 + (1.0 - p.pt.health) * 800.0).floor();
        p.damage.repair();
        p.pt.health = 1.0;
        p.pt.running = true;
        p.pt.stalled = false;
        p.wrecked = false;
        p.dynamics.tire_health = 1.0;
        p.lights.left_headlight_broken = false;
        p.lights.right_headlight_broken = false;
        if p.pt.fuel < 5.0 {
            p.pt.fuel = p.spec.fuel_capacity * 0.5;
        }
        if cost > 0.0 {
            self.record.money -= cost;
            self.sim.emit(Event::Info {
                text: format!("Vehicle repaired: -€${cost}"),
            });
        }
    }

    // ----------------------------------------------------------- law

    fn law_and_order(&mut self, dt: f64) {
        if let Some(pi) = self.sim.player_idx() {
            let mut obs = std::mem::take(&mut self.observations);
            self.monitor.update(dt, &self.sim, pi, &mut obs);
            for o in &obs {
                self.handle_violation(o.kind, o.camera_node.is_some(), o.speed_camera);
            }
            obs.clear();
            self.observations = obs;
            self.player_social(dt);
        }
        self.police
            .update(dt, &mut self.sim, &mut self.record, &self.spawner);
    }

    fn handle_violation(&mut self, kind: ViolationKind, red_camera: bool, speed_camera: bool) {
        let now = self.sim.time;
        if !self.record.can_charge(kind, now) {
            return;
        }
        let Some(p) = self.sim.player.and_then(|p| self.sim.vehicle(p)) else {
            return;
        };
        let (pid, px, py) = (p.id, p.x, p.y);
        let cops = self.sim.police_near(px, py, 110.0);
        let witness = if !cops.is_empty() {
            Witness::Police
        } else if red_camera || speed_camera {
            Witness::Camera
        } else {
            Witness::None
        };
        let rec = self.record.add(kind, now, witness, px, py);
        self.sim.emit(Event::Violation {
            vehicle: pid,
            kind,
            label: rec.label,
            fine: rec.fine,
            points: rec.points,
            witnessed: witness,
            x: px,
            y: py,
        });
        match witness {
            Witness::Camera => {
                self.sim.emit(Event::CameraFlash {
                    x: px,
                    y: py,
                    kind: if red_camera {
                        super::events::CameraKind::RedLight
                    } else {
                        super::events::CameraKind::Speed
                    },
                });
                self.sim.emit(Event::Ticket {
                    vehicle: pid,
                    total: rec.fine,
                    points: rec.points,
                    reason: format!("{} (camera)", rec.label),
                });
            }
            Witness::Police => {
                self.police.report(
                    &mut self.sim,
                    &self.spawner,
                    cops.first().copied(),
                    &mut self.record,
                    kind.def().heat,
                );
            }
            Witness::None => {}
        }
    }

    /// Player behaviour that upsets AI drivers: tailgating, near misses.
    fn player_social(&mut self, dt: f64) {
        let Some(pi) = self.sim.player_idx() else {
            return;
        };
        let p = &self.sim.vehicles[pi];
        let (pid, px, py, ph, pspeed, pvx, pvy, phw) = (
            p.id,
            p.x,
            p.y,
            p.heading,
            p.abs_speed(),
            p.vx(),
            p.vy(),
            p.half_wid,
        );
        let info = &self.monitor.info;
        match info.lane {
            Some(lane) if info.headway < 0.7 && pspeed > 6.0 && info.leader_gap < 20.0 => {
                let mut leader = None;
                let mut best = f64::INFINITY;
                for e in self.sim.occ.lane(lane) {
                    if e.vehicle == pid {
                        continue;
                    }
                    let Some(v) = self.sim.vehicle(e.vehicle) else {
                        continue;
                    };
                    let d = (v.x - px) * ph.cos() + (v.y - py) * ph.sin();
                    if d > 0.0 && d < best {
                        best = d;
                        leader = Some(v.id);
                    }
                }
                if let Some(l) = leader {
                    let t = self.player_tailgate.get(&l).copied().unwrap_or(0.0) + dt;
                    self.player_tailgate.insert(l, t);
                    if t > 2.0 {
                        self.sim.offend(pid, l, OffenseKind::Tailgating, dt * 2.5);
                    }
                }
            }
            _ => self.player_tailgate.clear(),
        }
        // Near misses: fast passes within a metre or two.
        if pspeed > 8.0 {
            let mut buf = Vec::new();
            self.vehicle_hash.query(px, py, 6.0, &mut buf);
            for vid in buf {
                if vid == pid {
                    continue;
                }
                let Some(v) = self.sim.vehicle(vid) else {
                    continue;
                };
                let d = hypot(v.x - px, v.y - py) - (v.half_wid + phw);
                let rel = hypot(v.vx() - pvx, v.vy() - pvy);
                if d < 1.2 && rel > 7.0 {
                    let now = self.sim.time;
                    let last = self.near_miss_memo.get(&vid).copied().unwrap_or(-10.0);
                    if now - last < 3.0 {
                        continue;
                    }
                    self.near_miss_memo.insert(vid, now);
                    self.sim.emit(Event::NearMiss { a: pid, b: vid });
                    self.sim
                        .offend(pid, vid, OffenseKind::NearMiss, clamp(rel / 12.0, 0.5, 2.0));
                }
            }
        }
    }

    // ----------------------------------------------------------- incidents

    fn update_incidents(&mut self, dt: f64) {
        let pp = self.player().map(|p| (p.x, p.y));
        if self.breakdown_interval > 0.0 {
            self.breakdown_timer -= dt;
            if self.breakdown_timer <= 0.0 {
                self.breakdown_timer = self
                    .sim
                    .rng
                    .range(self.breakdown_interval * 0.6, self.breakdown_interval * 1.4);
                // Prefer a car the player will meet: 50-300 m away, mid-block.
                let net = &self.sim.net;
                let cands: Vec<usize> = self
                    .sim
                    .vehicles
                    .iter()
                    .enumerate()
                    .filter(|(_, v)| {
                        let Some(SegRef::Lane(l)) = v.seg else {
                            return false;
                        };
                        if v.role != VehicleRole::Civilian
                            || v.mode != MotionMode::Lane
                            || v.speed < 3.0
                        {
                            return false;
                        }
                        if v.s < 15.0 || net.lane(l).length - v.s < 35.0 {
                            return false;
                        }
                        let Some(d) = v.driver() else { return false };
                        if d.hold_time > 0.0 || d.rage.is_some() {
                            return false;
                        }
                        pp.is_none_or(|(px, py)| {
                            let dist = hypot(v.x - px, v.y - py);
                            (50.0..=300.0).contains(&dist)
                        })
                    })
                    .map(|(i, _)| i)
                    .collect();
                if !cands.is_empty() {
                    let vi = self.sim.rng.pick(&cands);
                    let secs = self.sim.rng.range(40.0, 90.0);
                    let v = &mut self.sim.vehicles[vi];
                    let (vid, x, y) = (v.id, v.x, v.y);
                    if let Some(d) = v.driver_mut() {
                        d.hold(HoldKind::Breakdown, secs);
                    }
                    let place = v
                        .seg
                        .and_then(|s| s.lane())
                        .map(|l| self.sim.net.road(self.sim.net.lane(l).road).name.clone())
                        .unwrap_or_default();
                    self.sim.emit(Event::Incident {
                        kind: IncidentKind::Breakdown,
                        x,
                        y,
                        vehicle: Some(vid),
                        node: None,
                        place,
                    });
                }
            }
        }
        if self.outage_interval > 0.0 && !self.sim.signals.is_empty() {
            self.outage_timer -= dt;
            if self.outage_timer <= 0.0 {
                self.outage_timer = self
                    .sim
                    .rng
                    .range(self.outage_interval * 0.6, self.outage_interval * 1.4);
                let si = self.sim.rng.index(self.sim.signals.len());
                let mode = self.sim.signals[si].mode;
                if mode != SignalMode::Off && mode != SignalMode::Flashing {
                    self.sim.signals[si].set_mode(SignalMode::Off);
                    let until = self.sim.time + self.sim.rng.range(80.0, 140.0);
                    self.outages.push((SignalId::from(si), until));
                    let node = self.sim.net.node(self.sim.signals[si].node);
                    let (x, y, nid, place) = (node.x, node.y, node.id, node.name.clone());
                    self.sim.emit(Event::Incident {
                        kind: IncidentKind::PowerCut,
                        x,
                        y,
                        vehicle: None,
                        node: Some(nid),
                        place,
                    });
                }
            }
        }
        let now = self.sim.time;
        let mut i = self.outages.len();
        while i > 0 {
            i -= 1;
            let (sid, until) = self.outages[i];
            if now < until {
                continue;
            }
            self.sim.signals[sid.idx()].set_mode(SignalMode::Actuated);
            self.outages.remove(i);
            let node = self.sim.net.node(self.sim.signals[sid.idx()].node);
            let (x, y, nid, place) = (node.x, node.y, node.id, node.name.clone());
            self.sim.emit(Event::Incident {
                kind: IncidentKind::PowerRestored,
                x,
                y,
                vehicle: None,
                node: Some(nid),
                place,
            });
        }
    }

    // ----------------------------------------------------------- pedestrians

    fn update_pedestrians(&mut self, dt: f64) {
        let env = &self.sim.env;
        let night = 1.0 - env.daylight;
        let target = js_round(
            self.pedestrian_density
                * (1.0 - night * 0.6)
                * (0.5 + env.traffic_density * 0.5)
                * (1.0 - env.rain * 0.5),
        );
        self.sim.peds.target = target.max(0.0) as usize;
        {
            let Sim {
                peds,
                net,
                signals,
                vehicles,
                events,
                rng,
                ..
            } = &mut self.sim;
            let mut host = WorldPedHost {
                net,
                signals,
                vehicles,
                events,
                rng,
            };
            peds.update(dt, &mut host);
        }
        // Vehicles touching pedestrians.
        let mut k = 0;
        while k < self.sim.peds.peds.len() {
            let ped = &self.sim.peds.peds[k];
            if ped.state != PedState::Crossing && ped.state != PedState::Waiting {
                k += 1;
                continue;
            }
            let (pxx, pyy) = (ped.x, ped.y);
            let mut buf = Vec::new();
            self.vehicle_hash.query(pxx, pyy, 4.0, &mut buf);
            for vid in buf {
                let Some(v) = self.sim.vehicle(vid) else {
                    continue;
                };
                let ped = &self.sim.peds.peds[k];
                let c = v.heading.cos();
                let s = v.heading.sin();
                let lx = (ped.x - v.x) * c + (ped.y - v.y) * s;
                let ly = -(ped.x - v.x) * s + (ped.y - v.y) * c;
                let inside = lx.abs() < v.half_len + 0.3 && ly.abs() < v.half_wid + 0.3;
                let speed = v.abs_speed();
                let (is_player, vx, vy, physics) =
                    (v.is_player, v.x, v.y, v.mode == MotionMode::Physics);
                let crossing = ped.state == PedState::Crossing;
                if inside && speed > 1.2 && (physics || crossing) {
                    let ped = &mut self.sim.peds.peds[k];
                    ped.state = PedState::Down;
                    ped.ttl = 40.0;
                    let (pid, x, y) = (ped.id, ped.x, ped.y);
                    self.sim.emit(Event::PedHit {
                        ped: pid,
                        vehicle: vid,
                        severity: speed,
                        x,
                        y,
                    });
                    if speed > 4.0 {
                        self.request_ambulance(x, y);
                    }
                    if is_player {
                        self.observations
                            .push(Observation::new(ViolationKind::HitPedestrian));
                        self.monitor.hit_run = Some(HitRun {
                            x: vx,
                            y: vy,
                            time: self.sim.time,
                            stopped_for: 0.0,
                            victim: None,
                        });
                    }
                    break;
                }
                // Close pass in front of someone crossing: they shout, the player is charged.
                let near = ly.abs() < v.half_wid + 1.6 && lx > -v.half_len && lx < v.half_len + 2.5;
                if near && crossing && speed > 2.5 && ped.shout_cooldown <= 0.0 {
                    let ped = &mut self.sim.peds.peds[k];
                    ped.shout_cooldown = 5.0;
                    let (pid, x, y) = (ped.id, ped.x, ped.y);
                    let text = self.sim.rng.pick(&PED_SHOUTS);
                    self.sim.emit(Event::PedSpeech {
                        ped: pid,
                        text,
                        x,
                        y,
                    });
                    if is_player {
                        self.observations
                            .push(Observation::new(ViolationKind::FailYieldPedestrian));
                    }
                }
            }
            k += 1;
        }
    }

    // ----------------------------------------------------------- ambulances

    fn request_ambulance(&mut self, x: f64, y: f64) {
        if self
            .ambulance
            .is_some_and(|a| self.sim.vehicle(a).is_some())
        {
            return;
        }
        let Some((lane, s)) = self.spawner.find_spot(&mut self.sim, 200.0, 700.0) else {
            return;
        };
        let id = self.spawner.create(
            &mut self.sim,
            VehicleClass::Ambulance,
            VehicleRole::Ambulance,
            lane,
            s,
            8.0,
            None,
        );
        let Some(vi) = self.sim.vidx(id) else { return };
        let hits = self.sim.net.nearest_lanes(x, y, 30.0);
        let target = hits.first().map(|h| {
            let l = self.sim.net.lane(h.lane);
            (l.road, l.forward)
        });
        let Sim {
            net,
            router,
            vehicles,
            ..
        } = &mut self.sim;
        let v = &mut vehicles[vi];
        let cur = v.seg;
        let Some(d) = v.driver.as_deref_mut() else {
            return;
        };
        d.emergency = true;
        if let Some((road, fwd)) = target {
            d.route_to(net, router, cur, road, fwd);
        }
        v.lights.siren = SirenMode::Wail;
        v.lights.lightbar = true;
        self.ambulance = Some(id);
        self.ambulance_target = Some((x, y, 0.0));
        self.sim.emit(Event::Dispatch {
            unit: id,
            kind: super::events::DispatchKind::Ambulance,
            x,
            y,
            reason: "collision",
        });
    }

    fn ambulances(&mut self, dt: f64) {
        let (Some(a), Some((tx, ty, mut tt))) = (self.ambulance, self.ambulance_target) else {
            return;
        };
        let Some(vi) = self.sim.vidx(a) else {
            self.ambulance = None;
            return;
        };
        let v = &mut self.sim.vehicles[vi];
        let dist = hypot(v.x - tx, v.y - ty);
        tt += dt;
        let emergency = v.driver().is_some_and(|d| d.emergency);
        if (dist < 30.0 || tt > 120.0) && emergency {
            if let Some(d) = v.driver_mut() {
                d.emergency = false;
            }
            v.lights.siren = SirenMode::Off;
            tt = 0.0;
        }
        self.ambulance_target = Some((tx, ty, tt));
        let still_emergency = v.driver().is_some_and(|d| d.emergency);
        if v.driver().is_some() && !still_emergency && tt > 30.0 {
            v.lights.lightbar = false;
            self.ambulance = None;
            self.ambulance_target = None;
        }
    }

    // ----------------------------------------------------------- host physics bridge

    /// Set the pose of a host-simulated vehicle (call each step before
    /// [`World::step`]). Applies to physics-mode vehicles; lane-mode
    /// vehicles keep their kinematic lane pose (returns false).
    pub fn set_external_pose(&mut self, id: VehicleId, pose: ExternalPose) -> bool {
        let Some(v) = self.sim.vehicle_mut(id) else {
            return false;
        };
        if v.mode == MotionMode::Lane {
            return false;
        }
        let d = &mut v.dynamics;
        d.x = pose.x as f64;
        d.y = pose.y as f64;
        d.yaw = pose.heading as f64;
        d.set_world_velocity(pose.vx as f64, pose.vy as f64);
        d.r = pose.yaw_rate as f64;
        d.omega_f = d.u / v.spec.wheel_radius;
        d.omega_r = d.omega_f;
        d.steer_angle = v.steer_target;
        let prev_speed = v.speed;
        v.sync_from_dynamics();
        v.accel = v.speed - prev_speed;
        true
    }

    /// The controls the AI driver of `id` wants applied (physics mode only).
    pub fn ai_controls(&self, id: VehicleId) -> Option<DriveControls> {
        let v = self.sim.vehicle(id)?;
        if !v.has_ai || v.mode != MotionMode::Physics {
            return None;
        }
        Some(Self::controls_of(v))
    }

    /// The controls the player driver applied this step.
    pub fn player_controls(&self) -> Option<DriveControls> {
        self.player().map(Self::controls_of)
    }

    fn controls_of(v: &Vehicle) -> DriveControls {
        let g = v.pt.selected_gear();
        DriveControls {
            steer: (v.steer_target / v.spec.max_steer) as f32,
            steer_angle: v.steer_target as f32,
            throttle: v.inputs.throttle as f32,
            brake: v.inputs.brake as f32,
            handbrake: v.inputs.handbrake as f32,
            gear_dir: g.signum() as i8,
        }
    }

    /// Report a collision detected by the host's physics (`b = None` for a
    /// static obstacle). The World applies the same consequences as for its
    /// own detected collisions: physics mode, damage, crash reactions,
    /// anger/grudges, collision events, hit-and-run and ambulances.
    pub fn report_collision(
        &mut self,
        a: VehicleId,
        b: Option<VehicleId>,
        impulse: f32,
        point: (f32, f32),
    ) {
        let j = impulse as f64;
        let (px, py) = (point.0 as f64, point.1 as f64);
        if j.is_nan() || j <= 0.0 {
            return;
        }
        let Some(ai) = self.sim.vidx(a) else { return };
        let Some(b) = b else {
            // Static obstacle.
            let was_lane = self.sim.vehicles[ai].mode == MotionMode::Lane;
            if was_lane {
                self.sim.vehicles[ai].enter_physics();
            }
            let sev = j / self.sim.vehicles[ai].dynamics.mass;
            self.damage(ai, sev, px, py);
            if sev > 1.5 {
                self.sim.emit(Event::CrashStatic {
                    vehicle: a,
                    severity: sev,
                    x: px,
                    y: py,
                });
            }
            let chasing = self.sim.vehicles[ai]
                .driver()
                .is_some_and(|d| d.state == DriverState::Chasing);
            if was_lane || (sev > 3.0 && chasing) {
                self.sim.with_driver(ai, |d, sim| d.on_crash(sim, ai, sev));
            }
            return;
        };
        let Some(bi) = self.sim.vidx(b) else { return };
        if ai == bi {
            return;
        }
        let (va, vb) = (&self.sim.vehicles[ai], &self.sim.vehicles[bi]);
        let (mut nx, mut ny) = (vb.x - va.x, vb.y - va.y);
        let l = hypot(nx, ny);
        if l > 1e-6 {
            nx /= l;
            ny /= l;
        } else {
            nx = 1.0;
            ny = 0.0;
        }
        let closing = -((vb.vx() - va.vx()) * nx + (vb.vy() - va.vy()) * ny);
        let (a_lane, b_lane) = (va.mode == MotionMode::Lane, vb.mode == MotionMode::Lane);
        if a_lane && b_lane && closing < 2.0 {
            return;
        }
        if !a_lane && b_lane && closing < 1.2 {
            self.bumped(a, b);
            return;
        }
        if a_lane && !b_lane && closing < 1.2 {
            self.bumped(b, a);
            return;
        }
        let a_into = va.vx() * nx + va.vy() * ny;
        let b_into = -(vb.vx() * nx + vb.vy() * ny);
        let at_fault = if a_into >= b_into { a } else { b };
        if a_lane {
            self.sim.vehicles[ai].enter_physics();
        }
        if b_lane {
            self.sim.vehicles[bi].enter_physics();
        }
        self.collision_consequences(ai, bi, j, px, py, at_fault, a_lane, b_lane);
    }

    /// Mark a vehicle as host-simulated (or not).
    pub fn set_external(&mut self, id: VehicleId, external: bool) {
        if let Some(v) = self.sim.vehicle_mut(id) {
            v.external = external;
        }
    }
}
