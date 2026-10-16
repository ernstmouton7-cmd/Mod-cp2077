import { AIDriver } from "../ai/driver";
import { Rng } from "../math/rng";
import { hypot } from "../math/scalar";
import { Vehicle } from "../vehicle/vehicle";
import type { SimContext } from "../world/context";
import { POLICE_LINES } from "../ai/speech";
import { DrivingRecord } from "./violations";

export type PoliceStage = "none" | "responding" | "pullover" | "stopped" | "pursuit" | "searching";

/**
 * Traffic police: dispatches the nearest patrol car when an officer
 * witnesses a violation, performs the pull-over (follow, wait for the player
 * to stop, write the ticket) and escalates to a pursuit with more units when
 * the player flees. Losing all units for long enough ends the pursuit.
 */
export class PoliceSystem {
  stage: PoliceStage = "none";
  readonly units: Vehicle[] = [];
  private timer = 0;
  private stillTime = 0;
  private fleeTime = 0;
  private lostTime = 0;
  private routeTimer = 0;
  private pursuitTime = 0;
  /** Countdown shown to the player while being written up. */
  ticketCountdown = 0;
  private rng = new Rng(911);
  /** Set by the world: spawns a police car near (x, y) out of sight. */
  spawnUnit: ((x: number, y: number) => Vehicle | undefined) | undefined = undefined;

  active(): boolean {
    return this.stage !== "none";
  }

  /** An officer (unit) saw the player commit a violation. */
  report(ctx: SimContext, unit: Vehicle | undefined, rec: DrivingRecord, heat: number): void {
    const p = ctx.player;
    if (p === undefined) return;
    rec.heat = Math.min(5, Math.max(rec.heat, this.stage === "pursuit" ? rec.heat + heat * 0.5 : heat));
    ctx.events.emit({ type: "heat", level: Math.floor(rec.heat), reason: "violation witnessed" });
    if (this.stage === "none") {
      const u = unit ?? this.nearestFreeUnit(ctx, p.x, p.y);
      if (u === undefined) return;
      this.assign(ctx, u);
      this.stage = "responding";
      this.timer = 0;
      ctx.events.emit({ type: "dispatch", unit: u.id, kind: "police", x: p.x, y: p.y, reason: "traffic violation" });
    } else if (this.stage === "pursuit" && rec.heat >= 3 && this.units.length < Math.floor(rec.heat)) {
      this.addUnit(ctx);
    }
  }

  private nearestFreeUnit(ctx: SimContext, x: number, y: number): Vehicle | undefined {
    let best: Vehicle | undefined = undefined;
    let bd = Infinity;
    for (const v of ctx.vehicles) {
      if (v.role !== "police" || !v.alive || v.wrecked || this.units.indexOf(v) >= 0) continue;
      const d = (v.x - x) * (v.x - x) + (v.y - y) * (v.y - y);
      if (d < bd) {
        bd = d;
        best = v;
      }
    }
    return best;
  }

  private assign(ctx: SimContext, u: Vehicle): void {
    this.units.push(u);
    const d = ctx.driverOf(u);
    if (d !== undefined) {
      d.emergency = true;
      d.rage = undefined;
    }
    u.lights.siren = "wail";
    u.lights.lightbar = true;
  }

  private addUnit(ctx: SimContext): void {
    const p = ctx.player;
    if (p === undefined) return;
    let u = this.nearestFreeUnit(ctx, p.x, p.y);
    if ((u === undefined || hypot(u.x - p.x, u.y - p.y) > 400) && this.spawnUnit !== undefined) u = this.spawnUnit(p.x, p.y);
    if (u === undefined) return;
    this.assign(ctx, u);
    ctx.events.emit({ type: "dispatch", unit: u.id, kind: "police", x: p.x, y: p.y, reason: "backup" });
  }

  private release(ctx: SimContext): void {
    for (const u of this.units) {
      const d = ctx.driverOf(u);
      if (d !== undefined) {
        d.emergency = false;
        d.followTarget = -1;
        d.headFor = undefined;
        if (u.mode === "physics") d.state = "recovering";
      }
      u.lights.siren = "off";
      u.lights.lightbar = false;
    }
    this.units.splice(0, this.units.length);
    this.stage = "none";
    this.stillTime = 0;
    this.fleeTime = 0;
    this.lostTime = 0;
    this.pursuitTime = 0;
    this.ticketCountdown = 0;
  }

  private seen(ctx: SimContext, p: Vehicle, range: number): Vehicle | undefined {
    for (const u of this.units) {
      const dx = u.x - p.x;
      const dy = u.y - p.y;
      if (dx * dx + dy * dy < range * range && ctx.los(u.x, u.y, p.x, p.y)) return u;
    }
    return undefined;
  }

  update(dt: number, ctx: SimContext, rec: DrivingRecord): void {
    const p = ctx.player;
    if (p === undefined || this.stage === "none") return;
    // Drop wrecked units.
    for (let i = this.units.length - 1; i >= 0; i--) {
      const u = this.units[i];
      if (!u.alive || u.wrecked) this.units.splice(i, 1);
    }
    if (this.units.length === 0) {
      if (this.stage === "pursuit" || this.stage === "searching") {
        this.addUnit(ctx);
        if (this.units.length === 0) {
          ctx.events.emit({ type: "pullover", police: -1, stage: "escaped" });
          rec.heat = 0;
          this.release(ctx);
        }
      } else this.release(ctx);
      return;
    }
    this.timer += dt;
    const lead = this.units[0];
    const leadDriver = ctx.driverOf(lead);
    const dist = hypot(lead.x - p.x, lead.y - p.y);
    const speed = p.absSpeed();

    // Units not yet close drive on the road network toward the player.
    this.routeTimer -= dt;
    for (const u of this.units) {
      const d = ctx.driverOf(u);
      if (d === undefined) continue;
      const du = hypot(u.x - p.x, u.y - p.y);
      const close = du < 55 && ctx.los(u.x, u.y, p.x, p.y);
      if (close && d.followTarget < 0) {
        d.followTarget = p.id;
        d.followGap = this.stage === "pursuit" ? (rec.heat >= 3 && this.rng.chance(0.5) ? 1 : 5) : 11;
        u.enterPhysics();
      } else if (!close && d.followTarget >= 0 && du > 140) {
        d.followTarget = -1;
        d.state = "recovering";
      }
      if (d.followTarget < 0 && u.mode === "lane" && this.routeTimer <= 0) {
        const hits = ctx.net.nearestLanes(p.x, p.y, 25, []);
        if (hits.length > 0) d.routeTo(ctx, hits[0].lane.road, hits[0].lane.forward);
      }
    }
    if (this.routeTimer <= 0) this.routeTimer = 3;

    switch (this.stage) {
      case "responding":
        if (leadDriver !== undefined && leadDriver.followTarget === p.id) {
          this.stage = "pullover";
          this.timer = 0;
          ctx.events.emit({ type: "pullover", police: lead.id, stage: "request" });
          ctx.events.emit({ type: "speech", vehicle: lead.id, text: this.rng.pick(POLICE_LINES), mood: "calm" });
        } else if (this.timer > 90) {
          // Took too long: give up.
          rec.heat = 0;
          this.release(ctx);
        }
        break;
      case "pullover": {
        if (speed < 0.6 && dist < 35) this.stillTime += dt;
        else this.stillTime = 0;
        if (speed > 13 || dist > 100) this.fleeTime += dt;
        else this.fleeTime = Math.max(0, this.fleeTime - dt);
        if (this.stillTime > 2.5) {
          this.stage = "stopped";
          this.timer = 0;
          this.ticketCountdown = 6;
          ctx.events.emit({ type: "pullover", police: lead.id, stage: "stopped" });
        } else if (this.fleeTime > 6) {
          this.stage = "pursuit";
          this.pursuitTime = 0;
          ctx.events.emit({ type: "pullover", police: lead.id, stage: "fled" });
          rec.heat = Math.max(rec.heat, 2);
          ctx.events.emit({ type: "heat", level: Math.floor(rec.heat), reason: "evading" });
          this.onFlee(ctx, rec);
        } else if (this.timer > 8 && this.rng.chance(dt * 0.3)) {
          ctx.events.emit({ type: "speech", vehicle: lead.id, text: this.rng.pick(POLICE_LINES), mood: "calm" });
        }
        break;
      }
      case "stopped": {
        this.ticketCountdown -= dt;
        if (speed > 3) {
          // Drove off mid-stop.
          this.stage = "pursuit";
          this.pursuitTime = 0;
          rec.heat = Math.max(rec.heat, 2);
          ctx.events.emit({ type: "pullover", police: lead.id, stage: "fled" });
          this.onFlee(ctx, rec);
        } else if (this.ticketCountdown <= 0) {
          const t = rec.issueTicket();
          ctx.events.emit({ type: "ticket", vehicle: p.id, total: t.total, points: t.points, reason: t.count === 1 ? "1 violation" : `${t.count} violations` });
          ctx.events.emit({ type: "pullover", police: lead.id, stage: "issued" });
          rec.heat = 0;
          ctx.events.emit({ type: "heat", level: 0, reason: "ticket issued" });
          this.release(ctx);
        }
        break;
      }
      case "pursuit":
      case "searching": {
        this.pursuitTime += dt;
        const seer = this.seen(ctx, p, 150);
        if (seer !== undefined) {
          this.lostTime = 0;
          if (this.stage === "searching") this.stage = "pursuit";
        } else {
          this.lostTime += dt;
          if (this.lostTime > 10) this.stage = "searching";
        }
        // Heat rises the longer the chase lasts.
        const want = Math.min(5, 2 + Math.floor(this.pursuitTime / 45));
        if (want > rec.heat) {
          rec.heat = want;
          ctx.events.emit({ type: "heat", level: want, reason: "pursuit continues" });
        }
        if (this.units.length < Math.min(4, Math.floor(rec.heat)) && this.rng.chance(dt * 0.2)) this.addUnit(ctx);
        // Arrest: stopped with an officer right there.
        if (speed < 0.5 && dist < 16) this.stillTime += dt;
        else this.stillTime = 0;
        if (this.stillTime > 4) {
          const t = rec.issueTicket();
          rec.money -= 1000;
          ctx.events.emit({ type: "ticket", vehicle: p.id, total: t.total + 1000, points: t.points, reason: "arrested: fines + impound fee" });
          ctx.events.emit({ type: "pullover", police: lead.id, stage: "issued" });
          rec.heat = 0;
          ctx.events.emit({ type: "heat", level: 0, reason: "arrested" });
          this.release(ctx);
        } else if (this.lostTime > 22) {
          ctx.events.emit({ type: "pullover", police: lead.id, stage: "escaped" });
          rec.heat = 0;
          ctx.events.emit({ type: "heat", level: 0, reason: "escaped" });
          this.release(ctx);
        }
        break;
      }
      default:
        break;
    }
    void leadDriver;
  }

  private onFlee(ctx: SimContext, rec: DrivingRecord): void {
    const p = ctx.player;
    if (p !== undefined) {
      const r = rec.add("evading", ctx.time, "police", p.x, p.y);
      ctx.events.emit({ type: "violation", vehicle: p.id, kind: r.kind, label: r.label, fine: r.fine, points: r.points, witnessed: "police", x: p.x, y: p.y });
    }
    for (const u of this.units) {
      const d: AIDriver | undefined = ctx.driverOf(u);
      if (d !== undefined && d.followTarget >= 0) d.followGap = 5;
    }
    this.addUnit(ctx);
  }
}
