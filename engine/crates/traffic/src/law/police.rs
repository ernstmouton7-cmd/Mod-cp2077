//! Traffic police: dispatch, pull-over, ticket, pursuit and escape.

use super::violations::{DrivingRecord, ViolationKind, Witness};
use crate::ai::driver::DriverState;
use crate::ai::emotion::Mood;
use crate::ai::speech::POLICE_LINES;
use crate::ids::VehicleId;
use crate::math::scalar::hypot;
use crate::math::Rng;
use crate::vehicle::{MotionMode, SirenMode, VehicleClass, VehicleRole};
use crate::world::events::{DispatchKind, Event, PulloverStage};
use crate::world::sim::Sim;
use crate::world::spawner::Spawner;

/// Police state machine stage.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PoliceStage {
    None,
    Responding,
    Pullover,
    Stopped,
    Pursuit,
    Searching,
}

/// Traffic police: dispatches the nearest patrol car when an officer
/// witnesses a violation, performs the pull-over (follow, wait for the player
/// to stop, write the ticket) and escalates to a pursuit with more units when
/// the player flees. Losing all units for long enough ends the pursuit.
#[derive(Clone, Debug)]
pub struct PoliceSystem {
    pub stage: PoliceStage,
    pub units: Vec<VehicleId>,
    timer: f64,
    still_time: f64,
    flee_time: f64,
    lost_time: f64,
    route_timer: f64,
    pursuit_time: f64,
    /// Countdown shown to the player while being written up.
    pub ticket_countdown: f64,
    rng: Rng,
}

impl Default for PoliceSystem {
    fn default() -> Self {
        PoliceSystem {
            stage: PoliceStage::None,
            units: Vec::new(),
            timer: 0.0,
            still_time: 0.0,
            flee_time: 0.0,
            lost_time: 0.0,
            route_timer: 0.0,
            pursuit_time: 0.0,
            ticket_countdown: 0.0,
            rng: Rng::new(911),
        }
    }
}

fn player_pos(sim: &Sim) -> Option<(VehicleId, f64, f64)> {
    sim.player_idx().map(|i| (sim.vehicles[i].id, sim.vehicles[i].x, sim.vehicles[i].y))
}

impl PoliceSystem {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn active(&self) -> bool {
        self.stage != PoliceStage::None
    }

    /// An officer (`unit`) saw the player commit a violation worth `heat`.
    pub fn report(&mut self, sim: &mut Sim, spawner: &Spawner, unit: Option<VehicleId>, rec: &mut DrivingRecord, heat: f64) {
        let Some((_, px, py)) = player_pos(sim) else { return };
        rec.heat = 5f64.min(rec.heat.max(if self.stage == PoliceStage::Pursuit { rec.heat + heat * 0.5 } else { heat }));
        sim.emit(Event::Heat { level: rec.heat.floor() as u32, reason: "violation witnessed" });
        if self.stage == PoliceStage::None {
            let Some(u) = unit.or_else(|| self.nearest_free_unit(sim, px, py)) else { return };
            self.assign(sim, u);
            self.stage = PoliceStage::Responding;
            self.timer = 0.0;
            sim.emit(Event::Dispatch { unit: u, kind: DispatchKind::Police, x: px, y: py, reason: "traffic violation" });
        } else if self.stage == PoliceStage::Pursuit && rec.heat >= 3.0 && (self.units.len() as f64) < rec.heat.floor() {
            self.add_unit(sim, spawner);
        }
    }

    fn nearest_free_unit(&self, sim: &Sim, x: f64, y: f64) -> Option<VehicleId> {
        let mut best = None;
        let mut bd = f64::INFINITY;
        for v in &sim.vehicles {
            if v.role != VehicleRole::Police || !v.alive || v.wrecked || self.units.contains(&v.id) {
                continue;
            }
            let d = (v.x - x) * (v.x - x) + (v.y - y) * (v.y - y);
            if d < bd {
                bd = d;
                best = Some(v.id);
            }
        }
        best
    }

    fn assign(&mut self, sim: &mut Sim, u: VehicleId) {
        self.units.push(u);
        let Some(v) = sim.vehicle_mut(u) else { return };
        if let Some(d) = v.driver_mut() {
            d.emergency = true;
            d.rage = None;
        }
        v.lights.siren = SirenMode::Wail;
        v.lights.lightbar = true;
    }

    fn add_unit(&mut self, sim: &mut Sim, spawner: &Spawner) {
        let Some((_, px, py)) = player_pos(sim) else { return };
        let mut u = self.nearest_free_unit(sim, px, py);
        let too_far = u.and_then(|id| sim.vehicle(id)).is_none_or(|v| hypot(v.x - px, v.y - py) > 400.0);
        if too_far {
            // Spawn a patrol car out of sight.
            u = spawner.find_spot(sim, 120.0, 400.0).map(|(lane, s)| spawner.create(sim, VehicleClass::Police, VehicleRole::Police, lane, s, 8.0, None));
        }
        let Some(u) = u else { return };
        self.assign(sim, u);
        sim.emit(Event::Dispatch { unit: u, kind: DispatchKind::Police, x: px, y: py, reason: "backup" });
    }

    fn release(&mut self, sim: &mut Sim) {
        for &u in &self.units {
            let Some(v) = sim.vehicle_mut(u) else { continue };
            let physics = v.mode == MotionMode::Physics;
            if let Some(d) = v.driver_mut() {
                d.emergency = false;
                d.follow_target = None;
                d.head_for = None;
                if physics {
                    d.state = DriverState::Recovering;
                }
            }
            v.lights.siren = SirenMode::Off;
            v.lights.lightbar = false;
        }
        self.units.clear();
        self.stage = PoliceStage::None;
        self.still_time = 0.0;
        self.flee_time = 0.0;
        self.lost_time = 0.0;
        self.pursuit_time = 0.0;
        self.ticket_countdown = 0.0;
    }

    fn seen(&self, sim: &Sim, px: f64, py: f64, range: f64) -> Option<VehicleId> {
        self.units.iter().copied().find(|&u| {
            sim.vehicle(u).is_some_and(|v| {
                let dx = v.x - px;
                let dy = v.y - py;
                dx * dx + dy * dy < range * range && sim.los(v.x, v.y, px, py)
            })
        })
    }

    pub fn update(&mut self, dt: f64, sim: &mut Sim, rec: &mut DrivingRecord, spawner: &Spawner) {
        let Some(pi) = sim.player_idx() else { return };
        if self.stage == PoliceStage::None {
            return;
        }
        // Drop wrecked units.
        self.units.retain(|&u| sim.vehicle(u).is_some_and(|v| v.alive && !v.wrecked));
        if self.units.is_empty() {
            if self.stage == PoliceStage::Pursuit || self.stage == PoliceStage::Searching {
                self.add_unit(sim, spawner);
                if self.units.is_empty() {
                    sim.emit(Event::Pullover { police: None, stage: PulloverStage::Escaped });
                    rec.heat = 0.0;
                    self.release(sim);
                }
            } else {
                self.release(sim);
            }
            return;
        }
        let pi = sim.player_idx().unwrap_or(pi);
        let (pid, px, py, speed) = {
            let p = &sim.vehicles[pi];
            (p.id, p.x, p.y, p.abs_speed())
        };
        self.timer += dt;
        let lead = self.units[0];
        let (lx, ly) = sim.vehicle(lead).map(|v| (v.x, v.y)).unwrap_or((px, py));
        let dist = hypot(lx - px, ly - py);

        // Units not yet close drive on the road network toward the player.
        self.route_timer -= dt;
        for ui in 0..self.units.len() {
            let u = self.units[ui];
            let Some(vi) = sim.vidx(u) else { continue };
            if sim.vehicles[vi].driver.is_none() {
                continue;
            }
            let (ux, uy) = (sim.vehicles[vi].x, sim.vehicles[vi].y);
            let du = hypot(ux - px, uy - py);
            let close = du < 55.0 && sim.los(ux, uy, px, py);
            let follow = sim.vehicles[vi].driver().and_then(|d| d.follow_target);
            if close && follow.is_none() {
                let gap = if self.stage == PoliceStage::Pursuit {
                    if rec.heat >= 3.0 && self.rng.chance(0.5) {
                        1.0
                    } else {
                        5.0
                    }
                } else {
                    11.0
                };
                let v = &mut sim.vehicles[vi];
                if let Some(d) = v.driver_mut() {
                    d.follow_target = Some(pid);
                    d.follow_gap = gap;
                }
                v.enter_physics();
            } else if !close && follow.is_some() && du > 140.0 {
                if let Some(d) = sim.vehicles[vi].driver_mut() {
                    d.follow_target = None;
                    d.state = DriverState::Recovering;
                }
            }
            let follow = sim.vehicles[vi].driver().and_then(|d| d.follow_target);
            if follow.is_none() && sim.vehicles[vi].mode == MotionMode::Lane && self.route_timer <= 0.0 {
                let hits = sim.net.nearest_lanes(px, py, 25.0);
                if let Some(h) = hits.first() {
                    let lane = sim.net.lane(h.lane);
                    let (road, fwd) = (lane.road, lane.forward);
                    let cur = sim.vehicles[vi].seg;
                    let Sim { net, router, vehicles, .. } = &mut *sim;
                    if let Some(d) = vehicles[vi].driver_mut() {
                        d.route_to(net, router, cur, road, fwd);
                    }
                }
            }
        }
        if self.route_timer <= 0.0 {
            self.route_timer = 3.0;
        }

        match self.stage {
            PoliceStage::Responding => {
                let following = sim.driver(lead).is_some_and(|d| d.follow_target == Some(pid));
                if following {
                    self.stage = PoliceStage::Pullover;
                    self.timer = 0.0;
                    sim.emit(Event::Pullover { police: Some(lead), stage: PulloverStage::Request });
                    let line = self.rng.pick(&POLICE_LINES);
                    sim.emit(Event::Speech { vehicle: lead, text: line.into(), mood: Mood::Calm });
                } else if self.timer > 90.0 {
                    rec.heat = 0.0;
                    self.release(sim);
                }
            }
            PoliceStage::Pullover => {
                if speed < 0.6 && dist < 35.0 {
                    self.still_time += dt;
                } else {
                    self.still_time = 0.0;
                }
                if speed > 13.0 || dist > 100.0 {
                    self.flee_time += dt;
                } else {
                    self.flee_time = (self.flee_time - dt).max(0.0);
                }
                if self.still_time > 2.5 {
                    self.stage = PoliceStage::Stopped;
                    self.timer = 0.0;
                    self.ticket_countdown = 6.0;
                    sim.emit(Event::Pullover { police: Some(lead), stage: PulloverStage::Stopped });
                } else if self.flee_time > 6.0 {
                    self.stage = PoliceStage::Pursuit;
                    self.pursuit_time = 0.0;
                    sim.emit(Event::Pullover { police: Some(lead), stage: PulloverStage::Fled });
                    rec.heat = rec.heat.max(2.0);
                    sim.emit(Event::Heat { level: rec.heat.floor() as u32, reason: "evading" });
                    self.on_flee(sim, rec, spawner);
                } else if self.timer > 8.0 && self.rng.chance(dt * 0.3) {
                    let line = self.rng.pick(&POLICE_LINES);
                    sim.emit(Event::Speech { vehicle: lead, text: line.into(), mood: Mood::Calm });
                }
            }
            PoliceStage::Stopped => {
                self.ticket_countdown -= dt;
                if speed > 3.0 {
                    // Drove off mid-stop.
                    self.stage = PoliceStage::Pursuit;
                    self.pursuit_time = 0.0;
                    rec.heat = rec.heat.max(2.0);
                    sim.emit(Event::Pullover { police: Some(lead), stage: PulloverStage::Fled });
                    self.on_flee(sim, rec, spawner);
                } else if self.ticket_countdown <= 0.0 {
                    let t = rec.issue_ticket();
                    let reason = if t.count == 1 { "1 violation".to_string() } else { format!("{} violations", t.count) };
                    sim.emit(Event::Ticket { vehicle: pid, total: t.total, points: t.points, reason });
                    sim.emit(Event::Pullover { police: Some(lead), stage: PulloverStage::Issued });
                    rec.heat = 0.0;
                    sim.emit(Event::Heat { level: 0, reason: "ticket issued" });
                    self.release(sim);
                }
            }
            PoliceStage::Pursuit | PoliceStage::Searching => {
                self.pursuit_time += dt;
                if self.seen(sim, px, py, 150.0).is_some() {
                    self.lost_time = 0.0;
                    if self.stage == PoliceStage::Searching {
                        self.stage = PoliceStage::Pursuit;
                    }
                } else {
                    self.lost_time += dt;
                    if self.lost_time > 10.0 {
                        self.stage = PoliceStage::Searching;
                    }
                }
                // Heat rises the longer the chase lasts.
                let want = 5f64.min(2.0 + (self.pursuit_time / 45.0).floor());
                if want > rec.heat {
                    rec.heat = want;
                    sim.emit(Event::Heat { level: want as u32, reason: "pursuit continues" });
                }
                if (self.units.len() as f64) < 4f64.min(rec.heat.floor()) && self.rng.chance(dt * 0.2) {
                    self.add_unit(sim, spawner);
                }
                // Arrest: stopped with an officer right there.
                if speed < 0.5 && dist < 16.0 {
                    self.still_time += dt;
                } else {
                    self.still_time = 0.0;
                }
                if self.still_time > 4.0 {
                    let t = rec.issue_ticket();
                    rec.money -= 1000.0;
                    sim.emit(Event::Ticket { vehicle: pid, total: t.total + 1000.0, points: t.points, reason: "arrested: fines + impound fee".into() });
                    sim.emit(Event::Pullover { police: Some(lead), stage: PulloverStage::Issued });
                    rec.heat = 0.0;
                    sim.emit(Event::Heat { level: 0, reason: "arrested" });
                    self.release(sim);
                } else if self.lost_time > 22.0 {
                    sim.emit(Event::Pullover { police: Some(lead), stage: PulloverStage::Escaped });
                    rec.heat = 0.0;
                    sim.emit(Event::Heat { level: 0, reason: "escaped" });
                    self.release(sim);
                }
            }
            PoliceStage::None => {}
        }
    }

    fn on_flee(&mut self, sim: &mut Sim, rec: &mut DrivingRecord, spawner: &Spawner) {
        if let Some((pid, px, py)) = player_pos(sim) {
            let r = rec.add(ViolationKind::Evading, sim.time, Witness::Police, px, py);
            sim.emit(Event::Violation { vehicle: pid, kind: r.kind, label: r.label, fine: r.fine, points: r.points, witnessed: Witness::Police, x: px, y: py });
        }
        for &u in &self.units {
            if let Some(d) = sim.driver_mut(u) {
                if d.follow_target.is_some() {
                    d.follow_gap = 5.0;
                }
            }
        }
        self.add_unit(sim, spawner);
    }
}
