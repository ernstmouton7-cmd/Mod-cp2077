//! Pedestrians crossing at marked crosswalks.

use crate::ids::{CrosswalkId, PedId};
use crate::math::scalar::{angle_diff, clamp, DEG};
use crate::math::Rng;
use crate::road::{ControlType, Crosswalk, Network};
use crate::signals::{PedLamp, SignalController, SignalMode};

/// Pedestrian state machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PedState {
    Waiting,
    Crossing,
    Leaving,
    Down,
}

/// A pedestrian using a crosswalk (sim coordinates).
#[derive(Clone, Debug)]
pub struct Pedestrian {
    pub id: PedId,
    pub cw: CrosswalkId,
    pub x: f64,
    pub y: f64,
    pub heading: f64,
    /// Position along the crosswalk 0..1.
    pub t: f64,
    /// Walking direction (+1 a→b, -1 b→a).
    pub dir: f64,
    pub state: PedState,
    pub waited: f64,
    /// Seconds left before despawn while leaving / lying down.
    pub ttl: f64,
    /// Animation phase for renderers.
    pub stride: f64,
    pub speed: f64,
    /// Starts crossing on a flashing hand / accepts shorter gaps.
    pub reckless: bool,
    pub shirt: u32,
    pub height: f64,
    /// Seconds until this pedestrian may shout again.
    pub shout_cooldown: f64,
}

const SHIRTS: [u32; 9] = [0xe0e0e0, 0x2b2b2b, 0xb03a2e, 0x2e86c1, 0xf1c40f, 0x7d3c98, 0x16a085, 0xd35400, 0x34495e];

impl Pedestrian {
    pub fn new(id: PedId, cw: &Crosswalk, rng: &mut Rng) -> Self {
        let speed = clamp(rng.gauss(1.35, 0.25), 0.8, 2.0);
        let reckless = rng.chance(0.08);
        let shirt = rng.pick(&SHIRTS);
        let height = clamp(rng.gauss(1.72, 0.08), 1.5, 1.95);
        let dir = if rng.chance(0.5) { 1.0 } else { -1.0 };
        let mut p = Pedestrian {
            id,
            cw: cw.id,
            x: 0.0,
            y: 0.0,
            heading: 0.0,
            t: if dir > 0.0 { 0.0 } else { 1.0 },
            dir,
            state: PedState::Waiting,
            waited: 0.0,
            ttl: 0.0,
            stride: 0.0,
            speed,
            reckless,
            shirt,
            height,
            shout_cooldown: 0.0,
        };
        p.place(cw);
        p
    }

    /// Update the position from `t` (waiting pedestrians stand a step back from the curb).
    pub fn place(&mut self, cw: &Crosswalk) {
        let mut t = self.t;
        if self.state == PedState::Waiting {
            t = if self.dir > 0.0 { -0.9 / cw.length } else { 1.0 + 0.9 / cw.length };
        }
        self.x = cw.ax + (cw.bx - cw.ax) * t;
        self.y = cw.ay + (cw.by - cw.ay) * t;
        self.heading = ((cw.by - cw.ay) * self.dir).atan2((cw.bx - cw.ax) * self.dir);
    }
}

/// What the pedestrian system needs from the world.
pub trait PedHost {
    fn net(&self) -> &Network;
    fn signals(&self) -> &[SignalController];
    /// Vehicles near a point or heading through it within `seconds` (gap acceptance).
    fn traffic_approaching(&self, x: f64, y: f64, radius: f64, seconds: f64) -> bool;
    /// Is a vehicle body within `margin` of the point?
    fn vehicle_at(&self, x: f64, y: f64, margin: f64) -> bool;
    fn on_shout(&mut self, p: &Pedestrian);
}

/// Spawns pedestrians at crosswalk ends, lets them cross with the walk signal
/// (or a safe gap at stop-sign junctions), makes them look before stepping
/// off and pause for cars in their path, and removes them once done.
#[derive(Clone, Debug)]
pub struct PedestrianSystem {
    pub peds: Vec<Pedestrian>,
    pub crosswalks: Vec<Crosswalk>,
    pub rng: Rng,
    next_id: u32,
    spawn_timer: f64,
    /// Target number of pedestrians (scaled by time of day by the world).
    pub target: usize,
}

impl PedestrianSystem {
    pub fn new(crosswalks: Vec<Crosswalk>, rng: Rng) -> Self {
        PedestrianSystem { peds: Vec::new(), crosswalks, rng, next_id: 1, spawn_timer: 0.0, target: 40 }
    }

    /// Is walking allowed across this crosswalk right now?
    pub fn walk_allowed(&self, net: &Network, signals: &[SignalController], cw: CrosswalkId, reckless: bool) -> bool {
        walk_allowed_cw(net, signals, &self.crosswalks[cw.idx()], reckless)
    }

    pub fn update(&mut self, dt: f64, host: &mut impl PedHost) {
        self.spawn_timer -= dt;
        if self.spawn_timer <= 0.0 && !self.crosswalks.is_empty() {
            self.spawn_timer = 0.6;
            if self.peds.len() < self.target {
                let ci = self.rng.index(self.crosswalks.len());
                let cw = &self.crosswalks[ci];
                let crowded = self.peds.iter().filter(|p| p.cw == cw.id && p.state == PedState::Waiting).count();
                if crowded < 4 {
                    let id = PedId(self.next_id);
                    self.next_id += 1;
                    let p = Pedestrian::new(id, cw, &mut self.rng);
                    self.peds.push(p);
                }
            }
        }
        let mut i = self.peds.len();
        while i > 0 {
            i -= 1;
            let cw = &self.crosswalks[self.peds[i].cw.idx()];
            let p = &mut self.peds[i];
            p.shout_cooldown -= dt;
            match p.state {
                PedState::Waiting => {
                    p.waited += dt;
                    let mut go;
                    if host.net().node(cw.node).control == ControlType::Signal {
                        go = walk_allowed_cw(host.net(), host.signals(), cw, p.reckless);
                    } else {
                        let midx = (cw.ax + cw.bx) / 2.0;
                        let midy = (cw.ay + cw.by) / 2.0;
                        go = p.waited > 1.5 && !host.traffic_approaching(midx, midy, cw.length / 2.0 + 2.0, if p.reckless { 1.5 } else { 3.5 });
                    }
                    if p.waited > 120.0 {
                        go = true;
                    }
                    if go {
                        let sx = cw.ax + (cw.bx - cw.ax) * p.t;
                        let sy = cw.ay + (cw.by - cw.ay) * p.t;
                        if host.vehicle_at(sx, sy, 2.2) || host.traffic_approaching(sx, sy, 2.5, if p.reckless { 0.6 } else { 1.4 }) {
                            go = false;
                        }
                    }
                    if go {
                        p.state = PedState::Crossing;
                        p.place(cw);
                    }
                }
                PedState::Crossing => {
                    let ux = (cw.bx - cw.ax) / cw.length;
                    let uy = (cw.by - cw.ay) / cw.length;
                    if host.vehicle_at(p.x + ux * p.dir, p.y + uy * p.dir, 0.4) {
                        if p.shout_cooldown <= 0.0 && host.vehicle_at(p.x, p.y, 1.0) {
                            p.shout_cooldown = 6.0;
                            host.on_shout(p);
                        }
                        continue;
                    }
                    p.t += (p.dir * p.speed * dt) / cw.length;
                    p.stride += p.speed * dt * 1.8;
                    if p.t >= 1.0 || p.t <= 0.0 {
                        p.t = clamp(p.t, 0.0, 1.0);
                        p.state = PedState::Leaving;
                        p.ttl = 4.0;
                    }
                    p.place(cw);
                }
                PedState::Leaving => {
                    let away = cw.road_heading;
                    p.x += away.cos() * p.speed * dt;
                    p.y += away.sin() * p.speed * dt;
                    p.heading = away;
                    p.stride += p.speed * dt * 1.8;
                    p.ttl -= dt;
                    if p.ttl <= 0.0 {
                        self.peds.remove(i);
                    }
                }
                PedState::Down => {
                    p.ttl -= dt;
                    if p.ttl <= 0.0 {
                        self.peds.remove(i);
                    }
                }
            }
        }
    }

    /// Is someone crossing at (or walking toward) position `t` (0..1) within
    /// `half_width` metres?
    pub fn blocking(&self, cw: CrosswalkId, t: f64, half_width: f64) -> bool {
        let len = self.crosswalks[cw.idx()].length;
        self.peds.iter().any(|p| {
            if p.cw != cw || p.state != PedState::Crossing {
                return false;
            }
            let along = (p.t - t) * len;
            let approaching = along * p.dir < 0.0 && along.abs() < half_width + p.speed * 2.5;
            along.abs() < half_width || approaching
        })
    }
}

fn walk_allowed_cw(net: &Network, signals: &[SignalController], cw: &Crosswalk, reckless: bool) -> bool {
    let n = net.node(cw.node);
    let Some(sid) = n.signal.filter(|_| n.control == ControlType::Signal) else { return false };
    let sig = &signals[sid.idx()];
    if sig.mode == SignalMode::Flashing || sig.mode == SignalMode::Off {
        return false;
    }
    for &ap in &n.approaches {
        let a = net.approach(ap);
        if a.road == cw.road {
            continue;
        }
        let rel = (angle_diff(a.heading, cw.road_heading).abs() - 90.0 * DEG).abs();
        if rel > 30.0 * DEG {
            continue;
        }
        let h = sig.head(ap);
        if h.ped == PedLamp::Walk || (reckless && h.ped == PedLamp::Flash) {
            return true;
        }
    }
    false
}
