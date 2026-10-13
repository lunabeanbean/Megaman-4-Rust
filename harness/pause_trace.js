// Pause-menu recorder: node pause_trace.js <seed> [frames=3000]  -> pausetrace_<seed>.lua; run it with rt.ps1 on mm4_noenemy.nes, it writes pauselog_<seed>.txt
// Boots to stage 2, pokes a random set of weapon slots (and E-tanks), then taps random buttons; Start opens/closes the pause menu.
// One row per pause-loop pass (exec breakpoint at $3C96D7): P <$14> <$0138> <$A0> <$0131> <$B0..$BF> <$A2> <$AF>
const fs = require('fs');
const seed = parseInt(process.argv[2], 10), frames = parseInt(process.argv[3] || '3000', 10);
let s = seed >>> 0;
const rnd = () => { s = (s + 0x6D2B79F5) >>> 0; let t = s; t = Math.imul(t ^ (t >>> 15), t | 1); t ^= t + Math.imul(t ^ (t >>> 7), t | 61); return ((t ^ (t >>> 14)) >>> 0) / 4294967296; };
const dir = __dirname.split(String.fromCharCode(92)).join('/');
const own = [];
for (let w = 1; w < 14; w++) if (rnd() < (parseFloat(process.env.OWN || '0.6'))) own.push(`emu.write(0x${(0xB0 + w).toString(16)}, 0x${(0x80 + Math.floor(rnd() * 28)).toString(16)}, m)`);
const etanks = Math.floor(rnd() * 4);
const lua = `
local out = io.open("${dir}/pauselog_${seed}.txt", "w")
local function rd(a) return emu.read(a, emu.memType.nesMemory) end
local frame, last, stageFrame = 0, -1, nil
local rows = 0
local WAIT, FRAMES = 700, ${frames}
local function menuInput(f)
  local m = f % 160
  if m < 20 then return {start = true} elseif m < 60 then return {} elseif m < 80 then return {a = true}
  elseif m < 100 then return {} elseif m < 110 then return {right = true} elseif m < 130 then return {}
  elseif m < 140 then return {down = true} else return {} end
end
-- random taps, decided per 6-frame block from a seeded generator
local lcg = ${seed * 7919 + 13}
local function r(n) lcg = (lcg * 1103515245 + 12345) % 2147483648; return (lcg >> 8) % n end
local keys = {}
local function pauseInput(n)
  local k = n // 6
  if keys[k] == nil then
    local x = r(20)
    keys[k] = (x < 4) and 'none' or (x < 6) and 'right' or (x < 8) and 'left' or (x < 11) and 'down' or (x < 14) and 'up' or (x < 15) and 'a' or (x < 16) and 'start' or 'none'
    if r(8) == 0 and keys[k] ~= 'none' then keys[k] = keys[k] end
  end
  if n % 6 < 2 and keys[k] ~= 'none' then return {[keys[k]] = true} end
  return {}
end
emu.addMemoryCallback(function()
  if not stageFrame then return end
  local r2 = {}
  for a = 0xB0, 0xBF do r2[#r2 + 1] = string.format("%02X", rd(a)) end
  out:write(string.format("P %02X %02X %02X %02X %s %02X %02X\\n", rd(0x14), rd(0x138), rd(0xA0), rd(0x131), table.concat(r2, " "), rd(0xA2), rd(0xAF)))
  out:flush(); rows = rows + 1
end, emu.callbackType.exec, 0x796D7, 0x796D7, emu.cpuType.nes, emu.memType.nesPrgRom)
emu.addEventCallback(function()
  local s2 = rd(0x22)
  local prev = last
  if s2 ~= last then last = s2; if s2 < 0x10 and prev >= 0x10 and not stageFrame then stageFrame = frame end end
  if stageFrame then
    local t = frame - stageFrame
    if t == WAIT - 10 then local m = emu.memType.nesMemory; ${own.join('; ')}; emu.write(0xA2, ${etanks}, m); emu.write(0xB0, 0x80 + 10 + ${Math.floor(rnd() * 18)}, m) end
    if t >= WAIT + FRAMES then out:close(); emu.stop(0) end
  end
  frame = frame + 1
end, emu.eventType.endFrame)
local startAt = {}
emu.addEventCallback(function()
  if stageFrame and frame - stageFrame >= WAIT then
    local n = frame - stageFrame - WAIT
    -- Start taps open the menu now and then; inside the menu random taps apply (rd(0x30) ~ the game paused leaves the stage loop at $96D7)
    local inMenu = rd(0x22) == 0x10
    if inMenu then emu.setInput(pauseInput(n), 0)
    else emu.setInput((n % 90) < 3 and {start = true} or {}, 0) end
  elseif stageFrame then emu.setInput({}, 0) else emu.setInput(menuInput(frame), 0) end
end, emu.eventType.inputPolled)
`;
fs.writeFileSync(`${__dirname}/pausetrace_${seed}.lua`, lua);
console.log(`wrote pausetrace_${seed}.lua`);
