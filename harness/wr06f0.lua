-- logs every write to $06F0-$06F3 with the PC (which code sets the palette-animation ids), using the chain script
local DIR = "C:/Users/joele/Desktop/MegaMan4 Project/harness/"
local NFR = tonumber(os.getenv("CH_FRAMES") or "2300")
local spans = {}
for l in io.lines(os.getenv("CH_SCRIPT") or (DIR .. "chain_p.txt")) do
  local a, b, btn = l:match("^(%d+)%s+(%d+)%s+(%a+)")
  if a then spans[#spans + 1] = {tonumber(a), tonumber(b), btn} end
end
local log = io.open(DIR .. "wr06f0.log", "w")
local frame = 0
emu.addMemoryCallback(function(addr, value)
  local s = emu.getState()
  log:write(string.format("%d %04X <- %02X pc=%04X bank=%02X\n", frame, addr, value, s["cpu.pc"] or 0, 0)); log:flush()
end, emu.callbackType.write, 0x06F0, 0x06F3)
emu.addEventCallback(function() frame = frame + 1; if frame >= NFR then log:close(); emu.stop(0) end end, emu.eventType.endFrame)
emu.addEventCallback(function()
  local b = {}
  for _, s in ipairs(spans) do
    if frame >= s[1] and frame < s[2] then
      for c in s[3]:gmatch(".") do
        if c == "R" then b.right = true elseif c == "L" then b.left = true elseif c == "D" then b.down = true elseif c == "U" then b.up = true
        elseif c == "A" then b.a = true elseif c == "B" then b.b = true elseif c == "S" then b.start = true end
      end
    end
  end
  emu.setInput(b, 0)
end, emu.eventType.inputPolled)
