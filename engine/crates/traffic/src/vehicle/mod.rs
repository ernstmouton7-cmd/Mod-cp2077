//! Vehicles: specs, powertrain, rigid-body dynamics, lights and damage.

pub mod damage;
pub mod dynamics;
pub mod lights;
pub mod powertrain;
pub mod specs;
#[allow(clippy::module_inception)]
pub mod vehicle;

pub use damage::{DamageModel, DamageZone};
pub use dynamics::{Assists, DriveInputs, VehicleDynamics};
pub use lights::{HeadlightSwitch, Indicator, SirenMode, VehicleLights, WiperMode};
pub use powertrain::{Powertrain, TransmissionMode};
pub use specs::{max_accel_at, spec, top_speed, DriveLayout, VehicleClass, VehicleSpec, CIVILIAN_CLASSES, SPECS};
pub use vehicle::{MotionMode, Vehicle, VehicleRole};
