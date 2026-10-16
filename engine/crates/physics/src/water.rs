//! Water: buoyancy volumes, rain puddles (aquaplaning) and spray/splash particles.

use crate::body::BodyHandle;
use crate::world::PhysicsWorld;
use glam::{Vec2, Vec3};

/// An axis-aligned body of water (canal, harbour, flooded underpass).
#[derive(Clone, Copy, Debug)]
pub struct WaterVolume {
    pub min: Vec3,
    /// `max.y` is the water surface.
    pub max: Vec3,
    pub density: f32,
    /// Current (m/s).
    pub flow: Vec3,
    pub linear_drag: f32,
    pub angular_drag: f32,
}

impl WaterVolume {
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self {
            min,
            max,
            density: 1000.0,
            flow: Vec3::ZERO,
            linear_drag: 3.0,
            angular_drag: 1.5,
        }
    }
    pub fn surface(&self) -> f32 {
        self.max.y
    }
    pub fn contains_xz(&self, p: Vec3) -> bool {
        p.x >= self.min.x && p.x <= self.max.x && p.z >= self.min.z && p.z <= self.max.z
    }

    /// Apply buoyancy and drag to every awake dynamic body inside. Uses 8 sample points per body
    /// (collider bounds corners, shrunk) to get submersion and the righting torque.
    pub fn apply(&self, world: &mut PhysicsWorld) {
        let g = world.gravity;
        let mut work: Vec<(BodyHandle, Vec3, Vec3)> = Vec::new();
        for (bh, b) in world.bodies() {
            if !b.is_dynamic() || b.sleeping {
                continue;
            }
            let mut lo = Vec3::splat(f32::MAX);
            let mut hi = Vec3::splat(f32::MIN);
            for c in &b.colliders {
                if let Some(col) = world.collider(*c) {
                    let a = col.aabb();
                    lo = lo.min(a.min);
                    hi = hi.max(a.max);
                }
            }
            if lo.x > hi.x
                || hi.y < self.min.y
                || lo.y > self.max.y
                || !self.contains_xz((lo + hi) * 0.5)
            {
                continue;
            }
            // Approximate displaced volume from the body's own mass assuming density ~ 0.6 water
            // for vehicles/props unless colliders say otherwise.
            let mut volume = 0.0;
            for c in &b.colliders {
                if let Some(col) = world.collider(*c) {
                    volume += col.shape.mass_properties(1.0).mass;
                }
            }
            if volume <= 0.0 {
                continue;
            }
            let pose = b.pose;
            let mut total = Vec3::ZERO;
            let mut torque = Vec3::ZERO;
            let com = b.com();
            let ext = (hi - lo) * 0.5 * 0.8;
            let center = (lo + hi) * 0.5;
            let _ = pose;
            for k in 0..8 {
                let off = Vec3::new(
                    if k & 1 == 0 { -ext.x } else { ext.x },
                    if k & 2 == 0 { -ext.y } else { ext.y },
                    if k & 4 == 0 { -ext.z } else { ext.z },
                );
                let p = center + off;
                let depth = self.surface() - p.y;
                let sub = (depth / (ext.y * 2.0 + 1e-3) + 0.5).clamp(0.0, 1.0);
                if sub <= 0.0 {
                    continue;
                }
                let f = -g * (self.density * volume / 8.0 * sub);
                total += f;
                torque += (p - com).cross(f);
            }
            if total == Vec3::ZERO {
                continue;
            }
            let fraction = total.length() / (self.density * volume * g.length());
            let rel = self.flow - b.lin_vel;
            total += rel * (self.linear_drag * b.mass * fraction);
            torque -= b.ang_vel * (self.angular_drag * b.mass * fraction);
            work.push((bh, total, torque));
        }
        for (b, f, t) in work {
            world.apply_force(b, f, None);
            world.apply_torque(b, t);
        }
    }
}

/// A grid of standing water depth over the city (rain fills dips, sun dries them).
#[derive(Clone, Debug)]
pub struct PuddleField {
    pub origin: Vec2,
    pub cell: f32,
    pub width: usize,
    pub height: usize,
    /// Depth in metres.
    pub depth: Vec<f32>,
    /// Per-cell capacity (how deep a puddle the road surface there can hold).
    pub capacity: Vec<f32>,
}

impl PuddleField {
    /// `capacity_fn(x, z)` gives the max depth at a world point (0 for crowned road centre, more at gutters/dips).
    pub fn new(origin: Vec2, size: Vec2, cell: f32, capacity_fn: impl Fn(f32, f32) -> f32) -> Self {
        let width = (size.x / cell).ceil().max(1.0) as usize;
        let height = (size.y / cell).ceil().max(1.0) as usize;
        let mut capacity = Vec::with_capacity(width * height);
        for j in 0..height {
            for i in 0..width {
                capacity.push(
                    capacity_fn(
                        origin.x + (i as f32 + 0.5) * cell,
                        origin.y + (j as f32 + 0.5) * cell,
                    )
                    .max(0.0),
                );
            }
        }
        Self {
            origin,
            cell,
            width,
            height,
            depth: vec![0.0; width * height],
            capacity,
        }
    }

    /// Rain rate in mm/h; evaporation in mm/h (higher in sun and wind).
    pub fn update(&mut self, dt: f32, rain_mm_h: f32, evaporation_mm_h: f32) {
        let add = (rain_mm_h - evaporation_mm_h) / 1000.0 / 3600.0 * dt * 4.0; // runoff gathers ~4x
        for (d, c) in self.depth.iter_mut().zip(&self.capacity) {
            *d = (*d + add * (0.2 + *c * 20.0)).clamp(0.0, *c);
        }
    }

    /// Depth at a world point (bilinear).
    pub fn depth_at(&self, x: f32, z: f32) -> f32 {
        let fx = (x - self.origin.x) / self.cell - 0.5;
        let fz = (z - self.origin.y) / self.cell - 0.5;
        if fx < -0.5 || fz < -0.5 || fx > self.width as f32 - 0.5 || fz > self.height as f32 - 0.5 {
            return 0.0;
        }
        let i0 = fx.floor().clamp(0.0, (self.width - 1) as f32) as usize;
        let j0 = fz.floor().clamp(0.0, (self.height - 1) as f32) as usize;
        let i1 = (i0 + 1).min(self.width - 1);
        let j1 = (j0 + 1).min(self.height - 1);
        let tx = (fx - i0 as f32).clamp(0.0, 1.0);
        let tz = (fz - j0 as f32).clamp(0.0, 1.0);
        let d = |i: usize, j: usize| self.depth[j * self.width + i];
        let a = d(i0, j0) + (d(i1, j0) - d(i0, j0)) * tx;
        let b = d(i0, j1) + (d(i1, j1) - d(i0, j1)) * tx;
        a + (b - a) * tz
    }

    /// Displace water where a tyre rolls through (it sprays out and the puddle thins briefly).
    pub fn disturb(&mut self, x: f32, z: f32, amount: f32) {
        let i = ((x - self.origin.x) / self.cell) as isize;
        let j = ((z - self.origin.y) / self.cell) as isize;
        if i >= 0 && j >= 0 && (i as usize) < self.width && (j as usize) < self.height {
            let k = j as usize * self.width + i as usize;
            self.depth[k] = (self.depth[k] - amount).max(0.0);
        }
    }
}

/// Grip multiplier for a tyre crossing standing water. `depth` m, `speed` m/s.
/// Partial aquaplaning starts around 60 km/h in 3 mm of water and becomes total near 90 km/h in 8 mm.
pub fn aquaplane_grip(depth: f32, speed: f32) -> f32 {
    if depth <= 0.0005 {
        return 1.0;
    }
    let mm = depth * 1000.0;
    let onset = 26.0 - mm.min(10.0) * 0.9; // m/s
    let full = onset + 9.0;
    let t = ((speed - onset) / (full - onset)).clamp(0.0, 1.0);
    let film = (mm / 8.0).min(1.0);
    1.0 - t * film * 0.92 - (mm / 40.0).min(0.1)
}

#[derive(Clone, Copy, Debug)]
pub struct WaterParticle {
    pub position: Vec3,
    pub velocity: Vec3,
    pub size: f32,
    pub life: f32,
    pub max_life: f32,
}

/// Ballistic spray: tyre spray, splashes, hydrant geysers, rain splats. Renderers draw them
/// as soft particles; `ground` gives the surface height for bouncing/settling.
#[derive(Clone, Debug, Default)]
pub struct SprayParticles {
    pub particles: Vec<WaterParticle>,
    pub max: usize,
    seed: u32,
}

impl SprayParticles {
    pub fn new(max: usize) -> Self {
        Self {
            particles: Vec::with_capacity(max),
            max,
            seed: 0x1234_5678,
        }
    }
    fn rnd(&mut self) -> f32 {
        let mut x = self.seed;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.seed = x;
        (x >> 8) as f32 / 16_777_216.0
    }
    /// Emit `count` droplets around `dir` with a cone `spread` (rad) and `speed`.
    pub fn emit(&mut self, at: Vec3, dir: Vec3, speed: f32, spread: f32, count: usize, size: f32) {
        let (u, v) = crate::math::orthonormal_basis(dir.normalize_or(Vec3::Y));
        for _ in 0..count {
            if self.particles.len() >= self.max {
                self.particles.swap_remove(0);
            }
            let a = self.rnd() * std::f32::consts::TAU;
            let r = self.rnd().sqrt() * spread;
            let d = (dir.normalize_or(Vec3::Y) + (u * a.cos() + v * a.sin()) * r.tan()).normalize();
            let s = speed * (0.6 + 0.4 * self.rnd());
            let life = 0.6 + self.rnd() * 0.8;
            let sz = size * (0.5 + self.rnd());
            self.particles.push(WaterParticle {
                position: at,
                velocity: d * s,
                size: sz,
                life,
                max_life: life,
            });
        }
    }
    pub fn update(&mut self, dt: f32, gravity: Vec3, ground: impl Fn(Vec3) -> f32) {
        for p in &mut self.particles {
            p.velocity += gravity * dt;
            p.velocity *= 1.0 - (0.8 * dt).min(0.5);
            p.position += p.velocity * dt;
            p.life -= dt;
            let gy = ground(p.position);
            if p.position.y < gy {
                p.position.y = gy;
                p.life = p.life.min(0.08);
                p.velocity *= 0.2;
            }
        }
        self.particles.retain(|p| p.life > 0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aquaplaning_curve() {
        assert_eq!(aquaplane_grip(0.0, 40.0), 1.0);
        assert!(aquaplane_grip(0.003, 10.0) > 0.85);
        assert!(aquaplane_grip(0.008, 35.0) < 0.3);
        assert!(aquaplane_grip(0.003, 30.0) < aquaplane_grip(0.003, 20.0));
    }

    #[test]
    fn puddles_fill_and_dry() {
        let mut f = PuddleField::new(Vec2::ZERO, Vec2::splat(10.0), 1.0, |x, _| {
            if x < 5.0 {
                0.006
            } else {
                0.0
            }
        });
        for _ in 0..3600 {
            f.update(1.0, 8.0, 0.5);
        }
        assert!(f.depth_at(2.0, 2.0) > 0.004);
        assert!(f.depth_at(8.0, 2.0) < 1e-6);
        for _ in 0..36000 {
            f.update(1.0, 0.0, 1.0);
        }
        assert!(f.depth_at(2.0, 2.0) < 1e-4);
    }
}
