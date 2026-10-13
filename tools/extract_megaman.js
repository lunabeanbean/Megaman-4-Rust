// Extracts Mega Man's sprites from the ROM: raw tile sheet + every animation frame (assembled exactly like the game's draw code).
// node tools/extract_megaman.js "Mega Man 4 (USA).nes" <outdir> [stage=2]
'use strict';
const fs = require('fs');
const { NES_RGB, writePng, decodeTile } = require('./nes');
const { loadSet, cpuByte } = require('./gfx');
const rom = fs.readFileSync(process.argv[2]);
const out = process.argv[3];
const stage = +(process.argv[4] || 2);
fs.mkdirSync(out, { recursive: true });
const HEADER = 0x10, BANK = 0x2000;
const chr = Buffer.alloc(0x2000);
loadSet(rom, stage, chr);
const mmPal = []; for (let i = 0; i < 8; i++) mmPal.push(cpuByte(rom, {lo:0,hi:0}, 0xC853 + i));
console.log('Mega Man sprite palettes (NES colour ids):', mmPal.map(v => v.toString(16)).join(' '));
const pals = [mmPal.slice(0, 4), mmPal.slice(4, 8)];
pals.forEach(p => p[0] = 0x0F);   // index 0 = transparent

const bankNo = rom[HEADER + 0x3E * BANK + (0xDFFA - 0xC000)];
const bank = rom.subarray(HEADER + bankNo * BANK, HEADER + (bankNo + 1) * BANK);
const prev = rom.subarray(HEADER + (bankNo - 1) * BANK, HEADER + bankNo * BANK);
const byte = a => (a >= 0xA000 && a < 0xC000) ? bank[a - 0xA000] : (a >= 0x8000 && a < 0xA000) ? prev[a - 0x8000] : 0;
console.log('player sprite bank', bankNo);

function drawTile(img, W, tile, attr, x, y, flipH, flipV) {   // sprite tile from pattern table 1
  const px = decodeTile(chr, 0x1000 + tile * 16);
  const pal = pals[attr & 1] || pals[0];
  for (let j = 0; j < 8; j++) for (let i = 0; i < 8; i++) {
    const c = px[(flipV ? 7 - j : j) * 8 + (flipH ? 7 - i : i)];
    if (!c) continue;
    const X = x + i, Y = y + j;
    if (X < 0 || Y < 0 || X >= W || Y >= img.length / W / 4) continue;
    const rgb = NES_RGB[pal[c] & 0x3F];
    const o = (Y * W + X) * 4; img[o] = rgb[0]; img[o+1] = rgb[1]; img[o+2] = rgb[2]; img[o+3] = 255;
  }
}
// raw sheet of pattern table 1 (16x16 tiles) in Mega Man's colours
{
  const W = 128, H = 128, img = Buffer.alloc(W * H * 4);
  for (let t = 0; t < 256; t++) drawTile(img, W, t, 0, (t % 16) * 8, Math.floor(t / 16) * 8, false, false);
  save(out + '/tiles_raw.png', img, W, H, 4);
}
function save(file, rgba, w, h, scale, bg = [255, 0, 255]) {       // magenta = transparent, scaled up
  const rgb = Buffer.alloc(w * scale * h * scale * 3);
  for (let y = 0; y < h * scale; y++) for (let x = 0; x < w * scale; x++) {
    const o = (Math.floor(y / scale) * w + Math.floor(x / scale)) * 4, d = (y * w * scale + x) * 3;
    const a = rgba[o+3];
    rgb[d] = a ? rgba[o] : bg[0]; rgb[d+1] = a ? rgba[o+1] : bg[1]; rgb[d+2] = a ? rgba[o+2] : bg[2];
  }
  writePng(file, w * scale, h * scale, rgb);
}
// frames
const frames = [];
for (let id = 0; id < 256; id++) {
  const rec = (bank[0x1F00 + id] << 8) | bank[0x1E00 + id];
  if (rec < 0xA000 || rec >= 0xC000) continue;
  const n = byte(rec) & 0x7F, hiTab = byte(rec) & 0x80;
  if (id > 0x52) break;   // ids above $52 belong to other object types' data
  if (n > 20) continue;
  for (let f = 0; f <= n; f++) {
    const sid = byte(rec + 2 + f);
    if (!sid) continue;
    frames.push({ id, f, sid, hiTab });
  }
}
console.log('candidate frames', frames.length);
const CW = 48, CH = 48;   // cell big enough that nothing clips (sprites reach 8px past a 32px cell)
function renderFrame(fr, flip) {
  const [lo, hi] = fr.hiTab ? [0xB900, 0xBB00] : [0xB800, 0xBA00];
  const def = byte(lo + fr.sid) | byte(hi + fr.sid) << 8;
  const parts = byte(def);
  const xi = byte(def + 1) + flip;
  const off = byte(0xBC00 + xi) | byte(0xBD00 + xi) << 8;
  const img = Buffer.alloc(CW * CH * 4);
  const parts_out = [];
  for (let k = 0; k <= parts; k++) {
    const tile = byte(def + 2 + 2 * k), attr = byte(def + 3 + 2 * k);
    const dy = byte(off + 2 * k), dx = byte(off + 1 + 2 * k);
    const sy = (dy << 24 >> 24), sx = (dx << 24 >> 24);
    parts_out.push({ tile, attr, dx: sx, dy: sy });
    drawTile(img, CW, tile, attr, 24 + sx - 4, 32 + sy - 8, !!(attr & 0x40) !== !!flip && false || ((attr ^ (flip ? 0x40 : 0)) & 0x40) !== 0, (attr & 0x80) !== 0);
  }
  return { img, parts: parts_out };
}
const meta = [];
const cols = 16;
const animIds = [...new Set(frames.map(f => f.id))];
const rows = animIds.length;
for (const flip of [0, 1]) {
  const W = cols * CW, H = rows * CH, sheet = Buffer.alloc(W * H * 4);
  frames.forEach((fr, i) => {
    const { img, parts } = renderFrame(fr, flip);
    if (!flip) meta.push({ anim: fr.id, frame: fr.f, spriteId: fr.sid, cell: i, parts });
    const cx = fr.f * CW, cy = animIds.indexOf(fr.id) * CH;
    for (let y = 0; y < CH; y++) for (let x = 0; x < CW; x++) img.copy(sheet, ((cy + y) * W + cx + x) * 4, (y * CW + x) * 4, (y * CW + x) * 4 + 4);
  });
  save(out + (flip ? '/frames_facing_left.png' : '/frames_facing_right.png'), sheet, W, H, 3);
}
fs.writeFileSync(out + '/frames_index.json', JSON.stringify({ cellSize: [CW, CH], columns: cols, frames: meta }, null, 1));

// ---- clean sheet: one cell per UNIQUE sprite id (the game reuses a sprite id in many animations/frames), Mega Man's own tiles only.
// Sprites whose tiles are >= 144 live in the per-weapon / Rush / boss CHR banks, so with stage CHR loaded they render as garbage; they are listed in the index but left off the sheet.
{
  const DIG = ['111101101101111','010110010010111','111001111100111','111001111001111','101101111001001','111100111001111','111100111101111','111001001001001','111101111101111','111101111001111'];
  const uniq = new Map();   // sprite id -> { sid, fr, uses[] }
  frames.forEach(fr => {
    if (!uniq.has(fr.sid)) uniq.set(fr.sid, { sid: fr.sid, fr, uses: [] });
    uniq.get(fr.sid).uses.push(`anim ${fr.id} frame ${fr.f}`);
  });
  const rendered = [...uniq.values()].map(u => ({ ...u, ...renderFrame(u.fr, 0) }));
  const good = rendered.filter(r => Math.max(...r.parts.map(p => p.tile)) < 144 && r.parts.length);
  const bad = rendered.filter(r => !good.includes(r));
  const LH = 6, CC = 12, rowsC = Math.ceil(good.length / CC);
  const W = CC * CW, H = rowsC * (CH + LH), sheet = Buffer.alloc(W * H * 4);
  good.forEach((r, i) => {
    const cx = (i % CC) * CW, cy = Math.floor(i / CC) * (CH + LH);
    for (let y = 0; y < CH; y++) for (let x = 0; x < CW; x++) r.img.copy(sheet, ((cy + y + LH) * W + cx + x) * 4, (y * CW + x) * 4, (y * CW + x) * 4 + 4);
    String(r.sid).split('').forEach((d, k) => {   // sprite id as a tiny label above the cell
      for (let j = 0; j < 5; j++) for (let i2 = 0; i2 < 3; i2++) if (DIG[+d][j * 3 + i2] === '1') {
        const o = ((cy + j) * W + cx + 1 + k * 4 + i2) * 4; sheet[o] = 255; sheet[o+1] = 255; sheet[o+2] = 255; sheet[o+3] = 255;
      }
    });
  });
  save(out + '/clean_sheet.png', sheet, W, H, 4);   // 4x preview
  save(out + '/clean_sheet_1x.png', sheet, W, H, 1);   // true 1 px = 1 NES pixel, for editing in Aseprite
  fs.mkdirSync(out + '/single', { recursive: true });
  good.forEach(r => save(out + '/single/sprite_' + String(r.sid).padStart(3, '0') + '.png', r.img, CW, CH, 1));
  fs.writeFileSync(out + '/clean_index.json', JSON.stringify({
    note: 'one entry per unique sprite id; "usedBy" lists the animation frames that draw it. onSheet=false means it needs a weapon/Rush/boss tile bank.',
    sprites: rendered.map(r => ({ spriteId: r.sid, onSheet: good.includes(r), usedBy: r.uses, parts: r.parts }))
  }, null, 1));
  console.log(`clean sheet: ${good.length} unique sprites on sheet, ${bad.length} need other tile banks, from ${frames.length} animation frames`);
}

// ---- Zero template: 96x64 cells, body centre at x=48, feet on y=56, so there is room for saber swings and hair.
// The original Mega Man pose is copied in at the same anchor as a reference to redraw over.
{
  const TW = 96, TH = 64, AX = 48, FY = 56, OX = AX - 20, OY = FY - 36;   // Mega Man's centre is x=20 and his standing feet y=36 in a 48x48 cell
  const poses = [...uniq2()].filter(r => r.sid <= 31 && r.sid !== 14 && r.sid !== 21 && r.sid !== 22 && r.sid !== 23);
  function uniq2() { const m = new Map(); frames.forEach(fr => { if (!m.has(fr.sid)) m.set(fr.sid, { sid: fr.sid, fr }); }); return [...m.values()].map(u => ({ ...u, ...renderFrame(u.fr, 0) })).filter(r => Math.max(...r.parts.map(p => p.tile)) < 144); }
  const cols = 8, rowsT = Math.ceil(poses.length / cols);
  const SW = cols * TW, SH = rowsT * TH;
  const guide = (buf, W, cx, cy) => {   // centre line (x=48) and floor line (y=56), faint cyan
    for (let y = 0; y < TH; y++) put(buf, W, cx + AX, cy + y, 90, 200, 200);
    for (let x = 0; x < TW; x++) put(buf, W, cx + x, cy + FY, 90, 200, 200);
    for (let x = 0; x < TW; x++) { put(buf, W, cx + x, cy, 60, 60, 60); put(buf, W, cx + x, cy + TH - 1, 60, 60, 60); }
    for (let y = 0; y < TH; y++) { put(buf, W, cx, cy + y, 60, 60, 60); put(buf, W, cx + TW - 1, cy + y, 60, 60, 60); }
  };
  const put = (buf, W, x, y, r, g, b) => { const o = (y * W + x) * 4; buf[o] = r; buf[o+1] = g; buf[o+2] = b; buf[o+3] = 255; };
  const blank = Buffer.alloc(SW * SH * 4), ref = Buffer.alloc(SW * SH * 4);
  fs.mkdirSync(out + '/zero_template/single', { recursive: true });
  poses.forEach((r, i) => {
    const cx = (i % cols) * TW, cy = Math.floor(i / cols) * TH;
    const one = Buffer.alloc(TW * TH * 4);
    for (let y = 0; y < CH; y++) for (let x = 0; x < CW; x++) {
      const s = (y * CW + x) * 4; if (!r.img[s + 3]) continue;
      const X = x + OX, Y = y + OY; if (X < 0 || Y < 0 || X >= TW || Y >= TH) { console.log('template clip, sprite', r.sid); continue; }
      r.img.copy(ref, ((cy + Y) * SW + cx + X) * 4, s, s + 4); r.img.copy(one, (Y * TW + X) * 4, s, s + 4);
    }
    save(out + '/zero_template/single/pose_' + String(r.sid).padStart(3, '0') + '.png', one, TW, TH, 1);
  });
  poses.forEach((r, i) => guide(blank, SW, (i % cols) * TW, Math.floor(i / cols) * TH));
  poses.forEach((r, i) => { const cx = (i % cols) * TW, cy = Math.floor(i / cols) * TH; for (let y = 0; y < TH; y++) for (let x = 0; x < TW; x++) { const o = ((cy + y) * SW + cx + x) * 4; if (!ref[o + 3]) { const g = ((y === FY || x === AX || y === 0 || y === TH - 1 || x === 0 || x === TW - 1)); if (g) { ref[o] = 90; ref[o+1] = 200; ref[o+2] = 200; ref[o+3] = 255; } } } });
  save(out + '/zero_template/blank_grid_1x.png', blank, SW, SH, 1);
  save(out + '/zero_template/reference_poses_1x.png', ref, SW, SH, 1);
  fs.writeFileSync(out + '/zero_template/README.txt', `Cells are ${TW}x${TH}. Body centre line x=${AX}, floor line y=${FY} (both drawn in cyan). Pose ids match clean_sheet.png.\nreference_poses_1x.png = original Mega Man poses on the grid; draw Zero over them, keep the feet on the floor line.\nblank_grid_1x.png = same grid, empty.\n`);
  console.log('zero template:', poses.length, 'poses');
}
