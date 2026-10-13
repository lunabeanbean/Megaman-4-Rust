-- Records a scripted play-through from power-on for the chain check: screenshots of chosen frames + a log line per frame.
-- Env: CH_SCRIPT (file: lines "FROM TO BUTTONS", BUTTONS from RLDUABST, frames = console frames), CH_FRAMES, CH_SHOTS ("a-b"), CH_DIR (folder under harness/),
--      CH_RAM (hex addresses logged per frame), CH_PAL=1 (also the PPU palette), CH_HIJACK=frame (force the stage clear then: $30 = CH_STATE (8), $0133 = 1),
--      CH_POKE="addr=val,..." (poked at the hijack), CH_LOAD=name / CH_SAVE=frame:name (savestate chaining like ppu_frames.lua)
-- Log line: frame $22 $10 $95 $92 $30 <CH_RAM values> [palette]
local DIR = "C:/Users/joele/Desktop/MegaMan4 Project/harness/"
local function env(k, d) local v = os.getenv(k); if v == nil or v == "" then return d end return v end
local OUT = env("CH_DIR", "chain") .. "/"
local NFR = tonumber(env("CH_FRAMES", "1000"))
local sa, sb = env("CH_SHOTS", "0-0"):match("(%d+)-(%d+)"); sa, sb = tonumber(sa), tonumber(sb)
local spans = {}
for l in io.lines(env("CH_SCRIPT", DIR .. "chain_a.txt")) do
  local a, b, btn = l:match("^(%d+)%s+(%d+)%s+(%a+)")
  if a then spans[#spans + 1] = {tonumber(a), tonumber(b), btn} end
end
local log = io.open(DIR .. OUT .. "mesen.log", "w")
local RAMS = {}
for h in env("CH_RAM", "14,16,F0,69,FC,64,18,9A"):gmatch("%x+") do RAMS[#RAMS + 1] = tonumber(h, 16) end
local PAL = env("CH_PAL", "") ~= ""
local CHR = env("CH_CHR", "") ~= ""
local function chrhash(a0)
  local h = 5381
  for a = a0, a0 + 0xFFF do h = (h * 33 + emu.read(a, emu.memType.nesPpuMemory)) % 4294967296 end
  return string.format("%08X", h)
end
local HIJ = tonumber(env("CH_HIJACK", "-1"))
local hijacked = false
local frame = 0
-- sound requests (sndreq_snippet.lua): one line per call of code_3FF6BC / code_3FF6BE, "<frame> <C|E> <id> <return address>"
SNDOUT = io.open(DIR .. OUT .. "snd.log", "w")
function SNDTAG() return frame end
dofile(DIR .. "sndreq_snippet.lua")
local function rd(a) return emu.read(a, emu.memType.nesMemory) end
local function poke(a, v) emu.write(a, v, emu.memType.nesMemory) end
emu.addMemoryCallback(function()
  if HIJ >= 0 and not hijacked and frame >= HIJ and rd(0x22) < 0x10 then
    hijacked = true
    poke(0x30, tonumber(env("CH_STATE", "8"))); poke(0x133, 1); poke(0x134, 0)
    local p = os.getenv("CH_POKE")
    if p and p ~= "" then for a, v in p:gmatch("(%x+)=(%x+)") do poke(tonumber(a, 16), tonumber(v, 16)) end end
    log:write("# hijack at frame " .. frame .. string.char(10))
  end
  local kp = os.getenv("CH_KEEP")
  if kp and kp ~= "" and frame >= tonumber(env("CH_KEEPFROM", "0")) then for a, v in kp:gmatch("(%x+)=(%x+)") do poke(tonumber(a, 16), tonumber(v, 16)) end end
end, emu.callbackType.exec, 0xC7C6)
local EARLY = {}
for f, a, v in env("CH_EARLY", ""):gmatch("(%d+):(%x+)=(%x+)") do EARLY[#EARLY + 1] = {tonumber(f), tonumber(a, 16), tonumber(v, 16)} end
emu.addEventCallback(function()
  for _, e in ipairs(EARLY) do if frame == e[1] then poke(e[2], e[3]) end end
  if frame >= sa and frame <= sb then
    local g = io.open(DIR .. OUT .. "sh_" .. frame .. ".png", "wb"); g:write(emu.takeScreenshot()); g:close()
  end
  if frame == tonumber(env("CH_CHRDUMP", "-1")) then
    local g = io.open(DIR .. OUT .. "chr.bin", "wb")
    for a = 0, 0x1FFF do g:write(string.char(emu.read(a, emu.memType.nesPpuMemory))) end
    g:close()
    local g2 = io.open(DIR .. OUT .. "nt.bin", "wb")
    for a = 0x2000, 0x2FFF do g2:write(string.char(emu.read(a, emu.memType.nesPpuMemory))) end
    g2:close()
  end
  local extra = ""
  for _, a in ipairs(RAMS) do extra = extra .. string.format(" %02X", rd(a)) end
  if CHR then extra = extra .. " " .. chrhash(0x0000) .. chrhash(0x1000) end
  if PAL then extra = extra .. " "; for a = 0x3F00, 0x3F1F do extra = extra .. string.format("%02X", emu.read(a, emu.memType.nesPpuMemory)) end end
  log:write(string.format("%d %02X %02X %02X %02X %02X", frame, rd(0x22), rd(0x10), rd(0x95), rd(0x92), rd(0x30)) .. extra .. string.char(10))
  if frame % 200 == 0 then log:flush() end
  frame = frame + 1
  if frame % 200 == 0 then SNDOUT:flush() end
  if frame >= NFR then log:close(); SNDOUT:close(); emu.stop(0) end
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
