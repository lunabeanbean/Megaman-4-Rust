-- Trace: boot, navigate menus, then in a stage run a scripted input sequence and log object slot 0 each frame.
-- Run with: powershell -ExecutionPolicy Bypass -File rt.ps1 trace.lua
-- Output: writes_out.txt (absolute path, forward slashes; Mesen's own cwd is Documents/MesenCE)
local OUT = "C:/Users/joele/Desktop/MegaMan4 Project/harness/writes_out.txt"
local out = io.open(OUT, "w")
local active = false
local MAXF = 9000
local WAIT = 700
local frame, last, stageFrame, lastLine = 0, -1, nil, ""
local function rd(a) return emu.read(a, emu.memType.nesMemory) end

local seq = {  -- {from, to, buttons}, frames relative to the end of WAIT
  {0, 40, {}},
  {40, 100, {right = true}},
  {100, 130, {}},
  {130, 150, {a = true}},
  {150, 200, {}},
  {200, 260, {a = true, right = true}},
  {260, 320, {}},
  {320, 380, {a = true}},
  {380, 420, {left = true}},
}
local function stageInput(t)
  for _, s in ipairs(seq) do if t >= s[1] and t < s[2] then return s[3] end end
  return {}
end
local function menuInput(f)
  local m = f % 160
  if m < 20 then return {start = true}
  elseif m < 60 then return {}
  elseif m < 80 then return {a = true}
  elseif m < 100 then return {}
  elseif m < 110 then return {right = true}
  elseif m < 130 then return {}
  elseif m < 140 then return {down = true}
  else return {} end
end

emu.addEventCallback(function()
  local s = rd(0x22)
  local prev = last
  if s ~= last then
    out:write(string.format("# frame %d scene $22=%02X\n", frame, s))
    last = s
    if s < 0x10 and prev >= 0x10 then stageFrame = frame end
  end
  if stageFrame then
    local t = frame - stageFrame
    local line = string.format("X=%02X:%02X.%02X Y=%02X:%02X.%02X vx=%02X.%02X vy=%02X.%02X fl=%02X hp=%02X st=%02X", rd(0x348), rd(0x330), rd(0x318), rd(0x390), rd(0x378), rd(0x360), rd(0x3C0), rd(0x3A8), rd(0x3F0), rd(0x3D8), rd(0x420), rd(0xB0), rd(0x300))
    if line ~= lastLine then lastLine = line
      out:write(string.format("%d t=%d s=%02X st=%02X X=%02X:%02X.%02X Y=%02X:%02X.%02X vx=%02X.%02X vy=%02X.%02X fl=%02X hp=%02X pad=%02X\n",
        frame, t, s, rd(0x300), rd(0x348), rd(0x330), rd(0x318), rd(0x390), rd(0x378), rd(0x360),
        rd(0x3C0), rd(0x3A8), rd(0x3F0), rd(0x3D8), rd(0x420), rd(0xB0), rd(0x14)))
    end
  end
  frame = frame + 1
  if frame >= MAXF or (stageFrame and frame - stageFrame > WAIT + 430) then out:close(); emu.stop(0) end
end, emu.eventType.endFrame)

emu.addEventCallback(function()
  local inp
  if stageFrame and frame - stageFrame >= WAIT then inp = stageInput(frame - stageFrame - WAIT)
  elseif stageFrame then inp = {}
  else inp = menuInput(frame) end
  emu.setInput(inp, 0)
end, emu.eventType.inputPolled)

-- log writes to slot-0 Y speed ($03D8 sub, $03F0 pixel) and X speed settings with the writing instruction's address
local function onWrite(addr, value)
  if not active then return end
  local st = emu.getState()
  local pc = st["cpu.pc"] or st["cpu.PC"] or -1
  out:write(string.format("W f=%d [$%04X]=%02X pc=%04X\n", frame, addr, value, pc))
end
for _, a in ipairs({0x3D8, 0x3F0, 0x3A8, 0x3C0}) do
  emu.addMemoryCallback(onWrite, emu.callbackType.write, a)
end
emu.addEventCallback(function()
  active = stageFrame ~= nil and (frame - stageFrame) >= WAIT
end, emu.eventType.endFrame)
