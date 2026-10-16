//! Road model: network, routing, right-of-way, crosswalks and the city generator.

pub mod city_builder;
pub mod controls;
pub mod crosswalks;
pub mod network;
pub mod router;
pub mod types;

pub use city_builder::{build_city, Building, BuildingKind, City, CityOptions, SpeedCamera};
pub use crosswalks::{build_crosswalks, Crosswalk, CrosswalkCrossing};
pub use network::{
    Approach, ConflictKind, ConflictZone, Connector, Lane, LaneHit, Network, Node, Road,
    RoadOptions,
};
pub use router::{RouteStep, Router};
pub use types::{ApproachSign, ControlType, OccEntry, Occupancy, RoadKind, TurnType};
