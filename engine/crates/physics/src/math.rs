//! Small math helpers on top of `glam`: rigid poses, skew matrices, bases.

use glam::{Mat3, Mat4, Quat, Vec3};

/// A rigid transform: rotation `q` followed by translation `p`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    pub p: Vec3,
    pub q: Quat,
}

impl Default for Pose {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Pose {
    pub const IDENTITY: Pose = Pose { p: Vec3::ZERO, q: Quat::IDENTITY };

    #[inline]
    pub fn new(p: Vec3, q: Quat) -> Self {
        Self { p, q }
    }
    #[inline]
    pub fn from_translation(p: Vec3) -> Self {
        Self { p, q: Quat::IDENTITY }
    }
    #[inline]
    pub fn transform_point(&self, v: Vec3) -> Vec3 {
        self.p + self.q * v
    }
    #[inline]
    pub fn transform_vector(&self, v: Vec3) -> Vec3 {
        self.q * v
    }
    #[inline]
    pub fn inverse_transform_point(&self, v: Vec3) -> Vec3 {
        self.q.conjugate() * (v - self.p)
    }
    #[inline]
    pub fn inverse_transform_vector(&self, v: Vec3) -> Vec3 {
        self.q.conjugate() * v
    }
    /// `self * other`: apply `other` first, then `self`.
    #[inline]
    pub fn mul(&self, other: &Pose) -> Pose {
        Pose { p: self.p + self.q * other.p, q: (self.q * other.q).normalize() }
    }
    #[inline]
    pub fn inverse(&self) -> Pose {
        let qi = self.q.conjugate();
        Pose { p: qi * -self.p, q: qi }
    }
    /// Pose of `other` expressed in this pose's frame (`self⁻¹ * other`).
    #[inline]
    pub fn inv_mul(&self, other: &Pose) -> Pose {
        let qi = self.q.conjugate();
        Pose { p: qi * (other.p - self.p), q: (qi * other.q).normalize() }
    }
    pub fn to_mat4(&self) -> Mat4 {
        Mat4::from_rotation_translation(self.q, self.p)
    }
}

/// Cross-product matrix: `skew(a) * b == a.cross(b)`.
#[inline]
pub fn skew(v: Vec3) -> Mat3 {
    Mat3::from_cols(Vec3::new(0.0, v.z, -v.y), Vec3::new(-v.z, 0.0, v.x), Vec3::new(v.y, -v.x, 0.0))
}

/// Two unit vectors that complete `n` (unit) to a right-handed orthonormal basis.
/// Duff et al. 2017, "Building an Orthonormal Basis, Revisited".
#[inline]
pub fn orthonormal_basis(n: Vec3) -> (Vec3, Vec3) {
    let sign = 1.0f32.copysign(n.z);
    let a = -1.0 / (sign + n.z);
    let b = n.x * n.y * a;
    (
        Vec3::new(1.0 + sign * n.x * n.x * a, sign * b, -sign * n.x),
        Vec3::new(b, sign + n.y * n.y * a, -n.y),
    )
}

/// Integrate an orientation by angular velocity `w` over `h` seconds.
#[inline]
pub fn integrate_rotation(q: Quat, w: Vec3, h: f32) -> Quat {
    let dq = Quat::from_xyzw(w.x, w.y, w.z, 0.0) * q;
    let r = Quat::from_xyzw(q.x + 0.5 * h * dq.x, q.y + 0.5 * h * dq.y, q.z + 0.5 * h * dq.z, q.w + 0.5 * h * dq.w);
    r.normalize()
}

/// Rotation `R I R^T` of a symmetric tensor.
#[inline]
pub fn rotate_tensor(r: Mat3, i: Mat3) -> Mat3 {
    r * i * r.transpose()
}

/// Inverse of a symmetric 3x3 matrix; returns zero for singular matrices.
#[inline]
pub fn inverse_or_zero(m: Mat3) -> Mat3 {
    let det = m.determinant();
    if det.abs() < 1e-12 {
        Mat3::ZERO
    } else {
        m.inverse()
    }
}

/// Closest points between segments `p1-q1` and `p2-q2` (Ericson 5.1.9).
/// Returns (s, t, c1, c2) with c1 = p1 + s (q1 - p1), c2 = p2 + t (q2 - p2).
pub fn closest_segment_segment(p1: Vec3, q1: Vec3, p2: Vec3, q2: Vec3) -> (f32, f32, Vec3, Vec3) {
    let d1 = q1 - p1;
    let d2 = q2 - p2;
    let r = p1 - p2;
    let a = d1.dot(d1);
    let e = d2.dot(d2);
    let f = d2.dot(r);
    const EPS: f32 = 1e-10;
    let (s, t);
    if a <= EPS && e <= EPS {
        return (0.0, 0.0, p1, p2);
    }
    if a <= EPS {
        s = 0.0;
        t = (f / e).clamp(0.0, 1.0);
    } else {
        let c = d1.dot(r);
        if e <= EPS {
            t = 0.0;
            s = (-c / a).clamp(0.0, 1.0);
        } else {
            let b = d1.dot(d2);
            let denom = a * e - b * b;
            let mut s0 = if denom > EPS { ((b * f - c * e) / denom).clamp(0.0, 1.0) } else { 0.0 };
            let mut t0 = (b * s0 + f) / e;
            if t0 < 0.0 {
                t0 = 0.0;
                s0 = (-c / a).clamp(0.0, 1.0);
            } else if t0 > 1.0 {
                t0 = 1.0;
                s0 = ((b - c) / a).clamp(0.0, 1.0);
            }
            s = s0;
            t = t0;
        }
    }
    (s, t, p1 + d1 * s, p2 + d2 * t)
}

/// Closest point to `p` on segment `a-b`, and its parameter.
#[inline]
pub fn closest_point_segment(p: Vec3, a: Vec3, b: Vec3) -> (Vec3, f32) {
    let ab = b - a;
    let l2 = ab.length_squared();
    if l2 < 1e-12 {
        return (a, 0.0);
    }
    let t = ((p - a).dot(ab) / l2).clamp(0.0, 1.0);
    (a + ab * t, t)
}

/// Closest point to `p` on triangle `abc` (Ericson 5.1.5).
pub fn closest_point_triangle(p: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Vec3 {
    let ab = b - a;
    let ac = c - a;
    let ap = p - a;
    let d1 = ab.dot(ap);
    let d2 = ac.dot(ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }
    let bp = p - b;
    let d3 = ab.dot(bp);
    let d4 = ac.dot(bp);
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let v = d1 / (d1 - d3);
        return a + ab * v;
    }
    let cp = p - c;
    let d5 = ab.dot(cp);
    let d6 = ac.dot(cp);
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let w = d2 / (d2 - d6);
        return a + ac * w;
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return b + (c - b) * w;
    }
    let denom = 1.0 / (va + vb + vc);
    let v = vb * denom;
    let w = vc * denom;
    a + ab * v + ac * w
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basis_is_orthonormal() {
        for n in [Vec3::X, Vec3::Y, Vec3::Z, -Vec3::Z, Vec3::new(1.0, 2.0, -3.0).normalize()] {
            let (a, b) = orthonormal_basis(n);
            assert!(a.dot(n).abs() < 1e-5 && b.dot(n).abs() < 1e-5 && a.dot(b).abs() < 1e-5);
            assert!((a.cross(b) - n).length() < 1e-4);
        }
    }

    #[test]
    fn pose_inverse_roundtrip() {
        let p = Pose::new(Vec3::new(1.0, 2.0, 3.0), Quat::from_rotation_y(0.7) * Quat::from_rotation_x(0.3));
        let v = Vec3::new(-2.0, 0.5, 4.0);
        assert!((p.inverse_transform_point(p.transform_point(v)) - v).length() < 1e-5);
        let id = p.mul(&p.inverse());
        assert!(id.p.length() < 1e-5);
    }
}
