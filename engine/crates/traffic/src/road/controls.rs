//! Right-of-way assignment: node control types become per-approach signs.

use super::network::{Network, Road};
use super::types::{ApproachSign, ControlType, RoadKind};
use crate::ids::NodeId;

fn kind_rank(k: RoadKind) -> i32 {
    match k {
        RoadKind::Ring | RoadKind::Boulevard => 5,
        RoadKind::Avenue => 4,
        RoadKind::Oneway => 3,
        RoadKind::Street => 2,
        RoadKind::Alley => 1,
    }
}

/// Importance of a road for right-of-way decisions.
pub fn road_rank(r: &Road) -> i32 {
    kind_rank(r.kind) * 10 + (r.opts.lanes_fwd + r.opts.lanes_back) as i32
}

/// Translate each node's control type into per-approach signs and major-road
/// flags. Call after `Network::finalize()`.
pub fn assign_approach_signs(net: &mut Network) {
    for n in 0..net.nodes.len() {
        assign_node(net, NodeId::from(n));
    }
}

/// Assign signs for one node (also turns a degenerate two-way stop into an all-way stop).
pub fn assign_node(net: &mut Network, nid: NodeId) {
    let mut best = -1;
    for r in &net.nodes[nid.idx()].roads {
        best = best.max(road_rank(&net.roads[r.idx()]));
    }
    let control = net.nodes[nid.idx()].control;
    let aps = net.nodes[nid.idx()].approaches.clone();
    for &ap in &aps {
        let is_major = road_rank(&net.roads[net.approaches[ap.idx()].road.idx()]) >= best;
        let a = &mut net.approaches[ap.idx()];
        match control {
            ControlType::Signal => {
                a.sign = ApproachSign::Signal;
                a.major = is_major;
            }
            ControlType::AllwayStop => {
                a.sign = ApproachSign::Stop;
                a.major = false;
            }
            ControlType::TwowayStop => {
                a.sign = if is_major { ApproachSign::None } else { ApproachSign::Stop };
                a.major = is_major;
            }
            ControlType::Yield => {
                a.sign = if is_major { ApproachSign::None } else { ApproachSign::Yield };
                a.major = is_major;
            }
            ControlType::None => {
                a.sign = ApproachSign::None;
                a.major = true;
            }
        }
    }
    // A two-way stop where every road ties for rank degenerates: make it all-way.
    if control == ControlType::TwowayStop || control == ControlType::Yield {
        let minors = aps.iter().filter(|a| !net.approaches[a.idx()].major).count();
        if minors == 0 && aps.len() >= 3 {
            net.nodes[nid.idx()].control = ControlType::AllwayStop;
            for &ap in &aps {
                let a = &mut net.approaches[ap.idx()];
                a.sign = ApproachSign::Stop;
                a.major = false;
            }
        }
    }
}
