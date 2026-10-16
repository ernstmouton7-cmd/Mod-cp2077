-- Gridlock for Cyberpunk 2077 (Cyber Engine Tweaks mod).
-- Loads the Gridlock simulation core (gridlock_core.lua, generated from the
-- TypeScript sources by TypeScriptToLua) and runs the companion systems.
local Gridlock = { overlay = false, companion = nil, error = nil }

local core = require("gridlock_core")
local Adapter = require("adapter")
local Companion = require("companion")
local Hud = require("hud")
local config = require("config")

registerForEvent("onInit", function()
  local ok, err = pcall(function()
    Gridlock.companion = Companion.new(core, Adapter, config)
  end)
  if not ok then
    Gridlock.error = tostring(err)
    print("[Gridlock] init failed: " .. Gridlock.error)
  else
    print("[Gridlock] core v" .. tostring(core.VERSION) .. " ready")
  end
end)

registerForEvent("onUpdate", function(dt)
  local c = Gridlock.companion
  if c == nil then return end
  local ok, err = pcall(c.update, c, dt)
  if not ok and Gridlock.error == nil then
    Gridlock.error = tostring(err)
    print("[Gridlock] update error: " .. Gridlock.error)
  end
end)

registerForEvent("onDraw", function()
  local ok, err = pcall(Hud.draw, Gridlock.companion, Gridlock.overlay)
  if not ok and Gridlock.error == nil then Gridlock.error = tostring(err) end
end)

registerForEvent("onOverlayOpen", function() Gridlock.overlay = true end)
registerForEvent("onOverlayClose", function() Gridlock.overlay = false end)

local function press(action)
  return function()
    if Gridlock.companion ~= nil then Gridlock.companion:press(action) end
  end
end

registerHotkey("gridlock_indicator_left", "Left indicator", press("indicator_left"))
registerHotkey("gridlock_indicator_right", "Right indicator", press("indicator_right"))
registerHotkey("gridlock_hazards", "Hazard lights / thank-you blink", press("hazards"))
registerHotkey("gridlock_seatbelt", "Seatbelt", press("seatbelt"))
registerHotkey("gridlock_flash", "Flash high beams at the car ahead", press("flash"))
registerHotkey("gridlock_wave", "Wave sorry (calms drivers you upset)", press("apologize"))
registerHotkey("gridlock_rude", "Rude gesture", press("rude_gesture"))
registerHotkey("gridlock_limit_up", "Posted speed limit +10", press("limit_up"))
registerHotkey("gridlock_limit_down", "Posted speed limit -10", press("limit_down"))
registerHotkey("gridlock_toggle_hud", "Toggle Gridlock HUD", press("toggle_hud"))
registerInput("gridlock_horn", "Horn (bind to the same key as the game's horn)", function(down)
  if Gridlock.companion ~= nil then Gridlock.companion:setHorn(down) end
end)

return Gridlock
