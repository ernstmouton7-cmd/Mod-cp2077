import { angleDiff, clamp, DEG, mod } from "../math/scalar";
import { Approach, Connector, Node } from "../road/network";

/** What a single movement (connector) is shown. */
export type Indication =
  | "green" //            protected: go
  | "permissive" //       green ball for a left turn: go but yield to oncoming
  | "yellow" //           stop if you safely can
  | "red" //              stop (right-on-red after a full stop where allowed)
  | "flash_yellow" //     proceed with caution
  | "flash_red" //        treat as stop sign
  | "dark"; //            power failure: treat as all-way stop

export type Lamp = "red" | "yellow" | "green" | "off";
export type PedLamp = "walk" | "flash" | "dont" | "off";

/** Display state of the signal heads facing one approach. */
export interface HeadState {
  ball: Lamp;
  /** Left-turn arrow head (undefined when the approach has no protected left). */
  arrow: Lamp | undefined;
  ballFlashing: boolean;
  /** Pedestrian signal for the crosswalk parallel to this approach. */
  ped: PedLamp;
  pedCountdown: number;
}

export type SignalMode = "fixed" | "actuated" | "flashing" | "off";

type PhaseState = "green" | "yellow" | "allred";

interface Phase {
  name: string;
  /** Movement rights: connector id -> protected/permissive. */
  serves: Map<number, "green" | "permissive">;
  approaches: Approach[];
  isLeftArrow: boolean;
  minGreen: number;
  maxGreen: number;
  /** Green duration used in fixed-time mode. */
  fixedGreen: number;
  yellow: number;
  allRed: number;
  /** Always served even without demand (main street recall). */
  recall: boolean;
}

/** Vehicles waiting for or approaching a movement; provided by the world. */
export type DemandFn = (connector: Connector) => number;

export interface SignalChange {
  nodeId: number;
  phase: string;
  state: PhaseState;
}

/**
 * Traffic signal controller for one intersection. Builds a phase plan from the
 * node geometry (opposing approaches share a phase, dedicated left lanes get a
 * leading protected arrow) and runs it fixed-time, actuated, flashing or dark.
 */
export class SignalController {
  readonly phases: Phase[] = [];
  mode: SignalMode;
  private phaseIndex = 0;
  private state: PhaseState = "green";
  private t = 0;
  /** Seconds the active green has been extended since the last detected vehicle. */
  private gapTimer = 0;
  /** Approach requesting emergency preemption (-1 = none) and time left on the request. */
  private preemptApproach = -1;
  private preemptTtl = 0;
  private flashClock = 0;
  readonly allowRightOnRed: boolean;
  /** Approaches considered the main road (flash yellow at night). */
  private mainApproaches: Approach[] = [];
  private readonly pedWalkTime = 7;
  onChange: ((c: SignalChange) => void) | undefined = undefined;
  /** Approach ids that have a protected left arrow head. */
  private readonly arrowApproaches: number[] = [];

  constructor(
    readonly id: number,
    readonly node: Node,
    opts: { mode?: SignalMode; offset?: number; allowRightOnRed?: boolean; cycleScale?: number },
  ) {
    this.mode = opts.mode ?? "actuated";
    this.allowRightOnRed = opts.allowRightOnRed ?? true;
    this.buildPhases(opts.cycleScale ?? 1);
    // Coordination offset: advance the fixed plan so neighbouring signals form a green wave.
    const offset = opts.offset ?? 0;
    if (offset > 0) {
      const saved = this.mode;
      this.mode = "fixed";
      let left = offset;
      while (left > 0) {
        const dt = Math.min(0.5, left);
        this.update(dt, undefined);
        left -= dt;
      }
      this.mode = saved;
    }
  }

  private buildPhases(cycleScale: number): void {
    const aps = this.node.approaches;
    // Group opposing approaches into axes.
    const used: boolean[] = [];
    for (let i = 0; i < aps.length; i++) used.push(false);
    const groups: Approach[][] = [];
    for (let i = 0; i < aps.length; i++) {
      if (used[i]) continue;
      const g: Approach[] = [aps[i]];
      used[i] = true;
      for (let j = i + 1; j < aps.length; j++) {
        if (used[j]) continue;
        if (Math.abs(Math.abs(angleDiff(aps[i].heading, aps[j].heading)) - Math.PI) < 35 * DEG) {
          g.push(aps[j]);
          used[j] = true;
          break;
        }
      }
      groups.push(g);
    }
    // The main axis is the widest one.
    let bestW = -1;
    for (const g of groups) {
      let w = 0;
      for (const a of g) w += a.lanes.length * (a.road.speedLimit + 1);
      if (w > bestW) {
        bestW = w;
        this.mainApproaches = g;
      }
    }
    let gi = 0;
    for (const g of groups) {
      g.forEach((a) => (a.group = gi));
      const isMain = g === this.mainApproaches;
      const speed = Math.max(...g.map((a) => a.road.speedLimit));
      const yellow = clamp(1 + speed / (2 * 3.05), 3, 5.5);
      const crossWidth = this.node.radius * 2;
      const allRed = clamp((crossWidth + 5) / Math.max(speed, 8), 1, 2.5);
      // Protected left phase when a dedicated left lane exists and something opposes it.
      const hasDedicatedLeft = g.some((a) => a.lanes.some((l) => l.turns.length === 1 && l.turns[0] === "left"));
      if (hasDedicatedLeft && g.length > 1) {
        const serves = new Map<number, "green" | "permissive">();
        for (const a of g) for (const l of a.lanes) for (const c of l.outgoing) if (c.turn === "left") serves.set(c.id, "green");
        this.phases.push({
          name: `G${gi}-left`,
          serves,
          approaches: g,
          isLeftArrow: true,
          minGreen: 4,
          maxGreen: 12 * cycleScale,
          fixedGreen: 8 * cycleScale,
          yellow: 3,
          allRed: 1,
          recall: false,
        });
      }
      const serves = new Map<number, "green" | "permissive">();
      for (const a of g) {
        const opposed = g.length > 1;
        for (const l of a.lanes) {
          for (const c of l.outgoing) {
            if (c.turn === "left" || c.turn === "uturn") serves.set(c.id, opposed ? "permissive" : "green");
            else serves.set(c.id, "green");
          }
        }
      }
      this.phases.push({
        name: `G${gi}-through`,
        serves,
        approaches: g,
        isLeftArrow: false,
        minGreen: isMain ? 10 : 6,
        maxGreen: (isMain ? 40 : 24) * cycleScale,
        fixedGreen: (isMain ? 26 : 16) * cycleScale,
        yellow,
        allRed,
        recall: isMain,
      });
      gi++;
    }
    for (const p of this.phases) if (p.isLeftArrow) for (const a of p.approaches) if (this.arrowApproaches.indexOf(a.id) < 0) this.arrowApproaches.push(a.id);
    // Start on the main through phase.
    for (let i = 0; i < this.phases.length; i++) {
      if (this.phases[i].recall) {
        this.phaseIndex = i;
        break;
      }
    }
  }

  currentPhase(): Phase {
    return this.phases[this.phaseIndex];
  }

  phaseState(): PhaseState {
    return this.state;
  }

  stateTime(): number {
    return this.t;
  }

  /** Cycle length of the fixed plan (seconds). */
  cycleLength(): number {
    let c = 0;
    for (const p of this.phases) c += p.fixedGreen + p.yellow + p.allRed;
    return c;
  }

  setMode(mode: SignalMode): void {
    if (mode === this.mode) return;
    this.mode = mode;
    // Coming back from flashing/dark: restart with an all-red clearance.
    if (mode === "fixed" || mode === "actuated") {
      this.state = "allred";
      this.t = 0;
    }
  }

  /** An emergency vehicle requests green for `approach` (call every step while needed). */
  requestPreempt(approach: Approach): void {
    this.preemptApproach = approach.id;
    this.preemptTtl = 1.0;
  }

  preempting(): boolean {
    return this.preemptApproach >= 0;
  }

  private phaseServesApproach(p: Phase, apId: number): boolean {
    if (p.isLeftArrow) return false;
    for (const a of p.approaches) if (a.id === apId) return true;
    return false;
  }

  private phaseDemand(p: Phase, demand: DemandFn | undefined): number {
    if (demand === undefined) return 1;
    let d = 0;
    for (const a of p.approaches) {
      for (const l of a.lanes) {
        for (const c of l.outgoing) {
          if (p.serves.has(c.id)) d += demand(c);
        }
      }
    }
    return d;
  }

  private emit(): void {
    if (this.onChange !== undefined) this.onChange({ nodeId: this.node.id, phase: this.currentPhase().name, state: this.state });
  }

  private pickNextPhase(demand: DemandFn | undefined): number {
    const n = this.phases.length;
    if (this.preemptApproach >= 0) {
      for (let k = 0; k < n; k++) if (this.phaseServesApproach(this.phases[k], this.preemptApproach)) return k;
    }
    if (this.mode === "fixed") return mod(this.phaseIndex + 1, n);
    for (let step = 1; step <= n; step++) {
      const k = mod(this.phaseIndex + step, n);
      const p = this.phases[k];
      if (this.phaseDemand(p, demand) > 0 || (p.recall && step === n)) return k;
    }
    // Nothing waiting: rest on the recall phase.
    for (let k = 0; k < n; k++) if (this.phases[k].recall) return k;
    return mod(this.phaseIndex + 1, n);
  }

  /** Should the current green end now? */
  private greenShouldEnd(demand: DemandFn | undefined, dt: number): boolean {
    const p = this.currentPhase();
    if (this.preemptApproach >= 0) {
      return !this.phaseServesApproach(p, this.preemptApproach) && this.t >= Math.min(p.minGreen, 4);
    }
    if (this.mode === "fixed") return this.t >= p.fixedGreen;
    if (this.t < p.minGreen) return false;
    if (this.t >= p.maxGreen) return this.othersWaiting(demand);
    // Gap-out: end when no vehicle has been detected for the passage time.
    const mine = this.phaseDemand(p, demand);
    if (mine > 0) this.gapTimer = 0;
    else this.gapTimer += dt;
    const passage = p.isLeftArrow ? 1.5 : 3.0;
    return this.gapTimer >= passage && this.othersWaiting(demand);
  }

  private othersWaiting(demand: DemandFn | undefined): boolean {
    for (let k = 0; k < this.phases.length; k++) {
      if (k !== this.phaseIndex && this.phaseDemand(this.phases[k], demand) > 0) return true;
    }
    return false;
  }

  update(dt: number, demand: DemandFn | undefined): void {
    this.flashClock += dt;
    if (this.preemptTtl > 0) {
      this.preemptTtl -= dt;
      if (this.preemptTtl <= 0) this.preemptApproach = -1;
    }
    if (this.mode === "flashing" || this.mode === "off") return;
    this.t += dt;
    const p = this.currentPhase();
    if (this.state === "green") {
      if (this.greenShouldEnd(demand, dt)) {
        this.state = "yellow";
        this.t = 0;
        this.emit();
      }
    } else if (this.state === "yellow") {
      if (this.t >= p.yellow) {
        this.state = "allred";
        this.t = 0;
        this.emit();
      }
    } else if (this.t >= p.allRed) {
      this.phaseIndex = this.pickNextPhase(demand);
      this.state = "green";
      this.t = 0;
      this.gapTimer = 0;
      this.emit();
    }
  }

  private flashOn(): boolean {
    return mod(this.flashClock, 1.0) < 0.5;
  }

  private isMain(a: Approach): boolean {
    return this.mainApproaches.indexOf(a) >= 0;
  }

  /** The indication a vehicle on connector `c` sees. */
  indication(c: Connector): Indication {
    if (this.mode === "off") return "dark";
    if (this.mode === "flashing") return this.isMain(c.approach) ? "flash_yellow" : "flash_red";
    const p = this.currentPhase();
    const right = p.serves.get(c.id);
    if (right === undefined || this.state === "allred") return "red";
    if (this.state === "yellow") return "yellow";
    return right;
  }

  /** Seconds until the indication for c may change (estimate; for HUD/AI anticipation). */
  timeInState(): number {
    return this.t;
  }

  /** Signal-head display for an approach. */
  head(a: Approach): HeadState {
    const out: HeadState = { ball: "red", arrow: undefined, ballFlashing: false, ped: "dont", pedCountdown: 0 };
    const hasArrow = this.arrowApproaches.indexOf(a.id) >= 0;
    if (hasArrow) out.arrow = "off";
    if (this.mode === "off") {
      out.ball = "off";
      out.ped = "off";
      if (hasArrow) out.arrow = "off";
      return out;
    }
    if (this.mode === "flashing") {
      out.ballFlashing = true;
      out.ball = this.flashOn() ? (this.isMain(a) ? "yellow" : "red") : "off";
      out.ped = "off";
      return out;
    }
    const p = this.currentPhase();
    const inPhase = p.approaches.indexOf(a) >= 0;
    const lamp: Lamp = this.state === "green" ? "green" : this.state === "yellow" ? "yellow" : "red";
    if (inPhase && this.state !== "allred") {
      if (p.isLeftArrow) {
        out.arrow = lamp;
        out.ball = "red";
      } else {
        out.ball = lamp;
        if (hasArrow) out.arrow = "off";
      }
    } else if (hasArrow) {
      out.arrow = "red";
    }
    // Pedestrians cross parallel to this approach while its through phase is green.
    if (inPhase && !p.isLeftArrow && this.state === "green") {
      const clearance = this.pedClearance();
      const greenLeft = this.mode === "fixed" ? p.fixedGreen - this.t : Math.max(p.minGreen, clearance + this.pedWalkTime) - this.t;
      if (this.t < this.pedWalkTime) out.ped = "walk";
      else if (greenLeft > 0) {
        out.ped = "flash";
        out.pedCountdown = Math.ceil(Math.max(0, greenLeft));
      }
    }
    return out;
  }

  private pedClearance(): number {
    return (this.node.radius * 2) / 1.2;
  }

  /** Is approach `a` part of the active phase and the phase in green? */
  isGreenFor(a: Approach): boolean {
    return this.state === "green" && this.currentPhase().approaches.indexOf(a) >= 0 && !this.currentPhase().isLeftArrow;
  }
}
