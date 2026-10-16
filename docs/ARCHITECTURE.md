# Architecture

## One core, two targets

`src/core` is plain TypeScript with no DOM, Node or engine APIs. It compiles to:

| Target | Tool | Output | Used by |
|---|---|---|---|
| JavaScript | esbuild | bundled into `dist/gridlock.html` | browser sandbox (`src/sandbox`) |
| Lua 5.1 / LuaJIT | TypeScriptToLua | `build/lua/gridlock_core.lua` (one file) | CET mod, any Lua host |

A Lua host loads the bundle with `require` or `dofile` and gets the module table:

```lua
local core = require("gridlock_core")
local world = core.createWorld({ seed = 7, traffic = 60 })
world.input:set("throttle", 1)
for i = 1, 600 do world:step(1 / 60) end
print(world.player.dyn.u * 3.6, "km/h")
```

Classes are created through factory functions (`createWorld`, `createLights`,
`createEmotion`, `createRecord`, `createRng`, `createNetwork`, `createSignal`)
because plain Lua cannot call a TypeScript constructor. Methods use `:`;
fields are plain table fields.

## The simulation step

`World.step(dt)` runs once per fixed step (default 1/60 s). Hosts call
`World.update(realDt)`, which accumulates real time and runs the fixed steps.
The browser sandbox steps the world itself so it can interpolate poses between
steps. Each step:

1. **Environment**: clock, weather, wetness and derived grip, visibility and traffic density.
2. **Signals**: night flashing switch-over, then every controller advances, reading
   detector demand from lane occupancy.
3. **Occupancy**: every vehicle registers on the segments it occupies (its lane or
   connector, its tail on the previous segment, its lane-change shadow). Free
   bodies (the player, crashed cars, chasers) are projected onto every lane and
   connector they overlap as *phantom* occupants, so AI drivers see and yield to them.
4. **Player**: the input mapper turns raw input into a control frame, and the
   player driver operates the car's equipment and pedals. Gestures, horn and
   flash affect nearby drivers.
5. **AI drivers**: lane-mode cars perceive (leader, junction, emergency vehicles,
   offenses), decide (IDM acceleration, stop distance, lane change, lights,
   horn) and move along their path. Physics-mode AI (crashed, recovering, chasing,
   police following) writes pedal and steering inputs.
6. **Physics**: powertrain and rigid-body dynamics for every physics-mode vehicle.
7. **Collisions**: broad phase (spatial hash), oriented-box contact, impulse
   response, damage, fault, offenses and events. Lane-mode cars that are hit hard
   switch to physics.
8. **Law**: the player monitor emits observations; witnesses (police in line of
   sight, cameras) decide what is charged; the police system runs pull-overs and pursuits.
9. **Ambulances and population**: dispatch, spawn and despawn.

Everything a renderer or mod reacts to is emitted as a typed `SimEvent`
(`world/events.ts`): horns, flashes, speech, offenses, mood changes, rage
start/end, collisions, signal changes, violations, tickets, camera flashes,
heat, pull-over stages, dispatches, gestures, stalls, shifts, spawns. Consumers
either subscribe (`events.on`) or drain the queue once per frame.

## Two motion modes

Most AI cars run in **lane mode**: a kinematic position (segment, arc length,
lateral offset) driven by the IDM acceleration. It's cheap and never leaves the
road. A car switches to **physics mode** (the same rigid body as the player's)
when it is hit, starts a road-rage chase, or is a police unit following you. It
then steers itself with pure pursuit until it can merge back into a lane
(`ai/driver.ts`, `recoverUpdate`).

## Road model

- `Road`: a centreline polyline between two `Node`s with lanes in each direction.
- `Lane`: an offset, trimmed polyline with an arc-length table (`math/path.ts`),
  adjacent lane links and allowed turns.
- `Connector`: a curved path through a junction from one lane to another,
  with its turn type, a curvature-based speed limit, conflict zones against
  every other connector at the node (crossing or merging), and divergence
  distances against connectors that share its start lane.
- `Approach`: the lanes entering a node from one road, with the sign or signal
  group and stop-line geometry.

Leader search walks the vehicle's planned path (lane → connector → lane…), and
also checks sibling connectors until they diverge and the lane being left during
a lane change.

## Keeping the core portable to Lua

TypeScriptToLua output is only as fast and correct as the TypeScript allows.
These rules are enforced by lint where possible (`eslint.config.js`):

| Rule | Why |
|---|---|
| No implicit number/string truthiness | `0` and `""` are truthy in Lua (`strict-boolean-expressions`). |
| No `%` operator; use `mod()` | JS `%` and Lua `%` disagree for negative operands. |
| No bitwise operators | Not available in Lua 5.1 without a library. |
| No `get`/`set` accessors | TSTL implements them with an `__index` function on the class, which slows every method call and field miss on that class, about 4× in profiles. |
| No closures created per step | LuaJIT can't compile `FNEW`/`UCLO` inside traces, so creating a closure in a hot function aborts the trace and the JIT thrashes. |
| No `for…in`, no `delete` on arrays, no `undefined` in arrays | Lua tables have no holes; `#` would be wrong. |
| Sort comparators must be total | Lua's `table.sort` is not stable and errors on inconsistent comparators. |
| Deterministic randomness only (`math/rng.ts`) | `Math.random` would break reproducibility. |

The generated bundle also enlarges the LuaJIT trace cache (`maxtrace`,
`maxmcode`). With the default sizes the simulation thrashed and ran 10× slower.

Performance at 110 vehicles: ≈0.3–0.8 ms per step in V8, ≈1.5 ms in LuaJIT,
≈6 ms in the plain Lua 5.1 interpreter.

## Determinism and parity

The same seed gives the same run within one runtime. LuaJIT and Lua 5.1 produce
identical results. JS and Lua agree closely but not bit-for-bit, because V8 and
libm round transcendental functions differently and traffic amplifies tiny
differences. `tests/parity.test.ts` mirrors `tests/lua/smoke.lua` so the two
builds can be compared.

## Rust port (`engine/crates/traffic`)

`gridlock-traffic` (lib `gridlock_traffic`) is a port of `src/core` to Rust for
the native engine. It keeps the same algorithms, constants, units (metres,
seconds, x east, y north, heading CCW from +x) and RNG (`math/rng.rs` is the
same Park–Miller generator), so a seed means the same city and traffic. The
ported test suite reproduces the TypeScript runs' printed metrics exactly
(network counts, the 10-minute city run, the smoke scenario, pedestrians,
incidents). It has no runtime dependencies.

| TypeScript | Rust |
|---|---|
| `road/`, `signals/`, `vehicle/`, `physics/`, `ai/`, `law/`, `player/`, `world/` | same module names (`road`, `signals`, `vehicle`, `physics`, `ai`, `law`, `player`, `world`) |
| object references between entities | typed indices: `NodeId`, `RoadId`, `LaneId`, `ConnectorId`, `ApproachId`, `SignalId`, `CrosswalkId`; `SegRef::{Lane, Connector}` for segments; stable `VehicleId` (never reused) |
| occupancy arrays on each segment | `Occupancy` (one `Vec<OccEntry>` per lane and connector) next to an immutable `Network` |
| `SimContext` (the `World` itself) | `world::Sim`: network, router, signals, vehicles, occupancy, events, RNG, environment, pedestrians, obstacles. `World` owns a `Sim` plus input, player driver, monitor, record, police and spawner, so systems borrow disjointly |
| `drivers: Map<id, AIDriver>` | each AI `Vehicle` owns its `AIDriver`; during its update the driver is taken out of the vehicle and gets `&mut Sim` |
| `EventBus` + `SimEvent` | `Event` enum queued in `EventQueue`; the host calls `World::drain_events()` each step |
| `SignalController.onChange` | `SignalController::update` returns `Option<SignalChange>` |

Offenses that reach a driver whose brain is running further up the call stack
(a horn reaction chain that comes back to it) are applied when its update
returns, not in the middle of it.

**Host-physics bridge.** `Vehicle::external` marks a host-simulated body: the
World does not integrate its dynamics or run 2D collision response for it, but
still runs its powertrain/gear logic and its AI. Each step the host calls
`set_external_pose(id, ExternalPose { x, y, heading, vx, vy, yaw_rate })` for
physics-mode external vehicles (lane-mode ones keep their kinematic lane pose,
which the host follows), reads `ai_controls(id)` (physics-mode AI) or
`player_controls()` as `DriveControls { steer, steer_angle, throttle, brake,
handbrake, gear_dir }`, and reports its contacts with `report_collision(a, b,
impulse, point)` (`b = None` for static geometry). Reported collisions get the
same consequences as internal ones: lane cars switch to physics mode, damage,
crash reactions, anger and grudges, `Collision`/`CrashStatic` events,
hit-and-run tracking and ambulances. `WorldOptions { external_physics: true }`
spawns every vehicle external and disables the internal 2D collision
detection.

Performance: about 0.09 ms per step for the default city (~90 vehicles) in a
release build (`cargo run -p gridlock-traffic --release --example bench`).
