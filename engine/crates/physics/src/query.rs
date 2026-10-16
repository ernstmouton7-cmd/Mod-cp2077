//! Scene queries: ray casts, sphere casts and overlap tests against the broadphase.

use crate::aabb::Aabb;
use crate::arena::Arena;
use crate::body::{BodyHandle, Collider, ColliderHandle, RigidBody, Surface};
use crate::broadphase::DynamicTree;
use crate::gjk::{shape_cast, PointSet};
use crate::shape::Shape;
use glam::Vec3;

/// Which colliders a query may hit.
#[derive(Clone, Copy, Debug)]
pub struct QueryFilter {
    /// Collider `memberships` must intersect this mask.
    pub mask: u32,
    pub exclude_body: Option<BodyHandle>,
    pub include_sensors: bool,
    /// Skip dynamic/kinematic bodies (static world only).
    pub static_only: bool,
}

impl Default for QueryFilter {
    fn default() -> Self {
        Self { mask: u32::MAX, exclude_body: None, include_sensors: false, static_only: false }
    }
}

impl QueryFilter {
    pub fn excluding(body: BodyHandle) -> Self {
        Self { exclude_body: Some(body), ..Default::default() }
    }
    pub fn mask(mut self, m: u32) -> Self {
        self.mask = m;
        self
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RayHit {
    pub collider: ColliderHandle,
    pub body: BodyHandle,
    pub point: Vec3,
    pub normal: Vec3,
    /// Distance along the (unit) direction.
    pub t: f32,
    pub surface: Surface,
    /// Triangle index for meshes, 0 otherwise.
    pub sub_key: u32,
}

/// A read-only view of the scene used by queries (also usable inside the step).
#[derive(Clone, Copy)]
pub struct QueryView<'a> {
    pub colliders: &'a Arena<Collider>,
    pub bodies: &'a Arena<RigidBody>,
    pub static_tree: &'a DynamicTree,
    pub dynamic_tree: &'a DynamicTree,
}

impl<'a> QueryView<'a> {
    #[inline]
    fn accept(&self, c: &Collider, f: &QueryFilter, is_static: bool) -> bool {
        if c.sensor && !f.include_sensors {
            return false;
        }
        if c.filter.memberships & f.mask == 0 {
            return false;
        }
        if f.static_only && !is_static {
            return false;
        }
        if let Some(ex) = f.exclude_body {
            if ex == c.body {
                return false;
            }
        }
        true
    }

    fn collider_at(&self, proxy_data: u32) -> Option<(ColliderHandle, &'a Collider)> {
        self.colliders.get_index(proxy_data).map(|(h, c)| (ColliderHandle(h), c))
    }

    /// Closest hit along a ray. `dir` need not be normalised.
    pub fn ray_cast(&self, origin: Vec3, dir: Vec3, max_t: f32, filter: &QueryFilter) -> Option<RayHit> {
        let len = dir.length();
        if len < 1e-12 {
            return None;
        }
        let d = dir / len;
        let mut best: Option<RayHit> = None;
        for (tree, is_static) in [(self.static_tree, true), (self.dynamic_tree, false)] {
            let limit = best.map_or(max_t, |b| b.t);
            tree.ray_cast(origin, d, limit, |proxy, t_max| {
                let Some((h, c)) = self.collider_at(tree.data(proxy)) else { return t_max };
                if !self.accept(c, filter, is_static) {
                    return t_max;
                }
                let lo = c.world.inverse_transform_point(origin);
                let ld = c.world.inverse_transform_vector(d);
                let (hit, key) = match &c.shape {
                    Shape::TriMesh(m) => match m.ray_cast(lo, ld, t_max) {
                        Some((t, n, tri)) => (Some((t, n)), tri as u32),
                        None => (None, 0),
                    },
                    s => (s.ray_cast_local(lo, ld, t_max, true), 0),
                };
                if let Some((t, n)) = hit {
                    if t <= t_max {
                        best = Some(RayHit {
                            collider: h,
                            body: c.body,
                            point: origin + d * t,
                            normal: c.world.transform_vector(n),
                            t,
                            surface: c.surface,
                            sub_key: key,
                        });
                        return t;
                    }
                }
                t_max
            });
        }
        best
    }

    /// Sweep a sphere; returns the first hit (point on the surface, normal pointing back at the sphere).
    pub fn sphere_cast(&self, center: Vec3, radius: f32, dir: Vec3, max_t: f32, filter: &QueryFilter) -> Option<RayHit> {
        let len = dir.length();
        if len < 1e-12 {
            return None;
        }
        let d = dir / len;
        let sweep = Aabb::from_center_half(center, Vec3::splat(radius)).swept(d * max_t);
        let mut best: Option<RayHit> = None;
        let core = [center];
        let a = PointSet(&core);
        for (tree, is_static) in [(self.static_tree, true), (self.dynamic_tree, false)] {
            tree.query(&sweep, |proxy| {
                let Some((h, c)) = self.collider_at(tree.data(proxy)) else { return true };
                if !self.accept(c, filter, is_static) {
                    return true;
                }
                let limit = best.map_or(max_t, |b| b.t);
                let mut consider = |hit: Option<crate::gjk::CastHit>, key: u32| {
                    if let Some(hit) = hit {
                        if hit.t <= best.map_or(max_t, |b| b.t) {
                            best = Some(RayHit { collider: h, body: c.body, point: hit.point, normal: hit.normal, t: hit.t, surface: c.surface, sub_key: key });
                        }
                    }
                };
                match &c.shape {
                    Shape::Sphere { radius: rb } => {
                        let p = [c.world.p];
                        consider(shape_cast(&a, radius, &PointSet(&p), *rb, d, limit), 0);
                    }
                    Shape::Capsule { half_height, radius: rb } => {
                        let s = [c.world.transform_point(Vec3::new(0.0, -half_height, 0.0)), c.world.transform_point(Vec3::new(0.0, *half_height, 0.0))];
                        consider(shape_cast(&a, radius, &PointSet(&s), *rb, d, limit), 0);
                    }
                    Shape::Cuboid { hull, .. } | Shape::Convex(hull) => {
                        let v: smallvec::SmallVec<[Vec3; 32]> = hull.verts.iter().map(|p| c.world.transform_point(*p)).collect();
                        consider(shape_cast(&a, radius, &PointSet(&v), 0.0, d, limit), 0);
                    }
                    Shape::TriMesh(m) => {
                        let local = sweep.transformed(&c.world.inverse());
                        m.bvh.query_aabb(&local, |ti| {
                            let [p0, p1, p2] = m.triangle(ti);
                            let tri = [c.world.transform_point(p0), c.world.transform_point(p1), c.world.transform_point(p2)];
                            let lim = best.map_or(max_t, |b| b.t);
                            if let Some(hit) = shape_cast(&a, radius, &PointSet(&tri), 0.0, d, lim) {
                                if hit.t <= best.map_or(max_t, |b| b.t) {
                                    best = Some(RayHit { collider: h, body: c.body, point: hit.point, normal: hit.normal, t: hit.t, surface: c.surface, sub_key: ti as u32 });
                                }
                            }
                        });
                    }
                }
                true
            });
        }
        best
    }

    /// Colliders whose bounds overlap a box.
    pub fn overlap_aabb(&self, aabb: &Aabb, filter: &QueryFilter, mut f: impl FnMut(ColliderHandle, &Collider)) {
        for (tree, is_static) in [(self.static_tree, true), (self.dynamic_tree, false)] {
            tree.query(aabb, |proxy| {
                if let Some((h, c)) = self.collider_at(tree.data(proxy)) {
                    if self.accept(c, filter, is_static) && c.aabb.overlaps(aabb) {
                        f(h, c);
                    }
                }
                true
            });
        }
    }

    /// Colliders touching a sphere (exact test against each shape).
    pub fn overlap_sphere(&self, center: Vec3, radius: f32, filter: &QueryFilter) -> Vec<ColliderHandle> {
        let mut out = Vec::new();
        let bb = Aabb::from_center_half(center, Vec3::splat(radius));
        let probe = Shape::sphere(radius);
        let pose = crate::math::Pose::from_translation(center);
        let mut scratch = Vec::new();
        self.overlap_aabb(&bb, filter, |h, c| {
            scratch.clear();
            crate::narrowphase::collide(&probe, &pose, &c.shape, &c.world, 0.0, &mut scratch);
            if scratch.iter().any(|m| m.points.iter().any(|p| p.separation <= 0.0)) {
                out.push(h);
            }
        });
        out
    }
}
