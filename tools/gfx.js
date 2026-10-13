// Emulates the game's graphics loader (code_3EDA05, bank38_3F.asm line 22548) to build CHR-RAM contents from the ROM.
// Mega Man 4 has no CHR ROM: tiles live in PRG banks and the game copies them into CHR-RAM ($0000-$1FFF of PPU space).
//
// Graphics set `id` is described by a script in bank $28 (mapped at $A000): pointer low = $B800+id, high = $B870+id.
// Script format: up to 8 palette bytes (a byte >= $80 means "no palette"), then entries
//   [bank, pages, srcHigh, ppuHigh]   copies pages*256 bytes from CPU ($srcHigh00) with banks (bank, bank+1) mapped at $8000/$A000
//   into PPU address $ppuHigh00, repeated until the next entry starts with a byte >= $80.
'use strict';
const HEADER = 0x10, BANK = 0x2000;

function cpuByte(rom, banks, addr) {      // banks = {8000: bankNo, A000: bankNo}; C000-FFFF fixed to banks $3E/$3F
  if (addr >= 0x8000 && addr < 0xA000) return rom[HEADER + banks.lo * BANK + (addr - 0x8000)];
  if (addr >= 0xA000 && addr < 0xC000) return rom[HEADER + banks.hi * BANK + (addr - 0xA000)];
  if (addr >= 0xC000 && addr < 0xE000) return rom[HEADER + 0x3E * BANK + (addr - 0xC000)];
  return rom[HEADER + 0x3F * BANK + (addr - 0xE000)];
}

// Applies graphics set `id` to `chr` (Buffer of 0x2000, modified in place). Returns {palette: [...], writes: [[ppuStart, length, bank, src]]}
function loadSet(rom, id, chr) {
  const lo = cpuByte(rom, { lo: 0x28, hi: 0x28 }, 0xB800 + id), hi = cpuByte(rom, { lo: 0x28, hi: 0x28 }, 0xB870 + id);
  let p = (hi << 8) | lo;
  const b28 = { lo: 0x28, hi: 0x29 };    // the game maps bank $28 at $8000 and $29 at $A000 here (F6=$28 only affects $A000: script lives at $B800)
  const script = a => cpuByte(rom, { lo: 0x28, hi: 0x28 }, a);
  const palette = [];
  let y = 0;
  while (true) {                            // palette bytes
    const v = script(p + y);
    if (v >= 0x80) { y++; break; }
    palette.push(v); y++;
    if (y === 8) break;
  }
  const writes = [];
  while (true) {
    const bank = script(p + y);
    if (bank >= 0x80) break;
    const pages = script(p + y + 1), srcHigh = script(p + y + 2), ppuHigh = script(p + y + 3);
    y += 4;
    const dest = ppuHigh << 8, len = pages * 256;
    for (let i = 0; i < len; i++) {
      const addr = (srcHigh << 8) + i;
      chr[(dest + i) & 0x1FFF] = cpuByte(rom, { lo: bank, hi: bank + 1 }, addr);
    }
    writes.push([dest, len, bank, srcHigh << 8]);
  }
  return { palette, writes };
}
module.exports = { loadSet, cpuByte };

if (require.main === module) {
  const fs = require('fs');
  const rom = fs.readFileSync(process.argv[2]);
  const ids = process.argv.slice(3).map(Number);
  const chr = Buffer.alloc(0x2000);
  for (const id of ids) console.log('set', id, JSON.stringify(loadSet(rom, id, chr), (k, v) => v, 0));
  if (process.env.OUT) fs.writeFileSync(process.env.OUT, chr);
}
