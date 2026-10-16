-- Runs the TypeScriptToLua build of the core under plain Lua (LuaJIT / 5.1).
-- Usage: luajit tests/lua/smoke.lua build/lua/gridlock_core.lua
local path = arg[1] or "build/lua/gridlock_core.lua"
local core = dofile(path)
assert(core.VERSION, "bundle did not return the core module")

local failures = 0
local function check(cond, msg)
  if not cond then
    failures = failures + 1
    print("FAIL: " .. msg)
  else
    print("ok   " .. msg)
  end
end

local t0 = os.clock()
local world = core.createWorld({ seed = 42, traffic = 110, startHour = 17.5 })
check(#world.vehicles > 50, "world spawned " .. #world.vehicles .. " vehicles")
check(#world.net.lanes > 100, "network has " .. #world.net.lanes .. " lanes")
check(#world.signals > 5, "has " .. #world.signals .. " signal controllers")

-- Drive the player: throttle for 6 s, then brake.
local counts = {}
world.events:onAny(function(e) counts[e.type] = (counts[e.type] or 0) + 1 end)
local p = world.player
local x0, y0 = p.x, p.y
world.input:set("throttle", 1)
for i = 1, 60 * 6 do world:step(1 / 60) end
local moved = math.sqrt((p.x - x0) ^ 2 + (p.y - y0) ^ 2)
check(moved > 20, string.format("player drove %.1f m under throttle (speed %.1f km/h, gear %s)", moved, p.dyn.u * 3.6, world.playerDriver:gearText()))
world.input:set("throttle", 0)
world.input:set("brake", 1)
local minSpeed = math.huge
for i = 1, 60 * 5 do
  world:step(1 / 60)
  minSpeed = math.min(minSpeed, math.abs(p.dyn.u))
end
check(minSpeed < 0.3, string.format("player braked to a stop (min %.2f m/s)", minSpeed))
-- Holding brake at a standstill engages reverse (automatic).
check(world.playerDriver:gearText() == "R", "holding brake at standstill selects reverse (gear " .. world.playerDriver:gearText() .. ")")
world.input:set("brake", 0)

-- Buttons: indicators, hazards, headlights, horn.
world.input:press("indicator_left")
world:step(1 / 60)
check(p.lights.indicator == "left", "left indicator toggles on")
world.input:press("hazards")
world:step(1 / 60)
check(p.lights.hazard == true, "hazards toggle on")
world.input:press("headlights")
world:step(1 / 60)
check(p.lights.headlightSwitch == "low", "headlight switch cycles to ON")

-- Traffic for two minutes.
for i = 1, 60 * 120 do world:step(1 / 60) end
local finite, movedCount = true, 0
for _, v in ipairs(world.vehicles) do
  if v.x ~= v.x or v.y ~= v.y then finite = false end
  if v.odometer > 100 then movedCount = movedCount + 1 end
end
check(finite, "all vehicle positions finite")
check(movedCount > #world.vehicles * 0.5, movedCount .. "/" .. #world.vehicles .. " vehicles moved")
check((counts.signal or 0) > 20, "signals changed " .. tostring(counts.signal or 0) .. " times")
local elapsed = os.clock() - t0
print(string.format("simulated 132 s in %.2f s CPU (%s), collisions=%d", elapsed, _VERSION .. (jit and (" " .. jit.version) or ""), world.stats.collisions))
local parts = {}
for k, v in pairs(counts) do parts[#parts + 1] = k .. "=" .. v end
table.sort(parts)
print("events: " .. table.concat(parts, " "))
if failures > 0 then
  print(failures .. " failure(s)")
  os.exit(1)
end
print("all Lua smoke checks passed")
