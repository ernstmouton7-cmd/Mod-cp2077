//! Chase / hood / cockpit / orbit cameras and the game->renderer environment mapping.

use glam::{Quat, Vec2, Vec3};
use gridlock_game::physics::Pose;
use gridlock_game::EnvironmentView;
use gridlock_render::{Camera, Environment, Fog};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CamMode {
    Chase,
    FarChase,
    Hood,
    Cockpit,
    Orbit,
    /// Fixed side view (screenshots / inspection).
    Side,
}

impl CamMode {
    pub fn next(self) -> Self {
        match self {
            CamMode::Chase => CamMode::FarChase,
            CamMode::FarChase => CamMode::Hood,
            CamMode::Hood => CamMode::Cockpit,
            CamMode::Cockpit => CamMode::Orbit,
            CamMode::Orbit => CamMode::Chase,
            CamMode::Side => CamMode::Chase,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            CamMode::Chase => "Chase",
            CamMode::FarChase => "Far chase",
            CamMode::Hood => "Hood",
            CamMode::Cockpit => "Cockpit",
            CamMode::Orbit => "Orbit",
            CamMode::Side => "Side",
        }
    }
}

pub struct CameraRig {
    pub mode: CamMode,
    pos: Vec3,
    yaw: f32,
    /// Right-stick look offsets (radians).
    look: Vec2,
    pub look_back: bool,
    initialised: bool,
    pub fov: f32,
}

impl Default for CameraRig {
    fn default() -> Self {
        Self {
            mode: CamMode::Chase,
            pos: Vec3::ZERO,
            yaw: 0.0,
            look: Vec2::ZERO,
            look_back: false,
            initialised: false,
            fov: 62.0,
        }
    }
}

impl CameraRig {
    pub fn update(
        &mut self,
        dt: f32,
        car: &Pose,
        speed: f32,
        look_input: Vec2,
        length: f32,
    ) -> Camera {
        let fwd = car.q * -Vec3::Z;
        let car_yaw = fwd.x.atan2(-fwd.z);
        // Look around with the right stick; springs back when released.
        let target_look = Vec2::new(look_input.x * 2.4, look_input.y * 0.6);
        self.look += (target_look - self.look) * (1.0 - (-dt * 8.0).exp());
        let back = if self.look_back {
            std::f32::consts::PI
        } else {
            0.0
        };
        let cut = !self.initialised;
        match self.mode {
            CamMode::Chase | CamMode::FarChase | CamMode::Orbit => {
                let (dist, height) = match self.mode {
                    CamMode::Chase => (length * 1.15 + 2.6, 1.55 + length * 0.12),
                    CamMode::FarChase => (length * 1.8 + 5.0, 2.6 + length * 0.2),
                    _ => (length * 1.6 + 4.0, 1.8),
                };
                // Yaw lags behind the car (more at speed) so turns read well.
                let mut want = car_yaw + back + self.look.x;
                if self.mode == CamMode::Orbit {
                    want = self.yaw + look_input.x * dt * 2.0;
                }
                let mut d = want - self.yaw;
                d = (d + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
                    - std::f32::consts::PI;
                let k = 1.0 - (-dt * (4.0 + speed.abs() * 0.08)).exp();
                self.yaw = if cut { want } else { self.yaw + d * k };
                let dir = Vec3::new(self.yaw.sin(), 0.0, -self.yaw.cos());
                let target = car.p + Vec3::Y * (1.1 + self.look.y);
                let desired = car.p - dir * dist + Vec3::Y * height;
                let follow = 1.0 - (-dt * 10.0).exp();
                self.pos = if cut {
                    desired
                } else {
                    self.pos + (desired - self.pos) * follow
                };
                self.initialised = true;
                let fov = self.fov + (speed.abs() * 3.6 / 200.0).min(1.0) * 10.0;
                let mut c = Camera::look_at(self.pos, target, fov);
                c.cut = cut;
                c
            }
            CamMode::Side => {
                let p = car.transform_point(Vec3::new(length * 0.9 + 2.5, 1.2, -length * 0.25));
                let target = car.transform_point(Vec3::new(0.0, 0.6, 0.0));
                let mut c = Camera::look_at(p, target, 45.0);
                c.cut = cut;
                self.initialised = true;
                c
            }
            CamMode::Hood | CamMode::Cockpit => {
                let local = if self.mode == CamMode::Hood {
                    Vec3::new(0.0, 1.35, -length * 0.25)
                } else {
                    Vec3::new(-0.36, 1.18, -0.05)
                };
                let p = car.transform_point(local);
                let q = car.q
                    * Quat::from_rotation_y(-self.look.x - back)
                    * Quat::from_rotation_x(self.look.y * 0.5);
                self.pos = p;
                self.yaw = car_yaw;
                self.initialised = true;
                let mut c = Camera::look_at(
                    p,
                    p + q * -Vec3::Z,
                    if self.mode == CamMode::Cockpit {
                        72.0
                    } else {
                        66.0
                    },
                );
                c.near = 0.05;
                c.cut = cut;
                c
            }
        }
    }

    pub fn reset(&mut self) {
        self.initialised = false;
    }
}

/// Map the simulation's time of day and weather onto renderer lighting.
pub fn environment(e: &EnvironmentView, time: f32) -> Environment {
    let sun = e.sun_dir;
    let day = e.daylight.clamp(0.0, 1.0);
    let overcast = (e.rain * 1.3 + e.fog * 0.5).min(1.0);
    let night = 1.0 - day;
    Environment {
        sun_direction: sun,
        sun_illuminance: 120_000.0 * (1.0 - overcast * 0.85),
        moon_direction: Vec3::new(-sun.x, (-sun.y).max(0.2), -sun.z).normalize(),
        moon_illuminance: 0.25 * night,
        cloud_cover: (0.25 + overcast * 0.75).min(1.0),
        turbidity: 1.8 + overcast,
        night_glow: Vec3::new(0.06, 0.035, 0.02) * (0.6 + night * 0.6),
        wetness: e.wetness,
        puddles: (e.wetness * 0.7).min(0.8),
        rain: e.rain,
        fog: Fog {
            density: 0.0006 + e.fog * 0.02 + e.rain * 0.004,
            height_falloff: 0.025,
            anisotropy: 0.6,
            ..Default::default()
        },
        exposure_compensation: if night > 0.6 { -0.2 } else { 0.0 },
        time,
        ..Default::default()
    }
}
