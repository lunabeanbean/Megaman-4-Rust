// Render a whole stage from the ROM with its real graphics (no emulator needed).
//   node tools/render_stage.js <rom.nes> <stage> <out.png> [screens]
// Uses: stage bank tables (notes/level-format.md), the graphics loader emulation (tools/gfx.js) and the BG palette at $B590.
'use strict';
const fs = require('fs');
const { NES_RGB, writePng, decodeTile } = require('./nes.js');
const { loadSet } = require('./gfx.js');
const [romPath, stageArg, out, screensArg] = process.argv.slice(2);
const rom = fs.readFileSync(romPath), stage = +stageArg;
const bank = rom.subarray(0x10 + (0x20 | stage) * 0x2000, 0x10 + (0x20 | stage) * 0x2000 + 0x2000);
const A = a => a - 0xA000;

const chr = Buffer.alloc(0x2000);
loadSet(rom, stage, chr);                       // set id = stage id (stage start loads set $22 then $47 if nonzero)

let screens = screensArg ? +screensArg : 0;
if (!screens) { screens = 1; while (screens < 48 && bank[A(0xB500) + screens] === bank[A(0xB500) + screens - 1] + 1) screens++; }

const bg = Array.from(bank.subarray(A(0xB590), A(0xB590) + 16));   // 4 palettes x 4 colours (palette set 0)
const W = screens * 256, H = 240, rgb = Buffer.alloc(W * H * 3);
const tileCache = new Map();
const tile = n => { if (!tileCache.has(n)) tileCache.set(n, decodeTile(chr, n * 16)); return tileCache.get(n); };
for (let s = 0; s < screens; s++) {
  const layout = bank[A(0xB500) + s];
  for (let cell = 0; cell < 64; cell++) {
    const block = bank[A(0xA900) + layout * 64 + cell];
    for (let k = 0; k < 4; k++) {
      const m = bank[A(0xA500) + block * 4 + k];
      const pal = bank[A(0xA400) + m] & 3;
      const mx = (cell % 8) * 2 + (k & 1), my = (Math.floor(cell / 8)) * 2 + (k >> 1);
      for (let t = 0; t < 4; t++) {
        const px = tile(bank[A(0xA000) + 0x100 * t + m]);
        const x0 = s * 256 + mx * 16 + (t & 1) * 8, y0 = my * 16 + (t >> 1) * 8;
        if (y0 >= H) continue;
        for (let y = 0; y < 8; y++) for (let x = 0; x < 8; x++) {
          if (y0 + y >= H) continue;
          const c = NES_RGB[bg[pal * 4 + px[y * 8 + x]] & 0x3F], o = ((y0 + y) * W + x0 + x) * 3;
          rgb[o] = c[0]; rgb[o + 1] = c[1]; rgb[o + 2] = c[2];
        }
      }
    }
  }
}
writePng(out, W, H, rgb);
console.log(`wrote ${out} (${W}x${H}, ${screens} screens)`);
