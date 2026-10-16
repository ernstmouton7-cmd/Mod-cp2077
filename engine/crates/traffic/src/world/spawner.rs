//! Population control: spawning civilians and patrol cars, despawning wrecks.

use super::sim::Sim;
use crate::ai::driver::AIDriver;
use crate::ai::personality::{sample_personality, Archetype};
use crate::ids::{LaneId, SegRef, VehicleId};
use crate::vehicle::{
    spec, HeadlightSwitch, MotionMode, Vehicle, VehicleClass, VehicleRole, CIVILIAN_CLASSES,
};

/// Population targets.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PopulationOptions {
    /// Civilian vehicles at traffic density 1.0.
    pub civilians: usize,
    pub police: usize,
    /// Minimum spawn/despawn distance from the player.
    pub min_distance: f64,
}

/// Keeps the population near its target: spawns civilians (weighted vehicle
/// mix, sampled personalities) and patrol cars away from the player, removes
/// long-dead wrecks and surplus vehicles far away.
#[derive(Clone, Debug)]
pub struct Spawner {
    timer: f64,
    /// Population control on/off (scripted scenarios turn it off).
    pub enabled: bool,
    pub opts: PopulationOptions,
    /// Spawn new vehicles as host-simulated (`Vehicle::external`).
    pub external: bool,
}

impl Spawner {
    pub fn new(opts: PopulationOptions) -> Self {
        Spawner {
            timer: 0.0,
            enabled: true,
            opts,
            external: false,
        }
    }

    /// Weighted civilian vehicle class.
    pub fn pick_class(&self, sim: &mut Sim) -> VehicleClass {
        let entries: Vec<(VehicleClass, f64)> = CIVILIAN_CLASSES
            .iter()
            .map(|&c| (c, spec(c).spawn_weight))
            .collect();
        sim.rng.weighted(&entries)
    }

    /// Create a vehicle with a driver on `lane` at arc length `s`.
    #[allow(clippy::too_many_arguments)]
    pub fn create(
        &self,
        sim: &mut Sim,
        cls: VehicleClass,
        role: VehicleRole,
        lane: LaneId,
        s: f64,
        speed: f64,
        archetype: Option<Archetype>,
    ) -> VehicleId {
        let sp = spec(cls);
        let color = sim.rng.pick(sp.colors);
        let id = sim.next_vehicle_id();
        let mut v = Vehicle::new(id, sp, color, role);
        v.external = self.external;
        let mut arch = archetype;
        if arch.is_none() {
            if role == VehicleRole::Police
                || role == VehicleRole::Ambulance
                || cls == VehicleClass::Bus
                || cls == VehicleClass::Truck
            {
                arch = Some(Archetype::Professional);
            } else if cls == VehicleClass::Taxi {
                arch = Some(if sim.rng.chance(0.5) {
                    Archetype::Professional
                } else {
                    Archetype::Speeder
                });
            } else if cls == VehicleClass::Sports && sim.rng.chance(0.4) {
                arch = Some(Archetype::Joyrider);
            }
        }
        let mut p = sample_personality(&mut sim.rng, arch);
        if role == VehicleRole::Police {
            p.lawfulness = 1.0;
            p.aggression = 0.3;
            p.temper = 0.3;
        }
        let seed = (sim.rng.next_f64() * 1e9).floor() as i64 + 1;
        let d = AIDriver::new(id, p, seed);
        v.place_on_segment(&sim.net, SegRef::Lane(lane), s, speed);
        v.lights.headlight_switch = if sim.rng.chance(0.95) {
            HeadlightSwitch::Auto
        } else {
            HeadlightSwitch::Off
        };
        sim.add_vehicle(v, d);
        id
    }

    /// A free spawn point at least `min_d` (at most `max_d`) from the player.
    pub fn find_spot(&self, sim: &mut Sim, min_d: f64, max_d: f64) -> Option<(LaneId, f64)> {
        let pp = sim
            .player_idx()
            .map(|i| (sim.vehicles[i].x, sim.vehicles[i].y));
        for _ in 0..25 {
            let li = sim.rng.index(sim.net.lanes.len());
            let lane = &sim.net.lanes[li];
            if lane.length < 30.0 {
                continue;
            }
            let len = lane.length;
            let s = sim.rng.range(10.0, len - 15.0);
            let pt = sim.net.lanes[li].path.sample(s);
            if let Some((px, py)) = pp {
                let d = ((pt.x - px) * (pt.x - px) + (pt.y - py) * (pt.y - py)).sqrt();
                if d < min_d || d > max_d {
                    continue;
                }
            }
            if !sim.lane_free(LaneId::from(li), s, 14.0) {
                continue;
            }
            return Some((LaneId::from(li), s));
        }
        None
    }

    /// Fill the city at start-up (anywhere except right on top of the player).
    pub fn populate(&self, sim: &mut Sim, density: f64) {
        let n = (self.opts.civilians as f64 * density).floor() as usize;
        for _ in 0..n {
            let Some((lane, s)) = self.find_spot(sim, 25.0, f64::INFINITY) else {
                continue;
            };
            let cls = self.pick_class(sim);
            let speed = sim.net.lane(lane).speed_limit * 0.6;
            self.create(sim, cls, VehicleRole::Civilian, lane, s, speed, None);
        }
        for _ in 0..self.opts.police {
            if let Some((lane, s)) = self.find_spot(sim, 60.0, f64::INFINITY) {
                self.create(
                    sim,
                    VehicleClass::Police,
                    VehicleRole::Police,
                    lane,
                    s,
                    5.0,
                    None,
                );
            }
        }
    }

    /// Population control (every 0.5 s). `busy` vehicles (police units,
    /// the ambulance) are never despawned.
    pub fn update(&mut self, dt: f64, density: f64, sim: &mut Sim, busy: &[VehicleId]) {
        if !self.enabled {
            return;
        }
        self.timer -= dt;
        if self.timer > 0.0 {
            return;
        }
        self.timer = 0.5;
        let pp = sim
            .player_idx()
            .map(|i| (sim.vehicles[i].x, sim.vehicles[i].y));
        let mut civ = 0;
        let mut police = 0;
        for v in &sim.vehicles {
            if v.role == VehicleRole::Police {
                police += 1;
            } else if v.role != VehicleRole::Player {
                civ += 1;
            }
        }
        let target = (self.opts.civilians as f64 * density).floor();
        // Despawn wrecks and stuck vehicles out of the player's way.
        let md = self.opts.min_distance;
        for i in 0..sim.vehicles.len() {
            let v = &mut sim.vehicles[i];
            if v.is_player || busy.contains(&v.id) {
                continue;
            }
            let far = pp
                .is_none_or(|(px, py)| (v.x - px) * (v.x - px) + (v.y - py) * (v.y - py) > md * md);
            if v.wrecked {
                v.despawn_timer += 0.5;
            }
            if (v.despawn_timer > 45.0 && far) || v.despawn_timer > 240.0 {
                let id = v.id;
                sim.remove_vehicle(id);
                return;
            }
        }
        if (civ as f64) < target {
            if let Some((lane, s)) = self.find_spot(sim, md, f64::INFINITY) {
                let cls = self.pick_class(sim);
                let speed = sim.net.lane(lane).speed_limit * 0.7;
                self.create(sim, cls, VehicleRole::Civilian, lane, s, speed, None);
            }
        } else if civ as f64 > target * 1.15 + 2.0 {
            let surplus = sim
                .vehicles
                .iter()
                .find(|v| {
                    let near = pp.is_some_and(|(px, py)| {
                        (v.x - px) * (v.x - px) + (v.y - py) * (v.y - py) < 150.0 * 150.0
                    });
                    v.role == VehicleRole::Civilian && v.mode == MotionMode::Lane && !near
                })
                .map(|v| v.id);
            if let Some(id) = surplus {
                sim.remove_vehicle(id);
            }
        }
        if police < self.opts.police {
            if let Some((lane, s)) = self.find_spot(sim, md, f64::INFINITY) {
                self.create(
                    sim,
                    VehicleClass::Police,
                    VehicleRole::Police,
                    lane,
                    s,
                    5.0,
                    None,
                );
            }
        }
    }
}
