//! The player's driver: turns control frames into vehicle inputs and operates
//! the car's equipment.

use super::actions::{ActionId, ControlFrame};
use crate::math::scalar::{clamp, KMH};
use crate::vehicle::{Indicator, TransmissionMode, Vehicle};
use crate::world::events::{Event, EventQueue, GestureKind};

/// Driving aids and preferences.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayerSettings {
    /// Manual gearbox: automatic clutch (no stalling) when the clutch is not used.
    pub clutch_assist: bool,
    /// Automatic gearbox: holding brake at a standstill selects reverse.
    pub auto_reverse: bool,
    /// Steering sensitivity multiplier.
    pub steer_sensitivity: f64,
}

/// Cruise control state.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CruiseState {
    pub on: bool,
    /// Target speed (m/s).
    pub target: f64,
    pub integral: f64,
}

/// Actions the simulation does not handle itself (forwarded to the host).
pub const HOST_ACTIONS: [ActionId; 11] = [
    ActionId::Camera,
    ActionId::LookBack,
    ActionId::LookLeft,
    ActionId::LookRight,
    ActionId::Pause,
    ActionId::Help,
    ActionId::Map,
    ActionId::Debug,
    ActionId::TimeForward,
    ActionId::WeatherCycle,
    ActionId::ResetVehicle,
];

/// Turns the player's [`ControlFrame`] into vehicle inputs and operates every
/// piece of equipment: gearbox (auto/manual, park, auto-reverse, clutch,
/// stalls, grinding), cruise control, speed limiter, ignition, lights,
/// indicators, hazards, wipers, horn, seatbelt and driver assists.
#[derive(Clone, Debug)]
pub struct PlayerDriver {
    pub settings: PlayerSettings,
    pub cruise: CruiseState,
    pub limiter_on: bool,
    pub seatbelt: bool,
    /// Automatic gearbox in Park.
    pub parked: bool,
    reverse_timer: f64,
    seatbelt_chime: f64,
    /// Host-side actions pressed this step.
    pub host_actions: Vec<ActionId>,
    /// Gesture requested this step (handled by the world's social system).
    pub gesture: Option<GestureKind>,
    /// HUD notifications.
    pub notes: Vec<String>,
    was_stalled: bool,
    clutch_now: f64,
}

impl Default for PlayerDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl PlayerDriver {
    pub fn new() -> Self {
        PlayerDriver {
            settings: PlayerSettings { clutch_assist: true, auto_reverse: true, steer_sensitivity: 1.0 },
            cruise: CruiseState::default(),
            limiter_on: false,
            seatbelt: true,
            parked: true,
            reverse_timer: 0.0,
            seatbelt_chime: 0.0,
            host_actions: Vec::new(),
            gesture: None,
            notes: Vec::new(),
            was_stalled: false,
            clutch_now: 0.0,
        }
    }

    /// Gear selector text: P R N D1.. (automatic) or R N 1..6 (manual).
    pub fn gear_text(&self, v: &Vehicle) -> String {
        if v.pt.mode == TransmissionMode::Automatic {
            if self.parked {
                return "P".into();
            }
            let l = v.pt.gear_label();
            return if l == "R" || l == "N" { l } else { format!("D{l}") };
        }
        v.pt.gear_label()
    }

    fn note(&mut self, s: String) {
        self.notes.push(s);
        if self.notes.len() > 8 {
            self.notes.remove(0);
        }
    }

    /// Apply one control frame to the player's vehicle.
    pub fn update(&mut self, dt: f64, f: &ControlFrame, v: &mut Vehicle, events: &mut EventQueue, dark: bool, speed_limit: f64) {
        self.gesture = None;
        self.clutch_now = f.clutch;
        for &id in &f.pressed {
            self.on_press(id, v, events, speed_limit);
        }
        let vid = v.id;
        // Horn & flash are hold actions.
        let horn_held = f.held.contains(&ActionId::Horn);
        if horn_held && !v.lights.horn {
            v.lights.horn_presses += 1;
            events.emit(Event::Horn { vehicle: vid, long: false });
        }
        v.lights.horn = horn_held;
        let flash_held = f.held.contains(&ActionId::Flash);
        if flash_held && !v.lights.flash {
            events.emit(Event::Flash { vehicle: vid });
        }
        v.lights.flash = flash_held;
        for &(id, held) in &f.released {
            if id == ActionId::Horn && held > 0.9 {
                events.emit(Event::Horn { vehicle: vid, long: true });
            }
        }

        // --- Pedals and gear selection.
        let mut throttle = f.throttle;
        let mut brake = f.brake;
        let u = v.dynamics.u;
        let pt = &mut v.pt;
        if pt.mode == TransmissionMode::Automatic && self.parked {
            // P: pawl holds the car; throttle selects D, brake selects R.
            if pt.gear != 0 {
                pt.shift_to(0);
            }
            if throttle > 0.15 && pt.running {
                self.parked = false;
                pt.shift_to(1);
            } else if brake > 0.15 && throttle < 0.05 && pt.running && self.settings.auto_reverse {
                self.reverse_timer += dt;
                if self.reverse_timer > 0.4 {
                    self.parked = false;
                    pt.shift_to(-1);
                    self.reverse_timer = 0.0;
                }
            }
        } else if pt.mode == TransmissionMode::Automatic {
            if !pt.running && u.abs() < 0.3 {
                self.parked = true;
            }
            if self.settings.auto_reverse {
                if pt.gear >= 1 {
                    if brake > 0.15 && throttle < 0.05 && u < 0.4 {
                        self.reverse_timer += dt;
                        if self.reverse_timer > 0.3 && u.abs() < 0.3 {
                            pt.shift_to(-1);
                            self.reverse_timer = 0.0;
                        }
                    } else {
                        self.reverse_timer = 0.0;
                    }
                } else if pt.gear == -1 {
                    if throttle > 0.15 && brake < 0.05 && u > -0.4 {
                        self.reverse_timer += dt;
                        if self.reverse_timer > 0.2 && u.abs() < 0.3 {
                            pt.shift_to(1);
                            self.reverse_timer = 0.0;
                        }
                    } else {
                        self.reverse_timer = 0.0;
                    }
                    // In reverse the brake key drives backwards, throttle brakes.
                    std::mem::swap(&mut throttle, &mut brake);
                }
            }
            pt.clutch_pedal = 0.0;
        } else {
            // Manual gearbox.
            if self.settings.clutch_assist && f.clutch < 0.05 {
                let wheel_rpm = (v.dynamics.driven_omega() * pt.ratio(pt.gear) * v.spec.final_drive).abs() * 9.549;
                let idle = v.spec.idle_rpm;
                if pt.shifting() {
                    pt.clutch_pedal = 1.0;
                } else if pt.gear != 0 && wheel_rpm < idle * 1.15 {
                    pt.clutch_pedal = clamp(1.0 - throttle * 1.6 - (wheel_rpm / idle) * 0.5, 0.0, 1.0);
                } else {
                    pt.clutch_pedal = 0.0;
                }
            } else {
                pt.clutch_pedal = f.clutch;
            }
            if pt.gear == -1 && u > 1.5 && throttle > 0.2 {
                // Grinding into reverse while rolling forward: the box refuses.
                pt.shift_to(0);
            }
        }
        if v.pt.stalled && !self.was_stalled {
            events.emit(Event::Stall { vehicle: vid });
            self.note("Engine stalled — press Start (I)".into());
            self.cruise.on = false;
        }
        self.was_stalled = v.pt.stalled;

        // --- Cruise control & limiter.
        if self.cruise.on {
            if f.brake > 0.08 || f.clutch > 0.5 || f.handbrake > 0.5 || !v.pt.running {
                self.cruise.on = false;
                self.note("Cruise cancelled".into());
            } else if v.pt.gear >= 1 {
                let cr = &mut self.cruise;
                let err = cr.target - u;
                cr.integral = clamp(cr.integral + err * dt, -8.0, 8.0);
                let cmd = clamp(0.18 * err + 0.06 * cr.integral + 0.15, 0.0, 1.0);
                throttle = throttle.max(cmd);
            }
        }
        v.pt.limiter = if self.limiter_on { speed_limit.max(5.0) } else { 0.0 };

        v.inputs.throttle = if v.pt.running || v.pt.cranking > 0.0 { throttle } else { 0.0 };
        v.inputs.brake = brake;
        v.inputs.handbrake = if self.parked && v.pt.mode == TransmissionMode::Automatic { 1.0 } else { f.handbrake };
        // Steering: full lock at parking speeds, reduced at speed.
        let speed_factor = 1.0 / (1.0 + u.abs() / 35.0);
        v.steer_target = -f.steer * v.spec.max_steer * speed_factor * self.settings.steer_sensitivity;

        v.lights.brake = if v.inputs.brake > 0.05 { 1.0 } else { 0.0 };
        v.lights.reverse = v.pt.gear == -1;
        v.lights.dark = dark;

        if !self.seatbelt && u.abs() > 3.0 {
            self.seatbelt_chime -= dt;
            if self.seatbelt_chime <= 0.0 {
                self.seatbelt_chime = 6.0;
                self.note("Fasten seatbelt (B)".into());
                events.emit(Event::Chime { vehicle: vid, reason: "seatbelt" });
            }
        }
    }

    /// Manual box without clutch assist: shifting with the clutch out grinds.
    fn grinds(&self, v: &Vehicle) -> bool {
        v.pt.mode == TransmissionMode::Manual && !self.settings.clutch_assist && v.pt.running && self.clutch_now < 0.5
    }

    fn on_press(&mut self, id: ActionId, v: &mut Vehicle, events: &mut EventQueue, speed_limit: f64) {
        let vid = v.id;
        match id {
            ActionId::ShiftUp => {
                if self.grinds(v) {
                    events.emit(Event::Grind { vehicle: vid });
                } else if v.pt.mode == TransmissionMode::Manual && v.pt.shift_up() {
                    events.emit(Event::Shift { vehicle: vid, gear: v.pt.gear_label() });
                } else if v.pt.mode == TransmissionMode::Automatic && v.pt.gear <= 0 {
                    self.parked = false;
                    v.pt.shift_to(1);
                }
            }
            ActionId::ShiftDown => {
                if self.grinds(v) {
                    events.emit(Event::Grind { vehicle: vid });
                } else if v.pt.mode == TransmissionMode::Manual && v.pt.shift_down() {
                    events.emit(Event::Shift { vehicle: vid, gear: v.pt.gear_label() });
                } else if v.pt.mode == TransmissionMode::Automatic && v.dynamics.u.abs() < 0.5 {
                    self.parked = true;
                }
            }
            ActionId::GearReverse => {
                if v.dynamics.u.abs() < 1.5 {
                    self.parked = false;
                    v.pt.shift_to(-1);
                }
            }
            ActionId::GearNeutral => {
                self.parked = false;
                v.pt.shift_to(0);
            }
            ActionId::TransmissionMode => {
                v.pt.mode = if v.pt.mode == TransmissionMode::Automatic { TransmissionMode::Manual } else { TransmissionMode::Automatic };
                self.parked = false;
                if v.pt.mode == TransmissionMode::Automatic && v.pt.gear == 0 {
                    v.pt.shift_to(1);
                }
                self.note(if v.pt.mode == TransmissionMode::Manual { "Manual transmission" } else { "Automatic transmission" }.into());
            }
            ActionId::IndicatorLeft => v.lights.toggle_indicator(Indicator::Left),
            ActionId::IndicatorRight => v.lights.toggle_indicator(Indicator::Right),
            ActionId::Hazards => v.lights.hazard = !v.lights.hazard,
            ActionId::Headlights => {
                v.lights.cycle_headlights();
                self.note(format!("Lights: {}", v.lights.headlight_switch.label()));
            }
            ActionId::HighBeam => v.lights.high_beam = !v.lights.high_beam,
            ActionId::Wipers => {
                v.lights.cycle_wipers();
                self.note(format!("Wipers: {}", v.lights.wipers.label()));
            }
            ActionId::Siren => {
                if v.spec.emergency {
                    v.lights.cycle_siren();
                }
            }
            ActionId::CruiseToggle => {
                if self.cruise.on {
                    self.cruise.on = false;
                    self.note("Cruise off".into());
                } else if v.dynamics.u > 30.0 * KMH * 0.5 && v.pt.running {
                    self.cruise = CruiseState { on: true, target: v.dynamics.u, integral: 0.0 };
                    self.note(format!("Cruise set {} km/h", (self.cruise.target * 3.6 + 0.5).floor()));
                } else {
                    self.note("Cruise needs at least 15 km/h".into());
                }
            }
            ActionId::CruiseUp => {
                if self.cruise.on {
                    self.cruise.target += 5.0 * KMH;
                    self.note(format!("Cruise {} km/h", (self.cruise.target * 3.6 + 0.5).floor()));
                }
            }
            ActionId::CruiseDown => {
                if self.cruise.on {
                    self.cruise.target = (15.0 * KMH).max(self.cruise.target - 5.0 * KMH);
                    self.note(format!("Cruise {} km/h", (self.cruise.target * 3.6 + 0.5).floor()));
                }
            }
            ActionId::Limiter => {
                self.limiter_on = !self.limiter_on;
                let n = if self.limiter_on { format!("Limiter {} km/h", (speed_limit * 3.6 + 0.5).floor()) } else { "Limiter off".into() };
                self.note(n);
            }
            ActionId::Engine => {
                if v.pt.running {
                    v.pt.stop();
                    events.emit(Event::Engine { vehicle: vid, on: false });
                    self.note("Engine off".into());
                } else {
                    v.pt.start();
                    events.emit(Event::Engine { vehicle: vid, on: true });
                }
            }
            ActionId::Seatbelt => {
                self.seatbelt = !self.seatbelt;
                self.note(if self.seatbelt { "Seatbelt fastened" } else { "Seatbelt unfastened" }.into());
            }
            ActionId::ToggleAbs => {
                v.assists.abs = !v.assists.abs;
                self.note(format!("ABS {}", if v.assists.abs { "on" } else { "off" }));
            }
            ActionId::ToggleTcs => {
                v.assists.tcs = !v.assists.tcs;
                self.note(format!("Traction control {}", if v.assists.tcs { "on" } else { "off" }));
            }
            ActionId::ToggleEsc => {
                v.assists.esc = !v.assists.esc;
                self.note(format!("Stability control {}", if v.assists.esc { "on" } else { "off" }));
            }
            ActionId::Apologize => self.gesture = Some(GestureKind::Wave),
            ActionId::RudeGesture => self.gesture = Some(GestureKind::Rude),
            other => {
                if HOST_ACTIONS.contains(&other) {
                    self.host_actions.push(other);
                }
            }
        }
    }
}
