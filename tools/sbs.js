// node tools/sbs.js shot.png ours.ppm out.png x0 y0 w h [scale]: Mesen crop | ours crop | difference, side by side (for looking at a mismatch)
'use strict';
const fs = require('fs'), zlib = require('zlib');
const { readPng } = require('./pngcmp.js');
function readPpm(file) { const b = fs.readFileSync(file); let p = 0; const tok = () => { while (b[p] === 10 || b[p] === 32 || b[p] === 13) p++; let s = ''; while (b[p] !== 10 && b[p] !== 32 && b[p] !== 13) s += String.fromCharCode(b[p++]); return s; }; tok(); const w = +tok(), h = +tok(); tok(); p++; return { w, h, rgb: b.slice(p, p + w * h * 3) }; }
function crc32(buf) { let c, crc = 0xFFFFFFFF; for (let n = 0; n < buf.length; n++) { c = (crc ^ buf[n]) & 0xFF; for (let k = 0; k < 8; k++) c = c & 1 ? 0xEDB88320 ^ (c >>> 1) : c >>> 1; crc = (crc >>> 8) ^ c; } return (crc ^ 0xFFFFFFFF) >>> 0; }
function chunk(t, d) { const l = Buffer.alloc(4); l.writeUInt32BE(d.length); const td = Buffer.concat([Buffer.from(t), d]); const c = Buffer.alloc(4); c.writeUInt32BE(crc32(td)); return Buffer.concat([l, td, c]); }
const [a, b, out, x0, y0, w, h, sc] = [process.argv[2], process.argv[3], process.argv[4], +process.argv[5], +process.argv[6], +process.argv[7], +process.argv[8], +(process.argv[9] || 6)];
const A = readPng(a), B = readPpm(b);
const W = (w * 3 + 4) * sc, H = h * sc, raw = Buffer.alloc((W * 3 + 1) * H);
for (let y = 0; y < H; y++) { raw[y * (W * 3 + 1)] = 0; for (let x = 0; x < W; x++) {
  const cell = Math.floor(x / sc), part = Math.floor(cell / (w + 1)), cx = cell % (w + 1); let px = [40, 40, 40];
  if (cx < w) { const sx = x0 + cx, sy = y0 + Math.floor(y / sc); const i = (sy * 256 + sx) * 3;
    const pa = [A.rgb[i], A.rgb[i + 1], A.rgb[i + 2]], pb = [B.rgb[i], B.rgb[i + 1], B.rgb[i + 2]];
    if (part === 0) px = pa; else if (part === 1) px = pb; else px = (pa[0] !== pb[0] || pa[1] !== pb[1] || pa[2] !== pb[2]) ? [255, 0, 255] : [0, 0, 0]; }
  const o = y * (W * 3 + 1) + 1 + x * 3; raw[o] = px[0]; raw[o + 1] = px[1]; raw[o + 2] = px[2]; } }
const ihdr = Buffer.alloc(13); ihdr.writeUInt32BE(W, 0); ihdr.writeUInt32BE(H, 4); ihdr[8] = 8; ihdr[9] = 2;
fs.writeFileSync(out, Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), chunk('IHDR', ihdr), chunk('IDAT', zlib.deflateSync(raw)), chunk('IEND', Buffer.alloc(0))]));
