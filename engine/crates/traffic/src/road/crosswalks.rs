//! Marked pedestrian crossings at controlled junctions.

use super::network::{Connector, Network};
use super::types::ControlType;
use crate::ids::{CrosswalkId, NodeId, RoadId};
use crate::math::path::segment_intersect;

/// A marked pedestrian crossing across one road at a junction.
#[derive(Clone, Debug)]
pub struct Crosswalk {
    pub id: CrosswalkId,
    pub node: NodeId,
    pub road: RoadId,
    /// End points: a = right curb of traffic leaving the node, b = left curb.
    pub ax: f64,
    pub ay: f64,
    pub bx: f64,
    pub by: f64,
    pub length: f64,
    /// Heading of the road leaving the node (traffic on it crosses this walk).
    pub road_heading: f64,
}

/// Where a connector path crosses a crosswalk.
#[derive(Clone, Copy, Debug)]
pub struct CrosswalkCrossing {
    pub cw: CrosswalkId,
    /// Arc length on the connector.
    pub s: f64,
    /// Position along the crosswalk, 0 (a) .. 1 (b).
    pub t: f64,
}

/// Builds crosswalks at every controlled junction (signals and signs) and,
/// for every connector (indexed by connector id), the crossings sorted by
/// arc length. Geometry: 2.6 m inside the junction box radius.
pub fn build_crosswalks(net: &Network) -> (Vec<Crosswalk>, Vec<Vec<CrosswalkCrossing>>) {
    let mut crosswalks: Vec<Crosswalk> = Vec::new();
    for n in &net.nodes {
        if n.roads.len() < 3 || n.control == ControlType::None {
            continue;
        }
        for &r in &n.roads {
            let road = net.road(r);
            let h = road.heading_from(n.id);
            let dx = h.cos();
            let dy = h.sin();
            let px = -dy;
            let py = dx;
            let mid = n.radius - 2.6;
            let half = road.width / 2.0 + 0.6;
            let cx = n.x + dx * mid;
            let cy = n.y + dy * mid;
            crosswalks.push(Crosswalk {
                id: CrosswalkId::from(crosswalks.len()),
                node: n.id,
                road: r,
                ax: cx - px * half,
                ay: cy - py * half,
                bx: cx + px * half,
                by: cy + py * half,
                length: half * 2.0,
                road_heading: h,
            });
        }
    }
    let mut crossings = vec![Vec::new(); net.connectors.len()];
    for c in &net.connectors {
        let mut list: Vec<CrosswalkCrossing> = crosswalks.iter().filter(|cw| cw.node == c.node).filter_map(|cw| first_crossing(c, cw)).collect();
        list.sort_by(|a, b| a.s.partial_cmp(&b.s).unwrap_or(std::cmp::Ordering::Equal).then(a.cw.cmp(&b.cw)));
        crossings[c.id.idx()] = list;
    }
    (crosswalks, crossings)
}

fn first_crossing(c: &Connector, cw: &Crosswalk) -> Option<CrosswalkCrossing> {
    let p = &c.path;
    for i in 0..p.xs.len() - 1 {
        if let Some((t, u)) = segment_intersect(p.xs[i], p.ys[i], p.xs[i + 1], p.ys[i + 1], cw.ax, cw.ay, cw.bx, cw.by) {
            return Some(CrosswalkCrossing { cw: cw.id, s: p.cum[i] + t * (p.cum[i + 1] - p.cum[i]), t: u });
        }
    }
    None
}
