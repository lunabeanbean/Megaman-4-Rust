// node tools/palcmp.js RUSTDIR MESENLOG FROM TO [LAG=6] : compares the PPU palette per frame (last token of each log line) for several Mesen offsets
'use strict';
const fs = require('fs');
const rs = {}, ms = {};
for (const l of fs.readFileSync(process.argv[2] + '/chain.log', 'utf8').split('\n')) { const p = l.trim().split(' '); if (p.length > 10) rs[+p[0]] = p[p.length - 1]; }
for (const l of fs.readFileSync(process.argv[3], 'utf8').split('\n')) { const p = l.trim().split(' '); if (p.length > 10 && /^\d+$/.test(p[0])) ms[+p[0]] = p[p.length - 1]; }
const from = +process.argv[4], to = +process.argv[5], lag = +(process.argv[6] || 6);
for (let off = -2; off <= 3; off++) {
  let ok = 0, bad = 0, first = -1;
  for (let f = from; f <= to; f++) { const a = rs[f - lag], b = ms[f + off]; if (!a || !b) continue; if (a === b) ok++; else { bad++; if (first < 0) first = f; } }
  console.log(`mesen offset ${off}: ${ok} same, ${bad} differ, first ${first}`);
}
