//! Axis-aligned bounding boxes.

use crate::math::Pose;
use glam::{Mat3, Vec3};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub const EMPTY: Aabb = Aabb { min: Vec3::splat(f32::MAX), max: Vec3::splat(-f32::MAX) };

    #[inline]
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }
    #[inline]
    pub fn from_center_half(c: Vec3, h: Vec3) -> Self {
        Self { min: c - h, max: c + h }
    }
    pub fn from_points(pts: impl IntoIterator<Item = Vec3>) -> Self {
        let mut a = Self::EMPTY;
        for p in pts {
            a.min = a.min.min(p);
            a.max = a.max.max(p);
        }
        a
    }
    #[inline]
    pub fn union(&self, o: &Aabb) -> Aabb {
        Aabb { min: self.min.min(o.min), max: self.max.max(o.max) }
    }
    #[inline]
    pub fn grow(&mut self, p: Vec3) {
        self.min = self.min.min(p);
        self.max = self.max.max(p);
    }
    #[inline]
    pub fn expanded(&self, m: f32) -> Aabb {
        Aabb { min: self.min - Vec3::splat(m), max: self.max + Vec3::splat(m) }
    }
    /// Extend in the direction of a displacement (for swept/predicted bounds).
    #[inline]
    pub fn swept(&self, d: Vec3) -> Aabb {
        Aabb { min: self.min + d.min(Vec3::ZERO), max: self.max + d.max(Vec3::ZERO) }
    }
    #[inline]
    pub fn contains(&self, o: &Aabb) -> bool {
        self.min.cmple(o.min).all() && self.max.cmpge(o.max).all()
    }
    #[inline]
    pub fn contains_point(&self, p: Vec3) -> bool {
        self.min.cmple(p).all() && self.max.cmpge(p).all()
    }
    #[inline]
    pub fn overlaps(&self, o: &Aabb) -> bool {
        self.min.cmple(o.max).all() && self.max.cmpge(o.min).all()
    }
    #[inline]
    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }
    #[inline]
    pub fn half_extents(&self) -> Vec3 {
        (self.max - self.min) * 0.5
    }
    /// Surface area (SAH cost metric).
    #[inline]
    pub fn area(&self) -> f32 {
        let d = self.max - self.min;
        2.0 * (d.x * d.y + d.y * d.z + d.z * d.x)
    }
    /// World bounds of a local box under a pose.
    pub fn transformed(&self, pose: &Pose) -> Aabb {
        let c = pose.transform_point(self.center());
        let m = Mat3::from_quat(pose.q);
        let h = self.half_extents();
        let e = Vec3::new(
            m.x_axis.x.abs() * h.x + m.y_axis.x.abs() * h.y + m.z_axis.x.abs() * h.z,
            m.x_axis.y.abs() * h.x + m.y_axis.y.abs() * h.y + m.z_axis.y.abs() * h.z,
            m.x_axis.z.abs() * h.x + m.y_axis.z.abs() * h.y + m.z_axis.z.abs() * h.z,
        );
        Aabb::from_center_half(c, e)
    }
    /// Slab test. Returns entry distance in [0, max_t] or None.
    #[inline]
    pub fn ray_hit(&self, origin: Vec3, inv_dir: Vec3, max_t: f32) -> Option<f32> {
        let t1 = (self.min - origin) * inv_dir;
        let t2 = (self.max - origin) * inv_dir;
        let tmin = t1.min(t2).max_element().max(0.0);
        let tmax = t1.max(t2).min_element().min(max_t);
        if tmin <= tmax {
            Some(tmin)
        } else {
            None
        }
    }
}
