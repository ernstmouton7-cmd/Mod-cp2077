//! The drivable network: nodes, roads, lanes, junction connectors and approaches.

use super::types::{ApproachSign, ControlType, RoadKind, TurnType};
use crate::ids::{ApproachId, ConnectorId, LaneId, NodeId, RoadId, SegRef, SignalId};
use crate::math::path::{cubic_bezier, offset_polyline, Path, Vec2};
use crate::math::scalar::{angle_diff, DEG, KMH};
use crate::util::SpatialHash;

/// Region where two connectors come too close to be used at the same time.
#[derive(Clone, Copy, Debug)]
pub struct ConflictZone {
    /// Connector we conflict with.
    pub other: ConnectorId,
    /// Zone on this connector [s0, s1].
    pub s0: f64,
    pub s1: f64,
    /// Zone on the other connector.
    pub os0: f64,
    pub os1: f64,
    pub kind: ConflictKind,
}

/// `Merge`: both connectors end in the same lane; `Cross`: paths intersect.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConflictKind {
    Merge,
    Cross,
}

/// Construction parameters of a road.
#[derive(Clone, Debug)]
pub struct RoadOptions {
    /// Lanes travelling from node A to node B.
    pub lanes_fwd: usize,
    /// Lanes travelling from node B to node A (0 = one-way road).
    pub lanes_back: usize,
    pub lane_width: Option<f64>,
    pub speed_limit_kmh: f64,
    pub kind: RoadKind,
    pub name: String,
    pub median_width: Option<f64>,
    pub sidewalk_width: Option<f64>,
    /// Optional interior points for curved roads (between A and B).
    pub via: Vec<Vec2>,
}

/// A junction or bend.
#[derive(Clone, Debug)]
pub struct Node {
    pub id: NodeId,
    pub x: f64,
    pub y: f64,
    pub control: ControlType,
    pub roads: Vec<RoadId>,
    pub approaches: Vec<ApproachId>,
    pub connectors: Vec<ConnectorId>,
    /// Junction "box" radius: incoming lanes end this far from the centre.
    pub radius: f64,
    /// Signal controller (when `control == Signal`).
    pub signal: Option<SignalId>,
    /// Red-light enforcement camera.
    pub red_light_camera: bool,
    pub name: String,
}

impl Node {
    /// True for a plain bend/merge with no crossing traffic.
    pub fn is_trivial(&self) -> bool {
        self.roads.len() <= 2
    }
}

/// A centreline polyline between two nodes with lanes in each direction.
#[derive(Clone, Debug)]
pub struct Road {
    pub id: RoadId,
    pub a: NodeId,
    pub b: NodeId,
    pub opts: RoadOptions,
    pub lanes_ab: Vec<LaneId>,
    pub lanes_ba: Vec<LaneId>,
    /// Full centreline polyline from A to B (untrimmed).
    pub centerline: Vec<Vec2>,
    pub center_path: Path,
    pub lane_width: f64,
    pub median_width: f64,
    pub sidewalk_width: f64,
    /// m/s
    pub speed_limit: f64,
    pub trim_a: f64,
    pub trim_b: f64,
    /// Has a speed camera somewhere along it.
    pub speed_camera: bool,
    pub kind: RoadKind,
    pub name: String,
    /// Total carriageway width.
    pub width: f64,
}

impl Road {
    /// Lanes arriving at node `n`.
    pub fn lanes_into(&self, n: NodeId) -> &[LaneId] {
        if n == self.b {
            &self.lanes_ab
        } else {
            &self.lanes_ba
        }
    }

    /// Lanes departing from node `n`.
    pub fn lanes_out_of(&self, n: NodeId) -> &[LaneId] {
        if n == self.a {
            &self.lanes_ab
        } else {
            &self.lanes_ba
        }
    }

    pub fn other(&self, n: NodeId) -> NodeId {
        if n == self.a {
            self.b
        } else {
            self.a
        }
    }

    /// Heading of the road when leaving node `n`.
    pub fn heading_from(&self, n: NodeId) -> f64 {
        let pts = &self.centerline;
        if n == self.a {
            return (pts[1].y - pts[0].y).atan2(pts[1].x - pts[0].x);
        }
        let k = pts.len() - 1;
        (pts[k - 1].y - pts[k].y).atan2(pts[k - 1].x - pts[k].x)
    }
}

/// One lane of a road: an offset, trimmed polyline with adjacency links and
/// allowed turns at its end.
#[derive(Clone, Debug)]
pub struct Lane {
    pub id: LaneId,
    pub road: RoadId,
    /// true = travels A→B.
    pub forward: bool,
    /// 0 = innermost (next to the centre line / leftmost).
    pub index: usize,
    pub from: NodeId,
    pub to: NodeId,
    pub path: Path,
    pub width: f64,
    pub outgoing: Vec<ConnectorId>,
    pub incoming: Vec<ConnectorId>,
    /// Adjacent lane closer to the road centre (driver's left).
    pub left: Option<LaneId>,
    /// Adjacent lane closer to the curb (driver's right).
    pub right: Option<LaneId>,
    pub approach: Option<ApproachId>,
    /// Allowed turns at the end of the lane.
    pub turns: Vec<TurnType>,
    pub speed_limit: f64,
    pub length: f64,
}

impl Lane {
    pub fn has_turn(&self, t: TurnType) -> bool {
        self.turns.contains(&t)
    }
}

/// A curved path through a junction from one lane to another.
#[derive(Clone, Debug)]
pub struct Connector {
    pub id: ConnectorId,
    pub node: NodeId,
    pub from: LaneId,
    pub to: LaneId,
    pub turn: TurnType,
    pub path: Path,
    pub length: f64,
    pub speed_limit: f64,
    pub approach: ApproachId,
    pub conflicts: Vec<ConflictZone>,
    /// Connectors starting from the same lane (diverging).
    pub siblings: Vec<ConnectorId>,
    /// For each sibling: arc length (on this connector) where the paths have separated.
    pub sibling_diverge: Vec<f64>,
}

/// The lanes entering a node from one road. Signals and signs attach here.
#[derive(Clone, Debug)]
pub struct Approach {
    pub id: ApproachId,
    pub node: NodeId,
    pub road: RoadId,
    pub lanes: Vec<LaneId>,
    pub sign: ApproachSign,
    /// Major road in a two-way-stop/yield intersection.
    pub major: bool,
    /// Signal group index within the node's controller (-1 = none).
    pub group: i32,
    /// Stop line endpoints (curb side and centre side).
    pub stop_a: Vec2,
    pub stop_b: Vec2,
    /// Direction of travel when entering the node.
    pub heading: f64,
}

/// A lane near a query point.
#[derive(Clone, Copy, Debug)]
pub struct LaneHit {
    pub lane: LaneId,
    pub s: f64,
    pub lateral: f64,
    pub dist: f64,
}

/// The drivable network. Build with `add_node`/`add_road`, then call
/// `finalize()` once.
#[derive(Clone, Debug)]
pub struct Network {
    pub nodes: Vec<Node>,
    pub roads: Vec<Road>,
    pub lanes: Vec<Lane>,
    pub connectors: Vec<Connector>,
    pub approaches: Vec<Approach>,
    lane_hash: SpatialHash<LaneId>,
    connector_hash: SpatialHash<ConnectorId>,
    pub finalized: bool,
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

impl Default for Network {
    fn default() -> Self {
        Self::new()
    }
}

impl Network {
    pub fn new() -> Self {
        Network {
            nodes: Vec::new(),
            roads: Vec::new(),
            lanes: Vec::new(),
            connectors: Vec::new(),
            approaches: Vec::new(),
            lane_hash: SpatialHash::new(40.0),
            connector_hash: SpatialHash::new(40.0),
            finalized: false,
            min_x: f64::INFINITY,
            min_y: f64::INFINITY,
            max_x: f64::NEG_INFINITY,
            max_y: f64::NEG_INFINITY,
        }
    }

    pub fn add_node(&mut self, x: f64, y: f64, control: ControlType) -> NodeId {
        let id = NodeId::from(self.nodes.len());
        self.nodes.push(Node {
            id,
            x,
            y,
            control,
            roads: Vec::new(),
            approaches: Vec::new(),
            connectors: Vec::new(),
            radius: 0.0,
            signal: None,
            red_light_camera: false,
            name: String::new(),
        });
        id
    }

    pub fn add_road(&mut self, a: NodeId, b: NodeId, opts: RoadOptions) -> RoadId {
        let id = RoadId::from(self.roads.len());
        let na = &self.nodes[a.idx()];
        let nb = &self.nodes[b.idx()];
        let mut pts = vec![Vec2::new(na.x, na.y)];
        pts.extend(opts.via.iter().copied());
        pts.push(Vec2::new(nb.x, nb.y));
        let center_path = Path::new(&pts);
        let lane_width = opts.lane_width.unwrap_or(3.5);
        let median_width =
            opts.median_width
                .unwrap_or(if opts.lanes_back > 0 && opts.lanes_fwd > 0 {
                    0.3
                } else {
                    0.0
                });
        let sidewalk_width = opts.sidewalk_width.unwrap_or(3.0);
        let n = (opts.lanes_fwd + opts.lanes_back) as f64;
        let m = if opts.lanes_back == 0 || opts.lanes_fwd == 0 {
            0.0
        } else {
            median_width
        };
        let road = Road {
            id,
            a,
            b,
            lanes_ab: Vec::new(),
            lanes_ba: Vec::new(),
            centerline: pts,
            center_path,
            lane_width,
            median_width,
            sidewalk_width,
            speed_limit: opts.speed_limit_kmh * KMH,
            trim_a: 0.0,
            trim_b: 0.0,
            speed_camera: false,
            kind: opts.kind,
            name: opts.name.clone(),
            width: n * lane_width + m,
            opts,
        };
        self.roads.push(road);
        self.nodes[a.idx()].roads.push(id);
        self.nodes[b.idx()].roads.push(id);
        id
    }

    // ------------------------------------------------------------ accessors

    #[inline]
    pub fn node(&self, id: NodeId) -> &Node {
        &self.nodes[id.idx()]
    }
    #[inline]
    pub fn road(&self, id: RoadId) -> &Road {
        &self.roads[id.idx()]
    }
    #[inline]
    pub fn lane(&self, id: LaneId) -> &Lane {
        &self.lanes[id.idx()]
    }
    #[inline]
    pub fn connector(&self, id: ConnectorId) -> &Connector {
        &self.connectors[id.idx()]
    }
    #[inline]
    pub fn approach(&self, id: ApproachId) -> &Approach {
        &self.approaches[id.idx()]
    }

    /// Geometry of a lane or connector.
    #[inline]
    pub fn seg_path(&self, s: SegRef) -> &Path {
        match s {
            SegRef::Lane(l) => &self.lanes[l.idx()].path,
            SegRef::Connector(c) => &self.connectors[c.idx()].path,
        }
    }

    #[inline]
    pub fn seg_length(&self, s: SegRef) -> f64 {
        match s {
            SegRef::Lane(l) => self.lanes[l.idx()].length,
            SegRef::Connector(c) => self.connectors[c.idx()].length,
        }
    }

    #[inline]
    pub fn seg_speed_limit(&self, s: SegRef) -> f64 {
        match s {
            SegRef::Lane(l) => self.lanes[l.idx()].speed_limit,
            SegRef::Connector(c) => self.connectors[c.idx()].speed_limit,
        }
    }

    /// Lanes of the same road and direction as `l`, index 0 = leftmost.
    pub fn siblings(&self, l: LaneId) -> &[LaneId] {
        let lane = &self.lanes[l.idx()];
        let r = &self.roads[lane.road.idx()];
        if lane.forward {
            &r.lanes_ab
        } else {
            &r.lanes_ba
        }
    }

    /// First connector out of `lane` into `road`.
    pub fn connector_to(&self, lane: LaneId, road: RoadId) -> Option<ConnectorId> {
        self.lanes[lane.idx()]
            .outgoing
            .iter()
            .copied()
            .find(|&c| self.lanes[self.connectors[c.idx()].to.idx()].road == road)
    }

    // ------------------------------------------------------------ building

    pub fn finalize(&mut self) {
        if self.finalized {
            return;
        }
        self.compute_node_radii();
        for r in 0..self.roads.len() {
            self.build_lanes(RoadId::from(r));
        }
        for n in 0..self.nodes.len() {
            self.build_approaches(NodeId::from(n));
        }
        for n in 0..self.nodes.len() {
            self.build_connectors(NodeId::from(n));
        }
        for n in 0..self.nodes.len() {
            self.build_conflicts(NodeId::from(n));
        }
        let mut lane_hash = SpatialHash::new(40.0);
        for l in &self.lanes {
            for i in 0..l.path.xs.len() {
                self.min_x = self.min_x.min(l.path.xs[i]);
                self.min_y = self.min_y.min(l.path.ys[i]);
                self.max_x = self.max_x.max(l.path.xs[i]);
                self.max_y = self.max_y.max(l.path.ys[i]);
            }
            hash_path(&l.path, l.id, &mut lane_hash);
        }
        let mut connector_hash = SpatialHash::new(40.0);
        for c in &self.connectors {
            hash_path(&c.path, c.id, &mut connector_hash);
        }
        self.lane_hash = lane_hash;
        self.connector_hash = connector_hash;
        self.finalized = true;
    }

    fn compute_node_radii(&mut self) {
        for n in 0..self.nodes.len() {
            let mut max_half: f64 = 0.0;
            for r in &self.nodes[n].roads {
                max_half = max_half.max(self.roads[r.idx()].width / 2.0);
            }
            let node = &mut self.nodes[n];
            node.radius = if node.roads.len() <= 2 {
                // Bends: small box so the curve stays close to the corner.
                if node.roads.len() == 2 {
                    (max_half * 0.9).max(4.0)
                } else {
                    2.0
                }
            } else {
                // Room for the crosswalk band between stop line and conflict area.
                max_half + 4.5
            };
        }
        for r in &mut self.roads {
            r.trim_a = self.nodes[r.a.idx()].radius;
            r.trim_b = self.nodes[r.b.idx()].radius;
            let max_trim = r.center_path.length * 0.45;
            if r.trim_a > max_trim {
                r.trim_a = max_trim;
            }
            if r.trim_b > max_trim {
                r.trim_b = max_trim;
            }
        }
    }

    fn build_lanes(&mut self, rid: RoadId) {
        let r = &self.roads[rid.idx()];
        let n_f = r.opts.lanes_fwd;
        let n_b = r.opts.lanes_back;
        let w = r.lane_width;
        let one_way = n_f == 0 || n_b == 0;
        let half_median = if one_way { 0.0 } else { r.median_width / 2.0 };
        let full_len = r.center_path.length;
        let speed_limit = r.speed_limit;
        let (a, b, trim_a, trim_b) = (r.a, r.b, r.trim_a, r.trim_b);
        let mut new_lanes: Vec<(bool, usize, Path)> = Vec::new();
        for i in 0..n_f {
            let off = if one_way {
                -(i as f64 + 0.5) * w + (n_f as f64 * w) / 2.0
            } else {
                -(half_median + (i as f64 + 0.5) * w)
            };
            let pts = offset_polyline(&r.centerline, off);
            let full = Path::new(&pts);
            let trimmed = Path::new(&full.slice(
                trim_a * (full.length / full_len),
                full.length - trim_b * (full.length / full_len),
            ));
            new_lanes.push((true, i, trimmed));
        }
        let rev: Vec<Vec2> = r.centerline.iter().rev().copied().collect();
        for i in 0..n_b {
            let off = if one_way {
                -(i as f64 + 0.5) * w + (n_b as f64 * w) / 2.0
            } else {
                -(half_median + (i as f64 + 0.5) * w)
            };
            let pts = offset_polyline(&rev, off);
            let full = Path::new(&pts);
            let trimmed = Path::new(&full.slice(
                trim_b * (full.length / full_len),
                full.length - trim_a * (full.length / full_len),
            ));
            new_lanes.push((false, i, trimmed));
        }
        for (forward, index, path) in new_lanes {
            let id = LaneId::from(self.lanes.len());
            let length = path.length;
            self.lanes.push(Lane {
                id,
                road: rid,
                forward,
                index,
                from: if forward { a } else { b },
                to: if forward { b } else { a },
                path,
                width: w,
                outgoing: Vec::new(),
                incoming: Vec::new(),
                left: None,
                right: None,
                approach: None,
                turns: Vec::new(),
                speed_limit,
                length,
            });
            let r = &mut self.roads[rid.idx()];
            if forward {
                r.lanes_ab.push(id);
            } else {
                r.lanes_ba.push(id);
            }
        }
        let groups = [
            self.roads[rid.idx()].lanes_ab.clone(),
            self.roads[rid.idx()].lanes_ba.clone(),
        ];
        for group in groups {
            for i in 0..group.len() {
                let l = &mut self.lanes[group[i].idx()];
                l.left = if i > 0 { Some(group[i - 1]) } else { None };
                l.right = if i + 1 < group.len() {
                    Some(group[i + 1])
                } else {
                    None
                };
            }
        }
    }

    fn build_approaches(&mut self, nid: NodeId) {
        let roads = self.nodes[nid.idx()].roads.clone();
        for rid in roads {
            let lanes = self.roads[rid.idx()].lanes_into(nid).to_vec();
            if lanes.is_empty() {
                continue;
            }
            let ap_id = ApproachId::from(self.approaches.len());
            for &l in &lanes {
                self.lanes[l.idx()].approach = Some(ap_id);
            }
            let first = &self.lanes[lanes[0].idx()];
            let last = &self.lanes[lanes[lanes.len() - 1].idx()];
            let sf = first.path.sample(first.length);
            let heading = sf.heading;
            let nx = -heading.sin();
            let ny = heading.cos();
            let stop_b = Vec2::new(sf.x + nx * first.width * 0.5, sf.y + ny * first.width * 0.5);
            let sl = last.path.sample(last.length);
            let stop_a = Vec2::new(sl.x - nx * last.width * 0.5, sl.y - ny * last.width * 0.5);
            self.approaches.push(Approach {
                id: ap_id,
                node: nid,
                road: rid,
                lanes,
                sign: ApproachSign::None,
                major: false,
                group: -1,
                stop_a,
                stop_b,
                heading,
            });
            self.nodes[nid.idx()].approaches.push(ap_id);
        }
    }

    fn turn_between(in_heading: f64, out_heading: f64) -> TurnType {
        let d = angle_diff(in_heading, out_heading);
        if d.abs() > 150.0 * DEG {
            TurnType::UTurn
        } else if d > 35.0 * DEG {
            TurnType::Left
        } else if d < -35.0 * DEG {
            TurnType::Right
        } else {
            TurnType::Straight
        }
    }

    fn build_connectors(&mut self, nid: NodeId) {
        let aps = self.nodes[nid.idx()].approaches.clone();
        let node_roads = self.nodes[nid.idx()].roads.clone();
        for ap_id in aps {
            let ap_road = self.approaches[ap_id.idx()].road;
            let ap_heading = self.approaches[ap_id.idx()].heading;
            let in_lanes = self.approaches[ap_id.idx()].lanes.clone();
            let k = in_lanes.len();
            // Classify outgoing roads by turn type.
            let mut outs: Vec<(RoadId, TurnType)> = Vec::new();
            for &r in &node_roads {
                if r == ap_road && node_roads.len() > 1 {
                    continue;
                }
                let road = &self.roads[r.idx()];
                if road.lanes_out_of(nid).is_empty() {
                    continue;
                }
                let t = Self::turn_between(ap_heading, road.heading_from(nid));
                if t == TurnType::UTurn && node_roads.len() > 1 {
                    continue;
                }
                outs.push((r, t));
            }
            let has_l = outs.iter().any(|o| o.1 == TurnType::Left);
            let has_s = outs.iter().any(|o| o.1 == TurnType::Straight);
            let has_r = outs.iter().any(|o| o.1 == TurnType::Right);
            // Lane-use assignment (right-hand traffic).
            for (i, &lid) in in_lanes.iter().enumerate() {
                let t = &mut self.lanes[lid.idx()].turns;
                if has_s {
                    let dedicated_left = k >= 3 && has_l;
                    if !(dedicated_left && i == 0) {
                        t.push(TurnType::Straight);
                    }
                    if has_l && i == 0 {
                        t.push(TurnType::Left);
                    }
                    if has_r && i == k - 1 {
                        t.push(TurnType::Right);
                    }
                } else if has_l && has_r {
                    if k == 1 {
                        t.push(TurnType::Left);
                        t.push(TurnType::Right);
                    } else if i < k.div_ceil(2) {
                        t.push(TurnType::Left);
                    } else {
                        t.push(TurnType::Right);
                    }
                } else if has_l {
                    t.push(TurnType::Left);
                } else if has_r {
                    t.push(TurnType::Right);
                }
                for o in &outs {
                    if o.1 == TurnType::UTurn {
                        t.push(TurnType::UTurn);
                    }
                }
            }
            // Create connectors.
            for &(orid, turn) in &outs {
                let out_lanes = self.roads[orid.idx()].lanes_out_of(nid).to_vec();
                let m = out_lanes.len() as i64;
                let users: Vec<LaneId> = in_lanes
                    .iter()
                    .copied()
                    .filter(|&l| self.lanes[l.idx()].has_turn(turn))
                    .collect();
                for (u, &from) in users.iter().enumerate() {
                    let u = u as i64;
                    let pick = |i: i64| out_lanes[i.clamp(0, m - 1) as usize];
                    let target = match turn {
                        TurnType::Left | TurnType::UTurn => pick(u),
                        TurnType::Right => pick(m - users.len() as i64 + u),
                        TurnType::Straight => {
                            // Keep lane index; when lanes drop, the inner ones merge.
                            let fi = self.lanes[from.idx()].index as i64;
                            if m >= k as i64 {
                                pick(fi)
                            } else {
                                pick(fi - (k as i64 - m))
                            }
                        }
                    };
                    let c = self.make_connector(nid, from, target, turn);
                    self.lanes[from.idx()].outgoing.push(c);
                    self.lanes[target.idx()].incoming.push(c);
                    self.nodes[nid.idx()].connectors.push(c);
                }
            }
        }
        let conns = self.nodes[nid.idx()].connectors.clone();
        for &cid in &conns {
            let from = self.connectors[cid.idx()].from;
            let outgoing = self.lanes[from.idx()].outgoing.clone();
            let mut sibs = Vec::new();
            let mut divs = Vec::new();
            for d in outgoing {
                if d == cid {
                    continue;
                }
                sibs.push(d);
                let c = &self.connectors[cid.idx()];
                let dp = &self.connectors[d.idx()].path;
                // Walk along c until d is more than a car width away.
                let mut sep = c.length;
                let steps = ((c.length / 0.5).ceil() as usize).max(4);
                for k in 1..=steps {
                    let sk = (c.length * k as f64) / steps as f64;
                    let p = c.path.sample(sk);
                    let pr = dp.project(p.x, p.y);
                    if pr.d2 > 2.3 * 2.3 {
                        sep = sk;
                        break;
                    }
                }
                divs.push(sep);
            }
            let c = &mut self.connectors[cid.idx()];
            c.siblings = sibs;
            c.sibling_diverge = divs;
        }
    }

    fn make_connector(
        &mut self,
        n: NodeId,
        from: LaneId,
        to: LaneId,
        turn: TurnType,
    ) -> ConnectorId {
        let fl = &self.lanes[from.idx()];
        let tl = &self.lanes[to.idx()];
        let a = fl.path.sample(fl.length);
        let p0 = Vec2::new(a.x, a.y);
        let h0 = a.heading;
        let b = tl.path.sample(0.0);
        let p3 = Vec2::new(b.x, b.y);
        let h3 = b.heading;
        let dx = p3.x - p0.x;
        let dy = p3.y - p0.y;
        let d = (dx * dx + dy * dy).sqrt();
        let pts = if turn == TurnType::Straight && angle_diff(h0, h3).abs() < 5.0 * DEG {
            vec![p0, p3]
        } else {
            let k = if turn == TurnType::UTurn {
                (d * 1.2).max(4.0)
            } else {
                d * 0.45
            };
            let p1 = Vec2::new(p0.x + h0.cos() * k, p0.y + h0.sin() * k);
            let p2 = Vec2::new(p3.x - h3.cos() * k, p3.y - h3.sin() * k);
            cubic_bezier(p0, p1, p2, p3, 16)
        };
        let path = Path::new(&pts);
        let length = path.length;
        // Comfortable lateral acceleration ~2.6 m/s² through the turn.
        let mut max_curv: f64 = 0.0;
        let steps = ((path.length / 2.0).floor() as usize).max(2);
        for i in 1..steps {
            let c = path
                .curvature_at((path.length * i as f64) / steps as f64, 1.5)
                .abs();
            if c > max_curv {
                max_curv = c;
            }
        }
        let turn_speed = if max_curv > 1e-4 {
            (2.6 / max_curv).sqrt()
        } else {
            99.0
        };
        let speed_limit = fl.speed_limit.max(tl.speed_limit).min(turn_speed.max(3.5));
        let approach = fl.approach.expect("connector from a lane without approach");
        let id = ConnectorId::from(self.connectors.len());
        self.connectors.push(Connector {
            id,
            node: n,
            from,
            to,
            turn,
            path,
            length,
            speed_limit,
            approach,
            conflicts: Vec::new(),
            siblings: Vec::new(),
            sibling_diverge: Vec::new(),
        });
        id
    }

    fn build_conflicts(&mut self, nid: NodeId) {
        let cs = self.nodes[nid.idx()].connectors.clone();
        for i in 0..cs.len() {
            for j in i + 1..cs.len() {
                let a = &self.connectors[cs[i].idx()];
                let b = &self.connectors[cs[j].idx()];
                if a.from == b.from {
                    continue; // diverging: handled as siblings
                }
                let merge = a.to == b.to;
                let clearance = 2.4;
                let mut s0 = -1.0;
                let mut s1 = -1.0;
                let mut os0 = f64::INFINITY;
                let mut os1 = f64::NEG_INFINITY;
                let steps = ((a.length / 0.75).ceil() as usize).max(4);
                for k in 0..=steps {
                    let s = (a.length * k as f64) / steps as f64;
                    let p = a.path.sample(s);
                    let pr = b.path.project(p.x, p.y);
                    if pr.d2 < clearance * clearance && pr.s > 0.01 && pr.s < b.length + 0.01 {
                        if s0 < 0.0 {
                            s0 = s;
                        }
                        s1 = s;
                        if pr.s < os0 {
                            os0 = pr.s;
                        }
                        if pr.s > os1 {
                            os1 = pr.s;
                        }
                    }
                }
                if s0 < 0.0 {
                    continue;
                }
                let kind = if merge {
                    ConflictKind::Merge
                } else {
                    ConflictKind::Cross
                };
                if merge {
                    s1 = a.length;
                    os1 = b.length;
                }
                let (al, bl, aid, bid) = (a.length, b.length, a.id, b.id);
                self.connectors[cs[i].idx()].conflicts.push(ConflictZone {
                    other: bid,
                    s0: (s0 - 1.0).max(0.0),
                    s1: al.min(s1 + 1.0),
                    os0: (os0 - 1.0).max(0.0),
                    os1: bl.min(os1 + 1.0),
                    kind,
                });
                self.connectors[cs[j].idx()].conflicts.push(ConflictZone {
                    other: aid,
                    s0: (os0 - 1.0).max(0.0),
                    s1: bl.min(os1 + 1.0),
                    os0: (s0 - 1.0).max(0.0),
                    os1: al.min(s1 + 1.0),
                    kind,
                });
            }
        }
    }

    // ------------------------------------------------------------ queries

    /// Lanes within `radius` of a point, ordered by distance (ties by id).
    pub fn nearest_lanes(&self, x: f64, y: f64, radius: f64) -> Vec<LaneHit> {
        let mut cands = Vec::new();
        self.lane_hash.query(x, y, radius, &mut cands);
        let mut out = Vec::with_capacity(cands.len());
        for l in cands {
            let pr = self.lanes[l.idx()].path.project(x, y);
            let d = pr.d2.sqrt();
            if d <= radius {
                out.push(LaneHit {
                    lane: l,
                    s: pr.s,
                    lateral: pr.lateral,
                    dist: d,
                });
            }
        }
        out.sort_by(|a, b| {
            a.dist
                .partial_cmp(&b.dist)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.lane.cmp(&b.lane))
        });
        out
    }

    /// Connectors whose geometry passes within roughly `radius` of a point.
    pub fn nearest_connectors(&self, x: f64, y: f64, radius: f64, out: &mut Vec<ConnectorId>) {
        self.connector_hash.query(x, y, radius, out);
    }

    /// Node whose junction box contains the point, if any (3+ roads only).
    pub fn node_at(&self, x: f64, y: f64) -> Option<NodeId> {
        let mut best = None;
        let mut best_d = f64::INFINITY;
        for n in &self.nodes {
            if n.roads.len() < 3 {
                continue;
            }
            let dx = x - n.x;
            let dy = y - n.y;
            let d = dx * dx + dy * dy;
            let r = n.radius - 1.0;
            if d < r * r && d < best_d {
                best_d = d;
                best = Some(n.id);
            }
        }
        best
    }
}

fn hash_path<T: Copy + PartialEq>(path: &Path, item: T, hash: &mut SpatialHash<T>) {
    let step = 20.0;
    let n = ((path.length / step).ceil() as usize).max(1);
    for i in 0..=n {
        let p = path.sample((path.length * i as f64) / n as f64);
        hash.insert(item, p.x, p.y, 12.0);
    }
}
