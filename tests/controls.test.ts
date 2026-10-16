import test from "node:test";
import assert from "node:assert/strict";
import { ACTIONS, InputMapper } from "../src/core/player/actions";

test("every action has a unique id, a label and help text", () => {
  const ids = new Set<string>();
  for (const a of ACTIONS) {
    assert.ok(!ids.has(a.id), `duplicate ${a.id}`);
    ids.add(a.id);
    assert.ok(a.label.length > 0 && a.help.length > 0);
  }
  // No keyboard key drives two actions by default.
  const keys = new Map<string, string>();
  for (const a of ACTIONS) for (const k of a.keys) {
    assert.ok(!keys.has(k), `${k} bound to ${keys.get(k)} and ${a.id}`);
    keys.set(k, a.id);
  }
});

test("keyboard taps are never lost, holds are tracked, pedals ramp", () => {
  const m = new InputMapper();
  m.key("KeyQ", true);
  m.key("KeyQ", false); // tap between frames
  let f = m.frame(1 / 60, 0);
  assert.ok(f.pressed.includes("indicator_left"));
  m.key("KeyW", true);
  f = m.frame(1 / 60, 0);
  assert.ok(f.throttle > 0 && f.throttle < 0.2, "throttle ramps up");
  for (let i = 0; i < 30; i++) f = m.frame(1 / 60, 0);
  assert.equal(f.throttle, 1);
  assert.ok(f.held.includes("throttle"));
  m.key("KeyW", false);
  f = m.frame(1 / 60, 0);
  assert.ok(f.released.some((r) => r.id === "throttle" && r.held > 0.4));
});

test("rebinding moves a key and steering is softer at speed", () => {
  const m = new InputMapper();
  m.bindKeys("horn", ["KeyQ"]);
  assert.deepEqual(m.bindings.get("horn")?.keys, ["KeyQ"]);
  assert.ok(!(m.bindings.get("indicator_left")?.keys ?? []).includes("KeyQ"), "key removed from its old action");
  const slow = new InputMapper();
  const fast = new InputMapper();
  slow.key("KeyD", true);
  fast.key("KeyD", true);
  let a = 0;
  let b = 0;
  for (let i = 0; i < 120; i++) {
    a = slow.frame(1 / 60, 2).steer;
    b = fast.frame(1 / 60, 40).steer;
  }
  assert.ok(a > 0.9 && b < 0.6 && b > 0, `steer at 2 m/s ${a.toFixed(2)}, at 40 m/s ${b.toFixed(2)}`);
});

test("gamepad analog triggers and stick map to pedals and steering", () => {
  const m = new InputMapper();
  const buttons = new Array(17).fill(0);
  buttons[7] = 0.5; // RT
  m.gamepad(buttons, [-0.8, 0, 0, 0]);
  const f = m.frame(1 / 60, 0);
  assert.ok(Math.abs(f.throttle - 0.5) < 1e-6, "analog throttle passes through");
  assert.ok(f.steer < -0.1, "left stick steers left");
  buttons[0] = 1; // A = handbrake
  m.gamepad(buttons, [0, 0, 0, 0]);
  assert.equal(m.frame(1 / 60, 0).handbrake, 1);
});

import { World } from "../src/core/world/world";

test("manual gearbox without clutch assist grinds unless the clutch is pressed", () => {
  const w = new World({ seed: 3, traffic: 0, police: 0 });
  const pd = w.playerDriver!;
  const p = w.player!;
  pd.settings.clutchAssist = false;
  w.input.press("transmission_mode");
  w.step(1 / 60);
  assert.equal(p.pt.mode, "manual");
  // Neutral and a running engine (dropping into gear at a standstill would stall).
  p.pt.shiftTo(0);
  p.pt.running = true;
  p.pt.stalled = false;
  for (let i = 0; i < 30; i++) w.step(1 / 60);
  assert.ok(p.pt.running);
  const grinds: number[] = [];
  w.events.on("grind", () => grinds.push(1));
  const gear = p.pt.gear;
  w.input.press("shift_up");
  w.step(1 / 60);
  assert.equal(grinds.length, 1, "grinds without clutch");
  assert.equal(p.pt.gear, gear, "gear did not change");
  w.input.set("clutch", 1);
  for (let i = 0; i < 30; i++) w.step(1 / 60);
  w.input.press("shift_up");
  for (let i = 0; i < 40; i++) w.step(1 / 60);
  assert.equal(grinds.length, 1);
  assert.equal(p.pt.gear, gear + 1, "shifts with the clutch in");
});
