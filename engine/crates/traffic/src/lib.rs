//! Gridlock traffic: an engine-agnostic city traffic simulation.
//!
//! A Rust port of the TypeScript `src/core`: a procedural road network with
//! lanes, junction connectors and conflict zones; actuated/fixed/flashing
//! traffic signals with pre-emption; vehicle powertrain and rigid-body
//! "bicycle" dynamics, lights and damage; AI drivers with IDM car following,
//! MOBIL lane changes, junction rules, emotions and road rage; pedestrians;
//! law enforcement (violations, cameras, police pull-overs and pursuits);
//! player input and driver; incidents and the environment.
//!
//! Units: metres, seconds, radians; x east, y north, heading CCW from +x.
//!
//! The [`World`] owns everything. Call [`World::step`] at a fixed rate and
//! [`World::drain_events`] to consume gameplay events. For a host with its
//! own 3D physics, build the world with `WorldOptions { external_physics:
//! true, .. }`, feed poses with [`World::set_external_pose`], read AI
//! controls with [`World::ai_controls`] and report contacts with
//! [`World::report_collision`].

#![allow(clippy::too_many_arguments)]

pub mod ai;
pub mod ids;
pub mod law;
pub mod math;
pub mod physics;
pub mod player;
pub mod road;
pub mod signals;
pub mod util;
pub mod vehicle;
pub mod world;

pub use ai::{
    idm_accel, idm_free, line_for, offense_anger, sample_personality, AIDriver, Archetype,
    DriverState, Emotion, HoldKind, IdmParams, Mood, OffenseKind, PedState, Pedestrian,
    PedestrianSystem, Personality, RageBehavior, ARCHETYPES,
};
pub use ids::{
    ApproachId, ConnectorId, CrosswalkId, LaneId, NodeId, PedId, RoadId, SegRef, SignalId,
    VehicleId,
};
pub use law::{
    DrivingRecord, Observation, PlayerMonitor, PoliceStage, PoliceSystem, ViolationDef,
    ViolationKind, ViolationRecord, Witness,
};
pub use math::{Path, Rng, Vec2};
pub use player::{
    action_def, ActionDef, ActionId, ControlFrame, InputMapper, PlayerDriver, ACTIONS,
};
pub use road::{
    build_city, Approach, ApproachSign, Building, City, CityOptions, ConflictKind, ConflictZone,
    Connector, ControlType, Crosswalk, Lane, Network, Node, Road, RoadKind, RoadOptions, Router,
    TurnType,
};
pub use signals::{
    HeadState, Indication, Lamp, PedLamp, PhaseState, SignalController, SignalMode, SignalOptions,
};
pub use vehicle::{
    max_accel_at, spec, top_speed, MotionMode, Powertrain, Vehicle, VehicleClass, VehicleDynamics,
    VehicleLights, VehicleRole, VehicleSpec, CIVILIAN_CLASSES, SPECS,
};
pub use world::{
    DriveControls, Environment, Event, ExternalPose, Sim, Weather, World, WorldOptions, WorldStats,
};

/// Crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
