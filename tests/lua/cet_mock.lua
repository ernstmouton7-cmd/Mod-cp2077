-- Runs the Cyberpunk 2077 CET mod (mods/cyberpunk2077/.../gridlock) under plain
-- Lua with stubbed Game/ImGui/CET APIs, driving a scripted scenario. This checks
-- the mod's own logic and its use of the core; the real game bindings live in
-- adapter.lua and still need an in-game test.
local modDir = arg[1] or "mods/cyberpunk2077/bin/x64/plugins/cyber_engine_tweaks/mods/gridlock"
package.path = modDir .. "/?.lua;" .. package.path

local failures = 0
local function check(cond, msg)
  if cond then print("ok   " .. msg) else failures = failures + 1; print("FAIL: " .. msg) end
end

-- ---- CET globals -------------------------------------------------------------
local events, hotkeys, inputs = {}, {}, {}
function registerForEvent(name, fn) events[name] = fn end
function registerHotkey(id, label, fn) hotkeys[id] = fn end
function registerInput(id, label, fn) inputs[id] = fn end
function GetDisplayResolution() return 1920, 1080 end
function GetLocalizedText(s) return s end

-- ImGui: record calls, return plausible values.
local imguiCalls = 0
ImGui = setmetatable({}, { __index = function(_, k)
  return function(...)
    imguiCalls = imguiCalls + 1
    if k == "Begin" then return true end
    if k == "Checkbox" then local _, v = ...; return v, false end
    if k == "SliderInt" then local _, v = ...; return v, false end
    if k == "RadioButton" or k == "IsItemHovered" then return false end
    return nil
  end
end })
ImGuiWindowFlags = setmetatable({}, { __index = function() return 1 end })
ImGuiCol = setmetatable({}, { __index = function() return 1 end })
ImGuiCond = setmetatable({}, { __index = function() return 1 end })
vehicleELightType = setmetatable({}, { __index = function(_, k) return k end })

-- ---- A tiny fake world: the player's car and one NPC ahead ------------------
local function vec(x, y) return { x = x, y = y, z = 0, w = 1 } end
local function makeVehicle(id, x, y, hx, hy, speed)
  local v = { id = id, x = x, y = y, hx = hx, hy = hy, speed = speed, lights = {} }
  function v:GetWorldPosition() return vec(self.x, self.y) end
  function v:GetWorldForward() return vec(self.hx, self.hy) end
  function v:GetCurrentSpeed() return self.speed end
  function v:GetEntityID() return { hash = self.id } end
  function v:GetDisplayName() return "Test Car " .. self.id end
  function v:IsVehicle() return true end
  function v:GetBlackboard() return { GetFloat = function() return 3200 end, GetInt = function() return 3 end } end
  function v:GetVehicleComponent()
    local car = self
    return { GetVehicleController = function() return { ToggleLights = function(_, on, t) car.lights[t] = on end } end }
  end
  return v
end
local player = { GetWorldPosition = function() return vec(0, 0) end }
local car = makeVehicle(100, 0, 0, 1, 0, 0)
local npc = makeVehicle(200, 12, 0, 1, 0, 0)
local mounted = true
local money = 10000
local notifications = 0

Game = {
  GetPlayer = function() return player end,
  GetMountedVehicle = function() return mounted and car or nil end,
  GetTimeSystem = function() return { GetGameTime = function() return { Hours = function() return 22 end, Minutes = function() return 30 end } end } end,
  GetTransactionSystem = function()
    return {
      RemoveItem = function(_, _, _, amount) money = money - amount end,
      GiveItem = function(_, _, _, amount) money = money + amount end,
      GetItemQuantity = function() return money end,
    }
  end,
  GetTargetingSystem = function()
    return { GetTargetParts = function()
      local part = {}
      function part:GetComponent() return { GetEntity = function() return npc end } end
      return true, { part }
    end }
  end,
  GetBlackboardSystem = function() return { Get = function() return { SetVariant = function() notifications = notifications + 1 end } end } end,
}
Game["TSQ_ALL;"] = function() return {} end
MarketSystem = { Money = function() return "money" end }
SimpleScreenMessage = { new = function() return {} end }
GetAllBlackboardDefs = function() return { Vehicle = {}, UI_Notifications = {} } end
ToVariant = function(x) return x end

-- ---- Load the mod -----------------------------------------------------------
local mod = dofile(modDir .. "/init.lua")
check(events.onInit ~= nil and events.onUpdate ~= nil and events.onDraw ~= nil, "mod registered CET events")
check(hotkeys.gridlock_indicator_left ~= nil and inputs.gridlock_horn ~= nil, "mod registered hotkeys and inputs")
events.onInit()
check(mod.companion ~= nil and mod.error == nil, "companion created" .. (mod.error and (": " .. mod.error) or ""))
local c = mod.companion

local dt = 1 / 60
local function frames(n, fn)
  for _ = 1, n do
    if fn then fn() end
    events.onUpdate(dt)
    events.onDraw()
  end
end

-- 1) Speeding at 100 km/h in the default 60 zone: fined.
car.speed = 100 / 3.6
npc.x = 1e6
frames(60 * 5, function() car.x = car.x + car.speed * dt end)
check(c.record.points > 0, "speeding recorded (" .. c.record.points .. " points)")
check(money < 10000, "fine charged to the game wallet (E$" .. (10000 - money) .. ")")

-- 2) Tailgating an NPC at 60 km/h, 3 m behind its bumper: it gets angry.
car.speed = 60 / 3.6
npc.speed = 60 / 3.6
npc.x, npc.y = car.x + 7.5, 0
c.cfg.enforcement = "off"
frames(60 * 8, function()
  car.x = car.x + car.speed * dt
  npc.x = npc.x + npc.speed * dt
end)
local tracked = c.npcs["200"]
check(tracked ~= nil, "NPC tracked via targeting query")
check(tracked ~= nil and tracked.emo.anger > 20, "tailgated NPC is angry (" .. (tracked and string.format("%.0f", tracked.emo.anger) or "?") .. ")")
local angerBefore = tracked.emo.anger

-- 3) Honk and flash at it.
inputs.gridlock_horn(true)
frames(70, function() car.x = car.x + car.speed * dt; npc.x = npc.x + npc.speed * dt end)
inputs.gridlock_horn(false)
hotkeys.gridlock_flash()
check(tracked.emo.anger > angerBefore, "honk + long honk + flash raised anger")

-- 4) Wave sorry: anger drops.
local a0 = tracked.emo.anger
hotkeys.gridlock_wave()
check(tracked.emo.anger < a0, string.format("wave calmed the driver (%.0f -> %.0f)", a0, tracked.emo.anger))

-- 5) Indicator: toggles, drives the car's blinker light, auto-cancels after a turn.
hotkeys.gridlock_indicator_left()
check(c.lights.indicator == "left", "left indicator on")
local blinkSeen = false
frames(60, function() if car.lights.LeftBlinker then blinkSeen = true end end)
check(blinkSeen, "game blinker light toggled with the relay")
-- Turn left: rotate heading over 2 s at 20 km/h, then straighten.
car.speed = 20 / 3.6
local h = 0
frames(120, function() h = h + (math.pi / 2) * dt / 2; car.hx, car.hy = math.cos(h), math.sin(h) end)
frames(60)
check(c.lights.indicator == "off", "indicator auto-cancelled after the turn")

-- 6) Seatbelt off at speed: fined when enforcement is on.
c.cfg.enforcement = "fines"
hotkeys.gridlock_seatbelt()
local pts = c.record.points
frames(60 * 6, function() car.x = car.x + car.speed * dt * car.hx; car.y = car.y + car.speed * dt * car.hy end)
check(c.record.points >= pts and c.lastViolation ~= nil and c.lastViolation.kind == "seatbelt", "unbelted driving fined")

-- 7) Leaving the car hides the HUD; no errors throughout.
mounted = false
frames(10)
check(c.inVehicle == false, "on foot: companion idle")
check(mod.error == nil, "no runtime errors (" .. tostring(mod.error) .. ")")
check(imguiCalls > 100, "HUD drew (" .. imguiCalls .. " ImGui calls)")
if failures > 0 then print(failures .. " failure(s)"); os.exit(1) end
print("all CET mock checks passed")
