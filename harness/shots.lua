-- Screenshots of the real game every 20 frames while the title/menu input of gen_ptrace.js runs (to find which frame shows which screen).
local DIR = "C:/Users/joele/Desktop/MegaMan4 Project/harness/shots/"
local frame = 0
local function rd(a) return emu.read(a, emu.memType.nesMemory) end
local function menuInput(f)
  local m = f % 160
  if m < 20 then return {start = true} elseif m < 60 then return {} elseif m < 80 then return {a = true}
  elseif m < 100 then return {} elseif m < 110 then return {right = true} elseif m < 130 then return {}
  elseif m < 140 then return {down = true} else return {} end
end
emu.addEventCallback(function()
  if frame % 20 == 0 and frame <= 900 then
    local f = io.open(DIR .. string.format("s%04d_%02X.png", frame, rd(0x22)), "wb"); f:write(emu.takeScreenshot()); f:close()
  end
  if frame >= 900 then emu.stop(0) end
  frame = frame + 1
end, emu.eventType.endFrame)
emu.addEventCallback(function() emu.setInput(menuInput(frame), 0) end, emu.eventType.inputPolled)
