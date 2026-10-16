//! Driver actions with default keyboard and gamepad bindings.
//!
//! Gamepad layout (Xbox names), designed so every action is reachable without a keyboard:
//!
//! | Input | Tap / press | Hold | With View held (modifier layer) |
//! |---|---|---|---|
//! | RT / LT | throttle / brake (reverse at a stop) | | |
//! | Left stick | steer | | |
//! | Right stick | look around (camera) | | |
//! | A | handbrake | | engine start/stop |
//! | B | shift up | | seatbelt |
//! | X | shift down | | auto/manual gearbox |
//! | Y | change camera | look back | siren |
//! | LB / RB | left / right indicator | | toggle TCS / toggle ABS |
//! | LB + RB | hazards | | |
//! | LS click | horn | long blast | reset vehicle |
//! | RS click | look back | | toggle ESC |
//! | D-pad ↑ | headlights | high beam | cruise control |
//! | D-pad ↓ | flash high beams | | wipers |
//! | D-pad ← | wave / sorry | | cruise −5 |
//! | D-pad → | rude gesture | | cruise +5 |
//! | Menu | pause | | |
//! | View (tap) | map | | |

use crate::pad::{PadAxis, PadButton};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Action {
    Throttle,
    Brake,
    SteerLeft,
    SteerRight,
    Handbrake,
    Clutch,
    ShiftUp,
    ShiftDown,
    GearReverse,
    GearNeutral,
    TransmissionMode,
    Horn,
    IndicatorLeft,
    IndicatorRight,
    Hazards,
    Headlights,
    HighBeam,
    Flash,
    Wipers,
    Siren,
    CruiseToggle,
    CruiseUp,
    CruiseDown,
    Limiter,
    Engine,
    Seatbelt,
    ToggleAbs,
    ToggleTcs,
    ToggleEsc,
    Apologize,
    RudeGesture,
    Camera,
    LookBack,
    LookLeft,
    LookRight,
    ResetVehicle,
    Pause,
    Help,
    Map,
    Debug,
    TimeForward,
    WeatherCycle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionKind {
    /// Continuous 0..1 value.
    Analog,
    /// Matters while held.
    Hold,
    /// Edge-triggered.
    Press,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    Driving,
    Gearbox,
    SignalsLights,
    ComfortAssists,
    Social,
    Camera,
    System,
}

/// What on the pad triggers an action.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PadInput {
    Button(PadButton),
    /// Axis beyond +0.5 (or past the trigger threshold).
    AxisPos(PadAxis),
    AxisNeg(PadAxis),
    /// Two buttons pressed together.
    Chord(PadButton, PadButton),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Gesture {
    /// Fires on press (and is "held" while down).
    Press,
    /// Fires on release if held shorter than the tap time.
    Tap,
    /// Fires once when held longer than the hold time (and is "held" afterwards).
    Hold,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Layer {
    Base,
    /// While the modifier button (View) is held.
    Modifier,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PadBinding {
    pub input: PadInput,
    pub gesture: Gesture,
    pub layer: Layer,
}

impl PadBinding {
    pub const fn press(b: PadButton) -> Self {
        Self {
            input: PadInput::Button(b),
            gesture: Gesture::Press,
            layer: Layer::Base,
        }
    }
    pub const fn tap(b: PadButton) -> Self {
        Self {
            input: PadInput::Button(b),
            gesture: Gesture::Tap,
            layer: Layer::Base,
        }
    }
    pub const fn hold(b: PadButton) -> Self {
        Self {
            input: PadInput::Button(b),
            gesture: Gesture::Hold,
            layer: Layer::Base,
        }
    }
    pub const fn modifier(b: PadButton) -> Self {
        Self {
            input: PadInput::Button(b),
            gesture: Gesture::Press,
            layer: Layer::Modifier,
        }
    }
    pub const fn axis(a: PadAxis) -> Self {
        Self {
            input: PadInput::AxisPos(a),
            gesture: Gesture::Press,
            layer: Layer::Base,
        }
    }
    pub const fn axis_neg(a: PadAxis) -> Self {
        Self {
            input: PadInput::AxisNeg(a),
            gesture: Gesture::Press,
            layer: Layer::Base,
        }
    }
    pub const fn chord(a: PadButton, b: PadButton) -> Self {
        Self {
            input: PadInput::Chord(a, b),
            gesture: Gesture::Press,
            layer: Layer::Base,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ActionDef {
    pub action: Action,
    pub id: &'static str,
    pub label: &'static str,
    pub category: Category,
    pub kind: ActionKind,
    /// Key codes in the W3C `KeyboardEvent.code` naming (same as winit's `KeyCode` debug names).
    pub keys: &'static [&'static str],
    pub pad: &'static [PadBinding],
    pub help: &'static str,
}

use Action::*;
use ActionKind::*;
use Category::*;
use PadButton as B;

macro_rules! def {
    ($a:expr, $id:expr, $label:expr, $cat:expr, $kind:expr, $keys:expr, $pad:expr, $help:expr) => {
        ActionDef {
            action: $a,
            id: $id,
            label: $label,
            category: $cat,
            kind: $kind,
            keys: $keys,
            pad: $pad,
            help: $help,
        }
    };
}

pub static ACTIONS: &[ActionDef] = &[
    def!(Throttle, "throttle", "Throttle", Driving, Analog, &["KeyW", "ArrowUp"], &[PadBinding::axis(PadAxis::RightTrigger)], "Accelerate. With the automatic gearbox in reverse, this brakes."),
    def!(Brake, "brake", "Brake / reverse", Driving, Analog, &["KeyS", "ArrowDown"], &[PadBinding::axis(PadAxis::LeftTrigger)], "Brake. With the automatic gearbox, keep holding at a standstill to reverse."),
    def!(SteerLeft, "steer_left", "Steer left", Driving, Analog, &["KeyA", "ArrowLeft"], &[PadBinding::axis_neg(PadAxis::LeftX)], "Steer left."),
    def!(SteerRight, "steer_right", "Steer right", Driving, Analog, &["KeyD", "ArrowRight"], &[PadBinding::axis(PadAxis::LeftX)], "Steer right."),
    def!(Handbrake, "handbrake", "Handbrake", Driving, Hold, &["Space"], &[PadBinding::press(B::South)], "Locks the rear wheels: parking, hairpins and drifts."),
    def!(Clutch, "clutch", "Clutch", Gearbox, Analog, &["KeyZ"], &[], "Manual gearbox without auto-clutch: press to shift or launch without stalling. On a pad, LB becomes the clutch when the clutch assist is off."),
    def!(ShiftUp, "shift_up", "Shift up", Gearbox, Press, &["ShiftLeft", "KeyR"], &[PadBinding::press(B::East)], "Next gear (manual)."),
    def!(ShiftDown, "shift_down", "Shift down", Gearbox, Press, &["ControlLeft", "KeyF"], &[PadBinding::press(B::West)], "Previous gear (manual)."),
    def!(GearReverse, "gear_reverse", "Select reverse", Gearbox, Press, &["Backspace"], &[], "Select R directly."),
    def!(GearNeutral, "gear_neutral", "Select neutral", Gearbox, Press, &["KeyN"], &[], "Select N directly."),
    def!(TransmissionMode, "transmission_mode", "Auto / manual", Gearbox, Press, &["KeyT"], &[PadBinding::modifier(B::West)], "Toggle automatic and manual transmission."),
    def!(Horn, "horn", "Horn", SignalsLights, Hold, &["KeyH"], &[PadBinding::press(B::LeftStick)], "Tap for a beep, hold for a long blast (other drivers notice)."),
    def!(IndicatorLeft, "indicator_left", "Left indicator", SignalsLights, Press, &["KeyQ"], &[PadBinding::tap(B::LeftBumper)], "Toggle left turn signal (auto-cancels after the turn)."),
    def!(IndicatorRight, "indicator_right", "Right indicator", SignalsLights, Press, &["KeyE"], &[PadBinding::tap(B::RightBumper)], "Toggle right turn signal (auto-cancels after the turn)."),
    def!(Hazards, "hazards", "Hazard lights", SignalsLights, Press, &["KeyX"], &[PadBinding::chord(B::LeftBumper, B::RightBumper)], "All four indicators. A quick blink after merging says thanks."),
    def!(Headlights, "headlights", "Headlights", SignalsLights, Press, &["KeyL"], &[PadBinding::tap(B::DpadUp)], "Cycle the light switch: OFF, AUTO, ON."),
    def!(HighBeam, "high_beam", "High beam", SignalsLights, Press, &["KeyK"], &[PadBinding::hold(B::DpadUp)], "Toggle high beams (dazzles oncoming drivers)."),
    def!(Flash, "flash", "Flash high beams", SignalsLights, Hold, &["KeyJ"], &[PadBinding::press(B::DpadDown)], "Flash to pass / warn. Drivers ahead take it personally."),
    def!(Wipers, "wipers", "Wipers", SignalsLights, Press, &["KeyP"], &[PadBinding::modifier(B::DpadDown)], "Cycle wipers: OFF, INT, LOW, HIGH."),
    def!(Siren, "siren", "Siren", SignalsLights, Press, &["KeyO"], &[PadBinding::modifier(B::North)], "Emergency vehicles only: cycle siren."),
    def!(CruiseToggle, "cruise_toggle", "Cruise control", ComfortAssists, Press, &["KeyC"], &[PadBinding::modifier(B::DpadUp)], "Set / cancel cruise at the current speed. Brake also cancels."),
    def!(CruiseUp, "cruise_up", "Cruise +5", ComfortAssists, Press, &["Equal", "NumpadAdd"], &[PadBinding::modifier(B::DpadRight)], "Raise cruise speed by 5 km/h."),
    def!(CruiseDown, "cruise_down", "Cruise -5", ComfortAssists, Press, &["Minus", "NumpadSubtract"], &[PadBinding::modifier(B::DpadLeft)], "Lower cruise speed by 5 km/h."),
    def!(Limiter, "limiter", "Speed limiter", ComfortAssists, Press, &["KeyU"], &[], "Cap speed at the posted limit."),
    def!(Engine, "engine", "Engine start / stop", ComfortAssists, Press, &["KeyI"], &[PadBinding::modifier(B::South)], "Turn the ignition. Needed after a stall."),
    def!(Seatbelt, "seatbelt", "Seatbelt", ComfortAssists, Press, &["KeyB"], &[PadBinding::modifier(B::East)], "Buckle up. Police fine unbelted drivers."),
    def!(ToggleAbs, "toggle_abs", "ABS on/off", ComfortAssists, Press, &["Digit1"], &[PadBinding::modifier(B::RightBumper)], "Anti-lock brakes."),
    def!(ToggleTcs, "toggle_tcs", "Traction control on/off", ComfortAssists, Press, &["Digit2"], &[PadBinding::modifier(B::LeftBumper)], "Traction control (off = burnouts)."),
    def!(ToggleEsc, "toggle_esc", "Stability control on/off", ComfortAssists, Press, &["Digit3"], &[PadBinding::modifier(B::RightStick)], "Electronic stability control."),
    def!(Apologize, "apologize", "Wave / sorry", Social, Press, &["KeyG"], &[PadBinding::press(B::DpadLeft)], "Apologise to nearby drivers. Calms people you just upset."),
    def!(RudeGesture, "rude_gesture", "Rude gesture", Social, Press, &["KeyY"], &[PadBinding::press(B::DpadRight)], "Insult nearby drivers. Expect consequences."),
    def!(Action::Camera, "camera", "Change camera", Category::Camera, Press, &["KeyV"], &[PadBinding::tap(B::North)], "Chase, far chase, hood, cockpit, top-down, cinematic."),
    def!(LookBack, "look_back", "Look back", Category::Camera, Hold, &["KeyM"], &[PadBinding::press(B::RightStick), PadBinding::hold(B::North)], "Hold to look behind."),
    def!(LookLeft, "look_left", "Look left", Category::Camera, Hold, &["Comma"], &[], "Hold to look left."),
    def!(LookRight, "look_right", "Look right", Category::Camera, Hold, &["Period"], &[], "Hold to look right."),
    def!(ResetVehicle, "reset_vehicle", "Reset vehicle", System, Press, &["Backquote"], &[PadBinding::modifier(B::LeftStick)], "Put the car back on the nearest road (repair costs money)."),
    def!(Pause, "pause", "Pause / menu", System, Press, &["Escape"], &[PadBinding::press(B::Menu)], "Pause and open the menu."),
    def!(Help, "help", "Controls help", System, Press, &["F1"], &[], "Show all controls."),
    def!(Map, "map", "Map", System, Press, &["Tab"], &[PadBinding::tap(B::View)], "Toggle the big map."),
    def!(Debug, "debug", "Debug overlay", System, Press, &["F3"], &[], "Show AI moods, intentions and signal phases."),
    def!(TimeForward, "time_forward", "Skip time", System, Press, &["BracketRight"], &[], "Advance the clock by one hour."),
    def!(WeatherCycle, "weather_cycle", "Change weather", System, Press, &["BracketLeft"], &[], "Cycle clear, rain, storm, fog."),
];

impl Action {
    pub fn def(self) -> &'static ActionDef {
        ACTIONS
            .iter()
            .find(|d| d.action == self)
            .expect("every action has a definition")
    }
    pub fn id(self) -> &'static str {
        self.def().id
    }
    pub fn from_id(id: &str) -> Option<Action> {
        ACTIONS.iter().find(|d| d.id == id).map(|d| d.action)
    }
}

/// Button used as the modifier layer key.
pub const MODIFIER: PadButton = PadButton::View;
