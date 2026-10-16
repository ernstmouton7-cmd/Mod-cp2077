//! Destruction: Voronoi fracture of convex shapes and breakable bodies.
//!
//! A `Destructible` wraps a rigid body whose collider is convex. When an impact exceeds its
//! strength, the body is replaced by convex Voronoi cells clustered around the impact point,
//! each inheriting the original velocity plus an outward kick. Cells can be fractured again.

use crate::body::{layers, BodyDesc, BodyHandle, ColliderDesc, Surface};
use crate::math::Pose;
use crate::shape::{ConvexHull, Shape};
use crate::world::{PhysicsEvent, PhysicsWorld};
use glam::Vec3;

/// Small deterministic RNG (xorshift) so fracture patterns are reproducible.
#[derive(Clone, Copy, Debug)]
pub struct FractureRng(u32);

impl FractureRng {
    pub fn new(seed: u32) -> Self {
        Self(seed.max(1))
    }
    pub fn next_f32(&mut self) -> f32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        (x >> 8) as f32 / 16_777_216.0
    }
    pub fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.next_f32()
    }
}

/// Clip a convex point set (given as hull vertices) by the half-space dot(n, x) <= d.
/// Works on the vertex/face description; returns the new vertex set.
fn clip_hull(h: &ConvexHull, n: Vec3, d: f32) -> Vec<Vec3> {
    let mut out: Vec<Vec3> = Vec::new();
    let side = |p: Vec3| n.dot(p) - d;
    for v in &h.verts {
        if side(*v) <= 0.0 {
            out.push(*v);
        }
    }
    for e in &h.edges {
        let a = h.verts[e.v0 as usize];
        let b = h.verts[e.v1 as usize];
        let (sa, sb) = (side(a), side(b));
        if (sa < 0.0 && sb > 0.0) || (sa > 0.0 && sb < 0.0) {
            out.push(a + (b - a) * (sa / (sa - sb)));
        }
    }
    out
}

/// Split a hull into Voronoi cells around `sites` (hull-local). Cells smaller than `min_volume` are dropped.
pub fn voronoi_cells(hull: &ConvexHull, sites: &[Vec3], min_volume: f32) -> Vec<ConvexHull> {
    let mut cells = Vec::new();
    for (i, si) in sites.iter().enumerate() {
        let mut cell = hull.clone();
        let mut ok = true;
        for (j, sj) in sites.iter().enumerate() {
            if i == j {
                continue;
            }
            let n = *sj - *si;
            let len = n.length();
            if len < 1e-5 {
                continue;
            }
            let n = n / len;
            let d = n.dot((*si + *sj) * 0.5);
            // Skip planes that cannot cut this cell.
            if cell.verts.iter().all(|v| n.dot(*v) <= d) {
                continue;
            }
            let pts = clip_hull(&cell, n, d);
            match ConvexHull::from_points(&pts) {
                Some(c) => cell = c,
                None => {
                    ok = false;
                    break;
                }
            }
        }
        if ok && cell.volume() >= min_volume {
            cells.push(cell);
        }
    }
    cells
}

/// Sites clustered near an impact point (dense near it, sparse far away) inside a hull.
pub fn impact_sites(
    hull: &ConvexHull,
    impact: Vec3,
    count: usize,
    rng: &mut FractureRng,
) -> Vec<Vec3> {
    let lo = hull.aabb.min;
    let hi = hull.aabb.max;
    let size = (hi - lo).length();
    let mut sites = Vec::with_capacity(count);
    let mut tries = 0;
    while sites.len() < count && tries < count * 40 {
        tries += 1;
        let p = if sites.len() % 3 == 0 {
            Vec3::new(
                rng.range(lo.x, hi.x),
                rng.range(lo.y, hi.y),
                rng.range(lo.z, hi.z),
            )
        } else {
            // Gaussian-ish cluster around the impact.
            let r = size * 0.35 * rng.next_f32().powi(2);
            let dir = Vec3::new(
                rng.range(-1.0, 1.0),
                rng.range(-1.0, 1.0),
                rng.range(-1.0, 1.0),
            )
            .normalize_or(Vec3::Y);
            impact + dir * r
        };
        if hull.faces.iter().all(|f| f.normal.dot(p) - f.d < -1e-3) {
            sites.push(p);
        }
    }
    sites
}

#[derive(Clone, Debug)]
pub struct Destructible {
    pub body: BodyHandle,
    pub hull: ConvexHull,
    /// Impulse (N·s) that breaks it.
    pub strength: f32,
    pub pieces: usize,
    /// Pieces may break again if larger than this volume (m³).
    pub refracture_volume: f32,
    pub density: f32,
    pub surface: Surface,
    /// Outward speed given to fragments (m/s).
    pub burst: f32,
    pub seed: u32,
    /// Lifetime of fragments before the game may despawn them (s); infinite = keep.
    pub debris_lifetime: f32,
}

/// A fragment produced by a fracture.
#[derive(Clone, Debug)]
pub struct Fragment {
    pub body: BodyHandle,
    /// Fragment shape in the fragment body's frame (draw it at the body pose).
    pub hull: ConvexHull,
    /// The destructible this came from (already removed from the world).
    pub source: BodyHandle,
    pub spawned_at: f64,
}

#[derive(Default)]
pub struct DestructionSystem {
    pub destructibles: Vec<Destructible>,
    pub fragments: Vec<Fragment>,
    /// Fragments older than this are removed (0 = never).
    pub debris_lifetime: f32,
    /// Hard cap on live fragments (oldest removed first).
    pub max_fragments: usize,
}

impl DestructionSystem {
    pub fn new() -> Self {
        Self {
            destructibles: Vec::new(),
            fragments: Vec::new(),
            debris_lifetime: 30.0,
            max_fragments: 600,
        }
    }

    /// Create a breakable convex object (a static wall if `fixed`).
    #[allow(clippy::too_many_arguments)]
    pub fn spawn(
        &mut self,
        world: &mut PhysicsWorld,
        hull: ConvexHull,
        pose: Pose,
        fixed: bool,
        density: f32,
        strength: f32,
        pieces: usize,
        surface: Surface,
    ) -> BodyHandle {
        let desc = if fixed {
            BodyDesc::fixed(pose)
        } else {
            BodyDesc::dynamic(pose)
        };
        let b = world.add_body(desc);
        world.add_collider(
            b,
            ColliderDesc::new(Shape::Convex(std::sync::Arc::new(hull.clone())))
                .density(density)
                .surface(surface)
                .filter(layers::PROP, layers::ALL),
        );
        self.destructibles.push(Destructible {
            body: b,
            hull,
            strength,
            pieces,
            refracture_volume: 0.05,
            density,
            surface,
            burst: 1.5,
            seed: b.0.index.wrapping_mul(2654435761) ^ 0x9e3779b9,
            debris_lifetime: 30.0,
        });
        b
    }

    /// Break destructibles hit hard enough during the last step. Returns new fragment bodies.
    pub fn update(&mut self, world: &mut PhysicsWorld) -> Vec<BodyHandle> {
        // (destructible index, point, impulse, impactor body, direction from impactor into the object)
        let mut hits: Vec<(usize, Vec3, f32, BodyHandle, Vec3)> = Vec::new();
        for e in world.events() {
            if let PhysicsEvent::Impact {
                body_a,
                body_b,
                point,
                normal,
                impulse,
                ..
            } = *e
            {
                for (i, d) in self.destructibles.iter().enumerate() {
                    let (other, dir) = if d.body == body_a {
                        (body_b, -normal)
                    } else if d.body == body_b {
                        (body_a, normal)
                    } else {
                        continue;
                    };
                    if impulse > d.strength && !hits.iter().any(|h| h.0 == i) {
                        hits.push((i, point, impulse, other, dir));
                    }
                }
            }
        }
        hits.sort_by(|a, b| b.0.cmp(&a.0));
        let mut spawned = Vec::new();
        for (i, point, impulse, other, dir) in hits {
            let d = self.destructibles.swap_remove(i);
            // The solver stopped the impactor as if the object were unbreakable; breaking only
            // absorbed `strength`, so hand back the rest of the momentum.
            let give_back = impulse - d.strength.min(impulse);
            if give_back > 0.0 && world.body(other).is_some_and(|b| b.is_dynamic()) {
                world.apply_impulse(other, dir * give_back * 0.85, point);
            }
            spawned.extend(self.shatter(world, &d, point, impulse));
        }
        // Debris cleanup.
        let now = world.time;
        if self.debris_lifetime > 0.0 {
            let life = self.debris_lifetime as f64;
            let (old, keep): (Vec<Fragment>, Vec<Fragment>) =
                self.fragments.drain(..).partition(|f| {
                    now - f.spawned_at > life && world.body(f.body).is_some_and(|b| b.sleeping)
                });
            for f in old {
                world.remove_body(f.body);
            }
            self.fragments = keep;
        }
        while self.fragments.len() > self.max_fragments {
            let f = self.fragments.remove(0);
            world.remove_body(f.body);
        }
        spawned
    }

    /// Fracture one destructible now (scripted explosions etc.).
    pub fn shatter(
        &mut self,
        world: &mut PhysicsWorld,
        d: &Destructible,
        point: Vec3,
        impulse: f32,
    ) -> Vec<BodyHandle> {
        let Some(body) = world.body(d.body) else {
            return Vec::new();
        };
        let pose = body.pose;
        let (lin, ang, com) = (body.lin_vel, body.ang_vel, body.com());
        let local_hit = pose.inverse_transform_point(point);
        let mut rng = FractureRng::new(d.seed);
        let n = d.pieces.clamp(2, 64);
        let sites = impact_sites(&d.hull, local_hit, n, &mut rng);
        let cells = voronoi_cells(&d.hull, &sites, 1e-5);
        world.remove_body(d.body);
        let mut out = Vec::new();
        let energy = (impulse / (d.hull.volume() * d.density).max(1.0)).min(8.0);
        for cell in cells {
            let c = cell.centroid;
            let local: Vec<Vec3> = cell.verts.iter().map(|v| *v - c).collect();
            let Some(piece) = ConvexHull::from_points(&local) else {
                continue;
            };
            let wp = pose.transform_point(c);
            let outward = (c - local_hit).normalize_or(Vec3::Y);
            let kick =
                pose.transform_vector(outward) * (d.burst + energy * 0.5) * rng.range(0.5, 1.0);
            let v = lin + ang.cross(wp - com) + kick;
            let b = world.add_body(BodyDesc {
                lin_vel: v,
                ang_vel: ang
                    + Vec3::new(
                        rng.range(-3.0, 3.0),
                        rng.range(-3.0, 3.0),
                        rng.range(-3.0, 3.0),
                    ),
                ..BodyDesc::dynamic(Pose::new(wp, pose.q))
            });
            world.add_collider(
                b,
                ColliderDesc::new(Shape::Convex(std::sync::Arc::new(piece.clone())))
                    .density(d.density)
                    .surface(d.surface)
                    .friction(0.6)
                    .filter(layers::DEBRIS, layers::ALL),
            );
            if piece.volume() > d.refracture_volume && d.pieces > 4 {
                self.destructibles.push(Destructible {
                    body: b,
                    hull: piece.clone(),
                    strength: d.strength * 0.6,
                    pieces: (d.pieces / 2).max(3),
                    seed: d.seed.wrapping_add(out.len() as u32 * 7919),
                    ..d.clone()
                });
            }
            self.fragments.push(Fragment {
                body: b,
                hull: piece,
                source: d.body,
                spawned_at: world.time,
            });
            out.push(b);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voronoi_cells_partition_the_volume() {
        let h = ConvexHull::cuboid(Vec3::new(1.0, 0.5, 0.25));
        let mut rng = FractureRng::new(7);
        let sites = impact_sites(&h, Vec3::new(0.5, 0.0, 0.0), 12, &mut rng);
        assert!(sites.len() >= 10);
        let cells = voronoi_cells(&h, &sites, 0.0);
        let total: f32 = cells.iter().map(|c| c.volume()).sum();
        assert!(
            (total - h.volume()).abs() < 0.02 * h.volume(),
            "cells {total} vs {}",
            h.volume()
        );
        assert!(cells.len() >= sites.len() - 1);
    }
}
