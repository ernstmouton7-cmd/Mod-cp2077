//! The world: shared simulation state, the step loop, environment, events,
//! population control and the host-physics bridge.

pub mod environment;
pub mod events;
pub mod sim;
pub mod spawner;
#[allow(clippy::module_inception)]
pub mod world;

pub use environment::{Environment, Weather};
pub use events::{
    CameraKind, DispatchKind, Event, EventQueue, GestureKind, IncidentKind, PlayerStats,
    PulloverStage,
};
pub use sim::{Sim, WorldStats};
pub use spawner::{PopulationOptions, Spawner};
pub use world::{DriveControls, ExternalPose, World, WorldOptions};
