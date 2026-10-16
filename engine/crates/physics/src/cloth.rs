//! XPBD cloth: flags, awnings, tarps and banners.
//!
//! Particles connected by stretch, shear and bend distance constraints (compliance-based,
//! Macklin et al. 2016), solved with substeps. Particles can be pinned to world points or to
//! rigid bodies, are pushed by wind with gusts and aerodynamic drag per triangle, and collide
//! with nearby rigid-body colliders (spheres, capsules, boxes and convex hulls).

use crate::body::BodyHandle;
use crate::math::Pose;
use crate::query::QueryFilter;
use crate::shape::Shape;
use crate::world::PhysicsWorld;
use glam::Vec3;

#[derive(Clone, Copy, Debug)]
struct DistanceConstraint {
    a: u32,
    b: u32,
    rest: f32,
    compliance: f32,
}

#[derive(Clone, Copy, Debug)]
pub enum Pin {
    World(Vec3),
    /// Attached to a body at a point in its frame.
    Body(BodyHandle, Vec3),
}

#[derive(Clone, Debug)]
pub struct ClothDesc {
    pub width: f32,
    pub height: f32,
    pub columns: usize,
    pub rows: usize,
    /// kg per m².
    pub areal_density: f32,
    pub stretch_compliance: f32,
    pub bend_compliance: f32,
    pub damping: f32,
    pub thickness: f32,
    pub friction: f32,
    /// Aerodynamic drag/lift coefficient.
    pub drag: f32,
}

impl Default for ClothDesc {
    fn default() -> Self {
        Self {
            width: 1.5,
            height: 1.0,
            columns: 20,
            rows: 14,
            areal_density: 0.25,
            stretch_compliance: 1e-7,
            bend_compliance: 2e-4,
            damping: 0.02,
            thickness: 0.02,
            friction: 0.4,
            drag: 1.1,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Cloth {
    pub desc: ClothDesc,
    pub positions: Vec<Vec3>,
    pub prev: Vec<Vec3>,
    pub velocities: Vec<Vec3>,
    pub inv_mass: Vec<f32>,
    pub indices: Vec<u32>,
    pub uvs: Vec<[f32; 2]>,
    pub normals: Vec<Vec3>,
    pins: Vec<(u32, Pin)>,
    constraints: Vec<DistanceConstraint>,
    lambdas: Vec<f32>,
    /// Max force before a pin tears (N); infinite = never.
    pub tear_force: f32,
    pub torn: bool,
}

impl Cloth {
    /// A rectangular sheet in the plane spanned by `right` and `down` starting at `origin` (top-left).
    pub fn sheet(desc: ClothDesc, origin: Vec3, right: Vec3, down: Vec3) -> Self {
        let (c, r) = (desc.columns.max(2), desc.rows.max(2));
        let mut positions = Vec::with_capacity(c * r);
        let mut uvs = Vec::with_capacity(c * r);
        for j in 0..r {
            for i in 0..c {
                let u = i as f32 / (c - 1) as f32;
                let v = j as f32 / (r - 1) as f32;
                positions.push(
                    origin
                        + right.normalize() * (u * desc.width)
                        + down.normalize() * (v * desc.height),
                );
                uvs.push([u, v]);
            }
        }
        let cell_area = desc.width * desc.height / ((c - 1) * (r - 1)) as f32;
        let m = desc.areal_density * cell_area;
        let inv_mass = vec![1.0 / m.max(1e-6); positions.len()];
        let idx = |i: usize, j: usize| (j * c + i) as u32;
        let mut indices = Vec::new();
        for j in 0..r - 1 {
            for i in 0..c - 1 {
                let (a, b, cc, d) = (idx(i, j), idx(i + 1, j), idx(i, j + 1), idx(i + 1, j + 1));
                indices.extend_from_slice(&[a, cc, b, b, cc, d]);
            }
        }
        let mut constraints = Vec::new();
        let mut add = |a: u32, b: u32, comp: f32, pos: &Vec<Vec3>| {
            constraints.push(DistanceConstraint {
                a,
                b,
                rest: pos[a as usize].distance(pos[b as usize]),
                compliance: comp,
            });
        };
        for j in 0..r {
            for i in 0..c {
                if i + 1 < c {
                    add(
                        idx(i, j),
                        idx(i + 1, j),
                        desc.stretch_compliance,
                        &positions,
                    );
                }
                if j + 1 < r {
                    add(
                        idx(i, j),
                        idx(i, j + 1),
                        desc.stretch_compliance,
                        &positions,
                    );
                }
                if i + 1 < c && j + 1 < r {
                    add(
                        idx(i, j),
                        idx(i + 1, j + 1),
                        desc.stretch_compliance * 10.0,
                        &positions,
                    );
                    add(
                        idx(i + 1, j),
                        idx(i, j + 1),
                        desc.stretch_compliance * 10.0,
                        &positions,
                    );
                }
                if i + 2 < c {
                    add(idx(i, j), idx(i + 2, j), desc.bend_compliance, &positions);
                }
                if j + 2 < r {
                    add(idx(i, j), idx(i, j + 2), desc.bend_compliance, &positions);
                }
            }
        }
        let n = positions.len();
        let lambdas = vec![0.0; constraints.len()];
        let mut cloth = Self {
            desc,
            prev: positions.clone(),
            velocities: vec![Vec3::ZERO; n],
            normals: vec![Vec3::Y; n],
            positions,
            inv_mass,
            indices,
            uvs,
            pins: Vec::new(),
            constraints,
            lambdas,
            tear_force: f32::INFINITY,
            torn: false,
        };
        cloth.recompute_normals();
        cloth
    }

    pub fn particle(&self, column: usize, row: usize) -> u32 {
        (row * self.desc.columns.max(2) + column) as u32
    }

    pub fn pin(&mut self, particle: u32, pin: Pin) {
        self.inv_mass[particle as usize] = 0.0;
        self.pins.push((particle, pin));
    }

    /// Pin a whole row (e.g. the top edge of a banner) to where it is now.
    pub fn pin_row_in_place(&mut self, row: usize) {
        for i in 0..self.desc.columns.max(2) {
            let p = self.particle(i, row);
            let wp = self.positions[p as usize];
            self.pin(p, Pin::World(wp));
        }
    }

    pub fn recompute_normals(&mut self) {
        for n in &mut self.normals {
            *n = Vec3::ZERO;
        }
        for t in self.indices.chunks_exact(3) {
            let (a, b, c) = (t[0] as usize, t[1] as usize, t[2] as usize);
            let n = (self.positions[b] - self.positions[a])
                .cross(self.positions[c] - self.positions[a]);
            self.normals[a] += n;
            self.normals[b] += n;
            self.normals[c] += n;
        }
        for n in &mut self.normals {
            *n = n.normalize_or(Vec3::Y);
        }
    }

    /// Advance by `dt` under gravity and `wind` (m/s), colliding with the world.
    pub fn step(&mut self, world: &PhysicsWorld, dt: f32, wind: Vec3, substeps: u32) {
        let n_sub = substeps.max(1);
        let h = dt / n_sub as f32;
        let g = world.gravity;
        // Pin targets.
        let mut pin_targets: Vec<(u32, Vec3)> = Vec::with_capacity(self.pins.len());
        for (p, pin) in &self.pins {
            let target = match pin {
                Pin::World(w) => Some(*w),
                Pin::Body(b, local) => world.body(*b).map(|bb| bb.pose.transform_point(*local)),
            };
            if let Some(t) = target {
                pin_targets.push((*p, t));
            }
        }
        // Nearby colliders once per step.
        let mut lo = Vec3::splat(f32::MAX);
        let mut hi = Vec3::splat(f32::MIN);
        for p in &self.positions {
            lo = lo.min(*p);
            hi = hi.max(*p);
        }
        let bounds = crate::aabb::Aabb::new(lo, hi).expanded(
            0.5 + self
                .velocities
                .iter()
                .fold(0.0f32, |m, v| m.max(v.length()))
                * dt,
        );
        let mut colliders: Vec<(Shape, Pose)> = Vec::new();
        world.query().overlap_aabb(
            &bounds,
            &QueryFilter {
                include_sensors: false,
                ..Default::default()
            },
            |_, c| {
                if !matches!(c.shape, Shape::TriMesh(_)) {
                    colliders.push((c.shape.clone(), c.world));
                }
            },
        );
        let thickness = self.desc.thickness;
        let drag = self.desc.drag;
        let rho = 1.225;
        for _ in 0..n_sub {
            // Aerodynamic forces per triangle.
            let mut force = vec![Vec3::ZERO; self.positions.len()];
            for t in self.indices.chunks_exact(3) {
                let (a, b, c) = (t[0] as usize, t[1] as usize, t[2] as usize);
                let nrm = (self.positions[b] - self.positions[a])
                    .cross(self.positions[c] - self.positions[a]);
                let area2 = nrm.length();
                if area2 < 1e-9 {
                    continue;
                }
                let nn = nrm / area2;
                let vel = (self.velocities[a] + self.velocities[b] + self.velocities[c]) / 3.0;
                let rel = wind - vel;
                let f = nn * (0.5 * rho * drag * 0.5 * area2 * rel.dot(nn) * rel.length());
                force[a] += f / 3.0;
                force[b] += f / 3.0;
                force[c] += f / 3.0;
            }
            for i in 0..self.positions.len() {
                self.prev[i] = self.positions[i];
                if self.inv_mass[i] == 0.0 {
                    continue;
                }
                self.velocities[i] += (g + force[i] * self.inv_mass[i]) * h;
                self.velocities[i] *= 1.0 - self.desc.damping * h * 10.0;
                self.positions[i] += self.velocities[i] * h;
            }
            for (p, t) in &pin_targets {
                self.positions[*p as usize] = *t;
            }
            for l in &mut self.lambdas {
                *l = 0.0;
            }
            let mut max_pin_force: f32 = 0.0;
            for _ in 0..2 {
                for (ci, c) in self.constraints.iter().enumerate() {
                    let (a, b) = (c.a as usize, c.b as usize);
                    let (wa, wb) = (self.inv_mass[a], self.inv_mass[b]);
                    let w = wa + wb;
                    if w == 0.0 {
                        continue;
                    }
                    let d = self.positions[b] - self.positions[a];
                    let len = d.length();
                    if len < 1e-9 {
                        continue;
                    }
                    let cval = len - c.rest;
                    let alpha = c.compliance / (h * h);
                    let dl = (-cval - alpha * self.lambdas[ci]) / (w + alpha);
                    self.lambdas[ci] += dl;
                    let corr = d / len * dl;
                    self.positions[a] -= corr * wa;
                    self.positions[b] += corr * wb;
                    if wa == 0.0 || wb == 0.0 {
                        max_pin_force = max_pin_force.max(self.lambdas[ci].abs() / (h * h));
                    }
                }
            }
            // Collisions (signed distance against each nearby shape).
            for i in 0..self.positions.len() {
                if self.inv_mass[i] == 0.0 {
                    continue;
                }
                for (shape, pose) in &colliders {
                    if let Some((dist, n)) = signed_distance(shape, pose, self.positions[i]) {
                        if dist < thickness {
                            self.positions[i] += n * (thickness - dist);
                            // Friction: damp tangential motion this substep.
                            let mv = self.positions[i] - self.prev[i];
                            let tang = mv - n * mv.dot(n);
                            self.positions[i] -= tang * self.desc.friction;
                        }
                    }
                }
            }
            for i in 0..self.positions.len() {
                if self.inv_mass[i] > 0.0 {
                    self.velocities[i] = (self.positions[i] - self.prev[i]) / h;
                }
            }
            if max_pin_force > self.tear_force && !self.pins.is_empty() {
                // Tear the most loaded side: release all pins (simple, dramatic).
                for (p, _) in self.pins.drain(..) {
                    self.inv_mass[p as usize] =
                        self.inv_mass.iter().copied().fold(0.0, f32::max).max(1.0);
                }
                self.torn = true;
            }
        }
        self.recompute_normals();
    }
}

/// Signed distance from a point to a posed shape and the outward normal (None for meshes).
pub fn signed_distance(shape: &Shape, pose: &Pose, p: Vec3) -> Option<(f32, Vec3)> {
    let lp = pose.inverse_transform_point(p);
    let (d, n) = match shape {
        Shape::Sphere { radius } => {
            let l = lp.length();
            (l - radius, if l > 1e-9 { lp / l } else { Vec3::Y })
        }
        Shape::Capsule {
            half_height,
            radius,
        } => {
            let c = Vec3::new(0.0, lp.y.clamp(-half_height, *half_height), 0.0);
            let d = lp - c;
            let l = d.length();
            (l - radius, if l > 1e-9 { d / l } else { Vec3::X })
        }
        Shape::Cuboid { half, .. } => {
            let q = lp.abs() - *half;
            let outside = q.max(Vec3::ZERO);
            let inside = q.max_element().min(0.0);
            let d = outside.length() + inside;
            let n = if outside.length() > 1e-9 {
                (outside * lp.signum()).normalize()
            } else {
                let mut n = Vec3::ZERO;
                let axis = if q.x >= q.y && q.x >= q.z {
                    0
                } else if q.y >= q.z {
                    1
                } else {
                    2
                };
                n[axis] = lp[axis].signum();
                n
            };
            (d, n)
        }
        Shape::Convex(h) => {
            let mut best = f32::MIN;
            let mut bn = Vec3::Y;
            for f in &h.faces {
                let s = f.normal.dot(lp) - f.d;
                if s > best {
                    best = s;
                    bn = f.normal;
                }
            }
            (best, bn)
        }
        Shape::TriMesh(_) => return None,
    };
    Some((d, pose.transform_vector(n)))
}
