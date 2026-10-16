//! Player input (actions, bindings, input mapper) and the player's driver.

pub mod actions;
pub mod player_driver;

pub use actions::{action_def, ActionCategory, ActionDef, ActionId, ActionKind, Binding, ControlFrame, GamepadBinding, InputMapper, ACTIONS, ACTION_COUNT};
pub use player_driver::{CruiseState, PlayerDriver, PlayerSettings, HOST_ACTIONS};
