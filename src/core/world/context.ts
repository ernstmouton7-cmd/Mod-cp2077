import type { AIDriver } from "../ai/driver";
import type { OffenseKind } from "../ai/emotion";
import type { Rng } from "../math/rng";
import type { Network } from "../road/network";
import type { Router } from "../road/router";
import type { SignalController } from "../signals/signalController";
import type { EventBus } from "../util/events";
import type { Vehicle } from "../vehicle/vehicle";
import type { Environment } from "./environment";
import type { SimEvent } from "./events";

/** All-way stop bookkeeping: arrival order per node. */
export interface AllWayQueue {
  /** vehicle id -> arrival time */
  arrivals: Map<number, number>;
}

/** What AI and law systems may read from / do to the world. */
export interface SimContext {
  readonly time: number;
  readonly net: Network;
  readonly router: Router;
  /** Indexed by node.signalId. */
  readonly signals: SignalController[];
  readonly vehicles: Vehicle[];
  readonly events: EventBus<SimEvent>;
  readonly rng: Rng;
  readonly env: Environment;
  readonly player: Vehicle | undefined;
  vehicle(id: number): Vehicle | undefined;
  driverOf(v: Vehicle): AIDriver | undefined;
  /** Route an offense to the victim's emotions (and stats). */
  offend(offender: Vehicle, victim: Vehicle, kind: OffenseKind, scale: number): void;
  allWay(nodeId: number): AllWayQueue;
  /** Police units within range of a point that can see it. */
  policeNear(x: number, y: number, range: number): Vehicle[];
  /** Unobstructed line of sight (buildings block). */
  los(ax: number, ay: number, bx: number, by: number): boolean;
}
