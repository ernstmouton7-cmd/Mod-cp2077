import { AIDriver } from "../ai/driver";
import { Archetype, samplePersonality } from "../ai/personality";
import { Rng } from "../math/rng";
import { Lane } from "../road/network";
import { CIVILIAN_CLASSES, SPECS, VehicleClass } from "../vehicle/specs";
import { Vehicle, VehicleRole } from "../vehicle/vehicle";

/** What the spawner needs from the world. */
export interface SpawnHost {
  readonly rng: Rng;
  readonly vehicles: Vehicle[];
  readonly lanes: Lane[];
  readonly playerPos: (this: void) => { x: number; y: number } | undefined;
  readonly add: (this: void, v: Vehicle, d: AIDriver) => void;
  readonly remove: (this: void, v: Vehicle) => void;
  readonly nextId: (this: void) => number;
  readonly driverOf: (this: void, v: Vehicle) => AIDriver | undefined;
  /** Is a lane position free of other vehicles within `clear` metres? */
  readonly laneFree: (this: void, lane: Lane, s: number, clear: number) => boolean;
  readonly busyUnit: (this: void, v: Vehicle) => boolean;
}

export interface PopulationOptions {
  /** Civilian vehicles at density 1.0. */
  civilians: number;
  police: number;
  /** Minimum spawn/despawn distance from the player. */
  minDistance: number;
}

/**
 * Keeps the population near its target: spawns civilians (weighted vehicle
 * mix, sampled personalities) and patrol cars away from the player, removes
 * long-dead wrecks and surplus vehicles far away.
 */
export class Spawner {
  private timer = 0;
  /** Population control on/off (scripted scenarios turn it off). */
  enabled = true;

  constructor(
    readonly host: SpawnHost,
    readonly opts: PopulationOptions,
  ) {}

  pickClass(): VehicleClass {
    const entries = CIVILIAN_CLASSES.map((c) => ({ item: c, weight: SPECS[c].spawnWeight }));
    return this.host.rng.weighted(entries);
  }

  /** Create a vehicle with a driver on `lane` at `s`. */
  create(cls: VehicleClass, role: VehicleRole, lane: Lane, s: number, speed: number, archetype?: Archetype): Vehicle {
    const h = this.host;
    const spec = SPECS[cls];
    const color = h.rng.pick(spec.colors);
    const v = new Vehicle(h.nextId(), spec, color, role);
    let arch = archetype;
    if (arch === undefined) {
      if (role === "police" || role === "ambulance" || cls === "bus" || cls === "truck") arch = "professional";
      else if (cls === "taxi") arch = h.rng.chance(0.5) ? "professional" : "speeder";
      else if (cls === "sports" && h.rng.chance(0.4)) arch = "joyrider";
    }
    const p = samplePersonality(h.rng, arch);
    if (role === "police") {
      p.lawfulness = 1;
      p.aggression = 0.3;
      p.temper = 0.3;
    }
    const d = new AIDriver(v, p, Math.floor(h.rng.next() * 1e9) + 1);
    v.placeOnSegment(lane, s, speed);
    v.lights.headlightSwitch = h.rng.chance(0.95) ? "auto" : "off";
    h.add(v, d);
    return v;
  }

  /** Find a free spawn point at least `minD` (and at most `maxD`) from the player. */
  findSpot(minD: number, maxD: number): { lane: Lane; s: number } | undefined {
    const h = this.host;
    const pp = h.playerPos();
    for (let attempt = 0; attempt < 25; attempt++) {
      const lane = h.lanes[Math.floor(h.rng.next() * h.lanes.length)];
      if (lane.length < 30) continue;
      const s = h.rng.range(10, lane.length - 15);
      const pt = { x: 0, y: 0, heading: 0 };
      lane.path.sample(s, pt);
      if (pp !== undefined) {
        const d = Math.sqrt((pt.x - pp.x) * (pt.x - pp.x) + (pt.y - pp.y) * (pt.y - pp.y));
        if (d < minD || d > maxD) continue;
      }
      if (!h.laneFree(lane, s, 14)) continue;
      return { lane, s };
    }
    return undefined;
  }

  /** Fill the city at start-up (anywhere except right on top of the player). */
  populate(density: number): void {
    const n = Math.floor(this.opts.civilians * density);
    for (let i = 0; i < n; i++) {
      const spot = this.findSpot(25, Infinity);
      if (spot === undefined) continue;
      this.create(this.pickClass(), "civilian", spot.lane, spot.s, spot.lane.speedLimit * 0.6);
    }
    for (let i = 0; i < this.opts.police; i++) {
      const spot = this.findSpot(60, Infinity);
      if (spot !== undefined) this.create("police", "police", spot.lane, spot.s, 5);
    }
  }

  update(dt: number, density: number): void {
    if (!this.enabled) return;
    this.timer -= dt;
    if (this.timer > 0) return;
    this.timer = 0.5;
    const h = this.host;
    const pp = h.playerPos();
    let civ = 0;
    let police = 0;
    for (const v of h.vehicles) {
      if (v.role === "police") police++;
      else if (v.role !== "player") civ++;
    }
    const target = Math.floor(this.opts.civilians * density);
    // Despawn wrecks and stuck vehicles out of the player's way.
    for (const v of h.vehicles) {
      if (v.isPlayer || h.busyUnit(v)) continue;
      const far = pp === undefined ? true : (v.x - pp.x) * (v.x - pp.x) + (v.y - pp.y) * (v.y - pp.y) > this.opts.minDistance * this.opts.minDistance;
      if (v.wrecked) v.despawnTimer += 0.5;
      if ((v.despawnTimer > 45 && far) || v.despawnTimer > 240) {
        h.remove(v);
        return;
      }
    }
    if (civ < target) {
      const spot = this.findSpot(this.opts.minDistance, Infinity);
      if (spot !== undefined) this.create(this.pickClass(), "civilian", spot.lane, spot.s, spot.lane.speedLimit * 0.7);
    } else if (civ > target * 1.15 + 2) {
      let surplus: Vehicle | undefined = undefined;
      for (const v of h.vehicles) {
        const near = pp !== undefined && (v.x - pp.x) * (v.x - pp.x) + (v.y - pp.y) * (v.y - pp.y) < 150 * 150;
        if (surplus === undefined && v.role === "civilian" && v.mode === "lane" && !near) surplus = v;
      }
      if (surplus !== undefined) h.remove(surplus);
    }
    if (police < this.opts.police) {
      const spot = this.findSpot(this.opts.minDistance, Infinity);
      if (spot !== undefined) this.create("police", "police", spot.lane, spot.s, 5);
    }
  }
}
