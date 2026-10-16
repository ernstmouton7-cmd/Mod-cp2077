//! Turns raw keyboard and gamepad state into driving controls and action events.

use crate::actions::*;
use crate::pad::{GamepadState, PadAxis, PadButton};
use glam::Vec2;
use std::collections::{HashMap, HashSet};

/// Stick-to-steering shaping.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SteeringConfig {
    /// Inner deadzone (fraction of travel).
    pub deadzone: f32,
    /// Travel at which full lock is reached.
    pub outer: f32,
    /// Response exponent: 1 = linear, 2 = gentle centre / quick edges.
    pub exponent: f32,
    /// Overall sensitivity multiplier (output clamped to ±1).
    pub sensitivity: f32,
    /// Light smoothing of stick noise (seconds); 0 = raw.
    pub smoothing: f32,
    /// Keyboard: full-lock time at a standstill and at speed (s), and centring time.
    pub key_attack_slow: f32,
    pub key_attack_fast: f32,
    pub key_release: f32,
}

impl Default for SteeringConfig {
    fn default() -> Self {
        Self {
            deadzone: 0.08,
            outer: 0.96,
            exponent: 1.55,
            sensitivity: 1.0,
            smoothing: 0.035,
            key_attack_slow: 0.28,
            key_attack_fast: 0.65,
            key_release: 0.18,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TriggerConfig {
    pub deadzone: f32,
    pub outer: f32,
    pub exponent: f32,
}

impl TriggerConfig {
    pub const THROTTLE: TriggerConfig = TriggerConfig {
        deadzone: 0.04,
        outer: 0.97,
        exponent: 1.25,
    };
    pub const BRAKE: TriggerConfig = TriggerConfig {
        deadzone: 0.05,
        outer: 0.95,
        exponent: 1.6,
    };
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InputConfig {
    pub steering: SteeringConfig,
    pub throttle: TriggerConfig,
    pub brake: TriggerConfig,
    pub look_deadzone: f32,
    pub look_exponent: f32,
    pub invert_look_y: bool,
    /// Manual gearbox without clutch assist: LB acts as the clutch pedal.
    pub clutch_on_bumper: bool,
    pub tap_time: f32,
    pub hold_time: f32,
    /// Keyboard throttle/brake ramp time (s).
    pub key_pedal_time: f32,
}

impl Default for InputConfig {
    fn default() -> Self {
        Self {
            steering: SteeringConfig::default(),
            throttle: TriggerConfig::THROTTLE,
            brake: TriggerConfig::BRAKE,
            look_deadzone: 0.12,
            look_exponent: 1.6,
            invert_look_y: false,
            clutch_on_bumper: false,
            tap_time: 0.28,
            hold_time: 0.42,
            key_pedal_time: 0.22,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Device {
    #[default]
    Keyboard,
    Gamepad,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ActionEvent {
    Pressed(Action),
    /// Released after being held this long (s).
    Released(Action, f32),
}

/// Controls for one simulation frame.
#[derive(Clone, Debug, Default)]
pub struct ControlFrame {
    pub throttle: f32,
    pub brake: f32,
    /// -1 full left .. +1 full right.
    pub steer: f32,
    pub handbrake: f32,
    pub clutch: f32,
    /// Right stick / look keys (x right, y up), shaped.
    pub look: Vec2,
    pub events: Vec<ActionEvent>,
    pub held: Vec<Action>,
    pub device: Device,
    pub modifier: bool,
}

impl ControlFrame {
    pub fn pressed(&self, a: Action) -> bool {
        self.events.iter().any(|e| *e == ActionEvent::Pressed(a))
    }
    pub fn released(&self, a: Action) -> Option<f32> {
        self.events.iter().find_map(|e| match e {
            ActionEvent::Released(x, t) if *x == a => Some(*t),
            _ => None,
        })
    }
    pub fn is_held(&self, a: Action) -> bool {
        self.held.contains(&a)
    }
}

/// Keyboard state as a set of held key codes (`KeyboardEvent.code` names).
#[derive(Clone, Debug, Default)]
pub struct KeyboardState {
    pub down: HashSet<String>,
    /// Keys pressed since last frame (so very short taps are not lost).
    pub pressed: Vec<String>,
}

impl KeyboardState {
    pub fn key_down(&mut self, code: &str) {
        if self.down.insert(code.to_string()) {
            self.pressed.push(code.to_string());
        }
    }
    pub fn key_up(&mut self, code: &str) {
        self.down.remove(code);
    }
    pub fn is_down(&self, code: &str) -> bool {
        self.down.contains(code)
    }
}

/// Mutable copy of the default bindings.
#[derive(Clone, Debug)]
pub struct Bindings {
    pub keys: HashMap<Action, Vec<String>>,
    pub pad: HashMap<Action, Vec<PadBinding>>,
}

impl Default for Bindings {
    fn default() -> Self {
        let mut keys = HashMap::new();
        let mut pad = HashMap::new();
        for d in ACTIONS {
            keys.insert(d.action, d.keys.iter().map(|s| s.to_string()).collect());
            pad.insert(d.action, d.pad.to_vec());
        }
        Self { keys, pad }
    }
}

impl Bindings {
    /// Bind keys to an action, removing them from any other action.
    pub fn bind_keys(&mut self, a: Action, keys: &[&str]) {
        for list in self.keys.values_mut() {
            list.retain(|k| !keys.contains(&k.as_str()));
        }
        self.keys
            .insert(a, keys.iter().map(|s| s.to_string()).collect());
    }
    /// Bind a pad input, removing the same input+layer+gesture from other actions.
    pub fn bind_pad(&mut self, a: Action, b: PadBinding) {
        for list in self.pad.values_mut() {
            list.retain(|x| *x != b);
        }
        self.pad.entry(a).or_default().push(b);
    }
    pub fn keys_for(&self, a: Action) -> &[String] {
        self.keys.get(&a).map(|v| v.as_slice()).unwrap_or(&[])
    }
    pub fn pad_for(&self, a: Action) -> &[PadBinding] {
        self.pad.get(&a).map(|v| v.as_slice()).unwrap_or(&[])
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct GestureState {
    down: bool,
    time: f32,
    fired: bool,
    /// Layer latched at press time.
    modifier: bool,
}

/// Shapes a stick or trigger value: deadzone, rescale, exponent.
pub fn shape(v: f32, deadzone: f32, outer: f32, exponent: f32) -> f32 {
    let a = v.abs();
    if a <= deadzone {
        return 0.0;
    }
    let t = ((a - deadzone) / (outer - deadzone).max(1e-3)).min(1.0);
    t.powf(exponent) * v.signum()
}

/// Radial deadzone for a 2D stick (keeps diagonals smooth).
pub fn shape_stick(v: Vec2, deadzone: f32, exponent: f32) -> Vec2 {
    let l = v.length();
    if l <= deadzone {
        return Vec2::ZERO;
    }
    let t = ((l - deadzone) / (0.97 - deadzone)).min(1.0).powf(exponent);
    v / l * t
}

pub struct InputMapper {
    pub config: InputConfig,
    pub bindings: Bindings,
    gestures: HashMap<(Action, usize), GestureState>,
    key_held: HashMap<Action, f32>,
    consumed: HashSet<PadButton>,
    modifier_used: bool,
    prev_buttons: [bool; PadButton::COUNT],
    // Smoothed analogs.
    steer_pad: f32,
    steer_key: f32,
    throttle_key: f32,
    brake_key: f32,
    clutch_key: f32,
    pub device: Device,
}

impl Default for InputMapper {
    fn default() -> Self {
        Self::new(InputConfig::default())
    }
}

impl InputMapper {
    pub fn new(config: InputConfig) -> Self {
        Self {
            config,
            bindings: Bindings::default(),
            gestures: HashMap::new(),
            key_held: HashMap::new(),
            consumed: HashSet::new(),
            modifier_used: false,
            prev_buttons: [false; PadButton::COUNT],
            steer_pad: 0.0,
            steer_key: 0.0,
            throttle_key: 0.0,
            brake_key: 0.0,
            clutch_key: 0.0,
            device: Device::Keyboard,
        }
    }

    fn keys_down(&self, kb: &KeyboardState, a: Action) -> bool {
        self.bindings.keys_for(a).iter().any(|k| kb.is_down(k))
    }
    fn keys_pressed(&self, kb: &KeyboardState, a: Action) -> bool {
        self.bindings
            .keys_for(a)
            .iter()
            .any(|k| kb.pressed.iter().any(|p| p == k))
    }

    /// Process one frame. `speed` (m/s) makes keyboard steering slower at speed.
    /// Clears `kb.pressed`.
    pub fn update(
        &mut self,
        dt: f32,
        pad: &GamepadState,
        kb: &mut KeyboardState,
        speed: f32,
    ) -> ControlFrame {
        let cfg = self.config;
        let mut f = ControlFrame::default();
        if pad.connected && pad.active() {
            self.device = Device::Gamepad;
        }
        if !kb.down.is_empty() || !kb.pressed.is_empty() {
            self.device = Device::Keyboard;
        }
        f.device = self.device;

        // ---------------------------------------------------------------- gamepad gestures
        let modifier = pad.connected && pad.button(MODIFIER);
        f.modifier = modifier;
        // Any other button while View is down makes it a modifier, not a "map" tap.
        if modifier
            && PadButton::ALL
                .iter()
                .any(|b| *b != MODIFIER && pad.button(*b) && !self.prev_buttons[b.index()])
        {
            self.modifier_used = true;
        }
        // Chords consume their member buttons so their own taps don't fire.
        for d in ACTIONS {
            for b in self.bindings.pad_for(d.action) {
                if let PadInput::Chord(x, y) = b.input {
                    if pad.button(x) && pad.button(y) {
                        self.consumed.insert(x);
                        self.consumed.insert(y);
                    }
                }
            }
        }
        if self.config.clutch_on_bumper {
            self.consumed.insert(PadButton::LeftBumper);
        }
        let mut held: Vec<Action> = Vec::new();
        let actions: Vec<Action> = ACTIONS.iter().map(|d| d.action).collect();
        for a in actions {
            let binds = self.bindings.pad_for(a).to_vec();
            for (bi, b) in binds.iter().enumerate() {
                let raw = pad.connected
                    && match b.input {
                        PadInput::Button(x) => pad.button(x),
                        PadInput::AxisPos(ax) => pad.axis(ax) > 0.5,
                        PadInput::AxisNeg(ax) => pad.axis(ax) < -0.5,
                        PadInput::Chord(x, y) => pad.button(x) && pad.button(y),
                    };
                let st = self.gestures.entry((a, bi)).or_default();
                if raw && !st.down {
                    st.down = true;
                    st.time = 0.0;
                    st.fired = false;
                    // The modifier button's own bindings live on the base layer.
                    st.modifier = modifier && b.input != PadInput::Button(MODIFIER);
                }
                let layer_ok = (b.layer == Layer::Modifier) == st.modifier;
                let is_consumed = match b.input {
                    PadInput::Button(x) => {
                        self.consumed.contains(&x) || (x == MODIFIER && self.modifier_used)
                    }
                    _ => false,
                };
                if st.down {
                    if raw {
                        st.time += dt;
                    }
                    match b.gesture {
                        Gesture::Press => {
                            if layer_ok && !st.fired {
                                st.fired = true;
                                f.events.push(ActionEvent::Pressed(a));
                            }
                            if layer_ok && st.fired && raw {
                                held.push(a);
                            }
                        }
                        Gesture::Hold => {
                            if layer_ok
                                && !st.fired
                                && raw
                                && st.time >= cfg.hold_time
                                && !is_consumed
                            {
                                st.fired = true;
                                f.events.push(ActionEvent::Pressed(a));
                            }
                            if st.fired && raw {
                                held.push(a);
                            }
                        }
                        Gesture::Tap => {}
                    }
                    if !raw {
                        st.down = false;
                        match b.gesture {
                            Gesture::Tap => {
                                if layer_ok && st.time < cfg.tap_time && !is_consumed {
                                    f.events.push(ActionEvent::Pressed(a));
                                    f.events.push(ActionEvent::Released(a, st.time));
                                }
                            }
                            _ => {
                                if st.fired && layer_ok {
                                    f.events.push(ActionEvent::Released(a, st.time));
                                }
                            }
                        }
                        st.fired = false;
                    }
                }
            }
        }
        // Clear consumption once the buttons are released.
        let still: Vec<PadButton> = self
            .consumed
            .iter()
            .copied()
            .filter(|b| pad.button(*b))
            .collect();
        self.consumed = still.into_iter().collect();
        if !pad.button(MODIFIER) {
            self.modifier_used = false;
        }
        self.prev_buttons = pad.buttons;

        // ---------------------------------------------------------------- keyboard
        for d in ACTIONS {
            let a = d.action;
            let down = self.keys_down(kb, a);
            let just = self.keys_pressed(kb, a);
            let t = self.key_held.get(&a).copied();
            match (t, down || just) {
                (None, true) => {
                    self.key_held.insert(a, 0.0);
                    f.events.push(ActionEvent::Pressed(a));
                    held.push(a);
                }
                (Some(t0), true) => {
                    self.key_held.insert(a, t0 + dt);
                    if down {
                        held.push(a);
                    }
                }
                (Some(t0), false) => {
                    self.key_held.remove(&a);
                    f.events.push(ActionEvent::Released(a, t0));
                }
                (None, false) => {}
            }
            if just && !down {
                // Tapped and released within one frame.
                self.key_held.remove(&a);
                f.events.push(ActionEvent::Released(a, 0.0));
            }
        }
        kb.pressed.clear();
        held.sort();
        held.dedup();

        // ---------------------------------------------------------------- analog
        let kdown = |a: Action| self.bindings.keys_for(a).iter().any(|k| kb.is_down(k));
        let ramp = |v: &mut f32, target: f32, up: f32, down: f32| {
            let rate = if target > *v {
                dt / up.max(1e-3)
            } else {
                dt / down.max(1e-3)
            };
            *v += (target - *v).clamp(-rate, rate);
        };
        let kt = if kdown(Action::Throttle) { 1.0 } else { 0.0 };
        let kbk = if kdown(Action::Brake) { 1.0 } else { 0.0 };
        let kc = if kdown(Action::Clutch) { 1.0 } else { 0.0 };
        let (pt, pb) = (cfg.key_pedal_time, cfg.key_pedal_time * 0.7);
        ramp(&mut self.throttle_key, kt, pt, 0.1);
        ramp(&mut self.brake_key, kbk, pb, 0.08);
        ramp(&mut self.clutch_key, kc, 0.12, 0.25);
        let ks: f32 = (if kdown(Action::SteerRight) { 1.0 } else { 0.0 })
            - (if kdown(Action::SteerLeft) { 1.0 } else { 0.0 });
        let sc = cfg.steering;
        let speed_t = (speed.abs() / 30.0).min(1.0);
        let attack = sc.key_attack_slow + (sc.key_attack_fast - sc.key_attack_slow) * speed_t;
        // Steering back through centre counts as a release (snappier counter-steer).
        let toward_centre = ks == 0.0 || ks.signum() != self.steer_key.signum();
        let rate = if toward_centre {
            dt / sc.key_release
        } else {
            dt / attack
        };
        self.steer_key += (ks - self.steer_key).clamp(-rate, rate);

        let (mut p_throttle, mut p_brake, mut p_steer, mut look) = (0.0, 0.0, 0.0, Vec2::ZERO);
        if pad.connected {
            p_throttle = shape(
                pad.axis(PadAxis::RightTrigger),
                cfg.throttle.deadzone,
                cfg.throttle.outer,
                cfg.throttle.exponent,
            )
            .max(0.0);
            p_brake = shape(
                pad.axis(PadAxis::LeftTrigger),
                cfg.brake.deadzone,
                cfg.brake.outer,
                cfg.brake.exponent,
            )
            .max(0.0);
            let raw = shape(pad.axis(PadAxis::LeftX), sc.deadzone, sc.outer, sc.exponent);
            let target = (raw * sc.sensitivity).clamp(-1.0, 1.0);
            let k = if sc.smoothing > 0.0 {
                1.0 - (-dt / sc.smoothing).exp()
            } else {
                1.0
            };
            self.steer_pad += (target - self.steer_pad) * k;
            p_steer = self.steer_pad;
            look = shape_stick(
                Vec2::new(pad.axis(PadAxis::RightX), pad.axis(PadAxis::RightY)),
                cfg.look_deadzone,
                cfg.look_exponent,
            );
            if cfg.invert_look_y {
                look.y = -look.y;
            }
        }
        f.throttle = p_throttle.max(self.throttle_key);
        f.brake = p_brake.max(self.brake_key);
        f.steer = if self.device == Device::Gamepad && p_steer != 0.0 || self.steer_key.abs() < 1e-3
        {
            p_steer
        } else {
            self.steer_key
        };
        f.handbrake = if held.contains(&Action::Handbrake) {
            1.0
        } else {
            0.0
        };
        let pad_clutch =
            if cfg.clutch_on_bumper && pad.connected && pad.button(PadButton::LeftBumper) {
                1.0
            } else {
                0.0
            };
        f.clutch = self.clutch_key.max(pad_clutch);
        let lk = (if kdown(Action::LookRight) { 1.0 } else { 0.0 })
            - (if kdown(Action::LookLeft) { 1.0 } else { 0.0 });
        if lk != 0.0 {
            look.x = lk;
        }
        f.look = look;
        f.held = held;
        f
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(m: &mut InputMapper, pad: &GamepadState, dt: f32) -> ControlFrame {
        let mut kb = KeyboardState::default();
        m.update(dt, pad, &mut kb, 0.0)
    }

    fn pad() -> GamepadState {
        GamepadState {
            connected: true,
            ..Default::default()
        }
    }

    #[test]
    fn trigger_and_stick_shaping() {
        assert_eq!(shape(0.03, 0.05, 0.95, 1.5), 0.0);
        assert!((shape(1.0, 0.05, 0.95, 1.5) - 1.0).abs() < 1e-6);
        assert!(shape(0.5, 0.05, 0.95, 1.5) < 0.5);
        assert!(shape(-0.5, 0.05, 0.95, 1.5) < 0.0);
        let mut m = InputMapper::default();
        let mut p = pad();
        p.set_axis(PadAxis::RightTrigger, 1.0);
        p.set_axis(PadAxis::LeftX, -1.0);
        let mut f = ControlFrame::default();
        for _ in 0..30 {
            f = step(&mut m, &p, 1.0 / 60.0);
        }
        assert!((f.throttle - 1.0).abs() < 1e-3);
        assert!(f.steer < -0.99, "{}", f.steer);
        assert_eq!(f.device, Device::Gamepad);
    }

    #[test]
    fn tap_hold_and_chord() {
        let mut m = InputMapper::default();
        let mut p = pad();
        // Tap Y -> camera; hold Y -> look back.
        p.set_button(PadButton::North, true);
        let mut evs = Vec::new();
        for _ in 0..5 {
            evs.extend(step(&mut m, &p, 1.0 / 60.0).events);
        }
        p.set_button(PadButton::North, false);
        evs.extend(step(&mut m, &p, 1.0 / 60.0).events);
        assert!(evs.contains(&ActionEvent::Pressed(Action::Camera)));
        assert!(!evs.contains(&ActionEvent::Pressed(Action::LookBack)));
        evs.clear();
        p.set_button(PadButton::North, true);
        let mut held_lookback = false;
        for _ in 0..40 {
            let f = step(&mut m, &p, 1.0 / 60.0);
            held_lookback |= f.is_held(Action::LookBack);
            evs.extend(f.events);
        }
        p.set_button(PadButton::North, false);
        evs.extend(step(&mut m, &p, 1.0 / 60.0).events);
        assert!(held_lookback);
        assert!(
            !evs.contains(&ActionEvent::Pressed(Action::Camera)),
            "long hold must not change camera"
        );
        // LB + RB -> hazards only.
        evs.clear();
        p.set_button(PadButton::LeftBumper, true);
        evs.extend(step(&mut m, &p, 1.0 / 60.0).events);
        p.set_button(PadButton::RightBumper, true);
        evs.extend(step(&mut m, &p, 1.0 / 60.0).events);
        p.set_button(PadButton::LeftBumper, false);
        p.set_button(PadButton::RightBumper, false);
        evs.extend(step(&mut m, &p, 1.0 / 60.0).events);
        assert!(evs.contains(&ActionEvent::Pressed(Action::Hazards)));
        assert!(!evs.contains(&ActionEvent::Pressed(Action::IndicatorLeft)));
        assert!(!evs.contains(&ActionEvent::Pressed(Action::IndicatorRight)));
    }

    #[test]
    fn modifier_layer() {
        let mut m = InputMapper::default();
        let mut p = pad();
        p.set_button(PadButton::View, true);
        step(&mut m, &p, 1.0 / 60.0);
        p.set_button(PadButton::East, true);
        let f = step(&mut m, &p, 1.0 / 60.0);
        assert!(f.pressed(Action::Seatbelt));
        assert!(!f.pressed(Action::ShiftUp));
        p.set_button(PadButton::East, false);
        p.set_button(PadButton::View, false);
        let f = step(&mut m, &p, 1.0 / 60.0);
        assert!(
            !f.pressed(Action::Map),
            "View used as modifier must not open the map"
        );
        // Plain View tap opens the map.
        p.set_button(PadButton::View, true);
        step(&mut m, &p, 1.0 / 60.0);
        p.set_button(PadButton::View, false);
        assert!(step(&mut m, &p, 1.0 / 60.0).pressed(Action::Map));
    }

    #[test]
    fn keyboard_ramps_and_quick_taps() {
        let mut m = InputMapper::default();
        let p = GamepadState::default();
        let mut kb = KeyboardState::default();
        kb.key_down("KeyW");
        kb.key_down("KeyD");
        let f = m.update(1.0 / 60.0, &p, &mut kb, 0.0);
        assert!(f.throttle > 0.0 && f.throttle < 0.2);
        assert!(f.steer > 0.0 && f.steer < 0.2);
        for _ in 0..60 {
            m.update(1.0 / 60.0, &p, &mut kb, 0.0);
        }
        let f = m.update(1.0 / 60.0, &p, &mut kb, 0.0);
        assert!((f.throttle - 1.0).abs() < 1e-3 && (f.steer - 1.0).abs() < 1e-3);
        // A key pressed and released between frames still registers.
        kb.key_down("KeyH");
        kb.key_up("KeyH");
        let f = m.update(1.0 / 60.0, &p, &mut kb, 0.0);
        assert!(f.pressed(Action::Horn));
        assert!(f.released(Action::Horn).is_some());
    }

    #[test]
    fn every_action_has_a_unique_id_and_pad_inputs_do_not_clash() {
        let mut ids = HashSet::new();
        for d in ACTIONS {
            assert!(ids.insert(d.id), "duplicate id {}", d.id);
        }
        let mut seen = HashMap::new();
        for d in ACTIONS {
            for b in d.pad {
                if let Some(prev) = seen.insert(*b, d.action) {
                    panic!("{:?} bound to both {:?} and {:?}", b, prev, d.action);
                }
            }
        }
    }
}
