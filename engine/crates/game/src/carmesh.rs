//! Procedural car models: a lofted body (cross-sections along the length with hood, windshield,
//! roof, rear window and deck), wheel arches, tumblehome and rounded ends, split into the parts
//! the damage model detaches (bumpers, hood, trunk, doors) plus glass and lamps. Wheels are
//! separate tyre + rim meshes. All in the chassis frame (+X right, +Y up, -Z forward, origin
//! on the ground at mid-wheelbase).

use glam::{Vec2, Vec3};
use gridlock_physics::deform::{DeformMesh, PartDesc, PartKind, PartMount};
use gridlock_physics::vehicle::presets::VehicleClass;
use gridlock_physics::VehicleDesc;

#[derive(Clone, Debug, Default)]
pub struct MeshData {
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub uvs: Vec<Vec2>,
    pub indices: Vec<u32>,
}

impl MeshData {
    pub fn compute_normals(&mut self) {
        self.normals = vec![Vec3::ZERO; self.positions.len()];
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
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LampKind {
    Head,
    Tail,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Paint {
    /// Body colour clear-coat.
    Body,
    /// Black plastic trim (bumper lower, mirrors).
    Trim,
    Glass,
    HeadLamp,
    TailLamp,
}

#[derive(Clone, Debug)]
pub struct CarPart {
    pub name: String,
    pub kind: PartKind,
    pub paint: Paint,
    pub mesh: MeshData,
    pub mount: PartMount,
    pub mass: f32,
    pub strength: f32,
    /// For lamps: which side (+1 right, -1 left).
    pub side: f32,
}

#[derive(Clone, Debug)]
pub struct CarModel {
    /// The shell that stays on the chassis.
    pub body: MeshData,
    pub parts: Vec<CarPart>,
    pub tyre: MeshData,
    pub rim: MeshData,
    /// Wheel radius and width used for the wheel meshes.
    pub wheel_radius: f32,
    pub wheel_width: f32,
}

#[derive(Clone, Copy, Debug)]
struct Style {
    /// Fractions of the length from the nose (0) to the tail (1).
    windshield_base: f32,
    roof_start: f32,
    roof_end: f32,
    rear_glass_end: f32,
    /// Heights as fractions of total height.
    nose: f32,
    belt: f32,
    deck: f32,
    /// Greenhouse half-width as a fraction of body half-width.
    tumblehome: f32,
    boxy: bool,
}

fn style_for(class: VehicleClass) -> Style {
    let sedan = Style {
        windshield_base: 0.3,
        roof_start: 0.43,
        roof_end: 0.7,
        rear_glass_end: 0.82,
        nose: 0.48,
        belt: 0.6,
        deck: 0.66,
        tumblehome: 0.8,
        boxy: false,
    };
    match class {
        VehicleClass::Compact => Style {
            windshield_base: 0.25,
            roof_start: 0.4,
            roof_end: 0.86,
            rear_glass_end: 0.97,
            nose: 0.5,
            belt: 0.6,
            deck: 0.62,
            ..sedan
        },
        VehicleClass::Sports => Style {
            windshield_base: 0.38,
            roof_start: 0.52,
            roof_end: 0.7,
            rear_glass_end: 0.9,
            nose: 0.38,
            belt: 0.58,
            deck: 0.66,
            tumblehome: 0.74,
            boxy: false,
        },
        VehicleClass::Muscle => Style {
            windshield_base: 0.36,
            roof_start: 0.48,
            roof_end: 0.68,
            rear_glass_end: 0.82,
            nose: 0.52,
            belt: 0.64,
            deck: 0.66,
            ..sedan
        },
        VehicleClass::Suv => Style {
            windshield_base: 0.26,
            roof_start: 0.37,
            roof_end: 0.92,
            rear_glass_end: 0.99,
            nose: 0.55,
            belt: 0.6,
            deck: 0.62,
            tumblehome: 0.86,
            boxy: false,
        },
        VehicleClass::Van | VehicleClass::Ambulance => Style {
            windshield_base: 0.08,
            roof_start: 0.24,
            roof_end: 0.995,
            rear_glass_end: 1.0,
            nose: 0.5,
            belt: 0.55,
            deck: 0.55,
            tumblehome: 0.95,
            boxy: true,
        },
        VehicleClass::Truck => Style {
            windshield_base: 0.06,
            roof_start: 0.16,
            roof_end: 0.995,
            rear_glass_end: 1.0,
            nose: 0.42,
            belt: 0.5,
            deck: 0.5,
            tumblehome: 0.98,
            boxy: true,
        },
        VehicleClass::Bus => Style {
            windshield_base: 0.01,
            roof_start: 0.04,
            roof_end: 0.995,
            rear_glass_end: 1.0,
            nose: 0.3,
            belt: 0.36,
            deck: 0.36,
            tumblehome: 0.99,
            boxy: true,
        },
        _ => sedan,
    }
}

fn smooth(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Build a car model sized to a physics vehicle description.
pub fn build_car(class: VehicleClass, desc: &VehicleDesc) -> CarModel {
    let st = style_for(class);
    let len = desc.size.x;
    let width = desc.size.y;
    let height = desc.size.z;
    let hw = width * 0.5;
    let clearance = if st.boxy { 0.28 } else { 0.16 };
    let wheel_r = desc.wheels.first().map(|w| w.radius).unwrap_or(0.32);
    let axles: Vec<f32> = {
        let mut zs: Vec<f32> = desc.wheels.iter().map(|w| w.hardpoint.z).collect();
        zs.sort_by(|a, b| a.total_cmp(b));
        zs.dedup_by(|a, b| (*a - *b).abs() < 0.05);
        zs
    };
    // Station profile.
    let stations = 56usize;
    let ring = 26usize; // points around the half-ring from bottom centre to roof centre
    let z_of = |k: usize| -len * 0.5 + len * (k as f32 / (stations - 1) as f32);
    let mut grid: Vec<Vec<Vec3>> = Vec::with_capacity(stations);
    for k in 0..stations {
        let z = z_of(k);
        let f = (z + len * 0.5) / len; // 0 nose .. 1 tail
                                       // Plan-view rounding of the ends.
        let end = (f.min(1.0 - f) / 0.07).clamp(0.0, 1.0);
        let round = if st.boxy {
            0.92 + 0.08 * end.sqrt()
        } else {
            0.62 + 0.38 * (1.0 - (1.0 - end).powi(2)).sqrt()
        };
        let w = hw * round;
        // Side-view heights.
        let nose_h = height * st.nose;
        let belt_h = height * st.belt;
        let deck_h = height * st.deck;
        let roof_h = height;
        let shoulder = if f < st.windshield_base {
            nose_h + (belt_h - nose_h) * smooth(0.0, st.windshield_base, f).sqrt()
        } else if f < st.rear_glass_end {
            belt_h + (deck_h - belt_h) * smooth(st.roof_end, st.rear_glass_end, f)
        } else {
            deck_h * (1.0 - 0.15 * smooth(st.rear_glass_end, 1.0, f))
        };
        let top = if f < st.windshield_base {
            shoulder
        } else if f < st.roof_start {
            shoulder + (roof_h - shoulder) * smooth(st.windshield_base, st.roof_start, f).powf(0.85)
        } else if f < st.roof_end {
            roof_h
                * (1.0
                    - 0.02
                        * ((f - st.roof_start) / (st.roof_end - st.roof_start) - 0.5).powi(2)
                        * 4.0)
        } else if f < st.rear_glass_end {
            roof_h + (shoulder - roof_h) * smooth(st.roof_end, st.rear_glass_end, f)
        } else {
            shoulder
        };
        // Bottom, lifted at the ends and in the wheel arches.
        let mut bottom = clearance + 0.12 * (1.0 - end);
        let mut arch = 0.0f32;
        for az in &axles {
            let dz = (z - az).abs();
            let ar = wheel_r * 1.12;
            if dz < ar {
                arch = arch.max(wheel_r + (ar * ar - dz * dz).sqrt() * 0.92);
            }
        }
        let gh_w = w * st.tumblehome;
        let mut pts = Vec::with_capacity(ring);
        for r in 0..ring {
            let t = r as f32 / (ring - 1) as f32;
            // 0..0.12 underside, 0.12..0.55 side, 0.55..0.62 shoulder, 0.62..1 greenhouse/roof.
            let p = if t < 0.12 {
                let u = t / 0.12;
                Vec3::new(w * 0.94 * u, bottom, z)
            } else if t < 0.55 {
                let u = (t - 0.12) / 0.43;
                // Side bulges slightly at mid-height.
                let y = bottom + (shoulder - bottom) * u;
                let bulge = 1.0 + 0.04 * (u * std::f32::consts::PI).sin();
                Vec3::new(
                    (w * (0.94 + 0.06 * (u * 3.0).min(1.0))) * bulge.min(1.04) / 1.04,
                    y,
                    z,
                )
            } else if t < 0.62 {
                let u = (t - 0.55) / 0.07;
                let y = shoulder + (top - shoulder).min(0.06) * u;
                Vec3::new(w + (gh_w - w) * u * 0.3, y, z)
            } else {
                let u = (t - 0.62) / 0.38;
                if top - shoulder < 0.05 {
                    // Hood or deck: gently crowned surface.
                    Vec3::new(
                        w * 0.97 * (1.0 - u),
                        shoulder + 0.03 * (1.0 - (1.0 - u).powi(2)) + 0.02,
                        z,
                    )
                } else {
                    let a = u * std::f32::consts::FRAC_PI_2;
                    let x = gh_w * (1.0 - u.powf(1.6)) * (0.97 + 0.03 * a.cos());
                    let y = shoulder
                        + 0.06
                        + (top - shoulder - 0.06) * (1.0 - (1.0 - u).powf(4.0)).min(1.0);
                    Vec3::new(x, y, z)
                }
            };
            pts.push(p);
        }
        // Raise the side in the arches.
        if arch > 0.0 {
            for p in pts.iter_mut() {
                if p.y < arch && p.x > w * 0.6 {
                    p.y = arch.min(shoulder - 0.05).max(p.y);
                }
            }
            bottom = bottom.max(arch);
            let _ = bottom;
        }
        grid.push(pts);
    }
    // Full ring: mirror to the left side.
    let mut all = MeshData::default();
    let ring_full = ring * 2 - 1;
    for (k, pts) in grid.iter().enumerate() {
        for r in 0..ring_full {
            let (src, sign) = if r < ring {
                (ring - 1 - r, -1.0)
            } else {
                (r - ring + 1, 1.0)
            };
            let p = pts[src];
            all.positions.push(Vec3::new(p.x * sign, p.y, p.z));
            all.uvs.push(Vec2::new(
                r as f32 / (ring_full - 1) as f32,
                k as f32 / (stations - 1) as f32,
            ));
        }
    }
    let idx = |k: usize, r: usize| (k * ring_full + r) as u32;
    for k in 0..stations - 1 {
        for r in 0..ring_full - 1 {
            let (a, b, c, d) = (idx(k, r), idx(k, r + 1), idx(k + 1, r), idx(k + 1, r + 1));
            // Outward winding (ring: roof centre -> left side -> bottom -> right side -> roof; stations nose -> tail).
            all.indices.extend_from_slice(&[a, b, c, b, d, c]);
        }
    }
    // End caps (nose and tail) as fans.
    for (k, flip) in [(0usize, true), (stations - 1, false)] {
        let center: Vec3 = (0..ring_full)
            .map(|r| all.positions[idx(k, r) as usize])
            .sum::<Vec3>()
            / ring_full as f32;
        let ci = all.positions.len() as u32;
        all.positions.push(center);
        all.uvs
            .push(Vec2::new(0.5, k as f32 / (stations - 1) as f32));
        for r in 0..ring_full - 1 {
            let (a, b) = (idx(k, r), idx(k, r + 1));
            if flip {
                all.indices.extend_from_slice(&[ci, b, a]);
            } else {
                all.indices.extend_from_slice(&[ci, a, b]);
            }
        }
    }
    all.compute_normals();

    // Classify triangles into parts by centroid.
    let ws_z = -len * 0.5 + len * st.windshield_base;
    let rg_z = -len * 0.5 + len * st.rear_glass_end;
    let roof_s = -len * 0.5 + len * st.roof_start;
    let roof_e = -len * 0.5 + len * st.roof_end;
    let belt_y = height * st.belt;
    let front_axle = axles.first().copied().unwrap_or(-len * 0.3);
    let rear_axle = axles.last().copied().unwrap_or(len * 0.3);
    let door_split = (front_axle + rear_axle) * 0.5 + 0.1;
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    enum Slot {
        Body,
        FrontBumper,
        RearBumper,
        Hood,
        Trunk,
        Door(u8),
        Glass,
        Head(i8),
        Tail(i8),
    }
    let classify = |c: Vec3, n: Vec3| -> Slot {
        let f = (c.z + len * 0.5) / len;
        let side = if c.x >= 0.0 { 1i8 } else { -1i8 };
        if f < 0.035
            && c.y > height * st.nose * 0.55
            && c.y < height * st.nose * 0.95
            && c.x.abs() > hw * 0.45
            && n.z < -0.3
        {
            return Slot::Head(side);
        }
        if f > 0.97
            && c.y > height * st.deck * 0.6
            && c.y < height * st.deck * 0.98
            && c.x.abs() > hw * 0.5
            && n.z > 0.3
        {
            return Slot::Tail(side);
        }
        if c.y < height * 0.42 && f < 0.08 {
            return Slot::FrontBumper;
        }
        if c.y < height * 0.42 && f > 0.93 {
            return Slot::RearBumper;
        }
        let glass_zone = c.y > belt_y + 0.05 && c.z > ws_z - 0.05 && c.z < rg_z + 0.02;
        if glass_zone && !(st.boxy && c.y > height * 0.96) {
            // Roof panel stays body.
            let roof = c.y > height * 0.97 && c.z > roof_s && c.z < roof_e;
            if !roof {
                return Slot::Glass;
            }
        }
        if !st.boxy && n.y > 0.5 && c.z < ws_z && f > 0.06 {
            return Slot::Hood;
        }
        if !st.boxy && n.y > 0.5 && c.z > rg_z && f < 0.95 {
            return Slot::Trunk;
        }
        if n.x.abs() > 0.6
            && c.y > height * 0.2
            && c.y < belt_y + 0.04
            && c.z > front_axle + wheel_r * 1.15
            && c.z < rear_axle - wheel_r * 1.15
        {
            let front = c.z < door_split;
            let d = match (side > 0, front) {
                (false, true) => 0,
                (true, true) => 1,
                (false, false) => 2,
                (true, false) => 3,
            };
            if !(class == VehicleClass::Sports && !front) {
                return Slot::Door(d);
            }
        }
        Slot::Body
    };
    let mut buckets: Vec<(Slot, MeshData, Vec<i32>)> = Vec::new();
    for t in all.indices.chunks_exact(3) {
        let ps = [
            all.positions[t[0] as usize],
            all.positions[t[1] as usize],
            all.positions[t[2] as usize],
        ];
        let c = (ps[0] + ps[1] + ps[2]) / 3.0;
        let n = (ps[1] - ps[0]).cross(ps[2] - ps[0]).normalize_or(Vec3::Y);
        let slot = classify(c, n);
        let bi = match buckets.iter().position(|b| b.0 == slot) {
            Some(i) => i,
            None => {
                buckets.push((slot, MeshData::default(), vec![-1; all.positions.len()]));
                buckets.len() - 1
            }
        };
        let (_, mesh, remap) = &mut buckets[bi];
        for &v in t {
            let vi = v as usize;
            if remap[vi] < 0 {
                remap[vi] = mesh.positions.len() as i32;
                mesh.positions.push(all.positions[vi]);
                mesh.uvs.push(all.uvs[vi]);
            }
            mesh.indices.push(remap[vi] as u32);
        }
    }
    let mut body = MeshData::default();
    let mut parts = Vec::new();
    let mass = desc.mass;
    for (slot, mut mesh, _) in buckets {
        mesh.compute_normals();
        let part =
            |name: &str, kind, paint, mount, m: f32, s: f32, side: f32, mesh: MeshData| CarPart {
                name: name.into(),
                kind,
                paint,
                mesh,
                mount,
                mass: m,
                strength: s,
                side,
            };
        match slot {
            Slot::Body => body = mesh,
            Slot::FrontBumper => parts.push(part(
                "front bumper",
                PartKind::FrontBumper,
                Paint::Body,
                PartMount::Fixed,
                mass * 0.006,
                9_000.0,
                0.0,
                mesh,
            )),
            Slot::RearBumper => parts.push(part(
                "rear bumper",
                PartKind::RearBumper,
                Paint::Body,
                PartMount::Fixed,
                mass * 0.006,
                9_000.0,
                0.0,
                mesh,
            )),
            Slot::Hood => {
                let pivot = Vec3::new(0.0, height * st.belt, ws_z);
                parts.push(part(
                    "hood",
                    PartKind::Hood,
                    Paint::Body,
                    PartMount::Hinge {
                        pivot,
                        axis: Vec3::X,
                        lower: -1.1,
                        upper: 0.0,
                    },
                    mass * 0.01,
                    15_000.0,
                    0.0,
                    mesh,
                ));
            }
            Slot::Trunk => {
                let pivot = Vec3::new(0.0, height * st.deck, rg_z);
                parts.push(part(
                    "trunk",
                    PartKind::Trunk,
                    Paint::Body,
                    PartMount::Hinge {
                        pivot,
                        axis: Vec3::X,
                        lower: 0.0,
                        upper: 1.2,
                    },
                    mass * 0.008,
                    13_000.0,
                    0.0,
                    mesh,
                ));
            }
            Slot::Door(d) => {
                let right = d % 2 == 1;
                let front = d < 2;
                let (lo, hi) = (front_axle + wheel_r * 1.15, rear_axle - wheel_r * 1.15);
                let hinge_z = if front { lo } else { door_split };
                let x = if right { hw * 0.97 } else { -hw * 0.97 };
                let (lower, upper) = if right { (0.0, 1.25) } else { (-1.25, 0.0) };
                let _ = hi;
                let name = format!(
                    "{} {} door",
                    if front { "front" } else { "rear" },
                    if right { "right" } else { "left" }
                );
                parts.push(part(
                    &name,
                    PartKind::Door,
                    Paint::Body,
                    PartMount::Hinge {
                        pivot: Vec3::new(x, belt_y * 0.6, hinge_z),
                        axis: Vec3::Y,
                        lower,
                        upper,
                    },
                    mass * 0.016,
                    16_000.0,
                    if right { 1.0 } else { -1.0 },
                    mesh,
                ));
            }
            Slot::Glass => parts.push(part(
                "glass",
                PartKind::Glass,
                Paint::Glass,
                PartMount::Fixed,
                10.0,
                6_000.0,
                0.0,
                mesh,
            )),
            Slot::Head(s) => parts.push(part(
                "headlamp",
                PartKind::Light,
                Paint::HeadLamp,
                PartMount::Fixed,
                2.0,
                2_500.0,
                s as f32,
                mesh,
            )),
            Slot::Tail(s) => parts.push(part(
                "tail lamp",
                PartKind::Light,
                Paint::TailLamp,
                PartMount::Fixed,
                1.0,
                2_000.0,
                s as f32,
                mesh,
            )),
        }
    }
    let wheel_w = desc.wheels.first().map(|w| w.width).unwrap_or(0.22);
    let (tyre, rim) = wheel_meshes(wheel_r, wheel_w);
    CarModel {
        body,
        parts,
        tyre,
        rim,
        wheel_radius: wheel_r,
        wheel_width: wheel_w,
    }
}

/// Tyre (rounded tread + sidewalls) and a five-spoke rim, axis along X, centred at the origin.
pub fn wheel_meshes(r: f32, w: f32) -> (MeshData, MeshData) {
    let seg = 40;
    let mut tyre = MeshData::default();
    // Profile around the tyre cross-section (x, radius).
    let rim_r = r * 0.66;
    let profile: Vec<(f32, f32)> = vec![
        (-w * 0.5, rim_r),
        (-w * 0.5, r * 0.82),
        (-w * 0.46, r * 0.95),
        (-w * 0.36, r),
        (w * 0.36, r),
        (w * 0.46, r * 0.95),
        (w * 0.5, r * 0.82),
        (w * 0.5, rim_r),
    ];
    for i in 0..=seg {
        let a = i as f32 / seg as f32 * std::f32::consts::TAU;
        for (j, (x, rr)) in profile.iter().enumerate() {
            tyre.positions
                .push(Vec3::new(*x, a.cos() * rr, a.sin() * rr));
            tyre.uvs.push(Vec2::new(
                i as f32 / seg as f32 * 8.0,
                j as f32 / (profile.len() - 1) as f32,
            ));
        }
    }
    let pn = profile.len() as u32;
    for i in 0..seg as u32 {
        for j in 0..pn - 1 {
            let a = i * pn + j;
            let b = a + pn;
            tyre.indices
                .extend_from_slice(&[a, a + 1, b, a + 1, b + 1, b]);
        }
    }
    tyre.compute_normals();
    // Rim: a dished disc with five spokes (windows between them are left open).
    let mut rim = MeshData::default();
    let spokes = 5;
    let rings = [
        (0.0f32, 0.12f32),
        (rim_r * 0.25, 0.1),
        (rim_r * 0.92, 0.04),
        (rim_r, 0.0),
    ];
    let n = 60;
    for i in 0..=n {
        let a = i as f32 / n as f32 * std::f32::consts::TAU;
        let spoke = ((a * spokes as f32 / 2.0).sin().abs()).powf(6.0);
        for (k, (rr, x)) in rings.iter().enumerate() {
            // Between spokes the face is pushed back (darker recess).
            let depth = if k == 1 || k == 2 {
                (1.0 - spoke) * w * 0.25
            } else {
                0.0
            };
            rim.positions
                .push(Vec3::new(w * 0.35 - x - depth, a.cos() * rr, a.sin() * rr));
            rim.uvs
                .push(Vec2::new(a / std::f32::consts::TAU, k as f32 / 3.0));
        }
    }
    let rn = rings.len() as u32;
    for i in 0..n as u32 {
        for k in 0..rn - 1 {
            let a = i * rn + k;
            let b = a + rn;
            rim.indices
                .extend_from_slice(&[a, b, a + 1, a + 1, b, b + 1]);
        }
    }
    // Mirror the rim face for the inner side.
    let count = rim.positions.len();
    for i in 0..count {
        let p = rim.positions[i];
        rim.positions.push(Vec3::new(-p.x, p.y, p.z));
        rim.uvs.push(rim.uvs[i]);
    }
    let base = count as u32;
    let tris: Vec<u32> = rim.indices.clone();
    for t in tris.chunks_exact(3) {
        rim.indices
            .extend_from_slice(&[base + t[0], base + t[2], base + t[1]]);
    }
    rim.compute_normals();
    (tyre, rim)
}

impl CarModel {
    /// Damage-model inputs: deformable body and detachable parts.
    pub fn damage_parts(&self) -> (DeformMesh, Vec<PartDesc>) {
        let body = DeformMesh::new(self.body.positions.clone(), self.body.indices.clone());
        let parts = self
            .parts
            .iter()
            .map(|p| PartDesc {
                name: p.name.clone(),
                kind: p.kind,
                mount: p.mount,
                mesh: DeformMesh::new(p.mesh.positions.clone(), p.mesh.indices.clone()),
                mass: p.mass,
                strength: p.strength,
            })
            .collect();
        (body, parts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_class_builds_closed_sensible_models() {
        for class in VehicleClass::ALL {
            let desc = class.desc();
            let m = build_car(class, &desc);
            let tris = m.body.triangle_count()
                + m.parts
                    .iter()
                    .map(|p| p.mesh.triangle_count())
                    .sum::<usize>();
            assert!(tris > 2000, "{class:?}: {tris} triangles");
            let mut lo = Vec3::splat(f32::MAX);
            let mut hi = Vec3::splat(f32::MIN);
            for p in m
                .body
                .positions
                .iter()
                .chain(m.parts.iter().flat_map(|p| p.mesh.positions.iter()))
            {
                lo = lo.min(*p);
                hi = hi.max(*p);
            }
            let size = hi - lo;
            assert!(
                (size.z - desc.size.x).abs() < 0.05,
                "{class:?} length {}",
                size.z
            );
            assert!(
                (size.x - desc.size.y).abs() < 0.15,
                "{class:?} width {}",
                size.x
            );
            assert!(
                (hi.y - desc.size.z).abs() < 0.12,
                "{class:?} height {}",
                hi.y
            );
            assert!(
                m.parts.iter().any(|p| p.kind == PartKind::Glass),
                "{class:?} has no glass"
            );
            if !matches!(
                class,
                VehicleClass::Bus
                    | VehicleClass::Truck
                    | VehicleClass::Van
                    | VehicleClass::Ambulance
            ) {
                assert!(
                    m.parts.iter().any(|p| p.kind == PartKind::Hood),
                    "{class:?} has no hood"
                );
                assert!(
                    m.parts.iter().filter(|p| p.kind == PartKind::Door).count() >= 2,
                    "{class:?} doors"
                );
            }
            assert!(
                m.parts
                    .iter()
                    .any(|p| p.kind == PartKind::Light && p.paint == Paint::HeadLamp),
                "{class:?} headlamps"
            );
        }
    }
}

#[cfg(test)]
mod winding {
    use super::*;
    #[test]
    fn normals_point_outward() {
        let class = VehicleClass::Compact;
        let m = build_car(class, &class.desc());
        let mut out = 0;
        let mut inw = 0;
        let all: Vec<&MeshData> = std::iter::once(&m.body)
            .chain(m.parts.iter().map(|p| &p.mesh))
            .collect();
        for mesh in all {
            for t in mesh.indices.chunks_exact(3) {
                let p: Vec<Vec3> = t.iter().map(|i| mesh.positions[*i as usize]).collect();
                let c = (p[0] + p[1] + p[2]) / 3.0;
                let n = (p[1] - p[0]).cross(p[2] - p[0]);
                if n.dot(c - Vec3::new(0.0, 0.7, c.z * 0.5)) > 0.0 {
                    out += 1
                } else {
                    inw += 1
                }
            }
        }
        eprintln!("outward {out} inward {inw}");
        assert!(out > inw * 4, "outward {out} inward {inw}");
    }
}
