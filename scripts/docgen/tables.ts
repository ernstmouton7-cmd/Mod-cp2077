// Prints markdown tables generated from the source of truth (run via scripts/gen-docs.mjs).
import { ACTIONS, ActionCategory } from "../../src/core/player/actions";
import { VIOLATIONS, ViolationKind } from "../../src/core/law/violations";
import { OFFENSE_ANGER, OffenseKind } from "../../src/core/ai/emotion";
import { SPECS, VehicleClass, topSpeed } from "../../src/core/vehicle/specs";
import { ARCHETYPES, samplePersonality } from "../../src/core/ai/personality";
import { Rng } from "../../src/core/math/rng";

const PAD: Record<number, string> = { 0: "A", 1: "B", 2: "X", 3: "Y", 4: "LB", 5: "RB", 6: "LT", 7: "RT", 8: "View", 9: "Menu", 10: "LS click", 11: "RS click", 12: "D-pad up", 13: "D-pad down", 14: "D-pad left", 15: "D-pad right" };
const key = (c: string) => (c.startsWith("Key") ? c.slice(3) : c.startsWith("Digit") ? c.slice(5) : c);

function controls(): string {
  const cats: ActionCategory[] = ["Driving", "Gearbox", "Signals & lights", "Comfort & assists", "Social", "Camera", "System"];
  let out = "";
  for (const cat of cats) {
    out += `\n### ${cat}\n\n| Action | Keyboard | Gamepad | What it does |\n|---|---|---|---|\n`;
    for (const a of ACTIONS.filter((x) => x.category === cat)) {
      const pad = a.pad.map((p) => (p.button !== undefined ? PAD[p.button] : p.axis === 0 ? (p.dir === -1 ? "L-stick left" : "L-stick right") : "")).join(", ");
      out += `| ${a.label} | ${a.keys.map((k) => "`" + key(k) + "`").join(" / ") || "—"} | ${pad || "—"} | ${a.help} |\n`;
    }
  }
  return out;
}

function violations(): string {
  let out = "| Violation | Fine (€$) | Points | Heat if seen by police |\n|---|---:|---:|---:|\n";
  for (const k of Object.keys(VIOLATIONS) as ViolationKind[]) {
    const v = VIOLATIONS[k];
    out += `| ${v.label} | ${v.fine} | ${v.points} | ${v.heat} |\n`;
  }
  return out;
}

function offenses(): string {
  let out = "| Offense | Base anger |\n|---|---:|\n";
  for (const k of Object.keys(OFFENSE_ANGER) as OffenseKind[]) out += `| ${k.replace(/_/g, " ")} | ${OFFENSE_ANGER[k]} |\n`;
  return out;
}

function vehicles(): string {
  let out = "| Class | Model | Mass (kg) | Power (kW) | Drive | Top speed (km/h) |\n|---|---|---:|---:|---|---:|\n";
  for (const c of Object.keys(SPECS) as VehicleClass[]) {
    const s = SPECS[c];
    out += `| ${c} | ${s.name} | ${s.mass} | ${s.powerKw} | ${s.drive.toUpperCase()} | ${Math.round(topSpeed(s) * 3.6)} |\n`;
  }
  return out;
}

function personalities(): string {
  const rng = new Rng(1);
  let out = "| Archetype | Aggression | Patience | Lawfulness | Speed vs limit | Headway (s) | Signals |\n|---|---:|---:|---:|---:|---:|---:|\n";
  for (const a of ARCHETYPES) {
    // Average a few samples to show the template centre.
    let ag = 0, pa = 0, la = 0, sp = 0, hw = 0, si = 0;
    const n = 50;
    let label = "";
    for (let i = 0; i < n; i++) {
      const p = samplePersonality(rng, a);
      label = p.label;
      ag += p.aggression; pa += p.patience; la += p.lawfulness; sp += p.speedFactor; hw += p.headway; si += p.signalDiscipline;
    }
    const f = (v: number) => (v / n).toFixed(2);
    out += `| ${label} | ${f(ag)} | ${f(pa)} | ${f(la)} | ${f(sp)}× | ${f(hw)} | ${Math.round((si / n) * 100)}% |\n`;
  }
  return out;
}

const which = process.argv[2];
const map: Record<string, () => string> = { controls, violations, offenses, vehicles, personalities };
process.stdout.write(map[which]());
