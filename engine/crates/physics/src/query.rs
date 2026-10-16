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
        Self {
            mask: u32::MAX,
            exclude_body: None,
            include_sensors: false,
            static_only: false,
        }
    }
}

impl QueryFilter {
    pub fn excluding(body: BodyHandle) -> Self {
        Self {
            exclude_body: Some(body),
            ..Default::default()
        }
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
        self.colliders
            .get_index(proxy_data)
            .map(|(h, c)| (ColliderHandle(h), c))
    }

    /// Closest hit along a ray. `dir` need not be normalised.
    pub fn ray_cast(
        &self,
        origin: Vec3,
        dir: Vec3,
        max_t: f32,
        filter: &QueryFilter,
    ) -> Option<RayHit> {
        let len = dir.length();
        if len < 1e-12 {
            return None;
        }
        let d = dir / len;
        let mut best: Option<RayHit> = None;
        for (tree, is_static) in [(self.static_tree, true), (self.dynamic_tree, false)] {
            let limit = best.map_or(max_t, |b| b.t);
            tree.ray_cast(origin, d, limit, |proxy, t_max| {
                let Some((h, c)) = self.collider_at(tree.data(proxy)) else {
                    return t_max;
                };
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
    pub fn sphere_cast(
        &self,
        center: Vec3,
        radius: f32,
        dir: Vec3,
        max_t: f32,
        filter: &QueryFilter,
    ) -> Option<RayHit> {
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
                let Some((h, c)) = self.collider_at(tree.data(proxy)) else {
                    return true;
                };
                if !self.accept(c, filter, is_static) {
                    return true;
                }
                let limit = best.map_or(max_t, |b| b.t);
                let mut consider = |hit: Option<crate::gjk::CastHit>, key: u32| {
                    if let Some(hit) = hit {
                        if hit.t <= best.map_or(max_t, |b| b.t) {
                            best = Some(RayHit {
                                collider: h,
                                body: c.body,
                                point: hit.point,
                                normal: hit.normal,
                                t: hit.t,
                                surface: c.surface,
                                sub_key: key,
                            });
                        }
                    }
                };
                match &c.shape {
                    Shape::Sphere { radius: rb } => {
                        let p = [c.world.p];
                        consider(shape_cast(&a, radius, &PointSet(&p), *rb, d, limit), 0);
                    }
                    Shape::Capsule {
                        half_height,
                        radius: rb,
                    } => {
                        let s = [
                            c.world.transform_point(Vec3::new(0.0, -half_height, 0.0)),
                            c.world.transform_point(Vec3::new(0.0, *half_height, 0.0)),
                        ];
                        consider(shape_cast(&a, radius, &PointSet(&s), *rb, d, limit), 0);
                    }
                    Shape::Cuboid { hull, .. } | Shape::Convex(hull) => {
                        let lo = c.world.inverse_transform_point(center);
                        let ld = c.world.inverse_transform_vector(d);
                        if let Some((t, n)) = sweep_sphere_hull(hull, lo, ld, radius, limit) {
                            let n = c.world.transform_vector(n);
                            let p = center + d * t - n * radius;
                            consider(
                                Some(crate::gjk::CastHit {
                                    t,
                                    point: p,
                                    normal: n,
                                }),
                                0,
                            );
                        }
                    }
                    Shape::TriMesh(m) => {
                        let local = sweep.transformed(&c.world.inverse());
                        m.bvh.query_aabb(&local, |ti| {
                            let tri = m.triangle(ti);
                            let lo = c.world.inverse_transform_point(center);
                            let ld = c.world.inverse_transform_vector(d);
                            let lim = best.map_or(max_t, |b| b.t);
                            if let Some((t, n)) =
                                sweep_sphere_triangle(tri, m.normals[ti], lo, ld, radius, lim)
                            {
                                if t <= best.map_or(max_t, |b| b.t) {
                                    let n = c.world.transform_vector(n);
                                    best = Some(RayHit {
                                        collider: h,
                                        body: c.body,
                                        point: center + d * t - n * radius,
                                        normal: n,
                                        t,
                                        surface: c.surface,
                                        sub_key: ti as u32,
                                    });
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
    pub fn overlap_aabb(
        &self,
        aabb: &Aabb,
        filter: &QueryFilter,
        mut f: impl FnMut(ColliderHandle, &Collider),
    ) {
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
    pub fn overlap_sphere(
        &self,
        center: Vec3,
        radius: f32,
        filter: &QueryFilter,
    ) -> Vec<ColliderHandle> {
        let mut out = Vec::new();
        let bb = Aabb::from_center_half(center, Vec3::splat(radius));
        let probe = Shape::sphere(radius);
        let pose = crate::math::Pose::from_translation(center);
        let mut scratch = Vec::new();
        self.overlap_aabb(&bb, filter, |h, c| {
            scratch.clear();
            crate::narrowphase::collide(&probe, &pose, &c.shape, &c.world, 0.0, &mut scratch);
            if scratch
                .iter()
                .any(|m| m.points.iter().any(|p| p.separation <= 0.0))
            {
                out.push(h);
            }
        });
        out
    }
}

/// Ray vs an infinite-length-clipped capsule body (segment a-b, radius r). Returns (t, normal).
fn ray_edge(o: Vec3, d: Vec3, a: Vec3, b: Vec3, r: f32, max_t: f32) -> Option<(f32, Vec3)> {
    let ab = b - a;
    let len = ab.length();
    if len < 1e-9 {
        return None;
    }
    let u = ab / len;
    let oa = o - a;
    let op = oa - u * oa.dot(u);
    let dp = d - u * d.dot(u);
    let qa = dp.dot(dp);
    if qa < 1e-12 {
        return None;
    }
    let qb = op.dot(dp);
    let qc = op.dot(op) - r * r;
    let disc = qb * qb - qa * qc;
    if disc < 0.0 {
        return None;
    }
    let t = (-qb - disc.sqrt()) / qa;
    if t < 0.0 || t > max_t {
        return None;
    }
    let s = (oa + d * t).dot(u);
    if s < 0.0 || s > len {
        return None;
    }
    let n = (op + dp * t).normalize_or_zero();
    Some((t, n))
}

/// Sphere swept along `d` (unit) against a convex polytope given by planes, loops and edges.
/// Exact rounded Minkowski sum: inflated faces, edge cylinders and vertex spheres.
fn sweep_sphere_poly(
    o: Vec3,
    d: Vec3,
    r: f32,
    max_t: f32,
    verts: &[Vec3],
    planes: &[(Vec3, f32)],
    face_loop: &dyn Fn(usize) -> smallvec::SmallVec<[u32; 16]>,
    edges: &[(u32, u32)],
) -> Option<(f32, Vec3)> {
    // Start overlapping (inside the inflated planes): report t = 0.
    let mut max_sep = f32::MIN;
    let mut sep_n = Vec3::Y;
    for (n, pd) in planes {
        let s = n.dot(o) - pd;
        if s > max_sep {
            max_sep = s;
            sep_n = *n;
        }
    }
    if max_sep < r * 0.999 && max_sep <= 0.0 {
        return Some((0.0, sep_n));
    }
    let mut best: Option<(f32, Vec3)> = None;
    let mut limit = max_t;
    for (fi, (n, pd)) in planes.iter().enumerate() {
        let dist = n.dot(o) - pd - r;
        let den = n.dot(d);
        if den >= -1e-9 || dist < -r {
            continue;
        }
        let t = (-dist / den).max(0.0);
        if t > limit {
            continue;
        }
        // Inside the face prism?
        let q = o + d * t - *n * r;
        let lp = face_loop(fi);
        let mut inside = true;
        for k in 0..lp.len() {
            let v0 = verts[lp[k] as usize];
            let v1 = verts[lp[(k + 1) % lp.len()] as usize];
            let side = (v1 - v0).cross(*n);
            if side.dot(q - v0) > 1e-6 * side.length() {
                inside = false;
                break;
            }
        }
        if inside {
            limit = t;
            best = Some((t, *n));
        }
    }
    for &(a, b) in edges {
        if let Some((t, n)) = ray_edge(o, d, verts[a as usize], verts[b as usize], r, limit) {
            limit = t;
            best = Some((t, n));
        }
    }
    for v in verts {
        if let Some((t, n)) = crate::shape::ray_sphere(o, d, *v, r, limit, false) {
            if t <= limit {
                limit = t;
                best = Some((t, n));
            }
        }
    }
    best
}

pub(crate) fn sweep_sphere_hull(
    h: &crate::shape::ConvexHull,
    o: Vec3,
    d: Vec3,
    r: f32,
    max_t: f32,
) -> Option<(f32, Vec3)> {
    let planes: smallvec::SmallVec<[(Vec3, f32); 32]> =
        h.faces.iter().map(|f| (f.normal, f.d)).collect();
    let edges: smallvec::SmallVec<[(u32, u32); 48]> =
        h.edges.iter().map(|e| (e.v0, e.v1)).collect();
    let fl = |f: usize| -> smallvec::SmallVec<[u32; 16]> {
        let face = h.faces[f];
        h.face_verts[face.first as usize..(face.first + face.count) as usize]
            .iter()
            .copied()
            .collect()
    };
    sweep_sphere_poly(o, d, r, max_t, &h.verts, &planes, &fl, &edges)
}

/// One-sided triangle (front face only; edges and vertices round).
pub(crate) fn sweep_sphere_triangle(
    tri: [Vec3; 3],
    n: Vec3,
    o: Vec3,
    d: Vec3,
    r: f32,
    max_t: f32,
) -> Option<(f32, Vec3)> {
    if n.dot(o - tri[0]) < 0.0 {
        return None;
    }
    let planes = [(n, n.dot(tri[0]))];
    let fl = |_f: usize| -> smallvec::SmallVec<[u32; 16]> { smallvec::smallvec![0, 1, 2] };
    let edges = [(0u32, 1u32), (1, 2), (2, 0)];
    sweep_sphere_poly_no_overlap(o, d, r, max_t, &tri, &planes, &fl, &edges)
}

#[allow(clippy::too_many_arguments)]
fn sweep_sphere_poly_no_overlap(
    o: Vec3,
    d: Vec3,
    r: f32,
    max_t: f32,
    verts: &[Vec3],
    planes: &[(Vec3, f32)],
    face_loop: &dyn Fn(usize) -> smallvec::SmallVec<[u32; 16]>,
    edges: &[(u32, u32)],
) -> Option<(f32, Vec3)> {
    // Already within r of the face region: t = 0.
    let (n, pd) = planes[0];
    let h = n.dot(o) - pd;
    if h < r {
        let q = o - n * h;
        let lp = face_loop(0);
        let inside = (0..lp.len()).all(|k| {
            let v0 = verts[lp[k] as usize];
            let v1 = verts[lp[(k + 1) % lp.len()] as usize];
            (v1 - v0).cross(n).dot(q - v0) <= 0.0
        });
        if inside {
            return Some((0.0, n));
        }
    }
    let mut best: Option<(f32, Vec3)> = None;
    let mut limit = max_t;
    let den = n.dot(d);
    if den < -1e-9 {
        let t = ((h - r) / -den).max(0.0);
        if t <= limit {
            let q = o + d * t - n * r;
            let lp = face_loop(0);
            let inside = (0..lp.len()).all(|k| {
                let v0 = verts[lp[k] as usize];
                let v1 = verts[lp[(k + 1) % lp.len()] as usize];
                (v1 - v0).cross(n).dot(q - v0) <= 0.0
            });
            if inside {
                limit = t;
                best = Some((t, n));
            }
        }
    }
    for &(a, b) in edges {
        if let Some((t, nn)) = ray_edge(o, d, verts[a as usize], verts[b as usize], r, limit) {
            limit = t;
            best = Some((t, nn));
        }
    }
    for v in verts {
        if let Some((t, nn)) = crate::shape::ray_sphere(o, d, *v, r, limit, false) {
            if t <= limit {
                limit = t;
                best = Some((t, nn));
            }
        }
    }
    best
}
