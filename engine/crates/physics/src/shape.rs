//! Collision shapes, convex hulls, mass properties and shape-local ray casts.

use crate::aabb::Aabb;
use crate::bvh::Bvh;
use crate::math::Pose;
use glam::{Mat3, Vec3};
use std::sync::Arc;

/// Collision geometry. Capsules are aligned with the local Y axis.
#[derive(Clone, Debug)]
pub enum Shape {
    Sphere { radius: f32 },
    Capsule { half_height: f32, radius: f32 },
    /// A box; `hull` is the same box as a polytope for the SAT routines.
    Cuboid { half: Vec3, hull: Arc<ConvexHull> },
    Convex(Arc<ConvexHull>),
    /// Static triangle soup (roads, terrain, imported level geometry).
    TriMesh(Arc<TriMesh>),
}

impl Shape {
    pub fn sphere(radius: f32) -> Self {
        Shape::Sphere { radius }
    }
    pub fn capsule(half_height: f32, radius: f32) -> Self {
        Shape::Capsule { half_height, radius }
    }
    pub fn cuboid(half: Vec3) -> Self {
        Shape::Cuboid { half, hull: Arc::new(ConvexHull::cuboid(half)) }
    }
    /// Convex hull of a point cloud; `None` if the points are degenerate (flat or too few).
    pub fn convex(points: &[Vec3]) -> Option<Self> {
        ConvexHull::from_points(points).map(|h| Shape::Convex(Arc::new(h)))
    }
    pub fn trimesh(vertices: Vec<Vec3>, triangles: Vec<[u32; 3]>) -> Self {
        Shape::TriMesh(Arc::new(TriMesh::new(vertices, triangles)))
    }

    /// The hull for polytope shapes.
    pub fn hull(&self) -> Option<&ConvexHull> {
        match self {
            Shape::Cuboid { hull, .. } => Some(hull),
            Shape::Convex(h) => Some(h),
            _ => None,
        }
    }

    pub fn local_aabb(&self) -> Aabb {
        match self {
            Shape::Sphere { radius } => Aabb::from_center_half(Vec3::ZERO, Vec3::splat(*radius)),
            Shape::Capsule { half_height, radius } => {
                Aabb::from_center_half(Vec3::ZERO, Vec3::new(*radius, half_height + radius, *radius))
            }
            Shape::Cuboid { half, .. } => Aabb::from_center_half(Vec3::ZERO, *half),
            Shape::Convex(h) => h.aabb,
            Shape::TriMesh(m) => m.bvh.root_aabb(),
        }
    }

    /// World bounds under a pose (tight for spheres/capsules/hulls).
    pub fn aabb(&self, pose: &Pose) -> Aabb {
        match self {
            Shape::Sphere { radius } => Aabb::from_center_half(pose.p, Vec3::splat(*radius)),
            Shape::Capsule { half_height, radius } => {
                let a = pose.transform_point(Vec3::new(0.0, *half_height, 0.0));
                let b = pose.transform_point(Vec3::new(0.0, -*half_height, 0.0));
                Aabb::new(a.min(b), a.max(b)).expanded(*radius)
            }
            Shape::Convex(h) => {
                if h.verts.len() <= 16 {
                    Aabb::from_points(h.verts.iter().map(|v| pose.transform_point(*v)))
                } else {
                    h.aabb.transformed(pose)
                }
            }
            _ => self.local_aabb().transformed(pose),
        }
    }

    /// Mass properties for a uniform density.
    pub fn mass_properties(&self, density: f32) -> MassProperties {
        match self {
            Shape::Sphere { radius } => {
                let m = density * 4.0 / 3.0 * std::f32::consts::PI * radius.powi(3);
                let i = 0.4 * m * radius * radius;
                MassProperties { mass: m, com: Vec3::ZERO, inertia: Mat3::from_diagonal(Vec3::splat(i)) }
            }
            Shape::Capsule { half_height, radius } => {
                let r = *radius;
                let h = 2.0 * half_height;
                let pi = std::f32::consts::PI;
                let mc = density * pi * r * r * h;
                let ms = density * 4.0 / 3.0 * pi * r.powi(3);
                // Cylinder plus two hemispheres displaced along Y.
                let iy = mc * r * r * 0.5 + ms * 0.4 * r * r;
                let ixz = mc * (3.0 * r * r + h * h) / 12.0
                    + ms * (0.4 * r * r + 0.5 * h * h * 0.5 + 0.375 * h * r);
                MassProperties { mass: mc + ms, com: Vec3::ZERO, inertia: Mat3::from_diagonal(Vec3::new(ixz, iy, ixz)) }
            }
            Shape::Cuboid { half, .. } => {
                let d = *half * 2.0;
                let m = density * d.x * d.y * d.z;
                let i = Vec3::new(d.y * d.y + d.z * d.z, d.x * d.x + d.z * d.z, d.x * d.x + d.y * d.y) * (m / 12.0);
                MassProperties { mass: m, com: Vec3::ZERO, inertia: Mat3::from_diagonal(i) }
            }
            Shape::Convex(h) => h.mass_properties(density),
            Shape::TriMesh(_) => MassProperties::default(),
        }
    }

    /// Radius of the sphere around the local origin that bounds the shape (for CCD margins).
    pub fn bounding_radius(&self) -> f32 {
        match self {
            Shape::Sphere { radius } => *radius,
            Shape::Capsule { half_height, radius } => half_height + radius,
            Shape::Cuboid { half, .. } => half.length(),
            Shape::Convex(h) => h.verts.iter().map(|v| v.length()).fold(0.0, f32::max),
            Shape::TriMesh(m) => m.bvh.root_aabb().half_extents().length() + m.bvh.root_aabb().center().length(),
        }
    }

    /// Ray cast in the shape's local frame. `dir` must be unit length. Returns (t, normal).
    pub fn ray_cast_local(&self, origin: Vec3, dir: Vec3, max_t: f32, solid: bool) -> Option<(f32, Vec3)> {
        match self {
            Shape::Sphere { radius } => ray_sphere(origin, dir, Vec3::ZERO, *radius, max_t, solid),
            Shape::Capsule { half_height, radius } => ray_capsule(origin, dir, *half_height, *radius, max_t, solid),
            Shape::Cuboid { half, .. } => ray_box(origin, dir, *half, max_t, solid),
            Shape::Convex(h) => h.ray_cast(origin, dir, max_t, solid),
            Shape::TriMesh(m) => m.ray_cast(origin, dir, max_t).map(|(t, n, _)| (t, n)),
        }
    }
}

/// Mass, centre of mass and inertia tensor about the centre of mass.
#[derive(Clone, Copy, Debug)]
pub struct MassProperties {
    pub mass: f32,
    pub com: Vec3,
    pub inertia: Mat3,
}

impl Default for MassProperties {
    fn default() -> Self {
        Self { mass: 0.0, com: Vec3::ZERO, inertia: Mat3::ZERO }
    }
}

impl MassProperties {
    /// Express in a parent frame (rotate and translate).
    pub fn transformed(&self, pose: &Pose) -> MassProperties {
        let r = Mat3::from_quat(pose.q);
        MassProperties { mass: self.mass, com: pose.transform_point(self.com), inertia: r * self.inertia * r.transpose() }
    }

    /// Sum of several bodies' mass properties (parallel axis theorem).
    pub fn combine(parts: &[MassProperties]) -> MassProperties {
        let mass: f32 = parts.iter().map(|p| p.mass).sum();
        if mass <= 0.0 {
            return MassProperties::default();
        }
        let com = parts.iter().map(|p| p.com * p.mass).sum::<Vec3>() / mass;
        let mut inertia = Mat3::ZERO;
        for p in parts {
            let d = p.com - com;
            let shift = Mat3::from_diagonal(Vec3::splat(d.dot(d))) - outer(d, d);
            inertia += p.inertia + shift * p.mass;
        }
        MassProperties { mass, com, inertia }
    }
}

#[inline]
pub(crate) fn outer(a: Vec3, b: Vec3) -> Mat3 {
    Mat3::from_cols(a * b.x, a * b.y, a * b.z)
}

/// A face of a convex hull: plane `dot(normal, x) = d` and a CCW vertex loop.
#[derive(Clone, Copy, Debug)]
pub struct HullFace {
    pub normal: Vec3,
    pub d: f32,
    pub first: u32,
    pub count: u32,
}

/// A unique hull edge with its two adjacent faces.
#[derive(Clone, Copy, Debug)]
pub struct HullEdge {
    pub v0: u32,
    pub v1: u32,
    pub f0: u32,
    pub f1: u32,
}

/// A convex polytope with face, edge and vertex adjacency, as used by SAT.
#[derive(Clone, Debug)]
pub struct ConvexHull {
    pub verts: Vec<Vec3>,
    pub faces: Vec<HullFace>,
    pub face_verts: Vec<u32>,
    pub edges: Vec<HullEdge>,
    pub centroid: Vec3,
    pub aabb: Aabb,
}

impl ConvexHull {
    pub fn cuboid(h: Vec3) -> Self {
        let mut pts = Vec::with_capacity(8);
        for i in 0..8 {
            pts.push(Vec3::new(
                if i & 1 == 0 { -h.x } else { h.x },
                if i & 2 == 0 { -h.y } else { h.y },
                if i & 4 == 0 { -h.z } else { h.z },
            ));
        }
        Self::from_points(&pts).expect("box hull")
    }

    /// Build the hull of a point cloud by brute-force plane enumeration (fine up to ~100 points).
    pub fn from_points(points: &[Vec3]) -> Option<Self> {
        let mut pts: Vec<Vec3> = Vec::with_capacity(points.len());
        let scale = points.iter().fold(0.0f32, |m, p| m.max(p.abs().max_element())).max(1e-3);
        let weld = 1e-5 * scale;
        for p in points {
            if !pts.iter().any(|q| q.distance_squared(*p) < weld * weld) {
                pts.push(*p);
            }
        }
        let n = pts.len();
        if n < 4 {
            return None;
        }
        let tol = 1e-4 * scale;
        let mut planes: Vec<(Vec3, f32)> = Vec::new();
        for i in 0..n {
            for j in (i + 1)..n {
                for k in (j + 1)..n {
                    let nrm = (pts[j] - pts[i]).cross(pts[k] - pts[i]);
                    let len = nrm.length();
                    if len < 1e-7 * scale * scale {
                        continue;
                    }
                    let nrm = nrm / len;
                    let d = nrm.dot(pts[i]);
                    let mut above = false;
                    let mut below = false;
                    for p in &pts {
                        let s = nrm.dot(*p) - d;
                        if s > tol {
                            above = true;
                        } else if s < -tol {
                            below = true;
                        }
                        if above && below {
                            break;
                        }
                    }
                    if above && below {
                        continue;
                    }
                    let (pn, pd) = if above { (-nrm, -d) } else { (nrm, d) };
                    if !planes.iter().any(|(qn, qd)| qn.dot(pn) > 1.0 - 1e-5 && (qd - pd).abs() < tol * 4.0) {
                        planes.push((pn, pd));
                    }
                }
            }
        }
        if planes.len() < 4 {
            return None;
        }
        // Build faces: ordered convex polygons of on-plane points.
        let mut used = vec![u32::MAX; n];
        let mut verts: Vec<Vec3> = Vec::new();
        let mut faces = Vec::new();
        let mut face_verts = Vec::new();
        for (pn, pd) in &planes {
            let on: Vec<usize> = (0..n).filter(|&i| (pn.dot(pts[i]) - pd).abs() <= tol * 2.0).collect();
            if on.len() < 3 {
                continue;
            }
            let (u, v) = crate::math::orthonormal_basis(*pn);
            let c = on.iter().map(|&i| pts[i]).sum::<Vec3>() / on.len() as f32;
            let loop2 = convex_polygon_2d(&on.iter().map(|&i| ((pts[i] - c).dot(u), (pts[i] - c).dot(v), i)).collect::<Vec<_>>());
            if loop2.len() < 3 {
                continue;
            }
            let first = face_verts.len() as u32;
            for &i in &loop2 {
                if used[i] == u32::MAX {
                    used[i] = verts.len() as u32;
                    verts.push(pts[i]);
                }
                face_verts.push(used[i]);
            }
            faces.push(HullFace { normal: *pn, d: *pd, first, count: loop2.len() as u32 });
        }
        if faces.len() < 4 {
            return None;
        }
        // Edges with adjacent faces.
        let mut edge_map: rustc_hash::FxHashMap<(u32, u32), usize> = Default::default();
        let mut edges: Vec<HullEdge> = Vec::new();
        for (fi, f) in faces.iter().enumerate() {
            for k in 0..f.count {
                let a = face_verts[(f.first + k) as usize];
                let b = face_verts[(f.first + (k + 1) % f.count) as usize];
                let key = (a.min(b), a.max(b));
                if let Some(&e) = edge_map.get(&key) {
                    edges[e].f1 = fi as u32;
                } else {
                    edge_map.insert(key, edges.len());
                    edges.push(HullEdge { v0: a, v1: b, f0: fi as u32, f1: u32::MAX });
                }
            }
        }
        // Edges must be shared by exactly two faces; drop any malformed ones (degenerate input).
        edges.retain(|e| e.f1 != u32::MAX);
        let centroid = verts.iter().sum::<Vec3>() / verts.len() as f32;
        let aabb = Aabb::from_points(verts.iter().copied());
        Some(ConvexHull { verts, faces, face_verts, edges, centroid, aabb })
    }

    #[inline]
    pub fn face_vertices(&self, f: usize) -> impl Iterator<Item = Vec3> + '_ {
        let face = self.faces[f];
        (face.first..face.first + face.count).map(move |i| self.verts[self.face_verts[i as usize] as usize])
    }

    /// Farthest vertex along a local direction.
    #[inline]
    pub fn support(&self, dir: Vec3) -> Vec3 {
        let mut best = self.verts[0];
        let mut bd = best.dot(dir);
        for v in &self.verts[1..] {
            let d = v.dot(dir);
            if d > bd {
                bd = d;
                best = *v;
            }
        }
        best
    }

    pub fn volume(&self) -> f32 {
        self.mass_properties(1.0).mass
    }

    /// Exact mass properties of the solid polytope (tetrahedral decomposition).
    pub fn mass_properties(&self, density: f32) -> MassProperties {
        let r = self.centroid;
        let mut vol = 0.0f32;
        let mut com = Vec3::ZERO;
        let mut cov = Mat3::ZERO;
        let canon = Mat3::from_cols(Vec3::new(2.0, 1.0, 1.0), Vec3::new(1.0, 2.0, 1.0), Vec3::new(1.0, 1.0, 2.0)) * (1.0 / 120.0);
        for f in 0..self.faces.len() {
            let vs: Vec<Vec3> = self.face_vertices(f).map(|v| v - r).collect();
            for k in 1..vs.len() - 1 {
                let a = Mat3::from_cols(vs[0], vs[k], vs[k + 1]);
                let det = a.determinant();
                vol += det / 6.0;
                com += (vs[0] + vs[k] + vs[k + 1]) * (det / 24.0);
                cov += a * canon * a.transpose() * det;
            }
        }
        if vol <= 1e-12 {
            return MassProperties::default();
        }
        let c = com / vol;
        let m = vol * density;
        let cov = cov * density - outer(c, c) * m;
        let inertia = Mat3::from_diagonal(Vec3::splat(cov.x_axis.x + cov.y_axis.y + cov.z_axis.z)) - cov;
        MassProperties { mass: m, com: r + c, inertia }
    }

    /// Ray against the solid polytope (local frame).
    pub fn ray_cast(&self, origin: Vec3, dir: Vec3, max_t: f32, solid: bool) -> Option<(f32, Vec3)> {
        let mut t_enter = 0.0f32;
        let mut t_exit = max_t;
        let mut n_enter = Vec3::ZERO;
        let mut inside = true;
        for f in &self.faces {
            let dist = f.normal.dot(origin) - f.d;
            let denom = f.normal.dot(dir);
            if dist > 0.0 {
                inside = false;
            }
            if denom.abs() < 1e-12 {
                if dist > 0.0 {
                    return None;
                }
                continue;
            }
            let t = -dist / denom;
            if denom < 0.0 {
                if t > t_enter {
                    t_enter = t;
                    n_enter = f.normal;
                }
            } else if t < t_exit {
                t_exit = t;
            }
            if t_enter > t_exit {
                return None;
            }
        }
        if inside {
            return if solid { Some((0.0, -dir)) } else { None };
        }
        Some((t_enter, n_enter))
    }
}

/// Convex hull of 2D points (monotone chain), CCW, dropping collinear points. Returns source ids.
fn convex_polygon_2d(pts: &[(f32, f32, usize)]) -> Vec<usize> {
    let mut p: Vec<(f32, f32, usize)> = pts.to_vec();
    p.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    let cross = |o: (f32, f32, usize), a: (f32, f32, usize), b: (f32, f32, usize)| (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0);
    let scale = p.iter().fold(0.0f32, |m, q| m.max(q.0.abs()).max(q.1.abs())).max(1e-6);
    let eps = 1e-9 * scale * scale;
    let mut hull: Vec<(f32, f32, usize)> = Vec::with_capacity(p.len() * 2);
    for &q in &p {
        while hull.len() >= 2 && cross(hull[hull.len() - 2], hull[hull.len() - 1], q) <= eps {
            hull.pop();
        }
        hull.push(q);
    }
    let lower = hull.len() + 1;
    for &q in p.iter().rev().skip(1) {
        while hull.len() >= lower && cross(hull[hull.len() - 2], hull[hull.len() - 1], q) <= eps {
            hull.pop();
        }
        hull.push(q);
    }
    hull.pop();
    hull.into_iter().map(|q| q.2).collect()
}

/// A static triangle mesh with a BVH and per-edge convexity flags.
#[derive(Clone, Debug)]
pub struct TriMesh {
    pub vertices: Vec<Vec3>,
    pub triangles: Vec<[u32; 3]>,
    pub normals: Vec<Vec3>,
    /// Bit k set: edge k (v[k] -> v[k+1]) is a real feature edge (open, or convex with a noticeable angle).
    /// Contacts on inactive edges are snapped to the triangle normal to avoid "ghost" bumps.
    pub edge_flags: Vec<u8>,
    pub bvh: Bvh,
}

impl TriMesh {
    pub fn new(vertices: Vec<Vec3>, triangles: Vec<[u32; 3]>) -> Self {
        let normals: Vec<Vec3> = triangles
            .iter()
            .map(|t| {
                let (a, b, c) = (vertices[t[0] as usize], vertices[t[1] as usize], vertices[t[2] as usize]);
                (b - a).cross(c - a).normalize_or_zero()
            })
            .collect();
        let mut edge_owner: rustc_hash::FxHashMap<(u32, u32), smallvec::SmallVec<[usize; 2]>> = Default::default();
        for (ti, t) in triangles.iter().enumerate() {
            for k in 0..3 {
                let a = t[k];
                let b = t[(k + 1) % 3];
                edge_owner.entry((a.min(b), a.max(b))).or_default().push(ti);
            }
        }
        let mut edge_flags = vec![0u8; triangles.len()];
        for (ti, t) in triangles.iter().enumerate() {
            for k in 0..3 {
                let a = t[k];
                let b = t[(k + 1) % 3];
                let key = (a.min(b), a.max(b));
                let neighbour = edge_owner.get(&key).and_then(|o| o.iter().copied().find(|&o| o != ti));
                let active = match neighbour {
                    None => true,
                    Some(o) => {
                        let n0 = normals[ti];
                        let n1 = normals[o];
                        if n0.dot(n1) > 0.999 {
                            false
                        } else {
                            // Convex if the neighbour's far vertex lies below our plane.
                            let far = triangles[o].iter().copied().find(|v| *v != a && *v != b).unwrap_or(a);
                            let s = n0.dot(vertices[far as usize] - vertices[a as usize]);
                            s < 0.0
                        }
                    }
                };
                if active {
                    edge_flags[ti] |= 1 << k;
                }
            }
        }
        let boxes: Vec<Aabb> = triangles
            .iter()
            .map(|t| Aabb::from_points(t.iter().map(|&i| vertices[i as usize])))
            .collect();
        let bvh = Bvh::build(&boxes);
        TriMesh { vertices, triangles, normals, edge_flags, bvh }
    }

    #[inline]
    pub fn triangle(&self, i: usize) -> [Vec3; 3] {
        let t = self.triangles[i];
        [self.vertices[t[0] as usize], self.vertices[t[1] as usize], self.vertices[t[2] as usize]]
    }

    /// Closest front- or back-face hit. Returns (t, normal facing the ray, triangle).
    pub fn ray_cast(&self, origin: Vec3, dir: Vec3, max_t: f32) -> Option<(f32, Vec3, usize)> {
        let mut best: Option<(f32, Vec3, usize)> = None;
        let mut limit = max_t;
        self.bvh.ray_cast(origin, dir, max_t, &mut |i, _| {
            let [a, b, c] = self.triangle(i);
            if let Some(t) = ray_triangle(origin, dir, a, b, c) {
                if t <= limit {
                    limit = t;
                    let n = self.normals[i];
                    best = Some((t, if n.dot(dir) > 0.0 { -n } else { n }, i));
                }
            }
            limit
        });
        best
    }
}

/// Möller–Trumbore, two-sided.
pub fn ray_triangle(o: Vec3, d: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Option<f32> {
    let e1 = b - a;
    let e2 = c - a;
    let p = d.cross(e2);
    let det = e1.dot(p);
    if det.abs() < 1e-12 {
        return None;
    }
    let inv = 1.0 / det;
    let s = o - a;
    let u = s.dot(p) * inv;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = s.cross(e1);
    let v = d.dot(q) * inv;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = e2.dot(q) * inv;
    if t >= 0.0 {
        Some(t)
    } else {
        None
    }
}

pub fn ray_sphere(o: Vec3, d: Vec3, c: Vec3, r: f32, max_t: f32, solid: bool) -> Option<(f32, Vec3)> {
    let m = o - c;
    let b = m.dot(d);
    let cc = m.dot(m) - r * r;
    if cc <= 0.0 {
        return if solid { Some((0.0, -d)) } else { None };
    }
    if b > 0.0 {
        return None;
    }
    let disc = b * b - cc;
    if disc < 0.0 {
        return None;
    }
    let t = -b - disc.sqrt();
    if t > max_t {
        return None;
    }
    let t = t.max(0.0);
    Some((t, (o + d * t - c).normalize_or_zero()))
}

pub fn ray_capsule(o: Vec3, d: Vec3, hh: f32, r: f32, max_t: f32, solid: bool) -> Option<(f32, Vec3)> {
    let a = Vec3::new(0.0, -hh, 0.0);
    let b = Vec3::new(0.0, hh, 0.0);
    let (cp, _) = crate::math::closest_point_segment(o, a, b);
    if o.distance_squared(cp) <= r * r {
        return if solid { Some((0.0, -d)) } else { None };
    }
    // Infinite cylinder around Y, then caps.
    let mut best: Option<(f32, Vec3)> = None;
    let dx = Vec3::new(d.x, 0.0, d.z);
    let ox = Vec3::new(o.x, 0.0, o.z);
    let qa = dx.dot(dx);
    if qa > 1e-12 {
        let qb = ox.dot(dx);
        let qc = ox.dot(ox) - r * r;
        let disc = qb * qb - qa * qc;
        if disc >= 0.0 {
            let t = (-qb - disc.sqrt()) / qa;
            if t >= 0.0 && t <= max_t {
                let p = o + d * t;
                if p.y.abs() <= hh {
                    best = Some((t, Vec3::new(p.x, 0.0, p.z).normalize_or_zero()));
                }
            }
        }
    }
    for cap in [a, b] {
        if let Some((t, n)) = ray_sphere(o, d, cap, r, max_t, false) {
            if best.is_none_or(|(bt, _)| t < bt) {
                best = Some((t, n));
            }
        }
    }
    best
}

pub fn ray_box(o: Vec3, d: Vec3, h: Vec3, max_t: f32, solid: bool) -> Option<(f32, Vec3)> {
    let mut tmin = 0.0f32;
    let mut tmax = max_t;
    let mut n = Vec3::ZERO;
    let mut inside = true;
    for axis in 0..3 {
        let (oa, da, ha) = (o[axis], d[axis], h[axis]);
        if oa.abs() > ha {
            inside = false;
        }
        if da.abs() < 1e-12 {
            if oa.abs() > ha {
                return None;
            }
            continue;
        }
        let inv = 1.0 / da;
        let mut t1 = (-ha - oa) * inv;
        let mut t2 = (ha - oa) * inv;
        let mut sign = -1.0;
        if t1 > t2 {
            std::mem::swap(&mut t1, &mut t2);
            sign = 1.0;
        }
        if t1 > tmin {
            tmin = t1;
            n = Vec3::ZERO;
            n[axis] = sign;
        }
        tmax = tmax.min(t2);
        if tmin > tmax {
            return None;
        }
    }
    if inside {
        return if solid { Some((0.0, -d)) } else { None };
    }
    Some((tmin, n))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn box_hull_topology() {
        let h = ConvexHull::cuboid(Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(h.verts.len(), 8);
        assert_eq!(h.faces.len(), 6);
        assert_eq!(h.edges.len(), 12);
        let mp = h.mass_properties(1.0);
        assert!((mp.mass - 48.0).abs() < 1e-3, "{}", mp.mass);
        // Same as analytic cuboid inertia.
        let a = Shape::cuboid(Vec3::new(1.0, 2.0, 3.0)).mass_properties(1.0);
        assert!((mp.inertia.x_axis.x - a.inertia.x_axis.x).abs() < 1e-2);
        assert!((mp.inertia.y_axis.y - a.inertia.y_axis.y).abs() < 1e-2);
        assert!(mp.inertia.x_axis.y.abs() < 1e-3);
    }

    #[test]
    fn hull_of_cloud_contains_points() {
        let mut pts = Vec::new();
        let mut s = 1u32;
        for _ in 0..40 {
            s = s.wrapping_mul(1664525).wrapping_add(1013904223);
            let x = (s >> 8) as f32 / 16777216.0 - 0.5;
            s = s.wrapping_mul(1664525).wrapping_add(1013904223);
            let y = (s >> 8) as f32 / 16777216.0 - 0.5;
            s = s.wrapping_mul(1664525).wrapping_add(1013904223);
            let z = (s >> 8) as f32 / 16777216.0 - 0.5;
            pts.push(Vec3::new(x, y, z));
        }
        let h = ConvexHull::from_points(&pts).unwrap();
        // Euler characteristic V - E + F = 2.
        assert_eq!(h.verts.len() as i32 - h.edges.len() as i32 + h.faces.len() as i32, 2);
        for p in &pts {
            for f in &h.faces {
                assert!(f.normal.dot(*p) - f.d < 1e-3);
            }
        }
    }

    #[test]
    fn ray_casts() {
        let b = Shape::cuboid(Vec3::ONE);
        let (t, n) = b.ray_cast_local(Vec3::new(-5.0, 0.0, 0.0), Vec3::X, 100.0, true).unwrap();
        assert!((t - 4.0).abs() < 1e-5 && n == Vec3::new(-1.0, 0.0, 0.0));
        let h = Shape::convex(&ConvexHull::cuboid(Vec3::ONE).verts).unwrap();
        let (t2, n2) = h.ray_cast_local(Vec3::new(0.0, 5.0, 0.0), -Vec3::Y, 100.0, true).unwrap();
        assert!((t2 - 4.0).abs() < 1e-4 && (n2 - Vec3::Y).length() < 1e-4);
        let c = Shape::capsule(1.0, 0.5);
        let (t3, _) = c.ray_cast_local(Vec3::new(0.0, 5.0, 0.0), -Vec3::Y, 100.0, true).unwrap();
        assert!((t3 - 3.5).abs() < 1e-4);
        let (t4, n4) = c.ray_cast_local(Vec3::new(5.0, 0.3, 0.0), -Vec3::X, 100.0, true).unwrap();
        assert!((t4 - 4.5).abs() < 1e-4 && (n4 - Vec3::X).length() < 1e-4);
    }
}
