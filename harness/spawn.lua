-- Trace: boot, navigate menus, then in a stage run a scripted input sequence and log object slot 0 each frame.
-- Run with: powershell -ExecutionPolicy Bypass -File rt.ps1 trace.lua
-- Output: spawn_out.txt (absolute path, forward slashes; Mesen's own cwd is Documents/MesenCE)
local OUT = "C:/Users/joele/Desktop/MegaMan4 Project/harness/spawn_out.txt"
local out = io.open(OUT, "w")
local MAXF = 9000
local WAIT = 700
local frame, last, stageFrame, lastLine = 0, -1, nil, ""
local function rd(a) return emu.read(a, emu.memType.nesMemory) end

local seq = {
  {0,950,{right=true},}
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
    local line = string.format("scrX=%02X scrY=%02X nt=%02X ctl=%02X mask=%02X pX=%02X:%02X pY=%02X:%02X", rd(0xFC), rd(0xFA), rd(0xFD), rd(0xFF), rd(0xFE), rd(0x348), rd(0x330), rd(0x390), rd(0x378))
    if line ~= lastLine then lastLine = line
      out:write(string.format("%d t=%d %s\n", frame, t, line))
    end
  end
  frame = frame + 1
  if frame >= MAXF or (stageFrame and frame - stageFrame > WAIT + 950) then out:close(); emu.stop(0) end
end, emu.eventType.endFrame)

emu.addEventCallback(function()
  local inp
  if stageFrame and frame - stageFrame >= WAIT then inp = stageInput(frame - stageFrame - WAIT)
  elseif stageFrame then inp = {}
  else inp = menuInput(frame) end
  emu.setInput(inp, 0)
end, emu.eventType.inputPolled)

local function onWrite(addr, value)
  if not stageFrame or frame - stageFrame < WAIT then return end
  local st = emu.getState()
  out:write(string.format("W f=%d [$%04X]=%02X pc=%04X\n", frame, addr, value, st["cpu.pc"] or -1))
end
emu.addMemoryCallback(onWrite, emu.callbackType.write, 0x301, 0x317)
