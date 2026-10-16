-- Gridlock HUD (ImGui, drawn by CET every frame).
local Hud = {}

local C = {
  ink = { 0.92, 0.91, 0.86, 1 },
  muted = { 0.61, 0.59, 0.55, 1 },
  sodium = { 1.0, 0.70, 0.36, 1 },
  green = { 0.21, 0.89, 0.60, 1 },
  amber = { 1.0, 0.76, 0.2, 1 },
  red = { 1.0, 0.30, 0.24, 1 },
  neon = { 1.0, 0.16, 0.43, 1 },
  dim = { 0.3, 0.3, 0.3, 1 },
}

local function text(col, s)
  ImGui.TextColored(col[1], col[2], col[3], col[4], s)
end

local KIND_COLOR = { violation = C.red, rage = C.neon, social = C.amber, good = C.green, info = C.muted }

function Hud.draw(c, overlayOpen)
  if c == nil then return end
  if c.inVehicle and c.cfg.hud then Hud.drawDriving(c) end
  if overlayOpen then Hud.drawSettings(c) end
end

function Hud.drawDriving(c)
  local w, h = 1920, 1080
  pcall(function() w, h = GetDisplayResolution() end)
  ImGui.SetNextWindowPos(w - 330, h - 360, ImGuiCond.Always)
  ImGui.SetNextWindowSize(310, 0, ImGuiCond.Always)
  ImGui.PushStyleColor(ImGuiCol.WindowBg, 0.05, 0.055, 0.075, 0.78)
  ImGui.PushStyleColor(ImGuiCol.Border, 0.92, 0.91, 0.86, 0.14)
  local flags = ImGuiWindowFlags.NoTitleBar + ImGuiWindowFlags.NoResize + ImGuiWindowFlags.NoMove + ImGuiWindowFlags.NoScrollbar + ImGuiWindowFlags.NoCollapse + ImGuiWindowFlags.NoInputs + ImGuiWindowFlags.AlwaysAutoResize
  if ImGui.Begin("Gridlock##hud", flags) then
    local l = c.lights
    local left = l:leftLampOn()
    local right = l:rightLampOn()
    text(left and C.green or C.dim, "<<")
    ImGui.SameLine()
    ImGui.SetWindowFontScale(1.6)
    local kmh = math.floor(math.abs(c.speed) * 3.6 + 0.5)
    text(C.ink, string.format("  %3d km/h  ", kmh))
    ImGui.SetWindowFontScale(1.0)
    ImGui.SameLine()
    text(right and C.green or C.dim, ">>")
    local limit = c.cfg.speedLimitKmh
    local over = kmh > limit * 1.1
    text(over and C.red or C.muted, string.format("Limit %d km/h", limit))
    if c.rpm ~= nil then
      ImGui.SameLine()
      text(C.muted, string.format("   %4d rpm  gear %s", math.floor(c.rpm), tostring(c.gear)))
    end
    local tt = {}
    if l.hazard then tt[#tt + 1] = { "HAZARD", l:blinkOn() and C.red or C.dim } end
    if l.indicator ~= "off" then tt[#tt + 1] = { "SIGNAL " .. string.upper(l.indicator), C.green } end
    if not c.seatbelt then tt[#tt + 1] = { "BELT", C.red } end
    if c.cfg.enforcement == "off" then tt[#tt + 1] = { "LAW OFF", C.muted } end
    for i, t in ipairs(tt) do
      if i > 1 then ImGui.SameLine() end
      text(t[2], "[" .. t[1] .. "]")
    end
    ImGui.Separator()
    local rec = c.record
    text(C.muted, string.format("License points %d/%d   Fines E$%d", rec.points, rec.maxPoints, c.finesTotal))
    local angry = c:angry()
    if #angry > 0 then
      ImGui.Separator()
      text(C.neon, "Drivers angry at you")
      for i = 1, math.min(4, #angry) do
        local npc = angry[i]
        local label = npc.p.label .. " - " .. npc.emo.mood .. (npc.raging and " (road rage!)" or "")
        ImGui.ProgressBar(npc.emo.anger / 100, 290, 12, label)
      end
    end
    if #c.log > 0 then
      ImGui.Separator()
      for i = 1, math.min(5, #c.log) do
        local e = c.log[i]
        local age = c.time - e.t
        if age < 8 then
          local col = KIND_COLOR[e.kind] or C.muted
          ImGui.TextColored(col[1], col[2], col[3], math.max(0.25, 1 - age / 8), e.text)
        end
      end
    end
  end
  ImGui.End()
  ImGui.PopStyleColor(2)
end

function Hud.drawSettings(c)
  if ImGui.Begin("Gridlock: traffic & driving") then
    local cfg = c.cfg
    text(C.sodium, "Gridlock companion")
    text(C.muted, "Bind keys in CET > Bindings (indicators, hazards, horn, seatbelt, wave...)")
    ImGui.Separator()
    cfg.hud = ImGui.Checkbox("Show driving HUD", cfg.hud)
    cfg.driveGameBlinkers = ImGui.Checkbox("Drive the car's blinker lights", cfg.driveGameBlinkers)
    cfg.speedLimitKmh = ImGui.SliderInt("Posted limit (km/h)", cfg.speedLimitKmh, 20, 130)
    cfg.socialRadius = ImGui.SliderInt("NPC reaction radius (m)", cfg.socialRadius, 15, 80)
    text(C.ink, "Enforcement")
    if ImGui.RadioButton("Fines", cfg.enforcement == "fines") then cfg.enforcement = "fines" end
    ImGui.SameLine()
    if ImGui.RadioButton("Record only", cfg.enforcement == "record") then cfg.enforcement = "record" end
    ImGui.SameLine()
    if ImGui.RadioButton("Off", cfg.enforcement == "off") then cfg.enforcement = "off" end
    ImGui.Separator()
    text(C.muted, string.format("Tracked NPC drivers: %d", c.npcCount))
    local nfail = 0
    for name, err in pairs(c.adapter.failures) do
      nfail = nfail + 1
      text(C.amber, "Unavailable: " .. name)
      if ImGui.IsItemHovered() then ImGui.SetTooltip(err) end
    end
    if nfail == 0 then text(C.green, "All game bindings working") end
  end
  ImGui.End()
end

return Hud
