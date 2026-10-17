//! Turns the game's static city and per-frame `SceneFrame` into renderer meshes, instances,
//! lights, decals and particles.

use crate::assets::{Assets, PbrSet};
use glam::{Mat4, Quat, Vec2, Vec3, Vec4};
use gridlock_game::city::{MarkKind, CORNER_RADIUS, CURB_HEIGHT};
use gridlock_game::physics::props::PropKind;
use gridlock_game::physics::vehicle::presets::VehicleClass as PClass;
use gridlock_game::physics::Pose;
use gridlock_game::{BuildingStyle, Game, Paint, SceneFrame};
use gridlock_render::procedural as pg;
use gridlock_render::*;
use std::collections::HashMap;

fn mesh_from_game(m: &gridlock_game::MeshData) -> MeshData {
    let mut out = MeshData {
        positions: m.positions.clone(),
        normals: m.normals.clone(),
        tangents: Vec::new(),
        uvs: if m.uvs.len() == m.positions.len() {
            m.uvs.clone()
        } else {
            vec![Vec2::ZERO; m.positions.len()]
        },
        uvs2: None,
        colors: None,
        indices: m.indices.clone(),
    };
    out.compute_tangents();
    out
}

fn mat4(p: &Pose) -> Mat4 {
    Mat4::from_rotation_translation(p.q, p.p)
}

fn srgb_to_linear(c: u32) -> Vec3 {
    let f = |v: u32| {
        let x = v as f32 / 255.0;
        if x <= 0.04045 {
            x / 12.92
        } else {
            ((x + 0.055) / 1.055).powf(2.4)
        }
    };
    Vec3::new(f((c >> 16) & 255), f((c >> 8) & 255), f(c & 255))
}

struct ClassMeshes {
    body: MeshId,
    /// Dark cabin block behind the glass.
    interior: MeshId,
    parts: Vec<(MeshId, Paint)>,
    tyre: MeshId,
    rim: MeshId,
}

struct CarInst {
    class: PClass,
    body: InstanceId,
    interior: InstanceId,
    parts: Vec<(InstanceId, Paint)>,
    wheels: Vec<(InstanceId, InstanceId)>,
    /// Own deformable mesh copies once the car has been damaged.
    own: Option<(MeshId, Vec<MeshId>)>,
    revision: u64,
    metallic: bool,
    seen: bool,
}

struct Mats {
    paint: MaterialId,
    paint_metal: MaterialId,
    trim: MaterialId,
    glass: MaterialId,
    head: MaterialId,
    tail: MaterialId,
    tyre: MaterialId,
    rim: MaterialId,
    pole: MaterialId,
    lamp_glow: MaterialId,
    sig: [MaterialId; 3],
    housing: MaterialId,
    sign: MaterialId,
    ped_shirt: MaterialId,
    ped_dark: MaterialId,
    ped_skin: MaterialId,
    orange: MaterialId,
    red: MaterialId,
    blue: MaterialId,
    concrete: MaterialId,
    bush: MaterialId,
}

/// Visual pieces of a prop kind: (mesh, material, local transform).
type PropVisual = Vec<(MeshId, MaterialId, Mat4)>;

pub struct WorldView {
    mats: Mats,
    classes: HashMap<PClass, ClassMeshes>,
    cars: HashMap<u32, CarInst>,
    prop_kinds: HashMap<PropKind, PropVisual>,
    props: Vec<Vec<(InstanceId, Mat4)>>,
    signal_lamps: Vec<[InstanceId; 3]>,
    peds: Vec<[InstanceId; 3]>,
    ped_meshes: (MeshId, MeshId, MeshId),
    facade_instances: Vec<(InstanceId, bool)>,
    lamp_glows: Vec<InstanceId>,
    lamp_positions: Vec<Vec3>,
    signal_glow: Vec<(Vec3, Vec3)>,
    unit_cube: MeshId,
    unit_sphere: MeshId,
    pub night: f32,
    pub debug_cars: bool,
}

impl WorldView {
    pub fn new(r: &mut Renderer, a: &mut Assets, game: &mut Game) -> Self {
        let mats = Self::materials(r, a);
        let unit_cube = r.create_mesh(&pg::cuboid(Vec3::splat(-0.5), Vec3::splat(0.5)));
        let unit_sphere = r.create_mesh(&pg::sphere(0.5, 16, 10));
        let mut v = Self {
            mats,
            classes: HashMap::new(),
            cars: HashMap::new(),
            prop_kinds: HashMap::new(),
            props: Vec::new(),
            signal_lamps: Vec::new(),
            peds: Vec::new(),
            ped_meshes: (unit_cube, unit_cube, unit_cube),
            facade_instances: Vec::new(),
            lamp_glows: Vec::new(),
            lamp_positions: game.city.lamp_lights.clone(),
            signal_glow: Vec::new(),
            unit_cube,
            unit_sphere,
            night: 0.0,
            debug_cars: false,
        };
        v.ped_meshes = (
            r.create_mesh(&pg::cylinder(0.19, 0.0, 0.5, 10)),
            r.create_mesh(&pg::cylinder(0.12, 0.0, 0.85, 8)),
            r.create_mesh(&pg::sphere(0.11, 12, 8)),
        );
        v.build_static(r, a, game);
        v.build_prop_kinds(r, a);
        v
    }

    fn materials(r: &mut Renderer, a: &mut Assets) -> Mats {
        let mut solid =
            |c: Vec3, rough: f32, metal: f32| r.create_material(&Material::solid(c, rough, metal));
        let trim = solid(Vec3::splat(0.03), 0.55, 0.0);
        let pole = solid(Vec3::new(0.12, 0.13, 0.14), 0.45, 0.8);
        let housing = solid(Vec3::new(0.04, 0.045, 0.03), 0.6, 0.2);
        let ped_dark = solid(Vec3::new(0.05, 0.05, 0.07), 0.8, 0.0);
        let ped_skin = solid(Vec3::new(0.55, 0.38, 0.28), 0.6, 0.0);
        let ped_shirt = solid(Vec3::ONE, 0.8, 0.0);
        let orange = solid(Vec3::new(0.9, 0.25, 0.02), 0.5, 0.0);
        let red = solid(Vec3::new(0.55, 0.03, 0.02), 0.45, 0.1);
        let blue = solid(Vec3::new(0.02, 0.08, 0.35), 0.4, 0.3);
        let bush = solid(Vec3::new(0.06, 0.16, 0.04), 0.85, 0.0);
        let sign = solid(Vec3::new(0.6, 0.02, 0.02), 0.35, 0.0);
        let concrete = match a.pbr(r, "tex.concrete_panels") {
            Some(p) => r.create_material(&pbr_material(&p, UvMode::Triplanar, 1.0)),
            None => r.create_material(&Material::solid(Vec3::splat(0.5), 0.8, 0.0)),
        };
        let tyre = match a.pbr(r, "tex.rubber") {
            Some(p) => r.create_material(&Material {
                base_color: Vec4::new(0.25, 0.25, 0.25, 1.0),
                ..pbr_material(&p, UvMode::Mesh, 1.0)
            }),
            None => r.create_material(&Material::solid(Vec3::splat(0.02), 0.85, 0.0)),
        };
        let rim = r.create_material(&Material::solid(Vec3::new(0.7, 0.7, 0.72), 0.22, 1.0));
        let paint = r.create_material(&Material::car_paint(Vec3::ONE, false));
        let paint_metal = r.create_material(&Material::car_paint(Vec3::ONE, true));
        let glass = r.create_material(&Material {
            roughness: 0.03,
            ..Material::glass(Vec3::new(0.015, 0.02, 0.025), 0.82)
        });
        let head = r.create_material(&Material {
            emissive: Vec3::new(1.0, 0.96, 0.9) * 3500.0,
            base_color: Vec4::new(0.9, 0.9, 0.9, 1.0),
            roughness: 0.05,
            ..Default::default()
        });
        let tail = r.create_material(&Material {
            emissive: Vec3::new(1.0, 0.03, 0.02) * 600.0,
            base_color: Vec4::new(0.4, 0.02, 0.02, 1.0),
            roughness: 0.1,
            ..Default::default()
        });
        let lamp_glow = r.create_material(&Material {
            emissive: kelvin_to_rgb(3200.0) * 9000.0,
            base_color: Vec4::new(1.0, 0.95, 0.85, 1.0),
            cast_shadows: false,
            ..Default::default()
        });
        let sig = [
            r.create_material(&Material {
                emissive: Vec3::new(1.0, 0.04, 0.02) * 2500.0,
                base_color: Vec4::new(0.3, 0.02, 0.02, 1.0),
                roughness: 0.15,
                ..Default::default()
            }),
            r.create_material(&Material {
                emissive: Vec3::new(1.0, 0.5, 0.02) * 2500.0,
                base_color: Vec4::new(0.3, 0.2, 0.02, 1.0),
                roughness: 0.15,
                ..Default::default()
            }),
            r.create_material(&Material {
                emissive: Vec3::new(0.08, 1.0, 0.45) * 2500.0,
                base_color: Vec4::new(0.02, 0.25, 0.1, 1.0),
                roughness: 0.15,
                ..Default::default()
            }),
        ];
        Mats {
            paint,
            paint_metal,
            trim,
            glass,
            head,
            tail,
            tyre,
            rim,
            pole,
            lamp_glow,
            sig,
            housing,
            sign,
            ped_shirt,
            ped_dark,
            ped_skin,
            orange,
            red,
            blue,
            concrete,
            bush,
        }
    }

    fn build_static(&mut self, r: &mut Renderer, a: &mut Assets, game: &Game) {
        let city = &game.city;
        // Road surface.
        let asphalt = match a.pbr(r, "tex.asphalt") {
            Some(p) => r.create_material(&Material {
                roughness: 0.9,
                ..pbr_material(&p, UvMode::WorldXZ, 1.0)
            }),
            None => r.create_material(&Material::solid(Vec3::splat(0.07), 0.85, 0.0)),
        };
        let c = (city.min + city.max) * 0.5;
        let half = (city.max - city.min) * 0.5;
        let road = r.create_mesh(&quad_xz(Vec3::new(c.x, 0.0, c.y), half));
        r.add_instance(road, asphalt, Mat4::IDENTITY);
        // Blocks: sidewalk tops, kerbs and verges.
        let sidewalk = match a.pbr(r, "tex.concrete_sidewalk") {
            Some(p) => r.create_material(&pbr_material(&p, UvMode::WorldXZ, 1.0)),
            None => r.create_material(&Material::solid(Vec3::splat(0.45), 0.8, 0.0)),
        };
        let curb = match a.pbr(r, "tex.curb") {
            Some(p) => r.create_material(&pbr_material(&p, UvMode::Triplanar, 1.0)),
            None => r.create_material(&Material::solid(Vec3::splat(0.55), 0.7, 0.0)),
        };
        let grass = match a.pbr(r, "tex.grass") {
            Some(p) => r.create_material(&pbr_material(&p, UvMode::WorldXZ, 1.0)),
            None => r.create_material(&Material::solid(Vec3::new(0.08, 0.15, 0.04), 0.9, 0.0)),
        };
        let mut tops = MeshData::default();
        let mut verge = MeshData::default();
        let mut kerbs = MeshData::default();
        for b in &city.blocks {
            let outline = b.outline(if b.outer { 0.0 } else { CORNER_RADIUS });
            let target = if b.outer { &mut verge } else { &mut tops };
            fan_top(target, &outline, CURB_HEIGHT);
            kerb_walls(&mut kerbs, &outline, CURB_HEIGHT);
        }
        for (m, mat) in [(tops, sidewalk), (verge, grass), (kerbs, curb)] {
            let mut m = m;
            m.compute_tangents();
            let id = r.create_mesh(&m);
            r.add_instance(id, mat, Mat4::IDENTITY);
        }
        // Buildings grouped by facade material.
        let facade_ids = [
            "tex.brick_facade",
            "tex.plaster",
            "tex.concrete_panels",
            "tex.facade_tiles",
            "tex.siding",
            "tex.brick_industrial",
            "tex.facade_office",
            "tex.facade_glass_curtain",
        ];
        let mut facades: Vec<(MaterialId, bool)> = Vec::new();
        for id in facade_ids {
            match a.pbr(r, id) {
                Some(p) => {
                    let emis = p.emission.is_some();
                    let mut m = pbr_material(&p, UvMode::Mesh, 1.0);
                    if emis {
                        m.emissive = kelvin_to_rgb(3000.0) * 60.0;
                    }
                    facades.push((r.create_material(&m), emis));
                }
                None => {
                    let hue = facades.len() as f32 * 0.13;
                    facades.push((
                        r.create_material(&Material::solid(
                            Vec3::new(0.4 + hue * 0.3, 0.35, 0.3 - hue * 0.1)
                                .clamp(Vec3::ZERO, Vec3::ONE),
                            0.8,
                            0.0,
                        )),
                        false,
                    ));
                }
            }
        }
        let roof = match a.pbr(r, "tex.roof_gravel") {
            Some(p) => r.create_material(&pbr_material(&p, UvMode::WorldXZ, 1.0)),
            None => r.create_material(&Material::solid(Vec3::splat(0.25), 0.9, 0.0)),
        };
        let mut groups: Vec<MeshData> = (0..facades.len()).map(|_| MeshData::default()).collect();
        let mut roofs = MeshData::default();
        for b in &city.buildings {
            if b.size.y < 0.5 {
                continue;
            }
            let pick = match b.style {
                BuildingStyle::Tower => 6 + (b.seed % 2) as usize,
                BuildingStyle::Garage => 2,
                _ => (b.seed % 6) as usize,
            }
            .min(facades.len() - 1);
            let lo = b.base - Vec3::new(b.size.x * 0.5, 0.0, b.size.z * 0.5);
            let hi = b.base + Vec3::new(b.size.x * 0.5, b.size.y, b.size.z * 0.5);
            let mut walls = pg::cuboid(lo, hi);
            // Drop the top and bottom faces from the facade mesh; the roof gets gravel.
            strip_horizontal(&mut walls);
            // Facade textures are ~3 m per floor; map metres -> tiles.
            let tile = a
                .pbr(r, facade_ids[pick])
                .map_or(Vec2::splat(3.0), |p| p.tile);
            for uv in &mut walls.uvs {
                *uv /= tile;
            }
            groups[pick].append(&walls, Mat4::IDENTITY);
            let top = quad_xz(
                Vec3::new(b.base.x, hi.y, b.base.z),
                glam::Vec2::new(b.size.x * 0.5, b.size.z * 0.5),
            );
            roofs.append(&top, Mat4::IDENTITY);
        }
        for (i, mut g) in groups.into_iter().enumerate() {
            if g.indices.is_empty() {
                continue;
            }
            g.compute_tangents();
            let id = r.create_mesh(&g);
            let inst = r.add_instance(id, facades[i].0, Mat4::IDENTITY);
            self.facade_instances.push((inst, facades[i].1));
        }
        roofs.compute_tangents();
        let roof_mesh = r.create_mesh(&roofs);
        r.add_instance(roof_mesh, roof, Mat4::IDENTITY);
        // Trees between lamps.
        let (leaf_albedo, leaf_normal) = pg::leaf_atlas(512, 7, Vec3::new(0.12, 0.28, 0.06));
        let la = r.create_texture(&leaf_albedo);
        let ln = r.create_texture(&leaf_normal);
        let leaves = r.create_material(&Material {
            albedo_tex: Some(la),
            normal_tex: Some(ln),
            alpha_mode: AlphaMode::Mask(0.45),
            double_sided: true,
            translucency: 0.45,
            roughness: 0.6,
            ..Default::default()
        });
        let bark = r.create_material(&Material::solid(Vec3::new(0.12, 0.09, 0.07), 0.9, 0.0));
        let variants: Vec<(MeshId, MeshId)> = (0..4)
            .map(|k| {
                let (t, l) = pg::tree(11 + k * 17, 7.0 + k as f32 * 1.3);
                (r.create_mesh(&t), r.create_mesh(&l))
            })
            .collect();
        for (k, (kind, pose)) in city.props.iter().enumerate() {
            if *kind != PropKind::StreetLamp || k % 2 == 1 {
                continue;
            }
            // Halfway to the next lamp, a little further from the kerb.
            let fwd = pose.q * Vec3::X;
            let back = pose.q * Vec3::Z * 0.4;
            let p = pose.p + fwd * 16.0 + back;
            let (t, l) = variants[k % variants.len()];
            let xf = Mat4::from_rotation_translation(Quat::from_rotation_y(k as f32 * 1.7), p);
            r.add_instance(t, bark, xf);
            r.add_instance(l, leaves, xf);
        }
        // Road markings as decals; straight polylines collapse to one decal each.
        let white = Vec4::new(0.85, 0.85, 0.82, 0.92);
        let yellow = Vec4::new(0.85, 0.6, 0.08, 0.92);
        for m in &city.markings {
            if m.points.len() < 2 {
                continue;
            }
            for (a0, b0) in straight_runs(&m.points) {
                let dir = b0 - a0;
                let len = dir.length();
                if len < 0.3 {
                    continue;
                }
                let (kind, color, width) = match m.kind {
                    MarkKind::WhiteSolid => (DecalKind::Paint, white, m.width),
                    MarkKind::WhiteDashed => (
                        DecalKind::Dashed {
                            dash: 3.0,
                            gap: 9.0,
                        },
                        white,
                        m.width,
                    ),
                    MarkKind::YellowSolid => (DecalKind::Paint, yellow, m.width),
                    MarkKind::StopLine => (DecalKind::Paint, white, m.width),
                };
                r.add_decal(&Decal::road((a0 + b0) * 0.5, dir, width, len, kind, color));
            }
        }
        for cw in &city.crosswalks {
            let dir = cw.b - cw.a;
            r.add_decal(&Decal::road(
                (cw.a + cw.b) * 0.5,
                dir,
                cw.width,
                dir.length(),
                DecalKind::Dashed {
                    dash: 0.5,
                    gap: 0.6,
                },
                white,
            ));
        }
        // Signal heads: arm, housing and three lamps each.
        for h in &city.signal_heads {
            let facing = Vec3::new(h.facing.x, 0.0, h.facing.z).normalize_or(Vec3::Z);
            let q = Quat::from_rotation_arc(Vec3::Z, facing);
            let housing =
                Mat4::from_scale_rotation_translation(Vec3::new(0.36, 1.05, 0.28), q, h.head);
            r.add_instance(self.unit_cube, self.mats.housing, housing);
            let top = Vec3::new(h.pole.x, h.head.y + 0.65, h.pole.z);
            let arm_end = Vec3::new(h.head.x, top.y, h.head.z);
            let mid = (top + arm_end) * 0.5;
            let dir = arm_end - top;
            let arm_q = Quat::from_rotation_arc(Vec3::Z, dir.normalize_or(Vec3::Z));
            r.add_instance(
                self.unit_cube,
                self.mats.pole,
                Mat4::from_scale_rotation_translation(
                    Vec3::new(0.12, 0.12, dir.length()),
                    arm_q,
                    mid,
                ),
            );
            let hanger = Mat4::from_scale_rotation_translation(
                Vec3::new(0.06, 0.6, 0.06),
                q,
                h.head + Vec3::Y * 0.8,
            );
            r.add_instance(self.unit_cube, self.mats.pole, hanger);
            let mut lamps = [InstanceId(0); 3];
            for (k, lamp) in lamps.iter_mut().enumerate() {
                let p = h.head + Vec3::Y * (0.32 * (1.0 - k as f32)) + facing * 0.15;
                *lamp = r.add_instance(
                    self.unit_sphere,
                    self.mats.sig[k],
                    Mat4::from_scale_rotation_translation(Vec3::splat(0.22), q, p),
                );
            }
            self.signal_lamps.push(lamps);
            self.signal_glow.push((h.head + facing * 0.4, facing));
        }
    }

    fn build_prop_kinds(&mut self, r: &mut Renderer, a: &mut Assets) {
        let m = &self.mats;
        let cyl =
            |r: &mut Renderer, rad: f32, h: f32| r.create_mesh(&pg::cylinder(rad, 0.0, h, 14));
        let boxm = |r: &mut Renderer, lo: Vec3, hi: Vec3| r.create_mesh(&pg::cuboid(lo, hi));
        let mut lamp: PropVisual = vec![(cyl(r, 0.09, 7.4), m.pole, Mat4::IDENTITY)];
        lamp.push((
            boxm(r, Vec3::new(-0.04, 7.18, -1.7), Vec3::new(0.04, 7.28, 0.0)),
            m.pole,
            Mat4::IDENTITY,
        ));
        lamp.push((
            boxm(
                r,
                Vec3::new(-0.22, 7.05, -1.95),
                Vec3::new(0.22, 7.25, -1.25),
            ),
            m.pole,
            Mat4::IDENTITY,
        ));
        self.prop_kinds.insert(PropKind::StreetLamp, lamp);
        let sign: PropVisual = vec![
            (cyl(r, 0.035, 2.6), m.pole, Mat4::IDENTITY),
            (
                boxm(
                    r,
                    Vec3::new(-0.38, 2.07, -0.01),
                    Vec3::new(0.38, 2.83, 0.01),
                ),
                m.sign,
                Mat4::IDENTITY,
            ),
        ];
        self.prop_kinds.insert(PropKind::TrafficSign, sign);
        self.prop_kinds.insert(
            PropKind::SignalPole,
            vec![(cyl(r, 0.14, 6.3), m.pole, Mat4::IDENTITY)],
        );
        self.prop_kinds.insert(
            PropKind::Bollard,
            vec![(cyl(r, 0.11, 1.0), m.pole, Mat4::IDENTITY)],
        );
        self.prop_kinds.insert(
            PropKind::TrafficCone,
            vec![(
                r.create_mesh(&pg::lathe(
                    &[
                        Vec2::new(0.25, 0.0),
                        Vec2::new(0.25, 0.03),
                        Vec2::new(0.16, 0.05),
                        Vec2::new(0.03, 0.72),
                        Vec2::new(0.0, 0.72),
                    ],
                    16,
                    true,
                )),
                m.orange,
                Mat4::IDENTITY,
            )],
        );
        self.prop_kinds.insert(
            PropKind::Mailbox,
            vec![(
                boxm(r, Vec3::new(-0.28, 0.0, -0.25), Vec3::new(0.28, 1.2, 0.25)),
                m.blue,
                Mat4::IDENTITY,
            )],
        );
        self.prop_kinds.insert(
            PropKind::Newsstand,
            vec![(
                boxm(r, Vec3::new(-0.3, 0.0, -0.25), Vec3::new(0.3, 1.1, 0.25)),
                m.red,
                Mat4::IDENTITY,
            )],
        );
        self.prop_kinds.insert(
            PropKind::Planter,
            vec![
                (
                    boxm(r, Vec3::new(-0.6, 0.0, -0.6), Vec3::new(0.6, 0.7, 0.6)),
                    m.concrete,
                    Mat4::IDENTITY,
                ),
                (
                    r.create_mesh(&pg::sphere(0.55, 12, 8)),
                    m.bush,
                    Mat4::from_translation(Vec3::Y * 0.9),
                ),
            ],
        );
        // CC0 models where available, normalised to the physical size.
        let model = |r: &mut Renderer,
                     a: &mut Assets,
                     id: &str,
                     height: f32,
                     fallback: PropVisual|
         -> PropVisual {
            let (parts, lo, hi) = a.model(r, id, &["variant", "aged"]);
            if parts.is_empty() || hi.y <= lo.y {
                return fallback;
            }
            let s = height / (hi.y - lo.y);
            let c = (lo + hi) * 0.5;
            let xf = Mat4::from_scale(Vec3::splat(s))
                * Mat4::from_translation(Vec3::new(-c.x, -lo.y, -c.z));
            parts
                .into_iter()
                .map(|(mesh, mat)| (mesh, mat, xf))
                .collect()
        };
        let hydrant_fb = vec![(cyl(r, 0.15, 0.8), m.red, Mat4::IDENTITY)];
        let v = model(r, a, "model.fire_hydrant", 0.8, hydrant_fb);
        self.prop_kinds.insert(PropKind::FireHydrant, v);
        let can_fb = vec![(cyl(r, 0.28, 1.0), m.pole, Mat4::IDENTITY)];
        let v = model(r, a, "model.trash_can", 1.0, can_fb);
        self.prop_kinds.insert(PropKind::TrashCan, v);
        let bench_fb = vec![(
            boxm(r, Vec3::new(-0.9, 0.41, -0.22), Vec3::new(0.9, 0.49, 0.22)),
            m.pole,
            Mat4::IDENTITY,
        )];
        let v = model(r, a, "model.bench", 0.85, bench_fb);
        self.prop_kinds.insert(PropKind::Bench, v);
        let barrier_fb = vec![(
            boxm(r, Vec3::new(-0.3, 0.0, -1.5), Vec3::new(0.3, 0.81, 1.5)),
            m.concrete,
            Mat4::IDENTITY,
        )];
        let v = model(r, a, "model.concrete_barrier", 0.81, barrier_fb);
        self.prop_kinds.insert(PropKind::ConcreteBarrier, v);
        // Lamp glass (emissive at night), one per street lamp, attached in `sync`.
    }

    fn class_meshes(&mut self, r: &mut Renderer, game: &mut Game, class: PClass) -> &ClassMeshes {
        if !self.classes.contains_key(&class) {
            let model = game.model(class).clone();
            let body = r.create_mesh(&mesh_from_game(&model.body));
            let parts = model
                .parts
                .iter()
                .map(|p| (r.create_mesh(&mesh_from_game(&p.mesh)), p.paint))
                .collect();
            let tyre = r.create_mesh(&mesh_from_game(&model.tyre));
            let rim = r.create_mesh(&mesh_from_game(&model.rim));
            // Interior: the glass bounds shrunk inwards.
            let mut lo = Vec3::splat(f32::MAX);
            let mut hi = Vec3::splat(f32::MIN);
            for p in model
                .parts
                .iter()
                .filter(|p| p.paint == Paint::Glass)
                .flat_map(|p| p.mesh.positions.iter())
            {
                lo = lo.min(*p);
                hi = hi.max(*p);
            }
            if lo.x > hi.x {
                lo = Vec3::new(-0.5, 0.6, -0.8);
                hi = Vec3::new(0.5, 1.2, 0.8);
            }
            let interior = r.create_mesh(&pg::cuboid(
                Vec3::new(lo.x + 0.12, lo.y - 0.25, lo.z + 0.25),
                Vec3::new(hi.x - 0.12, hi.y - 0.08, hi.z - 0.2),
            ));
            self.classes.insert(
                class,
                ClassMeshes {
                    body,
                    interior,
                    parts,
                    tyre,
                    rim,
                },
            );
        }
        &self.classes[&class]
    }

    fn paint_material(&self, paint: Paint, metallic: bool) -> MaterialId {
        match paint {
            Paint::Body => {
                if metallic {
                    self.mats.paint_metal
                } else {
                    self.mats.paint
                }
            }
            Paint::Trim => self.mats.trim,
            Paint::Glass => self.mats.glass,
            Paint::HeadLamp => self.mats.head,
            Paint::TailLamp => self.mats.tail,
        }
    }

    /// Update every dynamic instance from the scene. Returns local lights and particles.
    pub fn sync(
        &mut self,
        r: &mut Renderer,
        game: &mut Game,
        scene: &SceneFrame,
        camera: Vec3,
    ) -> (Vec<Light>, Vec<Particle>) {
        let mut lights = Vec::new();
        let mut particles = Vec::new();
        let night = self.night;
        for c in self.cars.values_mut() {
            c.seen = false;
        }
        for cv in &scene.cars {
            if !self.cars.contains_key(&cv.id) {
                let (body_mesh, interior_mesh, part_meshes, tyre, rim) = {
                    let cm = self.class_meshes(r, game, cv.class);
                    (cm.body, cm.interior, cm.parts.clone(), cm.tyre, cm.rim)
                };
                let interior = r.add_instance(interior_mesh, self.mats.trim, mat4(&cv.pose));
                let metallic = cv.id % 3 == 0;
                let body = r.add_instance(
                    body_mesh,
                    self.paint_material(Paint::Body, metallic),
                    mat4(&cv.pose),
                );
                let parts = part_meshes
                    .iter()
                    .map(|(m, p)| {
                        (
                            r.add_instance(*m, self.paint_material(*p, metallic), mat4(&cv.pose)),
                            *p,
                        )
                    })
                    .collect();
                let wheels = (0..cv.wheels.len())
                    .map(|_| {
                        (
                            r.add_instance(tyre, self.mats.tyre, Mat4::IDENTITY),
                            r.add_instance(rim, self.mats.rim, Mat4::IDENTITY),
                        )
                    })
                    .collect();
                self.cars.insert(
                    cv.id,
                    CarInst {
                        class: cv.class,
                        body,
                        interior,
                        parts,
                        wheels,
                        own: None,
                        revision: 0,
                        metallic,
                        seen: true,
                    },
                );
            }
            let car = self.cars.get_mut(&cv.id).unwrap();
            car.seen = true;
            let xf = mat4(&cv.pose);
            let tint = srgb_to_linear(cv.color).extend(1.0);
            // Damage: give the car its own meshes and re-upload dented vertices.
            if cv.body_revision != 0 && cv.body_revision != car.revision {
                let vid = gridlock_game::traffic::VehicleId(cv.id);
                if car.own.is_none() && game.damage_of(vid).is_some() {
                    let model = game.model(car.class).clone();
                    let body = r.create_mesh(&mesh_from_game(&model.body));
                    let parts: Vec<MeshId> = model
                        .parts
                        .iter()
                        .map(|p| r.create_mesh(&mesh_from_game(&p.mesh)))
                        .collect();
                    r.remove_instance(car.body);
                    let paint_mat = if car.metallic {
                        self.mats.paint_metal
                    } else {
                        self.mats.paint
                    };
                    car.body = r.add_instance(body, paint_mat, xf);
                    for (k, (inst, paint)) in car.parts.iter_mut().enumerate() {
                        r.remove_instance(*inst);
                        let mat = match paint {
                            Paint::Body => paint_mat,
                            Paint::Trim => self.mats.trim,
                            Paint::Glass => self.mats.glass,
                            Paint::HeadLamp => self.mats.head,
                            Paint::TailLamp => self.mats.tail,
                        };
                        *inst = r.add_instance(parts[k], mat, xf);
                    }
                    car.own = Some((body, parts));
                }
                if let (Some(dmg), Some((body, parts))) = (game.damage_of(vid), &car.own) {
                    r.update_mesh_vertices(*body, &dmg.body.positions, &dmg.body.normals);
                    for (k, p) in dmg.parts.iter().enumerate() {
                        if let Some(m) = parts.get(k) {
                            r.update_mesh_vertices(
                                *m,
                                &p.desc.mesh.positions,
                                &p.desc.mesh.normals,
                            );
                        }
                    }
                }
                car.revision = cv.body_revision;
            }
            r.set_instance_transform(car.body, xf);
            r.set_instance_transform(car.interior, xf);
            r.set_instance_params(car.body, tint, 1.0, true);
            let dmg = game.damage_of(gridlock_game::traffic::VehicleId(cv.id));
            for (k, (inst, paint)) in car.parts.iter().enumerate() {
                let state = dmg.and_then(|d| d.parts.get(k)).map(|p| p.state);
                let pose = dmg.and_then(|d| {
                    if d.parts[k].body.is_some() {
                        d.part_pose(&game.physics, k)
                    } else {
                        None
                    }
                });
                let shattered = state == Some(gridlock_game::physics::deform::PartState::Shattered);
                r.set_instance_transform(*inst, pose.map_or(xf, |p| mat4(&p)));
                let (t, e) = match paint {
                    Paint::Body => (tint, 1.0),
                    Paint::HeadLamp => {
                        let broken = if cv.wheels.is_empty() {
                            false
                        } else if k % 2 == 0 {
                            cv.lamps.left_head_broken
                        } else {
                            cv.lamps.right_head_broken
                        };
                        (
                            Vec4::ONE,
                            if (cv.lamps.low_beam || cv.lamps.high_beam) && !broken {
                                if cv.lamps.high_beam {
                                    1.6
                                } else {
                                    1.0
                                }
                            } else {
                                0.002
                            },
                        )
                    }
                    Paint::TailLamp => {
                        let ind = cv.lamps.left_indicator || cv.lamps.right_indicator;
                        let base = if cv.lamps.low_beam { 0.25 } else { 0.01 };
                        (
                            Vec4::ONE,
                            (base + cv.lamps.brake * 1.0 + if ind { 0.6 } else { 0.0 }).min(1.6),
                        )
                    }
                    _ => (Vec4::ONE, 1.0),
                };
                r.set_instance_params(*inst, t, e, !shattered);
            }
            for (k, (tyre, rim)) in car.wheels.iter().enumerate() {
                let Some(wp) = cv.wheels.get(k) else { continue };
                let detached = cv.wheel_detached.get(k).copied().unwrap_or(false);
                let wxf = mat4(wp);
                r.set_instance_transform(*tyre, wxf);
                r.set_instance_transform(*rim, wxf);
                r.set_instance_params(*tyre, Vec4::ONE, 1.0, !detached);
                r.set_instance_params(*rim, Vec4::ONE, 1.0, !detached);
            }
            // Lights for cars near the camera.
            let dist = cv.pose.p.distance(camera);
            if dist < 140.0 {
                let fwd = cv.pose.q * -Vec3::Z;
                let up = cv.pose.q * Vec3::Y;
                let right = cv.pose.q * Vec3::X;
                let half = gridlock_game::physics::vehicle::presets::VehicleClass::spec(cv.class)
                    .length
                    * 0.5;
                if cv.lamps.low_beam || cv.lamps.high_beam {
                    for side in [-1.0f32, 1.0] {
                        let p = cv.pose.p + fwd * (half - 0.15) + up * 0.68 + right * side * 0.62;
                        let dir =
                            (fwd - up * if cv.lamps.high_beam { 0.02 } else { 0.08 }).normalize();
                        let mut l = Light::spot(
                            p,
                            dir,
                            kelvin_to_rgb(4800.0),
                            if cv.lamps.high_beam { 28000.0 } else { 12000.0 },
                            if cv.lamps.high_beam { 110.0 } else { 55.0 },
                            0.18,
                            if cv.lamps.high_beam { 0.36 } else { 0.55 },
                        );
                        l.cast_shadows = cv.player;
                        l.volumetric = 0.6;
                        lights.push(l);
                    }
                }
                let brake = cv.lamps.brake;
                if brake > 0.05 || cv.lamps.low_beam {
                    let p = cv.pose.p - fwd * (half + 0.2) + up * 0.75;
                    lights.push(Light::point(
                        p,
                        Vec3::new(1.0, 0.02, 0.01),
                        30.0 + brake * 120.0,
                        6.0,
                    ));
                }
                if cv.lamps.reverse {
                    lights.push(Light::point(
                        cv.pose.p - fwd * (half + 0.3) + up * 0.6,
                        Vec3::ONE,
                        60.0,
                        6.0,
                    ));
                }
                if let Some(phase) = cv.lamps.lightbar {
                    let c = if phase % 2 == 0 {
                        Vec3::new(1.0, 0.05, 0.05)
                    } else {
                        Vec3::new(0.05, 0.2, 1.0)
                    };
                    lights.push(Light::point(cv.pose.p + up * 1.7, c, 900.0, 22.0));
                }
                // Tyre smoke when sliding.
                if cv.physics {
                    if let Some(vh) = game
                        .links
                        .get(&gridlock_game::traffic::VehicleId(cv.id))
                        .and_then(|l| match &l.state {
                            gridlock_game::CarState::Physics { vehicle, .. } => Some(*vehicle),
                            _ => None,
                        })
                    {
                        if let Some(v) = game.physics.vehicle(vh) {
                            for w in &v.wheels {
                                if w.in_contact && w.slide_speed > 4.0 && w.water_depth < 0.001 {
                                    let k = ((w.slide_speed - 4.0) / 8.0).min(1.0);
                                    particles.push(Particle {
                                        position: w.contact_point + Vec3::Y * 0.25,
                                        size: 0.4 + k * 0.6,
                                        color: Vec4::new(0.8, 0.8, 0.8, 0.25 * k),
                                        kind: ParticleKind::Smoke,
                                        velocity: Vec3::Y * 0.5,
                                        ..Default::default()
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
        let gone: Vec<u32> = self
            .cars
            .iter()
            .filter(|(_, c)| !c.seen)
            .map(|(k, _)| *k)
            .collect();
        for id in gone {
            if let Some(c) = self.cars.remove(&id) {
                r.remove_instance(c.body);
                r.remove_instance(c.interior);
                for (i, _) in c.parts {
                    r.remove_instance(i);
                }
                for (a, b) in c.wheels {
                    r.remove_instance(a);
                    r.remove_instance(b);
                }
            }
        }
        // Props.
        while self.props.len() < scene.props.len() {
            let pv = &scene.props[self.props.len()];
            let parts = self.prop_kinds.get(&pv.kind).cloned().unwrap_or_default();
            let insts: Vec<(InstanceId, Mat4)> = parts
                .iter()
                .map(|(m, mat, local)| (r.add_instance(*m, *mat, Mat4::IDENTITY), *local))
                .collect();
            if pv.kind == PropKind::StreetLamp {
                let lens = r.add_instance(self.unit_cube, self.mats.lamp_glow, Mat4::IDENTITY);
                self.lamp_glows.push(lens);
                let mut v = insts;
                v.push((
                    lens,
                    Mat4::from_scale_rotation_translation(
                        Vec3::new(0.38, 0.02, 0.6),
                        Quat::IDENTITY,
                        Vec3::new(0.0, 7.04, -1.6),
                    ),
                ));
                self.props.push(v);
            } else {
                self.props.push(insts);
            }
        }
        for (pv, insts) in scene.props.iter().zip(&self.props) {
            let base = mat4(&pv.pose);
            for (inst, local) in insts {
                r.set_instance_transform(*inst, base * *local);
            }
        }
        for g in &self.lamp_glows {
            r.set_instance_params(*g, Vec4::ONE, night.max(0.02), true);
        }
        for (inst, emissive) in &self.facade_instances {
            if *emissive {
                r.set_instance_params(*inst, Vec4::ONE, night, true);
            }
        }
        // Signals.
        let blink = (game.time * 2.0).fract() < 0.5;
        for sv in &scene.signals {
            let Some(lamps) = self.signal_lamps.get(sv.index) else {
                continue;
            };
            let lit = match sv.head.ball {
                gridlock_game::traffic::Lamp::Red => Some(0),
                gridlock_game::traffic::Lamp::Yellow => Some(1),
                gridlock_game::traffic::Lamp::Green => Some(2),
                gridlock_game::traffic::Lamp::Off => None,
            };
            let on = lit.filter(|_| !sv.head.ball_flashing || blink);
            for (k, inst) in lamps.iter().enumerate() {
                r.set_instance_params(
                    *inst,
                    Vec4::ONE,
                    if on == Some(k) { 1.0 } else { 0.015 },
                    true,
                );
            }
            if let (Some(k), Some((p, _))) = (on, self.signal_glow.get(sv.index)) {
                if p.distance(camera) < 160.0 {
                    let c = [
                        Vec3::new(1.0, 0.05, 0.02),
                        Vec3::new(1.0, 0.5, 0.02),
                        Vec3::new(0.1, 1.0, 0.5),
                    ][k];
                    lights.push(Light::point(*p, c, 180.0, 14.0));
                }
            }
        }
        // Pedestrians: torso, legs, head.
        while self.peds.len() < scene.peds.len() {
            let (torso, legs, head) = self.ped_meshes;
            self.peds.push([
                r.add_instance(torso, self.mats.ped_shirt, Mat4::IDENTITY),
                r.add_instance(legs, self.mats.ped_dark, Mat4::IDENTITY),
                r.add_instance(head, self.mats.ped_skin, Mat4::IDENTITY),
            ]);
        }
        for (i, insts) in self.peds.iter().enumerate() {
            let Some(p) = scene.peds.get(i) else {
                for inst in insts {
                    r.set_instance_params(*inst, Vec4::ONE, 1.0, false);
                }
                continue;
            };
            let h = p.height.max(1.4);
            let q = gridlock_game::coords::rotation_for_heading(p.heading as f64);
            let (q, base) = if p.down {
                (
                    q * Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
                    p.pos + Vec3::Y * 0.2,
                )
            } else {
                (q, p.pos + Vec3::Y * CURB_HEIGHT.min(p.pos.y.max(0.0)))
            };
            let bob = if p.walking {
                (p.stride * std::f32::consts::PI).sin().abs() * 0.04
            } else {
                0.0
            };
            let legs = Mat4::from_rotation_translation(q, base + Vec3::Y * bob)
                * Mat4::from_scale(Vec3::new(1.0, h / 1.75, 1.0));
            let torso =
                Mat4::from_rotation_translation(q, base + q * Vec3::Y * (0.85 * h / 1.75 + bob))
                    * Mat4::from_scale(Vec3::new(1.0, h / 1.75, 0.8));
            let head =
                Mat4::from_rotation_translation(q, base + q * Vec3::Y * (1.55 * h / 1.75 + bob));
            r.set_instance_transform(insts[1], legs);
            r.set_instance_transform(insts[0], torso);
            r.set_instance_transform(insts[2], head);
            r.set_instance_params(insts[0], srgb_to_linear(p.shirt).extend(1.0), 1.0, true);
            r.set_instance_params(insts[1], Vec4::ONE, 1.0, true);
            r.set_instance_params(insts[2], Vec4::ONE, 1.0, true);
        }
        // Street lamps near the camera.
        if night > 0.05 {
            let mut near: Vec<(f32, Vec3)> = self
                .lamp_positions
                .iter()
                .map(|p| (p.distance_squared(camera), *p))
                .filter(|(d, _)| *d < 260.0 * 260.0)
                .collect();
            near.sort_by(|a, b| a.0.total_cmp(&b.0));
            for (_, p) in near.into_iter().take(160) {
                let mut l = Light::spot(
                    p - Vec3::Y * 0.05,
                    Vec3::NEG_Y,
                    kelvin_to_rgb(3300.0),
                    4200.0 * night,
                    30.0,
                    0.5,
                    1.15,
                );
                l.source_radius = 0.2;
                l.volumetric = 0.8;
                lights.push(l);
            }
        }
        // Spray.
        for (pos, size, life) in &scene.spray {
            particles.push(Particle {
                position: *pos,
                size: size * 2.5,
                color: Vec4::new(0.8, 0.85, 0.9, 0.35 * life),
                kind: ParticleKind::Splash,
                ..Default::default()
            });
        }
        (lights, particles)
    }
}

fn pbr_material(p: &PbrSet, uv: UvMode, scale: f32) -> Material {
    let repeats = match uv {
        UvMode::WorldXZ | UvMode::Triplanar => Vec2::ONE / p.tile * scale,
        _ => Vec2::ONE * scale,
    };
    Material {
        albedo_tex: Some(p.albedo),
        normal_tex: p.normal,
        roughness_tex: p.arm,
        ao_tex: p.arm,
        metallic_tex: None,
        packed_metal_rough: p.arm.is_some(),
        emissive_tex: p.emission,
        base_color: Vec4::ONE,
        roughness: 1.0,
        metallic: 1.0,
        uv_mode: uv,
        uv_scale: repeats,
        ..Default::default()
    }
}

fn quad_xz(center: Vec3, half: Vec2) -> MeshData {
    let mut m = MeshData::default();
    let corners = [
        Vec3::new(-half.x, 0.0, -half.y),
        Vec3::new(half.x, 0.0, -half.y),
        Vec3::new(half.x, 0.0, half.y),
        Vec3::new(-half.x, 0.0, half.y),
    ];
    for c in corners {
        let p = center + c;
        m.positions.push(p);
        m.normals.push(Vec3::Y);
        m.uvs.push(Vec2::new(p.x, p.z));
    }
    // Counter-clockwise seen from above (+Y).
    m.indices.extend_from_slice(&[0, 2, 1, 0, 3, 2]);
    m.compute_tangents();
    m
}

fn fan_top(m: &mut MeshData, outline: &[Vec2], y: f32) {
    let base = m.positions.len() as u32;
    let c = outline.iter().copied().sum::<Vec2>() / outline.len() as f32;
    m.positions.push(Vec3::new(c.x, y, c.y));
    m.normals.push(Vec3::Y);
    m.uvs.push(c);
    for p in outline {
        m.positions.push(Vec3::new(p.x, y, p.y));
        m.normals.push(Vec3::Y);
        m.uvs.push(*p);
    }
    let n = outline.len() as u32;
    for i in 0..n {
        let a = base + 1 + i;
        let b = base + 1 + (i + 1) % n;
        // Make the triangle face +Y regardless of outline orientation.
        let (pa, pb) = (outline[i as usize], outline[((i + 1) % n) as usize]);
        let cross = (pa.x - c.x) * (pb.y - c.y) - (pa.y - c.y) * (pb.x - c.x);
        if cross < 0.0 {
            m.indices.extend_from_slice(&[base, a, b]);
        } else {
            m.indices.extend_from_slice(&[base, b, a]);
        }
    }
}

fn kerb_walls(m: &mut MeshData, outline: &[Vec2], h: f32) {
    let n = outline.len();
    let c = outline.iter().copied().sum::<Vec2>() / n as f32;
    for i in 0..n {
        let (a, b) = (outline[i], outline[(i + 1) % n]);
        let edge = b - a;
        let mut out = Vec2::new(edge.y, -edge.x).normalize_or_zero();
        if out.dot((a + b) * 0.5 - c) < 0.0 {
            out = -out;
        }
        let nrm = Vec3::new(out.x, 0.0, out.y);
        let base = m.positions.len() as u32;
        let len = edge.length();
        for (p, y, u) in [(a, 0.0, 0.0), (b, 0.0, len), (b, h, len), (a, h, 0.0)] {
            m.positions.push(Vec3::new(p.x, y, p.y));
            m.normals.push(nrm);
            m.uvs.push(Vec2::new(u, y));
        }
        let face = Vec3::new(edge.x, 0.0, edge.y).cross(Vec3::Y);
        if face.dot(nrm) >= 0.0 {
            m.indices
                .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        } else {
            m.indices
                .extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
        }
    }
}

/// Remove faces whose normal is vertical (roof/floor) from a cuboid.
fn strip_horizontal(m: &mut MeshData) {
    let mut keep = Vec::with_capacity(m.indices.len());
    for t in m.indices.chunks_exact(3) {
        if m.normals[t[0] as usize].y.abs() < 0.5 {
            keep.extend_from_slice(t);
        }
    }
    m.indices = keep;
}

/// Split a polyline into straight runs (merging collinear points).
fn straight_runs(pts: &[Vec3]) -> Vec<(Vec3, Vec3)> {
    let mut out = Vec::new();
    let mut start = pts[0];
    let mut dir = (pts[1] - pts[0]).normalize_or_zero();
    for k in 1..pts.len() {
        let next = pts.get(k + 1);
        let d = next.map(|n| (*n - pts[k]).normalize_or_zero());
        let bend = d.is_some_and(|d| d.dot(dir) < 0.9995);
        if next.is_none() || bend {
            out.push((start, pts[k]));
            start = pts[k];
            if let Some(d) = d {
                dir = d;
            }
        }
    }
    out
}
