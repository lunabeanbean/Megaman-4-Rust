// Turns a solid-colour background (default magenta ff00ff) in a PNG into real transparency. Never overwrites the input.
// node tools/png_transparent.js in.png [out.png] [rrggbb]
'use strict';
const fs = require('fs'), zlib = require('zlib');
const { } = require('./nes');
const inp = process.argv[2], outp = process.argv[3] || inp.replace(/\.png$/i, '_transparent.png');
const key = (process.argv[4] || 'ff00ff').match(/../g).map(h => parseInt(h, 16));
const b = fs.readFileSync(inp);
let p = 8, W, H, depth, ctype, plte = null, trns = null; const idat = [];
while (p < b.length) {
  const len = b.readUInt32BE(p), type = b.toString('ascii', p + 4, p + 8), d = b.subarray(p + 8, p + 8 + len);
  if (type === 'IHDR') { W = d.readUInt32BE(0); H = d.readUInt32BE(4); depth = d[8]; ctype = d[9]; if (d[12]) throw new Error('interlaced PNG not supported'); }
  else if (type === 'PLTE') plte = d; else if (type === 'tRNS') trns = d; else if (type === 'IDAT') idat.push(d);
  p += 12 + len;
}
if (depth !== 8) throw new Error('only 8-bit PNGs supported (got ' + depth + ')');
const ch = { 0: 1, 2: 3, 3: 1, 4: 2, 6: 4 }[ctype], bpp = ch, stride = W * ch;
const raw = zlib.inflateSync(Buffer.concat(idat)), px = Buffer.alloc(H * stride);
for (let y = 0; y < H; y++) {
  const f = raw[y * (stride + 1)];
  for (let x = 0; x < stride; x++) {
    const a = x >= bpp ? px[y * stride + x - bpp] : 0, up = y ? px[(y - 1) * stride + x] : 0, c = (x >= bpp && y) ? px[(y - 1) * stride + x - bpp] : 0;
    let v = raw[y * (stride + 1) + 1 + x];
    if (f === 1) v += a; else if (f === 2) v += up; else if (f === 3) v += (a + up) >> 1;
    else if (f === 4) { const pa = Math.abs(up - c), pb = Math.abs(a - c), pc = Math.abs(a + up - 2 * c); v += (pa <= pb && pa <= pc) ? a : pb <= pc ? up : c; }
    px[y * stride + x] = v & 255;
  }
}
const rgba = Buffer.alloc(W * H * 4); let cleared = 0;
for (let i = 0; i < W * H; i++) {
  let r, g, bl, a = 255;
  if (ctype === 6) { [r, g, bl, a] = [px[i*4], px[i*4+1], px[i*4+2], px[i*4+3]]; }
  else if (ctype === 2) { [r, g, bl] = [px[i*3], px[i*3+1], px[i*3+2]]; }
  else if (ctype === 3) { const k = px[i]; [r, g, bl] = [plte[k*3], plte[k*3+1], plte[k*3+2]]; if (trns && k < trns.length) a = trns[k]; }
  else if (ctype === 0) { r = g = bl = px[i]; } else { r = g = bl = px[i*2]; a = px[i*2+1]; }
  if (r === key[0] && g === key[1] && bl === key[2]) { a = 0; r = g = bl = 0; cleared++; }
  rgba.set([r, g, bl, a], i * 4);
}
const o = Buffer.alloc(H * (W * 4 + 1));
for (let y = 0; y < H; y++) rgba.copy(o, y * (W * 4 + 1) + 1, y * W * 4, (y + 1) * W * 4);
const crcT = []; for (let n = 0; n < 256; n++) { let c = n; for (let k = 0; k < 8; k++) c = c & 1 ? 0xEDB88320 ^ (c >>> 1) : c >>> 1; crcT[n] = c >>> 0; }
const crc = buf => { let c = 0xFFFFFFFF; for (const x of buf) c = crcT[(c ^ x) & 255] ^ (c >>> 8); return (c ^ 0xFFFFFFFF) >>> 0; };
const chunk = (t, d) => { const l = Buffer.alloc(4); l.writeUInt32BE(d.length); const td = Buffer.concat([Buffer.from(t), d]); const c = Buffer.alloc(4); c.writeUInt32BE(crc(td)); return Buffer.concat([l, td, c]); };
const ih = Buffer.alloc(13); ih.writeUInt32BE(W, 0); ih.writeUInt32BE(H, 4); ih[8] = 8; ih[9] = 6;
fs.writeFileSync(outp, Buffer.concat([Buffer.from([137,80,78,71,13,10,26,10]), chunk('IHDR', ih), chunk('IDAT', zlib.deflateSync(o)), chunk('IEND', Buffer.alloc(0))]));
console.log(`${outp}: ${W}x${H}, ${cleared} background pixels made transparent`);
