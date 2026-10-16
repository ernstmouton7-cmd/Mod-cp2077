-- Gridlock CET adapter: every call into the game lives here, wrapped in pcall.
-- If an API differs in your game/CET version, only this file needs changing;
-- a failing binding disables that feature instead of breaking the mod.
local Adapter = {
  failures = {}, -- name -> error string (shown in the overlay's diagnostics)
}

local function try(name, fn, ...)
  local ok, a, b, c = pcall(fn, ...)
  if not ok then
    if Adapter.failures[name] == nil then
      Adapter.failures[name] = tostring(a)
      print("[Gridlock] binding '" .. name .. "' failed: " .. tostring(a))
    end
    return nil
  end
  return a, b, c
end

function Adapter.player()
  return try("GetPlayer", function() return Game.GetPlayer() end)
end

--- Vehicle the player is mounted in (nil on foot).
function Adapter.mountedVehicle()
  local p = Adapter.player()
  if p == nil then return nil end
  return try("GetMountedVehicle", function() return Game.GetMountedVehicle(p) end)
end

--- World position {x, y, z} (CP2077 is Z-up, x/y ground plane: same as Gridlock).
function Adapter.position(ent)
  return try("GetWorldPosition", function()
    local v = ent:GetWorldPosition()
    return { x = v.x, y = v.y, z = v.z }
  end)
end

--- Unit forward vector on the ground plane.
function Adapter.forward(ent)
  return try("GetWorldForward", function()
    local f = ent:GetWorldForward()
    local l = math.sqrt(f.x * f.x + f.y * f.y)
    if l < 1e-6 then return { x = 1, y = 0 } end
    return { x = f.x / l, y = f.y / l }
  end)
end

--- Forward speed in m/s (VehicleObject.GetCurrentSpeed).
function Adapter.speed(veh)
  return try("GetCurrentSpeed", function() return veh:GetCurrentSpeed() end) or 0
end

--- Engine rpm and gear from the vehicle blackboard (used by the game's own HUD).
function Adapter.rpmGear(veh)
  return try("VehicleBlackboard", function()
    local bb = veh:GetBlackboard()
    local defs = GetAllBlackboardDefs().Vehicle
    return bb:GetFloat(defs.RPMValue), bb:GetInt(defs.GearValue)
  end)
end

function Adapter.entityKey(ent)
  return try("GetEntityID", function() return tostring(ent:GetEntityID().hash) end)
end

function Adapter.displayName(veh)
  return try("GetDisplayName", function() return GetLocalizedText(veh:GetDisplayName()) end) or "Vehicle"
end

--- In-game clock (hours as a float).
function Adapter.gameHour()
  return try("GameTime", function()
    local t = Game.GetTimeSystem():GetGameTime()
    return t:Hours() + t:Minutes() / 60
  end) or 12
end

--- Toggle one of the vehicle's light groups (best effort: vehicleController.ToggleLights).
local LIGHT = { left = "LeftBlinker", right = "RightBlinker", head = "Head", brake = "Brake", reverse = "Reverse" }
function Adapter.setLight(veh, which, on)
  return try("ToggleLights:" .. which, function()
    local ctrl = veh:GetVehicleComponent():GetVehicleController()
    ctrl:ToggleLights(on, vehicleELightType[LIGHT[which]])
    return true
  end)
end

--- Vehicles near the player (best effort: targeting system query).
function Adapter.nearbyVehicles(radius)
  local p = Adapter.player()
  if p == nil then return {} end
  return try("TargetingQuery", function()
    local query = Game["TSQ_ALL;"]()
    query.maxDistance = radius
    local ok, parts = Game.GetTargetingSystem():GetTargetParts(p, query)
    local out, seen = {}, {}
    if not ok or parts == nil then return out end
    for _, part in ipairs(parts) do
      local ent = part:GetComponent(part):GetEntity()
      if ent ~= nil and ent:IsVehicle() then
        local key = tostring(ent:GetEntityID().hash)
        if not seen[key] then
          seen[key] = true
          out[#out + 1] = ent
        end
      end
    end
    return out
  end) or {}
end

--- Charge the player (eurodollars). Negative amounts pay out.
function Adapter.removeMoney(amount)
  local p = Adapter.player()
  if p == nil or amount == 0 then return false end
  return try("Money", function()
    local ts = Game.GetTransactionSystem()
    if amount > 0 then
      ts:RemoveItem(p, MarketSystem.Money(), amount)
    else
      ts:GiveItem(p, MarketSystem.Money(), -amount)
    end
    return true
  end) or false
end

--- Player money (for the HUD).
function Adapter.money()
  local p = Adapter.player()
  if p == nil then return nil end
  return try("MoneyRead", function() return Game.GetTransactionSystem():GetItemQuantity(p, MarketSystem.Money()) end)
end

--- On-screen warning using the game's own message system (falls back to HUD only).
function Adapter.notify(text)
  return try("Notify", function()
    local msg = SimpleScreenMessage.new()
    msg.message = text
    msg.isShown = true
    msg.duration = 3.0
    local bb = Game.GetBlackboardSystem():Get(GetAllBlackboardDefs().UI_Notifications)
    bb:SetVariant(GetAllBlackboardDefs().UI_Notifications.WarningMessage, ToVariant(msg), true)
    return true
  end)
end

return Adapter
