//! Road-level A* routing with turn penalties.

use super::network::Network;
use super::types::{ControlType, TurnType};
use crate::ids::{LaneId, NodeId, RoadId};
use std::collections::HashMap;

/// A step of a route: drive along `road` in the given direction, arriving at `to`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RouteStep {
    pub road: RoadId,
    pub forward: bool,
    pub to: NodeId,
}

struct OpenEntry {
    f: f64,
    g: f64,
    node: NodeId,
    /// Road we arrived by (for turn costs).
    via: Option<RoadId>,
    parent: Option<usize>,
    step_road: Option<RoadId>,
    step_forward: bool,
}

/// Road-level A* with turn penalties, junction delays and optional
/// congestion costs. Routes are lists of road steps after the vehicle's
/// current lane.
#[derive(Clone, Debug)]
pub struct Router {
    /// Extra seconds per road from congestion.
    pub congestion: HashMap<RoadId, f64>,
    max_speed: f64,
}

impl Router {
    pub fn new(net: &Network) -> Self {
        let mut max_speed: f64 = 1.0;
        for r in &net.roads {
            max_speed = max_speed.max(r.speed_limit);
        }
        Router {
            congestion: HashMap::new(),
            max_speed,
        }
    }

    fn control_delay(net: &Network, n: NodeId) -> f64 {
        match net.node(n).control {
            ControlType::Signal => 12.0,
            ControlType::AllwayStop => 7.0,
            ControlType::TwowayStop => 4.0,
            _ => 0.5,
        }
    }

    /// Route from the end of `start_lane` (its `to` node) onto `goal_road` in
    /// the given direction, or `None` if unreachable.
    pub fn route(
        &self,
        net: &Network,
        start_lane: LaneId,
        goal_road: RoadId,
        goal_forward: bool,
    ) -> Option<Vec<RouteStep>> {
        let gr = net.road(goal_road);
        let goal_node_from = if goal_forward { gr.a } else { gr.b };
        let goal_to = if goal_forward { gr.b } else { gr.a };
        let sl = net.lane(start_lane);
        let start = sl.to;
        let gn = net.node(goal_node_from);
        let (gx, gy) = (gn.x, gn.y);
        let h = |n: NodeId| {
            let nn = net.node(n);
            ((nn.x - gx) * (nn.x - gx) + (nn.y - gy) * (nn.y - gy)).sqrt() / self.max_speed
        };
        let mut arena: Vec<OpenEntry> = Vec::new();
        let mut open: Vec<usize> = Vec::new();
        let mut best: HashMap<(NodeId, Option<RoadId>), f64> = HashMap::new();
        arena.push(OpenEntry {
            f: h(start),
            g: 0.0,
            node: start,
            via: Some(sl.road),
            parent: None,
            step_road: None,
            step_forward: true,
        });
        open.push(0);
        let mut iterations = 0;
        while !open.is_empty() && iterations < 20000 {
            iterations += 1;
            // Pop lowest f (linear scan; graphs are small).
            let mut bi = 0;
            for i in 1..open.len() {
                let a = &arena[open[i]];
                let b = &arena[open[bi]];
                if a.f < b.f || (a.f == b.f && a.node < b.node) {
                    bi = i;
                }
            }
            let cur_i = open.swap_remove(bi);
            let (cur_node, cur_via, cur_g) = (arena[cur_i].node, arena[cur_i].via, arena[cur_i].g);
            if cur_node == goal_node_from
                && cur_via != Some(goal_road)
                && self.can_turn(net, cur_node, cur_via, goal_road, goal_forward)
            {
                let mut steps = vec![RouteStep {
                    road: goal_road,
                    forward: goal_forward,
                    to: goal_to,
                }];
                let mut e = Some(cur_i);
                while let Some(ei) = e {
                    let en = &arena[ei];
                    let Some(sr) = en.step_road else { break };
                    steps.push(RouteStep {
                        road: sr,
                        forward: en.step_forward,
                        to: en.node,
                    });
                    e = en.parent;
                }
                steps.reverse();
                return Some(steps);
            }
            let node = net.node(cur_node);
            for &r in &node.roads {
                if Some(r) == cur_via && node.roads.len() > 1 {
                    continue;
                }
                let road = net.road(r);
                let fwd = road.a == cur_node;
                if !self.can_turn(net, cur_node, cur_via, r, fwd) {
                    continue;
                }
                let next = road.other(cur_node);
                let lanes = if fwd { &road.lanes_ab } else { &road.lanes_ba };
                let cost = road.center_path.length / road.speed_limit
                    + Self::control_delay(net, next)
                    + self.congestion.get(&r).copied().unwrap_or(0.0)
                    + Self::turn_cost(net, cur_node, cur_via, r)
                    + 3.0 / lanes.len() as f64;
                let g = cur_g + cost;
                let k = (next, Some(r));
                if let Some(&prev) = best.get(&k) {
                    if prev <= g {
                        continue;
                    }
                }
                best.insert(k, g);
                arena.push(OpenEntry {
                    f: g + h(next),
                    g,
                    node: next,
                    via: Some(r),
                    parent: Some(cur_i),
                    step_road: Some(r),
                    step_forward: fwd,
                });
                open.push(arena.len() - 1);
            }
        }
        None
    }

    /// Is there any lane connection from road `from` into road `to` (in the
    /// given direction) at node `n`?
    pub fn can_turn(
        &self,
        net: &Network,
        n: NodeId,
        from: Option<RoadId>,
        to: RoadId,
        forward: bool,
    ) -> bool {
        let tr = net.road(to);
        let out_lanes = if forward { &tr.lanes_ab } else { &tr.lanes_ba };
        if out_lanes.is_empty() {
            return false;
        }
        if (if forward { tr.a } else { tr.b }) != n {
            return false;
        }
        let Some(from) = from else { return true };
        for &l in net.road(from).lanes_into(n) {
            for &c in &net.lane(l).outgoing {
                if net.lane(net.connector(c).to).road == to {
                    return true;
                }
            }
        }
        false
    }

    fn turn_cost(net: &Network, n: NodeId, from: Option<RoadId>, to: RoadId) -> f64 {
        let Some(from) = from else { return 0.0 };
        for &l in net.road(from).lanes_into(n) {
            for &c in &net.lane(l).outgoing {
                let cc = net.connector(c);
                if net.lane(cc.to).road == to {
                    return match cc.turn {
                        TurnType::Left => 8.0,
                        TurnType::Right => 3.0,
                        _ => 0.0,
                    };
                }
            }
        }
        0.0
    }
}
