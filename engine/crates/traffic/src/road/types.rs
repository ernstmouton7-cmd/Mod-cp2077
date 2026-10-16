//! Enumerations and small records shared by the road model.

use crate::ids::{ConnectorId, LaneId, SegRef, VehicleId};

/// Movement through a junction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TurnType {
    Left,
    Straight,
    Right,
    UTurn,
}

/// How an intersection is controlled.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ControlType {
    /// Traffic lights (see [`SignalController`](crate::SignalController)).
    Signal,
    /// Every approach has a stop sign, first-come first-served.
    AllwayStop,
    /// Minor approaches stop, the major road has priority.
    TwowayStop,
    /// Minor approaches yield (no full stop required).
    Yield,
    /// Uncontrolled (bends, merges); the right-hand rule applies.
    None,
}

/// Sign facing an individual approach.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ApproachSign {
    Signal,
    Stop,
    Yield,
    None,
}

/// Road classification (lane counts, speed, right-of-way rank).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RoadKind {
    Avenue,
    Street,
    Oneway,
    Boulevard,
    Ring,
    Alley,
}

/// One occupancy record on a segment, rebuilt every simulation step.
#[derive(Clone, Copy, Debug)]
pub struct OccEntry {
    pub vehicle: VehicleId,
    /// Index of the vehicle in the world's vehicle list when registered (hint).
    pub(crate) vi: u32,
    /// Arc length of the occupant's centre on the segment.
    pub s: f64,
    /// Speed component along the segment direction.
    pub v: f64,
    /// True when the occupant is not lane-bound here (player, wreck, lane-change shadow).
    pub phantom: bool,
    /// Lateral offset from the segment centreline (m, + = left).
    pub lat: f64,
    pub length: f64,
    pub width: f64,
    /// Intended connector when registered (live value is read from the vehicle).
    pub intent: Option<ConnectorId>,
}

/// Which vehicles occupy each lane and connector during the current step.
#[derive(Clone, Debug, Default)]
pub struct Occupancy {
    pub lanes: Vec<Vec<OccEntry>>,
    pub connectors: Vec<Vec<OccEntry>>,
}

impl Occupancy {
    pub fn new(lanes: usize, connectors: usize) -> Self {
        Occupancy { lanes: vec![Vec::new(); lanes], connectors: vec![Vec::new(); connectors] }
    }

    /// Reset every segment (start of a step).
    pub fn clear(&mut self) {
        for l in &mut self.lanes {
            l.clear();
        }
        for c in &mut self.connectors {
            c.clear();
        }
    }

    #[inline]
    pub fn seg(&self, s: SegRef) -> &[OccEntry] {
        match s {
            SegRef::Lane(l) => &self.lanes[l.idx()],
            SegRef::Connector(c) => &self.connectors[c.idx()],
        }
    }

    #[inline]
    pub fn lane(&self, l: LaneId) -> &[OccEntry] {
        &self.lanes[l.idx()]
    }

    #[inline]
    pub fn connector(&self, c: ConnectorId) -> &[OccEntry] {
        &self.connectors[c.idx()]
    }

    #[inline]
    pub fn add(&mut self, s: SegRef, e: OccEntry) {
        match s {
            SegRef::Lane(l) => self.lanes[l.idx()].push(e),
            SegRef::Connector(c) => self.connectors[c.idx()].push(e),
        }
    }
}
