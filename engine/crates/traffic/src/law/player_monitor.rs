//! Watches the player's vehicle against the road network and rules.

use super::violations::ViolationKind;
use crate::ids::{ConnectorId, LaneId, NodeId, VehicleId};
use crate::math::scalar::{angle_diff, DEG};
use crate::road::{ApproachSign, ControlType, LaneHit, TurnType};
use crate::signals::{HeadState, Indication};
use crate::vehicle::Indicator;
use crate::world::sim::Sim;
use std::collections::HashMap;

/// Control at the next junction ahead.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AheadControl {
    Signal,
    Stop,
    Yield,
    None,
}

/// The next controlled junction ahead of the player.
#[derive(Clone, Debug, PartialEq)]
pub struct AheadInfo {
    pub distance: f64,
    pub control: AheadControl,
    pub head: Option<HeadState>,
    /// Indication for the movement implied by the player's indicator.
    pub indication: Option<Indication>,
    pub node_name: String,
    pub camera: bool,
}

/// What the HUD shows about the road around the player.
#[derive(Clone, Debug, PartialEq)]
pub struct RoadInfo {
    pub road_name: String,
    pub speed_limit: f64,
    pub on_road: bool,
    pub wrong_way: bool,
    pub in_junction: bool,
    pub lane: Option<LaneId>,
    pub ahead: Option<AheadInfo>,
    /// Gap and time headway to the vehicle ahead.
    pub leader_gap: f64,
    pub headway: f64,
}

/// A raw rule breach; the world decides who witnessed it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Observation {
    pub kind: ViolationKind,
    /// Node with a red-light camera (camera witness).
    pub camera_node: Option<NodeId>,
    pub speed_camera: bool,
}

impl Observation {
    pub fn new(kind: ViolationKind) -> Self {
        Observation {
            kind,
            camera_node: None,
            speed_camera: false,
        }
    }
}

/// Collision bookkeeping for hit-and-run.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HitRun {
    pub x: f64,
    pub y: f64,
    pub time: f64,
    pub stopped_for: f64,
    pub victim: Option<VehicleId>,
}

/// Watches the player: lane and direction, wrong way, sidewalk, lane changes
/// and turns without signalling, red lights and stop signs (evaluated per
/// junction traversal), speeding, lights, seatbelt, horn abuse, tailgating,
/// reckless sliding, failing to yield to emergency vehicles and hit-and-run.
#[derive(Clone, Debug)]
pub struct PlayerMonitor {
    pub info: RoadInfo,
    last_lane: Option<LaneId>,
    wrong_way_time: f64,
    off_road_time: f64,
    speeding_time: f64,
    dark_no_lights_time: f64,
    unbelted_time: f64,
    tailgate_time: f64,
    skid_time: f64,
    emergency_behind_time: f64,
    horn_held: f64,
    box_stop_time: f64,
    entry_lane: Option<LaneId>,
    entry_snapshot: HashMap<ConnectorId, Indication>,
    entry_node: Option<NodeId>,
    min_speed_near_line: f64,
    stopped_near_line: bool,
    signal_left_at: f64,
    signal_right_at: f64,
    hazard_at: f64,
    pub hit_run: Option<HitRun>,
    pub seatbelt: bool,
}

impl Default for PlayerMonitor {
    fn default() -> Self {
        PlayerMonitor {
            info: RoadInfo {
                road_name: String::new(),
                speed_limit: 13.9,
                on_road: true,
                wrong_way: false,
                in_junction: false,
                lane: None,
                ahead: None,
                leader_gap: f64::INFINITY,
                headway: f64::INFINITY,
            },
            last_lane: None,
            wrong_way_time: 0.0,
            off_road_time: 0.0,
            speeding_time: 0.0,
            dark_no_lights_time: 0.0,
            unbelted_time: 0.0,
            tailgate_time: 0.0,
            skid_time: 0.0,
            emergency_behind_time: 0.0,
            horn_held: 0.0,
            box_stop_time: 0.0,
            entry_lane: None,
            entry_snapshot: HashMap::new(),
            entry_node: None,
            min_speed_near_line: f64::INFINITY,
            stopped_near_line: false,
            signal_left_at: -100.0,
            signal_right_at: -100.0,
            hazard_at: -100.0,
            hit_run: None,
            seatbelt: true,
        }
    }
}

impl PlayerMonitor {
    pub fn new() -> Self {
        Self::default()
    }

    /// Observe the player (vehicle index `pi`) for one step.
    pub fn update(&mut self, dt: f64, sim: &Sim, pi: usize, out: &mut Vec<Observation>) {
        let p = &sim.vehicles[pi];
        let net = &sim.net;
        let now = sim.time;
        let speed = p.abs_speed();
        let lights = &p.lights;
        if lights.indicator == Indicator::Left || lights.hazard {
            self.signal_left_at = now;
        }
        if lights.indicator == Indicator::Right || lights.hazard {
            self.signal_right_at = now;
        }
        if lights.hazard {
            self.hazard_at = now;
        }

        // --- Which lane are we in?
        let hits = net.nearest_lanes(p.x, p.y, 7.0);
        let mut aligned: Option<LaneHit> = None;
        let mut opposite: Option<LaneHit> = None;
        for h in &hits {
            let lane = net.lane(h.lane);
            let half = lane.width / 2.0 + 0.4;
            if h.lateral.abs() > half || h.s <= 0.01 || h.s >= lane.length - 0.01 {
                continue;
            }
            let d = angle_diff(p.heading, lane.path.heading_at(h.s)).abs();
            if d < 70.0 * DEG && aligned.is_none() {
                aligned = Some(*h);
            }
            if d > 110.0 * DEG && opposite.is_none() {
                opposite = Some(*h);
            }
        }
        let node = net.node_at(p.x, p.y);
        self.info.in_junction = node.is_some();
        let lane = aligned.or(opposite).map(|h| h.lane);
        self.info.lane = aligned.map(|h| h.lane);
        let near_road = hits
            .first()
            .is_some_and(|h| h.dist < net.lane(h.lane).width / 2.0 + 1.3);
        self.info.on_road = lane.is_some() || node.is_some() || near_road;
        if let Some(l) = lane {
            let ln = net.lane(l);
            self.info.road_name = net.road(ln.road).name.clone();
            self.info.speed_limit = ln.speed_limit;
        }
        // Wrong way: only the opposite direction here while moving forward.
        let wrong = aligned.is_none() && opposite.is_some() && p.speed > 2.0 && node.is_none();
        self.info.wrong_way = wrong;
        if wrong {
            self.wrong_way_time += dt;
            if self.wrong_way_time > 1.5 {
                out.push(Observation::new(ViolationKind::WrongWay));
            }
        } else {
            self.wrong_way_time = 0.0;
        }
        // Sidewalk / off-road driving.
        if !self.info.on_road && speed > 4.0 {
            self.off_road_time += dt;
            if self.off_road_time > 1.0 {
                out.push(Observation::new(ViolationKind::Sidewalk));
            }
        } else {
            self.off_road_time = 0.0;
        }

        // --- Lane changes without signalling.
        let cur = aligned.map(|h| h.lane);
        if let (Some(c), Some(last)) = (cur, self.last_lane) {
            let cl = net.lane(c);
            let ll = net.lane(last);
            if c != last && cl.road == ll.road && cl.forward == ll.forward && speed > 3.0 {
                let to_left = cl.index < ll.index;
                let t = if to_left {
                    self.signal_left_at
                } else {
                    self.signal_right_at
                };
                if now - t > 3.0 {
                    out.push(Observation::new(ViolationKind::NoSignalLane));
                }
            }
        }

        // --- Junction traversal.
        if let (Some(c), Some(ah)) = (cur, aligned) {
            let cl = net.lane(c);
            let to_line = cl.length - ah.s;
            if to_line < 18.0 {
                self.min_speed_near_line = self.min_speed_near_line.min(speed);
                if speed < 0.4 && to_line < 7.0 {
                    self.stopped_near_line = true;
                }
            }
            let last_road = self.last_lane.map(|l| net.lane(l).road);
            if Some(c) != self.last_lane && Some(cl.road) != last_road {
                // Left a junction into a new road: evaluate the movement.
                if self.entry_lane.is_some() && self.entry_node.is_some_and(|n| cl.from == n) {
                    self.evaluate_movement(sim, c, out);
                }
                self.entry_lane = None;
                self.entry_node = None;
                self.min_speed_near_line = f64::INFINITY;
                self.stopped_near_line = false;
            }
            self.last_lane = Some(c);
        }
        if let Some(n) = node {
            if self.entry_lane.is_none() && self.last_lane.is_some_and(|l| net.lane(l).to == n) {
                self.entry_lane = self.last_lane;
                self.entry_node = Some(n);
                self.entry_snapshot.clear();
                let nn = net.node(n);
                if nn.control == ControlType::Signal {
                    if let Some(sid) = nn.signal {
                        let sig = &sim.signals[sid.idx()];
                        for &ap in &nn.approaches {
                            for &l in &net.approach(ap).lanes {
                                for &c in &net.lane(l).outgoing {
                                    self.entry_snapshot
                                        .insert(c, sig.indication(net.connector(c)));
                                }
                            }
                        }
                    }
                }
            }
        }
        // Stopped inside the box while our movement is red.
        match (node, self.entry_lane) {
            (Some(n), Some(el)) if speed < 0.5 && net.node(n).control == ControlType::Signal => {
                let any_green = net.node(n).signal.is_some_and(|sid| {
                    let sig = &sim.signals[sid.idx()];
                    net.lane(el).outgoing.iter().any(|&c| {
                        let i = sig.indication(net.connector(c));
                        i == Indication::Green || i == Indication::Permissive
                    })
                });
                if !any_green {
                    self.box_stop_time += dt;
                    if self.box_stop_time > 4.0 {
                        out.push(Observation::new(ViolationKind::BlockingBox));
                    }
                } else {
                    self.box_stop_time = 0.0;
                }
            }
            _ => self.box_stop_time = 0.0,
        }

        // --- Speeding.
        if let (Some(l), None) = (lane, node) {
            let ln = net.lane(l);
            let lim = ln.speed_limit;
            let over = (speed - lim) / lim;
            if over > 0.1 && speed - lim > 1.0 {
                self.speeding_time += dt;
                if self.speeding_time > 2.0 {
                    let kind = if over > 0.5 {
                        ViolationKind::SpeedingExtreme
                    } else if over > 0.25 {
                        ViolationKind::SpeedingMajor
                    } else {
                        ViolationKind::SpeedingMinor
                    };
                    let cam = net.road(ln.road).speed_camera
                        && Self::near_speed_camera(ln.length, aligned.map_or(0.0, |h| h.s));
                    out.push(Observation {
                        kind,
                        camera_node: None,
                        speed_camera: cam,
                    });
                }
            } else {
                self.speeding_time = 0.0;
            }
        }

        // --- Equipment.
        if sim.env.dark && !lights.any_headlight() && speed > 3.0 {
            self.dark_no_lights_time += dt;
            if self.dark_no_lights_time > 10.0 {
                out.push(Observation::new(ViolationKind::NoHeadlights));
            }
        } else {
            self.dark_no_lights_time = 0.0;
        }
        if !self.seatbelt && speed > 5.5 {
            self.unbelted_time += dt;
            if self.unbelted_time > 5.0 {
                out.push(Observation::new(ViolationKind::Seatbelt));
            }
        } else {
            self.unbelted_time = 0.0;
        }
        if lights.horn {
            self.horn_held += dt;
            if self.horn_held > 4.0 {
                out.push(Observation::new(ViolationKind::HornAbuse));
            }
        } else {
            self.horn_held = (self.horn_held - dt * 0.5).max(0.0);
        }

        // --- Tailgating (time headway < 0.6 s at speed).
        self.info.leader_gap = f64::INFINITY;
        self.info.headway = f64::INFINITY;
        if let (Some(c), Some(ah)) = (cur, aligned) {
            let mut gap = f64::INFINITY;
            for e in sim.occ.lane(c) {
                if e.vehicle == p.id {
                    continue;
                }
                let d = e.s - ah.s - (e.length + p.length) / 2.0;
                if d > -0.5 && d < gap && (e.lat - ah.lateral).abs() < 2.2 {
                    gap = d;
                }
            }
            self.info.leader_gap = gap;
            self.info.headway = if speed > 1.0 {
                gap / speed
            } else {
                f64::INFINITY
            };
            if speed > 12.0 && self.info.headway < 0.6 {
                self.tailgate_time += dt;
                if self.tailgate_time > 4.0 {
                    out.push(Observation::new(ViolationKind::Tailgating));
                }
            } else {
                self.tailgate_time = 0.0;
            }
        }

        // --- Reckless: sustained sliding at speed.
        if p.dynamics.skid > 0.6 && speed > 6.0 {
            self.skid_time += dt;
            if self.skid_time > 2.0 {
                out.push(Observation::new(ViolationKind::Reckless));
            }
        } else {
            self.skid_time = (self.skid_time - dt).max(0.0);
        }

        // --- Emergency vehicles behind with siren.
        let mut emergency_behind = false;
        for e in &sim.vehicles {
            if !e.siren_on() || e.id == p.id {
                continue;
            }
            let dx = e.x - p.x;
            let dy = e.y - p.y;
            if dx * dx + dy * dy > 45.0 * 45.0 {
                continue;
            }
            let behind = dx * p.heading.cos() + dy * p.heading.sin() < -3.0;
            if behind && angle_diff(p.heading, e.heading).abs() < 45.0 * DEG && e.abs_speed() > 4.0
            {
                emergency_behind = true;
            }
        }
        if emergency_behind && speed > 4.0 {
            self.emergency_behind_time += dt;
            if self.emergency_behind_time > 7.0 {
                out.push(Observation::new(ViolationKind::FailYieldEmergency));
            }
        } else {
            self.emergency_behind_time = (self.emergency_behind_time - dt * 2.0).max(0.0);
        }

        // --- Hit and run.
        if let Some(mut hr) = self.hit_run {
            if speed < 0.5 {
                hr.stopped_for += dt;
            }
            self.hit_run = Some(hr);
            let dx = p.x - hr.x;
            let dy = p.y - hr.y;
            if hr.stopped_for > 3.0 || now - hr.time > 40.0 {
                self.hit_run = None;
            } else if dx * dx + dy * dy > 90.0 * 90.0 {
                out.push(Observation::new(ViolationKind::HitAndRun));
                self.hit_run = None;
            }
        }

        self.update_ahead(sim, pi, cur, aligned);
    }

    /// Recently used the hazards (a "thank you" blink)?
    pub fn hazard_thanks(&self, now: f64) -> bool {
        now - self.hazard_at < 1.5
    }

    fn near_speed_camera(lane_len: f64, s: f64) -> bool {
        (s - lane_len / 2.0).abs() < 30.0
    }

    fn evaluate_movement(&mut self, sim: &Sim, exit_lane: LaneId, out: &mut Vec<Observation>) {
        let net = &sim.net;
        let (Some(entry), Some(node)) = (self.entry_lane, self.entry_node) else {
            return;
        };
        let exit_road = net.lane(exit_lane).road;
        let el = net.lane(entry);
        let c = match net.connector_to(entry, exit_road) {
            Some(c) => c,
            None => {
                // Not allowed from our lane: allowed from another lane of the approach?
                let mut any = None;
                if let Some(ap) = el.approach {
                    for &l in &net.approach(ap).lanes {
                        if let Some(k) = net.connector_to(l, exit_road) {
                            any = Some(k);
                        }
                    }
                }
                match any {
                    Some(k) => k,
                    None => {
                        out.push(Observation::new(ViolationKind::IllegalTurn));
                        return;
                    }
                }
            }
        };
        let now = sim.time;
        let turn = net.connector(c).turn;
        if turn == TurnType::Left && now - self.signal_left_at > 6.0 {
            out.push(Observation::new(ViolationKind::NoSignalTurn));
        }
        if turn == TurnType::Right && now - self.signal_right_at > 6.0 {
            out.push(Observation::new(ViolationKind::NoSignalTurn));
        }
        let nn = net.node(node);
        if nn.control == ControlType::Signal {
            let ind = self.entry_snapshot.get(&c).copied();
            match ind {
                Some(Indication::Red) => {
                    let allow_ror = nn
                        .signal
                        .is_some_and(|s| sim.signals[s.idx()].allow_right_on_red);
                    if turn == TurnType::Right && allow_ror {
                        if !self.stopped_near_line {
                            out.push(Observation::new(ViolationKind::RightOnRedNoStop));
                        }
                    } else {
                        out.push(Observation {
                            kind: ViolationKind::RedLight,
                            camera_node: if nn.red_light_camera {
                                Some(node)
                            } else {
                                None
                            },
                            speed_camera: false,
                        });
                    }
                }
                Some(Indication::FlashRed) | Some(Indication::Dark)
                    if self.min_speed_near_line > 1.5 =>
                {
                    out.push(Observation::new(ViolationKind::StopSign))
                }
                _ => {}
            }
        } else if el
            .approach
            .is_some_and(|a| net.approach(a).sign == ApproachSign::Stop)
            && self.min_speed_near_line > 1.4
        {
            out.push(Observation::new(ViolationKind::StopSign));
        }
    }

    fn update_ahead(&mut self, sim: &Sim, pi: usize, cur: Option<LaneId>, hit: Option<LaneHit>) {
        let net = &sim.net;
        let (Some(cur), Some(hit)) = (cur, hit) else {
            if !self.info.in_junction {
                self.info.ahead = None;
            }
            return;
        };
        let cl = net.lane(cur);
        let dist = cl.length - hit.s;
        let Some(ap_id) = cl.approach.filter(|_| dist <= 160.0) else {
            self.info.ahead = None;
            return;
        };
        let ap = net.approach(ap_id);
        let node = net.node(ap.node);
        let mut control = AheadControl::None;
        let mut head = None;
        let mut indication = None;
        if node.control == ControlType::Signal {
            control = AheadControl::Signal;
            if let Some(sid) = node.signal {
                let sig = &sim.signals[sid.idx()];
                head = Some(sig.head(ap_id));
                let want = match sim.vehicles[pi].lights.indicator {
                    Indicator::Left => TurnType::Left,
                    Indicator::Right => TurnType::Right,
                    Indicator::Off => TurnType::Straight,
                };
                let mut c = None;
                for &k in &cl.outgoing {
                    if net.connector(k).turn == want {
                        c = Some(k);
                    }
                }
                if c.is_none() {
                    c = cl.outgoing.first().copied();
                }
                if let Some(c) = c {
                    indication = Some(sig.indication(net.connector(c)));
                }
            }
        } else if ap.sign == ApproachSign::Stop {
            control = AheadControl::Stop;
        } else if ap.sign == ApproachSign::Yield {
            control = AheadControl::Yield;
        }
        self.info.ahead = Some(AheadInfo {
            distance: dist,
            control,
            head,
            indication,
            node_name: node.name.clone(),
            camera: node.red_light_camera,
        });
    }
}
