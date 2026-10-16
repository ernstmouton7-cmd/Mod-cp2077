import test from "node:test";
import assert from "node:assert/strict";
import { World } from "../src/core/world/world";
import { Lane, Connector } from "../src/core/road/network";
import { SimEvent } from "../src/core/world/events";

const DT = 1 / 60;

function emptyWorld(seed = 9, police = 0): { world: World; events: SimEvent[] } {
  const world = new World({ seed, traffic: 0, police, startHour: 14 });
  world.env.timeScale = 0;
  world.env.autoWeather = false;
  const events: SimEvent[] = [];
  world.events.onAny((e) => events.push(e));
  return { world, events };
}

function placePlayer(world: World, lane: Lane, s: number, speed: number): void {
  const p = world.player!;
  const pt = { x: 0, y: 0, heading: 0 };
  lane.path.sample(s, pt);
  p.dyn.reset(pt.x, pt.y, pt.heading, speed);
  p.syncFromDynamics();
  world.playerDriver!.parked = false;
  p.pt.shiftTo(3);
  world.input.releaseAll();
}

function run(world: World, seconds: number, each?: () => void): void {
  for (let i = 0; i < seconds * 60; i++) {
    if (each !== undefined) each();
    world.step(DT);
  }
}

/** A long straight lane into a signalised junction, with a straight connector. */
function signalApproach(world: World, camera: boolean): { lane: Lane; c: Connector } {
  for (const n of world.net.nodes) {
    if (n.control !== "signal" || n.approaches.length < 4) continue;
    n.redLightCamera = camera;
    for (const ap of n.approaches) {
      for (const l of ap.lanes) {
        if (l.length < 70) continue;
        const c = l.outgoing.find((k) => k.turn === "straight");
        if (c !== undefined) return { lane: l, c };
      }
    }
  }
  throw new Error("no approach found");
}

test("running a red at a camera junction is photographed and fined", () => {
  const { world, events } = emptyWorld(9);
  const { lane, c } = signalApproach(world, true);
  const sig = world.signals[c.node.signalId];
  // Wait until our movement is red with plenty of red left.
  let guard = 0;
  while (!(sig.indication(c) === "red" && sig.phaseState() === "green") && guard++ < 60 * 300) world.step(DT);
  assert.equal(sig.indication(c), "red");
  placePlayer(world, lane, lane.length - 45, 14);
  world.input.set("throttle", 0.35);
  run(world, 7);
  const v = events.find((e) => e.type === "violation" && e.kind === "red_light");
  assert.ok(v !== undefined, "red light violation detected");
  assert.ok(v.type === "violation" && v.witnessed === "camera", "witnessed by the camera");
  assert.ok(events.some((e) => e.type === "camera_flash"), "camera flashed");
  assert.ok(world.record.points >= 3 && world.record.money < 5000, "fine and points applied");
});

test("police see you speeding, pull you over, and write a ticket when you stop", () => {
  const { world, events } = emptyWorld(4, 0);
  // A long avenue lane.
  const lane = world.net.lanes.filter((l) => l.road.kind === "avenue" && l.length > 80).sort((a, b) => b.length - a.length)[0];
  const cop = world.spawner.create("police", "police", lane, 6, 12);
  placePlayer(world, lane, 40, 30);
  world.input.set("throttle", 0.6);
  let stage = "";
  run(world, 6, () => (stage = world.police.stage));
  assert.ok(events.some((e) => e.type === "violation" && e.witnessed === "police"), "an officer witnessed a violation");
  // Keep driving until the unit asks us to pull over.
  for (let i = 0; i < 60 * 20 && world.police.stage !== "pullover"; i++) world.step(DT);
  assert.equal(world.police.stage, "pullover", `police stage ${world.police.stage}`);
  assert.ok(events.some((e) => e.type === "pullover" && e.stage === "request"));
  assert.ok(cop.sirenOn(), "siren on");
  // Pull over and stop.
  world.input.set("throttle", 0);
  world.input.set("brake", 1);
  const money = world.record.money;
  run(world, 1.5);
  world.input.set("brake", 0);
  world.playerDriver!.parked = true;
  for (let i = 0; i < 60 * 25 && !events.some((e) => e.type === "ticket"); i++) world.step(DT);
  const ticket = events.find((e) => e.type === "ticket");
  assert.ok(ticket !== undefined, "ticket issued");
  assert.ok(world.record.money < money, "fine paid");
  assert.equal(world.police.stage, "none", "officer leaves");
  assert.equal(world.record.heat, 0);
  void stage;
});

test("fleeing a pull-over starts a pursuit with heat and an evading charge", () => {
  const { world, events } = emptyWorld(4, 0);
  const lane = world.net.lanes.filter((l) => l.road.kind === "avenue" && l.length > 80).sort((a, b) => b.length - a.length)[0];
  world.spawner.create("police", "police", lane, 6, 12);
  placePlayer(world, lane, 40, 30);
  world.input.set("throttle", 0.6);
  for (let i = 0; i < 60 * 30 && world.police.stage !== "pullover"; i++) world.step(DT);
  assert.equal(world.police.stage, "pullover");
  // Escape: get well away from the unit (teleport down another road, keep moving).
  const far = world.net.lanes.filter((l) => l.length > 70).sort((a, b) => {
    const p = world.player!;
    const pa = a.path.points()[0];
    const pb = b.path.points()[0];
    return Math.hypot(pb.x - p.x, pb.y - p.y) - Math.hypot(pa.x - p.x, pa.y - p.y);
  })[0];
  for (let i = 0; i < 60 * 20 && (world.police.stage as string) !== "pursuit"; i++) {
    if (i % 60 === 0) placePlayer(world, far, 10, 16);
    world.input.set("throttle", 0.4);
    world.step(DT);
  }
  assert.equal(world.police.stage, "pursuit");
  assert.ok(world.record.heat >= 2, `heat ${world.record.heat}`);
  assert.ok(events.some((e) => e.type === "violation" && e.kind === "evading"));
});

test("tailgating, honking and gestures push a hot-head into road rage", () => {
  const { world, events } = emptyWorld(12, 0);
  const lane = world.net.lanes.filter((l) => l.road.kind === "avenue" && l.length > 80 && l.right === undefined).sort((a, b) => b.length - a.length)[0];
  const npc = world.spawner.create("sedan", "civilian", lane, 50, 12, "aggressor");
  const d = world.driverOf(npc)!;
  placePlayer(world, lane, 38, 12);
  // Follow 4 m behind, lean on the horn, flash, and flip them off.
  let t = 0;
  let rage = false;
  for (let i = 0; i < 60 * 40 && !rage; i++) {
    const p = world.player!;
    const gap = Math.hypot(npc.x - p.x, npc.y - p.y) - (npc.length + p.length) / 2;
    world.input.set("throttle", gap > 4 ? 0.5 : 0);
    world.input.set("brake", gap < 2.5 ? 0.6 : 0);
    if (i % 90 === 0) world.input.press("flash");
    if (i % 120 === 0) world.input.press("rude_gesture");
    world.input.set("horn", i % 100 < 70 ? 1 : 0);
    world.step(DT);
    t += DT;
    rage = events.some((e) => e.type === "rage_start" && e.target === p.id);
  }
  console.log(`rage after ${t.toFixed(1)} s, anger ${d.emo.anger.toFixed(0)}, mood ${d.emo.mood}`);
  assert.ok(events.some((e) => e.type === "offense" && e.victim === npc.id), "the driver was offended");
  assert.ok(rage, "road rage started against the player");
  // A sincere apology calms them down.
  world.input.set("horn", 0);
  const before = d.emo.anger;
  world.input.press("apologize");
  world.step(DT);
  assert.ok(d.emo.anger < before, "wave lowered anger");
});
