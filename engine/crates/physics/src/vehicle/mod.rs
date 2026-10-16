//! Vehicle dynamics on top of the rigid-body solver.
//!
//! The chassis is an ordinary dynamic rigid body. Each wheel sphere-casts down from its
//! suspension hardpoint every solver substep; suspension (spring, bump/rebound damping,
//! bump stop, anti-roll bars), combined-slip tyre forces and drivetrain torques are applied
//! as impulses before the contact solve. Tyre impulses are clamped so friction can never
//! reverse a contact patch's sliding velocity within a substep, which keeps the model
//! stable at standstill and on slopes without any low-speed hacks.
//!
//! Chassis frame: +X right, +Y up, -Z forward. The body origin is on the ground plane at the
//! middle of the wheelbase when the car sits at its static ride height.

pub mod powertrain;
pub mod presets;
pub mod tyre;

use crate::arena::{Arena, Handle};
use crate::body::{layers, BodyDesc, BodyHandle, ColliderDesc, Surface};
use crate::math::Pose;
use crate::query::QueryFilter;
use crate::shape::{MassProperties, Shape};
use crate::world::{PhysicsWorld, SubstepContext};
use glam::{Mat3, Quat, Vec3};
use powertrain::*;
use tyre::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VehicleHandle(pub Handle);

#[derive(Clone, Debug)]
pub struct WheelDesc {
    /// Suspension hardpoint in the chassis frame: the wheel centre at full bump.
    pub hardpoint: Vec3,
    pub radius: f32,
    pub width: f32,
    /// Wheel + tyre + brake rotating inertia (kg·m²).
    pub inertia: f32,
    /// Suspension travel from full bump to full droop (m).
    pub travel: f32,
    pub spring: f32,
    pub bump_damping: f32,
    pub rebound_damping: f32,
    /// Multiplier on the steering angle (1 front, 0 rear; negative for rear-steer).
    pub steer: f32,
    pub driven: bool,
    /// Share of the total foot-brake torque.
    pub brake_share: f32,
    pub handbrake: bool,
    pub tyre: TyreDesc,
    /// Static toe angle (rad, positive = toe-in on this side).
    pub toe: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct AntiRollBar {
    pub left: usize,
    pub right: usize,
    /// N/m of compression difference.
    pub stiffness: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Aero {
    /// Drag coefficient × frontal area (m²).
    pub drag_area: f32,
    /// Downforce coefficient × area at the front and rear axles (m², positive = down).
    pub lift_front: f32,
    pub lift_rear: f32,
}

#[derive(Clone, Debug)]
pub struct VehicleDesc {
    pub name: String,
    pub mass: f32,
    /// Centre of mass in the chassis frame.
    pub com: Vec3,
    /// Inertia about the centre of mass (chassis frame axes).
    pub inertia: Vec3,
    /// Chassis collision hull (chassis frame). Wheels are casts and need no collider.
    pub hull: Vec<Vec3>,
    pub wheels: Vec<WheelDesc>,
    pub anti_roll: Vec<AntiRollBar>,
    pub engine: EngineDesc,
    pub gearbox: GearboxDesc,
    pub layout: DriveLayout,
    pub front_diff: Differential,
    pub rear_diff: Differential,
    /// Total foot-brake torque (N·m) at full pedal.
    pub brake_torque: f32,
    pub handbrake_torque: f32,
    pub max_steer: f32,
    /// Steering rack speed (rad/s at the wheels).
    pub steer_speed: f32,
    pub aero: Aero,
    pub wheelbase: f32,
    pub track: f32,
    /// Extents for gameplay/rendering (length, width, height).
    pub size: Vec3,
}

/// Driver inputs. All axes 0..1 except `steer` (-1 right .. +1 left).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct VehicleControls {
    pub throttle: f32,
    pub brake: f32,
    pub steer: f32,
    pub handbrake: f32,
    /// Clutch pedal (1 = pressed / disengaged). Ignored with auto-clutch.
    pub clutch: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Assists {
    pub abs: bool,
    pub tcs: bool,
    pub esc: bool,
    /// 0..1: speed-sensitive steering range and counter-steer help (gamepads).
    pub steering: f32,
    /// Manual gearbox without a clutch pedal.
    pub auto_clutch: bool,
}

impl Default for Assists {
    fn default() -> Self {
        Self {
            abs: true,
            tcs: true,
            esc: true,
            steering: 0.5,
            auto_clutch: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct WheelState {
    pub in_contact: bool,
    /// Suspension compression from full droop (m).
    pub compression: f32,
    pub compression_velocity: f32,
    pub load: f32,
    pub contact_point: Vec3,
    pub contact_normal: Vec3,
    pub contact_body: Option<BodyHandle>,
    pub surface: Surface,
    /// Wheel centre in world space.
    pub center: Vec3,
    pub steer_angle: f32,
    /// Spin rate (rad/s, positive rolling forward).
    pub omega: f32,
    /// Accumulated spin angle for rendering.
    pub spin: f32,
    pub slip_ratio: f32,
    pub slip_angle: f32,
    /// Combined normalised slip (>1 = sliding).
    pub slip: f32,
    /// Sliding speed of the contact patch (m/s) — drives skid sounds, marks and smoke.
    pub slide_speed: f32,
    pub fx: f32,
    pub fy: f32,
    pub drive_torque: f32,
    pub brake_torque: f32,
    pub abs_factor: f32,
    /// 0..1 damage: punctured tyre, bent suspension.
    pub damage: f32,
    pub detached: bool,
    /// Standing water under the tyre (m), for spray and sounds.
    pub water_depth: f32,
    /// Grip multiplier from aquaplaning (1 = none).
    pub aquaplane: f32,
}

/// Damage the dynamics care about (cosmetic deformation lives in `deform`).
#[derive(Clone, Copy, Debug)]
pub struct MechanicalDamage {
    /// 1 = healthy. Scales engine torque; below 0.15 the engine dies.
    pub engine: f32,
    /// Constant steering pull (rad) from a bent tie rod.
    pub steering_offset: f32,
    pub radiator_leak: bool,
}

impl Default for MechanicalDamage {
    fn default() -> Self {
        Self {
            engine: 1.0,
            steering_offset: 0.0,
            radiator_leak: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Telemetry {
    /// Forward speed (m/s, negative reversing).
    pub speed: f32,
    pub rpm: f32,
    pub lateral_g: f32,
    pub longitudinal_g: f32,
    pub abs_active: bool,
    pub tcs_active: bool,
    pub esc_active: bool,
    pub wheels_on_ground: usize,
    /// Body slip angle (rad).
    pub drift_angle: f32,
    pub yaw_rate: f32,
}

#[derive(Clone, Debug)]
pub struct Vehicle {
    pub chassis: BodyHandle,
    pub desc: VehicleDesc,
    pub controls: VehicleControls,
    pub assists: Assists,
    pub wheels: Vec<WheelState>,
    pub powertrain: PowertrainState,
    pub damage: MechanicalDamage,
    /// Weather/surface grip multiplier set by the game (rain, ice).
    pub grip_scale: f32,
    /// 0..1 wetness of the road (reduces grip per surface type).
    pub wetness: f32,
    pub telemetry: Telemetry,
    /// Current rack position (rad at the wheels).
    pub steer_angle: f32,
    tcs_scale: f32,
    esc_brake: [f32; 8],
    prev_velocity: Vec3,
    request_shift: i32,
}

impl Vehicle {
    pub fn new(chassis: BodyHandle, desc: VehicleDesc) -> Self {
        let wheels = desc
            .wheels
            .iter()
            .map(|_| WheelState {
                abs_factor: 1.0,
                ..Default::default()
            })
            .collect();
        let powertrain = PowertrainState::new(desc.engine.idle_rpm);
        Self {
            chassis,
            desc,
            controls: VehicleControls::default(),
            assists: Assists::default(),
            wheels,
            powertrain,
            damage: MechanicalDamage::default(),
            grip_scale: 1.0,
            wetness: 0.0,
            telemetry: Telemetry::default(),
            steer_angle: 0.0,
            tcs_scale: 1.0,
            esc_brake: [0.0; 8],
            prev_velocity: Vec3::ZERO,
            request_shift: 0,
        }
    }

    /// A parked vehicle with no input may sleep.
    pub fn can_sleep(&self) -> bool {
        self.controls.throttle < 0.01
            && (self.telemetry.speed.abs() < 0.2)
            && self.wheels.iter().all(|w| w.omega.abs() < 0.5)
    }

    /// Request an up (+1) or down (-1) shift (manual) — applied on the next step.
    pub fn shift(&mut self, dir: i32) {
        self.request_shift += dir.signum();
    }

    /// Select a gear directly (manual H-pattern): -1 R, 0 N, 1.. forward.
    pub fn select_gear(&mut self, gear: i32) {
        let pt = &mut self.powertrain;
        let max = self.desc.gearbox.ratios.len() as i32;
        let g = gear.clamp(-1, max);
        if g != pt.gear && pt.pending_gear != Some(g) {
            let clutch_ok = self.assists.auto_clutch || self.controls.clutch > 0.6 || g == 0;
            if clutch_ok {
                pt.pending_gear = Some(g);
                pt.shift_timer = self.desc.gearbox.shift_time;
                pt.grinding = false;
            } else {
                pt.grinding = true;
            }
        }
    }

    pub fn set_selector(&mut self, s: Selector) {
        self.powertrain.selector = s;
        match s {
            Selector::Park | Selector::Neutral => self.powertrain.gear = 0,
            Selector::Reverse => self.powertrain.gear = -1,
            Selector::Drive => {
                if self.powertrain.gear <= 0 {
                    self.powertrain.gear = 1;
                }
            }
        }
    }

    pub fn set_gear_mode(&mut self, m: GearMode) {
        self.powertrain.mode = m;
        if m == GearMode::Manual && self.powertrain.selector == Selector::Park {
            self.powertrain.selector = Selector::Drive;
        }
    }

    pub fn start_engine(&mut self) {
        let pt = &mut self.powertrain;
        if !pt.running && self.damage.engine > 0.15 {
            pt.running = true;
            pt.stalled = false;
            pt.engine_omega = self.desc.engine.idle_rpm * RPM_TO_RAD;
        }
    }
    pub fn stop_engine(&mut self) {
        self.powertrain.running = false;
    }

    /// World transform of a wheel for rendering (includes steer and spin).
    pub fn wheel_pose(&self, i: usize, chassis: &Pose) -> Pose {
        let w = &self.wheels[i];
        let d = &self.desc.wheels[i];
        let side = if d.hardpoint.x >= 0.0 { 1.0 } else { -1.0 };
        let q = chassis.q
            * Quat::from_rotation_y(w.steer_angle)
            * Quat::from_rotation_x(-w.spin)
            * Quat::from_rotation_z(0.0 * side);
        Pose::new(w.center, q)
    }
}

// ------------------------------------------------------------------ world integration

impl PhysicsWorld {
    /// Spawn a vehicle at `pose` (pose of the chassis origin, i.e. ground level under the car).
    pub fn add_vehicle(&mut self, desc: VehicleDesc, pose: Pose) -> VehicleHandle {
        let body = self.add_body(BodyDesc {
            angular_damping: 0.02,
            linear_damping: 0.0,
            ..BodyDesc::dynamic(pose)
        });
        let hull = Shape::convex(&desc.hull).unwrap_or_else(|| Shape::cuboid(desc.size * 0.5));
        self.add_collider(
            body,
            ColliderDesc::new(hull)
                .friction(0.45)
                .restitution(0.05)
                .filter(layers::VEHICLE, layers::ALL & !layers::WHEEL_RAY)
                .user_data(0),
        );
        self.set_mass_properties(
            body,
            MassProperties {
                mass: desc.mass,
                com: desc.com,
                inertia: Mat3::from_diagonal(desc.inertia),
            },
        );
        let mut v = Vehicle::new(body, desc);
        v.set_selector(Selector::Drive);
        // Settle wheels at ride height.
        for (w, d) in v.wheels.iter_mut().zip(&v.desc.wheels) {
            w.center = pose.transform_point(d.hardpoint - Vec3::Y * d.travel * 0.5);
        }
        VehicleHandle(self.vehicles.insert(v))
    }

    pub fn vehicle(&self, h: VehicleHandle) -> Option<&Vehicle> {
        self.vehicles.get(h.0)
    }
    pub fn vehicle_mut(&mut self, h: VehicleHandle) -> Option<&mut Vehicle> {
        self.vehicles.get_mut(h.0)
    }
    pub fn vehicle_handles(&self) -> Vec<VehicleHandle> {
        self.vehicles
            .iter()
            .map(|(h, _)| VehicleHandle(h))
            .collect()
    }

    /// Set inputs and wake the car if needed.
    pub fn set_vehicle_controls(&mut self, h: VehicleHandle, c: VehicleControls) {
        let Some(v) = self.vehicles.get_mut(h.0) else {
            return;
        };
        let changed = v.controls != c;
        v.controls = c;
        let chassis = v.chassis;
        if changed && (c.throttle > 0.0 || c.steer != 0.0 || c.brake > 0.0) {
            self.wake(chassis);
        }
    }

    pub fn remove_vehicle(&mut self, h: VehicleHandle) {
        if let Some(v) = self.vehicles.remove(h.0) {
            self.remove_body(v.chassis);
        }
    }

    pub(crate) fn vehicles_pre_step(&mut self, dt: f32) {
        let mut wake = Vec::new();
        for (_, v) in self.vehicles.iter_mut() {
            let Some(body) = self.bodies.get(v.chassis.0) else {
                continue;
            };
            pre_step(v, body, dt);
            if body.sleeping && (v.controls.throttle > 0.01 || v.powertrain.shifting()) {
                wake.push(v.chassis);
            }
        }
        for b in wake {
            self.wake(b);
        }
    }

    pub(crate) fn vehicles_post_step(&mut self, dt: f32) {
        for (_, v) in self.vehicles.iter_mut() {
            let Some(body) = self.bodies.get(v.chassis.0) else {
                continue;
            };
            post_step(v, body, dt);
        }
    }
}

/// Per-step logic: steering rack, gear selection, shift timing, assist states.
fn pre_step(v: &mut Vehicle, body: &crate::body::RigidBody, dt: f32) {
    let q = body.pose.q;
    let fwd = q * -Vec3::Z;
    let speed = body.lin_vel.dot(fwd);
    v.telemetry.speed = speed;

    // Steering: speed-sensitive range with assist, rate-limited rack.
    let assist = v.assists.steering.clamp(0.0, 1.0);
    let range = v.desc.max_steer / (1.0 + assist * (speed.abs() / 22.0).powi(2));
    let mut target = v.controls.steer.clamp(-1.0, 1.0) * range;
    if assist > 0.0 && speed > 3.0 {
        // Counter-steer: steer toward the direction of travel when the tail slides out.
        let right = q * Vec3::X;
        let v_lat = body.lin_vel.dot(right);
        let beta = v_lat.atan2(speed.abs());
        target -= beta * assist * 0.8;
    }
    target += v.damage.steering_offset;
    let rate = v.desc.steer_speed
        * if target.abs() < v.steer_angle.abs() {
            1.6
        } else {
            1.0
        };
    let delta = (target - v.steer_angle).clamp(-rate * dt, rate * dt);
    v.steer_angle += delta;

    // Gearbox.
    let pt = &mut v.powertrain;
    pt.since_shift += dt;
    let gb = &v.desc.gearbox;
    let n_gears = gb.ratios.len() as i32;
    if pt.shift_timer > 0.0 {
        pt.shift_timer -= dt;
        if pt.shift_timer <= gb.shift_time * 0.5 {
            if let Some(g) = pt.pending_gear.take() {
                pt.gear = g;
                pt.since_shift = 0.0;
            }
        }
        if pt.shift_timer <= 0.0 {
            pt.shift_timer = 0.0;
        }
    }
    let req = std::mem::take(&mut v.request_shift);
    match pt.mode {
        GearMode::Manual => {
            if req != 0 && !pt.shifting() {
                let target = (pt.gear + req).clamp(-1, n_gears);
                let clutch_ok = v.assists.auto_clutch || v.controls.clutch > 0.6;
                if target != pt.gear {
                    if clutch_ok {
                        pt.pending_gear = Some(target);
                        pt.shift_timer = gb.shift_time;
                        pt.grinding = false;
                    } else {
                        pt.grinding = true;
                    }
                }
            }
        }
        GearMode::Automatic => {
            if pt.selector == Selector::Drive && !pt.shifting() && pt.gear > 0 {
                let rpm = pt.rpm();
                let e = &v.desc.engine;
                let throttle = v.controls.throttle;
                let up_rpm =
                    e.idle_rpm + (e.redline * 0.96 - e.idle_rpm) * (0.32 + 0.68 * throttle);
                let ratio_now = gear_ratio(gb, pt.gear);
                let wheel_rpm = if ratio_now != 0.0 {
                    rpm / ratio_now
                } else {
                    0.0
                };
                if pt.gear < n_gears && rpm > up_rpm && pt.since_shift > 0.8 && pt.clutch > 0.9 {
                    pt.pending_gear = Some(pt.gear + 1);
                    pt.shift_timer = gb.shift_time;
                } else if pt.gear > 1 && pt.since_shift > 0.6 {
                    let lower = gear_ratio(gb, pt.gear - 1);
                    let rpm_lower = wheel_rpm * lower;
                    let down_rpm = e.idle_rpm * 1.25 + throttle * (e.redline * 0.55 - e.idle_rpm);
                    let kickdown =
                        throttle > 0.9 && rpm_lower < e.redline * 0.85 && rpm < e.redline * 0.6;
                    if (rpm < down_rpm || kickdown) && rpm_lower < e.redline * 0.92 {
                        pt.pending_gear = Some(pt.gear - 1);
                        pt.shift_timer = gb.shift_time * 0.8;
                    }
                }
            }
            if pt.selector == Selector::Drive && pt.gear <= 0 {
                pt.gear = 1;
            }
            if pt.selector == Selector::Reverse {
                pt.gear = -1;
            }
        }
    }

    // Traction control: cut torque while driven wheels spin.
    let mut spin: f32 = 0.0;
    for (w, d) in v.wheels.iter().zip(&v.desc.wheels) {
        if d.driven && w.in_contact {
            spin = spin.max(w.slip_ratio * pt_sign(v.powertrain.gear));
        }
    }
    let tcs_on = v.assists.tcs && spin > 0.14 && speed.abs() > 0.5;
    v.tcs_scale = if tcs_on {
        (v.tcs_scale - dt * 6.0).max(0.15)
    } else {
        (v.tcs_scale + dt * 2.5).min(1.0)
    };
    v.telemetry.tcs_active = tcs_on;

    // Stability control: compare yaw rate to what the steering asks for.
    v.esc_brake = [0.0; 8];
    v.telemetry.esc_active = false;
    if v.assists.esc && speed > 6.0 {
        let up = q * Vec3::Y;
        let yaw = body.ang_vel.dot(up);
        let l = v.desc.wheelbase.max(1.0);
        let wanted = speed * v.steer_angle.tan() / (l * (1.0 + 0.0025 * speed * speed));
        let limit = 9.0 / speed.max(1.0);
        let wanted = wanted.clamp(-limit, limit);
        let err = yaw - wanted;
        if err.abs() > 0.12 {
            v.telemetry.esc_active = true;
            let amount = ((err.abs() - 0.12) * 2.0).min(0.6);
            // Positive yaw = turning left. Oversteer (too much yaw): brake the outer front.
            for (i, d) in v.desc.wheels.iter().enumerate().take(8) {
                let left = d.hardpoint.x < 0.0;
                let front = d.hardpoint.z < 0.0;
                let oversteer = err.signum() == yaw.signum() && yaw.abs() > wanted.abs();
                let pick = if oversteer {
                    front && (if yaw > 0.0 { !left } else { left })
                } else {
                    !front && (if wanted > 0.0 { left } else { !left })
                };
                if pick {
                    v.esc_brake[i] = amount;
                }
            }
        }
    }
}

#[inline]
fn pt_sign(gear: i32) -> f32 {
    if gear < 0 {
        -1.0
    } else {
        1.0
    }
}

fn post_step(v: &mut Vehicle, body: &crate::body::RigidBody, dt: f32) {
    let q = body.pose.q;
    let fwd = q * -Vec3::Z;
    let right = q * Vec3::X;
    let up = q * Vec3::Y;
    let vel = body.lin_vel;
    let acc = (vel - v.prev_velocity) / dt.max(1e-4);
    v.prev_velocity = vel;
    let t = &mut v.telemetry;
    t.speed = vel.dot(fwd);
    t.rpm = v.powertrain.rpm();
    t.longitudinal_g = acc.dot(fwd) / 9.81;
    t.lateral_g = acc.dot(right) / 9.81;
    t.yaw_rate = body.ang_vel.dot(up);
    t.drift_angle = if vel.length() > 2.0 {
        vel.dot(right).atan2(vel.dot(fwd).abs())
    } else {
        0.0
    };
    t.wheels_on_ground = v.wheels.iter().filter(|w| w.in_contact).count();
    t.abs_active = v.wheels.iter().any(|w| w.abs_factor < 0.99);
}

/// Called by the solver once per substep.
pub(crate) fn substep_vehicles(vehicles: &mut Arena<Vehicle>, ctx: &mut SubstepContext) {
    for (_, v) in vehicles.iter_mut() {
        substep(v, ctx);
    }
}

fn substep(v: &mut Vehicle, ctx: &mut SubstepContext) {
    let Some(ci) = ctx.solver_index(v.chassis) else {
        return;
    };
    let h = ctx.h;
    let n = v.wheels.len();
    let (c, q) = (ctx.bodies[ci].c, ctx.bodies[ci].q);
    let origin = c - q * ctx.bodies[ci].local_com;
    let up = q * Vec3::Y;
    let fwd = q * -Vec3::Z;
    let filter = QueryFilter {
        mask: !(layers::DEBRIS | layers::SENSOR),
        exclude_body: Some(v.chassis),
        include_sensors: false,
        static_only: false,
    };

    // 1. Suspension casts.
    for i in 0..n {
        let d = &v.desc.wheels[i];
        let w = &mut v.wheels[i];
        if w.detached {
            w.in_contact = false;
            continue;
        }
        let hp = origin + q * d.hardpoint;
        // A sphere narrower than the tyre rounds curb edges; bottoms aligned with the wheel.
        let rs = (d.radius * 0.55).min(d.width);
        let start = hp - up * (d.radius - rs);
        let prev_comp = w.compression;
        match ctx.query.sphere_cast(start, rs, -up, d.travel, &filter) {
            Some(hit) => {
                let t = hit.t.max(0.0);
                w.in_contact = true;
                w.compression = d.travel - t;
                w.center = hp - up * t;
                w.contact_point = hit.point;
                w.contact_normal = hit.normal;
                w.contact_body = Some(hit.body);
                w.surface = hit.surface;
            }
            None => {
                w.in_contact = false;
                w.compression = 0.0;
                w.center = hp - up * d.travel;
                w.contact_body = None;
                w.load = 0.0;
            }
        }
        w.compression_velocity = (w.compression - prev_comp) / h;
    }

    // 2. Suspension forces (+ anti-roll bars).
    let mut susp = [0.0f32; 16];
    for i in 0..n.min(16) {
        let d = &v.desc.wheels[i];
        let w = &v.wheels[i];
        if !w.in_contact {
            continue;
        }
        let x = w.compression;
        let xd = w.compression_velocity.clamp(-5.0, 5.0);
        let damping = if xd > 0.0 {
            d.bump_damping
        } else {
            d.rebound_damping
        };
        let mut f = d.spring * x + damping * xd;
        // Bump stop in the last 15% of travel.
        let stop = d.travel * 0.85;
        if x > stop {
            f += d.spring * 12.0 * (x - stop);
        }
        susp[i] = f;
    }
    for arb in &v.desc.anti_roll {
        let (l, r) = (arb.left, arb.right);
        if l < n && r < n && v.wheels[l].in_contact && v.wheels[r].in_contact {
            let f = arb.stiffness * (v.wheels[l].compression - v.wheels[r].compression);
            susp[l] += f;
            susp[r] -= f;
        }
    }

    // 3. Drivetrain.
    drivetrain_substep(v, h);

    // 4. Per-wheel: suspension impulse, brakes, tyre impulses.
    let grip_env = v.grip_scale;
    for i in 0..n {
        let d = v.desc.wheels[i].clone();
        let steer = v.steer_angle * d.steer + d.toe * if d.hardpoint.x > 0.0 { -1.0 } else { 1.0 };
        // Ackermann: the inner wheel turns more.
        let steer = if d.steer != 0.0 && steer.abs() > 1e-4 {
            let l = v.desc.wheelbase;
            let radius = l / steer.tan();
            let side = d.hardpoint.x; // +right
            (l / (radius + side)).atan()
        } else {
            steer
        };
        let brake_input = v.controls.brake.clamp(0.0, 1.0);
        let mut brake_t =
            v.desc.brake_torque * d.brake_share * brake_input * v.wheels[i].abs_factor;
        if d.handbrake {
            brake_t += v.desc.handbrake_torque * v.controls.handbrake.clamp(0.0, 1.0);
        }
        if i < 8 {
            brake_t += v.desc.brake_torque * d.brake_share.max(0.1) * v.esc_brake[i];
        }
        // Park pawl on driven wheels.
        if v.powertrain.selector == Selector::Park
            && v.powertrain.mode == GearMode::Automatic
            && d.driven
        {
            brake_t += 8000.0;
        }
        let w = &mut v.wheels[i];
        w.steer_angle = steer;
        w.brake_torque = brake_t;
        if w.detached {
            continue;
        }
        // Brakes: friction that cannot reverse the spin. If it can stop the wheel this substep the
        // wheel counts as locked for the tyre solve, with the leftover capacity holding it.
        let brake_cap = brake_t * h;
        let mut lock_cap = 0.0;
        if w.omega.abs() * d.inertia <= brake_cap {
            lock_cap = brake_cap - w.omega.abs() * d.inertia;
            w.omega = 0.0;
        } else {
            w.omega -= brake_cap / d.inertia * w.omega.signum();
        }
        if !w.in_contact {
            w.slip = 0.0;
            w.slide_speed = 0.0;
            w.fx = 0.0;
            w.fy = 0.0;
            w.spin += w.omega * h;
            continue;
        }
        let load = susp[i].max(0.0);
        w.load = load;
        let nrm = w.contact_normal;
        let p = w.contact_point;
        let r_ch = p - ctx.bodies[ci].c;
        let ground_i = w
            .contact_body
            .and_then(|b| ctx.solver_index(b))
            .filter(|&g| g != ci);
        // Suspension pushes the chassis up along its axis at the wheel centre.
        let js = up * (load * h);
        ctx.bodies[ci].apply_impulse(js, w.center - ctx.bodies[ci].c);
        if let Some(g) = ground_i {
            let rg = p - ctx.bodies[g].c;
            ctx.bodies[g].apply_impulse(-js, rg);
        }
        // Wheel axes projected onto the contact plane.
        let wf = Quat::from_axis_angle(up, steer) * fwd;
        let f_c = (wf - nrm * wf.dot(nrm)).normalize_or_zero();
        if f_c == Vec3::ZERO {
            continue;
        }
        let r_c = f_c.cross(nrm);
        let ground_v = ground_i.map_or(Vec3::ZERO, |g| {
            ctx.bodies[g].velocity_at(p - ctx.bodies[g].c)
        });
        let vrel = ctx.bodies[ci].velocity_at(r_ch) - ground_v;
        let v_long = vrel.dot(f_c);
        let v_lat = vrel.dot(r_c);
        let surface_mu = surface_grip(w.surface)
            * (1.0 + (wet_factor(w.surface) - 1.0) * v.wetness.clamp(0.0, 1.0));
        let damage_mu = 1.0 - 0.5 * w.damage;
        let depth = ctx
            .puddles
            .map_or(0.0, |p| p.depth_at(w.contact_point.x, w.contact_point.z));
        w.water_depth = depth;
        w.aquaplane = crate::water::aquaplane_grip(depth, vrel.length());
        let damage_mu = damage_mu * w.aquaplane;
        let wheel_speed = w.omega * d.radius;
        let denom = v_long.abs().max(1.0);
        let kappa = (wheel_speed - v_long) / denom;
        let tan_alpha = v_lat / v_long.abs().max(1.0);
        let tf = tyre_force(
            &d.tyre,
            kappa,
            tan_alpha,
            load,
            grip_env * surface_mu * damage_mu,
        );
        w.slip_ratio = kappa;
        w.slip_angle = tan_alpha.atan();
        w.slip = tf.slip;
        // Effective masses at the contact patch.
        let inv_m = |dir: Vec3, body: &crate::solver::SolverBody, r: Vec3| {
            let rn = r.cross(dir);
            body.inv_mass + rn.dot(body.inv_i * rn)
        };
        let mut k_long = inv_m(f_c, &ctx.bodies[ci], r_ch);
        let mut k_lat = inv_m(r_c, &ctx.bodies[ci], r_ch);
        if let Some(g) = ground_i {
            let rg = p - ctx.bodies[g].c;
            k_long += inv_m(f_c, &ctx.bodies[g], rg);
            k_lat += inv_m(r_c, &ctx.bodies[g], rg);
        }
        let locked = lock_cap > 0.0;
        let k_wheel = if locked {
            0.0
        } else {
            d.radius * d.radius / d.inertia
        };
        // Longitudinal: impulse that would exactly remove the slip velocity this substep.
        let slip_vel = wheel_speed - v_long;
        if k_long + k_wheel <= 1e-12 || k_lat <= 1e-12 {
            continue;
        }
        let j_stop_x = slip_vel / (k_long + k_wheel);
        let jx = clamp_toward(tf.fx * h, j_stop_x);
        let j_stop_y = -v_lat / k_lat;
        let jy = clamp_toward(tf.fy * h, j_stop_y);
        // Rolling resistance and soft-ground drag.
        let rr = (d.tyre.rolling_resistance + surface_drag(w.surface)) * load * h;
        let jrr = -rr.min((v_long / k_long).abs()) * v_long.signum();
        let j = f_c * (jx + jrr) + r_c * jy;
        ctx.bodies[ci].apply_impulse(j, r_ch);
        if let Some(g) = ground_i {
            let rg = p - ctx.bodies[g].c;
            ctx.bodies[g].apply_impulse(-j, rg);
        }
        let spin_impulse = jx * d.radius;
        if locked {
            let excess = spin_impulse.abs() - lock_cap;
            if excess > 0.0 {
                w.omega -= excess * spin_impulse.signum() / d.inertia;
            }
        } else {
            w.omega -= spin_impulse / d.inertia;
        }
        w.fx = jx / h;
        w.fy = jy / h;
        w.slide_speed = (Vec2Like(slip_vel - (jx * (k_long + k_wheel)), v_lat + jy * k_lat)).len();
        w.spin += w.omega * h;
        // ABS: release a wheel that starts to lock, reapply when it spins up.
        if v.assists.abs && brake_input > 0.05 && v_long.abs() > 2.0 {
            if kappa * v_long.signum() < -0.14 {
                w.abs_factor = (w.abs_factor - h * 25.0).max(0.15);
            } else {
                w.abs_factor = (w.abs_factor + h * 12.0).min(1.0);
            }
        } else {
            w.abs_factor = 1.0;
        }
    }

    // 5. Aerodynamics: drag at the centre of mass, downforce at the axles.
    let body = &mut ctx.bodies[ci];
    let vel = body.v;
    let speed = vel.length();
    if speed > 0.1 {
        let rho = 1.225;
        let drag = -vel * (0.5 * rho * v.desc.aero.drag_area * speed);
        body.apply_impulse(drag * h, Vec3::ZERO);
        let fwd_speed = vel.dot(fwd);
        let q2 = 0.5 * rho * fwd_speed * fwd_speed;
        let half = v.desc.wheelbase * 0.5;
        for (lift, z) in [
            (v.desc.aero.lift_front, -half),
            (v.desc.aero.lift_rear, half),
        ] {
            if lift != 0.0 {
                let p = origin + q * Vec3::new(0.0, 0.3, z);
                body.apply_impulse(-up * (lift * q2 * h), p - body.c);
            }
        }
    }
}

struct Vec2Like(f32, f32);
impl Vec2Like {
    fn len(&self) -> f32 {
        (self.0 * self.0 + self.1 * self.1).sqrt()
    }
}

/// `want` limited so that it does not exceed `stop` (the impulse that zeroes the slip) and has its sign.
#[inline]
fn clamp_toward(want: f32, stop: f32) -> f32 {
    if stop >= 0.0 {
        want.clamp(0.0, stop)
    } else {
        want.clamp(stop, 0.0)
    }
}

fn drivetrain_substep(v: &mut Vehicle, h: f32) {
    let e = v.desc.engine.clone();
    let gb = &v.desc.gearbox;
    let layout = v.desc.layout;
    let pt = &mut v.powertrain;
    let idle = e.idle_rpm * RPM_TO_RAD;
    let redline = e.redline * RPM_TO_RAD;

    // Engine torque.
    let mut throttle = v.controls.throttle.clamp(0.0, 1.0) * v.tcs_scale;
    if pt.shifting() {
        throttle *= 0.15;
    }
    if pt.engine_omega > redline {
        pt.limiter = true;
    } else if pt.engine_omega < redline * 0.97 {
        pt.limiter = false;
    }
    if pt.limiter || !pt.running {
        throttle = 0.0;
    }
    pt.throttle_applied = throttle;
    let rpm = pt.engine_omega / RPM_TO_RAD;
    let full = e.torque_at(rpm) * v.damage.engine.clamp(0.0, 1.0);
    let friction = e.engine_brake * (pt.engine_omega / redline).clamp(0.0, 1.5) + 4.0;
    let mut t_engine = if pt.running {
        full * throttle - friction * (1.0 - throttle)
    } else {
        -friction * 3.0
    };
    // Idle governor.
    if pt.running && pt.engine_omega < idle * 1.05 {
        t_engine +=
            ((idle * 1.05 - pt.engine_omega) * e.inertia * 40.0).min(e.peak_torque() * 0.45);
    }
    pt.engine_torque = t_engine;

    // Driven wheel speed.
    let ratio = gear_ratio(gb, pt.gear);
    let mut i_total = 0.0;
    let mut omega_sum = 0.0;
    let mut count = 0.0;
    for (d, w) in v.desc.wheels.iter().zip(&v.wheels) {
        if d.driven && !w.detached {
            i_total += d.inertia;
            omega_sum += w.omega;
            count += 1.0;
        }
    }
    let omega_drive = if count > 0.0 { omega_sum / count } else { 0.0 };

    // Clutch engagement.
    let auto = pt.mode == GearMode::Automatic || v.assists.auto_clutch;
    let in_gear = pt.gear != 0 && !pt.shifting() && count > 0.0;
    let engage = if !in_gear {
        0.0
    } else if auto {
        let shaft = (omega_drive * ratio).abs();
        let launch = idle * (1.25 + 0.9 * v.controls.throttle);
        let slip_engage =
            ((pt.engine_omega - idle * 1.02) / (launch - idle * 1.02).max(1.0)).clamp(0.0, 1.0);
        // Locked once the shaft turns faster than idle; creeps a little at idle.

        if shaft > idle * 1.1 {
            1.0
        } else {
            slip_engage.max(if v.controls.brake > 0.3 { 0.02 } else { 0.22 })
        }
    } else {
        1.0 - v.controls.clutch.clamp(0.0, 1.0)
    };
    pt.clutch = engage;
    let mut t_clutch = 0.0;
    if engage > 0.0 && ratio != 0.0 && i_total > 0.0 {
        let cap = engage * e.peak_torque() * gb.clutch_capacity;
        let slip = pt.engine_omega - omega_drive * ratio;
        let t_lock = slip / (h * (1.0 / e.inertia + ratio * ratio / i_total));
        t_clutch = t_lock.clamp(-cap, cap);
    }
    pt.clutch_torque = t_clutch;
    pt.engine_omega += (t_engine - t_clutch) / e.inertia * h;
    if pt.engine_omega < 0.0 {
        pt.engine_omega = 0.0;
    }
    // Stall: manual with the clutch engaged and the engine dragged below the stall speed.
    if pt.running && !auto && pt.engine_omega < e.stall_rpm * RPM_TO_RAD && engage > 0.5 {
        pt.running = false;
        pt.stalled = true;
    }
    if v.damage.engine < 0.15 {
        pt.running = false;
    }

    // Wheel torques through the differentials.
    let t_wheels = t_clutch * ratio * gb.efficiency;
    let (front_share, rear_share) = match layout {
        DriveLayout::Fwd => (1.0, 0.0),
        DriveLayout::Rwd => (0.0, 1.0),
        DriveLayout::Awd { front } => (front, 1.0 - front),
    };
    for (axle_front, share, diff) in [
        (true, front_share, v.desc.front_diff),
        (false, rear_share, v.desc.rear_diff),
    ] {
        if share <= 0.0 {
            continue;
        }
        let idx: smallvec::SmallVec<[usize; 4]> = (0..v.wheels.len())
            .filter(|&i| {
                v.desc.wheels[i].driven
                    && !v.wheels[i].detached
                    && (v.desc.wheels[i].hardpoint.z < 0.0) == axle_front
            })
            .collect();
        if idx.is_empty() {
            continue;
        }
        let axle_t = t_wheels * share;
        if idx.len() == 2 {
            let (l, r) = if v.desc.wheels[idx[0]].hardpoint.x < v.desc.wheels[idx[1]].hardpoint.x {
                (idx[0], idx[1])
            } else {
                (idx[1], idx[0])
            };
            let inertia = v.desc.wheels[l].inertia;
            let (tl, tr) = split_axle(
                diff,
                axle_t,
                v.wheels[l].omega,
                v.wheels[r].omega,
                inertia,
                h,
            );
            for (i, t) in [(l, tl), (r, tr)] {
                v.wheels[i].drive_torque = t;
                v.wheels[i].omega += t * h / v.desc.wheels[i].inertia;
            }
        } else {
            let each = axle_t / idx.len() as f32;
            for &i in &idx {
                v.wheels[i].drive_torque = each;
                v.wheels[i].omega += each * h / v.desc.wheels[i].inertia;
            }
        }
    }
}
