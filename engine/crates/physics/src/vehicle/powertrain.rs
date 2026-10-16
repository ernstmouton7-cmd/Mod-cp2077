//! Engine, clutch, gearbox and differentials.

#[derive(Clone, Debug)]
pub struct EngineDesc {
    /// Full-throttle torque curve: (rpm, N·m), ascending rpm.
    pub torque_curve: Vec<(f32, f32)>,
    pub idle_rpm: f32,
    pub redline: f32,
    pub stall_rpm: f32,
    /// Rotating inertia of engine + flywheel (kg·m²).
    pub inertia: f32,
    /// Closed-throttle friction torque at redline (N·m).
    pub engine_brake: f32,
}

impl EngineDesc {
    pub fn torque_at(&self, rpm: f32) -> f32 {
        let c = &self.torque_curve;
        if c.is_empty() {
            return 0.0;
        }
        if rpm <= c[0].0 {
            return c[0].1 * (rpm / c[0].0.max(1.0)).clamp(0.0, 1.0);
        }
        for w in c.windows(2) {
            if rpm <= w[1].0 {
                let t = (rpm - w[0].0) / (w[1].0 - w[0].0);
                return w[0].1 + (w[1].1 - w[0].1) * t;
            }
        }
        let last = c[c.len() - 1];
        // Falls off past the last point.
        (last.1 * (1.0 - (rpm - last.0) / 1500.0)).max(0.0)
    }
    pub fn peak_torque(&self) -> f32 {
        self.torque_curve.iter().fold(0.0, |m, p| m.max(p.1))
    }
    /// Peak power in W.
    pub fn peak_power(&self) -> f32 {
        let mut best: f32 = 0.0;
        let mut rpm = self.idle_rpm;
        while rpm <= self.redline {
            best = best.max(self.torque_at(rpm) * rpm * RPM_TO_RAD);
            rpm += 50.0;
        }
        best
    }
}

pub const RPM_TO_RAD: f32 = std::f32::consts::PI / 30.0;

#[derive(Clone, Debug)]
pub struct GearboxDesc {
    /// Forward ratios (1st first).
    pub ratios: Vec<f32>,
    pub reverse: f32,
    pub final_drive: f32,
    /// Time with the clutch open during a shift (s).
    pub shift_time: f32,
    pub efficiency: f32,
    /// Clutch torque capacity as a multiple of peak engine torque.
    pub clutch_capacity: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DriveLayout {
    Fwd,
    Rwd,
    /// Fixed torque split; `front` is the fraction sent to the front axle.
    Awd {
        front: f32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Differential {
    Open,
    /// Clutch-type LSD: locking torque = preload + ramp * |input torque|.
    Limited {
        preload: f32,
        ramp: f32,
    },
    Locked,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GearMode {
    /// Automatic: gear chosen by the shift logic.
    Automatic,
    /// Manual (sequential or H-pattern); `clutch` input is used unless auto-clutch is on.
    Manual,
}

/// Selector position for automatics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Selector {
    Park,
    Reverse,
    Neutral,
    Drive,
}

#[derive(Clone, Debug)]
pub struct PowertrainState {
    pub engine_omega: f32,
    /// -1 reverse, 0 neutral, 1.. forward.
    pub gear: i32,
    pub selector: Selector,
    pub mode: GearMode,
    pub running: bool,
    pub stalled: bool,
    /// Clutch engagement 0..1 actually applied this substep.
    pub clutch: f32,
    pub clutch_torque: f32,
    pub engine_torque: f32,
    pub shift_timer: f32,
    pub pending_gear: Option<i32>,
    pub since_shift: f32,
    pub limiter: bool,
    /// Throttle after TCS/limiter/shift cut.
    pub throttle_applied: f32,
    /// True when a manual shift was attempted without the clutch (gear grind).
    pub grinding: bool,
}

impl PowertrainState {
    pub fn new(idle_rpm: f32) -> Self {
        Self {
            engine_omega: idle_rpm * RPM_TO_RAD,
            gear: 0,
            selector: Selector::Drive,
            mode: GearMode::Automatic,
            running: true,
            stalled: false,
            clutch: 0.0,
            clutch_torque: 0.0,
            engine_torque: 0.0,
            shift_timer: 0.0,
            pending_gear: None,
            since_shift: 10.0,
            limiter: false,
            throttle_applied: 0.0,
            grinding: false,
        }
    }
    pub fn rpm(&self) -> f32 {
        self.engine_omega / RPM_TO_RAD
    }
    pub fn shifting(&self) -> bool {
        self.shift_timer > 0.0
    }
    /// Human-readable gear (P, R, N, D3, 3...).
    pub fn gear_text(&self) -> String {
        match (self.mode, self.selector, self.gear) {
            (GearMode::Automatic, Selector::Park, _) => "P".into(),
            (GearMode::Automatic, Selector::Neutral, _) => "N".into(),
            (_, _, g) if g < 0 => "R".into(),
            (_, _, 0) => "N".into(),
            (GearMode::Automatic, _, g) => format!("D{g}"),
            (GearMode::Manual, _, g) => format!("{g}"),
        }
    }
}

/// Overall ratio (engine revolutions per wheel revolution) for a gear.
pub fn gear_ratio(g: &GearboxDesc, gear: i32) -> f32 {
    if gear > 0 {
        g.ratios.get((gear - 1) as usize).copied().unwrap_or(1.0) * g.final_drive
    } else if gear < 0 {
        -g.reverse * g.final_drive
    } else {
        0.0
    }
}

/// Split axle torque between left/right wheels. Returns (left, right).
pub fn split_axle(
    diff: Differential,
    torque: f32,
    w_left: f32,
    w_right: f32,
    inertia: f32,
    h: f32,
) -> (f32, f32) {
    match diff {
        Differential::Open => (torque * 0.5, torque * 0.5),
        Differential::Limited { preload, ramp } => {
            let cap = preload + ramp * torque.abs();
            // Torque that would equalise the wheel speeds this substep.
            let lock = (w_left - w_right) * inertia / (2.0 * h);
            let t = lock.clamp(-cap, cap);
            (torque * 0.5 - t, torque * 0.5 + t)
        }
        Differential::Locked => {
            let lock = (w_left - w_right) * inertia / (2.0 * h);
            (torque * 0.5 - lock, torque * 0.5 + lock)
        }
    }
}
