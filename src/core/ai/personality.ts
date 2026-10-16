import { clamp } from "../math/scalar";
import { Rng } from "../math/rng";

export type Archetype =
  | "commuter"
  | "cautious"
  | "speeder"
  | "aggressor"
  | "distracted"
  | "professional"
  | "joyrider"
  | "zen";

/** Stable traits of a simulated driver (0..1 unless noted). */
export interface Personality {
  archetype: Archetype;
  label: string;
  aggression: number;
  /** How fast anger fades. */
  patience: number;
  /** Respect for signals, limits and signs. */
  lawfulness: number;
  /** Reaction skill; low = late, harsh reactions. */
  skill: number;
  /** Probability-ish of staying focused; low = phone at the wheel. */
  attentiveness: number;
  /** MOBIL politeness factor. */
  politeness: number;
  /** Desired speed relative to the limit (e.g. 1.1 = 10% over). */
  speedFactor: number;
  /** Desired time headway (s). */
  headway: number;
  /** Max comfortable acceleration multiplier. */
  accelFactor: number;
  /** Comfortable deceleration (m/s^2). */
  comfortDecel: number;
  /** Standstill gap (m). */
  minGap: number;
  /** Chance of signalling turns/lane changes. */
  signalDiscipline: number;
  /** Anger gain multiplier. */
  temper: number;
  /** Tendency to use the horn. */
  hornHappiness: number;
  /** Critical gap for yielding/turning across traffic (s). */
  criticalGap: number;
}

interface Template {
  label: string;
  weight: number;
  base: Omit<Personality, "archetype" | "label">;
}

const T: Record<Archetype, Template> = {
  commuter: {
    label: "Commuter", weight: 40,
    base: { aggression: 0.35, patience: 0.55, lawfulness: 0.75, skill: 0.6, attentiveness: 0.8, politeness: 0.4, speedFactor: 1.03, headway: 1.4, accelFactor: 1, comfortDecel: 2.2, minGap: 2.2, signalDiscipline: 0.85, temper: 1, hornHappiness: 0.35, criticalGap: 4.5 },
  },
  cautious: {
    label: "Cautious driver", weight: 14,
    base: { aggression: 0.1, patience: 0.85, lawfulness: 0.95, skill: 0.4, attentiveness: 0.85, politeness: 0.8, speedFactor: 0.9, headway: 2.0, accelFactor: 0.7, comfortDecel: 1.8, minGap: 3, signalDiscipline: 0.98, temper: 0.5, hornHappiness: 0.1, criticalGap: 6 },
  },
  speeder: {
    label: "Hurried courier", weight: 14,
    base: { aggression: 0.6, patience: 0.35, lawfulness: 0.45, skill: 0.7, attentiveness: 0.75, politeness: 0.15, speedFactor: 1.2, headway: 0.9, accelFactor: 1.3, comfortDecel: 3, minGap: 1.6, signalDiscipline: 0.5, temper: 1.3, hornHappiness: 0.6, criticalGap: 3.5 },
  },
  aggressor: {
    label: "Hot-head", weight: 8,
    base: { aggression: 0.9, patience: 0.12, lawfulness: 0.3, skill: 0.65, attentiveness: 0.7, politeness: 0.0, speedFactor: 1.25, headway: 0.7, accelFactor: 1.4, comfortDecel: 3.5, minGap: 1.4, signalDiscipline: 0.25, temper: 2.0, hornHappiness: 0.95, criticalGap: 2.8 },
  },
  distracted: {
    label: "Distracted driver", weight: 8,
    base: { aggression: 0.3, patience: 0.6, lawfulness: 0.7, skill: 0.35, attentiveness: 0.25, politeness: 0.4, speedFactor: 0.97, headway: 1.5, accelFactor: 0.9, comfortDecel: 2.2, minGap: 2.2, signalDiscipline: 0.55, temper: 0.9, hornHappiness: 0.2, criticalGap: 4.5 },
  },
  professional: {
    label: "Professional driver", weight: 8,
    base: { aggression: 0.4, patience: 0.65, lawfulness: 0.8, skill: 0.9, attentiveness: 0.9, politeness: 0.3, speedFactor: 1.05, headway: 1.2, accelFactor: 1.1, comfortDecel: 2.5, minGap: 2, signalDiscipline: 0.9, temper: 0.9, hornHappiness: 0.55, criticalGap: 3.8 },
  },
  joyrider: {
    label: "Joyrider", weight: 5,
    base: { aggression: 0.75, patience: 0.3, lawfulness: 0.2, skill: 0.55, attentiveness: 0.6, politeness: 0.05, speedFactor: 1.4, headway: 0.8, accelFactor: 1.5, comfortDecel: 3.5, minGap: 1.5, signalDiscipline: 0.2, temper: 1.5, hornHappiness: 0.8, criticalGap: 2.5 },
  },
  zen: {
    label: "Zen master", weight: 3,
    base: { aggression: 0.02, patience: 1.0, lawfulness: 0.9, skill: 0.8, attentiveness: 0.95, politeness: 1.0, speedFactor: 0.98, headway: 1.8, accelFactor: 0.9, comfortDecel: 2, minGap: 2.5, signalDiscipline: 1, temper: 0.2, hornHappiness: 0.02, criticalGap: 5 },
  },
};

export const ARCHETYPES: Archetype[] = ["commuter", "cautious", "speeder", "aggressor", "distracted", "professional", "joyrider", "zen"];

/** Sample a personality; `forced` picks an archetype explicitly. */
export function samplePersonality(rng: Rng, forced?: Archetype): Personality {
  const entries = ARCHETYPES.map((a) => ({ item: a, weight: T[a].weight }));
  const arch = forced ?? rng.weighted(entries);
  const t = T[arch];
  const j = (v: number, spread: number, lo: number, hi: number) => clamp(v + rng.gauss(0, spread), lo, hi);
  const b = t.base;
  return {
    archetype: arch,
    label: t.label,
    aggression: j(b.aggression, 0.08, 0, 1),
    patience: j(b.patience, 0.08, 0, 1),
    lawfulness: j(b.lawfulness, 0.07, 0, 1),
    skill: j(b.skill, 0.08, 0.1, 1),
    attentiveness: j(b.attentiveness, 0.06, 0.05, 1),
    politeness: j(b.politeness, 0.08, 0, 1),
    speedFactor: j(b.speedFactor, 0.04, 0.8, 1.55),
    headway: j(b.headway, 0.12, 0.5, 2.6),
    accelFactor: j(b.accelFactor, 0.08, 0.5, 1.6),
    comfortDecel: j(b.comfortDecel, 0.2, 1.4, 4),
    minGap: j(b.minGap, 0.2, 1.2, 3.5),
    signalDiscipline: j(b.signalDiscipline, 0.05, 0, 1),
    temper: j(b.temper, 0.1, 0.1, 2.5),
    hornHappiness: j(b.hornHappiness, 0.06, 0, 1),
    criticalGap: j(b.criticalGap, 0.3, 2, 7),
  };
}
