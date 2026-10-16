import { clamp } from "../math/scalar";
import { Personality } from "./personality";

export type Mood = "calm" | "irritated" | "angry" | "furious";

export type OffenseKind =
  | "cut_off"
  | "tailgating"
  | "honked_at"
  | "long_honk"
  | "high_beams"
  | "blocked"
  | "collision"
  | "near_miss"
  | "rude_gesture"
  | "brake_check"
  | "slow_driver"
  | "long_wait";

/** Base anger per offense (before temper/stress scaling). */
export const OFFENSE_ANGER: Record<OffenseKind, number> = {
  cut_off: 18,
  tailgating: 4,
  honked_at: 7,
  long_honk: 14,
  high_beams: 7,
  blocked: 2.5,
  collision: 38,
  near_miss: 14,
  rude_gesture: 28,
  brake_check: 22,
  slow_driver: 1.5,
  long_wait: 0.6,
};

interface Grudge {
  id: number;
  amount: number;
  last: number;
}

/**
 * Anger model behind road rage. Offenses add anger (scaled by temper and
 * current stress) and a grudge against the offender; anger decays with
 * patience. Mood thresholds use hysteresis so drivers don't flicker.
 */
export class Emotion {
  anger = 0;
  /** 0..1 background stress (congestion, waiting, lateness). */
  stress = 0;
  mood: Mood = "calm";
  private grudges: Grudge[] = [];
  /** Offender currently hated the most (-1 = nobody). */
  target = -1;
  lastOffense = -100;
  lastOffender = -1;
  lastKind: OffenseKind | undefined = undefined;
  /** Accumulated apology credit (s) — recently forgiven offenders. */
  private forgiven: Grudge[] = [];

  constructor(readonly p: Personality) {}

  offend(offender: number, kind: OffenseKind, scale: number, now: number): number {
    // Forgiven recently: much less sensitive to the same person.
    let forgiveness = 1;
    for (const f of this.forgiven) if (f.id === offender && now - f.last < 20) forgiveness = 0.4;
    const gain = OFFENSE_ANGER[kind] * scale * this.p.temper * (1 + this.stress * 0.6) * forgiveness;
    this.anger = clamp(this.anger + gain, 0, 100);
    if (offender < 0) return gain; // situational (traffic, waiting): no grudge
    let g: Grudge | undefined = undefined;
    for (const e of this.grudges) if (e.id === offender) g = e;
    if (g === undefined) {
      g = { id: offender, amount: 0, last: now };
      this.grudges.push(g);
    }
    g.amount = Math.min(100, g.amount + gain);
    g.last = now;
    this.lastOffense = now;
    this.lastOffender = offender;
    this.lastKind = kind;
    this.pickTarget();
    return gain;
  }

  /** Offender apologised (wave / hazard "thank you"). Returns anger removed. */
  apologize(offender: number, now: number): number {
    let g: Grudge | undefined = undefined;
    for (const e of this.grudges) if (e.id === offender) g = e;
    if (g === undefined) return 0;
    const recent = now - g.last < 10;
    const base = recent ? 26 : 10;
    const calmed = base * (1.2 - this.p.aggression * 0.7);
    const before = this.anger;
    this.anger = clamp(this.anger - calmed, 0, 100);
    g.amount = Math.max(0, g.amount - calmed * 1.5);
    this.forgiven.push({ id: offender, amount: 0, last: now });
    if (this.forgiven.length > 6) this.forgiven.shift();
    this.pickTarget();
    return before - this.anger;
  }

  /** Mean gesture: certain drivers find it hilarious, most don't. */
  grudgeAgainst(id: number): number {
    for (const e of this.grudges) if (e.id === id) return e.amount;
    return 0;
  }

  private pickTarget(): void {
    let best = 15;
    this.target = -1;
    for (const g of this.grudges) {
      if (g.amount > best) {
        best = g.amount;
        this.target = g.id;
      }
    }
  }

  update(dt: number, now: number, congestion: number): void {
    // Stress follows congestion slowly.
    this.stress += (clamp(congestion, 0, 1) - this.stress) * Math.min(1, dt * 0.05);
    const sinceOffense = now - this.lastOffense;
    // Anger decays faster for patient drivers and once the offense is old.
    const decay = (0.6 + this.p.patience * 3.2) * (sinceOffense > 6 ? 1.6 : 0.6) * (1 - this.stress * 0.4);
    this.anger = Math.max(0, this.anger - decay * dt);
    for (let i = this.grudges.length - 1; i >= 0; i--) {
      const g = this.grudges[i];
      g.amount -= decay * 0.8 * dt;
      if (g.amount <= 0) this.grudges.splice(i, 1);
    }
    if (this.target >= 0) this.pickTarget();
    // Hysteresis thresholds.
    const a = this.anger;
    const m = this.mood;
    if (m === "calm") {
      if (a > 22) this.mood = "irritated";
    } else if (m === "irritated") {
      if (a > 48) this.mood = "angry";
      else if (a < 14) this.mood = "calm";
    } else if (m === "angry") {
      if (a > 74) this.mood = "furious";
      else if (a < 38) this.mood = "irritated";
    } else if (a < 60) this.mood = "angry";
  }

  /** Is this driver willing to escalate into physical road rage? */
  canRage(): boolean {
    return this.mood === "furious" && this.p.aggression > 0.55;
  }
}
