// Render a captured nametable (ppu_nt.bin) with captured CHR (ppu_chr.bin) and palette (ppu_pal.bin) to a PNG.
// usage: node tools/render_ppu.js <dir with ppu_*.bin> <out.png> [bgPatternBase=0]
'use strict';
const fs = require('fs'); const { NES_RGB, writePng, decodeTile } = require('./nes.js');
const dir = process.argv[2], out = process.argv[3], base = parseInt(process.argv[4] || '0', 10);
const chr = fs.readFileSync(dir + '/ppu_chr.bin'), nt = fs.readFileSync(dir + '/ppu_nt.bin'), pal = fs.readFileSync(dir + '/ppu_pal.bin');
const W = 512, H = 480; const rgb = Buffer.alloc(W * H * 3);
for (let n = 0; n < 4; n++) {
  const ox = (n % 2) * 256, oy = Math.floor(n / 2) * 240, ntBase = n * 0x400;
  for (let ty = 0; ty < 30; ty++) for (let tx = 0; tx < 32; tx++) {
    const tile = nt[ntBase + ty * 32 + tx];
    const attr = nt[ntBase + 0x3C0 + (ty >> 2) * 8 + (tx >> 2)];
    const shift = ((ty >> 1) & 1) * 4 + ((tx >> 1) & 1) * 2;
    const p = (attr >> shift) & 3;
    const px = decodeTile(chr, base + tile * 16);
    for (let y = 0; y < 8; y++) for (let x = 0; x < 8; x++) {
      const c = NES_RGB[pal[p * 4 + px[y * 8 + x]] & 0x3F];
      const o = ((oy + ty * 8 + y) * W + ox + tx * 8 + x) * 3; rgb[o] = c[0]; rgb[o + 1] = c[1]; rgb[o + 2] = c[2];
    }
  }
}
writePng(out, W, H, rgb); console.log('wrote ' + out);
