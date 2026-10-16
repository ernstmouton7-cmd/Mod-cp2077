# Gridlock for Cyberpunk 2077 (Cyber Engine Tweaks)

Brings the Gridlock simulation core into Night City as a companion layer on top
of the game's own traffic:

- **Indicators and hazards** with the core's relay timing and steering-column
  auto-cancel (estimated from your yaw rate). The mod also drives the car's
  blinker lights, best effort.
- **NPC driver emotions.** Each car near you gets a Gridlock personality and
  temper. Tailgate it, cut it off, honk or flash at it, or flip it off, and the
  driver gets irritated, angry, then furious (its lines show in the HUD). A wave
  or a hazard "thank you" blink calms people down.
- **Driving record.** Speeding against a posted limit you set, an unbuckled
  seatbelt, reckless sliding, horn abuse and collisions cost eddies (taken from
  your real wallet) and license points. Enforcement can be set to fines,
  record only, or off.
- **HUD.** Speed against the limit, rpm and gear, indicator arrows, tell-tales,
  your record, the drivers angry at you, and a message log.

## Install

1. Install [Cyber Engine Tweaks](https://www.nexusmods.com/cyberpunk2077/mods/107).
2. Copy this folder's `bin` directory into your Cyberpunk 2077 install folder, so you end up with
   `Cyberpunk 2077/bin/x64/plugins/cyber_engine_tweaks/mods/gridlock/init.lua`.
3. Start the game, open the CET overlay, and bind the Gridlock hotkeys under **Bindings**.

| Hotkey | Suggested key | Action |
|---|---|---|
| Left / right indicator | `Q` / `E` (or `,` `.`) | Toggle a turn signal |
| Hazard lights | `X` | Hazards; a quick blink thanks drivers behind you |
| Horn | same key as the game's horn | Lets NPCs hear you honk (short tap vs long blast) |
| Flash high beams | `J` | Flash the car ahead |
| Wave sorry / Rude gesture | `G` / `Y` | Calm or provoke drivers near you |
| Seatbelt | `B` | Buckle up |
| Posted limit +10 / −10 | `PgUp` / `PgDn` | Set the limit for the road you're on |
| Toggle HUD | `F7` | Show or hide the driving HUD |

Settings (enforcement, HUD, NPC radius, posted limit) are in the **Gridlock**
window while the CET overlay is open, and their defaults are in `config.lua`.

## Files

| File | Role |
|---|---|
| `gridlock_core.lua` | The compiled simulation core. Generated, do not edit: run `npm run build:lua` from the repo root. |
| `init.lua` | CET entry point: events and hotkeys. |
| `companion.lua` | Game-side logic built on the core (lights, emotions, record). |
| `adapter.lua` | **Every** call into the game, each wrapped in `pcall`. |
| `hud.lua` | ImGui HUD and settings window. |
| `config.lua` | Defaults. |

## What has been verified

- The core runs under LuaJIT (the VM that CET uses) and Lua 5.1, and passes
  `tests/lua/smoke.lua`.
- The whole mod (`init.lua` → companion → HUD) runs in `tests/lua/cet_mock.lua`
  against stubbed CET, `Game` and `ImGui` APIs. The scripted drive checks
  fines, NPC anger from tailgating/honking/flashing, calming by wave, blinker
  control and indicator auto-cancel.
- **Not yet tested inside the game.** The bindings in `adapter.lua` follow
  commonly used CET/redscript APIs (`Game.GetMountedVehicle`,
  `GetCurrentSpeed`, the vehicle blackboard, `vehicleController.ToggleLights`,
  the targeting-system query, `TransactionSystem` with `MarketSystem.Money()`),
  but names differ between game and CET versions. If one fails, the mod keeps
  running without that feature and the settings window lists it under
  *Unavailable*. Fix the call in `adapter.lua`.

## Limits

The game's road network, traffic lights and NPC driving AI aren't exposed to
Lua, so the in-game layer can't read real signal states or make NPCs
physically tailgate or ram you. Red-light, stop-sign and wrong-way checks need
map data, so they run only in the standalone simulation (the browser sandbox),
where Gridlock owns the whole city.
