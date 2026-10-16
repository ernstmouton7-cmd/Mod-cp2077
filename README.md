# Gridlock

A traffic, signal, vehicle-dynamics and road-rage simulation for open-world games,
written once in TypeScript and compiled two ways:

- **JavaScript**, for a playable 3D city in the browser (`dist/gridlock.html`).
- **Lua** (5.1 / LuaJIT), as a single `gridlock_core.lua` for Lua game hosts.
  This repo ships it inside a **Cyberpunk 2077 Cyber Engine Tweaks mod**.

The simulation core has no engine dependencies. It runs at a fixed 60 Hz step,
is deterministic for a given seed, and exposes everything a renderer, an audio
system or a mod needs through plain objects and an event queue.

## Play it

Open `dist/gridlock.html` in a desktop browser (Chrome, Edge, Firefox, Safari). The
file is self-contained, so no server or install is needed. A gamepad and touch
screens also work.

| | |
|---|---|
| Drive | `W` `A` `S` `D` or arrows. Hold `S` at a standstill to reverse (automatic) |
| Indicators / hazards | `Q` / `E`, `X` (a hazard blink after merging says thanks) |
| Horn / flash | `H` (hold for a long blast), `J` |
| Lights / high beam / wipers | `L`, `K`, `P` |
| Gearbox | `T` auto/manual, `L-Shift`/`R` up, `L-Ctrl`/`F` down, `Z` clutch, `N` neutral |
| Cruise / limiter | `C` set or cancel, `=`/`-` adjust, `U` limiter |
| Ignition / seatbelt | `I`, `B` |
| Social | `G` wave sorry, `Y` rude gesture |
| Camera | `V` cycle, `M` look back, `,` `.` look left/right, mouse drag to orbit |
| Menus | `Esc` pause and settings, `F1` controls, `Tab` map, `F3` AI debug labels |

The [full control reference](docs/CONTROLS.md) has all 42 actions with their gamepad bindings.
Every key can be rebound in the pause menu.

## What is simulated

**Roads.** A procedural grid city with a perimeter boulevard, three-lane avenues
with medians and dedicated left lanes, one-way streets and small two-lane
streets. Every junction gets lane-use rules (left-only, straight, straight/right),
curved turn paths, and precomputed conflict zones between crossing and merging
movements. Routing uses A* with turn and signal delays.

**Signals.** Each signalised junction builds its own phase plan from its
geometry. Opposing approaches share a phase, and a dedicated left lane earns a
leading protected arrow (shown as a flashing yellow arrow when the left is
permissive). Controllers run actuated (skip empty phases, extend green while
detectors see traffic, gap-out, max-out), fixed-time with green-wave offsets,
flashing at night on minor junctions, or dark after a power cut. Yellow and
all-red times follow the approach speed. Pedestrian heads show walk, flashing
countdown and don't-walk. Emergency vehicles pre-empt the signal.

**Drivers.** Every AI car has a personality (eight archetypes, from zen master
to hot-head) that sets its desired speed, headway, acceleration, politeness,
signal discipline, temper and gap acceptance. Cars follow with the Intelligent
Driver Model and change lanes with a MOBIL-style gain test, signalling first if
they bother. At junctions they decide the yellow-light dilemma, stop fully at
stop signs, take turns at all-way stops by arrival order, yield on permissive
lefts while waiting in the box, turn right on red after stopping, and avoid
blocking the box (unless they don't care). They react late when distracted,
which occasionally causes rear-end crashes, and they pull over for sirens.

**Road rage.** Offenses (cut-offs, tailgating, honks, high beams, rude gestures,
brake checks, collisions, near misses, blocking a green light) add anger and a
grudge against the offender, scaled by temper and stress. Anger decays with
patience. Drivers escalate from irritated (short honks) through angry (long
honks, flashing, tailgating) to furious: tailgating you, brake-checking you, or
leaving their lane to chase and ram you. A wave or a hazard blink calms them,
and police nearby end the episode.

**Your car.** A planar rigid body with longitudinal weight transfer,
magic-formula tyres and a friction circle. Without ABS the wheels lock (and you
cannot steer), without traction control they spin, and stability control trims
yaw. The automatic gearbox creeps, parks, kicks down and reverses on the brake
pedal. The manual has a clutch and stalls if you lug it. Damage per body zone
weakens the engine, breaks headlights and can wreck the car. Fuel burns with
power.

**Law.** The game checks red lights (with red-light cameras), rolling stops,
right-on-red without stopping, speeding in three tiers (with speed cameras),
wrong-way driving, sidewalks, turns and lane changes without signalling, illegal
turns, headlights after dark, seatbelt, tailgating, reckless sliding, blocking
the box, horn abuse, failing to yield to emergency vehicles, collisions and
hit-and-run. Cameras fine you by mail. An officer who sees it pulls you over:
stop at the curb and wait for the ticket, or flee and start a pursuit with
rising heat, extra units and an arrest if you stop next to a squad car.

**World.** A day/night clock, weather (rain, storm, fog) that changes grip,
visibility and driver caution, rush-hour traffic density, and an ambulance
dispatched to serious crashes.

## Build and test

```bash
npm install
npm run check        # typecheck + lint + JS tests + Lua build + Lua tests
npm run build:web    # dist/gridlock.html
npm run build:lua    # build/lua/gridlock_core.lua (+ copy into the CET mod)
node scripts/gen-docs.mjs   # refresh generated tables in the docs
```

The Lua tests need `luajit` and/or `lua5.1` on the PATH.

## Layout

```
src/core/        engine-agnostic simulation (TypeScript, Lua-portable)
  math/ util/    scalar helpers, deterministic RNG, arc-length paths, events, spatial hash
  road/          network, procedural city, junction controls, router
  signals/       signal controller
  vehicle/       specs, powertrain, dynamics, lights and equipment, damage
  physics/       box collisions and impulse response
  ai/            driver brain, IDM, junction logic, personalities, emotions, speech
  law/           violations, driving record, player monitor, police
  player/        actions and bindings, player driver
  world/         world loop, environment, spawner, events
src/sandbox/     three.js renderer, HUD, audio, input for the browser build
mods/cyberpunk2077/  Cyber Engine Tweaks mod (Lua) using the compiled core
tests/           node tests, Lua smoke test, CET mock harness
docs/            architecture, systems reference, controls
```

## Cyberpunk 2077 mod

See [mods/cyberpunk2077/README.md](mods/cyberpunk2077/README.md). Copy the `bin`
folder into your game directory (with Cyber Engine Tweaks installed). The mod
runs the core's indicator/hazard logic, NPC driver emotions, and a driving
record with fines inside the game. Its game bindings are isolated in one file
and haven't been tested in-game yet; see the mod README for what is verified.

## Documentation

- [Architecture](docs/ARCHITECTURE.md): how the core is structured, the step order, and the rules that keep it portable to Lua.
- [Systems reference](docs/SYSTEMS.md): signals, driver models, emotions, violations and vehicles in detail, with the tables generated from code.
- [Controls](docs/CONTROLS.md): every action with its keyboard and gamepad binding.
