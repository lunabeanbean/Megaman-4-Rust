-- Who writes a RAM range? Boots with the menu script (stage 2), logs PC, frame, address, value of writes to WW_FROM..WW_TO (hex) between frames WW_F0..WW_F1.
-- The first write also dumps the 80 code bytes around the PC as mapped at that moment (line `CODE start bytes...`). Output whowrites.txt
local DIR = "C:/Users/joele/Desktop/MegaMan4 Project/harness/"
local function env(k, d) local v = os.getenv(k); if v == nil or v == "" then return d end return v end
local A0, A1 = tonumber(env("WW_FROM", "600"), 16), tonumber(env("WW_TO", "60F"), 16)
local F0, F1 = tonumber(env("WW_F0", "690")), tonumber(env("WW_F1", "760"))
local out = io.open(DIR .. env("WW_OUT", "whowrites.txt"), "w")
local frame = 0
local dumped = false
local function menuInput(f)
  local m = f % 160
  if m < 20 then return {start = true} elseif m < 60 then return {} elseif m < 80 then return {a = true}
  elseif m < 100 then return {} elseif m < 110 then return {right = true} elseif m < 130 then return {}
  elseif m < 140 then return {down = true} else return {} end
end
emu.addMemoryCallback(function(addr, value)
  if frame >= F0 and frame <= F1 then
    local st = emu.getState()
    local pc = st["cpu.pc"]
    if not dumped then
      dumped = true
      local t = {}
      for a = pc - 48, pc + 32 do t[#t + 1] = string.format("%02X", emu.read(a, emu.memType.nesMemory)) end
      out:write(string.format("CODE %04X %s\n", pc - 48, table.concat(t, " ")))
      out:write(string.format("BANKS f5=%02X f6=%02X\n", emu.read(0xF5, emu.memType.nesMemory), emu.read(0xF6, emu.memType.nesMemory)))
    end
    out:write(string.format("f=%d pc=%04X addr=%04X val=%02X\n", frame, pc, addr, value))
  end
end, emu.callbackType.write, A0, A1)
emu.addEventCallback(function() frame = frame + 1; if frame > F1 then out:close(); emu.stop(0) end end, emu.eventType.endFrame)
emu.addEventCallback(function() emu.setInput(menuInput(frame), 0) end, emu.eventType.inputPolled)
