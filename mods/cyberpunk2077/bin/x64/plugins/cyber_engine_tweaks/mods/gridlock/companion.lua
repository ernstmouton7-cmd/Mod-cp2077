-- Gridlock companion: runs the engine-agnostic Gridlock core inside Cyberpunk 2077.
-- It drives the player's indicators/hazards, keeps a driving record (fines and
-- license points for what can be detected without map data), and gives nearby
-- NPC drivers the Gridlock emotion model: tailgate them, cut them off, honk or
-- flash at them and they get angry; wave to apologise.
local Companion = {}
Companion.__index = Companion

local PLAYER_ID = 1 -- offender id used for the player in NPC grudges

local function clamp(v, lo, hi)
  if v < lo then return lo elseif v > hi then return hi end
  return v
end

local function angleDiff(a, b)
  local d = b - a
  while d > math.pi do d = d - 2 * math.pi end
  while d < -math.pi do d = d + 2 * math.pi end
  return d
end

function Companion.new(core, adapter, config)
  local self = setmetatable({}, Companion)
  self.core = core
  self.adapter = adapter
  self.cfg = config
  local seed = 2077
  pcall(function() seed = (os.time() % 100000) + 1 end)
  self.rng = core.createRng(seed)
  self.lights = core.createLights(1)
  self.record = core.createRecord()
  self.npcs = {}
  self.npcCount = 0
  self.log = {}
  self.time = 0
  self.seatbelt = true
  self.horn = false
  self.hornTime = 0
  self.flashTime = 0
  self.inVehicle = false
  self.vehicleName = ""
  self.speed = 0
  self.rpm = nil
  self.gear = nil
  self.timers = { speeding = 0, belt = 0, slide = 0, scan = 0 }
  self.lastLamps = { left = false, right = false }
  self.finesTotal = 0
  self.lastViolation = nil
  self.pending = {}
  return self
end

function Companion:note(text, kind)
  table.insert(self.log, 1, { text = text, kind = kind or "info", t = self.time })
  while #self.log > 6 do table.remove(self.log) end
end

-- ----------------------------------------------------------------- controls

function Companion:press(action)
  local l = self.lights
  if action == "indicator_left" then l:toggleIndicator("left")
  elseif action == "indicator_right" then l:toggleIndicator("right")
  elseif action == "hazards" then
    l.hazard = not l.hazard
    if l.hazard then self:hazardThanks() end
  elseif action == "seatbelt" then
    self.seatbelt = not self.seatbelt
    self:note(self.seatbelt and "Seatbelt fastened" or "Seatbelt unfastened")
  elseif action == "flash" then self:flashAhead()
  elseif action == "apologize" then self:gesture("wave")
  elseif action == "rude_gesture" then self:gesture("rude")
  elseif action == "limit_up" then
    self.cfg.speedLimitKmh = math.min(130, self.cfg.speedLimitKmh + 10)
    self:note("Posted limit " .. self.cfg.speedLimitKmh .. " km/h")
  elseif action == "limit_down" then
    self.cfg.speedLimitKmh = math.max(20, self.cfg.speedLimitKmh - 10)
    self:note("Posted limit " .. self.cfg.speedLimitKmh .. " km/h")
  elseif action == "toggle_hud" then self.cfg.hud = not self.cfg.hud
  end
end

function Companion:setHorn(down)
  if down and not self.horn then self:honkAhead(false) end
  if not down and self.horn and self.hornTime > 0.9 then self:honkAhead(true) end
  self.horn = down
  if not down then self.hornTime = 0 end
end

-- ----------------------------------------------------------------- NPCs

--- Relative geometry of an NPC to the player: ahead (m), side (m, + = left), same direction.
function Companion:relative(npc)
  local p, f = self.pos, self.fwd
  local dx, dy = npc.pos.x - p.x, npc.pos.y - p.y
  local ahead = dx * f.x + dy * f.y
  local side = -dx * f.y + dy * f.x
  local same = (f.x * npc.fwd.x + f.y * npc.fwd.y) > 0.8
  return ahead, side, same
end

function Companion:offend(npc, kind, scale)
  local gain = npc.emo:offend(PLAYER_ID, kind, scale, self.time)
  if gain > 0.5 and kind ~= "tailgating" then
    local mood = npc.emo.mood
    if mood ~= "calm" and self.time - npc.lastSpeech > 6 then
      npc.lastSpeech = self.time
      self:note(npc.p.label .. ": \"" .. self.core.lineFor(self.rng, mood, kind) .. "\"", mood == "furious" and "rage" or "social")
    end
  end
end

function Companion:scanNpcs()
  local list = self.adapter.nearbyVehicles(self.cfg.socialRadius)
  local seen = {}
  for _, ent in ipairs(list) do
    local key = self.adapter.entityKey(ent)
    if key ~= nil and key ~= self.vehicleKey then
      local npc = self.npcs[key]
      if npc == nil then
        local p = self.core.randomPersonality(self.rng)
        npc = { key = key, ent = ent, p = p, emo = self.core.createEmotion(p), lastSpeech = -100, tail = 0, mood = "calm", side = 0, raging = false }
        self.npcs[key] = npc
        self.npcCount = self.npcCount + 1
      end
      npc.ent = ent
      npc.pos = self.adapter.position(ent) or npc.pos
      npc.fwd = self.adapter.forward(ent) or npc.fwd
      npc.speed = self.adapter.speed(ent)
      npc.lastSeen = self.time
      seen[key] = true
    end
  end
  for key, npc in pairs(self.npcs) do
    if not seen[key] and self.time - (npc.lastSeen or 0) > 15 then
      self.npcs[key] = nil
      self.npcCount = self.npcCount - 1
    end
  end
end

function Companion:updateNpcs(dt)
  for _, npc in pairs(self.npcs) do
    if npc.pos ~= nil and npc.fwd ~= nil then
      local ahead, side, same = self:relative(npc)
      -- Tailgating: we sit right behind them at speed.
      if same and ahead > 1.5 and ahead < 18 and math.abs(side) < 1.8 and self.speed > 8 then
        local gap = ahead - 4.6
        if gap / self.speed < 0.6 then
          npc.tail = npc.tail + dt
          if npc.tail > 2 then self:offend(npc, "tailgating", dt * 2.5) end
        else npc.tail = 0 end
      else npc.tail = 0 end
      -- Cut-off: they are now right behind us, and were beside us a moment ago.
      if same and ahead < -3 and ahead > -12 and math.abs(side) < 1.6 and (npc.speed or 0) > 6 and math.abs(npc.side) > 2.4 and self.time - (npc.cutAt or -100) > 5 then
        npc.cutAt = self.time
        self:offend(npc, "cut_off", clamp((12 + ahead) / 6, 0.5, 2))
      end
      npc.side = npc.side + (side - npc.side) * math.min(1, dt * 1.5)
      npc.emo:update(dt, self.time, 0)
      local mood = npc.emo.mood
      if mood ~= npc.mood then
        if mood == "furious" then self:note(npc.p.label .. " is furious with you", "rage") end
        npc.mood = mood
      end
      npc.raging = npc.emo:canRage() and npc.emo.target == PLAYER_ID
    end
  end
end

function Companion:inCone(npc, range, width)
  if npc.pos == nil then return false end
  local ahead, side = self:relative(npc)
  return ahead > 0 and ahead < range and math.abs(side) < width + ahead * 0.2
end

function Companion:honkAhead(long)
  for _, npc in pairs(self.npcs) do
    if self:inCone(npc, 30, 4) then self:offend(npc, long and "long_honk" or "honked_at", 1) end
  end
end

function Companion:flashAhead()
  for _, npc in pairs(self.npcs) do
    if self:inCone(npc, 45, 3) then self:offend(npc, "high_beams", 1) end
  end
end

function Companion:gesture(kind)
  local calmed, upset = 0, 0
  for _, npc in pairs(self.npcs) do
    if npc.pos ~= nil then
      local d = math.sqrt((npc.pos.x - self.pos.x) ^ 2 + (npc.pos.y - self.pos.y) ^ 2)
      if kind == "wave" and d < 45 and npc.emo:grudgeAgainst(PLAYER_ID) > 0 then
        calmed = calmed + npc.emo:apologize(PLAYER_ID, self.time)
      elseif kind == "rude" and d < 30 then
        self:offend(npc, "rude_gesture", 1)
        upset = upset + 1
      end
    end
  end
  if kind == "wave" then self:note(calmed > 1 and ("Apology accepted (anger -" .. math.floor(calmed) .. ")") or "You wave. Nobody was upset.", "good")
  elseif upset > 0 then self:note("You flip off " .. upset .. " driver(s)", "rage") end
end

function Companion:hazardThanks()
  for _, npc in pairs(self.npcs) do
    if npc.pos ~= nil and npc.emo:grudgeAgainst(PLAYER_ID) > 0 then
      local ahead = self:relative(npc)
      if ahead < 0 and ahead > -50 then npc.emo:apologize(PLAYER_ID, self.time) end
    end
  end
end

function Companion:nearestNpc(maxD)
  local best, bd = nil, maxD
  for _, npc in pairs(self.npcs) do
    if npc.pos ~= nil then
      local d = math.sqrt((npc.pos.x - self.pos.x) ^ 2 + (npc.pos.y - self.pos.y) ^ 2)
      if d < bd then best, bd = npc, d end
    end
  end
  return best
end

--- NPCs angry at the player, angriest first (for the HUD).
function Companion:angry()
  local out = {}
  for _, npc in pairs(self.npcs) do
    local g = npc.emo:grudgeAgainst(PLAYER_ID)
    if g > 10 then out[#out + 1] = npc end
  end
  table.sort(out, function(a, b) return a.emo.anger > b.emo.anger end)
  return out
end

-- ----------------------------------------------------------------- law

function Companion:violation(kind)
  if self.cfg.enforcement == "off" then return end
  local rec = self.record
  if not rec:canCharge(kind, self.time) then return end
  local fines = self.cfg.enforcement == "fines"
  local v = rec:add(kind, self.time, fines and "camera" or "none", self.pos.x, self.pos.y)
  self.lastViolation = v
  if fines then
    self.adapter.removeMoney(v.fine)
    self.finesTotal = self.finesTotal + v.fine
    self:note(v.label .. ": -E$" .. v.fine .. ", +" .. v.points .. " pts", "violation")
    self.adapter.notify("Gridlock: " .. v.label .. " (-E$" .. v.fine .. ")")
  else
    self:note(v.label .. " (logged)", "violation")
  end
end

function Companion:law(dt, accel, slip)
  local limit = self.cfg.speedLimitKmh / 3.6
  local sp = math.abs(self.speed)
  if sp > limit * 1.1 + 0.3 then
    self.timers.speeding = self.timers.speeding + dt
    if self.timers.speeding > 3 then
      local over = (sp - limit) / limit
      self:violation(over > 0.5 and "speeding_extreme" or over > 0.25 and "speeding_major" or "speeding_minor")
    end
  else self.timers.speeding = 0 end
  if not self.seatbelt and sp > 5.5 then
    self.timers.belt = self.timers.belt + dt
    if self.timers.belt > 5 then self:violation("seatbelt") end
  else self.timers.belt = 0 end
  if slip > 0.45 and sp > 8 then
    self.timers.slide = self.timers.slide + dt
    if self.timers.slide > 2 then self:violation("reckless") end
  else self.timers.slide = math.max(0, self.timers.slide - dt) end
  if self.horn then
    self.hornTime = self.hornTime + dt
    if self.hornTime > 4 then self:violation("horn_abuse") end
  end
  -- Sudden deceleration while moving: a collision.
  if accel < -14 and self.prevSpeed > 5 then
    local npc = self:nearestNpc(8)
    if npc ~= nil then self:offend(npc, "collision", clamp(-accel / 30, 0.5, 2.5)) end
    self:violation("collision")
  end
end

-- ----------------------------------------------------------------- frame

function Companion:onEnter(veh)
  self.inVehicle = true
  self.vehicleKey = self.adapter.entityKey(veh)
  self.vehicleName = self.adapter.displayName(veh)
  self.prevPos, self.prevHeading, self.prevSpeed = nil, nil, 0
  self:note("Driving " .. self.vehicleName .. (self.seatbelt and "" or " (belt off)"))
end

function Companion:update(dt)
  if dt <= 0 then return end
  self.time = self.time + dt
  local veh = self.adapter.mountedVehicle()
  if veh == nil then
    self.inVehicle = false
    return
  end
  if not self.inVehicle then self:onEnter(veh) end
  local pos = self.adapter.position(veh)
  local fwd = self.adapter.forward(veh)
  if pos == nil or fwd == nil then return end
  self.pos, self.fwd = pos, fwd
  self.speed = self.adapter.speed(veh)
  self.rpm, self.gear = self.adapter.rpmGear(veh)
  local heading = math.atan2(fwd.y, fwd.x)
  local yawRate, accel, slip = 0, 0, 0
  if self.prevPos ~= nil then
    local vx, vy = (pos.x - self.prevPos.x) / dt, (pos.y - self.prevPos.y) / dt
    local vel = math.sqrt(vx * vx + vy * vy)
    yawRate = angleDiff(self.prevHeading, heading) / dt
    accel = (self.speed - self.prevSpeed) / dt
    if vel > 3 then slip = math.abs(angleDiff(heading, math.atan2(vy, vx))) end
    if slip > math.pi / 2 then slip = math.pi - slip end -- reversing
  end
  -- Equivalent front-wheel angle from the yaw rate (bicycle model), for indicator auto-cancel.
  local steer = clamp(yawRate * 2.7 / math.max(math.abs(self.speed), 2), -0.6, 0.6)
  self.lights:update(dt, steer)
  if self.cfg.driveGameBlinkers then
    local l, r = self.lights:leftLampOn(), self.lights:rightLampOn()
    if l ~= self.lastLamps.left then self.adapter.setLight(veh, "left", l) end
    if r ~= self.lastLamps.right then self.adapter.setLight(veh, "right", r) end
    self.lastLamps.left, self.lastLamps.right = l, r
  end
  self.timers.scan = self.timers.scan - dt
  if self.timers.scan <= 0 then
    self.timers.scan = 0.25
    self:scanNpcs()
  end
  self:updateNpcs(dt)
  self:law(dt, accel, slip)
  self.record:driveClean(math.abs(self.speed) * dt)
  self.prevPos, self.prevHeading, self.prevSpeed = pos, heading, self.speed
end

return Companion
