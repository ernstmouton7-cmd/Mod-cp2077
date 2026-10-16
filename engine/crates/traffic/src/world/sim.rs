//! Shared simulation state: what AI, law and world systems read and modify.
//!
//! This replaces the TypeScript `SimContext`. The [`World`](crate::World)
//! owns one `Sim` plus the player-facing systems (input, monitor, police...),
//! so those systems can borrow the `Sim` mutably and themselves disjointly.

use super::environment::Environment;
use super::events::{Event, EventQueue};
use crate::ai::driver::AIDriver;
use crate::ai::emotion::OffenseKind;
use crate::ai::pedestrians::PedestrianSystem;
use crate::ids::{ConnectorId, LaneId, NodeId, SegRef, VehicleId};
use crate::math::scalar::hypot;
use crate::math::Rng;
use crate::physics::BoxShape;
use crate::road::{CrosswalkCrossing, Network, OccEntry, Occupancy, Router};
use crate::signals::{Indication, SignalController};
use crate::util::SpatialHash;
use crate::vehicle::{MotionMode, Vehicle, VehicleRole};
use std::collections::HashMap;

/// World-level counters.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WorldStats {
    pub collisions: u32,
    pub rage_episodes: u32,
    pub horns: u32,
    pub red_light_runners: u32,
    pub spawned: u32,
}

/// Shared simulation state (network, signals, vehicles with their drivers,
/// occupancy, events, RNG, environment, pedestrians, obstacles).
pub struct Sim {
    pub time: f64,
    pub net: Network,
    pub router: Router,
    /// Indexed by `SignalId`.
    pub signals: Vec<SignalController>,
    /// Sorted by id (ids only grow). AI drivers live inside their vehicle.
    pub vehicles: Vec<Vehicle>,
    pub occ: Occupancy,
    pub events: EventQueue,
    pub rng: Rng,
    pub env: Environment,
    pub player: Option<VehicleId>,
    /// All-way stop arrivals: node → (vehicle → arrival time).
    pub all_way: HashMap<NodeId, HashMap<VehicleId, f64>>,
    pub peds: PedestrianSystem,
    /// Crosswalk crossings of each connector (indexed by connector id).
    pub crossings: Vec<Vec<CrosswalkCrossing>>,
    /// Static obstacles (buildings, boundary walls).
    pub obstacles: Vec<BoxShape>,
    pub(crate) obstacle_hash: SpatialHash<u32>,
    pub stats: WorldStats,
    pub(crate) next_id: u32,
    deferred: Vec<(VehicleId, VehicleId, OffenseKind, f64)>,
    offend_depth: u32,
    flushing: bool,
}

impl Sim {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        net: Network,
        router: Router,
        rng: Rng,
        env: Environment,
        peds: PedestrianSystem,
        crossings: Vec<Vec<CrosswalkCrossing>>,
        obstacles: Vec<BoxShape>,
        obstacle_hash: SpatialHash<u32>,
    ) -> Self {
        let occ = Occupancy::new(net.lanes.len(), net.connectors.len());
        Sim {
            time: 0.0,
            net,
            router,
            signals: Vec::new(),
            vehicles: Vec::new(),
            occ,
            events: EventQueue::default(),
            rng,
            env,
            player: None,
            all_way: HashMap::new(),
            peds,
            crossings,
            obstacles,
            obstacle_hash,
            stats: WorldStats::default(),
            next_id: 1,
            deferred: Vec::new(),
            offend_depth: 0,
            flushing: false,
        }
    }

    // ------------------------------------------------------------ lookups

    /// Index of a vehicle in `vehicles`.
    #[inline]
    pub fn vidx(&self, id: VehicleId) -> Option<usize> {
        self.vehicles.binary_search_by_key(&id, |v| v.id).ok()
    }

    #[inline]
    pub fn vehicle(&self, id: VehicleId) -> Option<&Vehicle> {
        self.vidx(id).map(|i| &self.vehicles[i])
    }

    #[inline]
    pub fn vehicle_mut(&mut self, id: VehicleId) -> Option<&mut Vehicle> {
        self.vidx(id).map(move |i| &mut self.vehicles[i])
    }

    /// The AI driver of a vehicle.
    pub fn driver(&self, id: VehicleId) -> Option<&AIDriver> {
        self.vehicle(id).and_then(|v| v.driver())
    }

    pub fn driver_mut(&mut self, id: VehicleId) -> Option<&mut AIDriver> {
        self.vehicle_mut(id).and_then(|v| v.driver_mut())
    }

    /// The player's vehicle index.
    pub fn player_idx(&self) -> Option<usize> {
        self.player.and_then(|p| self.vidx(p))
    }

    /// Live intended connector of an occupancy entry's vehicle.
    #[inline]
    pub fn intent_of(&self, e: &OccEntry) -> Option<ConnectorId> {
        match self.vehicles.get(e.vi as usize) {
            Some(v) if v.id == e.vehicle => v.intent_connector,
            _ => match self.vidx(e.vehicle) {
                Some(i) => self.vehicles[i].intent_connector,
                None => e.intent,
            },
        }
    }

    /// Signal indication for a connector (None when the node is not signalised).
    pub fn indication_for(&self, c: ConnectorId) -> Option<Indication> {
        let conn = self.net.connector(c);
        let n = self.net.node(conn.node);
        if n.control != crate::road::ControlType::Signal {
            return None;
        }
        n.signal.map(|s| self.signals[s.idx()].indication(conn))
    }

    /// Arrival queue of an all-way stop.
    pub fn all_way_mut(&mut self, node: NodeId) -> &mut HashMap<VehicleId, f64> {
        self.all_way.entry(node).or_default()
    }

    /// Police units within `range` of a point that can see it (in id order).
    pub fn police_near(&self, x: f64, y: f64, range: f64) -> Vec<VehicleId> {
        let mut out = Vec::new();
        for v in &self.vehicles {
            if v.role != VehicleRole::Police || !v.alive || v.wrecked {
                continue;
            }
            let dx = v.x - x;
            let dy = v.y - y;
            if dx * dx + dy * dy > range * range {
                continue;
            }
            if self.los(v.x, v.y, x, y) {
                out.push(v.id);
            }
        }
        out
    }

    /// Unobstructed line of sight (buildings block).
    pub fn los(&self, ax: f64, ay: f64, bx: f64, by: f64) -> bool {
        let mx = (ax + bx) / 2.0;
        let my = (ay + by) / 2.0;
        let r = hypot(bx - ax, by - ay) / 2.0 + 1.0;
        let mut buf = Vec::new();
        self.obstacle_hash.query(mx, my, r, &mut buf);
        for oi in buf {
            let o = &self.obstacles[oi as usize];
            let min_x = o.x - o.hl + 0.5;
            let max_x = o.x + o.hl - 0.5;
            let min_y = o.y - o.hw + 0.5;
            let max_y = o.y + o.hw - 0.5;
            let mut t0: f64 = 0.0;
            let mut t1: f64 = 1.0;
            let dx = bx - ax;
            let dy = by - ay;
            let mut hit = true;
            if dx.abs() < 1e-9 {
                if ax < min_x || ax > max_x {
                    hit = false;
                }
            } else {
                let mut ta = (min_x - ax) / dx;
                let mut tb = (max_x - ax) / dx;
                if ta > tb {
                    std::mem::swap(&mut ta, &mut tb);
                }
                t0 = t0.max(ta);
                t1 = t1.min(tb);
                if t0 > t1 {
                    hit = false;
                }
            }
            if hit {
                if dy.abs() < 1e-9 {
                    if ay < min_y || ay > max_y {
                        hit = false;
                    }
                } else {
                    let mut ta = (min_y - ay) / dy;
                    let mut tb = (max_y - ay) / dy;
                    if ta > tb {
                        std::mem::swap(&mut ta, &mut tb);
                    }
                    t0 = t0.max(ta);
                    t1 = t1.min(tb);
                    if t0 > t1 {
                        hit = false;
                    }
                }
            }
            if hit {
                return false;
            }
        }
        true
    }

    /// Is a lane position free of other vehicles within `clear` metres?
    pub fn lane_free(&self, lane: LaneId, s: f64, clear: f64) -> bool {
        if self.occ.lane(lane).iter().any(|e| (e.s - s).abs() < clear) {
            return false;
        }
        !self
            .vehicles
            .iter()
            .any(|v| v.seg == Some(SegRef::Lane(lane)) && (v.s - s).abs() < clear)
    }

    #[inline]
    pub fn emit(&mut self, e: Event) {
        self.events.emit(e);
    }

    // ------------------------------------------------------------ drivers

    /// Run `f` with the driver of the vehicle at index `vi` taken out of its
    /// vehicle (so it can use the whole `Sim`). Returns None if there is no
    /// driver (player, or a driver already in use further up the stack).
    pub fn with_driver<R>(
        &mut self,
        vi: usize,
        f: impl FnOnce(&mut AIDriver, &mut Sim) -> R,
    ) -> Option<R> {
        let mut d = self.vehicles[vi].driver.take()?;
        let r = f(&mut d, self);
        self.vehicles[vi].driver = Some(d);
        Some(r)
    }

    /// Route an offense to the victim's emotions; non-continuous offenses
    /// emit an `Offense` event and trigger the victim's reaction.
    pub fn offend(
        &mut self,
        offender: VehicleId,
        victim: VehicleId,
        kind: OffenseKind,
        scale: f64,
    ) {
        if offender == victim {
            return;
        }
        let Some(vi) = self.vidx(victim) else { return };
        if !self.vehicles[vi].has_ai {
            return;
        }
        let Some(mut d) = self.vehicles[vi].driver.take() else {
            // The victim's brain is running further up the stack: apply when it returns.
            self.deferred.push((offender, victim, kind, scale));
            return;
        };
        self.offend_depth += 1;
        let gain = d.emo.offend(Some(offender), kind, scale, self.time);
        if !kind.is_continuous() && gain > 0.5 {
            self.events.emit(Event::Offense {
                offender,
                victim,
                kind,
                anger: d.emo.anger,
            });
            d.on_offended(self, vi, offender, kind);
        }
        self.vehicles[vi].driver = Some(d);
        self.offend_depth -= 1;
        if self.offend_depth == 0 {
            self.flush_deferred();
        }
    }

    /// Apply offenses that were deferred because their victim was busy.
    pub(crate) fn flush_deferred(&mut self) {
        if self.flushing {
            return;
        }
        self.flushing = true;
        for _ in 0..4 {
            if self.deferred.is_empty() {
                break;
            }
            let pending = std::mem::take(&mut self.deferred);
            let mut kept = Vec::new();
            let mut applied = false;
            for (o, v, k, s) in pending {
                match self.vidx(v) {
                    Some(vi) if self.vehicles[vi].driver.is_some() => {
                        self.offend(o, v, k, s);
                        applied = true;
                    }
                    Some(_) => kept.push((o, v, k, s)),
                    None => {}
                }
            }
            self.deferred.extend(kept);
            if !applied {
                break;
            }
        }
        self.flushing = false;
    }

    /// Anger vehicles in front of a honking car.
    pub fn horn_victims(&mut self, honker: VehicleId, long: bool) {
        let Some(h) = self.vehicle(honker) else {
            return;
        };
        let (hx, hy) = (h.x, h.y);
        let hc = h.heading.cos();
        let hs = h.heading.sin();
        let victims: Vec<VehicleId> = self
            .vehicles
            .iter()
            .filter(|o| {
                if o.id == honker || !o.alive {
                    return false;
                }
                let dx = o.x - hx;
                let dy = o.y - hy;
                let fwd = dx * hc + dy * hs;
                if !(0.0..=30.0).contains(&fwd) {
                    return false;
                }
                let side = (-dx * hs + dy * hc).abs();
                side <= 4.0 + fwd * 0.2
            })
            .map(|o| o.id)
            .collect();
        let kind = if long {
            OffenseKind::LongHonk
        } else {
            OffenseKind::HonkedAt
        };
        for o in victims {
            self.offend(honker, o, kind, 1.0);
        }
    }

    // ------------------------------------------------------------ population

    pub(crate) fn next_vehicle_id(&mut self) -> VehicleId {
        let id = VehicleId(self.next_id);
        self.next_id += 1;
        id
    }

    /// Add an AI vehicle (already placed on a lane) with its driver.
    pub(crate) fn add_vehicle(&mut self, mut v: Vehicle, mut d: AIDriver) {
        let lane = match v.seg {
            Some(SegRef::Lane(l)) => l,
            Some(SegRef::Connector(c)) => self.net.connector(c).to,
            None => LaneId(0),
        };
        d.new_destination(&self.net, &self.router, lane);
        v.has_ai = true;
        v.driver = Some(Box::new(d));
        let id = v.id;
        self.insert_vehicle(v);
        self.stats.spawned += 1;
        self.events.emit(Event::Spawn { vehicle: id });
    }

    /// Insert keeping `vehicles` sorted by id.
    pub(crate) fn insert_vehicle(&mut self, v: Vehicle) {
        match self.vehicles.binary_search_by_key(&v.id, |x| x.id) {
            Ok(i) => self.vehicles[i] = v,
            Err(i) => self.vehicles.insert(i, v),
        }
    }

    /// Remove a vehicle from the world.
    pub fn remove_vehicle(&mut self, id: VehicleId) -> Option<Vehicle> {
        let i = self.vidx(id)?;
        let mut v = self.vehicles.remove(i);
        v.alive = false;
        for q in self.all_way.values_mut() {
            q.remove(&id);
        }
        self.events.emit(Event::Despawn { vehicle: id });
        Some(v)
    }

    /// Lane-mode vehicles and their tails/shadows, plus free bodies projected
    /// onto every lane and connector they overlap (phantom occupants).
    pub fn rebuild_occupancy(&mut self) {
        self.occ.clear();
        for i in 0..self.vehicles.len() {
            let v = &self.vehicles[i];
            if !v.alive {
                continue;
            }
            let entry = |s: f64, sp: f64, phantom: bool, lat: f64| OccEntry {
                vehicle: v.id,
                vi: i as u32,
                s,
                v: sp,
                phantom,
                lat,
                length: v.length,
                width: v.width,
                intent: v.intent_connector,
            };
            if let (MotionMode::Lane, Some(seg)) = (v.mode, v.seg) {
                self.occ.add(seg, entry(v.s, v.speed, false, v.lat));
                if let Some(prev) = v.prev_seg {
                    if v.s < v.half_len + 0.5 {
                        self.occ.add(
                            prev,
                            entry(self.net.seg_length(prev) + v.s, v.speed, false, v.lat),
                        );
                    }
                }
                if let (Some(d), SegRef::Lane(cur)) = (v.driver(), seg) {
                    // Overtaking in the oncoming lane: oncoming drivers must see us.
                    if let Some(opp) = d.passing {
                        if v.lat.abs() > 1.2 {
                            let ol = self.net.lane(opp).length;
                            self.occ
                                .add(SegRef::Lane(opp), entry(ol - v.s, -v.speed, true, 0.0));
                        }
                    }
                    if let Some(sh) = d.shadow_lane {
                        let shl = self.net.lane(sh);
                        let cl = self.net.lane(cur);
                        let s_s = (v.s * shl.length) / cl.length;
                        let offset = if v.lat > 0.0 {
                            v.lat - (shl.width + cl.width) / 2.0
                        } else {
                            v.lat + (shl.width + cl.width) / 2.0
                        };
                        self.occ
                            .add(SegRef::Lane(sh), entry(s_s, v.speed, true, offset));
                    }
                }
            } else {
                self.register_free_body(i);
            }
        }
    }

    fn register_free_body(&mut self, i: usize) {
        let v = &self.vehicles[i];
        let hits = self.net.nearest_lanes(v.x, v.y, 7.0);
        let (vx, vy) = (v.vx(), v.vy());
        let entry = |s: f64, sp: f64, lat: f64| OccEntry {
            vehicle: v.id,
            vi: i as u32,
            s,
            v: sp,
            phantom: true,
            lat,
            length: v.length,
            width: v.width,
            intent: v.intent_connector,
        };
        let mut adds: Vec<(SegRef, OccEntry)> = Vec::new();
        for h in &hits {
            let lane = self.net.lane(h.lane);
            let lh = lane.path.heading_at(h.s);
            let rel = (v.heading - lh).sin().abs();
            let half_across = v.half_wid * (1.0 - rel) + v.half_len * rel;
            if h.lateral.abs() > lane.width / 2.0 + half_across - 0.3 {
                continue;
            }
            if h.s < -v.half_len || h.s > lane.length + v.half_len {
                continue;
            }
            let v_along = vx * lh.cos() + vy * lh.sin();
            adds.push((SegRef::Lane(h.lane), entry(h.s, v_along, h.lateral)));
        }
        let mut conns = Vec::new();
        self.net.nearest_connectors(v.x, v.y, 10.0, &mut conns);
        for c in conns {
            let path = &self.net.connector(c).path;
            let pr = path.project(v.x, v.y);
            if pr.d2 > 2.6 * 2.6 {
                continue;
            }
            if pr.s <= 0.01 && pr.d2 > 1.0 {
                continue;
            }
            let lh = path.heading_at(pr.s);
            let v_along = vx * lh.cos() + vy * lh.sin();
            adds.push((SegRef::Connector(c), entry(pr.s, v_along, pr.lateral)));
        }
        for (s, e) in adds {
            self.occ.add(s, e);
        }
    }
}
