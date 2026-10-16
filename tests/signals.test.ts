import test from "node:test";
import assert from "node:assert/strict";
import { buildCity } from "../src/core/road/cityBuilder";
import { SignalController } from "../src/core/signals/signalController";
import { Connector, Node } from "../src/core/road/network";

const city = buildCity({ seed: 2077 });
const net = city.network;
const signalNodes = net.nodes.filter((n) => n.control === "signal");

function controllerFor(n: Node, mode: "fixed" | "actuated"): SignalController {
  return new SignalController(0, n, { mode });
}

test("no two crossing movements are ever green together", () => {
  for (const mode of ["fixed", "actuated"] as const) {
    for (const n of signalNodes) {
      const sig = controllerFor(n, mode);
      // Demand everywhere so actuated controllers cycle.
      for (let t = 0; t < 400; t += 0.1) {
        sig.update(0.1, () => 1);
        for (const c of n.connectors) {
          const ic = sig.indication(c);
          if (ic !== "green") continue;
          for (const z of c.conflicts) {
            if (z.kind !== "cross") continue;
            const io = sig.indication(z.other);
            assert.notEqual(io, "green", `node ${n.id}: ${c.turn} ${c.id} and ${z.other.turn} ${z.other.id} both protected green (${mode})`);
          }
        }
      }
    }
  }
});

test("phase sequence is green -> yellow -> all-red with speed-based yellow", () => {
  const n = signalNodes[0];
  const sig = controllerFor(n, "fixed");
  const seen: string[] = [];
  let last = "";
  let yellowStart = -1;
  const yellowDurations: number[] = [];
  for (let t = 0; t < 300; t += 0.05) {
    sig.update(0.05, undefined);
    const st = sig.phaseState();
    if (st !== last) {
      if (st === "yellow") yellowStart = t;
      if (last === "yellow") yellowDurations.push(t - yellowStart);
      seen.push(st);
      last = st;
    }
  }
  for (let i = 1; i < seen.length; i++) {
    const a = seen[i - 1];
    const b = seen[i];
    const ok = (a === "green" && b === "yellow") || (a === "yellow" && b === "allred") || (a === "allred" && b === "green");
    assert.ok(ok, `bad transition ${a} -> ${b}`);
  }
  for (const d of yellowDurations) assert.ok(d >= 2.9 && d <= 5.6, `yellow ${d.toFixed(2)} s out of range`);
});

test("protected left arrow exists where a dedicated left lane exists", () => {
  for (const n of signalNodes) {
    const sig = controllerFor(n, "fixed");
    const hasDedicated = n.approaches.some((a) => a.lanes.some((l) => l.turns.length === 1 && l.turns[0] === "left"));
    const hasArrow = sig.phases.some((p) => p.isLeftArrow);
    if (hasDedicated && n.approaches.length >= 4) assert.ok(hasArrow, `node ${n.id} should have a left arrow phase`);
  }
});

test("actuated signal rests on the main road and serves side-road demand", () => {
  const n = signalNodes.find((x) => x.approaches.length === 4) as Node;
  const sig = controllerFor(n, "actuated");
  for (let t = 0; t < 60; t += 0.1) sig.update(0.1, () => 0);
  assert.ok(sig.currentPhase().recall, "with no demand it rests on the main-road phase");
  assert.equal(sig.phaseState(), "green");
  // A side-road car arrives.
  const side = sig.phases.find((p) => !p.recall && !p.isLeftArrow);
  assert.ok(side !== undefined);
  const sideConnectors = new Set<number>();
  side.serves.forEach((_v, id) => sideConnectors.add(id));
  let served = -1;
  for (let t = 0; t < 120; t += 0.1) {
    sig.update(0.1, (c: Connector) => (sideConnectors.has(c.id) && served < 0 ? 1 : 0));
    if (served < 0 && sig.currentPhase() === side && sig.phaseState() === "green") served = t;
  }
  assert.ok(served >= 0 && served < 60, `side road served after ${served.toFixed(1)} s`);
});

test("emergency pre-emption switches to the requested approach quickly", () => {
  const n = signalNodes.find((x) => x.approaches.length === 4) as Node;
  const sig = controllerFor(n, "actuated");
  for (let t = 0; t < 20; t += 0.1) sig.update(0.1, () => 1);
  const target = n.approaches.find((a) => !sig.isGreenFor(a)) as (typeof n.approaches)[number];
  let t = 0;
  while (!sig.isGreenFor(target) && t < 30) {
    sig.requestPreempt(target);
    sig.update(0.1, () => 1);
    t += 0.1;
  }
  assert.ok(sig.isGreenFor(target), "pre-empted approach gets green");
  assert.ok(t < 14, `pre-emption took ${t.toFixed(1)} s`);
});

test("flashing and dark modes give the right indications", () => {
  const n = signalNodes[0];
  const sig = controllerFor(n, "fixed");
  sig.setMode("flashing");
  const inds = new Set(n.connectors.map((c) => sig.indication(c)));
  assert.ok([...inds].every((i) => i === "flash_yellow" || i === "flash_red"));
  assert.ok(inds.has("flash_yellow") && inds.has("flash_red"), "main road flashes yellow, side roads red");
  sig.setMode("off");
  assert.ok(n.connectors.every((c) => sig.indication(c) === "dark"));
  sig.setMode("actuated");
  assert.equal(sig.phaseState(), "allred", "restarts with an all-red clearance");
});

test("pedestrian head walks, then counts down, during its through phase", () => {
  const n = signalNodes[0];
  const sig = controllerFor(n, "fixed");
  const states = new Set<string>();
  let countdownSeen = false;
  for (let t = 0; t < 200; t += 0.1) {
    sig.update(0.1, undefined);
    for (const ap of n.approaches) {
      const h = sig.head(ap);
      states.add(h.ped);
      if (h.ped === "flash" && h.pedCountdown > 0) countdownSeen = true;
    }
  }
  assert.ok(states.has("walk") && states.has("flash") && states.has("dont"));
  assert.ok(countdownSeen);
});
