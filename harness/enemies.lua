-- Verify enemy placement format: at a few frames while running right, list live enemy slots ($08-$17)
local OUT = "C:/Users/joele/Desktop/MegaMan4 Project/harness/enemies_out.txt"
local out = io.open(OUT, "w")
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
  if stageFrame then
    local t = frame - stageFrame
    if t >= 700 and t % 30 == 0 then
      out:write(string.format("t=%d cam=%02X:%02X player=%02X:%02X\n", t, rd(0xF9), rd(0xFC), rd(0x348), rd(0x330)))
      for x = 8, 23 do
        local ty = rd(0x300 + x)
        if ty ~= 0 then
          out:write(string.format("  slot %02d type=%02X listIdx=%02X pos=%02X:%02X y=%02X flags=%02X\n", x, ty, rd(0x438 + x), rd(0x348 + x), rd(0x330 + x), rd(0x378 + x), rd(0x528 + x)))
        end
      end
    end
    if t > 1500 then out:close(); emu.stop(0) end
  end
end, emu.eventType.endFrame)
emu.addEventCallback(function()
  if stageFrame then
    local t = frame - stageFrame
    emu.setInput((t >= 700) and {right = true} or {}, 0)
  else emu.setInput(menuInput(frame), 0) end
end, emu.eventType.inputPolled)
