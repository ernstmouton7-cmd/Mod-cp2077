//! Traffic signal controllers: phase plans, actuation, pre-emption, flashing.

use crate::ids::{ApproachId, ConnectorId, NodeId, SignalId};
use crate::math::scalar::{angle_diff, clamp, fmod, DEG};
use crate::road::{Connector, Network, TurnType};

/// What a single movement (connector) is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Indication {
    /// Protected: go.
    Green,
    /// Green ball for a left turn: go but yield to oncoming traffic.
    Permissive,
    /// Stop if you safely can.
    Yellow,
    /// Stop (right on red after a full stop where allowed).
    Red,
    /// Proceed with caution.
    FlashYellow,
    /// Treat as a stop sign.
    FlashRed,
    /// Power failure: treat as an all-way stop.
    Dark,
}

/// A signal head lamp.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Lamp {
    Red,
    Yellow,
    Green,
    Off,
}

/// Pedestrian signal state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PedLamp {
    Walk,
    Flash,
    Dont,
    Off,
}

/// Display state of the signal heads facing one approach.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeadState {
    pub ball: Lamp,
    /// Left-turn arrow head (`None` when the approach has no protected left).
    pub arrow: Option<Lamp>,
    pub ball_flashing: bool,
    /// Pedestrian signal for the crosswalk parallel to this approach.
    pub ped: PedLamp,
    pub ped_countdown: f64,
}

/// Controller operating mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SignalMode {
    Fixed,
    Actuated,
    Flashing,
    Off,
}

/// Interval within the active phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PhaseState {
    Green,
    Yellow,
    AllRed,
}

/// Movement right granted by a phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Right {
    Green,
    Permissive,
}

/// One phase of the plan.
#[derive(Clone, Debug)]
pub struct Phase {
    pub name: String,
    /// Movement rights: connector → protected/permissive.
    pub serves: Vec<(ConnectorId, Right)>,
    pub approaches: Vec<ApproachId>,
    pub is_left_arrow: bool,
    pub min_green: f64,
    pub max_green: f64,
    /// Green duration used in fixed-time mode.
    pub fixed_green: f64,
    pub yellow: f64,
    pub all_red: f64,
    /// Always served even without demand (main street recall).
    pub recall: bool,
    /// Served connectors in detector order (approach → lane → outgoing).
    demand_order: Vec<ConnectorId>,
}

impl Phase {
    pub fn right_for(&self, c: ConnectorId) -> Option<Right> {
        self.serves.iter().find(|s| s.0 == c).map(|s| s.1)
    }

    pub fn serves_connector(&self, c: ConnectorId) -> bool {
        self.serves.iter().any(|s| s.0 == c)
    }
}

/// A phase/interval change, reported by [`SignalController::update`].
#[derive(Clone, Debug, PartialEq)]
pub struct SignalChange {
    pub node: NodeId,
    pub phase: String,
    pub state: PhaseState,
}

/// Options for [`SignalController::new`].
#[derive(Clone, Copy, Debug)]
pub struct SignalOptions {
    pub mode: SignalMode,
    /// Coordination offset (s): the fixed plan is advanced by this much.
    pub offset: f64,
    pub allow_right_on_red: bool,
    pub cycle_scale: f64,
}

impl Default for SignalOptions {
    fn default() -> Self {
        SignalOptions {
            mode: SignalMode::Actuated,
            offset: 0.0,
            allow_right_on_red: true,
            cycle_scale: 1.0,
        }
    }
}

/// Traffic signal controller for one intersection. Builds a phase plan from
/// the node geometry (opposing approaches share a phase, dedicated left lanes
/// get a leading protected arrow) and runs it fixed-time, actuated (detector
/// demand, gap-out, max green, main-road recall), flashing or dark, with
/// emergency pre-emption.
#[derive(Clone, Debug)]
pub struct SignalController {
    pub id: SignalId,
    pub node: NodeId,
    pub phases: Vec<Phase>,
    pub mode: SignalMode,
    phase_index: usize,
    state: PhaseState,
    t: f64,
    gap_timer: f64,
    preempt_approach: Option<ApproachId>,
    preempt_ttl: f64,
    flash_clock: f64,
    pub allow_right_on_red: bool,
    /// Approaches considered the main road (flash yellow at night).
    main_approaches: Vec<ApproachId>,
    ped_walk_time: f64,
    arrow_approaches: Vec<ApproachId>,
    node_radius: f64,
}

impl SignalController {
    /// Build the controller for a finalized node.
    pub fn new(id: SignalId, net: &mut Network, node: NodeId, opts: SignalOptions) -> Self {
        let mut s = SignalController {
            id,
            node,
            phases: Vec::new(),
            mode: opts.mode,
            phase_index: 0,
            state: PhaseState::Green,
            t: 0.0,
            gap_timer: 0.0,
            preempt_approach: None,
            preempt_ttl: 0.0,
            flash_clock: 0.0,
            allow_right_on_red: opts.allow_right_on_red,
            main_approaches: Vec::new(),
            ped_walk_time: 7.0,
            arrow_approaches: Vec::new(),
            node_radius: net.node(node).radius,
        };
        s.build_phases(net, opts.cycle_scale);
        // Coordination offset: advance the fixed plan so neighbours form a green wave.
        if opts.offset > 0.0 {
            let saved = s.mode;
            s.mode = SignalMode::Fixed;
            let mut left = opts.offset;
            while left > 0.0 {
                let dt = left.min(0.5);
                s.update(dt, None);
                left -= dt;
            }
            s.mode = saved;
        }
        s
    }

    fn build_phases(&mut self, net: &mut Network, cycle_scale: f64) {
        let aps = net.node(self.node).approaches.clone();
        let mut used = vec![false; aps.len()];
        let mut groups: Vec<Vec<ApproachId>> = Vec::new();
        for i in 0..aps.len() {
            if used[i] {
                continue;
            }
            let mut g = vec![aps[i]];
            used[i] = true;
            for j in i + 1..aps.len() {
                if used[j] {
                    continue;
                }
                let hi = net.approach(aps[i]).heading;
                let hj = net.approach(aps[j]).heading;
                if (angle_diff(hi, hj).abs() - std::f64::consts::PI).abs() < 35.0 * DEG {
                    g.push(aps[j]);
                    used[j] = true;
                    break;
                }
            }
            groups.push(g);
        }
        // The main axis is the widest one.
        let mut best_w = -1.0;
        let mut main = 0;
        for (gi, g) in groups.iter().enumerate() {
            let w: f64 = g
                .iter()
                .map(|&a| {
                    net.approach(a).lanes.len() as f64
                        * (net.road(net.approach(a).road).speed_limit + 1.0)
                })
                .sum();
            if w > best_w {
                best_w = w;
                main = gi;
            }
        }
        if !groups.is_empty() {
            self.main_approaches = groups[main].clone();
        }
        for (gi, g) in groups.iter().enumerate() {
            for &a in g {
                net.approaches[a.idx()].group = gi as i32;
            }
            let is_main = gi == main;
            let speed = g
                .iter()
                .map(|&a| net.road(net.approach(a).road).speed_limit)
                .fold(f64::NEG_INFINITY, f64::max);
            let yellow = clamp(1.0 + speed / (2.0 * 3.05), 3.0, 5.5);
            let cross_width = self.node_radius * 2.0;
            let all_red = clamp((cross_width + 5.0) / speed.max(8.0), 1.0, 2.5);
            let has_dedicated_left = g.iter().any(|&a| {
                net.approach(a)
                    .lanes
                    .iter()
                    .any(|&l| net.lane(l).turns == [TurnType::Left])
            });
            let conns_of = |net: &Network| -> Vec<ConnectorId> {
                let mut v = Vec::new();
                for &a in g {
                    for &l in &net.approach(a).lanes {
                        v.extend(net.lane(l).outgoing.iter().copied());
                    }
                }
                v
            };
            let all = conns_of(net);
            if has_dedicated_left && g.len() > 1 {
                let mut serves = Vec::new();
                for &c in &all {
                    if net.connector(c).turn == TurnType::Left
                        && !serves.iter().any(|s: &(ConnectorId, Right)| s.0 == c)
                    {
                        serves.push((c, Right::Green));
                    }
                }
                let demand_order = all
                    .iter()
                    .copied()
                    .filter(|c| serves.iter().any(|s| s.0 == *c))
                    .collect();
                self.phases.push(Phase {
                    name: format!("G{gi}-left"),
                    serves,
                    approaches: g.clone(),
                    is_left_arrow: true,
                    min_green: 4.0,
                    max_green: 12.0 * cycle_scale,
                    fixed_green: 8.0 * cycle_scale,
                    yellow: 3.0,
                    all_red: 1.0,
                    recall: false,
                    demand_order,
                });
            }
            let opposed = g.len() > 1;
            let mut serves: Vec<(ConnectorId, Right)> = Vec::new();
            for &c in &all {
                let t = net.connector(c).turn;
                let right = if t == TurnType::Left || t == TurnType::UTurn {
                    if opposed {
                        Right::Permissive
                    } else {
                        Right::Green
                    }
                } else {
                    Right::Green
                };
                if let Some(e) = serves.iter_mut().find(|s| s.0 == c) {
                    e.1 = right;
                } else {
                    serves.push((c, right));
                }
            }
            let demand_order = all.clone();
            self.phases.push(Phase {
                name: format!("G{gi}-through"),
                serves,
                approaches: g.clone(),
                is_left_arrow: false,
                min_green: if is_main { 10.0 } else { 6.0 },
                max_green: (if is_main { 40.0 } else { 24.0 }) * cycle_scale,
                fixed_green: (if is_main { 26.0 } else { 16.0 }) * cycle_scale,
                yellow,
                all_red,
                recall: is_main,
                demand_order,
            });
        }
        for p in &self.phases {
            if p.is_left_arrow {
                for &a in &p.approaches {
                    if !self.arrow_approaches.contains(&a) {
                        self.arrow_approaches.push(a);
                    }
                }
            }
        }
        // Start on the main through phase.
        if let Some(i) = self.phases.iter().position(|p| p.recall) {
            self.phase_index = i;
        }
    }

    pub fn current_phase(&self) -> &Phase {
        &self.phases[self.phase_index]
    }

    pub fn current_phase_index(&self) -> usize {
        self.phase_index
    }

    pub fn phase_state(&self) -> PhaseState {
        self.state
    }

    /// Seconds spent in the current interval.
    pub fn state_time(&self) -> f64 {
        self.t
    }

    /// Cycle length of the fixed plan (s).
    pub fn cycle_length(&self) -> f64 {
        self.phases
            .iter()
            .map(|p| p.fixed_green + p.yellow + p.all_red)
            .sum()
    }

    pub fn set_mode(&mut self, mode: SignalMode) {
        if mode == self.mode {
            return;
        }
        self.mode = mode;
        // Coming back from flashing/dark: restart with an all-red clearance.
        if mode == SignalMode::Fixed || mode == SignalMode::Actuated {
            self.state = PhaseState::AllRed;
            self.t = 0.0;
        }
    }

    /// An emergency vehicle requests green for `approach` (call every step while needed).
    pub fn request_preempt(&mut self, approach: ApproachId) {
        self.preempt_approach = Some(approach);
        self.preempt_ttl = 1.0;
    }

    pub fn preempting(&self) -> bool {
        self.preempt_approach.is_some()
    }

    fn phase_serves_approach(p: &Phase, ap: ApproachId) -> bool {
        !p.is_left_arrow && p.approaches.contains(&ap)
    }

    fn phase_demand(p: &Phase, demand: Option<&dyn Fn(ConnectorId) -> f64>) -> f64 {
        match demand {
            None => 1.0,
            Some(f) => p.demand_order.iter().map(|&c| f(c)).sum(),
        }
    }

    fn change(&self) -> SignalChange {
        SignalChange {
            node: self.node,
            phase: self.current_phase().name.clone(),
            state: self.state,
        }
    }

    fn pick_next_phase(&self, demand: Option<&dyn Fn(ConnectorId) -> f64>) -> usize {
        let n = self.phases.len();
        if let Some(ap) = self.preempt_approach {
            if let Some(k) = self
                .phases
                .iter()
                .position(|p| Self::phase_serves_approach(p, ap))
            {
                return k;
            }
        }
        if self.mode == SignalMode::Fixed {
            return (self.phase_index + 1) % n;
        }
        for step in 1..=n {
            let k = (self.phase_index + step) % n;
            let p = &self.phases[k];
            if Self::phase_demand(p, demand) > 0.0 || (p.recall && step == n) {
                return k;
            }
        }
        // Nothing waiting: rest on the recall phase.
        if let Some(k) = self.phases.iter().position(|p| p.recall) {
            return k;
        }
        (self.phase_index + 1) % n
    }

    fn green_should_end(&mut self, demand: Option<&dyn Fn(ConnectorId) -> f64>, dt: f64) -> bool {
        let p = &self.phases[self.phase_index];
        if let Some(ap) = self.preempt_approach {
            return !Self::phase_serves_approach(p, ap) && self.t >= p.min_green.min(4.0);
        }
        if self.mode == SignalMode::Fixed {
            return self.t >= p.fixed_green;
        }
        if self.t < p.min_green {
            return false;
        }
        if self.t >= p.max_green {
            return self.others_waiting(demand);
        }
        // Gap-out: end when no vehicle has been detected for the passage time.
        let mine = Self::phase_demand(p, demand);
        let passage = if p.is_left_arrow { 1.5 } else { 3.0 };
        if mine > 0.0 {
            self.gap_timer = 0.0;
        } else {
            self.gap_timer += dt;
        }
        self.gap_timer >= passage && self.others_waiting(demand)
    }

    fn others_waiting(&self, demand: Option<&dyn Fn(ConnectorId) -> f64>) -> bool {
        self.phases
            .iter()
            .enumerate()
            .any(|(k, p)| k != self.phase_index && Self::phase_demand(p, demand) > 0.0)
    }

    /// Advance by `dt`. `demand` returns the detector count for a connector
    /// (`None` = demand everywhere). Returns the interval change, if any.
    pub fn update(
        &mut self,
        dt: f64,
        demand: Option<&dyn Fn(ConnectorId) -> f64>,
    ) -> Option<SignalChange> {
        self.flash_clock += dt;
        if self.preempt_ttl > 0.0 {
            self.preempt_ttl -= dt;
            if self.preempt_ttl <= 0.0 {
                self.preempt_approach = None;
            }
        }
        if self.mode == SignalMode::Flashing || self.mode == SignalMode::Off {
            return None;
        }
        self.t += dt;
        match self.state {
            PhaseState::Green => {
                if self.green_should_end(demand, dt) {
                    self.state = PhaseState::Yellow;
                    self.t = 0.0;
                    return Some(self.change());
                }
            }
            PhaseState::Yellow => {
                if self.t >= self.phases[self.phase_index].yellow {
                    self.state = PhaseState::AllRed;
                    self.t = 0.0;
                    return Some(self.change());
                }
            }
            PhaseState::AllRed => {
                if self.t >= self.phases[self.phase_index].all_red {
                    self.phase_index = self.pick_next_phase(demand);
                    self.state = PhaseState::Green;
                    self.t = 0.0;
                    self.gap_timer = 0.0;
                    return Some(self.change());
                }
            }
        }
        None
    }

    fn flash_on(&self) -> bool {
        fmod(self.flash_clock, 1.0) < 0.5
    }

    fn is_main(&self, a: ApproachId) -> bool {
        self.main_approaches.contains(&a)
    }

    /// The indication a vehicle on connector `c` sees.
    pub fn indication(&self, c: &Connector) -> Indication {
        match self.mode {
            SignalMode::Off => return Indication::Dark,
            SignalMode::Flashing => {
                return if self.is_main(c.approach) {
                    Indication::FlashYellow
                } else {
                    Indication::FlashRed
                };
            }
            _ => {}
        }
        let p = self.current_phase();
        match p.right_for(c.id) {
            None => Indication::Red,
            Some(_) if self.state == PhaseState::AllRed => Indication::Red,
            Some(_) if self.state == PhaseState::Yellow => Indication::Yellow,
            Some(Right::Green) => Indication::Green,
            Some(Right::Permissive) => Indication::Permissive,
        }
    }

    /// Seconds in the current interval (for HUD / AI anticipation).
    pub fn time_in_state(&self) -> f64 {
        self.t
    }

    /// Signal-head display for an approach.
    pub fn head(&self, a: ApproachId) -> HeadState {
        let mut out = HeadState {
            ball: Lamp::Red,
            arrow: None,
            ball_flashing: false,
            ped: PedLamp::Dont,
            ped_countdown: 0.0,
        };
        let has_arrow = self.arrow_approaches.contains(&a);
        if has_arrow {
            out.arrow = Some(Lamp::Off);
        }
        if self.mode == SignalMode::Off {
            out.ball = Lamp::Off;
            out.ped = PedLamp::Off;
            return out;
        }
        if self.mode == SignalMode::Flashing {
            out.ball_flashing = true;
            out.ball = if self.flash_on() {
                if self.is_main(a) {
                    Lamp::Yellow
                } else {
                    Lamp::Red
                }
            } else {
                Lamp::Off
            };
            out.ped = PedLamp::Off;
            return out;
        }
        let p = self.current_phase();
        let in_phase = p.approaches.contains(&a);
        let lamp = match self.state {
            PhaseState::Green => Lamp::Green,
            PhaseState::Yellow => Lamp::Yellow,
            PhaseState::AllRed => Lamp::Red,
        };
        if in_phase && self.state != PhaseState::AllRed {
            if p.is_left_arrow {
                out.arrow = Some(lamp);
                out.ball = Lamp::Red;
            } else {
                out.ball = lamp;
                if has_arrow {
                    out.arrow = Some(Lamp::Off);
                }
            }
        } else if has_arrow {
            out.arrow = Some(Lamp::Red);
        }
        // Pedestrians cross parallel to this approach while its through phase is green.
        if in_phase && !p.is_left_arrow && self.state == PhaseState::Green {
            let clearance = self.ped_clearance();
            let green_left = if self.mode == SignalMode::Fixed {
                p.fixed_green - self.t
            } else {
                p.min_green.max(clearance + self.ped_walk_time) - self.t
            };
            if self.t < self.ped_walk_time {
                out.ped = PedLamp::Walk;
            } else if green_left > 0.0 {
                out.ped = PedLamp::Flash;
                out.ped_countdown = green_left.max(0.0).ceil();
            }
        }
        out
    }

    fn ped_clearance(&self) -> f64 {
        (self.node_radius * 2.0) / 1.2
    }

    /// Is approach `a` in the active through phase, in green?
    pub fn is_green_for(&self, a: ApproachId) -> bool {
        let p = self.current_phase();
        self.state == PhaseState::Green && p.approaches.contains(&a) && !p.is_left_arrow
    }
}
