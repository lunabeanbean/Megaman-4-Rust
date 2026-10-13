-- Trace: boot, navigate menus, then in a stage run a scripted input sequence and log object slot 0 each frame.
-- Run with: powershell -ExecutionPolicy Bypass -File rt.ps1 trace.lua
-- Output: states_out.txt (absolute path, forward slashes; Mesen's own cwd is Documents/MesenCE)
local OUT = "C:/Users/joele/Desktop/MegaMan4 Project/harness/states_out.txt"
local out = io.open(OUT, "w")
local MAXF = 9000
local WAIT = 700
local frame, last, stageFrame, lastLine = 0, -1, nil, ""
local function rd(a) return emu.read(a, emu.memType.nesMemory) end

local seq = {
  {0,30,{}}, {30,60,{right=true}}, {60,90,{}}, {90,100,{a=true}}, {100,160,{}},
  {160,190,{down=true,a=true}}, {190,260,{}},
  {260,270,{b=true}}, {270,320,{}}, {320,420,{b=true}}, {420,500,{}},
  {500,530,{up=true}}, {530,560,{left=true,a=true}}, {560,640,{}}
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
    local line = string.format("X=%02X:%02X.%02X Y=%02X:%02X.%02X vx=%02X.%02X vy=%02X.%02X fl=%02X hp=%02X st=%02X S30=%02X", rd(0x348), rd(0x330), rd(0x318), rd(0x390), rd(0x378), rd(0x360), rd(0x3C0), rd(0x3A8), rd(0x3F0), rd(0x3D8), rd(0x420), rd(0xB0), rd(0x300), rd(0x30))
    if line ~= lastLine then lastLine = line
      out:write(string.format("%d t=%d s=%02X st=%02X X=%02X:%02X.%02X Y=%02X:%02X.%02X vx=%02X.%02X vy=%02X.%02X fl=%02X hp=%02X pad=%02X S30=%02X S31=%02X S34=%02X\n",
        frame, t, s, rd(0x300), rd(0x348), rd(0x330), rd(0x318), rd(0x390), rd(0x378), rd(0x360),
        rd(0x3C0), rd(0x3A8), rd(0x3F0), rd(0x3D8), rd(0x420), rd(0xB0), rd(0x14), rd(0x30), rd(0x31), rd(0x34)))
    end
  end
  frame = frame + 1
  if frame >= MAXF or (stageFrame and frame - stageFrame > WAIT + 700) then out:close(); emu.stop(0) end
end, emu.eventType.endFrame)

emu.addEventCallback(function()
  local inp
  if stageFrame and frame - stageFrame >= WAIT then inp = stageInput(frame - stageFrame - WAIT)
  elseif stageFrame then inp = {}
  else inp = menuInput(frame) end
  emu.setInput(inp, 0)
end, emu.eventType.inputPolled)
