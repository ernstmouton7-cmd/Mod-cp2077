import test from "node:test";
import assert from "node:assert/strict";
import { buildCity } from "../src/core/road/cityBuilder";
import { Router } from "../src/core/road/router";

test("city builds a connected network with sane geometry", () => {
  const city = buildCity({ seed: 7 });
  const net = city.network;
  assert.ok(net.nodes.length >= 30);
  assert.ok(net.lanes.length > 100);
  assert.ok(net.connectors.length > 100);
  for (const l of net.lanes) {
    assert.ok(l.length > 5, `lane ${l.id} too short: ${l.length}`);
    // Every lane must lead somewhere (no dead ends in the grid).
    assert.ok(l.outgoing.length > 0, `lane ${l.id} on ${l.road.name} has no exits`);
    assert.ok(l.turns.length > 0);
  }
  for (const c of net.connectors) {
    assert.ok(c.length > 0.5 && c.length < 80, `connector length ${c.length}`);
    assert.ok(c.speedLimit > 3);
  }
  // Crossing movements at a 4-way must have conflicts.
  const four = net.nodes.find((n) => n.roads.length === 4)!;
  const straight = four.connectors.filter((c) => c.turn === "straight");
  assert.ok(straight.some((c) => c.conflicts.length > 0));
  console.log(`nodes=${net.nodes.length} roads=${net.roads.length} lanes=${net.lanes.length} connectors=${net.connectors.length} buildings=${city.buildings.length}`);
});

test("lane-use and approaches are assigned", () => {
  const net = buildCity({ seed: 3 }).network;
  for (const n of net.nodes) {
    for (const ap of n.approaches) {
      assert.ok(ap.lanes.length > 0);
      if (n.control === "signal") assert.equal(ap.sign, "signal");
    }
  }
  const avenueLane = net.lanes.find((l) => l.road.kind === "avenue" && l.index === 0 && l.to.roads.length === 4)!;
  assert.deepEqual(avenueLane.turns, ["left"]);
});

test("router finds routes between arbitrary roads", () => {
  const net = buildCity({ seed: 11 }).network;
  const router = new Router(net);
  let found = 0;
  for (let i = 0; i < 40; i++) {
    const start = net.lanes[(i * 37) % net.lanes.length];
    const goal = net.lanes[(i * 91 + 13) % net.lanes.length];
    const r = router.route(start, goal.road, goal.forward);
    if (goal.road === start.road && goal.forward === start.forward) continue;
    assert.ok(r !== undefined, `no route from lane ${start.id} to road ${goal.road.id}`);
    // Consecutive steps must be connected.
    let node = start.to;
    let via = start.road;
    for (const step of r!) {
      assert.ok(router.canTurn(node, via, step.road, step.forward), "route contains impossible turn");
      node = step.to;
      via = step.road;
    }
    found++;
  }
  assert.ok(found > 30);
});
