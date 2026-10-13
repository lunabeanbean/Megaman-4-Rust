-- Per-video-frame PPU/RAM recording of the presentation code (menus, title, cutscenes) for the screen ports. Replaces ad-hoc dump scripts.
-- Boots with the title/menu input of gen_ptrace.js (title -> stage select -> stage 2), optionally jumps into the stage-clear code, records DF frames.
-- Env (all optional):
--   PF_OUT      output file name in harness/ (default ppuframes.txt)
--   PF_FRAMES   frames to record (default 600)
--   PF_START    record from this absolute frame (default 0; ignored when PF_HIJACK is set)
--   PF_HIJACK   stage frame at which the stage-clear path is forced (player state $30 = 8, countdown $0133 = 1: the scheduler then calls `JSR $8003`, bank $39); recording starts then
--   PF_STAGE    stage 0-7 to enter (default: the menu script's own pick, stage 2)
--   PF_SHOT     comma list of recording frames at which a screenshot is written (shots_<out>_<frame>.png)
--   PF_STATE    player state forced at the hijack (default 8 = victory; 7 = dead, with PF_A1=0 that is the game over)
--   PF_A9 / PF_AA / PF_A1   hex values poked at that moment (boss-defeated bits, Cossack/Wily bits, lives)
--   PF_POKE     "addr=val,addr=val" (hex) poked at the same moment
--   PF_SEQ      Lua table body of {fromFrame, toFrame, {button=true}} entries, frames relative to the start of the recording (default: no input)
--   PF_FROM     first recording frame that is written (default 0); earlier frames are only played (keeps long recordings from exhausting the emulator's script memory)
--   PF_MENU     "1": keep using the title/menu script input after PF_START when there is no hijack
-- Output: per frame `F n scene=.. x10=.. ...` then `PAL`, `OAM`, `BUF` ($0780..$07BF), `RAM` ($A0..$BF), `NT0..NT3` and `CHR` only when changed.
local DIR = "C:/Users/joele/Desktop/MegaMan4 Project/harness/"
local function env(k, d) local v = os.getenv(k); if v == nil or v == "" then return d end return v end
local OUT = io.open(DIR .. env("PF_OUT", "ppuframes.txt"), "w")
local NFR = tonumber(env("PF_FRAMES", "600"))
local START = tonumber(env("PF_START", "0"))
local FROM = tonumber(env("PF_FROM", "0"))
local HIJACK = tonumber(env("PF_HIJACK", "-1"))
local seq = load("return { " .. env("PF_SEQ", "") .. " }")()
local frame, last, stageFrame, t0 = 0, -1, nil, nil
local function rd(a) return emu.read(a, emu.memType.nesMemory) end
local function pr(a) return emu.read(a, emu.memType.nesPpuMemory) end
local hexs = {} for i = 0, 255 do hexs[i] = string.format("%02X", i) end
local function hx(a0, n, reader) local t = {} for a = a0, a0 + n - 1 do t[#t + 1] = hexs[reader(a)] end return table.concat(t) end
local function menuInput(f)
  local m = f % 160
  if m < 20 then return {start = true} elseif m < 60 then return {} elseif m < 80 then return {a = true}
  elseif m < 100 then return {} elseif m < 110 then return {right = true} elseif m < 130 then return {}
  elseif m < 140 then return {down = true} else return {} end
end
local STG = tonumber(env("PF_STAGE", "-1"))
-- PF_STAGE: stage (0-7) the menu script picks: the cursor is moved to its cell when the select loop reads Start
if STG >= 0 then
  emu.addMemoryCallback(function()
    local idx = ({[4]=0,[6]=1,[7]=2,[3]=3,[8]=4,[0]=5,[1]=6,[2]=7,[5]=8})[STG] or 5
    emu.write(0x10, idx % 3, emu.memType.nesMemory); emu.write(0x11, (idx // 3) * 3, emu.memType.nesMemory)
  end, emu.callbackType.exec, 0x720D2, 0x720D2, emu.cpuType.nes, emu.memType.nesPrgRom)
end
local SHOTS = {}
for n in (env("PF_SHOT", "")):gmatch("%d+") do SHOTS[tonumber(n)] = true end
local lastnt, lastchr = {}, ""
local hijacked = false
local function poke(a, v) emu.write(a, v, emu.memType.nesMemory) end
local function doPokes()
  local a9, aa, a1 = os.getenv("PF_A9"), os.getenv("PF_AA"), os.getenv("PF_A1")
  if a9 and a9 ~= "" then poke(0xA9, tonumber(a9, 16)) end
  if aa and aa ~= "" then poke(0xAA, tonumber(aa, 16)) end
  if a1 and a1 ~= "" then poke(0xA1, tonumber(a1, 16)) end
  local p = os.getenv("PF_POKE")
  if p and p ~= "" then for a, v in p:gmatch("(%x+)=(%x+)") do poke(tonumber(a, 16), tonumber(v, 16)) end end
end
local function dump(df)
  OUT:write(string.format("F %d scene=%02X x10=%02X x11=%02X a9=%02X aa=%02X a1=%02X p95=%02X p92=%02X p24=%02X p2a=%02X p97=%02X x18=%02X x19=%02X x64=%02X x14=%02X x16=%02X f5=%02X f6=%02X fc=%02X fa=%02X fd=%02X f9=%02X ff=%02X f0=%02X f1=%02X x69=%02X x6a=%02X x9b=%02X\n",
    df, rd(0x22), rd(0x10), rd(0x11), rd(0xA9), rd(0xAA), rd(0xA1), rd(0x95), rd(0x92), rd(0x24), rd(0x2A), rd(0x97), rd(0x18), rd(0x19), rd(0x64), rd(0x14), rd(0x16), rd(0xF5), rd(0xF6), rd(0xFC), rd(0xFA), rd(0xFD), rd(0xF9), rd(0xFF), rd(0xF0), rd(0xF1), rd(0x69), rd(0x6A), rd(0x9B)))
  OUT:write("PAL " .. hx(0x3F00, 0x20, pr) .. "\n")
  OUT:write("OAM " .. hx(0x200, 0x100, rd) .. "\n")
  OUT:write("BUF " .. hx(0x780, 0x40, rd) .. "\n")
  OUT:write("RAM " .. hx(0xA0, 0x20, rd) .. "\n")
  -- the first four object slots as the scene scripts keep them: type, flags, x page/px/sub, y page/px/sub, animation id/frame/tick, $05B8, speeds, $0468.. registers
  do
    local t = {}
    for x = 0, 3 do
      for _, a in ipairs({0x300, 0x528, 0x348, 0x330, 0x318, 0x390, 0x378, 0x360, 0x558, 0x540, 0x570, 0x5B8, 0x3A8, 0x3C0, 0x3D8, 0x3F0, 0x468, 0x480, 0x498, 0x4B0, 0x4C8, 0x4E0, 0x4F8, 0x510}) do t[#t + 1] = hexs[rd(a + x)] end
    end
    OUT:write("OBJ " .. table.concat(t) .. "\n")
    OUT:write("YSUB " .. hx(0x360, 24, rd) .. "\n")
  end
  for i = 0, 3 do local nt = hx(0x2000 + i * 0x400, 0x400, pr); if nt ~= lastnt[i] then OUT:write("NT" .. i .. " " .. nt .. "\n"); lastnt[i] = nt end end
  local c = hx(0, 0x2000, pr); if c ~= lastchr then OUT:write("CHR " .. c .. "\n"); lastchr = c end
end
-- the jump: at the end of a logic frame (after the object draw pass) the state the game has after a boss victory is faked
emu.addMemoryCallback(function()
  if HIJACK >= 0 and stageFrame and not hijacked and frame - stageFrame >= HIJACK then
    hijacked = true
    doPokes()
    -- player state 8 with the countdown at 1: the scheduler (code_3EC7C6) takes the stage-clear path (`JSR $8003`) at the end of this logic frame
    poke(0x30, tonumber(env("PF_STATE", "8"))); poke(0x133, 1); poke(0x134, 0)
    t0 = frame
  end
end, emu.callbackType.exec, 0xC7C6)
local SAVE, LOAD = tonumber(env('PF_SAVE', '-1')), env('PF_LOAD', '')
local loaded, pendingSave, nmiHooked = false, nil, false
local function hookNmi()
  if nmiHooked then return end
  nmiHooked = true
  local nmi = rd(0xFFFA) + 256 * rd(0xFFFB)
  emu.addMemoryCallback(function()
    if LOAD ~= '' and not loaded then
      loaded = true
      local f = io.open(DIR .. LOAD .. '.state', 'rb'); local st = f:read('*a'); f:close()
      local m = io.open(DIR .. LOAD .. '.meta', 'r'); local fr, tt, sf, la = m:read('*n', '*n', '*n', '*n'); m:close()
      emu.loadSavestate(st)
      frame, t0, stageFrame, last, hijacked = fr + 1, tt, sf, la, true
    elseif pendingSave then
      local f = io.open(DIR .. pendingSave.name .. '.state', 'wb'); f:write(emu.createSavestate()); f:close()
      local m = io.open(DIR .. pendingSave.name .. '.meta', 'w'); m:write(pendingSave.meta); m:close()
      pendingSave = nil
      emu.stop(0)
    end
  end, emu.callbackType.exec, nmi)
end
emu.addEventCallback(function()
  hookNmi()
  if LOAD ~= '' and not loaded then return end
  local s = rd(0x22)
  local prev = last
  if s ~= last then last = s; if s < 0x10 and prev >= 0x10 and not stageFrame then stageFrame = frame end end
  if HIJACK < 0 and not t0 and frame >= START then t0 = frame; doPokes() end
  if t0 and frame >= t0 then
    local df = frame - t0
    if df >= FROM then dump(df); OUT:flush() end
    if SHOTS[df] then local g = io.open(DIR .. "shots_" .. env("PF_OUT", "ppuframes.txt"):gsub("%.txt$", "") .. "_" .. df .. ".png", "wb"); g:write(emu.takeScreenshot()); g:close() end
    if df == SAVE then pendingSave = {name = env('PF_SAVEAS', 'pf_save'), meta = frame .. ' ' .. t0 .. ' ' .. tostring(stageFrame) .. ' ' .. last .. ' '} end
    if df >= NFR and not pendingSave then OUT:close(); emu.stop(0) end
  end
  if frame % 500 == 0 then local g = io.open(DIR .. "pf_progress.txt", "a"); g:write(frame .. " t0=" .. tostring(t0) .. " scene=" .. rd(0x22) .. " " .. collectgarbage("count") .. "\n"); g:close() end
  frame = frame + 1
end, emu.eventType.endFrame)
emu.addEventCallback(function()
  if t0 and (HIJACK >= 0 or os.getenv("PF_MENU") ~= "1") then
    local df = frame - t0
    local b = {}
    for _, s in ipairs(seq) do if df >= s[1] and df < s[2] then b = s[3] end end
    emu.setInput(b, 0)
  elseif stageFrame then emu.setInput({}, 0)
  else emu.setInput(menuInput(frame), 0) end
end, emu.eventType.inputPolled)
