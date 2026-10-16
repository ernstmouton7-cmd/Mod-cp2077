import { angleDiff, clamp, DEG } from "../math/scalar";
import { Rng } from "../math/rng";
import { Crosswalk } from "../road/crosswalks";
import { SignalController } from "../signals/signalController";

export type PedState = "waiting" | "crossing" | "leaving" | "down";

/** A pedestrian using a crosswalk. Positions are in sim coordinates. */
export class Pedestrian {
  x = 0;
  y = 0;
  heading = 0;
  /** Position along the crosswalk 0..1 and walking direction (+1 a->b, -1 b->a). */
  t = 0;
  dir = 1;
  state: PedState = "waiting";
  waited = 0;
  /** Seconds left before despawn while leaving / lying down. */
  ttl = 0;
  /** Animation phase for renderers. */
  stride = 0;
  readonly speed: number;
  /** Starts crossing on a flashing hand / accepts shorter gaps. */
  readonly reckless: boolean;
  readonly shirt: number;
  readonly height: number;
  /** Seconds until this pedestrian may shout again. */
  shoutCooldown = 0;

  constructor(
    readonly id: number,
    readonly cw: Crosswalk,
    rng: Rng,
  ) {
    this.speed = clamp(rng.gauss(1.35, 0.25), 0.8, 2.0);
    this.reckless = rng.chance(0.08);
    this.shirt = rng.pick([0xe0e0e0, 0x2b2b2b, 0xb03a2e, 0x2e86c1, 0xf1c40f, 0x7d3c98, 0x16a085, 0xd35400, 0x34495e]);
    this.height = clamp(rng.gauss(1.72, 0.08), 1.5, 1.95);
    this.dir = rng.chance(0.5) ? 1 : -1;
    this.t = this.dir > 0 ? 0 : 1;
    this.place();
  }

  place(): void {
    const cw = this.cw;
    // Waiting pedestrians stand back on the sidewalk, a step from the curb.
    let t = this.t;
    if (this.state === "waiting") t = this.dir > 0 ? -0.9 / cw.length : 1 + 0.9 / cw.length;
    this.x = cw.ax + (cw.bx - cw.ax) * t;
    this.y = cw.ay + (cw.by - cw.ay) * t;
    this.heading = Math.atan2((cw.by - cw.ay) * this.dir, (cw.bx - cw.ax) * this.dir);
  }
}

/** What the pedestrian system needs from the world. */
export interface PedHost {
  readonly signals: SignalController[];
  /** Vehicles near a point or heading through it within `seconds` (gap acceptance). */
  readonly trafficApproaching: (this: void, x: number, y: number, radius: number, seconds: number) => boolean;
  /** Is a vehicle body within `margin` of the point? */
  readonly vehicleAt: (this: void, x: number, y: number, margin: number) => boolean;
  readonly onShout: (this: void, p: Pedestrian) => void;
}

/**
 * Spawns pedestrians at crosswalk ends, lets them cross with the walk signal
 * (or a safe gap at stop-sign junctions), makes them look before stepping off
 * and pause for cars in their path, and removes them once they are done.
 */
export class PedestrianSystem {
  readonly peds: Pedestrian[] = [];
  private nextId = 1;
  private spawnTimer = 0;
  /** Target number of pedestrians (scaled by time of day by the world). */
  target = 40;

  constructor(
    readonly crosswalks: Crosswalk[],
    readonly rng: Rng,
  ) {}

  /** Is walking allowed across this crosswalk right now? */
  walkAllowed(host: PedHost, cw: Crosswalk, reckless: boolean): boolean {
    const n = cw.node;
    if (n.control !== "signal" || n.signalId < 0) return false;
    const sig = host.signals[n.signalId];
    if (sig.mode === "flashing" || sig.mode === "off") return false;
    // Walk with the through phase of approaches perpendicular to the road we cross.
    for (const ap of n.approaches) {
      if (ap.road === cw.road) continue;
      const rel = Math.abs(Math.abs(angleDiff(ap.heading, cw.roadHeading)) - 90 * DEG);
      if (rel > 30 * DEG) continue;
      const h = sig.head(ap);
      if (h.ped === "walk" || (reckless && h.ped === "flash")) return true;
    }
    return false;
  }

  update(dt: number, host: PedHost): void {
    this.spawnTimer -= dt;
    if (this.spawnTimer <= 0 && this.crosswalks.length > 0) {
      this.spawnTimer = 0.6;
      if (this.peds.length < this.target) {
        const cw = this.rng.pick(this.crosswalks);
        let crowded = 0;
        for (const p of this.peds) if (p.cw === cw && p.state === "waiting") crowded++;
        if (crowded < 4) this.peds.push(new Pedestrian(this.nextId++, cw, this.rng));
      }
    }
    for (let i = this.peds.length - 1; i >= 0; i--) {
      const p = this.peds[i];
      p.shoutCooldown -= dt;
      const cw = p.cw;
      if (p.state === "waiting") {
        p.waited += dt;
        let go = false;
        if (cw.node.control === "signal") go = this.walkAllowed(host, cw, p.reckless);
        else {
          const midx = (cw.ax + cw.bx) / 2;
          const midy = (cw.ay + cw.by) / 2;
          go = p.waited > 1.5 && !host.trafficApproaching(midx, midy, cw.length / 2 + 2, p.reckless ? 1.5 : 3.5);
        }
        if (p.waited > 120) go = true; // nobody waits forever
        // Look before stepping off: a car turning across the crosswalk goes first.
        if (go) {
          const sx = cw.ax + (cw.bx - cw.ax) * p.t;
          const sy = cw.ay + (cw.by - cw.ay) * p.t;
          if (host.vehicleAt(sx, sy, 2.2) || host.trafficApproaching(sx, sy, 2.5, p.reckless ? 0.6 : 1.4)) go = false;
        }
        if (go) {
          p.state = "crossing";
          p.place();
        }
      } else if (p.state === "crossing") {
        // Pause while a car body is right in our path.
        const ux = (cw.bx - cw.ax) / cw.length;
        const uy = (cw.by - cw.ay) / cw.length;
        if (host.vehicleAt(p.x + ux * p.dir * 1.0, p.y + uy * p.dir * 1.0, 0.4)) {
          if (p.shoutCooldown <= 0 && host.vehicleAt(p.x, p.y, 1.0)) {
            p.shoutCooldown = 6;
            host.onShout(p);
          }
          continue;
        }
        p.t += (p.dir * p.speed * dt) / cw.length;
        p.stride += p.speed * dt * 1.8;
        if (p.t >= 1 || p.t <= 0) {
          p.t = clamp(p.t, 0, 1);
          p.state = "leaving";
          p.ttl = 4;
        }
        p.place();
      } else if (p.state === "leaving") {
        // Walk along the sidewalk away from the junction, then vanish.
        const away = cw.roadHeading;
        p.x += Math.cos(away) * p.speed * dt;
        p.y += Math.sin(away) * p.speed * dt;
        p.heading = away;
        p.stride += p.speed * dt * 1.8;
        p.ttl -= dt;
        if (p.ttl <= 0) this.peds.splice(i, 1);
      } else {
        p.ttl -= dt;
        if (p.ttl <= 0) this.peds.splice(i, 1);
      }
    }
  }

  /** Is someone crossing at (or walking toward) position t (0..1) within `halfWidth` metres? */
  blocking(cw: Crosswalk, t: number, halfWidth: number): boolean {
    for (const p of this.peds) {
      if (p.cw !== cw || p.state !== "crossing") continue;
      const along = (p.t - t) * cw.length;
      const approaching = along * p.dir < 0 && Math.abs(along) < halfWidth + p.speed * 2.5;
      if (Math.abs(along) < halfWidth || approaching) return true;
    }
    return false;
  }
}
