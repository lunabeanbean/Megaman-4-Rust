// Reader for harness/ppu_frames.lua recordings.
//   node tools/pf.js <file> info [from] [to]        one line per run of frames with the same scene/palette/OAM (a timeline)
//   node tools/pf.js <file> png <frame> <out.png> [nt]   render the state at that frame (nt = nametable index, default from $FF bits 0-1)
//   node tools/pf.js <file> diff <frameA> <frameB>   what changed between two frames (palette, OAM sprites, nametable cells)
'use strict';
const fs = require('fs');
const { NES_RGB, writePng, decodeTile } = require('./nes.js');

function load(file) {
  const frames = [];
  let cur = null;
  const nt = [null, null, null, null];
  let chr = null;
  for (const l of fs.readFileSync(file, 'utf8').split('\n')) {
    if (l.startsWith('F ')) {
      if (cur) frames.push(cur);
      const w = l.split(' ');
      cur = { n: +w[1], reg: {}, nt: nt.map((x) => x), chr };
      for (const kv of w.slice(2)) { const [k, v] = kv.split('='); cur.reg[k] = v.includes(',') ? v : parseInt(v, 16); }
    } else if (cur) {
      const sp = l.indexOf(' ');
      const tag = l.slice(0, sp), hex = l.slice(sp + 1);
      if (tag === 'PAL') cur.pal = Buffer.from(hex, 'hex');
      else if (tag === 'OAM') cur.oam = Buffer.from(hex, 'hex');
      else if (tag === 'BUF') cur.buf = Buffer.from(hex, 'hex');
      else if (tag === 'RAM') cur.ram = Buffer.from(hex, 'hex');
      else if (tag.startsWith('NT')) { nt[+tag[2]] = Buffer.from(hex, 'hex'); cur.nt = nt.map((x) => x); }
      else if (tag === 'CHR') { chr = Buffer.from(hex, 'hex'); cur.chr = chr; }
    }
  }
  if (cur) frames.push(cur);
  return frames;
}

function render(f, ntIndex) {
  const W = 256, H = 240, rgb = Buffer.alloc(W * H * 3);
  const put = (x, y, c) => { if (x < 0 || y < 0 || x >= W || y >= H) return; const o = (y * W + x) * 3; rgb[o] = c[0]; rgb[o + 1] = c[1]; rgb[o + 2] = c[2]; };
  const ni = ntIndex !== undefined ? ntIndex : (f.reg.ff & 3);
  const nt = f.nt[ni] || f.nt.find((x) => x), pal = f.pal, chr = f.chr || Buffer.alloc(8192);
  const bgBase = f.reg.ff & 0x10 ? 0x1000 : 0, spBase = f.reg.ff & 0x08 ? 0x1000 : 0;
  for (let ty = 0; ty < 30; ty++) for (let tx = 0; tx < 32; tx++) {
    const tile = nt[ty * 32 + tx], attr = nt[0x3C0 + (ty >> 2) * 8 + (tx >> 2)], shift = ((ty >> 1) & 1) * 4 + ((tx >> 1) & 1) * 2, p = (attr >> shift) & 3;
    const px = decodeTile(chr, bgBase + tile * 16);
    for (let y = 0; y < 8; y++) for (let x = 0; x < 8; x++) put(tx * 8 + x, ty * 8 + y, NES_RGB[pal[px[y * 8 + x] ? p * 4 + px[y * 8 + x] : 0] & 0x3F]);
  }
  for (let i = 63; i >= 0; i--) {
    const o = f.oam, y = o[i * 4] + 1, tile = o[i * 4 + 1], at = o[i * 4 + 2], x = o[i * 4 + 3];
    if (o[i * 4] >= 0xEF) continue;
    const px = decodeTile(chr, spBase + tile * 16), p = at & 3;
    for (let yy = 0; yy < 8; yy++) for (let xx = 0; xx < 8; xx++) {
      const sx = at & 0x40 ? 7 - xx : xx, sy = at & 0x80 ? 7 - yy : yy, c = px[sy * 8 + sx];
      if (c && !(at & 0x20 && false)) put(x + xx, y + yy, NES_RGB[pal[16 + p * 4 + c] & 0x3F]);
    }
  }
  return { W, H, rgb };
}

module.exports = { load, render };

if (require.main === module) {
  const [file, cmd, a, b, c] = process.argv.slice(2);
  const fr = load(file);
  if (cmd === 'info') {
    const from = +(a || 0), to = +(b || 1e9);
    let prev = null, start = 0;
    const key = (f) => `scene=${f.reg.scene.toString(16)} pal=${f.pal.toString('hex').slice(0, 16)}.. oamsp=${[...Array(64).keys()].filter((i) => f.oam[i * 4] < 0xEF).length} x19=${f.reg.x19 ? 1 : 0} x18=${f.reg.x18 ? 1 : 0}`;
    for (const f of fr) {
      if (f.n < from || f.n > to) continue;
      const k = key(f);
      if (k !== prev) { if (prev) console.log(`${start}-${f.n - 1}: ${prev}`); prev = k; start = f.n; }
    }
    console.log(`${start}-: ${prev}`);
  } else if (cmd === 'png') {
    const f = fr.find((x) => x.n === +a);
    const r = render(f, c !== undefined ? +c : undefined);
    writePng(b, r.W, r.H, r.rgb);
    console.log('wrote ' + b);
  } else if (cmd === 'diff') {
    const A = fr.find((x) => x.n === +a), B = fr.find((x) => x.n === +b);
    console.log('pal', A.pal.equals(B.pal) ? 'same' : A.pal.toString('hex') + ' -> ' + B.pal.toString('hex'));
    for (let i = 0; i < 64; i++) if (!A.oam.slice(i * 4, i * 4 + 4).equals(B.oam.slice(i * 4, i * 4 + 4))) console.log('oam', i, A.oam.slice(i * 4, i * 4 + 4).toString('hex'), '->', B.oam.slice(i * 4, i * 4 + 4).toString('hex'));
    for (let k = 0; k < 4; k++) if (A.nt[k] && B.nt[k] && !A.nt[k].equals(B.nt[k])) { const d = []; for (let i = 0; i < 1024; i++) if (A.nt[k][i] !== B.nt[k][i]) d.push(`${i}:${A.nt[k][i].toString(16)}>${B.nt[k][i].toString(16)}`); console.log('nt' + k, d.length, 'cells', d.slice(0, 40).join(' ')); }
  }
}
