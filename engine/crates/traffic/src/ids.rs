//! Typed indices into the simulation's entity tables.
//!
//! Network entities (`NodeId`, `RoadId`, `LaneId`, `ConnectorId`,
//! `ApproachId`) are indices into the [`Network`](crate::Network) vectors and
//! never change after `finalize()`. [`VehicleId`] is a stable, monotonically
//! increasing id (never reused) so references to despawned vehicles simply
//! stop resolving.

macro_rules! id_type {
    ($(#[$m:meta])* $name:ident) => {
        $(#[$m])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
        pub struct $name(pub u32);

        impl $name {
            /// The id as a `usize` index.
            #[inline]
            pub fn idx(self) -> usize {
                self.0 as usize
            }
        }

        impl From<usize> for $name {
            #[inline]
            fn from(i: usize) -> Self {
                $name(i as u32)
            }
        }
    };
}

id_type!(
    /// A junction or bend (index into `Network::nodes`).
    NodeId
);
id_type!(
    /// A road between two nodes (index into `Network::roads`).
    RoadId
);
id_type!(
    /// One lane of a road in one direction (index into `Network::lanes`).
    LaneId
);
id_type!(
    /// A path through a junction from one lane to another (index into `Network::connectors`).
    ConnectorId
);
id_type!(
    /// The lanes entering a node from one road (index into `Network::approaches`).
    ApproachId
);
id_type!(
    /// A traffic signal controller (index into `World::signals`).
    SignalId
);
id_type!(
    /// A marked pedestrian crossing (index into the crosswalk list).
    CrosswalkId
);
id_type!(
    /// A vehicle. Stable for the vehicle's lifetime and never reused.
    VehicleId
);
id_type!(
    /// A pedestrian. Stable for its lifetime and never reused.
    PedId
);

/// A drivable segment: a lane or a junction connector.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SegRef {
    Lane(LaneId),
    Connector(ConnectorId),
}

impl SegRef {
    #[inline]
    pub fn is_lane(self) -> bool {
        matches!(self, SegRef::Lane(_))
    }

    #[inline]
    pub fn is_connector(self) -> bool {
        matches!(self, SegRef::Connector(_))
    }

    #[inline]
    pub fn lane(self) -> Option<LaneId> {
        match self {
            SegRef::Lane(l) => Some(l),
            SegRef::Connector(_) => None,
        }
    }

    #[inline]
    pub fn connector(self) -> Option<ConnectorId> {
        match self {
            SegRef::Connector(c) => Some(c),
            SegRef::Lane(_) => None,
        }
    }
}

impl From<LaneId> for SegRef {
    fn from(l: LaneId) -> Self {
        SegRef::Lane(l)
    }
}

impl From<ConnectorId> for SegRef {
    fn from(c: ConnectorId) -> Self {
        SegRef::Connector(c)
    }
}
