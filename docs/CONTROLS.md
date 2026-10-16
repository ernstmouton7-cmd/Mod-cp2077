# Controls

Generated from `src/core/player/actions.ts` (run `node scripts/gen-docs.mjs`).
Keyboard codes are physical keys (they don't move with your layout). Gamepad
names use the standard Xbox-style mapping. Everything is rebindable in the
pause menu, and the CET mod registers its own hotkeys in Cyber Engine Tweaks.

<!-- gen:controls -->
### Driving

| Action | Keyboard | Gamepad | What it does |
|---|---|---|---|
| Throttle | `W` / `ArrowUp` | RT | Accelerate (in reverse with automatic: brake). |
| Brake / reverse | `S` / `ArrowDown` | LT | Brake. With automatic gearbox, hold at a standstill to reverse. |
| Steer left | `A` / `ArrowLeft` | L-stick left | Steer left. |
| Steer right | `D` / `ArrowRight` | L-stick right | Steer right. |
| Handbrake | `Space` | A | Locks the rear wheels: parking, hairpins and drifts. |

### Gearbox

| Action | Keyboard | Gamepad | What it does |
|---|---|---|---|
| Clutch | `Z` | LB | Manual gearbox: press to shift or launch without stalling. |
| Shift up | `ShiftLeft` / `R` | B | Next gear (manual). |
| Shift down | `ControlLeft` / `F` | X | Previous gear (manual). |
| Select reverse | `Backspace` | — | Select R directly. |
| Select neutral | `N` | — | Select N directly. |
| Auto / manual | `T` | — | Toggle automatic and manual transmission. |

### Signals & lights

| Action | Keyboard | Gamepad | What it does |
|---|---|---|---|
| Horn | `H` | LS click | Tap for a beep, hold for a long blast (other drivers notice). |
| Left indicator | `Q` | D-pad left | Toggle left turn signal (auto-cancels after the turn). |
| Right indicator | `E` | D-pad right | Toggle right turn signal (auto-cancels after the turn). |
| Hazard lights | `X` | D-pad down | All four indicators. A quick blink after merging says thanks. |
| Headlights | `L` | D-pad up | Cycle the light switch: OFF, AUTO, ON. |
| High beam | `K` | — | Toggle high beams (dazzles oncoming drivers). |
| Flash high beams | `J` | RB | Flash to pass / warn. Drivers ahead take it personally. |
| Wipers | `P` | — | Cycle wipers: OFF, INT, LOW, HIGH. |
| Siren | `O` | — | Emergency vehicles only: cycle siren. |

### Comfort & assists

| Action | Keyboard | Gamepad | What it does |
|---|---|---|---|
| Cruise control | `C` | — | Set / cancel cruise at the current speed. Brake also cancels. |
| Cruise +5 | `Equal` / `NumpadAdd` | — | Raise cruise speed by 5 km/h. |
| Cruise -5 | `Minus` / `NumpadSubtract` | — | Lower cruise speed by 5 km/h. |
| Speed limiter | `U` | — | Cap speed at the posted limit. |
| Engine start / stop | `I` | — | Turn the ignition. Needed after a stall. |
| Seatbelt | `B` | — | Buckle up. Police fine unbelted drivers. |
| ABS on/off | `1` | — | Anti-lock brakes. |
| Traction control on/off | `2` | — | Traction control (off = burnouts). |
| Stability control on/off | `3` | — | Electronic stability control. |

### Social

| Action | Keyboard | Gamepad | What it does |
|---|---|---|---|
| Wave / sorry | `G` | Y | Apologise to nearby drivers. Calms people you just upset. |
| Rude gesture | `Y` | — | Insult nearby drivers. Expect consequences. |

### Camera

| Action | Keyboard | Gamepad | What it does |
|---|---|---|---|
| Change camera | `V` | View | Chase, far chase, hood, cockpit, top-down, cinematic. |
| Look back | `M` | RS click | Hold to look behind. |
| Look left | `Comma` | — | Hold to look left. |
| Look right | `Period` | — | Hold to look right. |

### System

| Action | Keyboard | Gamepad | What it does |
|---|---|---|---|
| Reset vehicle | `Backquote` | — | Put the car back on the nearest road (repair costs money). |
| Pause / menu | `Escape` | Menu | Pause and open the menu. |
| Controls help | `F1` | — | Show all controls. |
| Map | `Tab` | — | Toggle the big map. |
| Debug overlay | `F3` | — | Show AI moods, intentions and signal phases. |
| Skip time | `BracketRight` | — | Advance the clock by one hour. |
| Change weather | `BracketLeft` | — | Cycle clear, rain, storm, fog. |
<!-- /gen -->

## Notes

- **Automatic gearbox.** The car starts in **P**. Press the throttle to select
  D, or hold the brake to select R. In R the brake key drives backwards and the
  throttle brakes. Turning the engine off at a standstill parks the car.
- **Manual gearbox.** With clutch assist on (default), the clutch slips itself
  when pulling away. Turn assist off in Settings to use the clutch yourself: let it
  out at too low rpm and the engine stalls (restart with the ignition key).
- **Indicators** cancel themselves once you have turned the wheel and let it
  return, like a real steering column.
- **Social keys** affect the drivers around you. A wave works on anyone holding a
  grudge within 45 m, and the hazard "thank you" works on drivers behind you.
