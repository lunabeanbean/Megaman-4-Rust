// Compare a PNG screenshot (Mesen, 8-bit RGB/RGBA, not interlaced) with a PPM (P6): node tools/pngcmp.js <shot.png> <ours.ppm> [diff.png]
'use strict';
const fs = require('fs'), zlib = require('zlib');
function readPng(file) {
  const b = fs.readFileSync(file);
  let p = 8, w = 0, h = 0, ct = 2, idat = [];
  while (p < b.length) {
    const len = b.readUInt32BE(p), type = b.toString('ascii', p + 4, p + 8), data = b.slice(p + 8, p + 8 + len);
    if (type === 'IHDR') { w = data.readUInt32BE(0); h = data.readUInt32BE(4); ct = data[9]; if (data[8] !== 8 || data[12] !== 0) throw new Error('unsupported PNG'); }
    if (type === 'IDAT') idat.push(data);
    p += 12 + len;
  }
  const raw = zlib.inflateSync(Buffer.concat(idat)), bpp = ct === 6 ? 4 : 3, stride = w * bpp, out = Buffer.alloc(w * h * 3);
  let prev = Buffer.alloc(stride);
  for (let y = 0; y < h; y++) {
    const f = raw[y * (stride + 1)], line = Buffer.from(raw.slice(y * (stride + 1) + 1, (y + 1) * (stride + 1)));
    for (let x = 0; x < stride; x++) {
      const a = x >= bpp ? line[x - bpp] : 0, up = prev[x], c = x >= bpp ? prev[x - bpp] : 0;
      let v = line[x];
      if (f === 1) v += a; else if (f === 2) v += up; else if (f === 3) v += (a + up) >> 1;
      else if (f === 4) { const pa = Math.abs(up - c), pb = Math.abs(a - c), pc = Math.abs(a + up - 2 * c); v += (pa <= pb && pa <= pc) ? a : (pb <= pc ? up : c); }
      line[x] = v & 255;
    }
    for (let x = 0; x < w; x++) { out[(y * w + x) * 3] = line[x * bpp]; out[(y * w + x) * 3 + 1] = line[x * bpp + 1]; out[(y * w + x) * 3 + 2] = line[x * bpp + 2]; }
    prev = line;
  }
  return { w, h, rgb: out };
}
function readPpm(file) {
  const b = fs.readFileSync(file); let p = 0; const tok = () => { while (b[p] === 10 || b[p] === 32 || b[p] === 13) p++; let s = ''; while (b[p] !== 10 && b[p] !== 32 && b[p] !== 13) s += String.fromCharCode(b[p++]); return s; };
  tok(); const w = +tok(), h = +tok(); tok(); p++; return { w, h, rgb: b.slice(p, p + w * h * 3) };
}
if (require.main === module) {
  const a = readPng(process.argv[2]), c = readPpm(process.argv[3]);
  if (a.w !== c.w || a.h !== c.h) { console.log(`size differs: ${a.w}x${a.h} vs ${c.w}x${c.h}`); process.exit(2); }
  let d = 0, minx = 999, miny = 999, maxx = -1, maxy = -1;
  for (let i = 0; i < a.w * a.h; i++) if (a.rgb[i * 3] !== c.rgb[i * 3] || a.rgb[i * 3 + 1] !== c.rgb[i * 3 + 1] || a.rgb[i * 3 + 2] !== c.rgb[i * 3 + 2]) { d++; const x = i % a.w, y = (i / a.w) | 0; minx = Math.min(minx, x); maxx = Math.max(maxx, x); miny = Math.min(miny, y); maxy = Math.max(maxy, y); }
  console.log(`${d} of ${a.w * a.h} pixels differ` + (d ? ` (bbox x ${minx}-${maxx}, y ${miny}-${maxy})` : ''));
  process.exit(d ? 1 : 0);
}
module.exports = { readPng, readPpm };
