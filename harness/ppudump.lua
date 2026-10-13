-- Dump PPU state (CHR window, palettes, nametables) and mapper registers a while into stage 2, for graphics decoding.
local D = "C:/Users/joele/Desktop/MegaMan4 Project/harness/"
local frame, last, stageFrame = 0, -1, nil
local function rd(a) return emu.read(a, emu.memType.nesMemory) end
local function menuInput(f)
  local m = f % 160
  if m < 20 then return {start = true} elseif m < 60 then return {} elseif m < 80 then return {a = true}
  elseif m < 100 then return {} elseif m < 110 then return {right = true} elseif m < 130 then return {}
  elseif m < 140 then return {down = true} else return {} end
end
emu.addEventCallback(function()
  local s = rd(0x22)
  local prev = last
  if s ~= last then last = s; if s < 0x10 and prev >= 0x10 then stageFrame = frame end end
  frame = frame + 1
  if stageFrame and frame - stageFrame == 600 then
    local f = io.open(D .. "ppu_chr.bin", "wb")
    for a = 0, 0x1FFF do f:write(string.char(emu.read(a, emu.memType.nesPpuMemory))) end
    f:close()
    f = io.open(D .. "ppu_nt.bin", "wb")
    for a = 0x2000, 0x2FFF do f:write(string.char(emu.read(a, emu.memType.nesPpuMemory))) end
    f:close()
    f = io.open(D .. "ppu_pal.bin", "wb")
    for a = 0x3F00, 0x3F1F do f:write(string.char(emu.read(a, emu.memType.nesPpuMemory))) end
    f:close()
    local info = io.open(D .. "ppu_info.txt", "w")
    local st = emu.getState()
    for k, v in pairs(st) do
      local ks = tostring(k)
      if ks:lower():find("chr") or ks:lower():find("mmc3") or ks:lower():find("reg") or ks:lower():find("bank") then info:write(ks .. " = " .. tostring(v) .. "\n") end
    end
    info:write(string.format("RAM 06F0..06F3 = %02X %02X %02X %02X ; 0600..061F:", rd(0x6F0), rd(0x6F1), rd(0x6F2), rd(0x6F3)))
    for i = 0, 31 do info:write(string.format(" %02X", rd(0x600 + i))) end
    info:write(string.format("\nvars 45=%02X 46=%02X 47=%02X 48=%02X 1F=%02X", rd(0x45), rd(0x46), rd(0x47), rd(0x48), rd(0x1F)))
    info:write(string.format("\nscroll FC=%02X F9=%02X FD=%02X FF=%02X stage=%02X\n", rd(0xFC), rd(0xF9), rd(0xFD), rd(0xFF), rd(0x22)))
    info:close(); emu.stop(0)
  end
end, emu.eventType.endFrame)
emu.addEventCallback(function()
  if stageFrame then emu.setInput({}, 0) else emu.setInput(menuInput(frame), 0) end
end, emu.eventType.inputPolled)
