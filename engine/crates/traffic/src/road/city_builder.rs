//! Procedural grid city: perimeter boulevard, avenues, one-way and small streets.

use super::controls::assign_approach_signs;
use super::network::{Network, RoadOptions};
use super::types::{ControlType, RoadKind};
use crate::ids::{NodeId, RoadId};
use crate::math::Rng;

/// What a building block is used for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuildingKind {
    Tower,
    Block,
    Park,
    Plaza,
    Garage,
}

/// An axis-aligned building footprint (centre and size, metres).
#[derive(Clone, Copy, Debug)]
pub struct Building {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    /// Height in metres (0 for parks/plazas).
    pub height: f64,
    pub kind: BuildingKind,
    /// 0..1 hue used by renderers for neon accents.
    pub hue: f64,
}

/// A fixed speed camera on a road.
#[derive(Clone, Copy, Debug)]
pub struct SpeedCamera {
    pub road: RoadId,
    /// Arc length along the road centreline.
    pub s: f64,
    pub forward: bool,
}

/// A generated city: the network plus buildings and cameras.
#[derive(Clone, Debug)]
pub struct City {
    pub network: Network,
    pub buildings: Vec<Building>,
    pub speed_cameras: Vec<SpeedCamera>,
    /// Grid lines.
    pub xs: Vec<f64>,
    pub ys: Vec<f64>,
    pub name: String,
}

/// City generation parameters (all optional; defaults: seed 2077, 7×6 grid,
/// blocks about 110 × 95 m).
#[derive(Clone, Copy, Debug, Default)]
pub struct CityOptions {
    pub seed: Option<i64>,
    pub cols: Option<usize>,
    pub rows: Option<usize>,
    pub block_w: Option<f64>,
    pub block_h: Option<f64>,
}

#[derive(Clone, Debug)]
struct LineSpec {
    kind: RoadKind,
    inc: usize,
    dec: usize,
    speed: f64,
    name: String,
    width: f64,
}

const STREET_NAMES: [&str; 14] = [
    "Kestrel", "Ashgrove", "Ozone", "Delamain", "Ironside", "Halcyon", "Corvid", "Lumen", "Tidewater", "Saffron", "Quartz", "Juniper", "Mercury", "Bellwether",
];
const AVENUE_NAMES: [&str; 4] = ["Meridian Ave", "Vanta Ave", "Arcadia Ave", "Helix Ave"];

fn line_specs(count: usize, rng: &mut Rng, col: bool) -> Vec<LineSpec> {
    let mut specs = Vec::with_capacity(count);
    let mut one_way_toggle = rng.chance(0.5);
    let mut avenue_idx = if col { 0 } else { 2 };
    let mut street_idx = if col { 0 } else { 7 };
    for i in 0..count {
        let mut s;
        if i == 0 || i == count - 1 {
            s = LineSpec { kind: RoadKind::Ring, inc: 2, dec: 2, speed: 70.0, name: "Perimeter Blvd".into(), width: 0.0 };
        } else if i == count / 2 || (count > 8 && i == 2) {
            s = LineSpec { kind: RoadKind::Avenue, inc: 3, dec: 3, speed: 60.0, name: AVENUE_NAMES[avenue_idx].into(), width: 0.0 };
            avenue_idx = if avenue_idx + 1 < AVENUE_NAMES.len() { avenue_idx + 1 } else { 0 };
        } else if rng.chance(0.35) {
            one_way_toggle = !one_way_toggle;
            let lanes = if rng.chance(0.5) { 2 } else { 3 };
            s = LineSpec {
                kind: RoadKind::Oneway,
                inc: if one_way_toggle { lanes } else { 0 },
                dec: if one_way_toggle { 0 } else { lanes },
                speed: 50.0,
                name: format!("{} St", STREET_NAMES[street_idx]),
                width: 0.0,
            };
            street_idx = if street_idx + 1 < STREET_NAMES.len() { street_idx + 1 } else { 0 };
        } else {
            let lanes = if rng.chance(0.3) { 2 } else { 1 };
            s = LineSpec {
                kind: RoadKind::Street,
                inc: lanes,
                dec: lanes,
                speed: if lanes == 2 { 50.0 } else { 40.0 },
                name: format!("{} St", STREET_NAMES[street_idx]),
                width: 0.0,
            };
            street_idx = if street_idx + 1 < STREET_NAMES.len() { street_idx + 1 } else { 0 };
        }
        let median = median_for(&s);
        s.width = (s.inc + s.dec) as f64 * 3.5 + median;
        specs.push(s);
    }
    specs
}

fn median_for(s: &LineSpec) -> f64 {
    if s.inc > 0 && s.dec > 0 {
        match s.kind {
            RoadKind::Avenue => 2.0,
            RoadKind::Ring => 1.5,
            _ => 0.3,
        }
    } else {
        0.0
    }
}

fn choose_control(a: &LineSpec, b: &LineSpec, rng: &mut Rng, roads: usize) -> ControlType {
    if roads <= 2 {
        return ControlType::None;
    }
    let big = |s: &LineSpec| s.kind == RoadKind::Avenue || s.kind == RoadKind::Ring;
    if a.kind == RoadKind::Ring && b.kind == RoadKind::Ring {
        return ControlType::None;
    }
    if big(a) && big(b) {
        return ControlType::Signal;
    }
    if a.kind == RoadKind::Avenue || b.kind == RoadKind::Avenue {
        return ControlType::Signal;
    }
    if a.kind == RoadKind::Ring || b.kind == RoadKind::Ring {
        return if rng.chance(0.4) { ControlType::Signal } else { ControlType::TwowayStop };
    }
    if a.kind == RoadKind::Oneway && b.kind == RoadKind::Oneway {
        return if rng.chance(0.6) { ControlType::Signal } else { ControlType::AllwayStop };
    }
    if a.kind == RoadKind::Oneway || b.kind == RoadKind::Oneway {
        return if rng.chance(0.5) { ControlType::TwowayStop } else { ControlType::Signal };
    }
    if rng.chance(0.65) {
        ControlType::AllwayStop
    } else {
        ControlType::TwowayStop
    }
}

/// Generate a procedural grid city: a perimeter boulevard, multi-lane avenues
/// with protected left turns, one-way streets and small streets with stop
/// signs, plus buildings, red-light and speed cameras.
pub fn build_city(opts: CityOptions) -> City {
    let mut rng = Rng::new(opts.seed.unwrap_or(2077));
    let cols = opts.cols.unwrap_or(7);
    let rows = opts.rows.unwrap_or(6);
    let bw = opts.block_w.unwrap_or(110.0);
    let bh = opts.block_h.unwrap_or(95.0);
    let mut xs = Vec::with_capacity(cols);
    let mut ys = Vec::with_capacity(rows);
    let mut x = 0.0;
    for _ in 0..cols {
        xs.push(x);
        x += bw * rng.range(0.8, 1.2);
    }
    let mut y = 0.0;
    for _ in 0..rows {
        ys.push(y);
        y += bh * rng.range(0.8, 1.2);
    }
    // Centre the city on the origin.
    let cx = xs[cols - 1] / 2.0;
    let cy = ys[rows - 1] / 2.0;
    for v in xs.iter_mut() {
        *v = (*v - cx).floor();
    }
    for v in ys.iter_mut() {
        *v = (*v - cy).floor();
    }

    let col_spec = line_specs(cols, &mut rng, true);
    let row_spec = line_specs(rows, &mut rng, false);
    let mut net = Network::new();
    let mut grid: Vec<Vec<NodeId>> = Vec::with_capacity(cols);
    for i in 0..cols {
        let mut col_nodes = Vec::with_capacity(rows);
        for j in 0..rows {
            let on_edge_x = i == 0 || i == cols - 1;
            let on_edge_y = j == 0 || j == rows - 1;
            let degree = if on_edge_x && on_edge_y {
                2
            } else if on_edge_x || on_edge_y {
                3
            } else {
                4
            };
            let control = choose_control(&col_spec[i], &row_spec[j], &mut rng, degree);
            let n = net.add_node(xs[i], ys[j], control);
            net.nodes[n.idx()].name = format!("{} & {}", col_spec[i].name, row_spec[j].name);
            col_nodes.push(n);
        }
        grid.push(col_nodes);
    }
    let mk = |net: &mut Network, a: NodeId, b: NodeId, s: &LineSpec| {
        let o = RoadOptions {
            lanes_fwd: s.inc,
            lanes_back: s.dec,
            lane_width: None,
            speed_limit_kmh: s.speed,
            kind: s.kind,
            name: s.name.clone(),
            median_width: Some(median_for(s)),
            sidewalk_width: Some(if s.kind == RoadKind::Ring { 4.0 } else { 3.0 }),
            via: Vec::new(),
        };
        net.add_road(a, b, o);
    };
    // Vertical roads (along columns, increasing y = north).
    for i in 0..cols {
        for j in 0..rows - 1 {
            mk(&mut net, grid[i][j], grid[i][j + 1], &col_spec[i]);
        }
    }
    // Horizontal roads (along rows, increasing x = east).
    for j in 0..rows {
        for i in 0..cols - 1 {
            mk(&mut net, grid[i][j], grid[i + 1][j], &row_spec[j]);
        }
    }
    net.finalize();
    assign_approach_signs(&mut net);

    for n in net.nodes.iter_mut() {
        if n.control == ControlType::Signal && rng.chance(0.3) {
            n.red_light_camera = true;
        }
    }

    let mut speed_cameras = Vec::new();
    for r in net.roads.iter_mut() {
        if (r.kind == RoadKind::Avenue || r.kind == RoadKind::Ring) && rng.chance(0.15) && r.center_path.length > 60.0 {
            r.speed_camera = true;
            speed_cameras.push(SpeedCamera { road: r.id, s: r.center_path.length / 2.0, forward: rng.chance(0.5) });
        }
    }

    // Buildings fill each block, inset from the curbs by the sidewalk.
    let mut buildings = Vec::new();
    let max_d = (cx * cx + cy * cy).sqrt();
    for i in 0..cols - 1 {
        for j in 0..rows - 1 {
            let x0 = xs[i] + col_spec[i].width / 2.0 + 3.5;
            let x1 = xs[i + 1] - col_spec[i + 1].width / 2.0 - 3.5;
            let y0 = ys[j] + row_spec[j].width / 2.0 + 3.5;
            let y1 = ys[j + 1] - row_spec[j + 1].width / 2.0 - 3.5;
            if x1 - x0 < 10.0 || y1 - y0 < 10.0 {
                continue;
            }
            let mx = (x0 + x1) / 2.0;
            let my = (y0 + y1) / 2.0;
            let centrality = 1.0 - (mx * mx + my * my).sqrt() / max_d;
            if rng.chance(0.08) {
                let kind = if rng.chance(0.5) { BuildingKind::Park } else { BuildingKind::Plaza };
                let hue = rng.next_f64();
                buildings.push(Building { x: mx, y: my, w: x1 - x0, h: y1 - y0, height: 0.0, kind, hue });
                continue;
            }
            let nx = if x1 - x0 > 70.0 { 2 } else { 1 };
            let ny = if y1 - y0 > 60.0 { 2 } else { 1 };
            let gap = 5.0;
            let cw = (x1 - x0 - gap * (nx - 1) as f64) / nx as f64;
            let ch = (y1 - y0 - gap * (ny - 1) as f64) / ny as f64;
            for a in 0..nx {
                for b in 0..ny {
                    let bx = x0 + a as f64 * (cw + gap) + cw / 2.0;
                    let by = y0 + b as f64 * (ch + gap) + ch / 2.0;
                    let tall = rng.chance(0.25 + centrality * 0.5);
                    let height = if tall { rng.range(45.0, 90.0) + centrality * 120.0 } else { rng.range(10.0, 30.0) };
                    let w = cw * rng.range(0.82, 1.0);
                    let h = ch * rng.range(0.82, 1.0);
                    let kind = if tall {
                        BuildingKind::Tower
                    } else if rng.chance(0.1) {
                        BuildingKind::Garage
                    } else {
                        BuildingKind::Block
                    };
                    let hue = rng.next_f64();
                    buildings.push(Building { x: bx, y: by, w, h, height, kind, hue });
                }
            }
        }
    }
    City { network: net, buildings, speed_cameras, xs, ys, name: "Gridlock City".into() }
}
