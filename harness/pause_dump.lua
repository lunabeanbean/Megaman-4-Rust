-- PPU dumps of the real pause menu: boots to stage 2, pokes some weapons, opens the pause menu with Start and dumps the PPU at set frames after that.
-- run: powershell -ExecutionPolicy Bypass -File rt.ps1 pause_dump.lua "<rom>"   -> pausedump_<n>.txt (format of ppu_dump.lua)
local DIR = "C:/Users/joele/Desktop/MegaMan4 Project/harness/"
local function rd(a) return emu.read(a, emu.memType.nesMemory) end
local function pr(a) return emu.read(a, emu.memType.nesPpuMemory) end
local frame, last, stageFrame, pauseAt = 0, -1, nil, nil
local DUMPS = { [4] = true, [8] = true, [12] = true, [20] = true, [30] = true, [40] = true, [55] = true }
local function menuInput(f)
  local m = f % 160
  if m < 20 then return {start = true} elseif m < 60 then return {} elseif m < 80 then return {a = true}
  elseif m < 100 then return {} elseif m < 110 then return {right = true} elseif m < 130 then return {}
  elseif m < 140 then return {down = true} else return {} end
end
local function dump(n)
  do local f = io.open(DIR .. "pausedump_" .. n .. ".png", "wb"); f:write(emu.takeScreenshot()); f:close() end
  local out = io.open(DIR .. "pausedump_" .. n .. ".txt", "w")
  local function line(tag, a0, c, reader) local t = {} for a = a0, a0 + c - 1 do t[#t + 1] = string.format("%02X", reader(a)) end out:write(tag .. " " .. table.concat(t) .. "\n") end
  line("NT0", 0x2000, 0x400, pr); line("NT1", 0x2400, 0x400, pr); line("NT2", 0x2800, 0x400, pr); line("NT3", 0x2C00, 0x400, pr)
  line("PAL", 0x3F00, 0x20, pr); line("CHR", 0x0000, 0x2000, pr); line("OAM", 0x200, 0x100, rd)
  out:write(string.format("REG scene=%02X cursor=%02X a0=%02X w131=%02X ff=%02X scroll=%02X,%02X p95=%02X a1=%02X a2=%02X b2b=%02X\n", rd(0x22), rd(0x138), rd(0xA0), rd(0x131), rd(0xFF), rd(0xFC), rd(0xFA), rd(0x95), rd(0xA1), rd(0xA2), rd(0x2B)))
  local e = {} for a = 0xB0, 0xBF do e[#e + 1] = string.format("%02X", rd(a)) end
  out:write("ENE " .. table.concat(e, " ") .. "\n")
  out:close()
end
emu.addEventCallback(function()
  local s = rd(0x22)
  local prev = last
  if s ~= last then last = s; if s < 0x10 and prev >= 0x10 and not stageFrame then stageFrame = frame end end
  if stageFrame then
    local t = frame - stageFrame
    if t == 690 then local m = emu.memType.nesMemory; emu.write(0xB2, 0x9C, m); emu.write(0xB4, 0x90, m); emu.write(0xB7, 0x85, m); emu.write(0xB9, 0x9C, m); emu.write(0xBB, 0x8E, m); emu.write(0xBD, 0x83, m); emu.write(0xA2, 3, m); emu.write(0xB0, 0x95, m) end
    if t >= 700 and not pauseAt and t == 800 then pauseAt = frame end
    if pauseAt then
      local n = frame - pauseAt
      if DUMPS[n] then dump(n) end
      if n > 170 then emu.stop(0) end
    end
  end
  frame = frame + 1
end, emu.eventType.endFrame)
emu.addEventCallback(function()
  if pauseAt then
    local n = frame - pauseAt
    if n >= 0 and n < 3 then emu.setInput({start = true}, 0)
    elseif n == 50 then emu.setInput({down = true}, 0) elseif n == 80 then emu.setInput({down = true}, 0)
    elseif n == 110 then emu.setInput({right = true}, 0) elseif n == 140 then emu.setInput({down = true}, 0)
    else emu.setInput({}, 0) end
  elseif stageFrame then
    local t = frame - stageFrame
    if t >= 800 and t < 803 then emu.setInput({start = true}, 0) else emu.setInput({}, 0) end
  else emu.setInput(menuInput(frame), 0) end
end, emu.eventType.inputPolled)
