//! A simulated vehicle: lane-mode kinematics or rigid-body physics.

use super::damage::DamageModel;
use super::dynamics::{Assists, DriveInputs, VehicleDynamics};
use super::lights::{SirenMode, VehicleLights};
use super::powertrain::{Powertrain, TransmissionMode};
use super::specs::VehicleSpec;
use crate::ai::driver::AIDriver;
use crate::ids::{ConnectorId, SegRef, VehicleId};
use crate::math::scalar::clamp;
use crate::road::Network;

/// What the vehicle is for (affects spawning, law and AI behaviour).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VehicleRole {
    Civilian,
    Player,
    Police,
    Ambulance,
    Taxi,
}

/// `Lane`: kinematic, glued to a lane/connector (cheap, used by most AI).
/// `Physics`: full rigid-body dynamics (player, crashed cars, chasers).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MotionMode {
    Lane,
    Physics,
}

/// A vehicle in the world. Lane-mode state (`seg`, `s`, `lat`) drives the
/// pose kinematically; in physics mode the pose comes from `dynamics` (or,
/// for `external` vehicles, from the host via `World::set_external_pose`).
#[derive(Clone, Debug)]
pub struct Vehicle {
    pub id: VehicleId,
    pub spec: &'static VehicleSpec,
    pub color: u32,
    pub role: VehicleRole,
    pub length: f64,
    pub width: f64,
    pub half_len: f64,
    pub half_wid: f64,
    pub mode: MotionMode,
    /// World pose (metres; heading radians CCW from +x).
    pub x: f64,
    pub y: f64,
    pub heading: f64,
    /// Signed forward speed (m/s).
    pub speed: f64,
    /// Longitudinal / lateral acceleration (m/s²).
    pub accel: f64,
    pub lat_accel: f64,
    pub yaw_rate: f64,
    /// Front wheel angle for rendering (rad, + = left).
    pub wheel_angle: f64,
    /// Accumulated wheel rotation for rendering (rad).
    pub wheel_spin: f64,
    // Lane-mode state.
    pub seg: Option<SegRef>,
    /// Segment the vehicle came from (its tail may still be there).
    pub prev_seg: Option<SegRef>,
    pub s: f64,
    /// Lateral offset from the segment centreline (m, + = left).
    pub lat: f64,
    pub lat_vel: f64,
    // Physics-mode state.
    pub dynamics: VehicleDynamics,
    pub pt: Powertrain,
    pub inputs: DriveInputs,
    /// Desired front-wheel angle (rad, + = left) set by the controller.
    pub steer_target: f64,
    pub assists: Assists,
    pub lights: VehicleLights,
    pub damage: DamageModel,
    pub is_player: bool,
    /// Always true while the vehicle is in the world.
    pub alive: bool,
    /// Disabled by damage: no engine, hazards on.
    pub wrecked: bool,
    pub stopped_time: f64,
    pub odometer: f64,
    /// World time of the last collision and the other party.
    pub last_hit_time: f64,
    pub last_hit_by: Option<VehicleId>,
    /// Seconds since entering physics mode.
    pub physics_time: f64,
    /// Connector this vehicle intends to take next. Read by other drivers.
    pub intent_connector: Option<ConnectorId>,
    /// Seconds counting toward despawn.
    pub despawn_timer: f64,
    /// Engine rpm/gear estimate in lane mode (audio).
    pub rpm_estimate: f64,
    pub gear_estimate: usize,
    /// Host-simulated body: the World does not integrate its dynamics or run
    /// 2D collision response for it; the host supplies the pose each step.
    pub external: bool,
    /// The AI brain (None for the player, or while the brain is updating).
    pub(crate) driver: Option<Box<AIDriver>>,
    pub(crate) has_ai: bool,
}

impl Vehicle {
    pub fn new(id: VehicleId, spec: &'static VehicleSpec, color: u32, role: VehicleRole) -> Self {
        let mut assists = Assists::default();
        if spec.emergency {
            assists.esc = true;
        }
        Vehicle {
            id,
            spec,
            color,
            role,
            length: spec.length,
            width: spec.width,
            half_len: spec.length / 2.0,
            half_wid: spec.width / 2.0,
            mode: MotionMode::Lane,
            x: 0.0,
            y: 0.0,
            heading: 0.0,
            speed: 0.0,
            accel: 0.0,
            lat_accel: 0.0,
            yaw_rate: 0.0,
            wheel_angle: 0.0,
            wheel_spin: 0.0,
            seg: None,
            prev_seg: None,
            s: 0.0,
            lat: 0.0,
            lat_vel: 0.0,
            dynamics: VehicleDynamics::new(spec),
            pt: Powertrain::new(spec),
            inputs: DriveInputs::default(),
            steer_target: 0.0,
            assists,
            lights: VehicleLights::new(id.0 as f64),
            damage: DamageModel::default(),
            is_player: role == VehicleRole::Player,
            alive: true,
            wrecked: false,
            stopped_time: 0.0,
            odometer: 0.0,
            last_hit_time: -100.0,
            last_hit_by: None,
            physics_time: 0.0,
            intent_connector: None,
            despawn_timer: 0.0,
            rpm_estimate: 0.0,
            gear_estimate: 1,
            external: false,
            driver: None,
            has_ai: false,
        }
    }

    /// The AI driver, if this is an AI vehicle.
    pub fn driver(&self) -> Option<&AIDriver> {
        self.driver.as_deref()
    }

    pub fn driver_mut(&mut self) -> Option<&mut AIDriver> {
        self.driver.as_deref_mut()
    }

    pub fn is_ai(&self) -> bool {
        self.has_ai
    }

    pub fn is_emergency(&self) -> bool {
        self.role == VehicleRole::Police || self.role == VehicleRole::Ambulance
    }

    pub fn siren_on(&self) -> bool {
        self.lights.siren != SirenMode::Off
    }

    pub fn abs_speed(&self) -> f64 {
        if self.mode == MotionMode::Physics {
            self.dynamics.abs_speed()
        } else {
            self.speed.abs()
        }
    }

    /// World velocity x.
    pub fn vx(&self) -> f64 {
        if self.mode == MotionMode::Physics {
            self.dynamics.vx()
        } else {
            self.heading.cos() * self.speed
        }
    }

    /// World velocity y.
    pub fn vy(&self) -> f64 {
        if self.mode == MotionMode::Physics {
            self.dynamics.vy()
        } else {
            self.heading.sin() * self.speed
        }
    }

    /// Place on a segment in lane mode.
    pub fn place_on_segment(&mut self, net: &Network, seg: SegRef, s: f64, speed: f64) {
        self.mode = MotionMode::Lane;
        self.seg = Some(seg);
        self.s = s;
        self.lat = 0.0;
        self.lat_vel = 0.0;
        self.speed = speed;
        self.sync_lane_pose(net);
    }

    /// Update the world pose from lane coordinates.
    pub fn sync_lane_pose(&mut self, net: &Network) {
        let Some(seg) = self.seg else { return };
        let len = net.seg_length(seg);
        let t = net.seg_path(seg).sample(clamp(self.s, 0.0, len));
        let mut x = t.x;
        let mut y = t.y;
        if self.s > len {
            x += t.heading.cos() * (self.s - len);
            y += t.heading.sin() * (self.s - len);
        }
        let nx = -t.heading.sin();
        let ny = t.heading.cos();
        self.x = x + nx * self.lat;
        self.y = y + ny * self.lat;
        let v = self.speed.abs().max(1.0);
        let new_heading = t.heading + self.lat_vel.atan2(v);
        let mut d = new_heading - self.heading;
        while d > std::f64::consts::PI {
            d -= std::f64::consts::PI * 2.0;
        }
        while d < -std::f64::consts::PI {
            d += std::f64::consts::PI * 2.0;
        }
        self.heading += d;
    }

    /// Switch to rigid-body simulation, keeping the current motion.
    pub fn enter_physics(&mut self) {
        if self.mode == MotionMode::Physics {
            return;
        }
        self.dynamics
            .reset(self.x, self.y, self.heading, self.speed);
        self.dynamics.v = self.lat_vel;
        self.mode = MotionMode::Physics;
        self.physics_time = 0.0;
        self.pt.mode = TransmissionMode::Automatic;
        if self.pt.gear <= 0 {
            self.pt.gear = 1;
        }
        self.seg = None;
    }

    /// Return to lane mode at the given lane coordinates.
    pub fn enter_lane(&mut self, net: &Network, seg: SegRef, s: f64, lat: f64) {
        let sp = self.dynamics.u;
        self.mode = MotionMode::Lane;
        self.seg = Some(seg);
        self.s = s;
        self.lat = lat;
        self.lat_vel = 0.0;
        self.speed = sp.max(0.0);
        self.sync_lane_pose(net);
    }

    /// Copy the pose from the dynamics body after a physics step.
    pub fn sync_from_dynamics(&mut self) {
        let d = &self.dynamics;
        self.x = d.x;
        self.y = d.y;
        self.heading = d.yaw;
        self.speed = d.u;
        self.accel = d.ax;
        self.lat_accel = d.ay;
        self.yaw_rate = d.r;
        self.wheel_angle = d.steer_angle;
    }

    /// Corners of the bounding box in world space: front-left, front-right,
    /// rear-right, rear-left... as `[x0, y0, x1, y1, ...]` (same order as the TS).
    pub fn corners(&self) -> [f64; 8] {
        let c = self.heading.cos();
        let s = self.heading.sin();
        let hl = self.half_len;
        let hw = self.half_wid;
        let mut out = [0.0; 8];
        let mut k = 0;
        for sx in [1.0, -1.0] {
            for sy in [1.0, -1.0] {
                let lx = sx * hl;
                let ly = sx * sy * hw;
                out[k] = self.x + lx * c - ly * s;
                out[k + 1] = self.y + lx * s + ly * c;
                k += 2;
            }
        }
        out
    }
}
