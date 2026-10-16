//! AI drivers: car following, lane changes, junction rules, emotions, road
//! rage, personalities and pedestrians.

pub mod control;
pub mod driver;
pub mod emotion;
pub mod idm;
pub mod intersection;
pub mod pedestrians;
pub mod personality;
pub mod speech;

pub use driver::{AIDriver, DriverState, HoldKind, Leader, RageBehavior, RageState};
pub use emotion::{offense_anger, Emotion, Mood, OffenseKind};
pub use idm::{decel_to_stop, idm_accel, idm_free, IdmParams};
pub use intersection::{EntryMode, JunctionState};
pub use pedestrians::{PedHost, PedState, Pedestrian, PedestrianSystem};
pub use personality::{sample_personality, Archetype, Personality, ARCHETYPES};
pub use speech::{line_for, APOLOGY_ACCEPTED, APOLOGY_REJECTED, POLICE_LINES};
