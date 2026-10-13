-- Probe: press Start/A periodically, log scene id ($22) changes. Output: probe_out.txt
local out = io.open("probe_out.txt", "w")
local frame = 0
local last = -1
local MAXF = 4000
local function rd(a) return emu.read(a, emu.memType.nesMemory) end
emu.addEventCallback(function()
  local s = rd(0x22)
  if s ~= last then out:write(string.format("frame %d scene $22=%02X\n", frame, s)); last = s end
  frame = frame + 1
  if frame >= MAXF then out:close(); emu.stop(0) end
end, emu.eventType.endFrame)
emu.addEventCallback(function()
  local press = (frame % 40) < 4
  emu.setInput({start = press, a = false}, 0)
end, emu.eventType.inputPolled)
