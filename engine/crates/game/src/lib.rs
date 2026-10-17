//! # gridlock-game
//!
//! The playable layer of the Gridlock engine. It owns the traffic simulation, the 3D physics
//! world, static city geometry, street props, destruction, vehicle damage and input, steps them
//! together at a fixed rate, and exposes a renderer-agnostic [`scene::SceneFrame`] each frame.
//! The wgpu app and the C ABI (for Unreal Engine) are thin hosts around [`Game`].

pub mod carmesh;
pub mod city;
pub mod coords;
pub mod game;
pub mod hud;
pub mod scene;

pub use carmesh::{build_car, CarModel, CarPart, MeshData, Paint};
pub use city::{BuildingGeom, BuildingStyle, MarkKind, Marking, StaticCity};
pub use game::{CarLink, CarState, Game, GameConfig, GameEvent};
pub use hud::HudInfo;
pub use scene::{CarView, EnvironmentView, LampState, PedView, PropView, SceneFrame, SignalView};

pub use gridlock_input as input;
pub use gridlock_physics as physics;
pub use gridlock_traffic as traffic;
