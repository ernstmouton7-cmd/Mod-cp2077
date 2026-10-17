# Gridlock for Unreal Engine 5

This plugin runs the whole Gridlock engine inside UE5: the city traffic simulation (AI drivers,
signals, junction rules, road rage, pedestrians, police and fines), full vehicle physics (tyres,
suspension, drivetrain, ABS/TCS/ESC), crash deformation with parts that come off, and breakable
street furniture. The Rust core is compiled to `gridlock_ffi.dll` and called through a C ABI
(`gridlock.h`); the plugin turns its output into procedural meshes, lights and a chase camera.

Status: written against UE 5.4/5.5 APIs but **not yet compiled in the editor** (it was built
without access to Unreal). Expect a few compile fixes on first build; the Rust side is tested.

## 1. Build the engine library (once, and after engine changes)

1. Install Rust: https://rustup.rs (default MSVC toolchain).
2. From the repository root run:

   ```powershell
   powershell -ExecutionPolicy Bypass -File ue5\build_ffi.ps1
   ```

   This runs `cargo build --release -p gridlock-ffi` in `engine/` and copies
   `gridlock_ffi.dll` and `gridlock_ffi.dll.lib` into
   `ue5/Gridlock/Source/ThirdParty/GridlockFFI/lib/Win64/`.

## 2. Add the plugin to a project

1. Create a UE5 C++ project (any template; "Blank" is fine).
2. Copy `ue5/Gridlock` into the project's `Plugins/` folder (so you have
   `<Project>/Plugins/Gridlock/Gridlock.uplugin`).
3. Right-click the `.uproject` → *Generate Visual Studio project files*, open the solution and
   build *Development Editor*. The plugin enables ProceduralMeshComponent and EnhancedInput.

## 3. Materials (5 minutes)

The plugin generates geometry; looks come from your materials. Create these and assign them on
the `GridlockWorld` actor (Details → Gridlock | Materials):

| Slot | Suggested setup |
|---|---|
| Paint | Metallic 0.6, Roughness 0.25, **Clear Coat 1**, vector parameter `Color` → Base Color |
| Trim | Black plastic, Roughness 0.6 (also used for street-furniture stand-ins) |
| Glass | Translucent, Opacity 0.35, Roughness 0.05, Specular 1 |
| Lamp | Unlit or Default Lit; vector `Color` × scalar `Emissive` → Emissive Color |
| Tyre / Rim | Rubber (Roughness 0.9) / Metal (Metallic 1, Roughness 0.3) |
| Road | Asphalt texture (UVs are in metres), Roughness 0.8; add a puddle/wetness layer if you like |
| Sidewalk / Grass | Concrete tiles / grass (UVs in metres) |
| Building | Facade texture (UVs in metres), optional vertex colour tint |
| Marking White / Yellow | Unlit-ish paint, slightly emissive so lines read at night |

Poly Haven / ambientCG CC0 textures listed in `engine/assets/manifest.toml` work well here (run
`cargo run -p gridlock-assets --bin fetch-assets --release` in `engine/` to download them).

## 4. Play

1. Open an empty level (keep a Directional Light, Sky Atmosphere, Sky Light and Exponential
   Height Fog for lighting).
2. Drag **Gridlock World** from the Place Actors panel (search "Gridlock") into the level at
   the origin.
3. Press Play. The camera follows your car; traffic, signals and pedestrians run around you.

If cars or buildings look inside-out (you see the inside, outsides missing), tick
**Reverse Winding** on the actor. Unreal's front-face convention could not be verified without
the editor, so this toggle flips every generated triangle.

### Controls

Xbox pad: RT throttle, LT brake (hold at a stop to reverse), left stick steer, A handbrake,
B/X shift up/down, Y camera (hold = look back), LB/RB indicators, LB+RB hazards, LS click horn,
D-pad: headlights (hold = high beam), flash, wave/sorry, rude gesture. Hold **View** for the
second layer: engine, seatbelt, gearbox mode, siren, cruise, wipers, assists, reset.
Keyboard: WASD/arrows, Space handbrake, Shift/Ctrl shift, H horn, Q/E indicators, X hazards,
L lights, G wave, Y rude gesture, B seatbelt, C cruise, T gearbox mode.

Rumble is mixed from the car's telemetry (impacts, kerbs, rough surfaces, slides, ABS, limiter)
and sent through Unreal's force-feedback interface. Trigger rumble is available in the native
Rust app on Windows; in Unreal it needs the platform's trigger-haptics API (not wired yet).

## Blueprint API

- `Hud` (struct): speed, rpm, gear, limit, street, money, points, heat, indicators, assists.
  Bind a UMG widget to it.
- `OnGridlockEvent(Kind, A, B, Location, Text)`: crashes, horn, AI speech, violations, road
  rage, parts torn off, glass, props snapped, hydrant bursts. Use it for sounds, particles and
  toasts. Kinds are the `GL_EVENT_*` values in `gridlock.h`.
- `Action("indicator_left")` etc. to drive actions from UI; `SetHour(h)`;
  `GetPlayerTransform()`.

## Coordinates

The DLL is created with `GL_COORDS_UNREAL`: centimetres, X east, Y south, Z up, quaternions in
that frame. Car meshes come in the same frame, so a car's forward axis is **-Y** in its local
space.

## Next steps

- Swap the basic-shape stand-ins for real meshes: subclass `AGridlockWorld` in Blueprint or
  extend `SyncProps()` to pick a static mesh per prop kind (the CC0 models in
  `engine/assets/manifest.toml` include a hydrant, barrier, bench, bin and street lamp).
- Use `UInstancedStaticMeshComponent` for props and Niagara for spray/tyre smoke at scale.
