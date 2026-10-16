import test from "node:test";
import assert from "node:assert/strict";
import { World } from "../src/core/world/world";

function run(world: World, seconds: number): { ms: number } {
  const t0 = Date.now();
  const steps = Math.floor(seconds * 60);
  for (let i = 0; i < steps; i++) world.step(1 / 60);
  return { ms: Date.now() - t0 };
}

test("city traffic flows for 3 simulated minutes without blowing up", () => {
  const world = new World({ seed: 42, traffic: 110, startHour: 17.5 });
  const events: Record<string, number> = {};
  world.events.onAny((e) => (events[e.type] = (events[e.type] ?? 0) + 1));
  const { ms } = run(world, 180);
  let moving = 0;
  let physics = 0;
  let totalOdo = 0;
  const states: Record<string, number> = {};
  for (const v of world.vehicles) {
    assert.ok(Number.isFinite(v.x) && Number.isFinite(v.y), `vehicle ${v.id} has NaN position`);
    if (v.odometer > 100) moving++;
    if (v.mode === "physics" && !v.isPlayer) physics++;
    totalOdo += v.odometer;
    const d = world.driverOf(v);
    if (d) states[d.status()] = (states[d.status()] ?? 0) + 1;
  }
  const avgSpeed = totalOdo / world.vehicles.length / 180;
  console.log(`sim 180s in ${ms}ms; vehicles=${world.vehicles.length} moving=${moving} physicsAI=${physics} avgSpeed=${(avgSpeed * 3.6).toFixed(1)}km/h collisions=${world.stats.collisions}`);
  console.log("events", JSON.stringify(events));
  console.log("states", JSON.stringify(states));
  assert.ok(moving > world.vehicles.length * 0.6, `only ${moving} vehicles moved`);
  assert.ok((events["signal"] ?? 0) > 20, "signals should cycle");
});
