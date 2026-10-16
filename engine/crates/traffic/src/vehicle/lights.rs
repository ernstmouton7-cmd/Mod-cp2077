//! Lamps, signals and other driver-operated equipment.

use crate::math::scalar::fmod;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Indicator {
    Off,
    Left,
    Right,
}

/// Rotary headlight switch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HeadlightSwitch {
    Off,
    Auto,
    Low,
}

impl HeadlightSwitch {
    pub fn label(self) -> &'static str {
        match self {
            HeadlightSwitch::Off => "OFF",
            HeadlightSwitch::Auto => "AUTO",
            HeadlightSwitch::Low => "LOW",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WiperMode {
    Off,
    Intermittent,
    Low,
    High,
}

impl WiperMode {
    pub fn label(self) -> &'static str {
        match self {
            WiperMode::Off => "OFF",
            WiperMode::Intermittent => "INTERMITTENT",
            WiperMode::Low => "LOW",
            WiperMode::High => "HIGH",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SirenMode {
    Off,
    Wail,
    Yelp,
}

/// Lamps, signals and other driver-operated equipment: indicators with
/// steering-column auto-cancel, hazards, low/high beam, flash-to-pass, brake
/// and reverse lamps, horn, wipers and emergency lightbar/siren.
#[derive(Clone, Debug)]
pub struct VehicleLights {
    pub headlight_switch: HeadlightSwitch,
    pub high_beam: bool,
    /// Momentary flash-to-pass (held).
    pub flash: bool,
    pub indicator: Indicator,
    pub hazard: bool,
    pub brake: f64,
    pub reverse: bool,
    pub horn: bool,
    /// Duration of the current horn press (s); 0 when not honking.
    pub horn_time: f64,
    /// Count of horn presses.
    pub horn_presses: u32,
    pub wipers: WiperMode,
    /// Wiper arm position 0..1..0 (sweep).
    pub wiper_pos: f64,
    wiper_timer: f64,
    wiper_dir: i32,
    pub siren: SirenMode,
    pub lightbar: bool,
    /// Lamps broken by collision damage.
    pub left_headlight_broken: bool,
    pub right_headlight_broken: bool,
    clock: f64,
    cancel_armed: bool,
    /// Dark enough for "auto" lights (set by the environment).
    pub dark: bool,
    /// Seconds the current indicator has been on.
    pub indicator_time: f64,
    /// Blink phase offset so AI cars don't blink in sync.
    pub phase: f64,
}

impl VehicleLights {
    pub fn new(seed: f64) -> Self {
        VehicleLights {
            headlight_switch: HeadlightSwitch::Auto,
            high_beam: false,
            flash: false,
            indicator: Indicator::Off,
            hazard: false,
            brake: 0.0,
            reverse: false,
            horn: false,
            horn_time: 0.0,
            horn_presses: 0,
            wipers: WiperMode::Off,
            wiper_pos: 0.0,
            wiper_timer: 0.0,
            wiper_dir: 0,
            siren: SirenMode::Off,
            lightbar: false,
            left_headlight_broken: false,
            right_headlight_broken: false,
            clock: 0.0,
            cancel_armed: false,
            dark: false,
            indicator_time: 0.0,
            phase: fmod(seed * 0.618, 1.0) * 0.8,
        }
    }

    /// Relay state of the turn signals (~85 blinks/min).
    pub fn blink_on(&self) -> bool {
        fmod(self.clock + self.phase, 0.7) < 0.38
    }

    pub fn left_lamp_on(&self) -> bool {
        (self.hazard || self.indicator == Indicator::Left) && self.blink_on()
    }

    pub fn right_lamp_on(&self) -> bool {
        (self.hazard || self.indicator == Indicator::Right) && self.blink_on()
    }

    pub fn low_beam_on(&self) -> bool {
        match self.headlight_switch {
            HeadlightSwitch::Low => true,
            HeadlightSwitch::Auto => self.dark,
            HeadlightSwitch::Off => false,
        }
    }

    /// High beam lit (switched or flashing). Flash works even with lights off.
    pub fn high_beam_on(&self) -> bool {
        self.flash || (self.high_beam && self.low_beam_on())
    }

    pub fn any_headlight(&self) -> bool {
        self.low_beam_on() || self.high_beam_on()
    }

    /// Lightbar colour phase: 0 = red/left, 1 = blue/right.
    pub fn lightbar_phase(&self) -> u8 {
        if fmod(self.clock * 3.0, 1.0) < 0.5 {
            0
        } else {
            1
        }
    }

    pub fn set_indicator(&mut self, dir: Indicator) {
        if dir == self.indicator {
            return;
        }
        self.indicator = dir;
        self.cancel_armed = false;
        self.indicator_time = 0.0;
    }

    /// Toggle like a real stalk: pressing the active side cancels.
    pub fn toggle_indicator(&mut self, dir: Indicator) {
        self.set_indicator(if self.indicator == dir { Indicator::Off } else { dir });
    }

    pub fn cycle_headlights(&mut self) {
        self.headlight_switch = match self.headlight_switch {
            HeadlightSwitch::Off => HeadlightSwitch::Auto,
            HeadlightSwitch::Auto => HeadlightSwitch::Low,
            HeadlightSwitch::Low => HeadlightSwitch::Off,
        };
    }

    pub fn cycle_wipers(&mut self) {
        self.wipers = match self.wipers {
            WiperMode::Off => WiperMode::Intermittent,
            WiperMode::Intermittent => WiperMode::Low,
            WiperMode::Low => WiperMode::High,
            WiperMode::High => WiperMode::Off,
        };
    }

    pub fn cycle_siren(&mut self) {
        self.siren = match self.siren {
            SirenMode::Off => SirenMode::Wail,
            SirenMode::Wail => SirenMode::Yelp,
            SirenMode::Yelp => SirenMode::Off,
        };
        self.lightbar = self.siren != SirenMode::Off || self.lightbar;
        if self.siren == SirenMode::Off {
            self.lightbar = false;
        }
    }

    /// Advance timers; `steer` is the front wheel angle (rad, + = left) used
    /// for the indicator auto-cancel.
    pub fn update(&mut self, dt: f64, steer: f64) {
        self.clock += dt;
        if self.horn {
            self.horn_time += dt;
        } else {
            self.horn_time = 0.0;
        }
        if self.indicator != Indicator::Off {
            self.indicator_time += dt;
            let toward = if self.indicator == Indicator::Left { steer } else { -steer };
            if toward > 0.18 {
                self.cancel_armed = true;
            } else if self.cancel_armed && toward < 0.04 {
                self.indicator = Indicator::Off;
                self.cancel_armed = false;
                self.indicator_time = 0.0;
            }
        }
        // Wipers: a sweep takes ~1.1 s (low), 0.7 s (high), intermittent pauses 3 s.
        let sweep = if self.wipers == WiperMode::High { 0.7 } else { 1.1 };
        if self.wipers == WiperMode::Off && self.wiper_dir == 0 {
            self.wiper_pos = 0.0;
            return;
        }
        if self.wiper_dir == 0 {
            self.wiper_timer -= dt;
            if self.wiper_timer <= 0.0 && self.wipers != WiperMode::Off {
                self.wiper_dir = 1;
            }
        } else {
            self.wiper_pos += ((self.wiper_dir as f64 * 2.0) / sweep) * dt;
            if self.wiper_pos >= 1.0 {
                self.wiper_pos = 1.0;
                self.wiper_dir = -1;
            } else if self.wiper_pos <= 0.0 {
                self.wiper_pos = 0.0;
                self.wiper_dir = 0;
                self.wiper_timer = if self.wipers == WiperMode::Intermittent { 3.0 } else { 0.0 };
            }
        }
    }
}
