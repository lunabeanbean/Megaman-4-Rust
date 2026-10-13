-- PPU dumps (+ screenshot) of the real stage-select screen: boots with the menu input of gen_ptrace.js, dumps at the chosen frames.
local DIR = "C:/Users/joele/Desktop/MegaMan4 Project/harness/"
local FRAMES = { [572] = true, [590] = true, [606] = true, [622] = true, [636] = true, [650] = true }
local LAST = 660
local frame = 0
local function rd(a) return emu.read(a, emu.memType.nesMemory) end
local function pr(a) return emu.read(a, emu.memType.nesPpuMemory) end
local function menuInput(f)
  local m = f % 160
  if m < 20 then return {start = true} elseif m < 60 then return {} elseif m < 80 then return {a = true}
  elseif m < 100 then return {} elseif m < 110 then return {right = true} elseif m < 130 then return {}
  elseif m < 140 then return {down = true} else return {} end
end
local function dump(f)
  do local g = io.open(DIR .. "seldump_" .. f .. ".png", "wb"); g:write(emu.takeScreenshot()); g:close() end
  local out = io.open(DIR .. "seldump_" .. f .. ".txt", "w")
  local function line(tag, a0, n, reader) local t = {} for a = a0, a0 + n - 1 do t[#t + 1] = string.format("%02X", reader(a)) end out:write(tag .. " " .. table.concat(t) .. "\n") end
  line("NT0", 0x2000, 0x400, pr); line("NT1", 0x2400, 0x400, pr); line("NT2", 0x2800, 0x400, pr); line("NT3", 0x2C00, 0x400, pr)
  line("PAL", 0x3F00, 0x20, pr); line("CHR", 0x0000, 0x2000, pr); line("OAM", 0x200, 0x100, rd)
  out:write(string.format("REG scene=%02X c10=%02X c11=%02X a9=%02X aa=%02X p95=%02X ff=%02X scroll=%02X,%02X\n", rd(0x22), rd(0x10), rd(0x11), rd(0xA9), rd(0xAA), rd(0x95), rd(0xFF), rd(0xFC), rd(0xFA)))
  local e = {} for a = 0xA0, 0xBF do e[#e + 1] = string.format("%02X", rd(a)) end
  out:write("RAM_A0 " .. table.concat(e, " ") .. "\n")
  out:close()
end
emu.addEventCallback(function()
  if FRAMES[frame] then dump(frame) end
  if frame >= LAST then emu.stop(0) end
  frame = frame + 1
end, emu.eventType.endFrame)
emu.addEventCallback(function() emu.setInput(menuInput(frame), 0) end, emu.eventType.inputPolled)
