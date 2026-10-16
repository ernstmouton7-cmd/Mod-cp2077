//! # gridlock-input
//!
//! Gamepad-first driving controls for the Gridlock engine:
//! - every driver action reachable from an Xbox-style pad (tap / hold / chord gestures and a
//!   modifier layer on View), with keyboard equivalents;
//! - steering and pedal shaping (deadzones, response curves, smoothing, speed-sensitive keyboard
//!   steering);
//! - a rumble mixer that turns vehicle telemetry into motor and trigger vibration;
//! - an optional real-device backend (`gilrs` feature) with Windows trigger rumble.
//!
//! The mapping logic is device-agnostic so other hosts (e.g. Unreal) can feed raw states.

pub mod actions;
pub mod mapper;
pub mod pad;
pub mod rumble;

#[cfg(feature = "gilrs")]
pub mod device;

pub use actions::{
    Action, ActionDef, ActionKind, Category, Gesture, Layer, PadBinding, PadInput, ACTIONS,
};
pub use mapper::{
    ActionEvent, Bindings, ControlFrame, Device, InputConfig, InputMapper, KeyboardState,
    SteeringConfig, TriggerConfig,
};
pub use pad::{axis_label, button_label, GamepadState, PadAxis, PadButton, PadKind};
pub use rumble::{Rumble, RumbleInputs, RumbleMixer};
