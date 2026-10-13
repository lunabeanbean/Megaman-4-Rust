// Render a PPU dump written by harness/ppu_dump.lua (NT0..NT3, PAL, CHR, OAM lines) to a PNG: node tools/render_dump.js <dump.txt> <out.png> [bgBase=0] [spriteBase=0x1000]
'use strict';
const fs = require('fs'); const { NES_RGB, writePng, decodeTile } = require('./nes.js');
const [file, out] = process.argv.slice(2); const bgBase = parseInt(process.argv[4] || '0'), spBase = parseInt(process.argv[5] || '4096');
const lines = fs.readFileSync(file, 'utf8').split('\n'); const get = (tag) => { const l = lines.find((x) => x.startsWith(tag + ' ')); return l ? Buffer.from(l.split(' ')[1], 'hex') : null; };
const NTI = process.argv[6] || '0'; const nt = get('NT' + NTI), pal = get('PAL'), chr = get('CHR'), oam = get('OAM');
const W = 256, H = 240, rgb = Buffer.alloc(W * H * 3);
const put = (x, y, c) => { if (x < 0 || y < 0 || x >= W || y >= H) return; const o = (y * W + x) * 3; rgb[o] = c[0]; rgb[o + 1] = c[1]; rgb[o + 2] = c[2]; };
for (let ty = 0; ty < 30; ty++) for (let tx = 0; tx < 32; tx++) {
  const tile = nt[ty * 32 + tx], attr = nt[0x3C0 + (ty >> 2) * 8 + (tx >> 2)], shift = ((ty >> 1) & 1) * 4 + ((tx >> 1) & 1) * 2, p = (attr >> shift) & 3;
  const px = decodeTile(chr, bgBase + tile * 16);
  for (let y = 0; y < 8; y++) for (let x = 0; x < 8; x++) put(tx * 8 + x, ty * 8 + y, NES_RGB[pal[(px[y * 8 + x] ? p * 4 + px[y * 8 + x] : 0)] & 0x3F]);
}
for (let i = 63; i >= 0; i--) {
  const y = oam[i * 4] + 1, tile = oam[i * 4 + 1], at = oam[i * 4 + 2], x = oam[i * 4 + 3]; if (oam[i * 4] >= 0xEF) continue;
  const px = decodeTile(chr, spBase + tile * 16), p = at & 3;
  for (let yy = 0; yy < 8; yy++) for (let xx = 0; xx < 8; xx++) {
    const sx = at & 0x40 ? 7 - xx : xx, sy = at & 0x80 ? 7 - yy : yy, c = px[sy * 8 + sx]; if (c) put(x + xx, y + yy, NES_RGB[pal[16 + p * 4 + c] & 0x3F]);
  }
}
writePng(out, W, H, rgb); console.log('wrote ' + out);
if (process.env.PPM) require('fs').writeFileSync(process.env.PPM, Buffer.concat([Buffer.from(`P6\n${W} ${H}\n255\n`), rgb]));
