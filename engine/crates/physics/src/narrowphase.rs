//! Contact generation between shape pairs.
//!
//! Polytopes (boxes, hulls, mesh triangles) use the separating axis test with Gauss-map edge pruning
//! (Gregorius, GDC 2013) and reference-face clipping. Spheres and capsules use GJK on their core
//! (point / segment) plus the radius. All functions report speculative contacts up to `margin`.

use crate::gjk::{gjk_distance, PointSet};
use crate::math::{closest_point_segment, closest_point_triangle, closest_segment_segment, Pose};
use crate::shape::{ConvexHull, Shape, TriMesh};
use glam::Vec3;
use smallvec::SmallVec;

/// Linear tolerance used to prefer face contacts over edge contacts.
const LINEAR_SLOP: f32 = 0.005;

#[derive(Clone, Copy, Debug)]
pub struct PointGeom {
    /// World point on A.
    pub pa: Vec3,
    /// World point on B.
    pub pb: Vec3,
    /// Signed distance along the normal (negative = penetrating).
    pub separation: f32,
}

#[derive(Clone, Debug, Default)]
pub struct ManifoldGeom {
    /// Unit normal pointing from A to B.
    pub normal: Vec3,
    pub points: SmallVec<[PointGeom; 4]>,
    /// Sub-shape key (triangle index for meshes) for contact matching.
    pub key: u32,
}

impl ManifoldGeom {
    fn flip(&mut self) {
        self.normal = -self.normal;
        for p in &mut self.points {
            std::mem::swap(&mut p.pa, &mut p.pb);
        }
    }
}

fn rank(s: &Shape) -> u8 {
    match s {
        Shape::Sphere { .. } => 0,
        Shape::Capsule { .. } => 1,
        Shape::Cuboid { .. } | Shape::Convex(_) => 2,
        Shape::TriMesh(_) => 3,
    }
}

/// Generate contact manifolds between two posed shapes. Appends to `out`.
pub fn collide(sa: &Shape, pa: &Pose, sb: &Shape, pb: &Pose, margin: f32, out: &mut Vec<ManifoldGeom>) {
    if rank(sa) > rank(sb) {
        let start = out.len();
        collide_ordered(sb, pb, sa, pa, margin, out);
        for m in &mut out[start..] {
            m.flip();
        }
    } else {
        collide_ordered(sa, pa, sb, pb, margin, out);
    }
}

fn capsule_segment(hh: f32, pose: &Pose) -> (Vec3, Vec3) {
    (pose.transform_point(Vec3::new(0.0, -hh, 0.0)), pose.transform_point(Vec3::new(0.0, hh, 0.0)))
}

fn collide_ordered(sa: &Shape, pa: &Pose, sb: &Shape, pb: &Pose, margin: f32, out: &mut Vec<ManifoldGeom>) {
    match (sa, sb) {
        (Shape::Sphere { radius: ra }, Shape::Sphere { radius: rb }) => {
            if let Some(m) = sphere_sphere(pa.p, *ra, pb.p, *rb, margin) {
                out.push(m);
            }
        }
        (Shape::Sphere { radius: ra }, Shape::Capsule { half_height, radius: rb }) => {
            let (s0, s1) = capsule_segment(*half_height, pb);
            let (c, _) = closest_point_segment(pa.p, s0, s1);
            if let Some(m) = sphere_sphere(pa.p, *ra, c, *rb, margin) {
                out.push(m);
            }
        }
        (Shape::Capsule { half_height: ha, radius: ra }, Shape::Capsule { half_height: hb, radius: rb }) => {
            let (a0, a1) = capsule_segment(*ha, pa);
            let (b0, b1) = capsule_segment(*hb, pb);
            if let Some(m) = capsule_capsule(a0, a1, *ra, b0, b1, *rb, margin) {
                out.push(m);
            }
        }
        (Shape::Sphere { radius }, _) if sb.hull().is_some() => {
            let poly = WorldPoly::from_hull(sb.hull().unwrap(), pb);
            if let Some(m) = core_poly(&[pa.p], *radius, &poly, margin) {
                out.push(m);
            }
        }
        (Shape::Capsule { half_height, radius }, _) if sb.hull().is_some() => {
            let (a0, a1) = capsule_segment(*half_height, pa);
            let poly = WorldPoly::from_hull(sb.hull().unwrap(), pb);
            if let Some(m) = core_poly(&[a0, a1], *radius, &poly, margin) {
                out.push(m);
            }
        }
        (_, _) if sa.hull().is_some() && sb.hull().is_some() => {
            let a = WorldPoly::from_hull(sa.hull().unwrap(), pa);
            let b = WorldPoly::from_hull(sb.hull().unwrap(), pb);
            if let Some(m) = poly_poly(&a, &b, margin) {
                out.push(m);
            }
        }
        (_, Shape::TriMesh(mesh)) => collide_mesh(sa, pa, mesh, pb, margin, out),
        _ => {}
    }
}

fn sphere_sphere(ca: Vec3, ra: f32, cb: Vec3, rb: f32, margin: f32) -> Option<ManifoldGeom> {
    let d = cb - ca;
    let dist = d.length();
    let sep = dist - ra - rb;
    if sep > margin {
        return None;
    }
    let n = if dist > 1e-6 { d / dist } else { Vec3::Y };
    let mut m = ManifoldGeom { normal: n, ..Default::default() };
    m.points.push(PointGeom { pa: ca + n * ra, pb: cb - n * rb, separation: sep });
    Some(m)
}

fn capsule_capsule(a0: Vec3, a1: Vec3, ra: f32, b0: Vec3, b1: Vec3, rb: f32, margin: f32) -> Option<ManifoldGeom> {
    let (_, _, c1, c2) = closest_segment_segment(a0, a1, b0, b1);
    let d = c2 - c1;
    let dist = d.length();
    if dist - ra - rb > margin {
        return None;
    }
    let da = a1 - a0;
    let db = b1 - b0;
    let n = if dist > 1e-6 {
        d / dist
    } else {
        let c = da.cross(db);
        if c.length_squared() > 1e-10 {
            c.normalize()
        } else {
            crate::math::orthonormal_basis(da.normalize_or(Vec3::Y)).0
        }
    };
    let mut m = ManifoldGeom { normal: n, ..Default::default() };
    let la = da.length();
    let lb = db.length();
    // Parallel capsules lying side by side get two points for stability.
    if la > 1e-4 && lb > 1e-4 && (da.dot(db) / (la * lb)).abs() > 0.99 {
        let ua = da / la;
        let t0 = (b0 - a0).dot(ua).clamp(0.0, la);
        let t1 = (b1 - a0).dot(ua).clamp(0.0, la);
        if (t1 - t0).abs() > 1e-3 {
            for t in [t0, t1] {
                let pa = a0 + ua * t;
                let (pb, _) = closest_point_segment(pa, b0, b1);
                let sep = (pb - pa).dot(n) - ra - rb;
                if sep <= margin {
                    m.points.push(PointGeom { pa: pa + n * ra, pb: pb - n * rb, separation: sep });
                }
            }
            if !m.points.is_empty() {
                return Some(m);
            }
        }
    }
    m.points.push(PointGeom { pa: c1 + n * ra, pb: c2 - n * rb, separation: dist - ra - rb });
    Some(m)
}

/// A polytope in world space.
pub struct WorldPoly {
    pub verts: SmallVec<[Vec3; 32]>,
    pub planes: SmallVec<[(Vec3, f32); 32]>,
    /// Face loops as (first, count) into `loops`.
    pub faces: SmallVec<[(u32, u32); 32]>,
    pub loops: SmallVec<[u32; 96]>,
    /// (v0, v1, face0, face1)
    pub edges: SmallVec<[(u32, u32, u32, u32); 48]>,
    pub centroid: Vec3,
}

impl WorldPoly {
    pub fn from_hull(h: &ConvexHull, pose: &Pose) -> Self {
        let verts: SmallVec<[Vec3; 32]> = h.verts.iter().map(|v| pose.transform_point(*v)).collect();
        let planes = h
            .faces
            .iter()
            .map(|f| {
                let n = pose.transform_vector(f.normal);
                let p0 = verts[h.face_verts[f.first as usize] as usize];
                (n, n.dot(p0))
            })
            .collect();
        WorldPoly {
            verts,
            planes,
            faces: h.faces.iter().map(|f| (f.first, f.count)).collect(),
            loops: h.face_verts.iter().copied().collect(),
            edges: h.edges.iter().map(|e| (e.v0, e.v1, e.f0, e.f1)).collect(),
            centroid: pose.transform_point(h.centroid),
        }
    }

    /// A one- or two-faced triangle (two faces give edges a proper Gauss map).
    pub fn triangle(a: Vec3, b: Vec3, c: Vec3) -> Self {
        let n = (b - a).cross(c - a).normalize_or(Vec3::Y);
        let mut faces = SmallVec::new();
        faces.push((0, 3));
        faces.push((3, 3));
        let mut loops = SmallVec::new();
        loops.extend_from_slice(&[0, 1, 2, 0, 2, 1]);
        let mut edges = SmallVec::new();
        edges.extend_from_slice(&[(0, 1, 0, 1), (1, 2, 0, 1), (2, 0, 0, 1)]);
        let mut planes = SmallVec::new();
        planes.push((n, n.dot(a)));
        planes.push((-n, -n.dot(a)));
        let mut verts = SmallVec::new();
        verts.extend_from_slice(&[a, b, c]);
        WorldPoly { verts, planes, faces, loops, edges, centroid: (a + b + c) / 3.0 }
    }

    #[inline]
    fn face_loop(&self, f: usize) -> &[u32] {
        let (first, count) = self.faces[f];
        &self.loops[first as usize..(first + count) as usize]
    }

    #[inline]
    fn min_along(&self, n: Vec3) -> f32 {
        self.verts.iter().fold(f32::MAX, |m, v| m.min(v.dot(n)))
    }
}

#[derive(Clone, Copy)]
struct FaceQuery {
    sep: f32,
    face: usize,
}

#[derive(Clone, Copy)]
struct EdgeQuery {
    sep: f32,
    ea: usize,
    eb: usize,
    axis: Vec3,
}

fn query_faces(a: &WorldPoly, b: &WorldPoly) -> FaceQuery {
    let mut best = FaceQuery { sep: f32::MIN, face: 0 };
    for (i, (n, d)) in a.planes.iter().enumerate() {
        let s = b.min_along(*n) - d;
        if s > best.sep {
            best = FaceQuery { sep: s, face: i };
        }
    }
    best
}

#[inline]
fn is_minkowski_face(a: Vec3, b: Vec3, bxa: Vec3, c: Vec3, d: Vec3, dxc: Vec3) -> bool {
    let cba = c.dot(bxa);
    let dba = d.dot(bxa);
    let adc = a.dot(dxc);
    let bdc = b.dot(dxc);
    cba * dba < 0.0 && adc * bdc < 0.0 && cba * bdc > 0.0
}

fn query_edges(a: &WorldPoly, b: &WorldPoly) -> EdgeQuery {
    let mut best = EdgeQuery { sep: f32::MIN, ea: 0, eb: 0, axis: Vec3::ZERO };
    for (ia, &(a0, a1, af0, af1)) in a.edges.iter().enumerate() {
        let pa = a.verts[a0 as usize];
        let qa = a.verts[a1 as usize];
        let ua = a.planes[af0 as usize].0;
        let va = a.planes[af1 as usize].0;
        let ea = qa - pa;
        for (ib, &(b0, b1, bf0, bf1)) in b.edges.iter().enumerate() {
            let pb = b.verts[b0 as usize];
            let qb = b.verts[b1 as usize];
            let ub = -b.planes[bf0 as usize].0;
            let vb = -b.planes[bf1 as usize].0;
            let eb = qb - pb;
            if !is_minkowski_face(ua, va, va.cross(ua), ub, vb, vb.cross(ub)) {
                continue;
            }
            let axis = ea.cross(eb);
            let len = axis.length();
            // Skip near-parallel edges: the face queries cover them.
            if len < 1e-5 * (ea.length() * eb.length()).max(1e-12) {
                continue;
            }
            let mut axis = axis / len;
            if axis.dot(pa - a.centroid) < 0.0 {
                axis = -axis;
            }
            let s = axis.dot(pb - pa);
            if s > best.sep {
                best = EdgeQuery { sep: s, ea: ia, eb: ib, axis };
            }
        }
    }
    best
}

/// Polytope vs polytope.
pub fn poly_poly(a: &WorldPoly, b: &WorldPoly, margin: f32) -> Option<ManifoldGeom> {
    let fa = query_faces(a, b);
    if fa.sep > margin {
        return None;
    }
    let fb = query_faces(b, a);
    if fb.sep > margin {
        return None;
    }
    let eq = query_edges(a, b);
    if eq.sep > margin {
        return None;
    }
    const K_REL: f32 = 0.95;
    let k_abs = 0.5 * LINEAR_SLOP;
    let face_max = fa.sep.max(fb.sep);
    if eq.sep > K_REL * face_max + k_abs {
        return Some(edge_contact(a, b, &eq));
    }
    if fb.sep > K_REL * fa.sep + k_abs {
        let mut m = face_contact(b, fb.face, a, margin)?;
        m.flip();
        Some(m)
    } else {
        face_contact(a, fa.face, b, margin)
    }
}

fn edge_contact(a: &WorldPoly, b: &WorldPoly, q: &EdgeQuery) -> ManifoldGeom {
    let (a0, a1, _, _) = a.edges[q.ea];
    let (b0, b1, _, _) = b.edges[q.eb];
    let (_, _, c1, c2) = closest_segment_segment(a.verts[a0 as usize], a.verts[a1 as usize], b.verts[b0 as usize], b.verts[b1 as usize]);
    let mut m = ManifoldGeom { normal: q.axis, ..Default::default() };
    m.points.push(PointGeom { pa: c1, pb: c2, separation: q.sep });
    m
}

/// Clip the incident face of `inc` against reference face `rf` of `refp`. Normal points from ref to inc.
fn face_contact(refp: &WorldPoly, rf: usize, inc: &WorldPoly, margin: f32) -> Option<ManifoldGeom> {
    let (n, d) = refp.planes[rf];
    // Incident face: most anti-parallel to the reference normal.
    let mut inc_face = 0;
    let mut min_dot = f32::MAX;
    for (i, (ni, _)) in inc.planes.iter().enumerate() {
        let dd = ni.dot(n);
        if dd < min_dot {
            min_dot = dd;
            inc_face = i;
        }
    }
    let mut poly: SmallVec<[Vec3; 16]> = inc.face_loop(inc_face).iter().map(|&i| inc.verts[i as usize]).collect();
    let rl = refp.face_loop(rf);
    let mut tmp: SmallVec<[Vec3; 16]> = SmallVec::new();
    for k in 0..rl.len() {
        let v0 = refp.verts[rl[k] as usize];
        let v1 = refp.verts[rl[(k + 1) % rl.len()] as usize];
        let side = (v1 - v0).cross(n);
        let sl = side.length();
        if sl < 1e-9 {
            continue;
        }
        let side = side / sl;
        let sd = side.dot(v0);
        clip_polygon(&poly, side, sd, &mut tmp);
        std::mem::swap(&mut poly, &mut tmp);
        if poly.is_empty() {
            return None;
        }
    }
    let mut m = ManifoldGeom { normal: n, ..Default::default() };
    let mut pts: SmallVec<[PointGeom; 16]> = SmallVec::new();
    for p in &poly {
        let s = n.dot(*p) - d;
        if s <= margin {
            pts.push(PointGeom { pa: *p - n * s, pb: *p, separation: s });
        }
    }
    if pts.is_empty() {
        return None;
    }
    reduce_points(&pts, n, &mut m.points);
    Some(m)
}

/// Sutherland–Hodgman against the half-space dot(n, x) <= d.
fn clip_polygon(input: &[Vec3], n: Vec3, d: f32, out: &mut SmallVec<[Vec3; 16]>) {
    out.clear();
    if input.is_empty() {
        return;
    }
    let mut a = input[input.len() - 1];
    let mut da = n.dot(a) - d;
    for &b in input {
        let db = n.dot(b) - d;
        if da <= 0.0 {
            out.push(a);
            if db > 0.0 {
                out.push(a + (b - a) * (da / (da - db)));
            }
        } else if db <= 0.0 {
            out.push(a + (b - a) * (da / (da - db)));
        }
        a = b;
        da = db;
    }
}

/// Keep at most four points: deepest, farthest from it, then those maximising the contact area.
fn reduce_points(pts: &[PointGeom], n: Vec3, out: &mut SmallVec<[PointGeom; 4]>) {
    if pts.len() <= 4 {
        out.extend_from_slice(pts);
        return;
    }
    let deepest = (0..pts.len()).min_by(|&i, &j| pts[i].separation.total_cmp(&pts[j].separation)).unwrap();
    let p0 = pts[deepest].pb;
    let i1 = (0..pts.len()).max_by(|&i, &j| pts[i].pb.distance_squared(p0).total_cmp(&pts[j].pb.distance_squared(p0))).unwrap();
    let p1 = pts[i1].pb;
    let area = |q: Vec3| (p1 - p0).cross(q - p0).dot(n);
    let i2 = (0..pts.len()).max_by(|&i, &j| area(pts[i].pb).abs().total_cmp(&area(pts[j].pb).abs())).unwrap();
    let sign = area(pts[i2].pb).signum();
    let p2 = pts[i2].pb;
    // Fourth point: maximise the area added on the opposite side of the triangle.
    let added = |q: Vec3| {
        let a01 = (p1 - p0).cross(q - p0).dot(n) * -sign;
        let a12 = (p2 - p1).cross(q - p1).dot(n) * -sign;
        let a20 = (p0 - p2).cross(q - p2).dot(n) * -sign;
        a01.max(a12).max(a20)
    };
    let i3 = (0..pts.len())
        .filter(|&i| i != deepest && i != i1 && i != i2)
        .max_by(|&i, &j| added(pts[i].pb).total_cmp(&added(pts[j].pb)));
    out.push(pts[deepest]);
    out.push(pts[i1]);
    if i2 != deepest && i2 != i1 {
        out.push(pts[i2]);
    }
    if let Some(i3) = i3 {
        if added(pts[i3].pb) > 0.0 {
            out.push(pts[i3]);
        }
    }
}

/// Point or segment core (inflated by `r`) vs polytope.
fn core_poly(core: &[Vec3], r: f32, poly: &WorldPoly, margin: f32) -> Option<ManifoldGeom> {
    let g = gjk_distance(&PointSet(core), &PointSet(&poly.verts), poly.centroid - core[0]);
    if !g.overlap && g.distance > 1e-5 {
        if g.distance - r > margin {
            return None;
        }
        let n = g.normal;
        let mut m = ManifoldGeom { normal: n, ..Default::default() };
        if core.len() == 2 {
            // Segment lying flat on a face: two points from clipping the segment to the face.
            let mut best_face = 0;
            let mut best = f32::MIN;
            for (i, (fnrm, _)) in poly.planes.iter().enumerate() {
                let dd = fnrm.dot(-n);
                if dd > best {
                    best = dd;
                    best_face = i;
                }
            }
            let seg = core[1] - core[0];
            let (fnrm, fd) = poly.planes[best_face];
            if best > 0.98 && seg.dot(fnrm).abs() < 0.2 * seg.length() {
                let mut pts: SmallVec<[Vec3; 16]> = SmallVec::new();
                pts.push(core[0]);
                pts.push(core[1]);
                let fl = poly.face_loop(best_face);
                let mut ok = true;
                for k in 0..fl.len() {
                    let v0 = poly.verts[fl[k] as usize];
                    let v1 = poly.verts[fl[(k + 1) % fl.len()] as usize];
                    let side = (v1 - v0).cross(fnrm).normalize_or_zero();
                    if !clip_segment(&mut pts, side, side.dot(v0)) {
                        ok = false;
                        break;
                    }
                }
                if ok {
                    for p in &pts {
                        let h = fnrm.dot(*p) - fd;
                        let s = h - r;
                        if s <= margin {
                            m.points.push(PointGeom { pa: *p - fnrm * r, pb: *p - fnrm * h, separation: s });
                        }
                    }
                    if !m.points.is_empty() {
                        m.normal = -fnrm;
                        return Some(m);
                    }
                }
            }
        }
        m.points.push(PointGeom { pa: g.point_a + n * r, pb: g.point_b, separation: g.distance - r });
        return Some(m);
    }
    // Deep: the core is inside the polytope. Use the face of least penetration.
    let mut best = f32::MIN;
    let mut best_face = 0;
    for (i, (fnrm, fd)) in poly.planes.iter().enumerate() {
        let s = core.iter().fold(f32::MAX, |m, p| m.min(fnrm.dot(*p))) - fd;
        if s > best {
            best = s;
            best_face = i;
        }
    }
    let (fnrm, fd) = poly.planes[best_face];
    let mut m = ManifoldGeom { normal: -fnrm, ..Default::default() };
    for p in core {
        let h = fnrm.dot(*p) - fd;
        let s = h - r;
        if s <= margin.max(best - r + 0.01) {
            m.points.push(PointGeom { pa: *p - fnrm * r, pb: *p - fnrm * h, separation: s });
        }
    }
    if m.points.is_empty() {
        return None;
    }
    Some(m)
}

/// Clip a 2-point segment against dot(n, x) <= d. Returns false if fully clipped.
fn clip_segment(pts: &mut SmallVec<[Vec3; 16]>, n: Vec3, d: f32) -> bool {
    let (a, b) = (pts[0], pts[1]);
    let (da, db) = (n.dot(a) - d, n.dot(b) - d);
    if da > 0.0 && db > 0.0 {
        return false;
    }
    if da > 0.0 {
        pts[0] = a + (b - a) * (da / (da - db));
    } else if db > 0.0 {
        pts[1] = a + (b - a) * (da / (da - db));
    }
    true
}

fn collide_mesh(sa: &Shape, pa: &Pose, mesh: &TriMesh, pm: &Pose, margin: f32, out: &mut Vec<ManifoldGeom>) {
    let world_box = sa.aabb(pa).expanded(margin);
    let local_box = world_box.transformed(&pm.inverse());
    let hull_poly = sa.hull().map(|h| WorldPoly::from_hull(h, pa));
    mesh.bvh.query_aabb(&local_box, |ti| {
        let [a, b, c] = mesh.triangle(ti);
        let (a, b, c) = (pm.transform_point(a), pm.transform_point(b), pm.transform_point(c));
        let n = pm.transform_vector(mesh.normals[ti]);
        let flags = mesh.edge_flags[ti];
        let m = match sa {
            Shape::Sphere { radius } => sphere_triangle(pa.p, *radius, a, b, c, n, flags, margin),
            Shape::Capsule { half_height, radius } => {
                let (s0, s1) = capsule_segment(*half_height, pa);
                // One-sided: ignore capsules whose core is fully behind the triangle.
                if (s0 - a).dot(n) < -radius && (s1 - a).dot(n) < -radius {
                    None
                } else {
                    let tri = WorldPoly::triangle(a, b, c);
                    core_poly(&[s0, s1], *radius, &tri, margin).filter(|m| m.normal.dot(n) < 0.3).map(|mut m| {
                        if flags == 0 {
                            snap_to_face(&mut m, n, a);
                        }
                        m
                    })
                }
            }
            _ => {
                let hp = hull_poly.as_ref().unwrap();
                if (hp.centroid - a).dot(n) < 0.0 {
                    None
                } else {
                    let tri = WorldPoly::triangle(a, b, c);
                    if flags == 0 {
                        // Fully interior triangle: only the face normal is a valid axis.
                        face_contact(&tri, 0, hp, margin).map(|mut m| {
                            m.flip();
                            m
                        })
                    } else {
                        poly_poly(hp, &tri, margin).filter(|m| m.normal.dot(n) < 0.3)
                    }
                }
            }
        };
        if let Some(mut m) = m {
            m.key = ti as u32;
            out.push(m);
        }
    });
}

/// Replace the normal with the triangle's face normal (A→B = -n) and recompute separations.
fn snap_to_face(m: &mut ManifoldGeom, n: Vec3, a: Vec3) {
    if m.normal.dot(-n) > 0.9999 {
        return;
    }
    m.normal = -n;
    for p in &mut m.points {
        // Keep the point on A; project onto the plane for B.
        let h = (p.pa - a).dot(n);
        p.pb = p.pa - n * h;
        p.separation = h;
    }
}

#[allow(clippy::too_many_arguments)]
fn sphere_triangle(c: Vec3, r: f32, a: Vec3, b: Vec3, cc: Vec3, n: Vec3, flags: u8, margin: f32) -> Option<ManifoldGeom> {
    let h = (c - a).dot(n);
    if h < 0.0 {
        return None;
    }
    let q = closest_point_triangle(c, a, b, cc);
    let d = q - c;
    let dist = d.length();
    if dist - r > margin {
        return None;
    }
    let mut normal = if dist > 1e-6 { d / dist } else { -n };
    // On an inactive edge or vertex: use the face normal to avoid bumps at seams.
    if normal.dot(-n) < 0.9999 {
        let on_inactive = {
            let bary = barycentric(q, a, b, cc);
            let mut ok = true;
            for k in 0..3 {
                // Edge k is (v_k, v_k+1), opposite vertex (k + 2) % 3.
                if bary[(k + 2) % 3] < 1e-4 && flags & (1 << k) != 0 {
                    ok = false;
                }
            }
            ok
        };
        if on_inactive {
            normal = -n;
            let mut m = ManifoldGeom { normal, ..Default::default() };
            m.points.push(PointGeom { pa: c - n * r, pb: c - n * h, separation: h - r });
            return Some(m);
        }
    }
    let mut m = ManifoldGeom { normal, ..Default::default() };
    m.points.push(PointGeom { pa: c + normal * r, pb: q, separation: dist - r });
    Some(m)
}

fn barycentric(p: Vec3, a: Vec3, b: Vec3, c: Vec3) -> [f32; 3] {
    let v0 = b - a;
    let v1 = c - a;
    let v2 = p - a;
    let d00 = v0.dot(v0);
    let d01 = v0.dot(v1);
    let d11 = v1.dot(v1);
    let d20 = v2.dot(v0);
    let d21 = v2.dot(v1);
    let denom = d00 * d11 - d01 * d01;
    if denom.abs() < 1e-20 {
        return [1.0, 0.0, 0.0];
    }
    let v = (d11 * d20 - d01 * d21) / denom;
    let w = (d00 * d21 - d01 * d20) / denom;
    [1.0 - v - w, v, w]
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Quat;

    fn manifolds(sa: &Shape, pa: Pose, sb: &Shape, pb: Pose) -> Vec<ManifoldGeom> {
        let mut out = Vec::new();
        collide(sa, &pa, sb, &pb, 0.02, &mut out);
        out
    }

    #[test]
    fn box_resting_on_box_gives_four_points() {
        let ground = Shape::cuboid(Vec3::new(10.0, 0.5, 10.0));
        let cube = Shape::cuboid(Vec3::splat(0.5));
        let m = manifolds(&cube, Pose::from_translation(Vec3::new(0.0, 0.99, 0.0)), &ground, Pose::IDENTITY);
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].points.len(), 4);
        assert!((m[0].normal - -Vec3::Y).length() < 1e-4, "{:?}", m[0].normal);
        for p in &m[0].points {
            assert!((p.separation + 0.01).abs() < 1e-4);
        }
    }

    #[test]
    fn rotated_box_edge_contact() {
        let a = Shape::cuboid(Vec3::splat(0.5));
        let q = Quat::from_rotation_z(std::f32::consts::FRAC_PI_4) * Quat::from_rotation_x(std::f32::consts::FRAC_PI_4);
        let pb = Pose::new(Vec3::new(0.0, 0.5 + 0.70, 0.0), q);
        let m = manifolds(&a, Pose::IDENTITY, &a, pb);
        assert_eq!(m.len(), 1);
        assert!(m[0].normal.y > 0.5);
        assert!(m[0].points[0].separation < 0.0);
    }

    #[test]
    fn sphere_and_capsule_on_box() {
        let ground = Shape::cuboid(Vec3::new(10.0, 0.5, 10.0));
        let s = Shape::sphere(0.5);
        let m = manifolds(&s, Pose::from_translation(Vec3::new(1.0, 0.98, 0.0)), &ground, Pose::IDENTITY);
        assert_eq!(m.len(), 1);
        assert!((m[0].points[0].separation + 0.02).abs() < 1e-4);
        let c = Shape::capsule(1.0, 0.25);
        let lying = Pose::new(Vec3::new(0.0, 0.74, 0.0), Quat::from_rotation_z(std::f32::consts::FRAC_PI_2));
        let m = manifolds(&c, lying, &ground, Pose::IDENTITY);
        assert_eq!(m[0].points.len(), 2);
        assert!((m[0].normal - -Vec3::Y).length() < 1e-4);
        // Deep sphere inside the box.
        let m = manifolds(&s, Pose::from_translation(Vec3::new(0.0, 0.3, 0.0)), &ground, Pose::IDENTITY);
        assert!((m[0].normal - -Vec3::Y).length() < 1e-4 && m[0].points[0].separation < -0.6);
    }

    #[test]
    fn box_on_trimesh_ground() {
        let v = vec![Vec3::new(-10.0, 0.0, -10.0), Vec3::new(10.0, 0.0, -10.0), Vec3::new(10.0, 0.0, 10.0), Vec3::new(-10.0, 0.0, 10.0)];
        let mesh = Shape::trimesh(v, vec![[0, 2, 1], [0, 3, 2]]);
        let cube = Shape::cuboid(Vec3::splat(0.5));
        let m = manifolds(&cube, Pose::from_translation(Vec3::new(0.3, 0.49, 0.2)), &mesh, Pose::IDENTITY);
        let total: usize = m.iter().map(|m| m.points.len()).sum();
        assert!(total >= 4, "{total}");
        for mm in &m {
            assert!((mm.normal - -Vec3::Y).length() < 1e-3, "{:?}", mm.normal);
        }
        let s = Shape::sphere(0.5);
        let m = manifolds(&s, Pose::from_translation(Vec3::new(0.0, 0.49, 0.0)), &mesh, Pose::IDENTITY);
        assert!(!m.is_empty() && (m[0].normal + Vec3::Y).length() < 1e-3);
    }
}
