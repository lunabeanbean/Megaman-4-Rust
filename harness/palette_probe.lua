-- Reads Mesen's colours for the 64 NES palette indices: on the title screen the backdrop ($3F00) is set to every index in turn and the screenshot's corner pixel is saved.
-- Output harness/mesen_palette_shots/<n>.png: the picture the emulator hands out lags one frame, so the colour of palette index i is in shot i+1.
local DIR = "C:/Users/joele/Desktop/MegaMan4 Project/harness/mesen_palette_shots/"
local frame, idx = 0, -1
local function rd(a) return emu.read(a, emu.memType.nesMemory) end
local function menuInput(f)
  if f >= 1300 and f < 1304 then return {start = true} end
  return {}
end
emu.addEventCallback(function()
  if frame >= 1500 then
    if idx >= 0 and idx <= 64 then
      local g = io.open(DIR .. string.format("%02d", idx) .. ".png", "wb"); g:write(emu.takeScreenshot()); g:close()
    end
    idx = idx + 1
    if idx > 64 then emu.stop(0) end
    -- the palette entry the next frame will show (all four mirrors of the backdrop)
    for _, a in ipairs({0x3F00, 0x3F10}) do emu.write(a, idx, emu.memType.nesPpuMemory) end
  end
  frame = frame + 1
end, emu.eventType.endFrame)
emu.addEventCallback(function() emu.setInput(menuInput(frame), 0) end, emu.eventType.inputPolled)
