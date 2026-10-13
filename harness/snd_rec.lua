-- Records the sound driver (bank $1E) the way the console runs it: every call of the per-frame update ($8000), every play request ($8003, A = id, Y) and every APU register write,
-- in the order they happen, from power-on with a button script. Ids can also be poked into the game's request queue ($DA/$DC..$E3) at chosen frames (a jukebox).
-- Env: SND_OUT (log file), SND_FRAMES, SND_SCRIPT (lines "FROM TO BUTTONS" like chain_*.txt), SND_SEQ ("frame:id,frame:id,..." hex ids), SND_STAGE (poke $22/state is not done here)
-- Log lines: "T frame" | "P frame A Y" | "W frame addr value" | "F frame" (end of frame, with $22 $95) ; numbers decimal frames, hex bytes.
local function env(k, d) local v = os.getenv(k); if v == nil or v == "" then return d end return v end
local OUT = env("SND_OUT", "C:/Users/joele/Desktop/MegaMan4 Project/harness/snd_log.txt")
local NFR = tonumber(env("SND_FRAMES", "600"))
local log = io.open(OUT, "w")
local spans = {}
local sc = env("SND_SCRIPT", "")
if sc ~= "" then
  for l in io.lines(sc) do
    local a, b, btn = l:match("^(%d+)%s+(%d+)%s+(%a+)")
    if a then spans[#spans + 1] = {tonumber(a), tonumber(b), btn} end
  end
end
local seq = {}
for f, id in env("SND_SEQ", ""):gmatch("(%d+):(%x+)") do seq[#seq + 1] = {tonumber(f), tonumber(id, 16)} end
local frame = 0
local function rd(a) return emu.read(a, emu.memType.nesMemory) end
local function wr(a, v) emu.write(a, v, emu.memType.nesMemory) end
-- bank $1E at $8000: PRG offset $1E*$2000 = $3C000
emu.addMemoryCallback(function()
  log:write("T " .. frame .. string.char(10))
end, emu.callbackType.exec, 0x3C000, 0x3C000, emu.cpuType.nes, emu.memType.nesPrgRom)
emu.addMemoryCallback(function()
  local s = emu.getState()
  log:write(string.format("P %d %02X %02X", frame, s["cpu.a"], s["cpu.y"]) .. string.char(10))
end, emu.callbackType.exec, 0x3C003, 0x3C003, emu.cpuType.nes, emu.memType.nesPrgRom)
-- only writes made by the driver itself (code in bank $1E at $8000-$9FFF); the game's own writes ($4014 DMA, $4016 pad, init of $4015/$4017) are not logged
emu.addMemoryCallback(function(addr, v)
  local pc = emu.getState()["cpu.pc"]
  if pc >= 0x8000 and pc < 0xA000 then
    log:write(string.format("W %d %04X %02X", frame, addr, v) .. string.char(10))
  end
end, emu.callbackType.write, 0x4000, 0x4017)
emu.addEventCallback(function()
  for _, e in ipairs(seq) do
    if e[1] == frame then
      local x = rd(0xDA)
      if rd(0xDC + x) == 0x88 then wr(0xDC + x, e[2]); wr(0xDA, (x + 1) & 7) end
    end
  end
  log:write(string.format("F %d %02X %02X", frame, rd(0x22), rd(0x95)) .. string.char(10))
  if env("SND_STATE", "") ~= "" then
    local s = emu.getState()
    local function g(k) return tonumber(s[k]) or 0 end
    -- channel state at the end of the frame (before the NMI's driver update): volume, length counter, period (pulse/noise) / linear counter (triangle)
    log:write(string.format("S %d p1 %d %d %d p2 %d %d %d t %d %d %d n %d %d %d", frame,
      g("apu.square1.envelope.volume"), g("apu.square1.envelope.lengthCounter.counter"), g("apu.square1.realPeriod"),
      g("apu.square2.envelope.volume"), g("apu.square2.envelope.lengthCounter.counter"), g("apu.square2.realPeriod"),
      g("apu.triangle.lengthCounter.counter"), g("apu.triangle.linearCounter"), g("apu.triangle.timer.period"),
      g("apu.noise.envelope.volume"), g("apu.noise.envelope.lengthCounter.counter"), g("apu.noise.timer.period")) .. string.char(10))
  end
  if frame % 200 == 0 then log:flush() end
  frame = frame + 1
  if frame >= NFR then log:close(); emu.stop(0) end
end, emu.eventType.endFrame)
emu.addEventCallback(function()
  local b = {}
  for _, s in ipairs(spans) do
    if frame >= s[1] and frame < s[2] then
      for c in s[3]:gmatch(".") do
        if c == "R" then b.right = true elseif c == "L" then b.left = true elseif c == "D" then b.down = true elseif c == "U" then b.up = true
        elseif c == "A" then b.a = true elseif c == "B" then b.b = true elseif c == "S" then b.start = true elseif c == "T" then b.select = true end
      end
    end
  end
  emu.setInput(b, 0)
end, emu.eventType.inputPolled)
