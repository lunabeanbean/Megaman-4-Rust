// node stack_png.js in.png out.png first last : cut the 256px-wide screens first..last out of a horizontal stage render and stack them vertically (side by side in 2 columns when many)
const zlib = require('zlib'), fs = require('fs');
const [inp, out, a, b] = process.argv.slice(2); const first = +a, last = +b;
const png = fs.readFileSync(inp);
let p = 8, idat = [], w, h;
while (p < png.length) { const len = png.readUInt32BE(p), type = png.toString('ascii', p + 4, p + 8); const d = png.slice(p + 8, p + 8 + len); if (type === 'IHDR') { w = d.readUInt32BE(0); h = d.readUInt32BE(4); } if (type === 'IDAT') idat.push(d); p += 12 + len; }
const raw = zlib.inflateSync(Buffer.concat(idat)); const bpp = 3, stride = w * bpp;
const img = Buffer.alloc(h * stride);
for (let y = 0; y < h; y++) {
  const f = raw[y * (stride + 1)];
  for (let i = 0; i < stride; i++) {
    const x = raw[y * (stride + 1) + 1 + i];
    const left = i >= bpp ? img[y * stride + i - bpp] : 0, up = y ? img[(y - 1) * stride + i] : 0, ul = (y && i >= bpp) ? img[(y - 1) * stride + i - bpp] : 0;
    let v;
    if (f === 0) v = x; else if (f === 1) v = x + left; else if (f === 2) v = x + up; else if (f === 3) v = x + ((left + up) >> 1);
    else { const pp = left + up - ul, pa = Math.abs(pp - left), pb = Math.abs(pp - up), pc = Math.abs(pp - ul); v = x + (pa <= pb && pa <= pc ? left : pb <= pc ? up : ul); }
    img[y * stride + i] = v & 255;
  }
}
const n = last - first + 1, cols = n > 6 ? 2 : 1, rows = Math.ceil(n / cols), gap = 4;
const W = cols * 256 + (cols - 1) * gap, H = rows * 240 + (rows - 1) * gap;
const o = Buffer.alloc(H * W * 3, 128);
for (let k = 0; k < n; k++) {
  const c = Math.floor(k / rows), r = k % rows;
  for (let y = 0; y < 240; y++) img.copy(o, ((r * (240 + gap) + y) * W + c * (256 + gap)) * 3, y * stride + (first + k) * 256 * 3, y * stride + (first + k) * 256 * 3 + 256 * 3);
}
const rawo = Buffer.alloc(H * (W * 3 + 1));
for (let y = 0; y < H; y++) { rawo[y * (W * 3 + 1)] = 0; o.copy(rawo, y * (W * 3 + 1) + 1, y * W * 3, (y + 1) * W * 3); }
const crcT = new Uint32Array(256).map((_, n) => { let c = n; for (let k = 0; k < 8; k++) c = c & 1 ? 0xEDB88320 ^ (c >>> 1) : c >>> 1; return c >>> 0; });
const crc = (buf) => { let c = 0xFFFFFFFF; for (const x of buf) c = crcT[(c ^ x) & 255] ^ (c >>> 8); return (c ^ 0xFFFFFFFF) >>> 0; };
const chunk = (t, d) => { const l = Buffer.alloc(4); l.writeUInt32BE(d.length); const td = Buffer.concat([Buffer.from(t), d]); const c = Buffer.alloc(4); c.writeUInt32BE(crc(td)); return Buffer.concat([l, td, c]); };
const ih = Buffer.alloc(13); ih.writeUInt32BE(W, 0); ih.writeUInt32BE(H, 4); ih[8] = 8; ih[9] = 2;
fs.writeFileSync(out, Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), chunk('IHDR', ih), chunk('IDAT', zlib.deflateSync(rawo)), chunk('IEND', Buffer.alloc(0))]));
console.log('wrote', out, W, H);
