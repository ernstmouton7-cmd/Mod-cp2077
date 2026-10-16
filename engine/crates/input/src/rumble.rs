//! Telemetry-driven vibration for two-motor pads with optional trigger motors.
//!
//! - Low-frequency (left, heavy) motor: engine at the limiter, impacts, rough surfaces, kerbs.
//! - High-frequency (right, light) motor: tyre slip, wheelspin, gear shifts, ABS chatter.
//! - Right trigger: wheelspin / traction control; left trigger: ABS pulses and wheel lock.

/// Vibration output, all 0..1.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rumble {
    pub low: f32,
    pub high: f32,
    pub left_trigger: f32,
    pub right_trigger: f32,
}

/// What the car is doing this frame.
#[derive(Clone, Copy, Debug, Default)]
pub struct RumbleInputs {
    pub speed: f32,
    /// Engine rpm / redline (0..1+).
    pub rpm_fraction: f32,
    pub limiter: bool,
    pub engine_running: bool,
    /// Largest contact-patch sliding speed (m/s).
    pub slide_speed: f32,
    /// Driven-wheel spin (slip ratio, positive).
    pub wheelspin: f32,
    pub abs_active: bool,
    pub tcs_active: bool,
    /// A wheel is locked under braking.
    pub wheel_locked: bool,
    /// Surface roughness under the tyres (0 asphalt, 0.3 cobbles, 0.6 gravel/grass).
    pub roughness: f32,
    /// Largest suspension compression velocity this frame (m/s) — kerbs and potholes.
    pub suspension_jolt: f32,
    /// Collision impulse this frame (N·s), scaled by the car's mass inside.
    pub impact: f32,
    pub mass: f32,
    /// A gear change happened this frame.
    pub shifted: bool,
    pub throttle: f32,
    pub brake: f32,
}

#[derive(Clone, Debug)]
pub struct RumbleMixer {
    /// Master strength 0..1 (0 disables).
    pub strength: f32,
    pub triggers: bool,
    impact_env: f32,
    shift_env: f32,
    jolt_env: f32,
    phase: f32,
}

impl Default for RumbleMixer {
    fn default() -> Self {
        Self {
            strength: 0.8,
            triggers: true,
            impact_env: 0.0,
            shift_env: 0.0,
            jolt_env: 0.0,
            phase: 0.0,
        }
    }
}

impl RumbleMixer {
    pub fn update(&mut self, dt: f32, i: &RumbleInputs) -> Rumble {
        if self.strength <= 0.0 {
            return Rumble::default();
        }
        self.phase += dt;
        // Envelopes for one-shot events.
        let impact = (i.impact / (i.mass.max(500.0) * 6.0)).min(1.0);
        self.impact_env = self.impact_env.max(impact);
        if i.shifted {
            self.shift_env = 1.0;
        }
        self.jolt_env = self
            .jolt_env
            .max(((i.suspension_jolt - 0.6) / 2.0).clamp(0.0, 1.0));
        let decay = |v: &mut f32, rate: f32| *v = (*v - dt * rate).max(0.0);

        let speed_f = (i.speed.abs() / 25.0).min(1.0);
        let road = i.roughness * speed_f * (0.75 + 0.25 * (self.phase * 37.0).sin());
        let limiter = if i.limiter {
            0.35 + 0.15 * (self.phase * 60.0).sin().abs()
        } else {
            0.0
        };
        let idle = if i.engine_running {
            0.02 + 0.04 * i.rpm_fraction * i.throttle
        } else {
            0.0
        };
        let low = (self.impact_env + self.jolt_env * 0.7 + road * 0.6 + limiter + idle).min(1.0);

        let slide = ((i.slide_speed - 1.0) / 6.0).clamp(0.0, 1.0);
        let spin = (i.wheelspin / 0.6).clamp(0.0, 1.0);
        let abs = if i.abs_active {
            0.45 * (0.5 + 0.5 * (self.phase * 2.0 * std::f32::consts::PI * 14.0).sin())
        } else {
            0.0
        };
        let high = (slide * 0.55
            + spin * 0.35
            + self.shift_env * 0.5
            + abs
            + road * 0.25
            + self.impact_env * 0.5)
            .min(1.0);

        let (lt, rt) = if self.triggers {
            let abs_pulse = if i.abs_active {
                0.6 * (0.5 + 0.5 * (self.phase * 2.0 * std::f32::consts::PI * 12.0).sin())
            } else {
                0.0
            };
            let lock = if i.wheel_locked && i.brake > 0.2 {
                0.5
            } else {
                0.0
            };
            let tcs = if i.tcs_active {
                0.45 * (0.5 + 0.5 * (self.phase * 2.0 * std::f32::consts::PI * 9.0).sin())
            } else {
                0.0
            };
            let rt = (spin * 0.6 * i.throttle + tcs + if i.limiter { 0.25 } else { 0.0 }).min(1.0);
            ((abs_pulse + lock).min(1.0), rt)
        } else {
            (0.0, 0.0)
        };

        decay(&mut self.impact_env, 3.5);
        decay(&mut self.shift_env, 9.0);
        decay(&mut self.jolt_env, 6.0);
        let s = self.strength;
        Rumble {
            low: low * s,
            high: high * s,
            left_trigger: lt * s,
            right_trigger: rt * s,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_kick_and_decay() {
        let mut m = RumbleMixer::default();
        let calm = m.update(
            1.0 / 60.0,
            &RumbleInputs {
                speed: 10.0,
                engine_running: true,
                ..Default::default()
            },
        );
        assert!(calm.low < 0.1 && calm.high < 0.1);
        let hit = m.update(
            1.0 / 60.0,
            &RumbleInputs {
                impact: 20000.0,
                mass: 1500.0,
                ..Default::default()
            },
        );
        assert!(hit.low > 0.5);
        let mut later = hit;
        for _ in 0..60 {
            later = m.update(1.0 / 60.0, &RumbleInputs::default());
        }
        assert!(later.low < 0.05, "impact rumble should fade: {}", later.low);
        let abs = m.update(
            1.0 / 60.0,
            &RumbleInputs {
                abs_active: true,
                brake: 1.0,
                ..Default::default()
            },
        );
        assert!(abs.left_trigger > 0.0 || abs.high > 0.0);
    }
}
