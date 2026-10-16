//! The brain of an AI vehicle.

use super::control::{brake_to_stop, drive_toward};
use super::emotion::{Emotion, Mood, OffenseKind};
use super::idm::{idm_accel, idm_free, IdmParams};
use super::intersection::{approach_stop_distance, connector_stop_distance, pedestrian_stop_distance, JunctionState};
use super::personality::{Archetype, Personality};
use super::speech::line_for;
use crate::ids::{ConnectorId, LaneId, RoadId, SegRef, VehicleId};
use crate::math::scalar::{angle_diff, clamp, fmod, lerp, DEG};
use crate::math::Rng;
use crate::road::{ControlType, Network, OccEntry, RouteStep, Router, TurnType};
use crate::vehicle::{max_accel_at, Indicator, MotionMode, SirenMode, VehicleClass};
use crate::world::events::{Event, EventQueue};
use crate::world::sim::Sim;

/// High-level driver state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DriverState {
    Drive,
    Crashed,
    Recovering,
    Chasing,
}

/// What a raging driver does to its target.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RageBehavior {
    Tailgate,
    BrakeCheck,
    Ram,
    Chase,
}

/// An active road-rage episode.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RageState {
    pub target: VehicleId,
    pub behavior: RageBehavior,
    pub time: f64,
    pub lost_time: f64,
    pub brake_check: f64,
    pub cooldown: f64,
    pub rams: u32,
}

/// Why a driver pulled over on purpose.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HoldKind {
    Breakdown,
    Bus,
}

/// Result of a leader search along the planned path.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Leader {
    pub id: Option<VehicleId>,
    /// Bumper-to-bumper gap (infinity when none).
    pub gap: f64,
    pub speed: f64,
    pub phantom: bool,
    /// Segment and arc length where the leader was found.
    pub seg: Option<SegRef>,
    pub s: f64,
}

impl Default for Leader {
    fn default() -> Self {
        Leader { id: None, gap: f64::INFINITY, speed: 0.0, phantom: false, seg: None, s: 0.0 }
    }
}

struct LaneEval {
    a_new: f64,
    follower_delta: f64,
    follower_acc: f64,
    follower: Option<VehicleId>,
}

/// Brain of an AI vehicle. In lane mode it follows its route with IDM car
/// following, MOBIL-style lane changes and the junction rules; emotions
/// modulate parameters and trigger horn/high-beam use, speech and road-rage
/// episodes; physics mode handles crashes, recovery, chases and police
/// follow/pursuit.
#[derive(Clone, Debug)]
pub struct AIDriver {
    pub vid: VehicleId,
    pub p: Personality,
    pub emo: Emotion,
    pub js: JunctionState,
    pub route: Vec<RouteStep>,
    pub ri: usize,
    /// Connector planned from the current lane.
    pub next: Option<ConnectorId>,
    pub state: DriverState,
    /// Emergency run (siren): treats reds as yields, gets pre-emption.
    pub emergency: bool,
    pub rage: Option<RageState>,
    pub leader: Leader,
    prev_leader_id: Option<VehicleId>,
    prev_leader_gap: f64,
    /// Distance to the current stop target (front bumper), infinity if none.
    pub stop_dist: f64,
    lc_timer: f64,
    lc_intent: Option<LaneId>,
    lc_intent_time: f64,
    /// The lane being left during a lane change (registered as a shadow).
    pub shadow_lane: Option<LaneId>,
    lat_target: f64,
    horn_cooldown: f64,
    horn_hold: f64,
    flash_timer: f64,
    flash_count: u32,
    pub speech_cooldown: f64,
    blocked_time: f64,
    slow_time: f64,
    distraction: f64,
    distract_check: f64,
    react_timer: f64,
    pub yield_emergency: f64,
    /// Mood at the previous step (the world emits mood-change events).
    pub last_mood: Mood,
    last_accel: f64,
    crash_timer: f64,
    recover_lane: Option<LaneId>,
    wander_timer: f64,
    stuck_time: f64,
    rng: Rng,
    /// Police override: follow/pursue this vehicle (physics).
    pub follow_target: Option<VehicleId>,
    pub follow_gap: f64,
    /// Police override: drive (on roads) toward this point.
    pub head_for: Option<(f64, f64)>,
    /// Stopped at the curb on purpose: seconds left once stopped.
    pub hold_time: f64,
    pub hold_kind: Option<HoldKind>,
    bus_stop_lane: Option<LaneId>,
    depart_signal: f64,
    /// Overtaking a stopped vehicle through this oncoming lane.
    pub passing: Option<LaneId>,
    pass_obstacle: Option<VehicleId>,
    pass_wait: f64,
    /// Mirrors of emotion state.
    pub mood: Mood,
    pub raging: bool,
    signal_decided: Option<SegRef>,
    will_signal: bool,
    params: IdmParams,
    stop_params: IdmParams,
}

impl AIDriver {
    pub fn new(vid: VehicleId, p: Personality, seed: i64) -> Self {
        let mut rng = Rng::new(seed);
        let lc_timer = rng.range(0.5, 2.5);
        let distract_check = rng.range(0.0, 1.0);
        AIDriver {
            vid,
            p,
            emo: Emotion::new(p),
            js: JunctionState::default(),
            route: Vec::new(),
            ri: 0,
            next: None,
            state: DriverState::Drive,
            emergency: false,
            rage: None,
            leader: Leader::default(),
            prev_leader_id: None,
            prev_leader_gap: f64::INFINITY,
            stop_dist: f64::INFINITY,
            lc_timer,
            lc_intent: None,
            lc_intent_time: 0.0,
            shadow_lane: None,
            lat_target: 0.0,
            horn_cooldown: 0.0,
            horn_hold: 0.0,
            flash_timer: 0.0,
            flash_count: 0,
            speech_cooldown: 0.0,
            blocked_time: 0.0,
            slow_time: 0.0,
            distraction: 0.0,
            distract_check,
            react_timer: 0.0,
            yield_emergency: 0.0,
            last_mood: Mood::Calm,
            last_accel: 0.0,
            crash_timer: 0.0,
            recover_lane: None,
            wander_timer: 0.0,
            stuck_time: 0.0,
            rng,
            follow_target: None,
            follow_gap: 10.0,
            head_for: None,
            hold_time: 0.0,
            hold_kind: None,
            bus_stop_lane: None,
            depart_signal: 0.0,
            passing: None,
            pass_obstacle: None,
            pass_wait: 0.0,
            mood: Mood::Calm,
            raging: false,
            signal_decided: None,
            will_signal: true,
            params: IdmParams { v0: 10.0, t: 1.4, a: 1.5, b: 2.0, s0: 2.0 },
            stop_params: IdmParams { v0: 10.0, t: 0.2, a: 1.5, b: 2.0, s0: 0.6 },
        }
    }

    fn sync_flags(&mut self) {
        self.mood = self.emo.mood;
        self.raging = self.rage.is_some();
    }

    /// Deterministic per-driver random in [0, 1).
    pub fn roll(&mut self) -> f64 {
        self.rng.next_f64()
    }

    /// Pull over and stop for a while (breakdown or bus stop).
    pub fn hold(&mut self, kind: HoldKind, seconds: f64) {
        self.hold_kind = Some(kind);
        self.hold_time = seconds;
        self.lc_intent = None;
    }

    /// Short description for debug overlays.
    pub fn status(&self, speed: f64) -> String {
        if self.state != DriverState::Drive {
            return match self.state {
                DriverState::Crashed => "crashed",
                DriverState::Recovering => "recovering",
                DriverState::Chasing => "chasing",
                DriverState::Drive => "drive",
            }
            .into();
        }
        if self.hold_time > 0.0 {
            return if self.hold_kind == Some(HoldKind::Bus) { "bus stop" } else { "broken down" }.into();
        }
        if self.passing.is_some() {
            return "overtaking".into();
        }
        if let Some(r) = &self.rage {
            let b = match r.behavior {
                RageBehavior::Tailgate => "tailgate",
                RageBehavior::BrakeCheck => "brake_check",
                RageBehavior::Ram => "ram",
                RageBehavior::Chase => "chase",
            };
            return format!("RAGE:{b}");
        }
        if self.distraction > 0.0 {
            return "distracted".into();
        }
        if self.yield_emergency > 0.0 {
            return "yield-emergency".into();
        }
        if !self.js.reason.is_empty() && self.stop_dist < 40.0 {
            return self.js.reason.into();
        }
        if self.lc_intent.is_some() {
            return "changing lanes".into();
        }
        if speed < 0.3 { "stopped" } else { "driving" }.into()
    }

    // ---------------------------------------------------------------- routing

    fn next_road(&self, idx: usize) -> Option<RouteStep> {
        self.route.get(idx).copied()
    }

    /// Pick a new random destination and route there from `lane`.
    pub fn new_destination(&mut self, net: &Network, router: &Router, lane: LaneId) {
        let lane_road = net.lane(lane).road;
        for _ in 0..6 {
            let goal = &net.lanes[self.rng.index(net.lanes.len())];
            if goal.road == lane_road {
                continue;
            }
            if let Some(r) = router.route(net, lane, goal.road, goal.forward) {
                if !r.is_empty() {
                    self.route = r;
                    self.ri = 0;
                    self.next = self.connector_from(net, lane);
                    return;
                }
            }
        }
        // Wander: no route, choose connectors at random.
        self.route.clear();
        self.ri = 0;
        self.next = self.connector_from(net, lane);
    }

    /// Route to a specific road (police dispatch, ambulances).
    pub fn route_to(&mut self, net: &Network, router: &Router, cur: Option<SegRef>, road: RoadId, forward: bool) -> bool {
        let Some(lane) = current_lane(net, cur) else { return false };
        if net.lane(lane).road == road {
            return true;
        }
        let Some(r) = router.route(net, lane, road, forward) else { return false };
        self.route = r;
        self.ri = 0;
        self.next = self.connector_from(net, lane);
        true
    }

    /// Connector to take out of `lane` according to the route (or a wander choice).
    fn connector_from(&mut self, net: &Network, lane: LaneId) -> Option<ConnectorId> {
        let step = self.next_road(self.ri);
        if let Some(st) = step {
            if let Some(c) = net.connector_to(lane, st.road) {
                return Some(c);
            }
        }
        let l = net.lane(lane);
        if l.outgoing.is_empty() {
            return None;
        }
        if step.is_none() {
            return Some(l.outgoing[self.rng.index(l.outgoing.len())]);
        }
        l.outgoing.iter().copied().find(|&c| net.connector(c).turn == TurnType::Straight).or(Some(l.outgoing[0]))
    }

    /// Lanes of the current road that lead where the route goes.
    fn route_lanes(&self, net: &Network, lane: LaneId) -> Vec<LaneId> {
        let Some(step) = self.next_road(self.ri) else { return Vec::new() };
        net.siblings(lane).iter().copied().filter(|&l| net.connector_to(l, step.road).is_some()).collect()
    }

    // ------------------------------------------------------------- perception

    fn scan_leader(&mut self, sim: &Sim, vi: usize) {
        let net = &sim.net;
        let v = &sim.vehicles[vi];
        let mut l = Leader::default();
        let Some(seg0) = v.seg else {
            self.leader = l;
            return;
        };
        let look = 55f64.max(v.speed * 4.5 + 25.0);
        let mut base = -v.s;
        let mut seg = Some(seg0);
        let mut depth = 0;
        let mut best_d = f64::INFINITY;
        let next_conn = self.next;
        let found = |e: &OccEntry, d: f64, at: SegRef, best_d: &mut f64, l: &mut Leader| {
            if d < *best_d {
                *best_d = d;
                l.id = Some(e.vehicle);
                l.speed = e.v;
                l.phantom = e.phantom;
                l.seg = Some(at);
                l.s = e.s;
            }
        };
        while let Some(sg) = seg {
            if depth >= 5 {
                break;
            }
            for e in sim.occ.seg(sg) {
                if e.vehicle == v.id {
                    continue;
                }
                if depth == 0 && e.s <= v.s + 0.05 {
                    continue;
                }
                if depth == 0 && (e.lat - v.lat).abs() > (e.width + v.width) / 2.0 + 0.25 {
                    continue;
                }
                let d = base + e.s - (e.length + v.length) / 2.0;
                found(e, d, sg, &mut best_d, &mut l);
            }
            if let SegRef::Connector(cid) = sg {
                // Cars just entering a diverging connector still block the shared start.
                let conn = net.connector(cid);
                for (k, &o) in conn.siblings.iter().enumerate() {
                    let diverge = conn.sibling_diverge[k];
                    for e in sim.occ.connector(o) {
                        if e.vehicle == v.id || e.s - e.length / 2.0 > diverge {
                            continue;
                        }
                        if depth == 0 && e.s <= v.s {
                            continue;
                        }
                        let d = base + e.s - (e.length + v.length) / 2.0;
                        found(e, d, SegRef::Connector(o), &mut best_d, &mut l);
                    }
                }
            }
            // Mid lane change: cars in the lane we are leaving still matter.
            if let (0, Some(sh), SegRef::Lane(cur)) = (depth, self.shadow_lane, sg) {
                let shl = net.lane(sh);
                let s_me = (v.s * shl.length) / net.lane(cur).length;
                for e in sim.occ.lane(sh) {
                    if e.vehicle == v.id || e.s <= s_me {
                        continue;
                    }
                    let d = e.s - s_me - (e.length + v.length) / 2.0;
                    found(e, d, SegRef::Lane(sh), &mut best_d, &mut l);
                }
            }
            if best_d < f64::INFINITY {
                break;
            }
            base += net.seg_length(sg);
            if base > look {
                break;
            }
            seg = match sg {
                SegRef::Lane(lid) => {
                    let c = if depth == 0 {
                        next_conn
                    } else if !net.lane(lid).outgoing.is_empty() {
                        self.peek_connector(net, lid, depth, v.seg.is_some_and(|s| s.is_connector()))
                    } else {
                        None
                    };
                    c.map(SegRef::Connector)
                }
                SegRef::Connector(cid) => Some(SegRef::Lane(net.connector(cid).to)),
            };
            depth += 1;
        }
        l.gap = best_d;
        self.leader = l;
    }

    fn peek_connector(&self, net: &Network, lane: LaneId, depth: usize, on_connector: bool) -> Option<ConnectorId> {
        let idx = self.ri + if on_connector { 0 } else { 1 } + (depth - 1) / 2;
        if let Some(step) = self.next_road(idx) {
            if let Some(c) = net.connector_to(lane, step.road) {
                return Some(c);
            }
        }
        let l = net.lane(lane);
        l.outgoing.iter().copied().find(|&c| net.connector(c).turn == TurnType::Straight).or(l.outgoing.first().copied())
    }

    // ----------------------------------------------------------------- update

    /// One simulation step. `vi` is this driver's vehicle index in `sim.vehicles`
    /// (the driver itself has been taken out of the vehicle while it runs).
    pub fn update(&mut self, dt: f64, sim: &mut Sim, vi: usize) {
        self.horn_cooldown -= dt;
        self.speech_cooldown -= dt;
        self.update_horn_and_flash(dt, sim, vi);
        let cong = self.congestion(sim, vi);
        self.emo.update(dt, sim.time, cong);
        self.sync_flags();
        let dark = sim.env.dark;
        let v = &mut sim.vehicles[vi];
        v.lights.dark = dark;
        if v.wrecked {
            self.state = DriverState::Crashed;
            v.lights.hazard = true;
        }
        if v.mode == MotionMode::Physics {
            self.physics_update(dt, sim, vi);
            return;
        }
        self.lane_update(dt, sim, vi);
    }

    fn congestion(&self, sim: &Sim, vi: usize) -> f64 {
        let v = &sim.vehicles[vi];
        let lim = v.seg.map_or(10.0, |s| sim.net.seg_speed_limit(s));
        clamp(1.0 - v.speed / lim.max(1.0), 0.0, 1.0) * if self.leader.gap < 15.0 { 1.0 } else { 0.3 }
    }

    fn lane_update(&mut self, dt: f64, sim: &mut Sim, vi: usize) {
        let Some(seg) = sim.vehicles[vi].seg else { return };
        let p = self.p;

        if let SegRef::Lane(l) = seg {
            if self.next.is_none_or(|c| sim.net.connector(c).from != l) {
                self.next = self.connector_from(&sim.net, l);
            }
        }
        sim.vehicles[vi].intent_connector = match seg {
            SegRef::Lane(_) => self.next,
            SegRef::Connector(c) => Some(c),
        };

        self.scan_leader(sim, vi);
        self.detect_offenses(dt, sim, vi);
        self.check_emergency_behind(dt, sim, vi);
        self.update_rage(dt, sim, vi);
        self.sync_flags();
        if sim.vehicles[vi].mode != MotionMode::Lane {
            return; // rage may have switched us to physics
        }

        // --- Parameters shaped by mood, weather and situation.
        let (v_speed, v_s, v_half_len, v_spec) = {
            let v = &sim.vehicles[vi];
            (v.speed, v.s, v.half_len, v.spec)
        };
        let seg_len = sim.net.seg_length(seg);
        let seg_limit = sim.net.seg_speed_limit(seg);
        let mood = self.emo.mood;
        let mood_speed = match mood {
            Mood::Angry => 1.1,
            Mood::Furious => 1.2,
            Mood::Irritated => 1.04,
            Mood::Calm => 1.0,
        };
        let mood_t = match mood {
            Mood::Angry => 0.7,
            Mood::Furious => 0.55,
            Mood::Irritated => 0.88,
            Mood::Calm => 1.0,
        };
        let wet = sim.env.wetness;
        let vis = sim.env.visibility;
        let weather_speed = (1.0 - wet * 0.15) * clamp(vis / 250.0, 0.55, 1.0);
        self.params.v0 = seg_limit * p.speed_factor * mood_speed * weather_speed;
        if self.emergency {
            self.params.v0 = seg_limit * 1.45;
        }
        self.params.t = p.headway * mood_t * (1.0 + wet * 0.3);
        self.params.a = max_accel_at(v_spec, v_speed).min(2.6) * p.accel_factor;
        self.params.b = p.comfort_decel;
        self.params.s0 = p.min_gap;
        let rage = self.rage;
        if let Some(r) = rage {
            if Some(r.target) == self.leader.id && r.behavior == RageBehavior::Tailgate {
                self.params.t = 0.3;
                self.params.s0 = 1.0;
                self.params.v0 *= 1.3;
            }
        }

        // --- Car following.
        let mut acc = idm_free(v_speed, &self.params);
        if self.leader.gap < f64::INFINITY {
            acc = idm_accel(v_speed, self.leader.gap, v_speed - self.leader.speed, &self.params);
        }

        // --- Curves: slow for the next connector's comfortable speed.
        match (seg, self.next) {
            (SegRef::Lane(_), Some(nc)) => {
                let vc = sim.net.connector(nc).speed_limit * lerp(0.9, 1.15, p.aggression);
                let dc = seg_len - v_s - v_half_len;
                if v_speed > vc {
                    acc = acc.min(-((v_speed * v_speed - vc * vc) / (2.0 * dc.max(2.0))) * 1.15);
                }
            }
            (SegRef::Connector(_), _) => {
                let vc = seg_limit * lerp(0.95, 1.15, p.aggression);
                if v_speed > vc {
                    acc = acc.min(-1.5);
                }
            }
            _ => {}
        }

        // --- Junction rules.
        self.stop_dist = f64::INFINITY;
        match (seg, self.next) {
            (SegRef::Lane(l), Some(nc)) if !self.raging_through_lights() => {
                self.stop_dist = approach_stop_distance(sim, self, vi, l, nc, dt);
                let conn = sim.net.connector(nc);
                let node = sim.net.node(conn.node);
                if self.emergency && node.control == ControlType::Signal && seg_len - v_s < 120.0 {
                    if let Some(sid) = node.signal {
                        let ap = conn.approach;
                        sim.signals[sid.idx()].request_preempt(ap);
                    }
                }
            }
            (SegRef::Connector(c), _) => {
                self.stop_dist = connector_stop_distance(sim, self, vi, c, dt);
            }
            _ => {}
        }
        // Pedestrians on crosswalks along our movement.
        match (seg, self.next) {
            (SegRef::Lane(_), Some(nc)) => {
                let to_line = seg_len - v_s - v_half_len;
                if to_line < 50.0 {
                    self.stop_dist = self.stop_dist.min(pedestrian_stop_distance(sim, vi, nc, to_line, 0.0));
                }
            }
            (SegRef::Connector(c), _) => {
                self.stop_dist = self.stop_dist.min(pedestrian_stop_distance(sim, vi, c, 0.0, v_s + v_half_len));
            }
            _ => {}
        }
        // End of a lane with no exit at all: stop.
        if seg.is_lane() && self.next.is_none() {
            self.stop_dist = self.stop_dist.min(seg_len - v_s - v_half_len);
        }
        if self.stop_dist < f64::INFINITY {
            self.stop_params.a = self.params.a;
            self.stop_params.b = self.params.b;
            self.stop_params.v0 = self.params.v0;
            acc = acc.min(idm_accel(v_speed, self.stop_dist, v_speed, &self.stop_params));
        }

        // --- Bus stops mid-block on the curb lane.
        if v_spec.id == VehicleClass::Bus && self.hold_time <= 0.0 {
            if let SegRef::Lane(l) = seg {
                let lane = sim.net.lane(l);
                if lane.right.is_none() && lane.length > 60.0 && self.bus_stop_lane != Some(l) && v_s > lane.length * 0.45 && v_s < lane.length * 0.6 {
                    self.bus_stop_lane = Some(l);
                    if self.rng.chance(0.7) {
                        let secs = self.rng.range(7.0, 14.0);
                        self.hold(HoldKind::Bus, secs);
                    }
                }
            }
        }
        // --- Holds: brake gently to the curb, then wait.
        if self.hold_time > 0.0 {
            acc = acc.min(if v_speed > 0.3 { -(0.8 + v_speed * 0.25).min(3.0) } else { -1.0 });
            if v_speed < 0.3 {
                self.hold_time -= dt;
            }
            let lights = &mut sim.vehicles[vi].lights;
            if self.hold_kind == Some(HoldKind::Breakdown) {
                lights.hazard = true;
            } else {
                lights.set_indicator(Indicator::Right);
            }
            if self.hold_time <= 0.0 {
                if self.hold_kind == Some(HoldKind::Breakdown) {
                    lights.hazard = false;
                }
                if self.hold_kind == Some(HoldKind::Bus) {
                    self.depart_signal = 2.5;
                }
                self.hold_kind = None;
            }
        }
        if self.depart_signal > 0.0 {
            self.depart_signal -= dt;
            sim.vehicles[vi].lights.set_indicator(if self.depart_signal > 0.0 { Indicator::Left } else { Indicator::Off });
        }
        // --- Overtake stopped vehicles through the oncoming lane.
        if let SegRef::Lane(l) = seg {
            if self.hold_time <= 0.0 {
                self.update_passing(dt, sim, vi, l);
            }
        }
        if self.passing.is_some() {
            self.params.v0 = self.params.v0.min(9.0);
        }

        // --- Lateral targets: curb holds, overtaking, pulling over for emergencies.
        let lane_here = seg.lane().map(|l| sim.net.lane(l));
        if self.hold_time > 0.0 && lane_here.is_some_and(|l| l.right.is_none()) {
            self.lat_target = -(lane_here.unwrap().width * 0.32);
        } else if self.passing.is_some() {
            if let Some(l) = lane_here {
                self.lat_target = l.width + sim.net.road(l.road).median_width;
            }
            if self.leader.gap < 6.0 && self.leader.speed < 1.0 {
                acc = acc.min(0.5);
            }
        } else if self.yield_emergency > 0.0 {
            if let Some(l) = lane_here {
                if l.right.is_none() {
                    self.lat_target = -(l.width * 0.42);
                }
            }
            acc = acc.min(if v_speed > 2.0 { -2.5 } else { -0.5 });
        } else if self.emergency && self.leader.gap < 30.0 && self.leader.speed < 3.0 {
            self.lat_target = 1.2; // pass on the left of yielding traffic
        } else {
            self.lat_target = 0.0;
        }

        // --- Brake check (rage).
        if let Some(r) = rage {
            if r.brake_check > 0.0 {
                acc = acc.min(-7.0);
            }
        }

        // --- Human factors: reaction delay, distraction.
        acc = self.human_factors(dt, acc, v_speed);

        let max_brake = 8.5 * sim.env.grip * v_spec.grip;
        acc = clamp(acc, -max_brake, self.params.a * 1.05);
        self.last_accel = acc;
        self.integrate(dt, acc, sim, vi);
        self.lane_changes(dt, sim, vi);
        self.lateral(dt, sim, vi);
        self.update_lights(dt, sim, vi);
        self.update_waiting(dt, sim, vi);
    }

    /// The innermost lane of the opposite direction, if the road is two-way.
    fn opposite_lane(net: &Network, lane: LaneId) -> Option<LaneId> {
        let l = net.lane(lane);
        let r = net.road(l.road);
        let g = if l.forward { &r.lanes_ba } else { &r.lanes_ab };
        g.first().copied()
    }

    /// No oncoming vehicle within `range` metres of our position on the opposite lane.
    fn oncoming_clear(sim: &Sim, vi: usize, opp: LaneId, range: f64) -> bool {
        let v = &sim.vehicles[vi];
        let ol = sim.net.lane(opp);
        let s_opp = ol.length - v.s;
        for e in sim.occ.lane(opp) {
            if e.vehicle == v.id {
                continue;
            }
            if e.s > s_opp - range && e.s < s_opp + 10.0 {
                return false;
            }
        }
        if s_opp - range < 0.0 {
            for &c in &ol.incoming {
                if !sim.occ.connector(c).is_empty() {
                    return false;
                }
            }
        }
        true
    }

    fn update_passing(&mut self, dt: f64, sim: &mut Sim, vi: usize, lane: LaneId) {
        let (vx, vy, vh, vhl, vs, vlat) = {
            let v = &sim.vehicles[vi];
            (v.x, v.y, v.heading, v.half_len, v.s, v.lat)
        };
        let lane_len = sim.net.lane(lane).length;
        if let Some(passing) = self.passing {
            let clear_of_it = match self.pass_obstacle.and_then(|o| sim.vehicle(o)) {
                None => true,
                Some(obs) => {
                    let along = (obs.x - vx) * vh.cos() + (obs.y - vy) * vh.sin();
                    along < -(obs.half_len + vhl + 4.0)
                }
            };
            let aborting = vlat.abs() < 1.2 && !Self::oncoming_clear(sim, vi, passing, 45.0);
            if clear_of_it || aborting || lane_len - vs < 12.0 || sim.net.lane(passing).road != sim.net.lane(lane).road {
                self.passing = None;
                self.pass_wait = 0.0;
            }
            return;
        }
        let l = self.leader;
        let lead = l.id.and_then(|id| sim.vehicle(id));
        let Some(lead) = lead else {
            self.pass_wait = 0.0;
            return;
        };
        if l.gap > 14.0 || l.speed > 0.3 || sim.net.lane(lane).left.is_some() || l.seg != Some(SegRef::Lane(lane)) {
            self.pass_wait = 0.0;
            return;
        }
        let stuck = lead.wrecked || lead.mode == MotionMode::Physics || lead.driver().is_some_and(|d| d.hold_time > 0.0);
        let lead_id = lead.id;
        let opp = Self::opposite_lane(&sim.net, lane);
        let Some(opp) = opp.filter(|_| stuck && lane_len - vs >= 30.0) else {
            self.pass_wait = 0.0;
            return;
        };
        self.pass_wait += dt;
        let patience = lerp(4.0, 1.5, self.p.aggression);
        if self.pass_wait > patience && Self::oncoming_clear(sim, vi, opp, 90.0) {
            self.passing = Some(opp);
            self.pass_obstacle = Some(lead_id);
            if self.rng.chance(self.p.signal_discipline) {
                sim.vehicles[vi].lights.set_indicator(Indicator::Left);
            }
        }
    }

    fn raging_through_lights(&self) -> bool {
        self.rage.is_some_and(|r| r.behavior == RageBehavior::Tailgate || r.behavior == RageBehavior::Chase) && self.emo.anger > 85.0 && self.p.lawfulness < 0.3
    }

    fn human_factors(&mut self, dt: f64, mut acc: f64, speed: f64) -> f64 {
        let p = self.p;
        // Starting from standstill takes a reaction time (queue discharge wave).
        if speed < 0.15 && acc > 0.2 {
            self.react_timer += dt;
            let reaction = lerp(1.4, 0.45, p.skill) + if self.distraction > 0.0 { 2.5 } else { 0.0 };
            if self.react_timer < reaction {
                acc = 0.0;
            }
        } else if speed > 0.5 {
            self.react_timer = 0.0;
        }
        // Distraction: the driver keeps the old command until a reflex kicks in.
        self.distract_check -= dt;
        if self.distract_check <= 0.0 {
            self.distract_check = 1.0;
            if self.distraction <= 0.0 && self.rng.chance((1.0 - p.attentiveness) * 0.025) {
                self.distraction = self.rng.range(1.2, 3.5);
            }
        }
        if self.distraction > 0.0 {
            self.distraction -= dt;
            let ttc = if self.leader.gap < f64::INFINITY && speed > self.leader.speed { self.leader.gap / (speed - self.leader.speed) } else { f64::INFINITY };
            let stop_ttc = if self.stop_dist < f64::INFINITY && speed > 0.5 { self.stop_dist / speed } else { f64::INFINITY };
            if ttc < 1.3 || stop_ttc < 1.0 {
                self.distraction = 0.0;
                return acc.min(-8.0);
            }
            return self.last_accel.min(acc.max(self.last_accel - 0.2));
        }
        acc
    }

    fn integrate(&mut self, dt: f64, acc: f64, sim: &mut Sim, vi: usize) {
        {
            let v = &mut sim.vehicles[vi];
            let mut sp = v.speed + acc * dt;
            if sp < 0.0 {
                sp = 0.0;
            }
            v.accel = (sp - v.speed) / dt;
            v.speed = sp;
            v.s += sp * dt;
            v.odometer += sp * dt;
            v.wheel_spin += (sp * dt) / v.spec.wheel_radius;
        }
        // Segment transitions.
        let mut guard = 0;
        while let Some(seg) = sim.vehicles[vi].seg {
            let len = sim.net.seg_length(seg);
            if sim.vehicles[vi].s <= len || guard >= 4 {
                break;
            }
            guard += 1;
            let over = sim.vehicles[vi].s - len;
            match seg {
                SegRef::Lane(l) => {
                    let mut c = self.next;
                    if c.is_none_or(|c| sim.net.connector(c).from != l) {
                        c = self.connector_from(&sim.net, l);
                    }
                    let Some(c) = c else {
                        let v = &mut sim.vehicles[vi];
                        v.s = len;
                        v.speed = 0.0;
                        break;
                    };
                    let vid = self.vid;
                    if self.js.queued {
                        let node = sim.net.connector(c).node;
                        if let Some(q) = sim.all_way.get_mut(&node) {
                            q.remove(&vid);
                        }
                    }
                    self.js.queued = false;
                    let v = &mut sim.vehicles[vi];
                    v.prev_seg = Some(seg);
                    v.seg = Some(SegRef::Connector(c));
                    v.s = over;
                    self.shadow_lane = None;
                }
                SegRef::Connector(c) => {
                    let lane = sim.net.connector(c).to;
                    {
                        let v = &mut sim.vehicles[vi];
                        v.prev_seg = Some(seg);
                        v.seg = Some(SegRef::Lane(lane));
                        v.s = over;
                    }
                    let lane_road = sim.net.lane(lane).road;
                    match self.next_road(self.ri) {
                        Some(step) if step.road == lane_road => self.ri += 1,
                        Some(_) => self.route.clear(), // went off-route
                        None => {}
                    }
                    if self.head_for.is_none() && self.ri >= self.route.len() {
                        self.new_destination(&sim.net, &sim.router, lane);
                    }
                    self.next = self.connector_from(&sim.net, lane);
                    self.js.reset(lane);
                    if self.lc_intent.is_some() {
                        self.lc_intent = None;
                        sim.vehicles[vi].lights.set_indicator(Indicator::Off);
                    }
                }
            }
        }
        let net = &sim.net;
        let v = &mut sim.vehicles[vi];
        v.sync_lane_pose(net);
        if let Some(seg) = v.seg {
            v.wheel_angle = clamp(net.seg_path(seg).curvature_at(v.s + 2.0, 2.0) * v.spec.wheelbase * 1.1 + v.lat_vel * 0.05, -0.6, 0.6);
        }
    }

    // ----------------------------------------------------------- lane changes

    fn lane_changes(&mut self, dt: f64, sim: &mut Sim, vi: usize) {
        let Some(SegRef::Lane(lane)) = sim.vehicles[vi].seg else { return };
        let (v_lat, v_s, v_speed) = {
            let v = &sim.vehicles[vi];
            (v.lat, v.s, v.speed)
        };
        if v_lat.abs() > 0.6 {
            return; // still executing one
        }
        let l = sim.net.lane(lane);
        let (l_left, l_right, l_index) = (l.left, l.right, l.index);
        let to_end = l.length - v_s;
        // Keep a pending intent alive: signal first, then go when safe.
        if let Some(intent) = self.lc_intent {
            self.lc_intent_time += dt;
            let signal_time = lerp(0.6, 2.2, self.p.signal_discipline);
            let urgency = clamp(1.0 - (to_end - 20.0) / 80.0, 0.0, 1.0);
            if self.lc_intent_time >= signal_time * (1.0 - urgency * 0.7) {
                let b_safe = lerp(2.5, 5.0, self.p.aggression) + urgency * 3.0 + if self.raging { 2.0 } else { 0.0 };
                if self.try_lane_change(sim, vi, lane, intent, b_safe) {
                    return;
                }
            }
            if self.lc_intent_time > 12.0 {
                self.lc_intent = None;
                sim.vehicles[vi].lights.set_indicator(Indicator::Off);
            }
            return;
        }
        self.lc_timer -= dt;
        if self.lc_timer > 0.0 {
            return;
        }
        self.lc_timer = self.rng.range(0.8, 1.8);
        if to_end < 12.0 || v_s < 4.0 {
            return;
        }
        // Mandatory: get into a lane that has our turn.
        let good = self.route_lanes(&sim.net, lane);
        if !good.is_empty() && !good.contains(&lane) {
            let dir = if sim.net.lane(good[0]).index < l_index { l_left } else { l_right };
            if let Some(d) = dir {
                self.intend(sim, vi, lane, d);
            }
            return;
        }
        // Emergency vehicle behind: move right.
        if self.yield_emergency > 0.0 {
            if let Some(r) = l_right {
                self.intend(sim, vi, lane, r);
                return;
            }
        }
        // Discretionary (MOBIL): only where both lanes still work for the route.
        if v_speed < 1.0 && self.leader.gap > 8.0 {
            return;
        }
        let a_old = if self.leader.gap < f64::INFINITY { idm_accel(v_speed, self.leader.gap, v_speed - self.leader.speed, &self.params) } else { idm_free(v_speed, &self.params) };
        let mut best = None;
        let mut best_gain = 0.0;
        for cand in [l_left, l_right].into_iter().flatten() {
            if !good.is_empty() && !good.contains(&cand) && to_end < 150.0 {
                continue;
            }
            let Some(g) = self.evaluate_lane(sim, vi, lane, cand) else { continue };
            let bias = if Some(cand) == l_right { 0.12 } else { -0.05 }; // keep-right rule
            let politeness = if self.raging { 0.0 } else { self.p.politeness };
            let gain = g.a_new - a_old + politeness * g.follower_delta + bias;
            let threshold = lerp(0.45, 0.1, self.p.aggression);
            if gain > threshold && gain > best_gain {
                best_gain = gain;
                best = Some(cand);
            }
        }
        if let Some(b) = best {
            self.intend(sim, vi, lane, b);
        }
    }

    fn intend(&mut self, sim: &mut Sim, vi: usize, lane: LaneId, target: LaneId) {
        self.lc_intent = Some(target);
        self.lc_intent_time = 0.0;
        let signals = self.rng.chance(self.p.signal_discipline);
        if signals {
            let left = sim.net.lane(lane).left == Some(target);
            sim.vehicles[vi].lights.set_indicator(if left { Indicator::Left } else { Indicator::Right });
        }
    }

    /// Neighbour lane situation at our position.
    fn evaluate_lane(&self, sim: &Sim, vi: usize, lane: LaneId, cand: LaneId) -> Option<LaneEval> {
        let v = &sim.vehicles[vi];
        let cl = sim.net.lane(cand);
        let s_t = (v.s * cl.length) / sim.net.lane(lane).length;
        // (s, v, len) of the leader and (s, v, len, id) of the follower in the target lane.
        let mut lead: Option<(f64, f64, f64)> = None;
        let mut fol: Option<(f64, f64, f64, VehicleId)> = None;
        for e in sim.occ.lane(cand) {
            if e.vehicle == v.id {
                continue;
            }
            if e.s >= s_t {
                if lead.is_none_or(|l| e.s < l.0) {
                    lead = Some((e.s, e.v, e.length));
                }
            } else if fol.is_none_or(|f| e.s > f.0) {
                fol = Some((e.s, e.v, e.length, e.vehicle));
            }
        }
        // Cars still on connectors feeding the target lane count as followers.
        if fol.is_none() && s_t < 30.0 {
            for &c in &cl.incoming {
                let clen = sim.net.connector(c).length;
                for e in sim.occ.connector(c) {
                    if e.vehicle == v.id {
                        continue;
                    }
                    let s = e.s - clen;
                    if fol.is_none_or(|f| s > f.0) {
                        fol = Some((s, e.v, e.length, e.vehicle));
                    }
                }
            }
        }
        let gap_ahead = lead.map_or(f64::INFINITY, |l| l.0 - s_t - (l.2 + v.length) / 2.0);
        let gap_behind = fol.map_or(f64::INFINITY, |f| s_t - f.0 - (f.2 + v.length) / 2.0);
        let min_ahead = 2.5f64.max(v.speed * lerp(0.6, 0.3, self.p.aggression));
        let min_behind = fol.map_or(0.0, |f| 2.0f64.max(f.1 * lerp(0.5, 0.25, self.p.aggression)));
        if gap_ahead < min_ahead || gap_behind < min_behind {
            return None;
        }
        let a_new = match lead {
            None => idm_free(v.speed, &self.params),
            Some(l) => idm_accel(v.speed, gap_ahead, v.speed - l.1, &self.params),
        };
        let mut follower_delta = 0.0;
        let mut follower_acc = 0.0;
        if let Some(f) = fol {
            let fp = IdmParams { v0: cl.speed_limit, t: 1.3, a: 1.5, b: 2.0, s0: 2.0 };
            let before = match lead {
                None => idm_free(f.1, &fp),
                Some(l) => idm_accel(f.1, l.0 - f.0 - (l.2 + f.2) / 2.0, f.1 - l.1, &fp),
            };
            follower_acc = idm_accel(f.1, gap_behind, f.1 - v.speed, &fp);
            follower_delta = follower_acc - before;
        }
        Some(LaneEval { a_new, follower_delta, follower_acc, follower: fol.map(|f| f.3) })
    }

    fn try_lane_change(&mut self, sim: &mut Sim, vi: usize, lane: LaneId, target: LaneId, b_safe: f64) -> bool {
        let (v_s, v_half_len, v_speed) = {
            let v = &sim.vehicles[vi];
            (v.s, v.half_len, v.speed)
        };
        let l = sim.net.lane(lane);
        // Never swerve across at the stop line unless crawling.
        if l.length - v_s - v_half_len < 10.0 && v_speed > 2.0 {
            return false;
        }
        let Some(ev) = self.evaluate_lane(sim, vi, lane, target) else { return false };
        if ev.follower_acc < -b_safe {
            return false;
        }
        let tl = sim.net.lane(target);
        let offset = if Some(target) == l.right { (l.width + tl.width) / 2.0 } else { -(l.width + tl.width) / 2.0 };
        let s_t = (v_s * tl.length) / l.length;
        self.shadow_lane = Some(lane);
        let entry = {
            let v = &mut sim.vehicles[vi];
            v.seg = Some(SegRef::Lane(target));
            v.s = s_t;
            v.lat += offset;
            OccEntry { vehicle: v.id, vi: vi as u32, s: s_t, v: v.speed, phantom: false, lat: v.lat, length: v.length, width: v.width, intent: v.intent_connector }
        };
        // Visible immediately to vehicles updated later this step.
        sim.occ.add(SegRef::Lane(target), entry);
        self.lc_intent = None;
        self.next = self.connector_from(&sim.net, target);
        self.js.reset(target);
        // Forcing the new follower to brake hard is "cutting them off".
        if let Some(f) = ev.follower {
            if ev.follower_acc < -2.2 && sim.vehicle(f).is_some() {
                sim.offend(self.vid, f, OffenseKind::CutOff, clamp(-ev.follower_acc / 3.0, 0.5, 2.0));
            }
        }
        true
    }

    fn lateral(&mut self, dt: f64, sim: &mut Sim, vi: usize) {
        let turn_soon = self.turn_signal_wanted(sim, vi);
        let v = &mut sim.vehicles[vi];
        let err = self.lat_target - v.lat;
        let max_v = lerp(0.9, 1.8, self.p.aggression) * if self.raging { 1.4 } else { 1.0 };
        let want = clamp(err * 1.1, -max_v, max_v);
        v.lat_vel += clamp(want - v.lat_vel, -2.5 * dt, 2.5 * dt);
        v.lat += v.lat_vel * dt;
        if err.abs() < 0.03 && v.lat_vel.abs() < 0.05 {
            v.lat = self.lat_target;
            v.lat_vel = 0.0;
        }
        if self.shadow_lane.is_some() && v.lat.abs() < v.width / 2.0 {
            self.shadow_lane = None;
        }
        // Indicator off once the change is done (if no further intent).
        if self.lc_intent.is_none() && v.lat.abs() < 0.2 && v.seg.is_some_and(|s| s.is_lane()) && turn_soon == Indicator::Off && v.lights.indicator != Indicator::Off && v.lights.indicator_time > 1.0 {
            v.lights.set_indicator(Indicator::Off);
        }
    }

    // ---------------------------------------------------------- lights, horn

    fn turn_signal_wanted(&self, sim: &Sim, vi: usize) -> Indicator {
        let v = &sim.vehicles[vi];
        let Some(seg) = v.seg else { return Indicator::Off };
        let (c, dist) = match seg {
            SegRef::Lane(l) => (self.next, sim.net.lane(l).length - v.s),
            SegRef::Connector(c) => (Some(c), 0.0),
        };
        let Some(c) = c else { return Indicator::Off };
        if dist > 45.0 {
            return Indicator::Off;
        }
        match sim.net.connector(c).turn {
            TurnType::Left | TurnType::UTurn => Indicator::Left,
            TurnType::Right => Indicator::Right,
            TurnType::Straight => Indicator::Off,
        }
    }

    fn update_lights(&mut self, dt: f64, sim: &mut Sim, vi: usize) {
        let want = self.turn_signal_wanted(sim, vi);
        let dark = sim.env.dark;
        let v = &mut sim.vehicles[vi];
        let lights = &mut v.lights;
        lights.brake = if self.last_accel < -0.6 || (v.speed < 0.2 && self.stop_dist < 20.0) || (v.speed < 0.2 && self.leader.gap < 8.0) { 1.0 } else { 0.0 };
        lights.reverse = false;
        if self.state == DriverState::Drive && !v.wrecked {
            lights.hazard = self.hold_kind == Some(HoldKind::Breakdown) || (self.yield_emergency > 0.0 && v.speed < 1.0 && self.p.lawfulness > 0.8);
        }
        if self.lc_intent.is_none() {
            if self.signal_decided != v.seg {
                self.signal_decided = v.seg;
                self.will_signal = self.rng.chance(self.p.signal_discipline);
            }
            if want != Indicator::Off && self.will_signal {
                lights.set_indicator(want);
            } else if want == Indicator::Off && v.lat.abs() < 0.2 && lights.indicator_time > 1.5 && v.seg.is_some_and(|s| s.is_lane()) {
                lights.set_indicator(Indicator::Off);
            }
        }
        // Lane mode has no steering wheel: drive the auto-cancel from curvature.
        lights.update(dt, v.wheel_angle);
        if self.emergency {
            lights.lightbar = true;
            if lights.siren == SirenMode::Off {
                lights.siren = SirenMode::Wail;
            }
        }
        // Night: some drivers forget their lights.
        if dark && lights.headlight_switch == crate::vehicle::HeadlightSwitch::Off && self.rng.chance(0.002) {
            lights.headlight_switch = crate::vehicle::HeadlightSwitch::Auto;
        }
    }

    fn update_horn_and_flash(&mut self, dt: f64, sim: &mut Sim, vi: usize) {
        let l = &mut sim.vehicles[vi].lights;
        if self.horn_hold > 0.0 {
            self.horn_hold -= dt;
            l.horn = true;
            if self.horn_hold <= 0.0 {
                l.horn = false;
            }
        }
        if self.flash_timer > 0.0 {
            self.flash_timer -= dt;
            let phase = fmod(self.flash_timer, 0.5);
            l.flash = phase > 0.25;
            if self.flash_timer <= 0.0 {
                l.flash = false;
            }
        }
    }

    /// Sound the horn (short or long); drivers in front take offense.
    pub fn honk(&mut self, sim: &mut Sim, long: bool) {
        if self.horn_cooldown > 0.0 {
            return;
        }
        self.horn_hold = if long { self.rng.range(1.2, 2.2) } else { self.rng.range(0.25, 0.45) };
        self.horn_cooldown = if long { 4.0 } else { 2.2 };
        sim.emit(Event::Horn { vehicle: self.vid, long });
        sim.horn_victims(self.vid, long);
    }

    /// Flash the high beams at the leader.
    pub fn flash_high_beams(&mut self, sim: &mut Sim) {
        if self.flash_timer > 0.0 {
            return;
        }
        self.flash_timer = 1.0;
        self.flash_count += 1;
        sim.emit(Event::Flash { vehicle: self.vid });
        if let Some(lid) = self.leader.id {
            if self.leader.gap < 40.0 && sim.vehicle(lid).is_some() {
                sim.offend(self.vid, lid, OffenseKind::HighBeams, 1.0);
            }
        }
    }

    /// Say something (rate limited).
    pub fn say(&mut self, events: &mut EventQueue, text: &str) {
        if self.speech_cooldown > 0.0 {
            return;
        }
        self.speech_cooldown = self.rng.range(5.0, 9.0);
        events.emit(Event::Speech { vehicle: self.vid, text: text.to_string(), mood: self.emo.mood });
    }

    /// Say a random line for `mood`/`kind`.
    fn say_line(&mut self, events: &mut EventQueue, mood: Mood, kind: Option<OffenseKind>) {
        let line = line_for(&mut self.rng, mood, kind);
        self.say(events, line);
    }

    /// React to an offense (called after anger was applied).
    pub fn on_offended(&mut self, sim: &mut Sim, _vi: usize, _offender: VehicleId, kind: OffenseKind) {
        let mood = self.emo.mood;
        let horn_chance = self.p.horn_happiness * if mood == Mood::Calm { 0.4 } else { 1.0 };
        if kind != OffenseKind::HonkedAt && kind != OffenseKind::LongHonk && self.rng.chance(horn_chance) {
            self.honk(sim, mood == Mood::Angry || mood == Mood::Furious);
        }
        if (mood != Mood::Calm || kind == OffenseKind::Collision || kind == OffenseKind::NearMiss) && self.rng.chance(0.6) {
            self.say_line(&mut sim.events, mood, Some(kind));
        }
    }

    /// An offense against this driver while its brain is running.
    fn offend_self(&mut self, sim: &mut Sim, vi: usize, offender: VehicleId, kind: OffenseKind, scale: f64) {
        if offender == self.vid {
            return;
        }
        let gain = self.emo.offend(Some(offender), kind, scale, sim.time);
        if !kind.is_continuous() && gain > 0.5 {
            sim.emit(Event::Offense { offender, victim: self.vid, kind, anger: self.emo.anger });
            self.on_offended(sim, vi, offender, kind);
        }
    }

    // ------------------------------------------------------------- offenses

    fn detect_offenses(&mut self, dt: f64, sim: &mut Sim, vi: usize) {
        let v_speed = sim.vehicles[vi].speed;
        let l = self.leader;
        // Phantom (player / free body) cutting in close in front of us.
        if l.id.is_some() && l.id != self.prev_leader_id && l.phantom && v_speed > 4.0 {
            let thw = l.gap / v_speed.max(1.0);
            if thw < 0.75 && l.gap < self.prev_leader_gap - 3.0 {
                if let Some(off) = l.id.filter(|&id| sim.vehicle(id).is_some()) {
                    self.offend_self(sim, vi, off, OffenseKind::CutOff, clamp((0.9 - thw) * 2.5, 0.5, 2.0));
                }
            }
        }
        self.prev_leader_id = l.id;
        self.prev_leader_gap = l.gap;
        // Blocked: stuck behind a stationary vehicle with no good reason.
        let leader_vi = l.id.and_then(|id| sim.vidx(id));
        let v_speed = sim.vehicles[vi].speed;
        if let Some(li) = leader_vi.filter(|_| l.gap < 12.0 && l.speed < 0.3 && v_speed < 0.3) {
            if !self.leader_has_reason(sim, li) {
                self.blocked_time += dt;
                let limit = 2.5 + self.p.patience * 5.0;
                if self.blocked_time > limit {
                    let lid = sim.vehicles[li].id;
                    self.offend_self(sim, vi, lid, OffenseKind::Blocked, dt * 2.0);
                    if self.horn_cooldown <= 0.0 && self.rng.chance(0.4 + self.p.horn_happiness * 0.5) {
                        let long = self.blocked_time > limit * 2.0 || self.emo.mood == Mood::Angry;
                        self.honk(sim, long);
                    }
                }
            } else {
                self.blocked_time = 0.0;
            }
        } else {
            self.blocked_time = 0.0;
        }
        // Slowpoke ahead.
        let v = &sim.vehicles[vi];
        let lim = v.seg.map_or(10.0, |s| sim.net.seg_speed_limit(s));
        let leader_alive = l.id.and_then(|id| sim.vidx(id));
        if let Some(li) = leader_alive.filter(|_| l.gap < 25.0 && l.speed > 1.0 && l.speed < lim * 0.55 && self.stop_dist == f64::INFINITY) {
            self.slow_time += dt;
            if self.slow_time > 6.0 + self.p.patience * 10.0 {
                let lid = sim.vehicles[li].id;
                self.offend_self(sim, vi, lid, OffenseKind::SlowDriver, dt);
            }
        } else {
            self.slow_time = 0.0;
        }
    }

    /// Is a stationary leader stopped for a legitimate reason (red light, queue, sign)?
    fn leader_has_reason(&self, sim: &Sim, li: usize) -> bool {
        let lead = &sim.vehicles[li];
        if let Some(d) = lead.driver() {
            if d.hold_time > 0.0 || d.stop_dist < 8.0 || (d.leader.gap < 8.0 && d.leader.speed < 0.5) {
                return true;
            }
            if d.state != DriverState::Drive {
                return d.state != DriverState::Crashed;
            }
            return d.yield_emergency > 0.0;
        }
        // Player/free body: near a stop line with a red or a sign?
        if let Some(SegRef::Lane(lid)) = self.leader.seg {
            let lane = sim.net.lane(lid);
            let to_line = lane.length - self.leader.s;
            if to_line < 10.0 {
                if let Some(ap) = lane.approach.map(|a| sim.net.approach(a)) {
                    match ap.sign {
                        crate::road::ApproachSign::Stop | crate::road::ApproachSign::Yield => return true,
                        crate::road::ApproachSign::Signal => {
                            if let Some(sid) = sim.net.node(ap.node).signal {
                                let sig = &sim.signals[sid.idx()];
                                if lane.outgoing.iter().any(|&c| sig.indication(sim.net.connector(c)) == crate::signals::Indication::Green) {
                                    return self.blocked_time < 4.0;
                                }
                            }
                            return true;
                        }
                        crate::road::ApproachSign::None => {}
                    }
                }
            }
            // Queued behind someone?
            for e in sim.occ.lane(lid) {
                if e.vehicle != lead.id && e.s > self.leader.s && e.s - self.leader.s < 10.0 && e.v < 0.5 {
                    return true;
                }
            }
        }
        false
    }

    fn check_emergency_behind(&mut self, dt: f64, sim: &Sim, vi: usize) {
        if self.yield_emergency > 0.0 {
            self.yield_emergency -= dt;
        }
        if self.emergency {
            return;
        }
        let v = &sim.vehicles[vi];
        let hc = v.heading.cos();
        let hs = v.heading.sin();
        for e in &sim.vehicles {
            if !e.siren_on() || e.id == v.id || !e.alive {
                continue;
            }
            let dx = e.x - v.x;
            let dy = e.y - v.y;
            if dx * dx + dy * dy > 80.0 * 80.0 {
                continue;
            }
            let behind = dx * hc + dy * hs < -2.0;
            let same_dir = angle_diff(v.heading, e.heading).abs() < 50.0 * DEG;
            if behind && same_dir {
                self.yield_emergency = 2.5;
                return;
            }
        }
    }

    fn update_waiting(&mut self, dt: f64, sim: &mut Sim, vi: usize) {
        if sim.vehicles[vi].speed < 0.2 {
            self.stuck_time += dt;
            if self.stuck_time > 30.0 {
                self.emo.offend(None, OffenseKind::LongWait, dt * 2.0, sim.time);
            }
        } else {
            self.stuck_time = 0.0;
        }
        // Angry drivers lean on the horn at their target.
        if let Some(t) = self.emo.target {
            if (self.emo.mood == Mood::Angry || self.emo.mood == Mood::Furious) && self.leader.id == Some(t) && self.leader.gap < 25.0 {
                if self.rng.chance(dt * 0.4 * (0.4 + self.p.horn_happiness)) {
                    self.honk(sim, true);
                }
                if self.rng.chance(dt * 0.3) {
                    self.flash_high_beams(sim);
                }
            }
        }
    }

    // ------------------------------------------------------------------ rage

    fn update_rage(&mut self, dt: f64, sim: &mut Sim, vi: usize) {
        let (vx, vy) = (sim.vehicles[vi].x, sim.vehicles[vi].y);
        let Some(mut rage) = self.rage else {
            if !self.emo.can_rage() || self.emergency {
                return;
            }
            let Some(target) = self.emo.target else { return };
            let Some(tgt) = sim.vehicle(target).filter(|t| t.alive) else { return };
            let dist = ((tgt.x - vx) * (tgt.x - vx) + (tgt.y - vy) * (tgt.y - vy)).sqrt();
            if dist > 70.0 {
                return;
            }
            if !sim.police_near(vx, vy, 60.0).is_empty() {
                return;
            }
            let target_behind_me = match tgt.driver() {
                Some(td) => td.leader.id == Some(self.vid),
                None => self.is_behind(sim, vi, target),
            };
            let behavior = if target_behind_me {
                RageBehavior::BrakeCheck
            } else if self.leader.id != Some(target) && self.p.aggression > 0.8 {
                RageBehavior::Chase
            } else {
                RageBehavior::Tailgate
            };
            self.rage = Some(RageState { target, behavior, time: 0.0, lost_time: 0.0, brake_check: 0.0, cooldown: 0.0, rams: 0 });
            sim.emit(Event::RageStart { vehicle: self.vid, target, behavior });
            self.say_line(&mut sim.events, Mood::Furious, None);
            if behavior == RageBehavior::Chase {
                self.start_chase(sim, vi);
            }
            return;
        };
        rage.time += dt;
        rage.cooldown -= dt;
        if rage.brake_check > 0.0 {
            rage.brake_check -= dt;
        }
        self.rage = Some(rage);
        let Some(tgt) = sim.vehicle(rage.target).filter(|t| t.alive) else {
            return self.end_rage(sim, "target gone");
        };
        let (tx, ty) = (tgt.x, tgt.y);
        let tgt_leader = tgt.driver().map(|td| (td.leader.id, td.leader.gap));
        let dist = ((tx - vx) * (tx - vx) + (ty - vy) * (ty - vy)).sqrt();
        if dist > 120.0 {
            rage.lost_time += dt;
        } else {
            rage.lost_time = 0.0;
        }
        self.rage = Some(rage);
        if rage.lost_time > 6.0 {
            return self.end_rage(sim, "lost target");
        }
        if self.emo.anger < 35.0 {
            return self.end_rage(sim, "calmed down");
        }
        if rage.time > 50.0 {
            return self.end_rage(sim, "gave up");
        }
        if !sim.police_near(vx, vy, 50.0).is_empty() {
            self.emo.anger = self.emo.anger.min(40.0);
            return self.end_rage(sim, "police nearby");
        }
        let target_behind_me = match tgt_leader {
            Some((lid, gap)) => lid == Some(self.vid) && gap < 18.0,
            None => self.is_behind(sim, vi, rage.target) && dist < 18.0,
        };
        match rage.behavior {
            RageBehavior::Tailgate => {
                if self.leader.id == Some(rage.target) && self.rng.chance(dt * 0.5) {
                    self.flash_high_beams(sim);
                }
                if self.leader.id == Some(rage.target) && self.rng.chance(dt * 0.35) {
                    self.honk(sim, true);
                }
                // Escalate: tailgating for a while makes the worst drivers go for contact.
                if rage.time > 8.0 && self.p.aggression > 0.82 && self.emo.anger > 88.0 && dist < 25.0 {
                    rage.behavior = RageBehavior::Ram;
                    self.rage = Some(rage);
                    sim.emit(Event::RageStart { vehicle: self.vid, target: rage.target, behavior: RageBehavior::Ram });
                    self.start_chase(sim, vi);
                } else if target_behind_me {
                    rage.behavior = RageBehavior::BrakeCheck;
                    self.rage = Some(rage);
                }
            }
            RageBehavior::BrakeCheck => {
                if target_behind_me && rage.cooldown <= 0.0 && sim.vehicles[vi].speed > 5.0 {
                    rage.brake_check = self.rng.range(0.8, 1.4);
                    rage.cooldown = self.rng.range(6.0, 10.0);
                    self.rage = Some(rage);
                    sim.offend(self.vid, rage.target, OffenseKind::BrakeCheck, 1.0);
                    sim.emit(Event::RageStart { vehicle: self.vid, target: rage.target, behavior: RageBehavior::BrakeCheck });
                }
                if !target_behind_me && rage.cooldown <= 0.0 {
                    if let Some(r) = self.rage.as_mut() {
                        r.behavior = RageBehavior::Tailgate;
                    }
                }
            }
            _ => {}
        }
    }

    fn end_rage(&mut self, sim: &mut Sim, reason: &'static str) {
        self.rage = None;
        self.raging = false;
        sim.emit(Event::RageEnd { vehicle: self.vid, reason });
        if self.state == DriverState::Chasing {
            self.state = DriverState::Recovering;
        }
    }

    fn is_behind(&self, sim: &Sim, vi: usize, other: VehicleId) -> bool {
        let v = &sim.vehicles[vi];
        let Some(o) = sim.vehicle(other) else { return false };
        let dx = o.x - v.x;
        let dy = o.y - v.y;
        dx * v.heading.cos() + dy * v.heading.sin() < -3.0 && angle_diff(v.heading, o.heading).abs() < 0.8
    }

    fn start_chase(&mut self, sim: &mut Sim, vi: usize) {
        let v = &mut sim.vehicles[vi];
        if v.mode != MotionMode::Physics {
            v.enter_physics();
        }
        self.state = DriverState::Chasing;
        v.lights.set_indicator(Indicator::Off);
    }

    // --------------------------------------------------------------- physics

    /// Called by the world when this vehicle was hit hard enough to leave its lane.
    pub fn on_crash(&mut self, sim: &mut Sim, vi: usize, severity: f64) {
        if self.state == DriverState::Chasing && severity < 6.0 {
            return;
        }
        self.state = DriverState::Crashed;
        self.crash_timer = self.rng.range(2.5, 6.0) + severity * 0.3;
        sim.vehicles[vi].lights.set_indicator(Indicator::Off);
    }

    fn physics_update(&mut self, dt: f64, sim: &mut Sim, vi: usize) {
        {
            let v = &mut sim.vehicles[vi];
            v.physics_time += dt;
            v.lights.brake = if v.inputs.brake > 0.05 { 1.0 } else { 0.0 };
            v.lights.reverse = v.pt.gear < 0;
            let sa = v.dynamics.steer_angle;
            v.lights.update(dt, sa);
        }
        if self.follow_target.is_some() {
            self.follow_update(sim, vi);
            return;
        }
        match self.state {
            DriverState::Crashed => {
                let v = &mut sim.vehicles[vi];
                brake_to_stop(v, true);
                if v.physics_time > 1.0 {
                    v.lights.hazard = true;
                }
                if v.wrecked {
                    return;
                }
                if v.abs_speed() < 0.5 {
                    self.crash_timer -= dt;
                }
                if self.crash_timer <= 0.0 {
                    if self.emo.can_rage() && self.emo.target.is_some() && self.rage.is_none() {
                        self.update_rage(dt, sim, vi);
                        if self.rage.is_some() {
                            self.state = DriverState::Chasing;
                            return;
                        }
                    }
                    self.state = DriverState::Recovering;
                    self.recover_lane = None;
                }
            }
            DriverState::Chasing => {
                let Some(rage) = self.rage else {
                    self.state = DriverState::Recovering;
                    return;
                };
                self.update_rage(dt, sim, vi);
                let tgt = sim.vehicle(rage.target).map(|t| (t.x, t.y, t.vx(), t.vy(), t.abs_speed()));
                // The episode may have changed behaviour (or ended) during the update.
                let (Some((tx0, ty0, tvx, tvy, t_speed)), Some(rage)) = (tgt, self.rage) else {
                    self.state = DriverState::Recovering;
                    return;
                };
                // Aim slightly ahead of the target; ramming means aiming at its flank.
                let lead = if rage.behavior == RageBehavior::Ram { 0.35 } else { 0.8 };
                let tx = tx0 + tvx * lead;
                let ty = ty0 + tvy * lead;
                let v = &mut sim.vehicles[vi];
                let dist = ((tx - v.x) * (tx - v.x) + (ty - v.y) * (ty - v.y)).sqrt();
                let want = if rage.behavior == RageBehavior::Ram {
                    t_speed + 6.0
                } else if dist > 20.0 {
                    t_speed + 8.0
                } else {
                    t_speed
                };
                drive_toward(v, tx, ty, clamp(want, 6.0, 40.0), true);
                if self.rng.chance(dt * 0.6) {
                    self.honk(sim, true);
                }
                if self.rng.chance(dt * 0.4) {
                    self.flash_high_beams(sim);
                }
                sim.vehicles[vi].lights.hazard = false;
            }
            DriverState::Recovering => self.recover_update(dt, sim, vi),
            DriverState::Drive => self.state = DriverState::Recovering,
        }
    }

    fn recover_update(&mut self, dt: f64, sim: &mut Sim, vi: usize) {
        sim.vehicles[vi].lights.hazard = false;
        let (dx, dy, dyaw) = {
            let d = &sim.vehicles[vi].dynamics;
            (d.x, d.y, d.yaw)
        };
        if self.recover_lane.is_none() || self.wander_timer <= 0.0 {
            self.wander_timer = 2.0;
            let hits = sim.net.nearest_lanes(dx, dy, 30.0);
            let mut best = None;
            let mut best_score = f64::INFINITY;
            for h in &hits {
                let lane = sim.net.lane(h.lane);
                if h.s > lane.length - 15.0 {
                    continue;
                }
                let ang = angle_diff(dyaw, lane.path.heading_at(h.s)).abs();
                let score = h.dist + ang * 12.0;
                if score < best_score {
                    best_score = score;
                    best = Some(h.lane);
                }
            }
            self.recover_lane = best;
            if best.is_none() {
                let v = &mut sim.vehicles[vi];
                brake_to_stop(v, false);
                v.despawn_timer += dt * 20.0;
                return;
            }
        }
        self.wander_timer -= dt;
        let lane_id = self.recover_lane.unwrap();
        let lane = sim.net.lane(lane_id);
        let pr = lane.path.project(dx, dy);
        let target = lane.path.sample(lane.length.min(pr.s + 9.0));
        let heading = lane.path.heading_at(pr.s);
        let lane_len = lane.length;
        let v = &mut sim.vehicles[vi];
        drive_toward(v, target.x, target.y, 6.0, true);
        let herr = angle_diff(dyaw, heading).abs();
        if pr.d2.sqrt() < 0.9 && herr < 0.3 && v.dynamics.u > 0.5 && pr.s < lane_len - 5.0 {
            v.enter_lane(&sim.net, SegRef::Lane(lane_id), pr.s, pr.lateral);
            v.lat_vel = 0.0;
            self.lat_target = 0.0;
            self.state = DriverState::Drive;
            self.new_destination(&sim.net, &sim.router, lane_id);
            self.js.reset(lane_id);
        }
        let v = &mut sim.vehicles[vi];
        if v.physics_time > 90.0 {
            v.despawn_timer += dt;
        }
    }

    /// Police: follow/pursue a vehicle in physics mode, keeping `follow_gap`.
    fn follow_update(&mut self, sim: &mut Sim, vi: usize) {
        let Some(tgt) = self.follow_target.and_then(|t| sim.vehicle(t)) else {
            self.follow_target = None;
            self.state = DriverState::Recovering;
            return;
        };
        let (tx, ty, th, tvx, tvy, t_speed) = (tgt.x, tgt.y, tgt.heading, tgt.vx(), tgt.vy(), tgt.abs_speed());
        let v = &mut sim.vehicles[vi];
        let dx = tx - v.x;
        let dy = ty - v.y;
        let dist = (dx * dx + dy * dy).sqrt();
        let gap_err = dist - self.follow_gap;
        let mut want = t_speed + clamp(gap_err * 0.6, -t_speed - 5.0, 25.0);
        if self.follow_gap < 2.0 {
            want = t_speed + 10.0; // ram/PIT
        }
        if dist < self.follow_gap && t_speed < 0.5 {
            brake_to_stop(v, false);
            return;
        }
        let back = if self.follow_gap > 2.0 { 0.0 } else { -0.5 };
        let ax = tx - th.cos() * back + tvx * 0.4;
        let ay = ty - th.sin() * back + tvy * 0.4;
        drive_toward(v, ax, ay, clamp(want, 0.0, 55.0), true);
    }

    /// Archetype of this driver.
    pub fn archetype(&self) -> Archetype {
        self.p.archetype
    }
}

/// The lane a segment belongs to (a connector's exit lane).
pub fn current_lane(net: &Network, seg: Option<SegRef>) -> Option<LaneId> {
    match seg? {
        SegRef::Lane(l) => Some(l),
        SegRef::Connector(c) => Some(net.connector(c).to),
    }
}
