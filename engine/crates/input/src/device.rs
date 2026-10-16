//! Real gamepads via gilrs, plus rumble (gilrs force feedback; Windows.Gaming.Input on Windows,
//! which also drives the Xbox impulse-trigger motors).

use crate::pad::{GamepadState, PadAxis, PadButton, PadKind};
use crate::rumble::Rumble;
use gilrs::ff::{BaseEffect, BaseEffectType, EffectBuilder, Replay, Ticks};
use gilrs::{Axis, Button, EventType, GamepadId, Gilrs};

pub struct GamepadManager {
    gilrs: Option<Gilrs>,
    active: Option<GamepadId>,
    effects: Option<(gilrs::ff::Effect, gilrs::ff::Effect)>,
    effects_for: Option<GamepadId>,
    pub state: GamepadState,
    /// Human-readable status for menus ("Xbox Wireless Controller", "no controller").
    pub status: String,
    #[cfg(windows)]
    wgi: wgi::Vibration,
}

impl Default for GamepadManager {
    fn default() -> Self {
        Self::new()
    }
}

impl GamepadManager {
    pub fn new() -> Self {
        let gilrs = Gilrs::new().ok();
        let status = if gilrs.is_some() {
            "no controller".to_string()
        } else {
            "gamepad backend unavailable".to_string()
        };
        let mut m = Self {
            gilrs,
            active: None,
            effects: None,
            effects_for: None,
            state: GamepadState::default(),
            status,
            #[cfg(windows)]
            wgi: wgi::Vibration::default(),
        };
        if let Some(g) = &m.gilrs {
            if let Some((id, _)) = g.gamepads().next() {
                m.active = Some(id);
            }
        }
        m
    }

    /// Drain events, pick the most recently used pad and refresh `state`.
    pub fn poll(&mut self) -> &GamepadState {
        let Some(g) = self.gilrs.as_mut() else {
            return &self.state;
        };
        while let Some(ev) = g.next_event() {
            match ev.event {
                EventType::Disconnected => {
                    if self.active == Some(ev.id) {
                        self.active = g.gamepads().map(|(id, _)| id).find(|id| *id != ev.id);
                        self.effects = None;
                    }
                }
                EventType::ButtonPressed(..) | EventType::Connected => self.active = Some(ev.id),
                EventType::AxisChanged(_, v, _) if v.abs() > 0.4 => self.active = Some(ev.id),
                _ => {}
            }
        }
        let mut s = GamepadState::default();
        if let Some(id) = self.active {
            if let Some(pad) = g.connected_gamepad(id) {
                s.connected = true;
                s.name = pad.name().to_string();
                s.kind = PadKind::from_name(&s.name);
                let map = [
                    (Button::South, PadButton::South),
                    (Button::East, PadButton::East),
                    (Button::West, PadButton::West),
                    (Button::North, PadButton::North),
                    (Button::LeftTrigger, PadButton::LeftBumper),
                    (Button::RightTrigger, PadButton::RightBumper),
                    (Button::Select, PadButton::View),
                    (Button::Start, PadButton::Menu),
                    (Button::LeftThumb, PadButton::LeftStick),
                    (Button::RightThumb, PadButton::RightStick),
                    (Button::DPadUp, PadButton::DpadUp),
                    (Button::DPadDown, PadButton::DpadDown),
                    (Button::DPadLeft, PadButton::DpadLeft),
                    (Button::DPadRight, PadButton::DpadRight),
                    (Button::Mode, PadButton::Guide),
                ];
                for (gb, pb) in map {
                    s.set_button(pb, pad.is_pressed(gb));
                }
                s.set_axis(PadAxis::LeftX, pad.value(Axis::LeftStickX));
                s.set_axis(PadAxis::LeftY, pad.value(Axis::LeftStickY));
                s.set_axis(PadAxis::RightX, pad.value(Axis::RightStickX));
                s.set_axis(PadAxis::RightY, pad.value(Axis::RightStickY));
                // Analog triggers (gilrs maps them to LeftTrigger2/RightTrigger2 button values).
                let lt = pad
                    .button_data(Button::LeftTrigger2)
                    .map(|d| d.value())
                    .unwrap_or(0.0);
                let rt = pad
                    .button_data(Button::RightTrigger2)
                    .map(|d| d.value())
                    .unwrap_or(0.0);
                s.set_axis(
                    PadAxis::LeftTrigger,
                    lt.max(pad.value(Axis::LeftZ).max(0.0)),
                );
                s.set_axis(
                    PadAxis::RightTrigger,
                    rt.max(pad.value(Axis::RightZ).max(0.0)),
                );
                self.status = s.name.clone();
            }
        } else {
            self.status = "no controller".into();
        }
        self.state = s;
        &self.state
    }

    /// Set motor levels (call every frame; zero to stop).
    pub fn set_rumble(&mut self, r: Rumble) {
        #[cfg(windows)]
        {
            if self.wgi.set(r) {
                return;
            }
        }
        let (Some(g), Some(id)) = (self.gilrs.as_mut(), self.active) else {
            return;
        };
        if self.effects_for != Some(id) || self.effects.is_none() {
            self.effects = None;
            let ff = g.connected_gamepad(id).is_some_and(|p| p.is_ff_supported());
            if ff {
                let make = |g: &mut Gilrs, kind: BaseEffectType| {
                    EffectBuilder::new()
                        .add_effect(BaseEffect {
                            kind,
                            scheduling: Replay {
                                play_for: Ticks::from_ms(60_000),
                                ..Default::default()
                            },
                            envelope: Default::default(),
                        })
                        .repeat(gilrs::ff::Repeat::Infinitely)
                        .gamepads(&[id])
                        .finish(g)
                        .ok()
                };
                let strong = make(
                    g,
                    BaseEffectType::Strong {
                        magnitude: u16::MAX,
                    },
                );
                let weak = make(
                    g,
                    BaseEffectType::Weak {
                        magnitude: u16::MAX,
                    },
                );
                if let (Some(s), Some(w)) = (strong, weak) {
                    let _ = s.set_gain(0.0);
                    let _ = w.set_gain(0.0);
                    let _ = s.play();
                    let _ = w.play();
                    self.effects = Some((s, w));
                }
            }
            self.effects_for = Some(id);
        }
        if let Some((s, w)) = &self.effects {
            let _ = s.set_gain(r.low.clamp(0.0, 1.0));
            let _ = w.set_gain(r.high.clamp(0.0, 1.0));
        }
    }
}

impl Drop for GamepadManager {
    fn drop(&mut self) {
        self.set_rumble(Rumble::default());
    }
}

#[cfg(windows)]
mod wgi {
    //! Windows.Gaming.Input vibration: both motors plus the impulse triggers on Xbox pads.
    use crate::rumble::Rumble;
    use windows::Gaming::Input::{Gamepad, GamepadVibration};

    #[derive(Default)]
    pub struct Vibration {
        failed: bool,
    }

    impl Vibration {
        /// Returns true if a WGI gamepad took the command.
        pub fn set(&mut self, r: Rumble) -> bool {
            if self.failed {
                return false;
            }
            let Ok(pads) = Gamepad::Gamepads() else {
                self.failed = true;
                return false;
            };
            let Ok(n) = pads.Size() else { return false };
            if n == 0 {
                return false;
            }
            let v = GamepadVibration {
                LeftMotor: r.low.clamp(0.0, 1.0) as f64,
                RightMotor: r.high.clamp(0.0, 1.0) as f64,
                LeftTrigger: r.left_trigger.clamp(0.0, 1.0) as f64,
                RightTrigger: r.right_trigger.clamp(0.0, 1.0) as f64,
            };
            let mut ok = false;
            for i in 0..n {
                if let Ok(p) = pads.GetAt(i) {
                    ok |= p.SetVibration(v).is_ok();
                }
            }
            ok
        }
    }
}
