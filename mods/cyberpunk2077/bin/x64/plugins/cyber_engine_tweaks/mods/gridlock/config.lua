-- User settings (edit freely). Hotkeys are bound in CET's overlay: Bindings tab.
return {
  -- Posted speed limit used for speeding checks (km/h). Change in-game with the hotkeys.
  speedLimitKmh = 60,
  -- "fines": detected violations cost eddies; "record": only logged; "off": no law checks.
  enforcement = "fines",
  -- NPC drivers within this radius react to you (honks, tailgating, cut-offs, gestures).
  socialRadius = 45,
  -- Show the Gridlock HUD while driving.
  hud = true,
  -- Mirror Gridlock indicator state onto the car's blinker lights (best effort).
  driveGameBlinkers = true,
  -- Night hours where headlights are required (for the "no headlights" check).
  nightStart = 20,
  nightEnd = 6,
}
