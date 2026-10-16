import test from "node:test";
import assert from "node:assert/strict";
import { World } from "../src/core/world/world";
import { Lane } from "../src/core/road/network";

test("traffic overtakes a broken-down car on a single-lane street", () => {
  const world = new World({ seed: 8, traffic: 0, police: 0, noPlayer: true, startHour: 12 });
  world.env.timeScale = 0;
  world.breakdownInterval = 0;
  world.outageInterval = 0;
  world.pedestrianDensity = 0;
  world.spawner.enabled = false;
  const lane = world.net.lanes.filter((l: Lane) => l.road.kind === "street" && l.road.opts.lanesFwd === 1 && l.road.opts.lanesBack === 1 && l.length > 80).sort((a, b) => b.length - a.length)[0];
  assert.ok(lane !== undefined, "found a single-lane two-way street");
  const broken = world.spawner.create("sedan", "civilian", lane, lane.length * 0.55, 0);
  world.driverOf(broken)!.hold("breakdown", 120);
  const followers = [world.spawner.create("compact", "civilian", lane, 8, 8, "commuter"), world.spawner.create("suv", "civilian", lane, 18, 8, "speeder")];
  let overtook = 0;
  let passingSeen = false;
  const startOdo = followers.map((f) => f.odometer);
  for (let i = 0; i < 60 * 60; i++) {
    world.step(1 / 60);
    for (const f of followers) if (world.driverOf(f)?.passing !== undefined) passingSeen = true;
  }
  for (let k = 0; k < followers.length; k++) if (followers[k].odometer - startOdo[k] > lane.length) overtook++;
  assert.ok(broken.lights.hazard, "broken-down car shows hazards");
  assert.ok(broken.speed < 0.3, "broken-down car stays put");
  assert.ok(passingSeen, "followers used the oncoming lane");
  assert.equal(overtook, 2, "both followers got past");
  assert.equal(world.stats.collisions, 0);
});

test("a power cut turns a junction into an all-way stop without deadlock", () => {
  const world = new World({ seed: 5, traffic: 110, noPlayer: true, startHour: 17 });
  world.env.timeScale = 0;
  world.outageInterval = 0;
  // Busiest signal: most approach lanes.
  const sig = world.signals.slice().sort((a, b) => b.node.approaches.length - a.node.approaches.length)[0];
  for (let i = 0; i < 60 * 30; i++) world.step(1 / 60);
  sig.setMode("off");
  let through = 0;
  const prev = new Map<number, unknown>();
  let maxStopped = 0;
  const stopped = new Map<number, number>();
  for (let i = 0; i < 60 * 180; i++) {
    world.step(1 / 60);
    for (const v of world.vehicles) {
      const seg = v.seg;
      if (seg !== undefined && seg.kind === "connector" && prev.get(v.id) !== seg && (seg as unknown as { node: unknown }).node === sig.node) through++;
      prev.set(v.id, seg);
      const s = v.absSpeed() < 0.2 ? (stopped.get(v.id) ?? 0) + 1 / 60 : 0;
      stopped.set(v.id, s);
      maxStopped = Math.max(maxStopped, s);
    }
  }
  console.log(`dark junction: ${through} vehicles through in 3 min, longest wait ${maxStopped.toFixed(0)} s, collisions ${world.stats.collisions}`);
  assert.ok(through > 40, "traffic keeps flowing through the dark junction");
  assert.ok(maxStopped < 150, "no deadlock");
});
