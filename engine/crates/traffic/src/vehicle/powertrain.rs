//! Engine, clutch and gearbox.

use super::specs::VehicleSpec;
use crate::math::scalar::{clamp, curve, sign};

const RPM_PER_RADS: f64 = 60.0 / (2.0 * std::f64::consts::PI);

/// Automatic or manual gearbox.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TransmissionMode {
    Automatic,
    Manual,
}

/// Engine, clutch and gearbox. Produces drive torque at the wheels from pedal
/// inputs and wheel speed: torque curve, engine braking, torque converter
/// (stall speed, multiplication, creep), throttle-dependent shift points and
/// kickdown, clutch slip and stalling, rev limiter, fuel and engine health.
///
/// Gear: -1 = reverse, 0 = neutral, 1..n forward.
#[derive(Clone, Debug)]
pub struct Powertrain {
    pub spec: &'static VehicleSpec,
    pub rpm: f64,
    pub gear: i32,
    pub mode: TransmissionMode,
    pub running: bool,
    /// Engine stalled (manual: dropped the clutch at low rpm).
    pub stalled: bool,
    /// 0 = clutch released (engaged), 1 = fully pressed.
    pub clutch_pedal: f64,
    pub fuel: f64,
    /// 0..1 engine health; reduces torque, can kill the engine.
    pub health: f64,
    shift_timer: f64,
    pending_gear: i32,
    pub limiter_active: bool,
    limiter_timer: f64,
    /// Last produced engine torque (N·m) and load 0..1 for audio.
    pub engine_torque: f64,
    pub load: f64,
    /// Seconds the starter has been cranking.
    pub cranking: f64,
    pub fuel_used: f64,
    /// Speed limiter target (m/s); 0 = off.
    pub limiter: f64,
    since_shift: f64,
    last_shift_dir: i32,
    pub max_gear: i32,
}

impl Powertrain {
    pub fn new(spec: &'static VehicleSpec) -> Self {
        Powertrain {
            spec,
            rpm: spec.idle_rpm,
            gear: 1,
            mode: TransmissionMode::Automatic,
            running: true,
            stalled: false,
            clutch_pedal: 0.0,
            fuel: spec.fuel_capacity * 0.75,
            health: 1.0,
            shift_timer: 0.0,
            pending_gear: 1,
            limiter_active: false,
            limiter_timer: 0.0,
            engine_torque: 0.0,
            load: 0.0,
            cranking: 0.0,
            fuel_used: 0.0,
            limiter: 0.0,
            since_shift: 10.0,
            last_shift_dir: 0,
            max_gear: spec.gears.len() as i32,
        }
    }

    pub fn ratio(&self, g: i32) -> f64 {
        if g == 0 {
            0.0
        } else if g < 0 {
            -self.spec.reverse_ratio
        } else {
            self.spec.gears[(g - 1) as usize]
        }
    }

    pub fn shifting(&self) -> bool {
        self.shift_timer > 0.0
    }

    /// The gear being engaged (pending gear while shifting).
    pub fn selected_gear(&self) -> i32 {
        if self.shift_timer > 0.0 {
            self.pending_gear
        } else {
            self.gear
        }
    }

    /// Request a gear. Returns false when already there.
    pub fn shift_to(&mut self, g: i32) -> bool {
        let target = g.clamp(-1, self.max_gear);
        if target == self.gear && self.shift_timer <= 0.0 {
            return false;
        }
        self.last_shift_dir = if target > self.gear { 1 } else { -1 };
        self.since_shift = 0.0;
        self.pending_gear = target;
        self.shift_timer = self.spec.shift_time;
        true
    }

    pub fn shift_up(&mut self) -> bool {
        let base = self.selected_gear();
        if base >= self.max_gear {
            return false;
        }
        self.shift_to(base + 1)
    }

    pub fn shift_down(&mut self) -> bool {
        let base = self.selected_gear();
        if base <= -1 {
            return false;
        }
        self.shift_to(base - 1)
    }

    /// R, N, 1..n
    pub fn gear_label(&self) -> String {
        let g = self.selected_gear();
        if g < 0 {
            "R".into()
        } else if g == 0 {
            "N".into()
        } else {
            g.to_string()
        }
    }

    pub fn start(&mut self) {
        if self.health <= 0.02 || self.fuel <= 0.0 {
            self.cranking = 1.2;
            return;
        }
        if !self.running {
            self.cranking = 0.8;
        }
    }

    pub fn stop(&mut self) {
        self.running = false;
        self.cranking = 0.0;
    }

    pub fn torque_at(&self, rpm: f64, throttle: f64) -> f64 {
        let full = curve(self.spec.torque_curve, rpm);
        let health_factor = 0.35 + 0.65 * clamp(self.health, 0.0, 1.0);
        let drag = self.spec.engine_brake * clamp((rpm - self.spec.idle_rpm * 1.05) / (self.spec.redline - self.spec.idle_rpm), 0.0, 1.2);
        throttle * full * health_factor - (1.0 - throttle) * drag
    }

    /// Advance and return drive torque at the driven wheels (N·m).
    ///
    /// `wheel_omega`: driven wheel speed (rad/s, signed); `speed`: forward
    /// speed (m/s); `torque_scale`: TCS/ESC torque cut (does not affect shifting).
    pub fn update(&mut self, dt: f64, wheel_omega: f64, mut throttle: f64, speed: f64, torque_scale: f64) -> f64 {
        let spec = self.spec;
        self.since_shift += dt;
        if self.cranking > 0.0 {
            self.cranking -= dt;
            self.rpm = 280.0 + 60.0 * (self.cranking * 47.0).sin();
            if self.cranking <= 0.0 && self.health > 0.02 && self.fuel > 0.0 {
                self.running = true;
                self.stalled = false;
                self.rpm = spec.idle_rpm * 1.4;
            }
            return 0.0;
        }
        if self.fuel <= 0.0 || self.health <= 0.02 {
            self.running = false;
        }
        if self.shift_timer > 0.0 {
            self.shift_timer -= dt;
            if self.shift_timer <= 0.0 {
                self.gear = self.pending_gear;
            }
        }
        if self.mode == TransmissionMode::Automatic && self.running {
            self.auto_shift(throttle, speed);
        }
        if self.limiter > 0.0 && speed.abs() > self.limiter {
            throttle = 0.0;
        }
        let ratio = self.ratio(self.gear) * spec.final_drive;
        let engaged = self.gear != 0 && self.shift_timer <= 0.0;
        let mut couple = if engaged { 1.0 - self.clutch_pedal } else { 0.0 };
        if !self.running {
            self.rpm = (self.rpm - 3000.0 * dt).max(0.0);
            self.engine_torque = 0.0;
            self.load = 0.0;
            // A stopped engine still resists wheel motion through the gearbox.
            return if engaged { -sign(wheel_omega) * (wheel_omega.abs() * 4.0).min(40.0) * ratio.abs() * couple } else { 0.0 };
        }
        let wheel_rpm = (wheel_omega * ratio).abs() * RPM_PER_RADS;
        let free_rev = spec.idle_rpm + throttle * (spec.redline - spec.idle_rpm);
        let target_rpm;
        if couple > 0.05 {
            if self.mode == TransmissionMode::Automatic {
                let stall = spec.idle_rpm + throttle * 2400f64.min((spec.redline - spec.idle_rpm) * 0.45);
                target_rpm = wheel_rpm.max(stall).max(spec.idle_rpm);
            } else {
                let slip = 1.0 - couple;
                let t = wheel_rpm * couple + free_rev * slip;
                if wheel_rpm < spec.idle_rpm * 0.55 && couple > 0.9 && throttle < 0.6 && speed.abs() < 2.5 && self.gear != 0 {
                    // Lugging below idle with the clutch out: stall.
                    self.running = false;
                    self.stalled = true;
                    return 0.0;
                }
                target_rpm = t.max(spec.idle_rpm * if couple < 1.0 { 1.0 } else { 0.6 });
            }
        } else {
            target_rpm = free_rev;
            couple = 0.0;
        }
        let rate = if couple > 0.05 { 25.0 } else { 8.0 };
        self.rpm += (target_rpm - self.rpm) * (rate * dt).min(1.0);

        if self.rpm >= spec.redline {
            self.limiter_timer = 0.08;
        }
        let mut eff_throttle = throttle;
        if self.limiter_timer > 0.0 {
            self.limiter_timer -= dt;
            eff_throttle = 0.0;
            self.limiter_active = true;
        } else {
            self.limiter_active = false;
        }
        self.rpm = self.rpm.min(spec.redline + 150.0);

        let mut tq = self.torque_at(self.rpm, eff_throttle);
        if self.mode == TransmissionMode::Automatic && couple > 0.0 && eff_throttle < 0.05 && wheel_rpm < spec.idle_rpm * 1.3 {
            tq = tq.max(curve(spec.torque_curve, spec.idle_rpm) * 0.12);
        }
        if self.mode == TransmissionMode::Automatic && couple > 0.0 && wheel_rpm < self.rpm * 0.85 {
            tq *= 1.0 + 0.6 * (1.0 - wheel_rpm / self.rpm.max(1.0));
        }
        self.engine_torque = tq;
        self.load = clamp(eff_throttle, 0.0, 1.0);

        // Fuel: ~0.33 L per kWh at load plus idle flow.
        let power_kw = ((tq * self.rpm) / RPM_PER_RADS / 1000.0).max(0.0);
        let used = (power_kw * 0.33 + 0.8) * (dt / 3600.0);
        self.fuel = (self.fuel - used).max(0.0);
        self.fuel_used += used;

        tq * ratio * 0.9 * couple * if tq > 0.0 { torque_scale } else { 1.0 }
    }

    fn auto_shift(&mut self, throttle: f64, speed: f64) {
        if self.shift_timer > 0.0 {
            return;
        }
        let spec = self.spec;
        if self.gear <= 0 {
            return;
        }
        let ratio = self.ratio(self.gear) * spec.final_drive;
        let rpm = (speed.abs() / spec.wheel_radius) * ratio * RPM_PER_RADS;
        let up_at = spec.idle_rpm + (spec.redline - spec.idle_rpm) * (0.35 + 0.6 * throttle);
        let down_at = spec.idle_rpm * 1.25 + (spec.redline - spec.idle_rpm) * 0.28 * throttle;
        let settled = self.since_shift > 1.5 || self.last_shift_dir == 0;
        if rpm > up_at && self.gear < self.max_gear {
            if settled || self.last_shift_dir > 0 {
                self.shift_to(self.gear + 1);
            }
        } else if self.gear > 1 {
            let lower_ratio = self.ratio(self.gear - 1) * spec.final_drive;
            let rpm_lower = (speed.abs() / spec.wheel_radius) * lower_ratio * RPM_PER_RADS;
            let lower_up_at = spec.idle_rpm + (spec.redline - spec.idle_rpm) * (0.35 + 0.6 * throttle);
            let kickdown = throttle > 0.9 && rpm_lower < (spec.redline * 0.8).min(lower_up_at * 0.9) && rpm < spec.redline * 0.5;
            if rpm < down_at || (kickdown && (settled || self.last_shift_dir < 0)) {
                self.shift_to(self.gear - 1);
            }
        }
    }
}
