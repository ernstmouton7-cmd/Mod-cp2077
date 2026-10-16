# Systems reference

Tables marked *generated* come from the source (`node scripts/gen-docs.mjs`).

## Traffic signals (`signals/signalController.ts`)

**Phase plan.** Approaches whose headings are opposite (within 35°) form a
group. Each group gets a *through* phase serving its straight and right
movements, plus left turns as **permissive** (yield to oncoming) when the
group has two approaches, or **protected** when it has one. If any approach in
the group has a dedicated left lane, the group also gets a leading
**protected-left** phase. The widest group is the main road: it rests in green
when nothing is waiting (recall) and flashes yellow at night.

**Timing.**

| Interval | Rule |
|---|---|
| Yellow | `clamp(1 + v / (2 · 3.05), 3, 5.5)` s, with `v` the approach speed limit (perception-reaction time plus stopping at 3.05 m/s²) |
| All-red | `clamp((junction width + 5 m) / v, 1, 2.5)` s |
| Min green | 10 s main road, 6 s side road, 4 s left arrow |
| Max green | 40 / 24 / 12 s (actuated) |
| Fixed green | 26 / 16 / 8 s (fixed-time) |
| Passage (gap-out) | 3 s through, 1.5 s left arrow |
| Pedestrian walk | 7 s, then flashing don't-walk with a countdown until the green ends |

**Modes.** `actuated` (detectors from lane occupancy: phases without demand
are skipped, green extends until a gap or max green), `fixed` (constant
cycle; coordinated offsets make a green wave), `flashing` (main road flashing
yellow, side roads flashing red; minor junctions 01:00–05:00), `off` (dark:
drivers treat it as an all-way stop). An emergency vehicle within 120 m with
its siren on requests **pre-emption**: the current phase ends early, then the
phase serving that approach is chosen next.

**What a movement sees.** `green`, `permissive`, `yellow`, `red` (right turn
on red allowed after a full stop at 85% of signals), `flash_yellow`,
`flash_red`, `dark`.

## Junction rules for AI (`ai/intersection.ts`)

| Control | Behaviour |
|---|---|
| Green | Go, but never into a conflict zone a committed vehicle hasn't cleared (red-light runners, the player). |
| Permissive left / flashing yellow | Yield to priority movements using gap acceptance. A left-turner may pull into the box and wait at the conflict point, one car at a time. |
| Yellow | Stop if the required deceleration is below the driver's threshold (4.6 m/s² for cautious drivers down to 2.0 m/s² for aggressive ones). The decision is latched for that approach. |
| Red | Stop. Right on red: stop fully, then yield. Lawless or furious drivers may decide once to run it. |
| Stop sign (two-way) | Full stop (or a rolling stop for lawless drivers), then yield to the major road. |
| All-way stop / flashing red / dark | Full stop, register arrival, go in arrival order among conflicting approaches. |
| Yield / uncontrolled | Gap acceptance against priority movements (major road > minor; straight > right > left; then traffic from the right). |
| Exit blocked | Wait at the line rather than block the box (unless the driver doesn't care today). |

Gap acceptance waits while a priority vehicle reaches the conflict zone sooner
than `max(criticalGap × weather × rage, timeToClear + 1.2 s)`. The critical gap
ranges from 2.5 s (joyrider) to 7 s (cautious), grows in rain and at night, and
shrinks the longer the driver has waited.

## Car following and lane changes (`ai/driver.ts`, `ai/idm.ts`)

Acceleration is the minimum of the IDM term toward the leader, an IDM stop
toward the stop distance from the junction logic, and curve braking toward the
next connector's speed (`√(2.6 m/s² / curvature)`).

| IDM parameter | Source |
|---|---|
| Desired speed `v0` | lane limit × personality speed factor × mood (up to +20% furious) × weather (−15% wet, slower in fog) |
| Time headway `T` | personality headway × mood (down to 0.55× furious) × (1 + 0.3 × wetness); 0.3 s when tailgating a rage target |
| Max acceleration `a` | min(vehicle traction/power limit, 2.6 m/s²) × personality |
| Comfortable braking `b` | personality (1.4–4 m/s²) |
| Jam gap `s0` | personality (1.2–3.5 m) |

**Human factors.** A reaction delay when pulling away (0.45–1.4 s by skill)
creates a realistic queue discharge wave. Distracted drivers keep their last
pedal input for 1–3.5 s until a time-to-collision under 1.3 s startles them into
an emergency stop.

**Lane changes.** Every 0.8–1.8 s a driver evaluates the lanes on each side.
It makes mandatory changes to reach a lane with its next turn, and
discretionary changes when `aNew − aOld + politeness × followerΔ + bias` beats
a threshold (0.45 down to 0.1 m/s² by aggression). The bias favours keeping
right. It signals first (0.6–2.2 s, if it signals at all), then checks the new
follower's braking against a safety limit. Forcing the follower to brake harder
than 2.2 m/s² is a cut-off and angers them.

## Emotions and road rage (`ai/emotion.ts`)

Anger (0–100) rises with each offense by `base × scale × temper × (1 + 0.6 × stress)`,
and a grudge is kept per offender. Anger decays at
`(0.6 + 3.2 × patience) × (1.6 if the last offense is over 6 s old, else 0.6) × (1 − 0.4 × stress)` per second.
A wave or hazard blink from the offender removes up to 26 anger (less for aggressive
drivers) and makes the driver less sensitive to that offender for 20 s.

Moods use hysteresis: calm → irritated at 22 (back below 14), angry at 48 (back
below 38), furious at 74 (back below 60).

| Mood | Behaviour |
|---|---|
| Irritated | Shorter headway, +4% speed, short honks when blocked |
| Angry | Long honks and high-beam flashes at the target, +10% speed, tailgates more |
| Furious (aggression > 0.55) | Road rage toward the target within 70 m: **tailgate** (0.3 s headway, horn, flashing), **brake-check** when the target is close behind, **chase** (leaves its lane, physics pursuit), **ram** after tailgating a while (aggression > 0.82, anger > 88). Ends when anger drops below 35, the target is out of reach (over 120 m for 6 s), after 50 s, or when police are within 50 m. |

Offense weights (*generated*):

<!-- gen:offenses -->
| Offense | Base anger |
|---|---:|
| cut off | 18 |
| tailgating | 4 |
| honked at | 7 |
| long honk | 14 |
| high beams | 7 |
| blocked | 2.5 |
| collision | 38 |
| near miss | 14 |
| rude gesture | 28 |
| brake check | 22 |
| slow driver | 1.5 |
| long wait | 0.6 |
<!-- /gen -->

Personalities (*generated*, averages of the sampled templates):

<!-- gen:personalities -->
| Archetype | Aggression | Patience | Lawfulness | Speed vs limit | Headway (s) | Signals |
|---|---:|---:|---:|---:|---:|---:|
| Commuter | 0.33 | 0.55 | 0.74 | 1.03× | 1.42 | 86% |
| Cautious driver | 0.10 | 0.86 | 0.95 | 0.90× | 2.03 | 97% |
| Hurried courier | 0.59 | 0.36 | 0.46 | 1.20× | 0.95 | 51% |
| Hot-head | 0.88 | 0.12 | 0.31 | 1.24× | 0.69 | 24% |
| Distracted driver | 0.29 | 0.62 | 0.72 | 0.97× | 1.49 | 55% |
| Professional driver | 0.41 | 0.67 | 0.79 | 1.05× | 1.19 | 91% |
| Joyrider | 0.75 | 0.32 | 0.20 | 1.40× | 0.82 | 21% |
| Zen master | 0.03 | 0.97 | 0.90 | 0.97× | 1.80 | 98% |
<!-- /gen -->

## Pedestrians (`ai/pedestrians.ts`, `road/crosswalks.ts`)

Every controlled junction has a marked crosswalk across each road, 2.6 m inside
the junction box. Each connector records where its path crosses a crosswalk.
Pedestrians spawn at crosswalk ends (fewer at night and in the rain). They wait
a step back from the curb for the walk signal of the parallel traffic, or, at
stop-sign junctions, for a 3.5 s gap. They look before stepping off, pause for
a car body in their path, shout at close passes, and walk away along the
sidewalk once across. About 8% are reckless: they start on the flashing hand
and accept shorter gaps.

Drivers stop 1.6 m before a crosswalk on their movement while someone is on it
or about to walk into their path. This matters most for turning traffic, which
yields to people crossing with the green. A car whose front is already past its
stop point keeps clearing the crosswalk unless a person is right in front of
its bumper, which prevents car and pedestrian waiting for each other. The player
is charged for failing to yield (passing close to someone crossing) or hitting
a pedestrian, which also dispatches an ambulance.

## Incidents (`world/world.ts`, `ai/driver.ts`)

- **Breakdowns** (about every 80 s, near the player): a car pulls toward the
  curb with its hazards on and stays for 40–90 s. Drivers behind change lanes.
  On single-lane two-way streets they **overtake through the oncoming lane**
  when it's clear for 90 m, and abort if it stops being clear before they are
  committed. While overtaking, the car is registered in the oncoming lane, so
  oncoming drivers see it and brake.
- **Buses** stop mid-block on curb lanes for 7–14 s with the right indicator,
  then signal left to pull out.
- **Power cuts** (about every 5 min) turn a junction's signals dark for 80–140 s.
  Drivers treat it as an all-way stop.

Both can be turned off (`world.breakdownInterval = 0`, `world.outageInterval = 0`, or in the sandbox settings).

## Law and police (`law/`)

Violations (*generated*):

<!-- gen:violations -->
| Violation | Fine (€$) | Points | Heat if seen by police |
|---|---:|---:|---:|
| Ran a red light | 400 | 3 | 1 |
| Right on red without stopping | 150 | 1 | 1 |
| Failed to stop at stop sign | 200 | 2 | 1 |
| Speeding (10–25% over) | 120 | 1 | 1 |
| Speeding (25–50% over) | 350 | 3 | 1 |
| Excessive speed (50%+ over) | 900 | 6 | 2 |
| Driving the wrong way | 500 | 4 | 2 |
| Driving on the sidewalk | 250 | 2 | 1 |
| Careless driving (collision) | 300 | 3 | 1 |
| Leaving the scene of a collision | 1500 | 6 | 3 |
| Lane change without signalling | 60 | 0 | 1 |
| Turn without signalling | 80 | 1 | 1 |
| Illegal turn | 200 | 2 | 1 |
| No headlights after dark | 90 | 1 | 1 |
| Not wearing a seatbelt | 70 | 0 | 1 |
| Failure to yield to emergency vehicle | 450 | 3 | 2 |
| Following too closely | 120 | 2 | 1 |
| Reckless driving | 800 | 5 | 2 |
| Blocking the intersection | 150 | 1 | 1 |
| Unnecessary use of horn | 50 | 0 | 1 |
| Evading police | 2500 | 8 | 3 |
| Failure to yield to a pedestrian | 350 | 3 | 1 |
| Hit a pedestrian | 5000 | 8 | 3 |
<!-- /gen -->

**Witnesses.** A police unit within 110 m with line of sight (buildings block),
or a red-light or speed camera. Camera tickets are charged immediately; police
witnesses lead to a stop. Unseen violations are logged but cost nothing.

**Pull-over.** The nearest patrol car switches on its siren, drives on the
road network toward you, then follows you 11 m back. Stop (under 0.6 m/s for
2.5 s within 35 m) and the officer writes a ticket for all pending violations
after 6 s. Driving off or speeding away starts a **pursuit**: an evading-police
charge, heat 2+, units following at 5 m (or 1 m to ram at heat 3+), backup
units, and heat rising every 45 s. Break line of sight for 22 s to escape.
Stopping within 16 m of a unit for 4 s during a pursuit is an arrest:
all pending fines plus a €$1,000 impound fee. Points slowly expire with clean
driving (one per 2 km); 12 points suspends your license.

## Vehicles (`vehicle/`)

Vehicle classes (*generated*):

<!-- gen:vehicles -->
| Class | Model | Mass (kg) | Power (kW) | Drive | Top speed (km/h) |
|---|---|---:|---:|---|---:|
| compact | Makigai-style Compact | 1100 | 75 | FWD | 188 |
| sedan | Archer-style Sedan | 1550 | 140 | FWD | 238 |
| sports | Quadra-style Sports Coupe | 1380 | 360 | RWD | 345 |
| muscle | Thorton-style Muscle Car | 1650 | 320 | RWD | 302 |
| suv | Chevillon-style SUV | 2150 | 210 | AWD | 231 |
| van | Mahir-style Van | 2400 | 130 | RWD | 173 |
| truck | Kaukaz-style Box Truck | 7500 | 220 | RWD | 133 |
| bus | City Bus | 12000 | 230 | RWD | 122 |
| taxi | Delamain-style Cab | 1600 | 180 | AWD | 262 |
| police | Patrol Interceptor | 1800 | 300 | AWD | 303 |
| ambulance | Trauma Response Unit | 4200 | 240 | RWD | 174 |
<!-- /gen -->

**Dynamics.** A planar body (`x, y, yaw, u, v, r`) is sub-stepped at 120 Hz.
Axle loads come from static distribution plus longitudinal transfer
`m·ax·h/L`. Lateral tyre force is `−μFz·sin(C·atan(B·α))`. Longitudinal force
comes from drive torque, brakes and rolling resistance. If it exceeds `μFz`,
the axle either locks (brakes without ABS: sliding friction opposes the slip
velocity, so steering stops working), spins (drive without TCS: lateral grip
drops to 55%), or is capped (ABS, TCS). The friction circle gives the lateral
force whatever the longitudinal force leaves. ESC applies a corrective yaw
moment when the yaw rate strays from the steady-state target. Below 3 m/s the
model blends toward kinematic rolling, so parking is stable.

**Powertrain.** Torque curve × throttle, minus engine braking above idle. The
automatic has a torque converter (stall speed, torque multiplication, idle
creep), shift points that depend on throttle, and kickdown. The manual takes
clutch-pedal slip and stalls below about 55% of idle rpm with the clutch out.
A rev limiter cuts fuel, shifts cut torque for the shift time, fuel burns at
about 0.33 L/kWh, and engine health scales torque.

**Equipment.** Indicators with a 0.7 s relay and steering-column auto-cancel,
hazards, a rotary light switch (OFF/AUTO/ON), high beam and flash-to-pass,
brake and reverse lamps, horn with press duration, wipers (INT/LOW/HIGH, with a
sweeping arm position), siren (wail/yelp) and lightbar.
