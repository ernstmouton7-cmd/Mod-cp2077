import test from "node:test";
import assert from "node:assert/strict";
import { idmAccel, idmFree, IdmParams } from "../src/core/ai/idm";
import { Emotion } from "../src/core/ai/emotion";
import { samplePersonality } from "../src/core/ai/personality";
import { Rng } from "../src/core/math/rng";
import { World } from "../src/core/world/world";
import { Connector } from "../src/core/road/network";
import { Segment } from "../src/core/road/types";

const P: IdmParams = { v0: 14, T: 1.4, a: 1.5, b: 2, s0: 2 };

test("IDM reaches desired speed and stops behind an obstacle without overlap", () => {
  let v = 0;
  for (let i = 0; i < 60 * 60; i++) v += idmFree(v, P) / 60;
  assert.ok(Math.abs(v - 14) < 0.3, `free speed ${v.toFixed(2)}`);
  let x = 0;
  v = 14;
  const wall = 200;
  let minGap = Infinity;
  for (let i = 0; i < 60 * 60; i++) {
    const gap = wall - x;
    v = Math.max(0, v + idmAccel(v, gap, v, P) / 60);
    x += v / 60;
    minGap = Math.min(minGap, wall - x);
  }
  assert.ok(minGap > 1.2 && minGap < 3, `standstill gap ${minGap.toFixed(2)} m`);
});

test("emotions escalate with offenses, decay with time, and apologies help", () => {
  const rng = new Rng(3);
  const hot = new Emotion(samplePersonality(rng, "aggressor"));
  const zen = new Emotion(samplePersonality(rng, "zen"));
  for (const e of [hot, zen]) {
    e.offend(9, "cut_off", 1.5, 0);
    e.offend(9, "long_honk", 1, 1);
    e.offend(9, "rude_gesture", 1, 2);
    for (let i = 0; i < 10; i++) e.update(0.1, 2 + i * 0.1, 0);
  }
  assert.ok(hot.anger > zen.anger * 3, `hot-head ${hot.anger.toFixed(0)} vs zen ${zen.anger.toFixed(0)}`);
  assert.equal(hot.mood, "furious");
  assert.equal(hot.target, 9);
  assert.ok(hot.canRage());
  const before = hot.anger;
  hot.apologize(9, 3.5);
  assert.ok(hot.anger < before);
  for (let t = 0; t < 120; t += 0.1) hot.update(0.1, 4 + t, 0);
  assert.equal(hot.mood, "calm", "anger fades over two minutes");
});

interface Stats {
  entries: number;
  redEntries: number;
  maxStopped: number;
  collisions: number;
  lcVehicles: number;
}

function cityRun(seconds: number, seed: number): { world: World; stats: Stats } {
  const world = new World({ seed, traffic: 110, noPlayer: true, startHour: 17 });
  world.env.timeScale = 0;
  const prevSeg = new Map<number, Segment | undefined>();
  const stoppedFor = new Map<number, number>();
  const lc = new Set<number>();
  const stats: Stats = { entries: 0, redEntries: 0, maxStopped: 0, collisions: 0, lcVehicles: 0 };
  world.events.on("collision", () => stats.collisions++);
  for (let i = 0; i < seconds * 60; i++) {
    world.step(1 / 60);
    for (const v of world.vehicles) {
      const d = world.driverOf(v);
      if (d === undefined) continue;
      if (d.shadowLane !== undefined) lc.add(v.id);
      const before = prevSeg.get(v.id);
      if (v.mode === "lane" && v.seg !== before && v.seg !== undefined && v.seg.kind === "connector" && before !== undefined && before.kind === "lane") {
        const c = v.seg as Connector;
        if (c.node.control === "signal") {
          stats.entries++;
          // Entering on red (beyond the grace of a just-changed light) counts as running it.
          const sig = world.signals[c.node.signalId];
          if (sig.indication(c) === "red" && sig.phaseState() !== "allred" && !(c.turn === "right")) stats.redEntries++;
        }
      }
      prevSeg.set(v.id, v.seg);
      const s = v.absSpeed() < 0.2 ? (stoppedFor.get(v.id) ?? 0) + 1 / 60 : 0;
      stoppedFor.set(v.id, s);
      if (s > stats.maxStopped) stats.maxStopped = s;
    }
  }
  stats.lcVehicles = lc.size;
  return { world, stats };
}

test("ten minutes of city traffic: lights obeyed, no gridlock, few crashes", () => {
  const { world, stats } = cityRun(600, 5);
  let odo = 0;
  for (const v of world.vehicles) odo += v.odometer;
  const vehHours = (world.vehicles.length * 600) / 3600;
  console.log(
    `entries=${stats.entries} redRuns=${stats.redEntries} (${((stats.redEntries / stats.entries) * 100).toFixed(2)}%) maxStopped=${stats.maxStopped.toFixed(0)}s collisions=${stats.collisions} (${(stats.collisions / vehHours).toFixed(2)}/veh-h) laneChangers=${stats.lcVehicles} avgOdo=${(odo / world.vehicles.length).toFixed(0)}m`,
  );
  assert.ok(stats.entries > 500, "lots of junction traffic");
  assert.ok(stats.redEntries / stats.entries < 0.03, "fewer than 3% enter on red");
  assert.ok(stats.maxStopped < 200, `longest standstill ${stats.maxStopped.toFixed(0)} s (gridlock?)`);
  assert.ok(stats.collisions / vehHours < 3, "crash rate stays low");
  assert.ok(stats.lcVehicles > 20, "drivers change lanes");
});
