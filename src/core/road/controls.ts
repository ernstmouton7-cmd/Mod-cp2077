import { Network, Node, Road } from "./network";
import { RoadKind } from "./types";

const KIND_RANK: Record<RoadKind, number> = {
  ring: 5,
  boulevard: 5,
  avenue: 4,
  oneway: 3,
  street: 2,
  alley: 1,
};

/** Importance of a road for right-of-way decisions. */
export function roadRank(r: Road): number {
  return KIND_RANK[r.kind] * 10 + r.opts.lanesFwd + r.opts.lanesBack;
}

/**
 * Translate each node's control type into per-approach signs and major-road
 * flags. Call after Network.finalize().
 */
export function assignApproachSigns(net: Network): void {
  for (const n of net.nodes) assignNode(n);
}

export function assignNode(n: Node): void {
  let best = -1;
  for (const r of n.roads) best = Math.max(best, roadRank(r));
  for (const ap of n.approaches) {
    const isMajor = roadRank(ap.road) >= best;
    switch (n.control) {
      case "signal":
        ap.sign = "signal";
        ap.major = isMajor;
        break;
      case "allway_stop":
        ap.sign = "stop";
        ap.major = false;
        break;
      case "twoway_stop":
        ap.sign = isMajor ? "none" : "stop";
        ap.major = isMajor;
        break;
      case "yield":
        ap.sign = isMajor ? "none" : "yield";
        ap.major = isMajor;
        break;
      default:
        ap.sign = "none";
        ap.major = true;
    }
  }
  // A two-way stop where every road ties for rank degenerates: make it all-way.
  if (n.control === "twoway_stop" || n.control === "yield") {
    let minors = 0;
    for (const ap of n.approaches) if (!ap.major) minors++;
    if (minors === 0 && n.approaches.length >= 3) {
      n.control = "allway_stop";
      for (const ap of n.approaches) {
        ap.sign = "stop";
        ap.major = false;
      }
    }
  }
}
