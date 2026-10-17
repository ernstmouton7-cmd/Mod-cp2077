//! Procedural meshes and textures: primitives, placeholder cars, street props
//! and fallback PBR textures (used by the demo, tests and as app fallbacks).

use glam::{Mat4, Quat, Vec2, Vec3, Vec4};

use crate::types::{ImageData, ImageFormat, MeshData};

// ---------------------------------------------------------------------------
// Primitives

/// Quad in the XZ plane facing +Y, centred at the origin. UVs in metres.
pub fn plane(size_x: f32, size_z: f32) -> MeshData {
    let (hx, hz) = (size_x * 0.5, size_z * 0.5);
    let positions = vec![
        Vec3::new(-hx, 0.0, -hz),
        Vec3::new(hx, 0.0, -hz),
        Vec3::new(hx, 0.0, hz),
        Vec3::new(-hx, 0.0, hz),
    ];
    let uvs = positions.iter().map(|p| Vec2::new(p.x, p.z)).collect();
    let mut m = MeshData {
        positions,
        normals: vec![Vec3::Y; 4],
        uvs,
        indices: vec![0, 2, 1, 0, 3, 2],
        ..Default::default()
    };
    m.compute_tangents();
    m
}

/// Axis aligned box between `lo` and `hi` with per-face normals and metre UVs.
pub fn cuboid(lo: Vec3, hi: Vec3) -> MeshData {
    let mut m = MeshData::default();
    let faces: [(Vec3, Vec3, Vec3); 6] = [
        (Vec3::X, Vec3::NEG_Z, Vec3::Y),
        (Vec3::NEG_X, Vec3::Z, Vec3::Y),
        (Vec3::Y, Vec3::X, Vec3::NEG_Z),
        (Vec3::NEG_Y, Vec3::X, Vec3::Z),
        (Vec3::Z, Vec3::X, Vec3::Y),
        (Vec3::NEG_Z, Vec3::NEG_X, Vec3::Y),
    ];
    let c = (lo + hi) * 0.5;
    let e = (hi - lo) * 0.5;
    for (n, u, v) in faces {
        let base = m.positions.len() as u32;
        let center = c + n * e;
        let eu = (u * e).abs().max_element();
        let ev = (v * e).abs().max_element();
        for (su, sv) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
            let p = center + u * eu * su + v * ev * sv;
            m.positions.push(p);
            m.normals.push(n);
            m.uvs.push(Vec2::new(p.dot(u), -p.dot(v)));
        }
        m.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    m.compute_tangents();
    m
}

/// UV sphere.
pub fn sphere(radius: f32, segments: u32, rings: u32) -> MeshData {
    let mut m = MeshData::default();
    for r in 0..=rings {
        let v = r as f32 / rings as f32;
        let theta = v * std::f32::consts::PI;
        for s in 0..=segments {
            let u = s as f32 / segments as f32;
            let phi = u * std::f32::consts::TAU;
            let n = Vec3::new(
                theta.sin() * phi.cos(),
                theta.cos(),
                theta.sin() * phi.sin(),
            );
            m.positions.push(n * radius);
            m.normals.push(n);
            m.uvs.push(Vec2::new(u * 4.0, v * 2.0));
        }
    }
    let w = segments + 1;
    for r in 0..rings {
        for s in 0..segments {
            let a = r * w + s;
            m.indices
                .extend_from_slice(&[a, a + 1, a + w, a + 1, a + w + 1, a + w]);
        }
    }
    m.compute_tangents();
    m
}

/// Lathe a 2D profile (radius, height) around the Y axis.
pub fn lathe(profile: &[Vec2], segments: u32, smooth: bool) -> MeshData {
    let mut m = MeshData::default();
    let w = segments + 1;
    for (i, p) in profile.iter().enumerate() {
        let _ = i;
        for s in 0..=segments {
            let a = s as f32 / segments as f32 * std::f32::consts::TAU;
            m.positions
                .push(Vec3::new(p.x * a.cos(), p.y, p.x * a.sin()));
            m.uvs.push(Vec2::new(
                s as f32 / segments as f32 * std::f32::consts::TAU * p.x.max(0.05),
                p.y,
            ));
        }
    }
    for i in 0..profile.len() as u32 - 1 {
        for s in 0..segments {
            let a = i * w + s;
            m.indices
                .extend_from_slice(&[a, a + w, a + 1, a + 1, a + w, a + w + 1]);
        }
    }
    if smooth {
        m.compute_normals();
    } else {
        m = flat_shade(&m);
    }
    m.compute_tangents();
    m
}

/// Cylinder along Y from y0 to y1 with caps.
pub fn cylinder(radius: f32, y0: f32, y1: f32, segments: u32) -> MeshData {
    let mut side = lathe(
        &[Vec2::new(radius, y0), Vec2::new(radius, y1)],
        segments,
        true,
    );
    let cap = |y: f32, up: bool| {
        let mut c = MeshData::default();
        c.positions.push(Vec3::new(0.0, y, 0.0));
        c.normals.push(if up { Vec3::Y } else { Vec3::NEG_Y });
        c.uvs.push(Vec2::ZERO);
        for s in 0..=segments {
            let a = s as f32 / segments as f32 * std::f32::consts::TAU;
            let p = Vec3::new(radius * a.cos(), y, radius * a.sin());
            c.positions.push(p);
            c.normals.push(if up { Vec3::Y } else { Vec3::NEG_Y });
            c.uvs.push(Vec2::new(p.x, p.z));
        }
        for s in 0..segments {
            if up {
                c.indices.extend_from_slice(&[0, s + 2, s + 1]);
            } else {
                c.indices.extend_from_slice(&[0, s + 1, s + 2]);
            }
        }
        c.compute_tangents();
        c
    };
    side.append(&cap(y1, true), Mat4::IDENTITY);
    side.append(&cap(y0, false), Mat4::IDENTITY);
    side.compute_tangents();
    side
}

/// Duplicates vertices so each triangle has its own face normal.
pub fn flat_shade(m: &MeshData) -> MeshData {
    let mut o = MeshData::default();
    for t in m.indices.chunks_exact(3) {
        let p: Vec<Vec3> = t.iter().map(|&i| m.positions[i as usize]).collect();
        let n = (p[1] - p[0]).cross(p[2] - p[0]).normalize_or(Vec3::Y);
        for (k, &i) in t.iter().enumerate() {
            o.positions.push(p[k]);
            o.normals.push(n);
            o.uvs.push(*m.uvs.get(i as usize).unwrap_or(&Vec2::ZERO));
            o.indices.push(o.indices.len() as u32);
        }
    }
    o
}

// ---------------------------------------------------------------------------
// Car placeholder: lofted body, glass greenhouse, wheels.

/// Meshes of a procedural car (local space: +Z forward, +Y up, origin on the ground at the centre).
pub struct CarMeshes {
    pub body: MeshData,
    pub glass: MeshData,
    pub interior: MeshData,
    pub tyre: MeshData,
    pub rim: MeshData,
    pub headlights: MeshData,
    pub taillights: MeshData,
    pub trim: MeshData,
    /// Wheel centres (local space) for placing `tyre`/`rim` (which are built around X axis).
    pub wheels: [Vec3; 4],
    pub length: f32,
    pub width: f32,
}

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Parameters for [`car`].
#[derive(Clone, Copy, Debug)]
pub struct CarStyle {
    pub length: f32,
    pub width: f32,
    pub height: f32,
    /// Fraction of length where the windscreen starts (from the front).
    pub cabin_start: f32,
    pub cabin_end: f32,
    pub wheel_radius: f32,
}

impl Default for CarStyle {
    fn default() -> Self {
        Self {
            length: 4.6,
            width: 1.85,
            height: 1.45,
            cabin_start: 0.33,
            cabin_end: 0.80,
            wheel_radius: 0.34,
        }
    }
}

/// Builds a smooth lofted car body: superelliptic sections along the length,
/// a tumblehome greenhouse, wheel arches, glass split out of the upper section.
pub fn car(style: CarStyle) -> CarMeshes {
    let l = style.length;
    let hw = style.width * 0.5;
    let h = style.height;
    let wr = style.wheel_radius;
    let front_axle = l * 0.5 - l * 0.17;
    let rear_axle = -l * 0.5 + l * 0.19;
    let belt = h * 0.64;
    let clearance = 0.16;
    let stations = 72usize;
    let around = 40usize;

    // Side profile helpers as a function of t in [0,1] from front (t=0) to rear.
    let z_of = |t: f32| l * 0.5 - t * l;
    let roof = |t: f32| -> f32 {
        let a = style.cabin_start;
        let b = a + 0.13; // windscreen top
        let c = style.cabin_end - 0.12; // rear glass start
        let d = style.cabin_end;
        let hood = belt + 0.06 * smoothstep(0.0, 0.12, t) - 0.07 * (1.0 - smoothstep(0.0, 0.08, t));
        let trunk = belt + 0.04;
        if t < a {
            hood
        } else if t < b {
            let s = (t - a) / (b - a);
            hood + (h - hood) * (s * (2.0 - s)).powf(0.9)
        } else if t < c {
            h - 0.03 * ((t - (b + c) * 0.5) / (c - b)).powi(2) * 4.0
        } else if t < d {
            let s = (t - c) / (d - c);
            h + (trunk - h) * s.powf(1.4)
        } else {
            trunk - 0.05 * smoothstep(0.92, 1.0, t)
        }
    };
    let bottom = |t: f32| -> f32 {
        let z = z_of(t);
        let mut b =
            clearance + 0.06 * (1.0 - smoothstep(0.0, 0.1, t)) + 0.05 * smoothstep(0.9, 1.0, t);
        for ax in [front_axle, rear_axle] {
            let dz = (z - ax).abs();
            let arch = wr + 0.06;
            if dz < arch {
                b = b.max(wr + (arch * arch - dz * dz).max(0.0).sqrt() * 0.92);
            }
        }
        b
    };
    let half_width = |t: f32| -> f32 {
        let nose = smoothstep(0.0, 0.07, t).powf(0.5);
        let tail = 1.0 - 0.06 * smoothstep(0.9, 1.0, t);
        hw * (0.86 + 0.14 * nose) * tail
    };
    let mut pos: Vec<Vec3> = Vec::new();
    let mut region: Vec<u8> = Vec::new(); // 0 body, 1 glass
    for i in 0..=stations {
        let t = i as f32 / stations as f32;
        let tt = 0.5 - 0.5 * (std::f32::consts::PI * t).cos(); // denser at the ends
        let z = z_of(tt);
        let top = roof(tt);
        let bot = bottom(tt);
        let wbody = half_width(tt);
        let in_cabin = tt > style.cabin_start && tt < style.cabin_end;
        let greenhouse_w = wbody * if in_cabin { 0.80 } else { 0.97 };
        for j in 0..=around {
            // s from -1 (left bottom) over the top to +1 (right bottom)
            let s = j as f32 / around as f32 * 2.0 - 1.0;
            let ang = s * std::f32::consts::PI * 0.5; // -pi/2..pi/2, 0 at the top
            let sx = ang.sin();
            let sy = ang.cos();
            // Superellipse exponent: boxy lower body, rounder top.
            let e = 0.32;
            let px = sx.signum() * sx.abs().powf(e);
            let py = sy.abs().powf(e);
            let yv = bot + (top - bot) * py;
            let mut x = px * wbody;
            if yv > belt {
                // Tumblehome: narrow the greenhouse towards the roof.
                let k = ((yv - belt) / (top - belt).max(0.01)).clamp(0.0, 1.0);
                let target = greenhouse_w * (1.0 - 0.18 * k * k);
                x = px * (wbody + (target - wbody) * smoothstep(0.0, 0.25, k));
            }
            // Slight body-side crease and flare over the wheels.
            pos.push(Vec3::new(x, yv, z));
            let side_band = yv > belt + 0.035 && yv < top - 0.05;
            let glass = in_cabin && side_band && {
                // Pillars: A at the windscreen edge, B in the middle, C at the rear.
                let b_pillar = (tt - (style.cabin_start + 0.24)).abs() < 0.012;
                let a_edge = tt < style.cabin_start + 0.02;
                let c_edge = tt > style.cabin_end - 0.025;
                !(b_pillar || a_edge || c_edge)
            };
            // Windscreen / rear window (top surface in the sloped regions).
            let windscreen = tt > style.cabin_start + 0.01
                && tt < style.cabin_start + 0.125
                && yv > belt + 0.03
                && sx.abs() < 0.8;
            let rear = tt > style.cabin_end - 0.115
                && tt < style.cabin_end - 0.01
                && yv > belt + 0.05
                && sx.abs() < 0.75;
            region.push(if glass || windscreen || rear { 1 } else { 0 });
        }
    }
    let w = around + 1;
    let mut body = MeshData::default();
    let mut glass = MeshData::default();
    body.positions = pos.clone();
    glass.positions = pos.clone();
    body.uvs = pos.iter().map(|p| Vec2::new(p.z, p.y)).collect();
    glass.uvs = body.uvs.clone();
    for i in 0..stations {
        for j in 0..around {
            let a = (i * w + j) as u32;
            let b = a + 1;
            let c = a + w as u32;
            let d = c + 1;
            let g =
                region[a as usize] + region[b as usize] + region[c as usize] + region[d as usize]
                    >= 3;
            let tri = [a, b, c, b, d, c];
            if g {
                glass.indices.extend_from_slice(&tri);
            } else {
                body.indices.extend_from_slice(&tri);
            }
        }
    }
    // Front and rear caps (fan).
    for (st, flip) in [(0usize, false), (stations, true)] {
        let base = st * w;
        let center = pos[base..base + w].iter().copied().sum::<Vec3>() / w as f32;
        let ci = body.positions.len() as u32;
        body.positions.push(center);
        body.uvs.push(Vec2::new(center.z, center.y));
        for j in 0..around {
            let a = (base + j) as u32;
            if flip {
                body.indices.extend_from_slice(&[ci, a, a + 1]);
            } else {
                body.indices.extend_from_slice(&[ci, a + 1, a]);
            }
        }
        // Close the underside between the two bottom edges.
    }
    // Underbody panel.
    for i in 0..stations {
        let a = (i * w) as u32;
        let b = (i * w + around) as u32;
        let c = a + w as u32;
        let d = b + w as u32;
        body.indices.extend_from_slice(&[a, c, b, b, c, d]);
    }
    body.compute_normals();
    // Glass shares the body's normals (same surface) so it sits flush.
    glass.normals = body.normals[..glass.positions.len()].to_vec();
    // Inset glass slightly so it does not z-fight with the pillars.
    for (p, n) in glass.positions.iter_mut().zip(&glass.normals) {
        *p -= *n * 0.004;
    }
    body.compute_tangents();
    glass.compute_tangents();
    // Make sure body normals point outwards (winding of the loft may be inverted).
    fix_winding_outward(&mut body, Vec3::new(0.0, h * 0.5, 0.0));
    fix_winding_outward(&mut glass, Vec3::new(0.0, h * 0.5, 0.0));

    // Interior: dark cabin block + seats so the glass shows something.
    let mut interior = cuboid(
        Vec3::new(-hw * 0.72, belt - 0.35, -l * 0.25),
        Vec3::new(hw * 0.72, h - 0.06, l * 0.14),
    );
    for (x, z) in [(-0.38, -0.05), (0.38, -0.05), (-0.38, -0.85), (0.38, -0.85)] {
        interior.append(
            &cuboid(
                Vec3::new(x - 0.24, belt - 0.3, z - 0.25),
                Vec3::new(x + 0.24, belt + 0.35, z + 0.2),
            ),
            Mat4::IDENTITY,
        );
    }
    interior.compute_tangents();
    fix_winding_outward(&mut interior, Vec3::new(0.0, -100.0, 0.0));

    // Wheels: tyre profile lathed around X.
    let tw = 0.24;
    let rr = wr * 0.62;
    let tyre_prof: Vec<Vec2> = [
        (rr, -tw * 0.42),
        (wr * 0.88, -tw * 0.5),
        (wr * 0.97, -tw * 0.45),
        (wr, -tw * 0.3),
        (wr, tw * 0.3),
        (wr * 0.97, tw * 0.45),
        (wr * 0.88, tw * 0.5),
        (rr, tw * 0.42),
    ]
    .iter()
    .map(|&(r, y)| Vec2::new(r, y))
    .collect();
    let to_x = Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2);
    let mut tyre = MeshData::default();
    tyre.append(&lathe(&tyre_prof, 40, true), to_x);
    tyre.compute_tangents();
    // Rim: dished disc with 5 spokes cut by angular modulation of depth.
    let mut rim = MeshData::default();
    let seg = 60u32;
    let rings = [0.0f32, 0.12, 0.25, 0.55, 0.85, 1.0];
    for (ri, &rf) in rings.iter().enumerate() {
        for s in 0..=seg {
            let a = s as f32 / seg as f32 * std::f32::consts::TAU;
            let spoke = (0.5 + 0.5 * (a * 5.0).cos()).powf(6.0);
            let r = rr * rf;
            let depth = if ri == 0 || ri == 1 {
                0.02
            } else if ri < 5 {
                0.02 + 0.06 * (1.0 - spoke) * (1.0 - (rf - 0.25).abs())
            } else {
                0.0
            };
            rim.positions
                .push(Vec3::new(tw * 0.38 - depth, r * a.sin(), r * a.cos()));
            rim.uvs.push(Vec2::new(a, rf));
        }
    }
    let wv = seg + 1;
    for ri in 0..rings.len() as u32 - 1 {
        for s in 0..seg {
            let a = ri * wv + s;
            rim.indices
                .extend_from_slice(&[a, a + 1, a + wv, a + 1, a + wv + 1, a + wv]);
        }
    }
    rim.compute_normals();
    fix_winding_outward(&mut rim, Vec3::new(-1.0, 0.0, 0.0));
    let rim_inner = cylinder(rr, -tw * 0.42, tw * 0.38, 40);
    rim.append(&rim_inner, to_x);
    rim.compute_tangents();

    // Lights.
    let mut headlights = MeshData::default();
    let mut taillights = MeshData::default();
    let yl = belt - 0.12;
    for sx in [-1.0f32, 1.0] {
        let x = sx * hw * 0.68;
        headlights.append(
            &sphere(0.09, 16, 8),
            Mat4::from_translation(Vec3::new(x, yl, l * 0.5 - 0.05))
                * Mat4::from_scale(Vec3::new(1.9, 0.6, 0.5)),
        );
        taillights.append(
            &sphere(0.09, 16, 8),
            Mat4::from_translation(Vec3::new(x, yl + 0.06, -l * 0.5 + 0.04))
                * Mat4::from_scale(Vec3::new(2.2, 0.5, 0.4)),
        );
    }
    headlights.compute_tangents();
    taillights.compute_tangents();
    // Trim: grille, bumpers' black strips, mirrors, plates.
    let mut trim = MeshData::default();
    trim.append(
        &cuboid(
            Vec3::new(-hw * 0.45, clearance + 0.18, l * 0.5 - 0.08),
            Vec3::new(hw * 0.45, clearance + 0.36, l * 0.5 + 0.005),
        ),
        Mat4::IDENTITY,
    );
    trim.append(
        &cuboid(
            Vec3::new(-hw * 0.92, clearance + 0.02, -l * 0.5 + 0.0),
            Vec3::new(hw * 0.92, clearance + 0.12, -l * 0.5 + 0.12),
        ),
        Mat4::IDENTITY,
    );
    for sx in [-1.0f32, 1.0] {
        let mz = l * 0.5 - l * style.cabin_start - 0.15;
        trim.append(
            &cuboid(
                Vec3::new(sx * (hw * 0.86), belt + 0.05, mz - 0.06),
                Vec3::new(sx * (hw * 0.86 + 0.16), belt + 0.16, mz + 0.06),
            ),
            Mat4::IDENTITY,
        );
    }
    trim.compute_normals();
    let mut trim = flat_shade(&trim);
    trim.compute_tangents();

    let wy = wr;
    let wx = hw - tw * 0.48;
    CarMeshes {
        body,
        glass,
        interior,
        tyre,
        rim,
        headlights,
        taillights,
        trim,
        wheels: [
            Vec3::new(-wx, wy, front_axle),
            Vec3::new(wx, wy, front_axle),
            Vec3::new(-wx, wy, rear_axle),
            Vec3::new(wx, wy, rear_axle),
        ],
        length: l,
        width: style.width,
    }
}

/// Flips triangles whose normal points towards `inside` (relative to the centroid direction).
fn fix_winding_outward(m: &mut MeshData, inside: Vec3) {
    let mut flips = 0i64;
    for t in m.indices.chunks_exact(3) {
        let p: Vec<Vec3> = t.iter().map(|&i| m.positions[i as usize]).collect();
        let n = (p[1] - p[0]).cross(p[2] - p[0]);
        let c = (p[0] + p[1] + p[2]) / 3.0;
        flips += if n.dot(c - inside) < 0.0 { 1 } else { -1 };
    }
    if flips > 0 {
        for t in m.indices.chunks_exact_mut(3) {
            t.swap(1, 2);
        }
        for n in m.normals.iter_mut() {
            *n = -*n;
        }
    }
    // Make vertex normals agree with the (possibly flipped) winding.
    let mut acc = vec![Vec3::ZERO; m.positions.len()];
    for t in m.indices.chunks_exact(3) {
        let p: Vec<Vec3> = t.iter().map(|&i| m.positions[i as usize]).collect();
        let n = (p[1] - p[0]).cross(p[2] - p[0]);
        for &i in t {
            acc[i as usize] += n;
        }
    }
    for (i, n) in m.normals.iter_mut().enumerate() {
        if acc[i].length_squared() > 0.0 && n.dot(acc[i]) < 0.0 {
            *n = -*n;
        }
    }
    m.compute_tangents();
}

// ---------------------------------------------------------------------------
// Procedural textures

fn hash2(x: i32, y: i32, seed: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x8da6_b343)
        ^ (y as u32).wrapping_mul(0xd816_3841)
        ^ seed.wrapping_mul(0xcb1a_b31f);
    h ^= h >> 13;
    h = h.wrapping_mul(0x5bd1_e995);
    h ^= h >> 15;
    (h & 0x00ff_ffff) as f32 / 16_777_215.0
}

/// Tileable value noise (period `p` cells).
pub fn tile_noise(x: f32, y: f32, p: i32, seed: u32) -> f32 {
    let xi = x.floor() as i32;
    let yi = y.floor() as i32;
    let fx = x - xi as f32;
    let fy = y - yi as f32;
    let u = fx * fx * (3.0 - 2.0 * fx);
    let v = fy * fy * (3.0 - 2.0 * fy);
    let h = |a: i32, b: i32| hash2(a.rem_euclid(p), b.rem_euclid(p), seed);
    let a = h(xi, yi);
    let b = h(xi + 1, yi);
    let c = h(xi, yi + 1);
    let d = h(xi + 1, yi + 1);
    a + (b - a) * u + (c - a) * v + (a - b - c + d) * u * v
}

pub fn tile_fbm(x: f32, y: f32, p: i32, octaves: u32, seed: u32) -> f32 {
    let mut s = 0.0;
    let mut a = 0.5;
    let mut f = 1.0;
    let mut norm = 0.0;
    for o in 0..octaves {
        s += a * tile_noise(x * f, y * f, p * f as i32, seed + o * 17);
        norm += a;
        a *= 0.5;
        f *= 2.0;
    }
    s / norm
}

/// A set of generated PBR maps.
pub struct PbrImages {
    pub albedo: ImageData,
    pub normal: ImageData,
    /// glTF/Poly Haven style ARM: R = AO, G = roughness, B = metallic.
    pub arm: ImageData,
}

/// Builds albedo/normal/ARM images from per-pixel height, colour and roughness closures.
pub fn pbr_from_fn(
    size: u32,
    normal_strength: f32,
    f: impl Fn(f32, f32) -> (f32, Vec3, f32, f32),
) -> PbrImages {
    let n = size as usize;
    let mut height = vec![0.0f32; n * n];
    let mut alb = vec![0u8; n * n * 4];
    let mut arm = vec![0u8; n * n * 4];
    let enc = |c: f32| -> u8 {
        let c = c.clamp(0.0, 1.0);
        let s = if c <= 0.003_130_8 {
            c * 12.92
        } else {
            1.055 * c.powf(1.0 / 2.4) - 0.055
        };
        (s * 255.0 + 0.5) as u8
    };
    for y in 0..n {
        for x in 0..n {
            let (hgt, col, rough, ao) = f(x as f32 / size as f32, y as f32 / size as f32);
            height[y * n + x] = hgt;
            let i = (y * n + x) * 4;
            alb[i] = enc(col.x);
            alb[i + 1] = enc(col.y);
            alb[i + 2] = enc(col.z);
            alb[i + 3] = 255;
            arm[i] = (ao.clamp(0.0, 1.0) * 255.0) as u8;
            arm[i + 1] = (rough.clamp(0.0, 1.0) * 255.0) as u8;
            arm[i + 2] = 0;
            arm[i + 3] = 255;
        }
    }
    let mut nor = vec![0u8; n * n * 4];
    for y in 0..n {
        for x in 0..n {
            let hx = height[y * n + (x + 1) % n] - height[y * n + (x + n - 1) % n];
            let hy = height[((y + 1) % n) * n + x] - height[((y + n - 1) % n) * n + x];
            // OpenGL convention: +Y is "up" in the image (decreasing row index).
            let v = Vec3::new(-hx * normal_strength, hy * normal_strength, 1.0).normalize();
            let i = (y * n + x) * 4;
            nor[i] = ((v.x * 0.5 + 0.5) * 255.0) as u8;
            nor[i + 1] = ((v.y * 0.5 + 0.5) * 255.0) as u8;
            nor[i + 2] = ((v.z * 0.5 + 0.5) * 255.0) as u8;
            nor[i + 3] = 255;
        }
    }
    PbrImages {
        albedo: ImageData::rgba8(size, size, alb, true),
        normal: ImageData::rgba8(size, size, nor, false),
        arm: ImageData {
            width: size,
            height: size,
            format: ImageFormat::Rgba8,
            mips: vec![arm],
            srgb: false,
        },
    }
}

/// Fallback asphalt (covers 4 m).
pub fn asphalt(size: u32) -> PbrImages {
    pbr_from_fn(size, 6.0, |u, v| {
        let g = tile_fbm(u * 64.0, v * 64.0, 64, 4, 1);
        let stones = tile_noise(u * 400.0, v * 400.0, 400, 2);
        let patch = tile_fbm(u * 4.0, v * 4.0, 4, 3, 3);
        let h = g * 0.4 + stones.powf(3.0) * 0.6;
        let base = 0.055 + 0.04 * patch + 0.05 * stones.powf(4.0);
        let col = Vec3::new(base, base * 0.98, base * 0.95);
        let rough = 0.82 + 0.12 * g - 0.15 * stones.powf(6.0);
        (h, col, rough, 1.0 - 0.3 * (1.0 - h))
    })
}

/// Fallback concrete sidewalk with 1.5 m slab joints (covers 3 m).
pub fn concrete_slabs(size: u32) -> PbrImages {
    pbr_from_fn(size, 4.0, |u, v| {
        let ju = (u * 2.0).fract();
        let jv = (v * 2.0).fract();
        let joint = ((ju.min(1.0 - ju)).min(jv.min(1.0 - jv)) * 3.0 / 0.012).min(1.0);
        let g = tile_fbm(u * 48.0, v * 48.0, 48, 4, 7);
        let stain = tile_fbm(u * 6.0, v * 6.0, 6, 3, 9);
        let h = joint * (0.8 + 0.2 * g);
        let b = (0.36 + 0.07 * g - 0.08 * stain) * (0.6 + 0.4 * joint);
        (
            h,
            Vec3::new(b, b * 0.98, b * 0.94),
            0.78 + 0.15 * g,
            0.55 + 0.45 * joint,
        )
    })
}

/// Fallback brick wall (covers 2 m).
pub fn brick(size: u32, tint: Vec3) -> PbrImages {
    pbr_from_fn(size, 5.0, move |u, v| {
        let rows = 30.0;
        let r = (v * rows).floor();
        let off = if r as i32 % 2 == 0 { 0.0 } else { 0.5 };
        let cols = 9.0;
        let cu = u * cols + off;
        let fu = cu.fract();
        let fv = (v * rows).fract();
        let mortar = (fu.min(1.0 - fu) * 20.0)
            .min(fv.min(1.0 - fv) * 7.0)
            .min(1.0);
        let id = hash2(cu.floor() as i32 % cols as i32, r as i32, 5);
        let g = tile_fbm(u * 40.0, v * 40.0, 40, 3, 11);
        let bc = tint * (0.75 + 0.35 * id) * (0.85 + 0.2 * g);
        let mc = Vec3::splat(0.42 + 0.05 * g);
        let m = smoothstep(0.0, 0.6, mortar);
        let col = mc.lerp(bc, m);
        (m * (0.9 + 0.1 * g), col, 0.75 + 0.15 * g, 0.6 + 0.4 * m)
    })
}

/// Solid flat colour texture.
pub fn solid_rgba(color: Vec4) -> ImageData {
    let px = [
        (color.x * 255.0) as u8,
        (color.y * 255.0) as u8,
        (color.z * 255.0) as u8,
        (color.w * 255.0) as u8,
    ];
    ImageData::rgba8(1, 1, px.to_vec(), true)
}

// ---------------------------------------------------------------------------
// Foliage

/// Leaf-cluster atlas: RGBA albedo (alpha = leaf mask, coverage-preserving mips) + normal map.
pub fn leaf_atlas(size: u32, seed: u32, base: Vec3) -> (ImageData, ImageData) {
    let n = size as usize;
    let mut col = vec![[0.0f32; 4]; n * n];
    let mut nrm = vec![[0.5f32, 0.5, 1.0]; n * n];
    let mut s = seed.wrapping_mul(2_654_435_761).max(1);
    let mut rnd = || {
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        (s & 0x00ff_ffff) as f32 / 16_777_215.0
    };
    let leaves = 4 * (34 + (size / 24) as usize);
    let cell = size as f32 * 0.5;
    for li in 0..leaves {
        // Four round leaf clusters (2x2 cells); each card uses one cell.
        let (gx, gy) = ((li % 2) as f32, ((li / 2) % 2) as f32);
        let ra = rnd() * std::f32::consts::TAU;
        let rr = rnd().sqrt() * cell * 0.36;
        let cx = (gx + 0.5) * cell + ra.cos() * rr;
        let cy = (gy + 0.5) * cell + ra.sin() * rr;
        let ang = rnd() * std::f32::consts::TAU;
        let len = size as f32 * (0.07 + 0.05 * rnd());
        let wid = len * (0.38 + 0.12 * rnd());
        let hue = rnd();
        let c = Vec3::new(
            base.x * (0.7 + 0.6 * hue),
            base.y * (0.8 + 0.4 * rnd()),
            base.z * (0.6 + 0.6 * rnd()),
        );
        let (ca, sa) = (ang.cos(), ang.sin());
        let r = len.max(wid) as i32 + 2;
        for dy in -r..=r {
            for dx in -r..=r {
                let (xi, yi) = (cx as i32 + dx, cy as i32 + dy);
                if xi < 0 || yi < 0 || xi >= size as i32 || yi >= size as i32 {
                    continue;
                }
                let (x, y) = (xi as usize, yi as usize);
                let lx = (dx as f32 * ca + dy as f32 * sa) / len;
                let ly = (-dx as f32 * sa + dy as f32 * ca) / wid;
                // Leaf outline: pointed ellipse.
                let taper = 1.0 - 0.35 * lx.abs().powf(1.5);
                let d = lx * lx + (ly / taper.max(0.2)).powi(2);
                if d < 1.0 {
                    let vein = (1.0 - (ly.abs() * 12.0).min(1.0)) * 0.25;
                    let shade = 0.75 + 0.25 * (1.0 - d) + vein;
                    let i = y * n + x;
                    col[i] = [c.x * shade, c.y * shade, c.z * shade, 1.0];
                    // Dome-shaped leaf normal in texture space.
                    let gx = lx * ca - ly * sa;
                    let gy = lx * sa + ly * ca;
                    let v = Vec3::new(gx * 0.45, -gy * 0.45, 1.0).normalize();
                    nrm[i] = [v.x * 0.5 + 0.5, v.y * 0.5 + 0.5, v.z * 0.5 + 0.5];
                }
            }
        }
    }
    let enc = |c: f32| -> u8 {
        let c = c.clamp(0.0, 1.0);
        let s = if c <= 0.003_130_8 {
            c * 12.92
        } else {
            1.055 * c.powf(1.0 / 2.4) - 0.055
        };
        (s * 255.0 + 0.5) as u8
    };
    // Mips: average colour of covered texels only (avoids dark fringes), alpha boosted to keep coverage.
    let mut levels: Vec<Vec<[f32; 4]>> = vec![col];
    let mut w = n;
    while w > 1 {
        let prev = levels.last().unwrap();
        let nw = w / 2;
        let mut next = vec![[0.0f32; 4]; nw * nw];
        for y in 0..nw {
            for x in 0..nw {
                let mut acc = [0.0f32; 4];
                let mut a = 0.0;
                for (ox, oy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let p = prev[(2 * y + oy) * w + 2 * x + ox];
                    for k in 0..3 {
                        acc[k] += p[k] * p[3];
                    }
                    a += p[3];
                }
                let inv = if a > 0.0 { 1.0 / a } else { 0.0 };
                next[y * nw + x] = [
                    acc[0] * inv,
                    acc[1] * inv,
                    acc[2] * inv,
                    (a * 0.25 * 1.35).min(1.0),
                ];
            }
        }
        levels.push(next);
        w = nw;
    }
    let albedo_mips: Vec<Vec<u8>> = levels
        .iter()
        .map(|l| {
            l.iter()
                .flat_map(|p| [enc(p[0]), enc(p[1]), enc(p[2]), (p[3] * 255.0) as u8])
                .collect()
        })
        .collect();
    let nrm_bytes: Vec<u8> = nrm
        .iter()
        .flat_map(|p| {
            [
                (p[0] * 255.0) as u8,
                (p[1] * 255.0) as u8,
                (p[2] * 255.0) as u8,
                255,
            ]
        })
        .collect();
    (
        ImageData {
            width: size,
            height: size,
            format: ImageFormat::Rgba8,
            mips: albedo_mips,
            srgb: true,
        },
        ImageData::rgba8(size, size, nrm_bytes, false),
    )
}

/// Broadleaf street tree: (trunk + branches, leaf cards). Leaf card normals are bent
/// towards the crown's outward direction for soft, volumetric shading.
pub fn tree(seed: u32, height: f32) -> (MeshData, MeshData) {
    let mut s = seed
        .wrapping_mul(747_796_405)
        .wrapping_add(2_891_336_453)
        .max(1);
    let mut rnd = || {
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        (s & 0x00ff_ffff) as f32 / 16_777_215.0
    };
    let crown_c = Vec3::new(0.0, height * 0.68, 0.0);
    let crown_r = Vec3::new(height * 0.36, height * 0.3, height * 0.36);
    let mut wood = lathe(
        &[
            Vec2::new(0.17, 0.0),
            Vec2::new(0.13, 0.5),
            Vec2::new(0.11, height * 0.45),
            Vec2::new(0.07, height * 0.62),
        ],
        12,
        true,
    );
    for _ in 0..7 {
        let a = rnd() * std::f32::consts::TAU;
        let tip = crown_c
            + Vec3::new(
                a.cos() * crown_r.x * 0.75,
                (rnd() - 0.3) * crown_r.y,
                a.sin() * crown_r.z * 0.75,
            );
        let base = Vec3::new(0.0, height * (0.38 + 0.15 * rnd()), 0.0);
        let d = tip - base;
        let len = d.length();
        let b = lathe(&[Vec2::new(0.06, 0.0), Vec2::new(0.025, len)], 6, true);
        let rot = Quat::from_rotation_arc(Vec3::Y, d / len);
        wood.append(&b, Mat4::from_rotation_translation(rot, base));
    }
    wood.compute_tangents();
    let mut leaves = MeshData {
        colors: Some(Vec::new()),
        ..Default::default()
    };
    let cards = 150;
    let mut placed = 0;
    while placed < cards {
        let p = Vec3::new(rnd() * 2.0 - 1.0, rnd() * 2.0 - 1.0, rnd() * 2.0 - 1.0);
        let l = p.length();
        if !(0.45..=1.0).contains(&l) {
            continue;
        }
        placed += 1;
        let pos = crown_c + p * crown_r;
        let outward = (p / crown_r * crown_r.max_element()).normalize_or(Vec3::Y);
        let nrm = Vec3::new(rnd() * 2.0 - 1.0, rnd() * 2.0 - 1.0, rnd() * 2.0 - 1.0)
            .normalize_or(Vec3::Y);
        let t = nrm.any_orthonormal_vector();
        let b = nrm.cross(t);
        let size = 0.55 + 0.35 * rnd();
        let bent = (outward * 0.8 + nrm * 0.2).normalize();
        let shade = 0.55 + 0.45 * ((l - 0.45) / 0.55) * (0.6 + 0.4 * (p.y * 0.5 + 0.5));
        let u0 = (rnd() * 2.0).floor().min(1.0) * 0.5;
        let v0 = (rnd() * 2.0).floor().min(1.0) * 0.5;
        let base = leaves.positions.len() as u32;
        for (k, (su, sv)) in [(-1.0f32, -1.0f32), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
            .iter()
            .enumerate()
        {
            leaves.positions.push(pos + (t * *su + b * *sv) * size);
            leaves.normals.push(bent);
            let uv = [(0.0, 0.0), (0.5, 0.0), (0.5, 0.5), (0.0, 0.5)][k];
            leaves.uvs.push(Vec2::new(u0 + uv.0, v0 + uv.1));
            leaves
                .colors
                .as_mut()
                .unwrap()
                .push(Vec4::new(shade, shade, shade, 1.0));
        }
        leaves
            .indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    leaves.compute_tangents();
    // compute_tangents may not change normals; keep the bent normals.
    (wood, leaves)
}
