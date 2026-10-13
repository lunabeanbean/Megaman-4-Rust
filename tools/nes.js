// Shared helpers: PNG writer (no dependencies) and the standard NES master palette.
'use strict';
const zlib = require('zlib');

// The 2C02 palette as Mesen shows it (read back with harness/palette_probe.lua).
const NES_RGB = [
  [102,102,102],[0,42,136],[20,18,167],[59,0,164],[92,0,126],[110,0,64],[108,6,0],[86,29,0],[51,53,0],[11,72,0],[0,82,0],[0,79,8],[0,64,77],[0,0,0],[0,0,0],[0,0,0],
  [173,173,173],[21,95,217],[66,64,255],[117,39,254],[160,26,204],[183,30,123],[181,49,32],[153,78,0],[107,109,0],[56,135,0],[12,147,0],[0,143,50],[0,124,141],[0,0,0],[0,0,0],[0,0,0],
  [255,254,255],[100,176,255],[146,144,255],[198,118,255],[243,106,255],[254,110,204],[254,129,112],[234,158,34],[188,190,0],[136,216,0],[92,228,48],[69,224,130],[72,205,222],[79,79,79],[0,0,0],[0,0,0],
  [255,254,255],[192,223,255],[211,210,255],[232,200,255],[251,194,255],[254,196,234],[254,204,197],[247,216,165],[228,229,148],[207,239,150],[189,244,171],[179,243,204],[181,235,242],[184,184,184],[0,0,0],[0,0,0],
];

function crc32(buf) {
  let c, crc = 0xFFFFFFFF;
  for (let n = 0; n < buf.length; n++) {
    c = (crc ^ buf[n]) & 0xFF;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xEDB88320 ^ (c >>> 1) : c >>> 1;
    crc = (crc >>> 8) ^ c;
  }
  return (crc ^ 0xFFFFFFFF) >>> 0;
}
function chunk(type, data) {
  const len = Buffer.alloc(4); len.writeUInt32BE(data.length);
  const td = Buffer.concat([Buffer.from(type), data]);
  const crc = Buffer.alloc(4); crc.writeUInt32BE(crc32(td));
  return Buffer.concat([len, td, crc]);
}
// rgb: Buffer of w*h*3 bytes
function writePng(file, w, h, rgb) {
  const raw = Buffer.alloc((w * 3 + 1) * h);
  for (let y = 0; y < h; y++) { raw[y * (w * 3 + 1)] = 0; rgb.copy(raw, y * (w * 3 + 1) + 1, y * w * 3, (y + 1) * w * 3); }
  const ihdr = Buffer.alloc(13); ihdr.writeUInt32BE(w, 0); ihdr.writeUInt32BE(h, 4); ihdr[8] = 8; ihdr[9] = 2;
  require('fs').writeFileSync(file, Buffer.concat([Buffer.from([137,80,78,71,13,10,26,10]), chunk('IHDR', ihdr), chunk('IDAT', zlib.deflateSync(raw)), chunk('IEND', Buffer.alloc(0))]));
}
// Decode one 8x8 tile (16 bytes of 2bpp planar data) into an array of 64 colour indexes 0-3.
function decodeTile(chr, offset) {
  const px = new Array(64);
  for (let y = 0; y < 8; y++) for (let x = 0; x < 8; x++) {
    const lo = (chr[offset + y] >> (7 - x)) & 1, hi = (chr[offset + y + 8] >> (7 - x)) & 1;
    px[y * 8 + x] = lo | (hi << 1);
  }
  return px;
}
module.exports = { NES_RGB, writePng, decodeTile };
