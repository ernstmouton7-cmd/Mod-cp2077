import test from "node:test";
import assert from "node:assert/strict";
import { createWorld } from "../src/core/index";

/** Mirrors tests/lua/smoke.lua so JS and Lua builds can be compared. */
test("JS build matches the Lua smoke scenario", () => {
  const world = createWorld({ seed: 42, traffic: 110, startHour: 17.5 });
  const counts: Record<string, number> = {};
  world.events.onAny((e) => (counts[e.type] = (counts[e.type] ?? 0) + 1));
  const p = world.player!;
  const x0 = p.x;
  const y0 = p.y;
  world.input.set("throttle", 1);
  for (let i = 0; i < 360; i++) world.step(1 / 60);
  const moved = Math.hypot(p.x - x0, p.y - y0);
  world.input.set("throttle", 0);
  world.input.set("brake", 1);
  for (let i = 0; i < 300; i++) world.step(1 / 60);
  world.input.set("brake", 0);
  world.input.press("indicator_left");
  world.step(1 / 60);
  world.input.press("hazards");
  world.step(1 / 60);
  world.input.press("headlights");
  world.step(1 / 60);
  for (let i = 0; i < 7200; i++) world.step(1 / 60);
  const parts = Object.keys(counts).sort().map((k) => `${k}=${counts[k]}`);
  console.log(`moved=${moved.toFixed(1)} collisions=${world.stats.collisions} events: ${parts.join(" ")}`);
  assert.ok(moved > 20);
});
