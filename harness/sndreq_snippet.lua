-- sound requests as the game makes them: exec hooks on code_3FF6BC (music: also stores $D9) and code_3FF6BE (queue a sound; 6BC falls into it, so a music request logs both lines)
-- line: "<row/frame> <kind> <A hex> <return address hex>"   (kind C = F6BC, E = F6BE)
local function sndLog(kind)
  local s = emu.getState()
  local sp = s["cpu.sp"]
  local ret = (emu.read(0x100 + ((sp + 2) & 255), emu.memType.nesMemory) * 256 + emu.read(0x100 + ((sp + 1) & 255), emu.memType.nesMemory) + 1) & 0xFFFF
  SNDOUT:write(string.format("%s %s %02X %04X\n", tostring(SNDTAG()), kind, s["cpu.a"], ret))
end
emu.addMemoryCallback(function() sndLog("C") end, emu.callbackType.exec, 0xF6BC)
emu.addMemoryCallback(function() sndLog("E") end, emu.callbackType.exec, 0xF6BE)
