//! Every driver control as an abstract action with default bindings, and the
//! input mapper that turns raw input into control frames.
//!
//! Keyboard codes follow `KeyboardEvent.code`; gamepad indices follow the W3C
//! "standard" mapping.

use std::collections::HashMap;

macro_rules! actions {
    ($($id:ident),* $(,)?) => {
        /// A driver or host action.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum ActionId { $($id),* }
        impl ActionId {
            /// All actions in definition order.
            pub const ALL: &'static [ActionId] = &[$(ActionId::$id),*];
        }
    };
}

actions!(
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
);

/// Number of actions.
pub const ACTION_COUNT: usize = ActionId::ALL.len();

/// How an action is driven.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ActionKind {
    Analog,
    Hold,
    Press,
}

/// Help/menu grouping.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ActionCategory {
    Driving,
    Gearbox,
    SignalsLights,
    ComfortAssists,
    Social,
    Camera,
    System,
}

/// A gamepad binding: a button, or an axis direction (+1 / -1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GamepadBinding {
    Button(usize),
    Axis(usize, i8),
}

/// Definition of an action with its default bindings.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActionDef {
    pub id: ActionId,
    /// Snake-case id as used by the TypeScript core (`"indicator_left"`).
    pub name: &'static str,
    pub label: &'static str,
    pub category: ActionCategory,
    pub kind: ActionKind,
    pub keys: &'static [&'static str],
    pub pad: &'static [GamepadBinding],
    pub help: &'static str,
}

use ActionCategory as C;
use ActionId as I;
use ActionKind as K;
use GamepadBinding::{Axis, Button};

const fn a(
    id: ActionId,
    name: &'static str,
    label: &'static str,
    category: ActionCategory,
    kind: ActionKind,
    keys: &'static [&'static str],
    pad: &'static [GamepadBinding],
    help: &'static str,
) -> ActionDef {
    ActionDef {
        id,
        name,
        label,
        category,
        kind,
        keys,
        pad,
        help,
    }
}

/// All actions (same order, labels, bindings and help as `player/actions.ts`).
pub static ACTIONS: [ActionDef; ACTION_COUNT] = [
    a(
        I::Throttle,
        "throttle",
        "Throttle",
        C::Driving,
        K::Analog,
        &["KeyW", "ArrowUp"],
        &[Button(7)],
        "Accelerate (in reverse with automatic: brake).",
    ),
    a(
        I::Brake,
        "brake",
        "Brake / reverse",
        C::Driving,
        K::Analog,
        &["KeyS", "ArrowDown"],
        &[Button(6)],
        "Brake. With automatic gearbox, hold at a standstill to reverse.",
    ),
    a(
        I::SteerLeft,
        "steer_left",
        "Steer left",
        C::Driving,
        K::Analog,
        &["KeyA", "ArrowLeft"],
        &[Axis(0, -1)],
        "Steer left.",
    ),
    a(
        I::SteerRight,
        "steer_right",
        "Steer right",
        C::Driving,
        K::Analog,
        &["KeyD", "ArrowRight"],
        &[Axis(0, 1)],
        "Steer right.",
    ),
    a(
        I::Handbrake,
        "handbrake",
        "Handbrake",
        C::Driving,
        K::Hold,
        &["Space"],
        &[Button(0)],
        "Locks the rear wheels: parking, hairpins and drifts.",
    ),
    a(
        I::Clutch,
        "clutch",
        "Clutch",
        C::Gearbox,
        K::Analog,
        &["KeyZ"],
        &[Button(4)],
        "Manual gearbox: press to shift or launch without stalling.",
    ),
    a(
        I::ShiftUp,
        "shift_up",
        "Shift up",
        C::Gearbox,
        K::Press,
        &["ShiftLeft", "KeyR"],
        &[Button(1)],
        "Next gear (manual).",
    ),
    a(
        I::ShiftDown,
        "shift_down",
        "Shift down",
        C::Gearbox,
        K::Press,
        &["ControlLeft", "KeyF"],
        &[Button(2)],
        "Previous gear (manual).",
    ),
    a(
        I::GearReverse,
        "gear_reverse",
        "Select reverse",
        C::Gearbox,
        K::Press,
        &["Backspace"],
        &[],
        "Select R directly.",
    ),
    a(
        I::GearNeutral,
        "gear_neutral",
        "Select neutral",
        C::Gearbox,
        K::Press,
        &["KeyN"],
        &[],
        "Select N directly.",
    ),
    a(
        I::TransmissionMode,
        "transmission_mode",
        "Auto / manual",
        C::Gearbox,
        K::Press,
        &["KeyT"],
        &[],
        "Toggle automatic and manual transmission.",
    ),
    a(
        I::Horn,
        "horn",
        "Horn",
        C::SignalsLights,
        K::Hold,
        &["KeyH"],
        &[Button(10)],
        "Tap for a beep, hold for a long blast (other drivers notice).",
    ),
    a(
        I::IndicatorLeft,
        "indicator_left",
        "Left indicator",
        C::SignalsLights,
        K::Press,
        &["KeyQ"],
        &[Button(14)],
        "Toggle left turn signal (auto-cancels after the turn).",
    ),
    a(
        I::IndicatorRight,
        "indicator_right",
        "Right indicator",
        C::SignalsLights,
        K::Press,
        &["KeyE"],
        &[Button(15)],
        "Toggle right turn signal (auto-cancels after the turn).",
    ),
    a(
        I::Hazards,
        "hazards",
        "Hazard lights",
        C::SignalsLights,
        K::Press,
        &["KeyX"],
        &[Button(13)],
        "All four indicators. A quick blink after merging says thanks.",
    ),
    a(
        I::Headlights,
        "headlights",
        "Headlights",
        C::SignalsLights,
        K::Press,
        &["KeyL"],
        &[Button(12)],
        "Cycle the light switch: OFF, AUTO, ON.",
    ),
    a(
        I::HighBeam,
        "high_beam",
        "High beam",
        C::SignalsLights,
        K::Press,
        &["KeyK"],
        &[],
        "Toggle high beams (dazzles oncoming drivers).",
    ),
    a(
        I::Flash,
        "flash",
        "Flash high beams",
        C::SignalsLights,
        K::Hold,
        &["KeyJ"],
        &[Button(5)],
        "Flash to pass / warn. Drivers ahead take it personally.",
    ),
    a(
        I::Wipers,
        "wipers",
        "Wipers",
        C::SignalsLights,
        K::Press,
        &["KeyP"],
        &[],
        "Cycle wipers: OFF, INT, LOW, HIGH.",
    ),
    a(
        I::Siren,
        "siren",
        "Siren",
        C::SignalsLights,
        K::Press,
        &["KeyO"],
        &[],
        "Emergency vehicles only: cycle siren.",
    ),
    a(
        I::CruiseToggle,
        "cruise_toggle",
        "Cruise control",
        C::ComfortAssists,
        K::Press,
        &["KeyC"],
        &[],
        "Set / cancel cruise at the current speed. Brake also cancels.",
    ),
    a(
        I::CruiseUp,
        "cruise_up",
        "Cruise +5",
        C::ComfortAssists,
        K::Press,
        &["Equal", "NumpadAdd"],
        &[],
        "Raise cruise speed by 5 km/h.",
    ),
    a(
        I::CruiseDown,
        "cruise_down",
        "Cruise -5",
        C::ComfortAssists,
        K::Press,
        &["Minus", "NumpadSubtract"],
        &[],
        "Lower cruise speed by 5 km/h.",
    ),
    a(
        I::Limiter,
        "limiter",
        "Speed limiter",
        C::ComfortAssists,
        K::Press,
        &["KeyU"],
        &[],
        "Cap speed at the posted limit.",
    ),
    a(
        I::Engine,
        "engine",
        "Engine start / stop",
        C::ComfortAssists,
        K::Press,
        &["KeyI"],
        &[],
        "Turn the ignition. Needed after a stall.",
    ),
    a(
        I::Seatbelt,
        "seatbelt",
        "Seatbelt",
        C::ComfortAssists,
        K::Press,
        &["KeyB"],
        &[],
        "Buckle up. Police fine unbelted drivers.",
    ),
    a(
        I::ToggleAbs,
        "toggle_abs",
        "ABS on/off",
        C::ComfortAssists,
        K::Press,
        &["Digit1"],
        &[],
        "Anti-lock brakes.",
    ),
    a(
        I::ToggleTcs,
        "toggle_tcs",
        "Traction control on/off",
        C::ComfortAssists,
        K::Press,
        &["Digit2"],
        &[],
        "Traction control (off = burnouts).",
    ),
    a(
        I::ToggleEsc,
        "toggle_esc",
        "Stability control on/off",
        C::ComfortAssists,
        K::Press,
        &["Digit3"],
        &[],
        "Electronic stability control.",
    ),
    a(
        I::Apologize,
        "apologize",
        "Wave / sorry",
        C::Social,
        K::Press,
        &["KeyG"],
        &[Button(3)],
        "Apologise to nearby drivers. Calms people you just upset.",
    ),
    a(
        I::RudeGesture,
        "rude_gesture",
        "Rude gesture",
        C::Social,
        K::Press,
        &["KeyY"],
        &[],
        "Insult nearby drivers. Expect consequences.",
    ),
    a(
        I::Camera,
        "camera",
        "Change camera",
        C::Camera,
        K::Press,
        &["KeyV"],
        &[Button(8)],
        "Chase, far chase, hood, cockpit, top-down, cinematic.",
    ),
    a(
        I::LookBack,
        "look_back",
        "Look back",
        C::Camera,
        K::Hold,
        &["KeyM"],
        &[Button(11)],
        "Hold to look behind.",
    ),
    a(
        I::LookLeft,
        "look_left",
        "Look left",
        C::Camera,
        K::Hold,
        &["Comma"],
        &[],
        "Hold to look left.",
    ),
    a(
        I::LookRight,
        "look_right",
        "Look right",
        C::Camera,
        K::Hold,
        &["Period"],
        &[],
        "Hold to look right.",
    ),
    a(
        I::ResetVehicle,
        "reset_vehicle",
        "Reset vehicle",
        C::System,
        K::Press,
        &["Backquote"],
        &[],
        "Put the car back on the nearest road (repair costs money).",
    ),
    a(
        I::Pause,
        "pause",
        "Pause / menu",
        C::System,
        K::Press,
        &["Escape"],
        &[Button(9)],
        "Pause and open the menu.",
    ),
    a(
        I::Help,
        "help",
        "Controls help",
        C::System,
        K::Press,
        &["F1"],
        &[],
        "Show all controls.",
    ),
    a(
        I::Map,
        "map",
        "Map",
        C::System,
        K::Press,
        &["Tab"],
        &[],
        "Toggle the big map.",
    ),
    a(
        I::Debug,
        "debug",
        "Debug overlay",
        C::System,
        K::Press,
        &["F3"],
        &[],
        "Show AI moods, intentions and signal phases.",
    ),
    a(
        I::TimeForward,
        "time_forward",
        "Skip time",
        C::System,
        K::Press,
        &["BracketRight"],
        &[],
        "Advance the clock by one hour.",
    ),
    a(
        I::WeatherCycle,
        "weather_cycle",
        "Change weather",
        C::System,
        K::Press,
        &["BracketLeft"],
        &[],
        "Cycle clear, rain, storm, fog.",
    ),
];

/// Definition of an action.
pub fn action_def(id: ActionId) -> &'static ActionDef {
    ACTIONS.iter().find(|d| d.id == id).unwrap_or(&ACTIONS[0])
}

/// Snapshot of driver controls for one simulation step.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ControlFrame {
    pub throttle: f64,
    pub brake: f64,
    /// -1 left .. +1 right
    pub steer: f64,
    pub handbrake: f64,
    pub clutch: f64,
    /// Actions pressed since the previous frame (repeats kept).
    pub pressed: Vec<ActionId>,
    /// Actions released since the previous frame, with how long they were held.
    pub released: Vec<(ActionId, f64)>,
    /// Actions currently held.
    pub held: Vec<ActionId>,
}

/// Current bindings of one action.
#[derive(Clone, Debug, PartialEq)]
pub struct Binding {
    pub keys: Vec<String>,
    pub pad: Vec<GamepadBinding>,
}

/// Collects raw keyboard/gamepad/direct input and turns it into
/// [`ControlFrame`]s with keyboard smoothing (pedal ramps, speed-sensitive
/// steering). Taps between frames are never lost.
#[derive(Clone, Debug)]
pub struct InputMapper {
    raw: [f64; ACTION_COUNT],
    prev: [f64; ACTION_COUNT],
    held_time: [f64; ACTION_COUNT],
    keys_down: HashMap<String, bool>,
    pad_values: [f64; ACTION_COUNT],
    direct: [f64; ACTION_COUNT],
    pending_press: Vec<ActionId>,
    key_index: HashMap<String, Vec<ActionId>>,
    s_throttle: f64,
    s_brake: f64,
    s_steer: f64,
    s_clutch: f64,
    /// True if the last analog input came from a gamepad (no smoothing).
    pub analog_from_pad: bool,
    bindings: Vec<Binding>,
}

impl Default for InputMapper {
    fn default() -> Self {
        Self::new()
    }
}

fn ramp(cur: f64, target: f64, max_delta: f64) -> f64 {
    if cur < target {
        target.min(cur + max_delta)
    } else {
        target.max(cur - max_delta)
    }
}

impl InputMapper {
    pub fn new() -> Self {
        let bindings = ACTIONS
            .iter()
            .map(|d| Binding {
                keys: d.keys.iter().map(|k| k.to_string()).collect(),
                pad: d.pad.to_vec(),
            })
            .collect();
        let mut m = InputMapper {
            raw: [0.0; ACTION_COUNT],
            prev: [0.0; ACTION_COUNT],
            held_time: [0.0; ACTION_COUNT],
            keys_down: HashMap::new(),
            pad_values: [0.0; ACTION_COUNT],
            direct: [0.0; ACTION_COUNT],
            pending_press: Vec::new(),
            key_index: HashMap::new(),
            s_throttle: 0.0,
            s_brake: 0.0,
            s_steer: 0.0,
            s_clutch: 0.0,
            analog_from_pad: false,
            bindings,
        };
        m.rebuild_index();
        m
    }

    /// Current bindings of an action.
    pub fn bindings(&self, id: ActionId) -> &Binding {
        &self.bindings[id as usize]
    }

    fn rebuild_index(&mut self) {
        self.key_index.clear();
        for (i, b) in self.bindings.iter().enumerate() {
            for k in &b.keys {
                self.key_index
                    .entry(k.clone())
                    .or_default()
                    .push(ActionId::ALL[i]);
            }
        }
    }

    /// Rebind an action's keyboard keys (a key can only drive one action).
    pub fn bind_keys(&mut self, id: ActionId, keys: &[&str]) {
        for b in &mut self.bindings {
            b.keys.retain(|k| !keys.contains(&k.as_str()));
        }
        self.bindings[id as usize].keys = keys.iter().map(|k| k.to_string()).collect();
        self.rebuild_index();
    }

    pub fn bind_pad(&mut self, id: ActionId, pad: &[GamepadBinding]) {
        self.bindings[id as usize].pad = pad.to_vec();
    }

    /// Feed a key event. Returns true if the key is bound.
    pub fn key(&mut self, code: &str, down: bool) -> bool {
        let was = self.keys_down.get(code).copied().unwrap_or(false);
        self.keys_down.insert(code.to_string(), down);
        let Some(ids) = self.key_index.get(code) else {
            return false;
        };
        if down && !was {
            self.pending_press.extend(ids.iter().copied());
        }
        if down {
            self.analog_from_pad = false;
        }
        true
    }

    /// Feed a gamepad state (standard mapping).
    pub fn gamepad(&mut self, buttons: &[f64], axes: &[f64]) {
        let mut any = false;
        for (i, b) in self.bindings.iter().enumerate() {
            let mut v: f64 = 0.0;
            for p in &b.pad {
                match *p {
                    GamepadBinding::Button(bi) if bi < buttons.len() => v = v.max(buttons[bi]),
                    GamepadBinding::Axis(ai, dir) if ai < axes.len() => {
                        let a = axes[ai] * dir as f64;
                        let dz = 0.12;
                        if a > dz {
                            v = v.max((a - dz) / (1.0 - dz));
                        }
                    }
                    _ => {}
                }
            }
            let before = self.pad_values[i];
            if v > 0.5 && before <= 0.5 {
                self.pending_press.push(ActionId::ALL[i]);
            }
            if v > 0.05 {
                any = true;
            }
            self.pad_values[i] = v;
        }
        if any {
            self.analog_from_pad = true;
        }
    }

    /// Set an action directly (touch UI, mod hotkeys, tests).
    pub fn set(&mut self, id: ActionId, value: f64) {
        let before = self.direct[id as usize];
        if value > 0.5 && before <= 0.5 {
            self.pending_press.push(id);
        }
        self.direct[id as usize] = value;
    }

    /// Fire a one-shot press.
    pub fn press(&mut self, id: ActionId) {
        self.pending_press.push(id);
    }

    pub fn release_all(&mut self) {
        self.keys_down.clear();
        self.direct = [0.0; ACTION_COUNT];
        self.pad_values = [0.0; ACTION_COUNT];
    }

    fn value(&self, i: usize) -> f64 {
        let mut v = self.direct[i].max(self.pad_values[i]);
        if self.bindings[i]
            .keys
            .iter()
            .any(|k| self.keys_down.get(k).copied().unwrap_or(false))
        {
            v = 1.0;
        }
        v
    }

    /// Build the control frame for this step (`speed` for steering sensitivity).
    pub fn frame(&mut self, dt: f64, speed: f64) -> ControlFrame {
        let pressed = std::mem::take(&mut self.pending_press);
        let mut released = Vec::new();
        let mut held = Vec::new();
        for i in 0..ACTION_COUNT {
            let v = self.value(i);
            let before = self.prev[i];
            self.raw[i] = v;
            if v > 0.5 {
                held.push(ActionId::ALL[i]);
                self.held_time[i] += dt;
            } else if before > 0.5 {
                released.push((ActionId::ALL[i], self.held_time[i]));
                self.held_time[i] = 0.0;
            }
            self.prev[i] = v;
        }
        let r = |id: ActionId| self.raw[id as usize];
        let thr = r(ActionId::Throttle);
        let brk = r(ActionId::Brake);
        let steer = r(ActionId::SteerRight) - r(ActionId::SteerLeft);
        let clutch = r(ActionId::Clutch);
        let handbrake = r(ActionId::Handbrake);
        if self.analog_from_pad {
            self.s_throttle = thr;
            self.s_brake = brk;
            self.s_steer += (steer - self.s_steer) * (dt * 20.0).min(1.0);
            self.s_clutch = clutch;
        } else {
            self.s_throttle = ramp(
                self.s_throttle,
                thr,
                dt * if thr > self.s_throttle { 4.0 } else { 8.0 },
            );
            self.s_brake = ramp(
                self.s_brake,
                brk,
                dt * if brk > self.s_brake { 5.0 } else { 10.0 },
            );
            self.s_clutch = ramp(self.s_clutch, clutch, dt * 6.0);
            let speed_factor = 1.0 / (1.0 + speed.abs() / 18.0);
            let target = steer * (0.35 + 0.65 * speed_factor);
            let returning = target.abs() < self.s_steer.abs() || target * self.s_steer < 0.0;
            self.s_steer = ramp(
                self.s_steer,
                target,
                dt * if returning {
                    5.0
                } else {
                    2.2 + 1.8 * speed_factor
                },
            );
        }
        ControlFrame {
            throttle: self.s_throttle,
            brake: self.s_brake,
            steer: self.s_steer,
            handbrake,
            clutch: self.s_clutch,
            pressed,
            released,
            held,
        }
    }

    pub fn is_held(&self, id: ActionId) -> bool {
        self.raw[id as usize] > 0.5
    }

    pub fn held_for(&self, id: ActionId) -> f64 {
        self.held_time[id as usize]
    }
}
