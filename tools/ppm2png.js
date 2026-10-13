// Convert a binary PPM (P6) to PNG (no dependencies): node tools/ppm2png.js in.ppm out.png [scale]
const fs = require('fs'), zlib = require('zlib');
const [inp, out, sc] = process.argv.slice(2);
const scale = parseInt(sc || '1', 10);
const b = fs.readFileSync(inp);
let p = 0; const tok = () => { while (b[p] <= 32) p++; let s = ''; while (b[p] > 32) s += String.fromCharCode(b[p++]); return s; };
tok(); const w = +tok(), h = +tok(); tok(); p++;
const W = w * scale, H = h * scale;
const raw = Buffer.alloc((W * 3 + 1) * H);
for (let y = 0; y < H; y++) { raw[y * (W * 3 + 1)] = 0; for (let x = 0; x < W; x++) { const s = p + ((Math.floor(y / scale)) * w + Math.floor(x / scale)) * 3; const d = y * (W * 3 + 1) + 1 + x * 3; raw[d] = b[s]; raw[d + 1] = b[s + 1]; raw[d + 2] = b[s + 2]; } }
const crcT = []; for (let n = 0; n < 256; n++) { let c = n; for (let k = 0; k < 8; k++) c = c & 1 ? 0xEDB88320 ^ (c >>> 1) : c >>> 1; crcT[n] = c >>> 0; }
const crc = (buf) => { let c = 0xFFFFFFFF; for (const v of buf) c = crcT[(c ^ v) & 255] ^ (c >>> 8); return (c ^ 0xFFFFFFFF) >>> 0; };
const chunk = (t, d) => { const len = Buffer.alloc(4); len.writeUInt32BE(d.length); const td = Buffer.concat([Buffer.from(t), d]); const c = Buffer.alloc(4); c.writeUInt32BE(crc(td)); return Buffer.concat([len, td, c]); };
const ihdr = Buffer.alloc(13); ihdr.writeUInt32BE(W, 0); ihdr.writeUInt32BE(H, 4); ihdr[8] = 8; ihdr[9] = 2;
fs.writeFileSync(out, Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), chunk('IHDR', ihdr), chunk('IDAT', zlib.deflateSync(raw)), chunk('IEND', Buffer.alloc(0))]));
