//! Planar rigid-body vehicle model ("bicycle" model with load transfer).

use super::specs::{DriveLayout, VehicleSpec};
use crate::math::scalar::{clamp, lerp, sign, G};

/// Pedal and steering inputs of the rigid-body model.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DriveInputs {
    /// 0..1
    pub throttle: f64,
    /// 0..1
    pub brake: f64,
    /// -1 (full left) .. 1 (full right), shaped by the driver layer.
    pub steer: f64,
    /// 0..1
    pub handbrake: f64,
}

/// Driver assists.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Assists {
    pub abs: bool,
    pub tcs: bool,
    pub esc: bool,
}

impl Default for Assists {
    fn default() -> Self {
        Assists { abs: true, tcs: true, esc: true }
    }
}

const RHO: f64 = 1.2;

struct AxleForces {
    fx: f64,
    fy: f64,
    locked: bool,
    spin: bool,
    abs: bool,
    tcs: bool,
}

/// Planar rigid-body vehicle model. Each axle is a lumped tyre with a
/// magic-formula lateral curve and a traction-limited longitudinal force,
/// combined through a friction circle. Wheel lock (no ABS) and wheelspin (no
/// TCS) degrade lateral grip; ESC applies a corrective yaw moment; the model
/// blends toward kinematic rolling below 3 m/s. Sub-stepped at 120 Hz.
#[derive(Clone, Debug)]
pub struct VehicleDynamics {
    pub spec: &'static VehicleSpec,
    pub x: f64,
    pub y: f64,
    pub yaw: f64,
    /// Body-frame velocity: forward (u) and leftward (v), m/s.
    pub u: f64,
    pub v: f64,
    /// Yaw rate, rad/s (positive = turning left).
    pub r: f64,
    /// Actual front wheel angle, rad (positive = left).
    pub steer_angle: f64,
    /// Smoothed longitudinal / lateral acceleration (m/s²).
    pub ax: f64,
    pub ay: f64,
    /// Wheel angular speeds (rad/s).
    pub omega_f: f64,
    pub omega_r: f64,
    spin_slip: f64,
    pub abs_active: bool,
    pub tcs_active: bool,
    pub esc_active: bool,
    pub front_locked: bool,
    pub rear_locked: bool,
    pub wheelspin: bool,
    /// 0..1 how hard the tyres are sliding (skid sound & marks).
    pub skid: f64,
    pub slip_angle_f: f64,
    pub slip_angle_r: f64,
    pub mass: f64,
    pub iz: f64,
    /// CG to front axle / rear axle.
    pub a: f64,
    pub b: f64,
    pub wheelbase: f64,
    /// Grip multiplier from surface & weather (1 = dry asphalt).
    pub surface_grip: f64,
    /// Extra rolling resistance (off-road).
    pub surface_drag: f64,
    /// 0..1, reduces grip on damaged cars.
    pub tire_health: f64,
}

impl VehicleDynamics {
    pub fn new(spec: &'static VehicleSpec) -> Self {
        VehicleDynamics {
            spec,
            x: 0.0,
            y: 0.0,
            yaw: 0.0,
            u: 0.0,
            v: 0.0,
            r: 0.0,
            steer_angle: 0.0,
            ax: 0.0,
            ay: 0.0,
            omega_f: 0.0,
            omega_r: 0.0,
            spin_slip: 0.0,
            abs_active: false,
            tcs_active: false,
            esc_active: false,
            front_locked: false,
            rear_locked: false,
            wheelspin: false,
            skid: 0.0,
            slip_angle_f: 0.0,
            slip_angle_r: 0.0,
            mass: spec.mass,
            iz: (spec.mass * (spec.length * spec.length + spec.width * spec.width)) / 12.0 * 0.85,
            a: spec.wheelbase * (1.0 - spec.front_weight),
            b: spec.wheelbase * spec.front_weight,
            wheelbase: spec.wheelbase,
            surface_grip: 1.0,
            surface_drag: 0.0,
            tire_health: 1.0,
        }
    }

    pub fn abs_speed(&self) -> f64 {
        (self.u * self.u + self.v * self.v).sqrt()
    }

    /// World-frame velocity x.
    pub fn vx(&self) -> f64 {
        self.u * self.yaw.cos() - self.v * self.yaw.sin()
    }

    /// World-frame velocity y.
    pub fn vy(&self) -> f64 {
        self.u * self.yaw.sin() + self.v * self.yaw.cos()
    }

    pub fn set_world_velocity(&mut self, vx: f64, vy: f64) {
        let c = self.yaw.cos();
        let s = self.yaw.sin();
        self.u = vx * c + vy * s;
        self.v = -vx * s + vy * c;
    }

    /// Place the body with a forward speed and no slip.
    pub fn reset(&mut self, x: f64, y: f64, yaw: f64, forward_speed: f64) {
        self.x = x;
        self.y = y;
        self.yaw = yaw;
        self.u = forward_speed;
        self.v = 0.0;
        self.r = 0.0;
        self.ax = 0.0;
        self.ay = 0.0;
        self.spin_slip = 0.0;
        self.omega_f = forward_speed / self.spec.wheel_radius;
        self.omega_r = self.omega_f;
    }

    /// Angular speed of the driven wheels (for the powertrain).
    pub fn driven_omega(&self) -> f64 {
        match self.spec.drive {
            DriveLayout::Fwd => self.omega_f,
            DriveLayout::Rwd => self.omega_r,
            DriveLayout::Awd => (self.omega_f + self.omega_r) / 2.0,
        }
    }

    /// Apply an impulse (N·s) at world point (px, py).
    pub fn apply_impulse(&mut self, jx: f64, jy: f64, px: f64, py: f64) {
        let vx = self.vx() + jx / self.mass;
        let vy = self.vy() + jy / self.mass;
        let rx = px - self.x;
        let ry = py - self.y;
        self.r += (rx * jy - ry * jx) / self.iz;
        self.set_world_velocity(vx, vy);
    }

    fn lateral(&self, alpha: f64, fz: f64, mu: f64) -> f64 {
        -mu * fz * (self.spec.tire_c * (self.spec.tire_b * alpha).atan()).sin()
    }

    /// Advance by `dt` (sub-stepped). `drive_torque` comes from the powertrain,
    /// `steer_target` is the desired wheel angle (rad, + = left).
    pub fn step(&mut self, dt: f64, inputs: &DriveInputs, drive_torque: f64, steer_target: f64, assists: &Assists) {
        let sub = if dt > 1.0 / 90.0 { (dt / (1.0 / 120.0)).ceil() as usize } else { 1 };
        let h = dt / sub as f64;
        for _ in 0..sub {
            self.substep(h, inputs, drive_torque, steer_target, assists);
        }
    }

    fn substep(&mut self, h: f64, inp: &DriveInputs, drive_torque: f64, steer_target: f64, assists: &Assists) {
        let spec = self.spec;
        let m = self.mass;
        let l = self.wheelbase;
        let rad = spec.wheel_radius;
        let mu = spec.grip * self.surface_grip * (0.6 + 0.4 * self.tire_health);
        let mu_k = mu * 0.78;

        // Steering actuator (rate limited).
        let steer_rate = 3.2;
        let ds = clamp(steer_target - self.steer_angle, -steer_rate * h, steer_rate * h);
        self.steer_angle += ds;
        let delta = self.steer_angle;
        let cd = delta.cos();
        let sd = delta.sin();

        // Vertical loads with longitudinal weight transfer.
        let wt = (m * self.ax * spec.cg_height) / l;
        let fz_f = clamp((m * G * self.b) / l - wt, 0.15 * m * G, 0.85 * m * G);
        let fz_r = clamp((m * G * self.a) / l + wt, 0.15 * m * G, 0.85 * m * G);

        let u = self.u;
        let v = self.v;
        let r = self.r;
        let vy_f = v + self.a * r;
        let vy_r = v - self.b * r;
        let long_f = u * cd + vy_f * sd;
        let lat_f = -u * sd + vy_f * cd;
        let long_r = u;
        let lat_r = vy_r;
        let v_min = 1.5;
        let alpha_f = lat_f.atan2(long_f.abs().max(v_min));
        let alpha_r = lat_r.atan2(long_r.abs().max(v_min));
        self.slip_angle_f = alpha_f;
        self.slip_angle_r = alpha_r;

        let (tq_f, tq_r) = match spec.drive {
            DriveLayout::Fwd => (drive_torque, 0.0),
            DriveLayout::Rwd => (0.0, drive_torque),
            DriveLayout::Awd => (drive_torque * 0.4, drive_torque * 0.6),
        };
        let brake_t = inp.brake * spec.brake_torque;
        let b_f = (brake_t * spec.brake_bias) / rad;
        let mut b_r = (brake_t * (1.0 - spec.brake_bias)) / rad;
        let hb = inp.handbrake * spec.brake_torque * 0.9;
        b_r += hb / rad;
        let roll_f = spec.roll_resist * fz_f * (1.0 + self.surface_drag);
        let roll_r = spec.roll_resist * fz_r * (1.0 + self.surface_drag);

        let res_f = self.axle(tq_f / rad, b_f, roll_f, long_f, lat_f, alpha_f, fz_f, mu, mu_k, assists, false, inp.handbrake);
        let res_r = self.axle(tq_r / rad, b_r, roll_r, long_r, lat_r, alpha_r, fz_r, mu, mu_k, assists, true, inp.handbrake);
        self.front_locked = res_f.locked;
        self.rear_locked = res_r.locked;
        let spin = res_f.spin || res_r.spin;
        self.abs_active = res_f.abs || res_r.abs;
        self.tcs_active = res_f.tcs || res_r.tcs;
        self.wheelspin = spin;

        // Body-frame forces.
        let fx_fb = res_f.fx * cd - res_f.fy * sd;
        let fy_fb = res_f.fx * sd + res_f.fy * cd;
        let drag = 0.5 * RHO * spec.drag_area * u * u.abs();
        let lat_drag = 0.5 * RHO * spec.drag_area * 3.0 * v * v.abs();
        let mut fx = fx_fb + res_r.fx - drag;
        let fy = fy_fb + res_r.fy - lat_drag;
        let mut mz = self.a * fy_fb - self.b * res_r.fy;

        // Electronic stability control.
        self.esc_active = false;
        if assists.esc && u.abs() > 6.0 && inp.handbrake < 0.1 {
            let r_des = (u * delta.tan()) / (l * (1.0 + 0.0025 * u * u));
            let r_max = (mu * G * 0.9) / u.abs();
            let target = clamp(r_des, -r_max, r_max);
            let err = r - target;
            if err.abs() > 0.12 {
                mz -= clamp(err * self.iz * 4.0, -m * G * 1.2, m * G * 1.2);
                fx -= sign(u) * err.abs().min(1.0) * m * 1.5;
                self.esc_active = true;
            }
        }

        // Integrate (semi-implicit Euler in the body frame).
        let mut du = fx / m + v * r;
        let mut dv = fy / m - u * r;
        let dr = mz / self.iz;
        let mut u_n = u + du * h;
        let mut v_n = v + dv * h;
        let mut r_n = r + dr * h;

        // Brakes and rolling resistance cannot push the car backwards.
        let drive_f = ((tq_f + tq_r) / rad).abs();
        let resist = b_f + b_r + roll_f + roll_r;
        if ((u > 0.0 && u_n < 0.0) || (u < 0.0 && u_n > 0.0)) && (drive_f < resist || sign(drive_torque) == sign(u)) {
            u_n = 0.0;
        }
        if u_n.abs() < 0.05 && drive_f <= resist + 1.0 {
            u_n = 0.0;
            if v_n.abs() < 0.3 {
                v_n *= 0.5;
            }
        }

        // Low-speed blend toward kinematic rolling.
        let spd = u_n.abs();
        let k = clamp(1.0 - spd / 3.0, 0.0, 1.0);
        if k > 0.0 {
            let r_kin = (u_n * delta.tan()) / l;
            let v_kin = r_kin * self.b;
            let blend = (k * 12.0 * h).min(1.0);
            r_n = lerp(r_n, r_kin, blend);
            v_n = lerp(v_n, v_kin, blend);
        }
        du = (u_n - u) / h;
        dv = (v_n - v) / h;

        self.u = u_n;
        self.v = v_n;
        self.r = r_n;
        let cy = self.yaw.cos();
        let sy = self.yaw.sin();
        self.x += (u_n * cy - v_n * sy) * h;
        self.y += (u_n * sy + v_n * cy) * h;
        self.yaw += r_n * h;
        let acc_long = du - v_n * r_n;
        let acc_lat = dv + u_n * r_n;
        let sm = (10.0 * h).min(1.0);
        self.ax += (clamp(acc_long, -15.0, 15.0) - self.ax) * sm;
        self.ay += (clamp(acc_lat, -15.0, 15.0) - self.ay) * sm;

        // Wheel speeds.
        if spin {
            self.spin_slip = (self.spin_slip + 40.0 * h).min(25.0);
        } else {
            self.spin_slip = (self.spin_slip - 60.0 * h).max(0.0);
        }
        let sgn = if drive_torque >= 0.0 { 1.0 } else { -1.0 };
        let free_f = long_f / rad;
        let free_r = long_r / rad;
        let driven = spec.drive;
        self.omega_f = if self.front_locked { 0.0 } else { free_f + if driven != DriveLayout::Rwd { (sgn * self.spin_slip) / rad } else { 0.0 } };
        self.omega_r = if self.rear_locked { 0.0 } else { free_r + if driven != DriveLayout::Fwd { (sgn * self.spin_slip) / rad } else { 0.0 } };

        // Skid intensity.
        let peak = 0.12;
        let lat_skid = alpha_f.abs().max(alpha_r.abs()) - peak;
        let mut skid = clamp(lat_skid * 5.0, 0.0, 1.0) * clamp(self.abs_speed() / 6.0, 0.0, 1.0);
        if (self.front_locked || self.rear_locked) && self.abs_speed() > 1.0 {
            skid = skid.max(0.9);
        }
        if spin {
            skid = skid.max(0.7);
        }
        self.skid = skid;
    }

    #[allow(clippy::too_many_arguments)]
    fn axle(
        &self,
        drive_f: f64,
        brake_f: f64,
        roll_f: f64,
        v_long: f64,
        v_lat: f64,
        alpha: f64,
        fz: f64,
        mu: f64,
        mu_k: f64,
        assists: &Assists,
        rear: bool,
        handbrake: f64,
    ) -> AxleForces {
        let f_max = mu * fz;
        let dir = if v_long > 0.2 {
            1.0
        } else if v_long < -0.2 {
            -1.0
        } else {
            0.0
        };
        let mut fy_pure = self.lateral(alpha, fz, mu);
        if v_lat.abs() < 0.05 && v_long.abs() < 0.2 {
            fy_pure = 0.0;
        }
        let mut fx = drive_f;
        let mut locked = false;
        let mut spin = false;
        let mut abs = false;
        let mut tcs = false;
        let resist = brake_f + roll_f;
        if dir != 0.0 {
            fx -= dir * resist;
        } else if fx.abs() <= resist {
            fx = 0.0;
        } else {
            fx -= sign(fx) * resist;
        }
        let brake_dominant = brake_f > drive_f.abs() && dir != 0.0;
        if fx.abs() > f_max {
            if brake_dominant {
                let handbrake_lock = rear && handbrake > 0.3;
                if assists.abs && !handbrake_lock {
                    fx = sign(fx) * f_max * 0.97;
                    abs = true;
                } else {
                    locked = true;
                }
            } else if assists.tcs {
                fx = sign(fx) * f_max * 0.95;
                tcs = true;
            } else {
                spin = true;
            }
        }
        if locked {
            let sp = (v_long * v_long + v_lat * v_lat).sqrt();
            if sp > 0.01 {
                return AxleForces { fx: (-mu_k * fz * v_long) / sp, fy: (-mu_k * fz * v_lat) / sp, locked, spin, abs, tcs };
            }
            return AxleForces { fx: 0.0, fy: 0.0, locked, spin, abs, tcs };
        }
        if spin {
            fx = sign(fx) * mu_k * fz;
            let fy_max = (f_max * f_max - fx * fx).max(0.0).sqrt() + f_max * 0.15;
            return AxleForces { fx, fy: clamp(fy_pure * 0.55, -fy_max, fy_max), locked, spin, abs, tcs };
        }
        let fy_max = (f_max * f_max - fx * fx).max(0.0).sqrt();
        AxleForces { fx, fy: clamp(fy_pure, -fy_max, fy_max), locked, spin, abs, tcs }
    }
}
