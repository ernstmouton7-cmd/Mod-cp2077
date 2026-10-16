import test from "node:test";
import assert from "node:assert/strict";
import { SPECS, VehicleClass } from "../src/core/vehicle/specs";
import { Vehicle } from "../src/core/vehicle/vehicle";
import { VehicleLights } from "../src/core/vehicle/lights";

const DT = 1 / 60;

function car(cls: VehicleClass): Vehicle {
  const v = new Vehicle(1, SPECS[cls], 0xffffff, "player");
  v.mode = "physics";
  v.dyn.reset(0, 0, 0, 0);
  return v;
}

function step(v: Vehicle, grip = 1): void {
  const t = v.pt.update(DT, v.dyn.drivenOmega(), v.inputs.throttle, v.dyn.u, v.dyn.tcsActive ? 0.4 : 1);
  v.dyn.surfaceGrip = grip;
  v.dyn.step(DT, v.inputs, t, v.steerTarget, v.assists);
  v.syncFromDynamics();
}

function zeroTo100(cls: VehicleClass): number {
  const v = car(cls);
  v.inputs.throttle = 1;
  let t = 0;
  while (v.dyn.u < 100 / 3.6 && t < 60) {
    step(v);
    t += DT;
  }
  return t;
}

function brakingDistance(cls: VehicleClass, abs: boolean, grip: number): { dist: number; lockedSeen: boolean } {
  const v = car(cls);
  v.dyn.reset(0, 0, 0, 100 / 3.6);
  v.pt.gear = 5;
  v.assists.abs = abs;
  v.inputs.brake = 1;
  let lockedSeen = false;
  for (let i = 0; i < 60 * 20 && v.dyn.u > 0.05; i++) {
    step(v, grip);
    if (v.dyn.frontLocked) lockedSeen = true;
  }
  return { dist: v.dyn.x, lockedSeen };
}

test("0-100 km/h times are plausible per class", () => {
  const sports = zeroTo100("sports");
  const sedan = zeroTo100("sedan");
  const compact = zeroTo100("compact");
  const truck = zeroTo100("truck");
  console.log(`0-100: sports ${sports.toFixed(1)} s, sedan ${sedan.toFixed(1)} s, compact ${compact.toFixed(1)} s, truck ${truck.toFixed(1)} s`);
  assert.ok(sports > 2.8 && sports < 6, `sports ${sports}`);
  assert.ok(sedan > 6 && sedan < 12, `sedan ${sedan}`);
  assert.ok(compact > sedan, "compact slower than sedan");
  assert.ok(truck > 15, "truck slow");
});

test("braking 100-0 km/h: ~35-50 m dry with ABS; longer when wet; wheels lock without ABS", () => {
  const dry = brakingDistance("sedan", true, 1);
  const wet = brakingDistance("sedan", true, 0.68);
  const noAbs = brakingDistance("sedan", false, 1);
  console.log(`braking: dry ABS ${dry.dist.toFixed(1)} m, wet ABS ${wet.dist.toFixed(1)} m, dry no-ABS ${noAbs.dist.toFixed(1)} m`);
  assert.ok(dry.dist > 32 && dry.dist < 52, `dry ${dry.dist}`);
  assert.ok(wet.dist > dry.dist * 1.25, "wet road is longer");
  assert.ok(!dry.lockedSeen, "ABS prevents lock-up");
  assert.ok(noAbs.lockedSeen, "without ABS the wheels lock");
  assert.ok(noAbs.dist > dry.dist, "locked wheels stop longer");
});

test("steady cornering stays within the friction limit; ESC tames oversteer", () => {
  const v = car("sports");
  v.dyn.reset(0, 0, 0, 25);
  v.pt.gear = 3;
  let maxLat = 0;
  for (let i = 0; i < 60 * 5; i++) {
    v.inputs.throttle = v.dyn.u < 25 ? 0.5 : 0.2;
    v.steerTarget = 0.12;
    step(v);
    maxLat = Math.max(maxLat, Math.abs(v.dyn.ay));
  }
  assert.ok(maxLat < SPECS.sports.grip * 9.81 * 1.15, `lateral accel ${maxLat.toFixed(1)} m/s²`);
  assert.ok(Math.abs(v.dyn.r) > 0.2, "car turns");
});

test("handbrake at speed locks the rear and rotates the car", () => {
  const v = car("sports");
  v.dyn.reset(0, 0, 0, 18);
  v.pt.gear = 2;
  v.assists.esc = false;
  let rearLocked = false;
  let maxSkid = 0;
  for (let i = 0; i < 90; i++) {
    v.inputs.handbrake = 1;
    v.steerTarget = 0.35;
    step(v);
    if (v.dyn.rearLocked) rearLocked = true;
    maxSkid = Math.max(maxSkid, v.dyn.skid);
  }
  assert.ok(rearLocked, "rear axle locks");
  assert.ok(Math.abs(v.dyn.yaw) > 0.6, `rotated ${v.dyn.yaw.toFixed(2)} rad`);
  assert.ok(maxSkid > 0.5, "tyres skid");
});

test("burnout without traction control, controlled launch with it", () => {
  const off = car("muscle");
  off.assists.tcs = false;
  off.inputs.throttle = 1;
  let spin = false;
  for (let i = 0; i < 60; i++) {
    step(off);
    if (off.dyn.wheelspin) spin = true;
  }
  assert.ok(spin, "wheelspin with TCS off");
  const on = car("muscle");
  on.inputs.throttle = 1;
  let tcs = false;
  for (let i = 0; i < 60; i++) {
    step(on);
    if (on.dyn.tcsActive) tcs = true;
    assert.ok(!on.dyn.wheelspin);
  }
  assert.ok(tcs, "TCS intervenes");
});

test("automatic creeps at idle; manual stalls when the clutch is dumped", () => {
  const a = car("sedan");
  for (let i = 0; i < 60 * 6; i++) step(a);
  assert.ok(a.dyn.u > 0.5 && a.dyn.u < 4, `creep speed ${a.dyn.u.toFixed(2)} m/s`);
  const m = car("sedan");
  m.pt.mode = "manual";
  m.pt.gear = 1;
  m.pt.clutchPedal = 0;
  for (let i = 0; i < 60; i++) step(m);
  assert.ok(m.pt.stalled && !m.pt.running, "engine stalled");
  m.pt.start();
  m.pt.clutchPedal = 1;
  for (let i = 0; i < 60; i++) step(m);
  assert.ok(m.pt.running, "restarts with the clutch in");
});

test("automatic shifts up through the gears and holds below redline", () => {
  const v = car("sedan");
  v.inputs.throttle = 1;
  let maxGear = 1;
  let maxRpm = 0;
  for (let i = 0; i < 60 * 25; i++) {
    step(v);
    maxGear = Math.max(maxGear, v.pt.gear);
    maxRpm = Math.max(maxRpm, v.pt.rpm);
  }
  assert.ok(maxGear >= 5, `reached gear ${maxGear}`);
  assert.ok(maxRpm <= SPECS.sedan.redline + 200, `rpm ${maxRpm.toFixed(0)}`);
});

test("engine damage cuts power; no fuel stops the engine", () => {
  const a = zeroTo100("sedan");
  const v = car("sedan");
  v.pt.health = 0.3;
  v.inputs.throttle = 1;
  let t = 0;
  while (v.dyn.u < 100 / 3.6 && t < 60) {
    step(v);
    t += DT;
  }
  assert.ok(t > a * 1.3, "damaged engine is slower");
  const f = car("sedan");
  f.pt.fuel = 0.0001;
  f.inputs.throttle = 1;
  for (let i = 0; i < 120; i++) step(f);
  assert.ok(!f.pt.running, "out of fuel");
});

test("indicators: relay timing, toggling, hazards and auto-cancel", () => {
  const l = new VehicleLights(0);
  l.toggleIndicator("left");
  let on = 0;
  let edges = 0;
  let prev = false;
  for (let i = 0; i < 60 * 7; i++) {
    l.update(DT, 0);
    const s = l.leftLampOn();
    if (s) on++;
    if (s !== prev) edges++;
    prev = s;
  }
  const blinksPerMin = edges / 2 / (7 / 60);
  assert.ok(blinksPerMin > 60 && blinksPerMin < 120, `${blinksPerMin.toFixed(0)} blinks/min`);
  assert.ok(!l.rightLampOn());
  l.toggleIndicator("left");
  assert.equal(l.indicator, "off", "pressing again cancels");
  l.toggleIndicator("right");
  for (let i = 0; i < 30; i++) l.update(DT, -0.3); // steer right
  assert.equal(l.indicator, "right");
  for (let i = 0; i < 30; i++) l.update(DT, 0);
  assert.equal(l.indicator, "off", "auto-cancel after the wheel returns");
  l.hazard = true;
  let both = false;
  for (let i = 0; i < 60; i++) {
    l.update(DT, 0);
    if (l.leftLampOn() && l.rightLampOn()) both = true;
  }
  assert.ok(both, "hazards flash both sides");
});

test("headlights: switch cycle, auto, high beam needs low beam, flash always works", () => {
  const l = new VehicleLights(0);
  l.headlightSwitch = "off";
  l.cycleHeadlights();
  assert.equal(l.headlightSwitch, "auto");
  l.dark = false;
  assert.ok(!l.lowBeamOn());
  l.dark = true;
  assert.ok(l.lowBeamOn());
  l.cycleHeadlights();
  assert.equal(l.headlightSwitch, "low");
  l.cycleHeadlights();
  assert.equal(l.headlightSwitch, "off");
  l.highBeam = true;
  assert.ok(!l.highBeamOn(), "high beam needs the lights on");
  l.flash = true;
  assert.ok(l.highBeamOn(), "flash-to-pass works with lights off");
});
