//! GJK closest points between convex support mappings, and conservative-advancement shape casts.

use glam::Vec3;

/// A convex set described by its support function (in world space).
pub trait SupportMap {
    fn support(&self, dir: Vec3) -> Vec3;
}

/// A point, segment, or the vertices of a polytope, in world space.
#[derive(Clone, Copy, Debug)]
pub struct PointSet<'a>(pub &'a [Vec3]);

impl SupportMap for PointSet<'_> {
    #[inline]
    fn support(&self, dir: Vec3) -> Vec3 {
        let mut best = self.0[0];
        let mut bd = best.dot(dir);
        for v in &self.0[1..] {
            let d = v.dot(dir);
            if d > bd {
                bd = d;
                best = *v;
            }
        }
        best
    }
}

/// A support map translated by an offset.
pub struct Translated<'a, S: SupportMap>(pub &'a S, pub Vec3);

impl<S: SupportMap> SupportMap for Translated<'_, S> {
    #[inline]
    fn support(&self, dir: Vec3) -> Vec3 {
        self.0.support(dir) + self.1
    }
}

#[derive(Clone, Copy, Debug)]
struct SVert {
    w: Vec3,
    a: Vec3,
    b: Vec3,
}

#[derive(Clone, Copy, Debug)]
pub struct GjkResult {
    /// Distance between the sets (0 when overlapping).
    pub distance: f32,
    /// Closest point on A.
    pub point_a: Vec3,
    /// Closest point on B.
    pub point_b: Vec3,
    /// Unit direction from A to B (undefined when overlapping).
    pub normal: Vec3,
    pub overlap: bool,
}

/// Closest points between two convex sets. `guess` seeds the search direction (any vector).
pub fn gjk_distance<A: SupportMap, B: SupportMap>(a: &A, b: &B, guess: Vec3) -> GjkResult {
    let d0 = if guess.length_squared() > 1e-12 { guess } else { Vec3::X };
    let sa = a.support(-d0);
    let sb = b.support(d0);
    let mut simplex = [SVert { w: sa - sb, a: sa, b: sb }; 4];
    let mut lambda = [1.0f32, 0.0, 0.0, 0.0];
    let mut n = 1usize;
    let mut v = sa - sb;
    for _ in 0..64 {
        let dist2 = v.length_squared();
        if dist2 < 1e-14 {
            return overlap_result(&simplex, &lambda, n);
        }
        let dir = -v;
        let sa = a.support(dir);
        let sb = b.support(-dir);
        let w = sa - sb;
        // Converged: no significant progress along -v.
        if dist2 - v.dot(w) <= 1e-6 * dist2 + 1e-12 {
            break;
        }
        if simplex[..n].iter().any(|s| s.w.distance_squared(w) < 1e-14) {
            break;
        }
        simplex[n] = SVert { w, a: sa, b: sb };
        n += 1;
        let (nv, nn, nl) = solve_simplex(&simplex, n);
        if nn == 4 {
            return overlap_result(&nv, &nl, nn);
        }
        let new_v = nv[..nn].iter().zip(&nl[..nn]).map(|(s, l)| s.w * *l).sum::<Vec3>();
        if new_v.length_squared() >= dist2 * (1.0 - 1e-6) {
            // No progress (numerical stall): keep the previous simplex.
            break;
        }
        simplex = nv;
        lambda = nl;
        n = nn;
        v = new_v;
    }
    let pa = simplex[..n].iter().zip(&lambda[..n]).map(|(s, l)| s.a * *l).sum::<Vec3>();
    let pb = simplex[..n].iter().zip(&lambda[..n]).map(|(s, l)| s.b * *l).sum::<Vec3>();
    let d = pb - pa;
    let dist = d.length();
    if dist < 1e-7 {
        return GjkResult { distance: 0.0, point_a: pa, point_b: pb, normal: Vec3::Y, overlap: true };
    }
    GjkResult { distance: dist, point_a: pa, point_b: pb, normal: d / dist, overlap: false }
}

fn overlap_result(s: &[SVert; 4], l: &[f32; 4], n: usize) -> GjkResult {
    let pa = s[..n].iter().zip(&l[..n]).map(|(s, l)| s.a * *l).sum::<Vec3>();
    let pb = s[..n].iter().zip(&l[..n]).map(|(s, l)| s.b * *l).sum::<Vec3>();
    GjkResult { distance: 0.0, point_a: pa, point_b: pb, normal: Vec3::Y, overlap: true }
}

/// Closest point of the simplex to the origin. Returns the reduced simplex and its weights.
fn solve_simplex(s: &[SVert; 4], n: usize) -> ([SVert; 4], usize, [f32; 4]) {
    let mut out = [s[0]; 4];
    let mut l = [0.0f32; 4];
    match n {
        1 => {
            out[0] = s[0];
            l[0] = 1.0;
            (out, 1, l)
        }
        2 => {
            let (a, b) = (s[0], s[1]);
            let ab = b.w - a.w;
            let t = -a.w.dot(ab) / ab.length_squared().max(1e-30);
            if t <= 0.0 {
                out[0] = a;
                l[0] = 1.0;
                (out, 1, l)
            } else if t >= 1.0 {
                out[0] = b;
                l[0] = 1.0;
                (out, 1, l)
            } else {
                out[0] = a;
                out[1] = b;
                l[0] = 1.0 - t;
                l[1] = t;
                (out, 2, l)
            }
        }
        3 => {
            let (cnt, idx, w) = triangle_weights(s[0].w, s[1].w, s[2].w);
            for k in 0..cnt {
                out[k] = s[idx[k]];
                l[k] = w[k];
            }
            (out, cnt, l)
        }
        _ => {
            let pts = [s[0].w, s[1].w, s[2].w, s[3].w];
            let faces = [[0usize, 1, 2, 3], [0, 2, 3, 1], [0, 3, 1, 2], [1, 3, 2, 0]];
            let mut best_d = f32::MAX;
            let mut best: Option<(usize, [usize; 3], [f32; 3])> = None;
            let mut any_outside = false;
            for f in faces {
                if origin_outside_plane(pts[f[0]], pts[f[1]], pts[f[2]], pts[f[3]]) {
                    any_outside = true;
                    let (cnt, idx, w) = triangle_weights(pts[f[0]], pts[f[1]], pts[f[2]]);
                    let p = (0..cnt).map(|k| pts[f[idx[k]]] * w[k]).sum::<Vec3>();
                    let d = p.length_squared();
                    if d < best_d {
                        best_d = d;
                        let mut gi = [0usize; 3];
                        for k in 0..cnt {
                            gi[k] = f[idx[k]];
                        }
                        best = Some((cnt, gi, w));
                    }
                }
            }
            if !any_outside {
                // Origin inside: barycentric weights of the tetrahedron.
                let w = tetra_barycentric(pts);
                (*s, 4, w)
            } else {
                let (cnt, gi, w) = best.unwrap();
                for k in 0..cnt {
                    out[k] = s[gi[k]];
                    l[k] = w[k];
                }
                (out, cnt, l)
            }
        }
    }
}

fn origin_outside_plane(a: Vec3, b: Vec3, c: Vec3, d: Vec3) -> bool {
    let n = (b - a).cross(c - a);
    let sp = (-a).dot(n);
    let sd = (d - a).dot(n);
    if sd.abs() < 1e-14 {
        // Degenerate (flat) tetrahedron: treat each face as a candidate.
        return true;
    }
    sp * sd < 0.0
}

fn tetra_barycentric(p: [Vec3; 4]) -> [f32; 4] {
    let vol = |a: Vec3, b: Vec3, c: Vec3, d: Vec3| (b - a).dot((c - a).cross(d - a));
    let total = vol(p[0], p[1], p[2], p[3]);
    if total.abs() < 1e-20 {
        return [0.25; 4];
    }
    let o = Vec3::ZERO;
    let w0 = vol(o, p[1], p[2], p[3]) / total;
    let w1 = vol(p[0], o, p[2], p[3]) / total;
    let w2 = vol(p[0], p[1], o, p[3]) / total;
    [w0, w1, w2, 1.0 - w0 - w1 - w2]
}

/// Closest point on triangle abc to the origin as (count, vertex indices, weights).
fn triangle_weights(a: Vec3, b: Vec3, c: Vec3) -> (usize, [usize; 3], [f32; 3]) {
    let ab = b - a;
    let ac = c - a;
    let ap = -a;
    let d1 = ab.dot(ap);
    let d2 = ac.dot(ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return (1, [0, 0, 0], [1.0, 0.0, 0.0]);
    }
    let bp = -b;
    let d3 = ab.dot(bp);
    let d4 = ac.dot(bp);
    if d3 >= 0.0 && d4 <= d3 {
        return (1, [1, 0, 0], [1.0, 0.0, 0.0]);
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let v = d1 / (d1 - d3);
        return (2, [0, 1, 0], [1.0 - v, v, 0.0]);
    }
    let cp = -c;
    let d5 = ab.dot(cp);
    let d6 = ac.dot(cp);
    if d6 >= 0.0 && d5 <= d6 {
        return (1, [2, 0, 0], [1.0, 0.0, 0.0]);
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let w = d2 / (d2 - d6);
        return (2, [0, 2, 0], [1.0 - w, w, 0.0]);
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return (2, [1, 2, 0], [1.0 - w, w, 0.0]);
    }
    let denom = 1.0 / (va + vb + vc);
    let v = vb * denom;
    let w = vc * denom;
    (3, [0, 1, 2], [1.0 - v - w, v, w])
}

#[derive(Clone, Copy, Debug)]
pub struct CastHit {
    /// Distance travelled along the cast direction.
    pub t: f32,
    /// Contact point on B.
    pub point: Vec3,
    /// Surface normal of B at the contact (pointing toward A).
    pub normal: Vec3,
}

/// Sweep convex `a` (inflated by `ra`) along unit `dir` against `b` (inflated by `rb`).
pub fn shape_cast<A: SupportMap, B: SupportMap>(a: &A, ra: f32, b: &B, rb: f32, dir: Vec3, max_t: f32) -> Option<CastHit> {
    let margin = ra + rb;
    let mut t = 0.0f32;
    let mut guess = dir;
    for _ in 0..40 {
        let moved = Translated(a, dir * t);
        let g = gjk_distance(&moved, b, guess);
        if g.overlap {
            return if t == 0.0 { Some(CastHit { t: 0.0, point: g.point_b, normal: -dir }) } else { None };
        }
        let gap = g.distance - margin;
        let n_ba = -g.normal; // from B toward A
        if gap < 1e-3 {
            return Some(CastHit { t, point: g.point_b + n_ba * rb, normal: n_ba });
        }
        let closing = dir.dot(g.normal);
        if closing <= 1e-6 {
            return None;
        }
        t += gap / closing;
        if t > max_t {
            return None;
        }
        guess = g.point_a - g.point_b;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube(c: Vec3, h: f32) -> Vec<Vec3> {
        let mut v = Vec::new();
        for i in 0..8 {
            v.push(c + Vec3::new(if i & 1 == 0 { -h } else { h }, if i & 2 == 0 { -h } else { h }, if i & 4 == 0 { -h } else { h }));
        }
        v
    }

    #[test]
    fn separated_cubes() {
        let a = cube(Vec3::ZERO, 1.0);
        let b = cube(Vec3::new(5.0, 0.5, 0.0), 1.0);
        let r = gjk_distance(&PointSet(&a), &PointSet(&b), Vec3::X);
        assert!(!r.overlap);
        assert!((r.distance - 3.0).abs() < 1e-4, "{}", r.distance);
        assert!((r.normal - Vec3::X).length() < 1e-4);
    }

    #[test]
    fn overlapping_cubes() {
        let a = cube(Vec3::ZERO, 1.0);
        let b = cube(Vec3::new(1.5, 0.3, 0.2), 1.0);
        assert!(gjk_distance(&PointSet(&a), &PointSet(&b), Vec3::X).overlap);
    }

    #[test]
    fn point_to_segment_and_corner() {
        let seg = [Vec3::new(-1.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0)];
        let p = [Vec3::new(0.3, 2.0, 0.0)];
        let r = gjk_distance(&PointSet(&p), &PointSet(&seg), Vec3::Y);
        assert!((r.distance - 2.0).abs() < 1e-5 && (r.point_b - Vec3::new(0.3, 0.0, 0.0)).length() < 1e-5);
        let c = cube(Vec3::ZERO, 1.0);
        let q = [Vec3::new(2.0, 2.0, 2.0)];
        let r2 = gjk_distance(&PointSet(&q), &PointSet(&c), Vec3::X);
        assert!((r2.distance - 3f32.sqrt()).abs() < 1e-4);
    }

    #[test]
    fn sphere_cast_onto_cube() {
        let c = cube(Vec3::ZERO, 1.0);
        let p = [Vec3::new(0.2, 5.0, 0.1)];
        let hit = shape_cast(&PointSet(&p), 0.5, &PointSet(&c), 0.0, -Vec3::Y, 10.0).unwrap();
        assert!((hit.t - 3.5).abs() < 2e-3, "{}", hit.t);
        assert!((hit.normal - Vec3::Y).length() < 1e-3);
        assert!(shape_cast(&PointSet(&p), 0.5, &PointSet(&c), 0.0, Vec3::Y, 10.0).is_none());
    }
}
