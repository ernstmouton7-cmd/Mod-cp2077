//! Cascaded shadow map fitting and frustum culling helpers.

use crate::types::mat;
use glam::{Mat4, Vec3, Vec4};

/// Depth range of every cascade's orthographic projection (must match `SHADOW_DEPTH_RANGE` in WGSL).
pub(crate) const SHADOW_DEPTH_RANGE: f32 = 800.0;

#[derive(Clone, Copy, Debug)]
pub(crate) struct Frustum {
    planes: [Vec4; 6],
    count: usize,
}

impl Frustum {
    /// Extracts planes from a view-projection matrix with 0..1 depth.
    /// `depth_planes`: also add near/far (orthographic shadow views).
    pub fn from_matrix(m: Mat4, depth_planes: bool) -> Self {
        let r0 = m.row(0);
        let r1 = m.row(1);
        let r2 = m.row(2);
        let r3 = m.row(3);
        let mut planes = [r3 + r0, r3 - r0, r3 + r1, r3 - r1, r2, r3 - r2];
        for p in planes.iter_mut() {
            let l = p.truncate().length();
            if l > 0.0 {
                *p /= l;
            }
        }
        Self {
            planes,
            count: if depth_planes { 6 } else { 4 },
        }
    }

    pub fn aabb(&self, lo: Vec3, hi: Vec3) -> bool {
        for p in &self.planes[..self.count] {
            let n = p.truncate();
            let v = Vec3::new(
                if n.x >= 0.0 { hi.x } else { lo.x },
                if n.y >= 0.0 { hi.y } else { lo.y },
                if n.z >= 0.0 { hi.z } else { lo.z },
            );
            if n.dot(v) + p.w < 0.0 {
                return false;
            }
        }
        true
    }

    pub fn sphere(&self, c: Vec3, r: f32) -> bool {
        for p in &self.planes[..self.count] {
            if p.truncate().dot(c) + p.w < -r {
                return false;
            }
        }
        true
    }
}

pub(crate) struct Cascade {
    pub view_proj: Mat4,
    pub split_far: f32,
    pub texel: f32,
}

/// Practical split scheme with rotation-invariant bounding spheres and texel snapping.
pub(crate) fn fit_cascades(
    cam_pos: Vec3,
    cam_fwd: Vec3,
    fov_y: f32,
    aspect: f32,
    near: f32,
    distance: f32,
    count: u32,
    resolution: u32,
    light_dir: Vec3,
) -> Vec<Cascade> {
    let count = count.clamp(1, 4);
    let lambda = 0.78;
    let mut splits = Vec::with_capacity(count as usize + 1);
    let n0 = near.max(0.05);
    splits.push(n0);
    for i in 1..=count {
        let f = i as f32 / count as f32;
        let log = n0 * (distance / n0).powf(f);
        let lin = n0 + (distance - n0) * f;
        splits.push(lambda * log + (1.0 - lambda) * lin);
    }
    let k = (1.0 + aspect * aspect).sqrt() * (fov_y * 0.5).tan();
    let l = light_dir.normalize();
    let up = if l.y.abs() > 0.99 { Vec3::Z } else { Vec3::Y };
    let rot = mat::look_to(Vec3::ZERO, -l, up);
    let mut out = Vec::new();
    for i in 0..count as usize {
        // The first cascade starts at the camera so near geometry is always covered.
        let n = if i == 0 { 0.0 } else { splits[i] };
        let f = splits[i + 1];
        let mut c = 0.5 * (n + f) * (1.0 + k * k);
        let r = if c >= f {
            c = f;
            f * k
        } else {
            ((f - c) * (f - c) + f * f * k * k).sqrt()
        };
        // Quantise the radius to reduce shimmering when the FOV animates.
        let r = (r * 16.0).ceil() / 16.0;
        let center = cam_pos + cam_fwd * c;
        let texel = 2.0 * r / resolution as f32;
        let ls = rot.transform_point3(center);
        let snapped = Vec3::new(
            (ls.x / texel).floor() * texel,
            (ls.y / texel).floor() * texel,
            ls.z,
        );
        let half = SHADOW_DEPTH_RANGE * 0.5;
        let view = Mat4::from_translation(-Vec3::new(snapped.x, snapped.y, snapped.z + half)) * rot;
        let proj = mat::orthographic(-r, r, -r, r, 0.0, SHADOW_DEPTH_RANGE);
        out.push(Cascade {
            view_proj: proj * view,
            split_far: f,
            texel,
        });
    }
    out
}
