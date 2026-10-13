-- Every video frame FROM..TO of the title/menu input: nametable 0 (full only when changed), palette, OAM, key RAM, pattern RAM at the last frame.
-- Env: SF_FROM, SF_TO, SF_OUT (file name in harness/), SF_A9 / SF_AA (hex, poked at frame 590 before the stage select is built). Default output: selframes.txt
local DIR = "C:/Users/joele/Desktop/MegaMan4 Project/harness/"
local FROM, TO = tonumber(os.getenv("SF_FROM") or "560"), tonumber(os.getenv("SF_TO") or "720")
local A9, AA = os.getenv("SF_A9"), os.getenv("SF_AA")
local frame = 0
local function rd(a) return emu.read(a, emu.memType.nesMemory) end
local function pr(a) return emu.read(a, emu.memType.nesPpuMemory) end
local out = io.open(DIR .. (os.getenv("SF_OUT") or "selframes.txt"), "w")
local WALK = os.getenv("SF_WALK")
-- SF_WALK: from frame 612 on, one button (or two) held for 3 of every 6 frames: R R D L L L U U U U R D D D R+D L+U, then A
local walk = { 0x01, 0x01, 0x04, 0x02, 0x02, 0x02, 0x08, 0x08, 0x08, 0x08, 0x01, 0x04, 0x04, 0x04, 0x05, 0x0A }
local function menuInput(f)
  if WALK and f >= 612 then
    local i, w = (f - 612) // 6, (f - 612) % 6
    local b = (i < #walk and w < 3) and walk[i + 1] or 0
    return {right = b & 1 ~= 0, left = b & 2 ~= 0, down = b & 4 ~= 0, up = b & 8 ~= 0}
  end
  local m = f % 160
  if m < 20 then return {start = true} elseif m < 60 then return {} elseif m < 80 then return {a = true}
  elseif m < 100 then return {} elseif m < 110 then return {right = true} elseif m < 130 then return {}
  elseif m < 140 then return {down = true} else return {} end
end
local lastnt = ""
local function hx(a0, n, reader) local t = {} for a = a0, a0 + n - 1 do t[#t + 1] = string.format("%02X", reader(a)) end return table.concat(t) end
emu.addEventCallback(function()
  if A9 and frame == 590 then emu.write(0xA9, tonumber(A9, 16), emu.memType.nesMemory) end
  if AA and frame == 590 then emu.write(0xAA, tonumber(AA, 16), emu.memType.nesMemory) end
  if frame >= FROM and frame <= TO then
    out:write(string.format("F %d scene=%02X c10=%02X c11=%02X a9=%02X aa=%02X p95=%02X p92=%02X ff=%02X f1=%02X 18=%02X 19=%02X\n", frame, rd(0x22), rd(0x10), rd(0x11), rd(0xA9), rd(0xAA), rd(0x95), rd(0x92), rd(0xFF), rd(0xF1), rd(0x18), rd(0x19)))
    out:write("PAL " .. hx(0x3F00, 0x20, pr) .. "\n")
    out:write("OAM " .. hx(0x200, 0x100, rd) .. "\n")
    local nt = hx(0x2000, 0x400, pr)
    if nt ~= lastnt then out:write("NT0 " .. nt .. "\n"); lastnt = nt end
  end
  if frame == TO then out:write("CHR " .. hx(0, 0x2000, pr) .. "\n") end
  if frame >= TO then out:close(); emu.stop(0) end
  frame = frame + 1
end, emu.eventType.endFrame)
emu.addEventCallback(function() emu.setInput(menuInput(frame), 0) end, emu.eventType.inputPolled)
