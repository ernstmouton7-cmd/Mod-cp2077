//! Gameplay events emitted by the simulation, drained by the host each step.

use crate::ai::driver::RageBehavior;
use crate::ai::emotion::{Mood, OffenseKind};
use crate::ids::{NodeId, PedId, VehicleId};
use crate::law::violations::{ViolationKind, Witness};
use crate::signals::PhaseState;
use std::collections::HashSet;

/// Pull-over stages reported by the police system.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PulloverStage {
    Request,
    Stopped,
    Issued,
    Fled,
    Escaped,
}

/// What a dispatched unit is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DispatchKind {
    Police,
    Ambulance,
}

/// What kind of camera fired.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CameraKind {
    RedLight,
    Speed,
}

/// Player gesture.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GestureKind {
    Wave,
    Rude,
}

/// Random incidents.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IncidentKind {
    Breakdown,
    PowerCut,
    PowerRestored,
}

/// Every gameplay event the simulation emits (mirrors `world/events.ts`).
/// Renderers, audio and mod adapters consume them via `World::drain_events`.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Horn {
        vehicle: VehicleId,
        long: bool,
    },
    Flash {
        vehicle: VehicleId,
    },
    Speech {
        vehicle: VehicleId,
        text: String,
        mood: Mood,
    },
    Offense {
        offender: VehicleId,
        victim: VehicleId,
        kind: OffenseKind,
        anger: f64,
    },
    Mood {
        vehicle: VehicleId,
        mood: Mood,
        target: Option<VehicleId>,
    },
    RageStart {
        vehicle: VehicleId,
        target: VehicleId,
        behavior: RageBehavior,
    },
    RageEnd {
        vehicle: VehicleId,
        reason: &'static str,
    },
    Collision {
        a: VehicleId,
        b: VehicleId,
        impulse: f64,
        severity: f64,
        x: f64,
        y: f64,
        at_fault: VehicleId,
    },
    CrashStatic {
        vehicle: VehicleId,
        severity: f64,
        x: f64,
        y: f64,
    },
    NearMiss {
        a: VehicleId,
        b: VehicleId,
    },
    Signal {
        node: NodeId,
        phase: String,
        state: PhaseState,
    },
    Violation {
        vehicle: VehicleId,
        kind: ViolationKind,
        label: &'static str,
        fine: f64,
        points: u32,
        witnessed: Witness,
        x: f64,
        y: f64,
    },
    Ticket {
        vehicle: VehicleId,
        total: f64,
        points: u32,
        reason: String,
    },
    CameraFlash {
        x: f64,
        y: f64,
        kind: CameraKind,
    },
    Heat {
        level: u32,
        reason: &'static str,
    },
    Pullover {
        police: Option<VehicleId>,
        stage: PulloverStage,
    },
    Dispatch {
        unit: VehicleId,
        kind: DispatchKind,
        x: f64,
        y: f64,
        reason: &'static str,
    },
    Apology {
        from: VehicleId,
        to: VehicleId,
        calmed: f64,
    },
    Gesture {
        from: VehicleId,
        kind: GestureKind,
    },
    Stall {
        vehicle: VehicleId,
    },
    Engine {
        vehicle: VehicleId,
        on: bool,
    },
    Shift {
        vehicle: VehicleId,
        gear: String,
    },
    Grind {
        vehicle: VehicleId,
    },
    Chime {
        vehicle: VehicleId,
        reason: &'static str,
    },
    Spawn {
        vehicle: VehicleId,
    },
    Despawn {
        vehicle: VehicleId,
    },
    PedSpeech {
        ped: PedId,
        text: &'static str,
        x: f64,
        y: f64,
    },
    PedHit {
        ped: PedId,
        vehicle: VehicleId,
        severity: f64,
        x: f64,
        y: f64,
    },
    Incident {
        kind: IncidentKind,
        x: f64,
        y: f64,
        vehicle: Option<VehicleId>,
        node: Option<NodeId>,
        place: String,
    },
    Info {
        text: String,
    },
}

impl Event {
    /// The TypeScript `type` tag of this event (`"horn"`, `"rage_start"`...).
    pub fn type_name(&self) -> &'static str {
        match self {
            Event::Horn { .. } => "horn",
            Event::Flash { .. } => "flash",
            Event::Speech { .. } => "speech",
            Event::Offense { .. } => "offense",
            Event::Mood { .. } => "mood",
            Event::RageStart { .. } => "rage_start",
            Event::RageEnd { .. } => "rage_end",
            Event::Collision { .. } => "collision",
            Event::CrashStatic { .. } => "crash_static",
            Event::NearMiss { .. } => "near_miss",
            Event::Signal { .. } => "signal",
            Event::Violation { .. } => "violation",
            Event::Ticket { .. } => "ticket",
            Event::CameraFlash { .. } => "camera_flash",
            Event::Heat { .. } => "heat",
            Event::Pullover { .. } => "pullover",
            Event::Dispatch { .. } => "dispatch",
            Event::Apology { .. } => "apology",
            Event::Gesture { .. } => "gesture",
            Event::Stall { .. } => "stall",
            Event::Engine { .. } => "engine",
            Event::Shift { .. } => "shift",
            Event::Grind { .. } => "grind",
            Event::Chime { .. } => "chime",
            Event::Spawn { .. } => "spawn",
            Event::Despawn { .. } => "despawn",
            Event::PedSpeech { .. } => "ped_speech",
            Event::PedHit { .. } => "ped_hit",
            Event::Incident { .. } => "incident",
            Event::Info { .. } => "info",
        }
    }
}

/// The player's session statistics (driver report).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlayerStats {
    pub distance: f64,
    pub drive_time: f64,
    pub max_speed: f64,
    pub collisions: u32,
    pub near_misses: u32,
    /// Drivers the player made at least irritated.
    pub angered: u32,
    pub apologies_accepted: u32,
    pub rage_episodes: u32,
    pub honks: u32,
    pub pedestrians_hit: u32,
    pub red_lights_run: u32,
}

/// The per-step event queue. Every emitted event is also fed to the
/// player-statistics tracker. The queue keeps at most `limit` undrained events.
#[derive(Clone, Debug)]
pub struct EventQueue {
    queue: Vec<Event>,
    pub limit: usize,
    /// Total events emitted (including dropped ones).
    pub emitted: u64,
    pub(crate) player: Option<VehicleId>,
    pub player_stats: PlayerStats,
    angered: HashSet<VehicleId>,
}

impl Default for EventQueue {
    fn default() -> Self {
        EventQueue {
            queue: Vec::new(),
            limit: 512,
            emitted: 0,
            player: None,
            player_stats: PlayerStats::default(),
            angered: HashSet::new(),
        }
    }
}

impl EventQueue {
    pub fn emit(&mut self, e: Event) {
        self.emitted += 1;
        self.track(&e);
        if self.queue.len() < self.limit {
            self.queue.push(e);
        }
    }

    /// Returns and clears the queued events.
    pub fn drain(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.queue)
    }

    /// Events queued since the last drain.
    pub fn pending(&self) -> &[Event] {
        &self.queue
    }

    fn track(&mut self, e: &Event) {
        let Some(p) = self.player else { return };
        let st = &mut self.player_stats;
        match *e {
            Event::Collision { a, b, severity, .. } if (a == p || b == p) && severity > 0.8 => {
                st.collisions += 1
            }
            Event::NearMiss { a, .. } if a == p => st.near_misses += 1,
            Event::Offense {
                offender,
                victim,
                anger,
                ..
            } if offender == p && anger > 22.0 && !self.angered.contains(&victim) => {
                self.angered.insert(victim);
                st.angered += 1;
            }
            Event::Apology { calmed, .. } if calmed > 5.0 => st.apologies_accepted += 1,
            Event::RageStart { target, .. } if target == p => st.rage_episodes += 1,
            Event::Horn { vehicle, long } if vehicle == p && !long => st.honks += 1,
            Event::PedHit { vehicle, .. } if vehicle == p => st.pedestrians_hit += 1,
            Event::Violation {
                kind: ViolationKind::RedLight,
                ..
            } => st.red_lights_run += 1,
            _ => {}
        }
    }
}
