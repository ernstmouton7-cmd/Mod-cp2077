//! A renderer-agnostic snapshot of everything dynamic, built once per rendered frame.

use crate::coords::*;
use crate::game::{CarState, Game};
use glam::{Quat, Vec3};
use gridlock_physics::fracture::Fragment;
use gridlock_physics::props::PropKind;
use gridlock_physics::vehicle::presets::VehicleClass as PClass;
use gridlock_physics::Pose;
use gridlock_traffic::{HeadState, PedState, VehicleRole};

#[derive(Clone, Copy, Debug, Default)]
pub struct LampState {
    pub low_beam: bool,
    pub high_beam: bool,
    /// 0..1 brake lamp intensity.
    pub brake: f32,
    pub reverse: bool,
    pub left_indicator: bool,
    pub right_indicator: bool,
    /// Emergency light bar phase (None = off).
    pub lightbar: Option<u8>,
    pub left_head_broken: bool,
    pub right_head_broken: bool,
}

#[derive(Clone, Debug)]
pub struct CarView {
    pub id: u32,
    pub class: PClass,
    pub color: u32,
    pub pose: Pose,
    pub prev_pose: Pose,
    /// Wheel centre poses (tyre axis along local X).
    pub wheels: Vec<Pose>,
    pub wheel_detached: Vec<bool>,
    pub lamps: LampState,
    pub player: bool,
    pub police: bool,
    /// Simulated with full physics (dented meshes live in the game's damage model).
    pub physics: bool,
    /// Body mesh revision (re-upload deformed vertices when it changes).
    pub body_revision: u64,
    pub speed: f32,
    pub rpm: f32,
    pub horn: bool,
    pub siren: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct SignalView {
    pub index: usize,
    pub head: HeadState,
}

#[derive(Clone, Copy, Debug)]
pub struct PedView {
    pub pos: Vec3,
    pub heading: f32,
    pub stride: f32,
    pub shirt: u32,
    pub height: f32,
    pub down: bool,
    pub walking: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct PropView {
    pub kind: PropKind,
    pub pose: Pose,
    pub broken: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct EnvironmentView {
    pub hour: f32,
    pub daylight: f32,
    pub rain: f32,
    pub fog: f32,
    pub wetness: f32,
    /// Sun direction (toward the sun), world space.
    pub sun_dir: Vec3,
}

#[derive(Clone, Debug)]
pub struct SceneFrame {
    pub cars: Vec<CarView>,
    pub signals: Vec<SignalView>,
    pub peds: Vec<PedView>,
    pub props: Vec<PropView>,
    pub fragments: Vec<(Pose, usize)>,
    pub spray: Vec<(Vec3, f32, f32)>,
    pub env: EnvironmentView,
    pub player_pose: Option<Pose>,
}

/// Rough sun direction for a city at ~40°N in early autumn.
pub fn sun_direction(hour: f32) -> Vec3 {
    let t = (hour - 6.0) / 12.0 * std::f32::consts::PI; // 0 at 6am, pi at 6pm
    let elevation = t.sin() * 0.85 - 0.1;
    let azimuth = t; // east -> south -> west
    let x = azimuth.cos();
    let z = azimuth.sin() * 0.6;
    Vec3::new(x * elevation.cos(), elevation, z).normalize()
}

impl Game {
    pub fn scene(&self) -> SceneFrame {
        let player = self.traffic.player_id();
        let mut cars = Vec::with_capacity(self.links.len());
        for v in self.traffic.vehicles() {
            let Some(link) = self.links.get(&v.id) else {
                continue;
            };
            let lights = &v.lights;
            let lamps = LampState {
                low_beam: lights.low_beam_on(),
                high_beam: lights.high_beam_on(),
                brake: lights.brake as f32,
                reverse: lights.reverse,
                left_indicator: lights.left_lamp_on(),
                right_indicator: lights.right_lamp_on(),
                lightbar: if lights.lightbar {
                    Some(lights.lightbar_phase())
                } else {
                    None
                },
                left_head_broken: lights.left_headlight_broken,
                right_head_broken: lights.right_headlight_broken,
            };
            let desc = link.class.desc();
            let (pose, prev, wheels, detached, physics, rev, speed, rpm) = match &link.state {
                CarState::Physics { vehicle, damage } => {
                    let Some(pv) = self.physics.vehicle(*vehicle) else {
                        continue;
                    };
                    let Some(b) = self.physics.body(pv.chassis) else {
                        continue;
                    };
                    let wheels: Vec<Pose> = (0..pv.wheels.len())
                        .map(|i| pv.wheel_pose(i, &b.pose))
                        .collect();
                    let det = pv.wheels.iter().map(|w| w.detached).collect();
                    (
                        b.pose,
                        b.prev_pose,
                        wheels,
                        det,
                        true,
                        damage.body.revision,
                        pv.telemetry.speed,
                        pv.powertrain.rpm(),
                    )
                }
                CarState::Lane { body } => {
                    let Some(b) = self.physics.body(*body) else {
                        continue;
                    };
                    let spin = v.wheel_spin as f32;
                    let steer = v.wheel_angle as f32;
                    let wheels: Vec<Pose> = desc
                        .wheels
                        .iter()
                        .map(|w| {
                            let center = b.pose.transform_point(
                                w.hardpoint - Vec3::Y * (w.hardpoint.y - w.radius),
                            );
                            let st = if w.steer != 0.0 { steer } else { 0.0 };
                            Pose::new(
                                center,
                                b.pose.q * Quat::from_rotation_y(st) * Quat::from_rotation_x(-spin),
                            )
                        })
                        .collect();
                    (
                        b.pose,
                        b.prev_pose,
                        wheels,
                        vec![false; 4],
                        false,
                        0,
                        v.speed as f32,
                        v.rpm_estimate as f32,
                    )
                }
            };
            cars.push(CarView {
                id: v.id.0,
                class: link.class,
                color: v.color,
                pose,
                prev_pose: prev,
                wheels,
                wheel_detached: detached,
                lamps,
                player: Some(v.id) == player,
                police: v.role == VehicleRole::Police,
                physics,
                body_revision: rev,
                speed,
                rpm,
                horn: lights.horn,
                siren: lights.siren != gridlock_traffic::vehicle::SirenMode::Off,
            });
        }
        let mut signals = Vec::with_capacity(self.city.signal_heads.len());
        for (i, h) in self.city.signal_heads.iter().enumerate() {
            if let Some(sig) = self.traffic.signal_at(gridlock_traffic::NodeId(h.node)) {
                signals.push(SignalView {
                    index: i,
                    head: sig.head(gridlock_traffic::ApproachId(h.approach)),
                });
            }
        }
        let peds = self
            .traffic
            .peds()
            .peds
            .iter()
            .map(|p| PedView {
                pos: to_world(p.x, p.y, 0.0),
                heading: p.heading as f32,
                stride: p.stride as f32,
                shirt: p.shirt,
                height: p.height as f32,
                down: p.state == PedState::Down,
                walking: matches!(p.state, PedState::Crossing | PedState::Leaving),
            })
            .collect();
        let props = self
            .props
            .props
            .iter()
            .filter_map(|p| {
                self.physics.body(p.body).map(|b| PropView {
                    kind: p.kind,
                    pose: b.pose,
                    broken: p.broken,
                })
            })
            .collect();
        let fragments = self
            .destruction
            .fragments
            .iter()
            .enumerate()
            .filter_map(|(i, f): (usize, &Fragment)| self.physics.body(f.body).map(|b| (b.pose, i)))
            .collect();
        let spray = self
            .spray
            .particles
            .iter()
            .map(|p| (p.position, p.size, p.life / p.max_life))
            .collect();
        let e = self.traffic.env();
        let env = EnvironmentView {
            hour: e.hour as f32,
            daylight: e.daylight as f32,
            rain: e.rain as f32,
            fog: e.fog as f32,
            wetness: e.wetness as f32,
            sun_dir: sun_direction(e.hour as f32),
        };
        let player_pose = cars.iter().find(|c| c.player).map(|c| c.pose);
        SceneFrame {
            cars,
            signals,
            peds,
            props,
            fragments,
            spray,
            env,
            player_pose,
        }
    }
}
