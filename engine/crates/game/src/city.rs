//! Static city geometry derived from the traffic network: road surface, raised blocks with
//! curbs, buildings, lane markings, crosswalks, signal/sign placements and street furniture.
//! Everything is in world coordinates (see `coords`).

use crate::coords::*;
use glam::{Quat, Vec2, Vec3};
use gridlock_physics::props::PropKind;
use gridlock_physics::{BodyDesc, ColliderDesc, PhysicsWorld, Pose, Shape, Surface};
use gridlock_traffic::road::BuildingKind;
use gridlock_traffic::{ApproachSign, ControlType, World};

pub const CURB_HEIGHT: f32 = 0.15;
/// Kerb radius at block corners (m).
pub const CORNER_RADIUS: f32 = 6.0;

#[derive(Clone, Copy, Debug)]
pub struct BlockGeom {
    /// World XZ bounds (x, z).
    pub min: Vec2,
    pub max: Vec2,
    pub sidewalk: f32,
    /// Outside the road grid (verges around the city).
    pub outer: bool,
}

impl BlockGeom {
    /// Footprint outline (XZ, counter-clockwise seen from above) with rounded corners.
    pub fn outline(&self, radius: f32) -> Vec<Vec2> {
        let r = radius
            .min((self.max.x - self.min.x) * 0.45)
            .min((self.max.y - self.min.y) * 0.45)
            .max(0.0);
        if r < 0.05 {
            return vec![
                self.min,
                Vec2::new(self.max.x, self.min.y),
                self.max,
                Vec2::new(self.min.x, self.max.y),
            ];
        }
        let corners = [
            (
                Vec2::new(self.max.x - r, self.min.y + r),
                -std::f32::consts::FRAC_PI_2,
            ),
            (Vec2::new(self.max.x - r, self.max.y - r), 0.0),
            (
                Vec2::new(self.min.x + r, self.max.y - r),
                std::f32::consts::FRAC_PI_2,
            ),
            (
                Vec2::new(self.min.x + r, self.min.y + r),
                std::f32::consts::PI,
            ),
        ];
        let seg = 6;
        let mut out = Vec::with_capacity(4 * (seg + 1));
        for (c, a0) in corners {
            for k in 0..=seg {
                let a = a0 + std::f32::consts::FRAC_PI_2 * k as f32 / seg as f32;
                out.push(c + Vec2::new(a.cos(), a.sin()) * r);
            }
        }
        out
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuildingStyle {
    Tower,
    Block,
    Garage,
    Park,
    Plaza,
}

#[derive(Clone, Copy, Debug)]
pub struct BuildingGeom {
    /// Centre of the footprint at street level.
    pub base: Vec3,
    /// Full size (x, height, z).
    pub size: Vec3,
    pub style: BuildingStyle,
    pub hue: f32,
    /// Stable per-building random seed for facade variation.
    pub seed: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkKind {
    WhiteSolid,
    WhiteDashed,
    YellowSolid,
    StopLine,
}

#[derive(Clone, Debug)]
pub struct Marking {
    pub kind: MarkKind,
    /// Polyline on the road surface (y = small offset above 0).
    pub points: Vec<Vec3>,
    pub width: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct CrosswalkGeom {
    pub a: Vec3,
    pub b: Vec3,
    /// Stripe band width along the road (m).
    pub width: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct SignalHeadGeom {
    pub node: u32,
    pub approach: u32,
    /// Pole base position.
    pub pole: Vec3,
    /// Where the head hangs (over the lanes, ~5.5 m up).
    pub head: Vec3,
    /// Direction the lenses face (toward approaching traffic), world XZ unit vector.
    pub facing: Vec3,
}

#[derive(Clone, Copy, Debug)]
pub struct SignGeom {
    pub pos: Vec3,
    pub facing: Vec3,
    pub stop: bool,
}

#[derive(Clone, Debug, Default)]
pub struct StaticCity {
    pub min: Vec2,
    pub max: Vec2,
    pub blocks: Vec<BlockGeom>,
    pub buildings: Vec<BuildingGeom>,
    pub markings: Vec<Marking>,
    pub crosswalks: Vec<CrosswalkGeom>,
    pub signal_heads: Vec<SignalHeadGeom>,
    pub signs: Vec<SignGeom>,
    pub props: Vec<(PropKind, Pose)>,
    /// Street-lamp light positions (lamp heads), for the renderer's local lights.
    pub lamp_lights: Vec<Vec3>,
}

fn hash(a: u32, b: u32) -> u32 {
    let mut x = a.wrapping_mul(0x9E37_79B1) ^ b.wrapping_mul(0x85EB_CA77);
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    x
}

fn frac(h: u32) -> f32 {
    (h >> 8) as f32 / 16_777_216.0
}

impl StaticCity {
    pub fn build(world: &World) -> Self {
        let net = world.net();
        let xs = &world.grid_xs;
        let ys = &world.grid_ys;
        // Road widths along each grid line.
        let mut col_w = vec![7.0f64; xs.len()];
        let mut row_w = vec![7.0f64; ys.len()];
        let mut col_sw = vec![3.0f64; xs.len()];
        let mut row_sw = vec![3.0f64; ys.len()];
        for r in &net.roads {
            let (a, b) = (net.node(r.a), net.node(r.b));
            if (a.x - b.x).abs() < 0.5 {
                if let Some(i) = xs.iter().position(|x| (x - a.x).abs() < 0.5) {
                    col_w[i] = r.width;
                    col_sw[i] = r.sidewalk_width;
                }
            } else if (a.y - b.y).abs() < 0.5 {
                if let Some(j) = ys.iter().position(|y| (y - a.y).abs() < 0.5) {
                    row_w[j] = r.width;
                    row_sw[j] = r.sidewalk_width;
                }
            }
        }
        let mut city = StaticCity::default();
        let margin = 70.0;
        let (x_lo, x_hi) = (
            xs[0] - col_w[0] / 2.0,
            xs[xs.len() - 1] + col_w[xs.len() - 1] / 2.0,
        );
        let (y_lo, y_hi) = (
            ys[0] - row_w[0] / 2.0,
            ys[ys.len() - 1] + row_w[ys.len() - 1] / 2.0,
        );
        // Blocks between grid lines.
        for i in 0..xs.len() - 1 {
            for j in 0..ys.len() - 1 {
                let x0 = xs[i] + col_w[i] / 2.0;
                let x1 = xs[i + 1] - col_w[i + 1] / 2.0;
                let y0 = ys[j] + row_w[j] / 2.0;
                let y1 = ys[j + 1] - row_w[j + 1] / 2.0;
                if x1 <= x0 || y1 <= y0 {
                    continue;
                }
                let sw = col_sw[i]
                    .max(col_sw[i + 1])
                    .max(row_sw[j])
                    .max(row_sw[j + 1]) as f32;
                city.blocks.push(BlockGeom {
                    min: Vec2::new(x0 as f32, -(y1 as f32)),
                    max: Vec2::new(x1 as f32, -(y0 as f32)),
                    sidewalk: sw,
                    outer: false,
                });
            }
        }
        // Verges around the perimeter.
        let m = margin;
        for (a, b) in [
            ((x_lo - m, y_lo - m), (x_lo, y_hi + m)),
            ((x_hi, y_lo - m), (x_hi + m, y_hi + m)),
            ((x_lo, y_lo - m), (x_hi, y_lo)),
            ((x_lo, y_hi), (x_hi, y_hi + m)),
        ] {
            city.blocks.push(BlockGeom {
                min: Vec2::new(a.0 as f32, -(b.1 as f32)),
                max: Vec2::new(b.0 as f32, -(a.1 as f32)),
                sidewalk: 4.0,
                outer: true,
            });
        }
        city.min = Vec2::new((x_lo - m) as f32, -((y_hi + m) as f32));
        city.max = Vec2::new((x_hi + m) as f32, -((y_lo - m) as f32));

        // Buildings.
        for (k, b) in world.buildings.iter().enumerate() {
            let style = match b.kind {
                BuildingKind::Tower => BuildingStyle::Tower,
                BuildingKind::Block => BuildingStyle::Block,
                BuildingKind::Garage => BuildingStyle::Garage,
                BuildingKind::Park => BuildingStyle::Park,
                BuildingKind::Plaza => BuildingStyle::Plaza,
            };
            city.buildings.push(BuildingGeom {
                base: to_world(b.x, b.y, CURB_HEIGHT),
                size: Vec3::new(b.w as f32, b.height as f32, b.h as f32),
                style,
                hue: b.hue as f32,
                seed: hash(k as u32, 77),
            });
        }

        // Lane markings.
        let lift = 0.006;
        for lane in &net.lanes {
            let path = &lane.path;
            let len = path.length;
            if len < 2.0 {
                continue;
            }
            let road = net.road(lane.road);
            let two_way = !road.lanes_ab.is_empty() && !road.lanes_ba.is_empty();
            let half = lane.width / 2.0;
            let n = (len / 2.0).ceil().max(1.0) as usize;
            let mut left_pts = Vec::with_capacity(n + 1);
            let mut right_pts = Vec::with_capacity(n + 1);
            for k in 0..=n {
                let s = len * k as f64 / n as f64;
                let p = path.sample(s);
                let (lx, ly) = (-p.heading.sin(), p.heading.cos());
                left_pts.push(to_world(p.x + lx * half, p.y + ly * half, lift));
                right_pts.push(to_world(p.x - lx * half, p.y - ly * half, lift));
            }
            if lane.right.is_none() {
                city.markings.push(Marking {
                    kind: MarkKind::WhiteSolid,
                    points: right_pts,
                    width: 0.12,
                });
            }
            match lane.left {
                Some(_) => city.markings.push(Marking {
                    kind: MarkKind::WhiteDashed,
                    points: left_pts,
                    width: 0.12,
                }),
                None => city.markings.push(Marking {
                    kind: if two_way {
                        MarkKind::YellowSolid
                    } else {
                        MarkKind::WhiteSolid
                    },
                    points: left_pts,
                    width: 0.12,
                }),
            }
        }
        // Stop lines, signal heads and signs.
        for ap in &net.approaches {
            let node = net.node(ap.node);
            let a = to_world(ap.stop_a.x, ap.stop_a.y, lift);
            let b = to_world(ap.stop_b.x, ap.stop_b.y, lift);
            let travel = to_world(ap.heading.cos(), ap.heading.sin(), 0.0).normalize_or_zero();
            let facing = -travel;
            match ap.sign {
                ApproachSign::Signal | ApproachSign::Stop => {
                    // Shift the line back slightly so it sits just before the stop position.
                    let off = -travel * 0.3;
                    city.markings.push(Marking {
                        kind: MarkKind::StopLine,
                        points: vec![a + off, b + off],
                        width: 0.45,
                    });
                }
                _ => {}
            }
            let curb_dir = (a - b).normalize_or_zero();
            match ap.sign {
                ApproachSign::Signal if node.control == ControlType::Signal => {
                    // Pole on the far-side corner of the junction, arm reaching over the lanes.
                    let across = node.radius as f32 * 2.0;
                    let pole = a + travel * across + curb_dir * 1.6;
                    let reach = (a - b).length() * 0.55 + 1.6;
                    let head = pole - curb_dir * reach + Vec3::Y * 5.6;
                    city.signal_heads.push(SignalHeadGeom {
                        node: ap.node.0,
                        approach: ap.id.0,
                        pole: Vec3::new(pole.x, CURB_HEIGHT, pole.z),
                        head,
                        facing,
                    });
                    let yaw = Quat::from_rotation_arc(
                        -Vec3::Z,
                        Vec3::new(-curb_dir.x, 0.0, -curb_dir.z).normalize_or(-Vec3::Z),
                    );
                    city.props.push((
                        PropKind::SignalPole,
                        Pose::new(Vec3::new(pole.x, CURB_HEIGHT, pole.z), yaw),
                    ));
                }
                ApproachSign::Stop | ApproachSign::Yield => {
                    let pos = a + curb_dir * 1.2 - travel * 0.5;
                    city.signs.push(SignGeom {
                        pos: Vec3::new(pos.x, CURB_HEIGHT, pos.z),
                        facing,
                        stop: ap.sign == ApproachSign::Stop,
                    });
                    let yaw = Quat::from_rotation_arc(
                        -Vec3::Z,
                        Vec3::new(travel.x, 0.0, travel.z).normalize_or(-Vec3::Z),
                    );
                    city.props.push((
                        PropKind::TrafficSign,
                        Pose::new(Vec3::new(pos.x, CURB_HEIGHT, pos.z), yaw),
                    ));
                }
                _ => {}
            }
        }
        // Crosswalks.
        for cw in &world.peds().crosswalks {
            city.crosswalks.push(CrosswalkGeom {
                a: to_world(cw.ax, cw.ay, lift),
                b: to_world(cw.bx, cw.by, lift),
                width: 3.0,
            });
        }
        // Street furniture along block edges.
        for (bi, blk) in city.blocks.clone().iter().enumerate() {
            if blk.outer {
                continue;
            }
            let inset = 0.55;
            let edges = [
                (
                    Vec2::new(blk.min.x, blk.max.y),
                    Vec2::new(blk.max.x, blk.max.y),
                    Vec3::Z,
                ),
                (
                    Vec2::new(blk.min.x, blk.min.y),
                    Vec2::new(blk.max.x, blk.min.y),
                    -Vec3::Z,
                ),
                (
                    Vec2::new(blk.min.x, blk.min.y),
                    Vec2::new(blk.min.x, blk.max.y),
                    -Vec3::X,
                ),
                (
                    Vec2::new(blk.max.x, blk.min.y),
                    Vec2::new(blk.max.x, blk.max.y),
                    Vec3::X,
                ),
            ];
            for (ei, (p0, p1, out)) in edges.iter().enumerate() {
                let len = (*p1 - *p0).length();
                let dir = (*p1 - *p0) / len.max(1e-3);
                let count = ((len - 16.0) / 32.0).floor().max(0.0) as usize + 1;
                let toward_road = Quat::from_rotation_arc(-Vec3::Z, *out);
                for k in 0..count {
                    let t = if count == 1 {
                        len * 0.5
                    } else {
                        8.0 + (len - 16.0) * k as f32 / (count - 1) as f32
                    };
                    let p = *p0 + dir * t;
                    let pos = Vec3::new(p.x, CURB_HEIGHT, p.y) + *out * -inset;
                    city.props
                        .push((PropKind::StreetLamp, Pose::new(pos, toward_road)));
                    city.lamp_lights
                        .push(toward_road * Vec3::new(0.0, 7.15, -1.6) + pos);
                    // Something between lamps now and then.
                    let h = hash(bi as u32 * 8 + ei as u32, k as u32);
                    if k + 1 < count {
                        let mid = p + dir * 16.0;
                        let back = *out * -(inset + 0.6);
                        let r = frac(h);
                        let kind = if r < 0.18 {
                            Some(PropKind::TrashCan)
                        } else if r < 0.3 {
                            Some(PropKind::Bench)
                        } else if r < 0.36 {
                            Some(PropKind::Mailbox)
                        } else if r < 0.44 {
                            Some(PropKind::Newsstand)
                        } else if r < 0.5 {
                            Some(PropKind::Planter)
                        } else {
                            None
                        };
                        if let Some(kind) = kind {
                            let rot = if kind == PropKind::Bench {
                                Quat::from_rotation_arc(Vec3::Z, *out)
                            } else {
                                Quat::IDENTITY
                            };
                            city.props.push((
                                kind,
                                Pose::new(Vec3::new(mid.x, CURB_HEIGHT, mid.y) + back, rot),
                            ));
                        }
                    }
                }
            }
            // One hydrant per block, near a corner.
            let c = Vec2::new(blk.min.x + 4.0, blk.max.y);
            city.props.push((
                PropKind::FireHydrant,
                Pose::from_translation(Vec3::new(c.x, CURB_HEIGHT, c.y - 0.7)),
            ));
        }
        city
    }

    /// Create the static colliders (road slab, raised blocks, buildings).
    pub fn add_to_physics(&self, physics: &mut PhysicsWorld) {
        let center = (self.min + self.max) * 0.5;
        let half = (self.max - self.min) * 0.5;
        let ground = physics.add_body(BodyDesc::fixed(Pose::from_translation(Vec3::new(
            center.x, -0.5, center.y,
        ))));
        physics.add_collider(
            ground,
            ColliderDesc::new(Shape::cuboid(Vec3::new(half.x, 0.5, half.y)))
                .friction(1.0)
                .surface(Surface::Asphalt),
        );
        for b in &self.blocks {
            let c = (b.min + b.max) * 0.5;
            let body = physics.add_body(BodyDesc::fixed(Pose::from_translation(Vec3::new(
                c.x, 0.0, c.y,
            ))));
            let outline = b.outline(if b.outer { 0.0 } else { CORNER_RADIUS });
            let mut pts = Vec::with_capacity(outline.len() * 2);
            for p in &outline {
                pts.push(Vec3::new(p.x - c.x, 0.0, p.y - c.y));
                pts.push(Vec3::new(p.x - c.x, CURB_HEIGHT, p.y - c.y));
            }
            let h = (b.max - b.min) * 0.5;
            let shape = Shape::convex(&pts)
                .unwrap_or_else(|| Shape::cuboid(Vec3::new(h.x, CURB_HEIGHT * 0.5, h.y)));
            physics.add_collider(
                body,
                ColliderDesc::new(shape).friction(0.9).surface(if b.outer {
                    Surface::Grass
                } else {
                    Surface::Concrete
                }),
            );
        }
        for b in &self.buildings {
            if b.size.y <= 0.5 {
                continue;
            }
            let body = physics.add_body(BodyDesc::fixed(Pose::from_translation(
                b.base + Vec3::Y * (b.size.y * 0.5),
            )));
            physics.add_collider(
                body,
                ColliderDesc::new(Shape::cuboid(b.size * 0.5))
                    .friction(0.6)
                    .surface(Surface::Concrete),
            );
        }
    }
}
