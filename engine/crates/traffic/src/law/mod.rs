//! Law enforcement: violations, the player's record, monitoring and police.

pub mod player_monitor;
pub mod police;
pub mod violations;

pub use player_monitor::{AheadControl, AheadInfo, HitRun, Observation, PlayerMonitor, RoadInfo};
pub use police::{PoliceStage, PoliceSystem};
pub use violations::{DrivingRecord, TicketSummary, ViolationDef, ViolationKind, ViolationRecord, Witness};
