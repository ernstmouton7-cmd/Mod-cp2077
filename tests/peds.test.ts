import test from "node:test";
import assert from "node:assert/strict";
import { World } from "../src/core/world/world";

test("pedestrians cross with the walk signal and traffic yields to them", () => {
  const world = new World({ seed: 5, traffic: 110, noPlayer: true, startHour: 12 });
  world.env.timeScale = 0;
  let hits = 0;
  let shouts = 0;
  world.events.on("ped_hit", () => hits++);
  world.events.on("ped_speech", () => shouts++);
  let crossedOnDontWalk = 0;
  let maxPeds = 0;
  let crossings = 0;
  const started = new Set<number>();
  for (let i = 0; i < 60 * 300; i++) {
    world.step(1 / 60);
    maxPeds = Math.max(maxPeds, world.peds.peds.length);
    for (const p of world.peds.peds) {
      if (p.state === "crossing" && !started.has(p.id)) {
        started.add(p.id);
        crossings++;
        if (p.cw.node.control === "signal" && !world.peds.walkAllowed(world["pedHost" as keyof World] as never, p.cw, true) && p.waited < 120) crossedOnDontWalk++;
      }
    }
  }
  console.log(`peds max=${maxPeds} crossings=${crossings} startedOnDontWalk=${crossedOnDontWalk} hits=${hits} shouts=${shouts} collisions=${world.stats.collisions}`);
  assert.ok(maxPeds > 15, "pedestrians spawn");
  assert.ok(crossings > 100, "they cross");
  assert.equal(crossedOnDontWalk, 0, "nobody starts on a solid don't-walk");
  assert.ok(hits <= 1, `AI drivers hit ${hits} pedestrians`);
});
