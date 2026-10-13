// Compare a PPM (from the Rust port: --ppm) with a PNG (from tools/render_stage.js). Exit code 0 = identical.
//   node tools/compare_ppm.js <a.ppm> <b.png>
'use strict';
const fs = require('fs'), zlib = require('zlib');
function readPpm(f) {
  const b = fs.readFileSync(f); let pos = 0; const tok = () => { while (b[pos] <= 32) pos++; let s = pos; while (b[pos] > 32) pos++; return b.toString('ascii', s, pos); };
  if (tok() !== 'P6') throw new Error('not P6'); const w = +tok(), h = +tok(); tok(); pos++;
  return { w, h, rgb: b.subarray(pos) };
}
function readPng(f) {
  const png = fs.readFileSync(f); let off = 8, idat = [], w, h;
  while (off < png.length) { const len = png.readUInt32BE(off), type = png.toString('ascii', off + 4, off + 8);
    if (type === 'IHDR') { w = png.readUInt32BE(off + 8); h = png.readUInt32BE(off + 12); } if (type === 'IDAT') idat.push(png.subarray(off + 8, off + 8 + len)); off += 12 + len; }
  const raw = zlib.inflateSync(Buffer.concat(idat)), rgb = Buffer.alloc(w * h * 3);
  for (let y = 0; y < h; y++) raw.copy(rgb, y * w * 3, y * (w * 3 + 1) + 1, (y + 1) * (w * 3 + 1));
  return { w, h, rgb };
}
const a = readPpm(process.argv[2]), b = readPng(process.argv[3]);
if (a.w !== b.w || a.h !== b.h) { console.log(`size differs: ${a.w}x${a.h} vs ${b.w}x${b.h}`); process.exit(1); }
let diff = 0; for (let i = 0; i < a.w * a.h * 3; i++) if (a.rgb[i] !== b.rgb[i]) diff++;
console.log(diff === 0 ? `identical (${a.w}x${a.h})` : `${diff} bytes differ`); process.exit(diff === 0 ? 0 : 1);
