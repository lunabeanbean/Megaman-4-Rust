-- Dumps the PPU state (nametables, palette, OAM, pattern RAM) of the real game at chosen frames while the menu input of gen_ptrace.js runs.
-- Usage: edit FRAMES, run  powershell -ExecutionPolicy Bypass -File rt.ps1 ppu_dump.lua   -> ppudump_<frame>.txt (hex lines) and scene.txt (frame, $22).
local DIR = "C:/Users/joele/Desktop/MegaMan4 Project/harness/"
local FRAMES = { [60]=true, [120]=true, [200]=true, [300]=true, [420]=true, [560]=true }
local LAST = 600
local frame = 0
local function rd(a) return emu.read(a, emu.memType.nesMemory) end
local function pr(a) return emu.read(a, emu.memType.nesPpuMemory) end
local scene = io.open(DIR .. "scene.txt", "w")
local function menuInput(f)
  local m = f % 160
  if m < 20 then return {start = true} elseif m < 60 then return {} elseif m < 80 then return {a = true}
  elseif m < 100 then return {} elseif m < 110 then return {right = true} elseif m < 130 then return {}
  elseif m < 140 then return {down = true} else return {} end
end
local function dump(f)
  local out = io.open(DIR .. "ppudump_" .. f .. ".txt", "w")
  local function line(tag, a0, n, reader)
    local t = {}
    for a = a0, a0 + n - 1 do t[#t + 1] = string.format("%02X", reader(a)) end
    out:write(tag .. " " .. table.concat(t) .. "\n")
  end
  line("NT0", 0x2000, 0x400, pr); line("NT1", 0x2400, 0x400, pr)
  line("NT2", 0x2800, 0x400, pr); line("NT3", 0x2C00, 0x400, pr)
  line("PAL", 0x3F00, 0x20, pr)
  line("CHR", 0x0000, 0x2000, pr)
  line("OAM", 0x200, 0x100, rd)
  out:write(string.format("REG scene=%02X ff=%02X f5=%02X f6=%02X scroll=%02X,%02X\n", rd(0x22), rd(0xFF), rd(0xF5), rd(0xF6), rd(0xFC), rd(0xFA)))
  out:close()
end
emu.addEventCallback(function()
  scene:write(string.format("%d %02X\n", frame, rd(0x22)))
  if FRAMES[frame] then dump(frame) end
  if frame >= LAST then scene:close(); emu.stop(0) end
  frame = frame + 1
end, emu.eventType.endFrame)
emu.addEventCallback(function() emu.setInput(menuInput(frame), 0) end, emu.eventType.inputPolled)
