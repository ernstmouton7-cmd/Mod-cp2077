//! Junction rules for AI drivers: signals, signs, gap acceptance, all-way
//! stops, don't-block-the-box and pedestrians on crosswalks.

use super::driver::AIDriver;
use super::emotion::Mood;
use crate::ids::{ConnectorId, LaneId, VehicleId};
use crate::math::scalar::{angle_diff, clamp, lerp, DEG};
use crate::road::{ApproachSign, ConflictKind, ControlType, TurnType};
use crate::signals::Indication;
use crate::world::sim::Sim;

/// How a driver must treat the junction ahead.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EntryMode {
    Go,
    Stop,
    StopThenYield,
    Yield,
    Allway,
}

/// Latched yellow-light decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum YellowChoice {
    Stop,
    Go,
}

/// Per-approach latched decisions (reset whenever the driver enters a new lane).
#[derive(Clone, Debug, Default)]
pub struct JunctionState {
    pub lane: Option<LaneId>,
    pub yellow: Option<YellowChoice>,
    /// Decided to run this red light.
    pub red_run: bool,
    /// The run-or-stop decision for this red has been made (rolled once).
    pub red_decided: bool,
    /// Came to a complete stop at the line (stop signs, right on red).
    pub has_stopped: bool,
    /// Registered in the all-way-stop queue.
    pub queued: bool,
    /// Ignores don't-block-the-box this time.
    pub box_ignore: bool,
    /// Seconds waiting at this junction for yield reasons.
    pub wait_time: f64,
    /// Last reason we were held (debug HUD).
    pub reason: &'static str,
    /// Seconds held inside the junction by stationary vehicles only.
    pub conn_wait: f64,
    /// Deadlock broken in our favour.
    pub tie_break: bool,
    pub last_indication: Option<Indication>,
}

impl JunctionState {
    pub fn reset(&mut self, lane: LaneId) {
        *self = JunctionState {
            lane: Some(lane),
            ..Default::default()
        };
    }
}

fn move_rank(t: TurnType) -> i32 {
    match t {
        TurnType::Straight => 3,
        TurnType::Right => 2,
        TurnType::Left => 1,
        TurnType::UTurn => 0,
    }
}

fn ind_rank(i: Indication) -> i32 {
    match i {
        Indication::Green => 4,
        Indication::Yellow => 3,
        Indication::Permissive | Indication::FlashYellow => 2,
        Indication::FlashRed | Indication::Dark => 1,
        Indication::Red => 0,
    }
}

/// Does movement `o` have right of way over movement `c` right now?
pub fn has_priority(sim: &Sim, o: ConnectorId, c: ConnectorId) -> bool {
    let net = &sim.net;
    let oc = net.connector(o);
    let cc = net.connector(c);
    match (sim.indication_for(o), sim.indication_for(c)) {
        (Some(io), Some(ic)) => {
            let ro = ind_rank(io);
            let rc = ind_rank(ic);
            if ro != rc {
                return ro > rc;
            }
        }
        _ => {
            let mo = net.approach(oc.approach).major;
            let mc = net.approach(cc.approach).major;
            if mo != mc {
                return mo;
            }
        }
    }
    let a = move_rank(oc.turn);
    let b = move_rank(cc.turn);
    if a != b {
        return a > b;
    }
    // Same movement class: traffic from the right goes first.
    let d = angle_diff(
        net.approach(cc.approach).heading,
        net.approach(oc.approach).heading,
    );
    d > 45.0 * DEG && d < 135.0 * DEG
}

/// Resolve the entry mode for connector `c` from signals and signs.
pub fn entry_mode(
    sim: &Sim,
    d: &mut AIDriver,
    vi: usize,
    c: ConnectorId,
    dist_to_line: f64,
) -> EntryMode {
    let conn = sim.net.connector(c);
    let n = sim.net.node(conn.node);
    let ap = sim.net.approach(conn.approach);
    let speed = sim.vehicles[vi].speed;
    if let Some(ind) = sim.indication_for(c) {
        d.js.last_indication = Some(ind);
        return match ind {
            Indication::Green => {
                d.js.yellow = None;
                EntryMode::Go
            }
            Indication::Permissive | Indication::FlashYellow => EntryMode::Yield,
            Indication::FlashRed | Indication::Dark => EntryMode::Allway,
            Indication::Yellow => {
                if d.emergency {
                    return EntryMode::Yield;
                }
                if d.js.yellow.is_none() {
                    let sp = speed.abs();
                    let need = (sp * sp) / (2.0 * dist_to_line.max(0.1));
                    let threshold =
                        lerp(4.6, 2.0, d.p.aggression) * lerp(0.85, 1.1, d.p.lawfulness);
                    let too_close = dist_to_line < sp * 0.8;
                    d.js.yellow = Some(if need <= threshold && !too_close {
                        YellowChoice::Stop
                    } else {
                        YellowChoice::Go
                    });
                    if d.mood == Mood::Furious || d.raging {
                        d.js.yellow = Some(if need < 1.2 {
                            YellowChoice::Stop
                        } else {
                            YellowChoice::Go
                        });
                    }
                }
                if d.js.yellow == Some(YellowChoice::Go) {
                    EntryMode::Go
                } else {
                    EntryMode::Stop
                }
            }
            Indication::Red => {
                if d.emergency {
                    return EntryMode::StopThenYield;
                }
                if d.js.yellow == Some(YellowChoice::Go) && dist_to_line < 3.0 {
                    return EntryMode::Go;
                }
                if !d.js.red_decided && dist_to_line > 25.0 && speed.abs() > 8.0 {
                    d.js.red_decided = true;
                    let chance = (1.0 - d.p.lawfulness) * 0.08
                        + if d.raging { 0.5 } else { 0.0 }
                        + if d.mood == Mood::Furious { 0.15 } else { 0.0 };
                    if d.roll() < chance {
                        d.js.red_run = true;
                    }
                } else if !d.js.red_decided && dist_to_line <= 25.0 {
                    d.js.red_decided = true;
                }
                if d.js.red_run {
                    return EntryMode::Yield;
                }
                let sig = &sim.signals[n.signal.expect("signal node").idx()];
                if conn.turn == TurnType::Right && sig.allow_right_on_red {
                    return EntryMode::StopThenYield;
                }
                EntryMode::Stop
            }
        };
    }
    match ap.sign {
        ApproachSign::Stop => {
            if n.control == ControlType::AllwayStop {
                EntryMode::Allway
            } else {
                EntryMode::StopThenYield
            }
        }
        ApproachSign::Yield => EntryMode::Yield,
        _ => {
            if n.roads.len() <= 2 {
                EntryMode::Go
            } else {
                EntryMode::Yield
            }
        }
    }
}

/// Seconds needed to travel `dist` from `speed`, accelerating with `a` up to `vmax`.
pub fn time_to_cover(speed: f64, dist: f64, a: f64, vmax: f64) -> f64 {
    if dist <= 0.0 {
        return 0.0;
    }
    let sp = speed.max(0.0);
    let vm = vmax.max(sp).max(1.0);
    let t_acc = (vm - sp) / a;
    let d_acc = sp * t_acc + 0.5 * a * t_acc * t_acc;
    if d_acc >= dist {
        return (-sp + (sp * sp + 2.0 * a * dist).sqrt()) / a;
    }
    t_acc + (dist - d_acc) / vm
}

/// Vehicles already inside the junction on conflicting paths that would still
/// be in a shared zone when we arrive (applies regardless of priority).
fn committed_block(
    sim: &Sim,
    c: ConnectorId,
    me: VehicleId,
    me_half: f64,
    s_me: f64,
    speed: f64,
    strict: bool,
) -> bool {
    for z in &sim.net.connector(c).conflicts {
        if z.s1 < s_me - me_half {
            continue;
        }
        let o = z.other;
        for e in sim.occ.connector(o) {
            if e.vehicle == me {
                continue;
            }
            let half = e.length / 2.0;
            if e.s - half > z.os1 {
                continue;
            }
            let inside = e.s + half > z.os0;
            if !inside && e.v < 0.5 {
                if !has_priority(sim, o, c) {
                    continue;
                }
                if !strict || me < e.vehicle {
                    continue;
                }
            }
            let t_clear = (z.os1 - (e.s - half)) / e.v.max(1.0);
            let t_enter = (z.s0 - s_me - me_half).max(0.0) / speed.max(3.0);
            if t_clear > t_enter - 0.3 {
                return true;
            }
        }
    }
    false
}

/// Gap acceptance against movements with priority.
fn priority_block(
    sim: &Sim,
    d: &AIDriver,
    vi: usize,
    c: ConnectorId,
    dist_to_line: f64,
    gap_scale: f64,
) -> bool {
    let me = &sim.vehicles[vi];
    let a = 2.0;
    let conn = sim.net.connector(c);
    let vmax = conn.speed_limit;
    for z in &conn.conflicts {
        let o = z.other;
        if !has_priority(sim, o, c) {
            continue;
        }
        let t_me = time_to_cover(me.speed, dist_to_line + z.s1 + me.length, a, vmax);
        let need = (d.p.critical_gap * gap_scale).max(t_me + 1.2);
        for e in sim.occ.connector(o) {
            if e.vehicle == me.id {
                continue;
            }
            if e.s - e.length / 2.0 > z.os1 {
                continue;
            }
            let dist = z.os0 - e.s - e.length / 2.0;
            let t = if dist <= 0.0 {
                0.0
            } else {
                dist / e.v.max(0.5)
            };
            if e.v < 0.5 && dist > 2.0 {
                continue;
            }
            if t < need {
                return true;
            }
        }
        let lane_id = sim.net.connector(o).from;
        let lane = sim.net.lane(lane_id);
        for e in sim.occ.lane(lane_id) {
            if e.vehicle == me.id {
                continue;
            }
            let intent = sim.intent_of(e);
            if intent != Some(o) && intent.is_some() {
                continue;
            }
            if e.v < 1.0 {
                continue;
            }
            let dist = lane.length - e.s - e.length / 2.0 + z.os0;
            let t = dist / e.v.max(0.5);
            if t < need {
                return true;
            }
        }
    }
    false
}

/// Not enough room on the exit lane to clear the junction?
fn box_blocked(sim: &Sim, c: ConnectorId, me: VehicleId, me_len: f64) -> bool {
    let conn = sim.net.connector(c);
    let mut min_s = f64::INFINITY;
    let mut min_v = 0.0;
    for e in sim.occ.lane(conn.to) {
        if e.vehicle == me {
            continue;
        }
        let tail = e.s - e.length / 2.0;
        if tail < min_s {
            min_s = tail;
            min_v = e.v;
        }
    }
    let on_c: f64 = sim
        .occ
        .connector(c)
        .iter()
        .filter(|e| e.vehicle != me)
        .map(|e| e.length + 2.0)
        .sum();
    if min_s == f64::INFINITY {
        return on_c > conn.length + 4.0;
    }
    min_s - on_c < me_len + 2.5 && min_v < 3.0
}

/// Another car is already waiting inside the box on our movement.
fn connector_has_waiter(sim: &Sim, c: ConnectorId, me: VehicleId) -> bool {
    sim.occ
        .connector(c)
        .iter()
        .any(|e| e.vehicle != me && e.v < 1.5)
}

fn first_conflict_start(sim: &Sim, c: ConnectorId) -> f64 {
    let conn = sim.net.connector(c);
    let mut s = conn.length;
    for z in &conn.conflicts {
        if z.kind == ConflictKind::Cross && z.s0 < s {
            s = z.s0;
        }
    }
    s
}

/// All-way stop: is it our turn among vehicles waiting at conflicting approaches?
fn all_way_turn(sim: &Sim, me: VehicleId, c: ConnectorId) -> bool {
    let conn = sim.net.connector(c);
    let Some(q) = sim.all_way.get(&conn.node) else {
        return false;
    };
    let Some(&mine) = q.get(&me) else {
        return false;
    };
    for z in &conn.conflicts {
        let lane = sim.net.connector(z.other).from;
        for e in sim.occ.lane(lane) {
            if e.vehicle == me {
                continue;
            }
            let Some(&t) = q.get(&e.vehicle) else {
                continue;
            };
            let intent = sim.intent_of(e);
            if intent != Some(z.other) && intent.is_some() {
                continue;
            }
            if t < mine - 0.05 || ((t - mine).abs() <= 0.05 && e.vehicle < me) {
                return false;
            }
        }
    }
    true
}

/// Distance (from the front bumper) at which to stop for the junction
/// reached via connector `c`, or infinity to proceed. Called on the lane
/// leading into `c`.
pub fn approach_stop_distance(
    sim: &mut Sim,
    d: &mut AIDriver,
    vi: usize,
    lane: LaneId,
    c: ConnectorId,
    dt: f64,
) -> f64 {
    if d.js.lane != Some(lane) {
        d.js.reset(lane);
    }
    let (me_id, me_s, me_half, me_speed, me_len) = {
        let v = &sim.vehicles[vi];
        (v.id, v.s, v.half_len, v.speed, v.length)
    };
    let lane_len = sim.net.lane(lane).length;
    let dist_to_line = lane_len - me_s - me_half;
    let sp = me_speed.max(0.0);
    let range = 35f64.max((sp * sp) / 3.0 + 20.0);
    if dist_to_line > range {
        return f64::INFINITY;
    }

    let mut mode = entry_mode(sim, d, vi, c, dist_to_line);
    let stopped = sp < 0.4 && dist_to_line < 3.0;
    let rain_scale = 1.0 + sim.env.wetness * 0.25 + if sim.env.dark { 0.1 } else { 0.0 };
    let impatience = clamp(d.js.wait_time / 30.0, 0.0, 0.5);
    let gap_scale = rain_scale * if d.raging { 0.6 } else { 1.0 } * (1.0 - impatience);

    if mode == EntryMode::Stop {
        d.js.reason = "red";
        return dist_to_line;
    }
    if mode == EntryMode::StopThenYield || mode == EntryMode::Allway {
        if !d.js.has_stopped {
            let rolling = d.p.lawfulness < 0.35 && sp < 3.0 && dist_to_line < 2.0;
            if stopped || rolling {
                d.js.has_stopped = true;
            } else {
                d.js.reason = "stop sign";
                return dist_to_line;
            }
        }
        if mode == EntryMode::Allway {
            let node = sim.net.connector(c).node;
            if !d.js.queued {
                let now = sim.time;
                sim.all_way_mut(node).insert(me_id, now);
                d.js.queued = true;
            }
            if !all_way_turn(sim, me_id, c) && d.js.wait_time < 12.0 {
                d.js.wait_time += dt;
                d.js.reason = "all-way order";
                return dist_to_line;
            }
        } else {
            mode = EntryMode::Yield;
        }
    }

    if committed_block(
        sim,
        c,
        me_id,
        me_half,
        -me_half - dist_to_line,
        sp.max(3.0),
        d.js.wait_time < 15.0,
    ) {
        d.js.wait_time += dt;
        d.js.reason = "junction occupied";
        return dist_to_line;
    }
    if (mode == EntryMode::Yield || mode == EntryMode::Allway)
        && priority_block(sim, d, vi, c, dist_to_line, gap_scale)
    {
        // Permissive left: pull into the box and wait at the conflict point.
        let ind = d.js.last_indication;
        let turn = sim.net.connector(c).turn;
        if turn == TurnType::Left
            && (ind == Some(Indication::Permissive) || ind == Some(Indication::FlashYellow))
            && !box_blocked(sim, c, me_id, me_len)
            && !connector_has_waiter(sim, c, me_id)
        {
            d.js.reason = "waiting in box";
            return dist_to_line + first_conflict_start(sim, c) - 1.0;
        }
        d.js.wait_time += dt;
        d.js.reason = "yield";
        return dist_to_line;
    }
    // Don't block the box (unless this driver doesn't care today).
    if d.js.lane == Some(lane)
        && !d.js.box_ignore
        && d.js.wait_time == 0.0
        && d.p.lawfulness < 0.3
        && d.roll() < 0.3
    {
        d.js.box_ignore = true;
    }
    if !d.js.box_ignore && box_blocked(sim, c, me_id, me_len) {
        d.js.reason = "exit blocked";
        return dist_to_line;
    }
    d.js.reason = "";
    d.js.wait_time = 0.0;
    f64::INFINITY
}

/// While on a connector: distance to stop before a conflict zone that is
/// still occupied (or contested by a priority movement for permissive lefts).
pub fn connector_stop_distance(
    sim: &Sim,
    d: &mut AIDriver,
    vi: usize,
    c: ConnectorId,
    dt: f64,
) -> f64 {
    let me = &sim.vehicles[vi];
    let front = me.s + me.half_len;
    let sp = me.speed.max(0.0);
    let conn = sim.net.connector(c);
    let mut stop_at = f64::INFINITY;
    if d.js.conn_wait > 4.0 {
        d.js.tie_break = true;
    }
    let break_ties = d.js.tie_break;
    let mut only_stationary = true;
    for z in &conn.conflicts {
        if z.s0 < front - 0.5 {
            continue;
        }
        let dist_to_zone = z.s0 - front - 0.5;
        let t_arrive = dist_to_zone / sp.max(2.0);
        let o = z.other;
        let prio = has_priority(sim, o, c);
        let mut blocked = false;
        for e in sim.occ.connector(o) {
            if blocked {
                break;
            }
            if e.vehicle == me.id {
                continue;
            }
            let half = e.length / 2.0;
            if e.s - half > z.os1 {
                continue;
            }
            let inside = e.s + half > z.os0;
            if !inside && e.v < 0.5 && me.id < e.vehicle {
                continue;
            }
            if !inside && !prio && e.v < 3.0 {
                continue;
            }
            if e.v < 0.5 && break_ties && me.id < e.vehicle {
                continue;
            }
            let t_clear = (z.os1 - (e.s - half)) / e.v.max(1.0);
            let t_them = if inside {
                0.0
            } else {
                (z.os0 - e.s - half).max(0.0) / e.v.max(0.5)
            };
            if inside || (prio && t_them < t_arrive + 2.0) || t_clear > t_arrive - 0.2 {
                blocked = true;
                if e.v >= 0.5 {
                    only_stationary = false;
                }
            }
        }
        // Permissive lefts also wait for oncoming traffic still on its approach lane.
        if !blocked && conn.turn == TurnType::Left && prio {
            let lane_id = sim.net.connector(o).from;
            let lane = sim.net.lane(lane_id);
            let t_me = time_to_cover(
                sp,
                dist_to_zone + (z.s1 - z.s0) + me.length,
                2.0,
                conn.speed_limit,
            );
            for e in sim.occ.lane(lane_id) {
                if e.vehicle == me.id || e.v < 1.0 {
                    continue;
                }
                let intent = sim.intent_of(e);
                if intent != Some(o) && intent.is_some() {
                    continue;
                }
                let t = (lane.length - e.s - e.length / 2.0 + z.os0) / e.v.max(0.5);
                if t < t_me + 1.0 {
                    blocked = true;
                    only_stationary = false;
                    break;
                }
            }
        }
        if blocked {
            stop_at = stop_at.min(dist_to_zone);
            break;
        }
    }
    if stop_at < f64::INFINITY && sp < 0.3 && only_stationary {
        d.js.conn_wait += dt;
    }
    stop_at
}

/// Distance (from the front bumper) at which to stop for pedestrians on a
/// crosswalk that connector `c` crosses, or infinity. `ahead` is the distance
/// from the bumper to the start of `c` (negative once on it).
pub fn pedestrian_stop_distance(
    sim: &Sim,
    vi: usize,
    c: ConnectorId,
    ahead: f64,
    front_on_c: f64,
) -> f64 {
    for x in &sim.crossings[c.idx()] {
        if x.s < front_on_c - 0.3 {
            continue;
        }
        let dist = ahead + x.s - front_on_c.max(0.0) - 1.6;
        if dist > 45.0 {
            break;
        }
        if !sim.peds.blocking(x.cw, x.t, 1.7) {
            continue;
        }
        if dist > 0.0 {
            return dist;
        }
        // Too late to stop short: keep clearing unless someone is right in front.
        if ped_in_front(sim, vi) {
            return 0.0;
        }
    }
    f64::INFINITY
}

fn ped_in_front(sim: &Sim, vi: usize) -> bool {
    let me = &sim.vehicles[vi];
    let c = me.heading.cos();
    let s = me.heading.sin();
    sim.peds.peds.iter().any(|p| {
        if p.state != super::pedestrians::PedState::Crossing {
            return false;
        }
        let dx = p.x - me.x;
        let dy = p.y - me.y;
        let lx = dx * c + dy * s;
        let ly = -dx * s + dy * c;
        lx > me.half_len - 0.5 && lx < me.half_len + 2.5 && ly.abs() < me.half_wid + 0.6
    })
}
