import { Path } from "../math/path";

export type TurnType = "left" | "straight" | "right" | "uturn";

/**
 * How an intersection is controlled.
 *  - signal:       traffic lights (see signals/signalController)
 *  - allway_stop:  every approach has a stop sign, first-come first-served
 *  - twoway_stop:  minor approaches stop, major road has priority
 *  - yield:        minor approaches yield (no full stop required)
 *  - none:         uncontrolled (bends, merges); right-hand rule applies
 */
export type ControlType = "signal" | "allway_stop" | "twoway_stop" | "yield" | "none";

/** Sign facing an individual approach. */
export type ApproachSign = "signal" | "stop" | "yield" | "none";

export type RoadKind = "avenue" | "street" | "oneway" | "boulevard" | "ring" | "alley";

/** Anything that can occupy road space (AI cars, the player, wrecks). */
export interface Occupant {
  readonly id: number;
  length: number;
  width: number;
  /** Signed speed along own heading (m/s). */
  speed: number;
  /** Connector this occupant intends to take next (-1 unknown); lets others yield correctly. */
  intentConnector: number;
}

/** One occupancy record on a segment. Pooled & rebuilt each simulation step. */
export interface OccEntry {
  occ: Occupant;
  /** Arc length of the occupant's centre on the segment. */
  s: number;
  /** Speed component along the segment direction. */
  v: number;
  /** True when this occupant is not lane-bound (player, wreck, lane changer's shadow). */
  phantom: boolean;
  /** Lateral offset from the segment centreline (m, + = left). */
  lat: number;
}

/** Common interface of lanes and connectors: a drivable path. */
export interface Segment {
  readonly id: number;
  readonly kind: "lane" | "connector";
  readonly path: Path;
  readonly length: number;
  speedLimit: number;
  /** Occupancy, valid for the current step; entries [0, occCount) are live. */
  occ: OccEntry[];
  occCount: number;
}
