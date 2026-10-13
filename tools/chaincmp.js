// Compare the chain run of the Rust flow (port: mm4 --chain ... --out DIR, writes DIR/chain.log with a hash per frame) with the Mesen screenshots of harness/chain_rec.lua.
//   node tools/chaincmp.js DIR FROM TO [OFFSET=7]   Mesen screenshot of frame f shows the picture of flow frame f-OFFSET (1 for the screenshot lag + 6 frames the console
//                                                    spends after power-on before the first scene)
//   node tools/chaincmp.js DIR hash                  turns DIR/sh_<f>.png into DIR/sh.hash (one FNV hash per frame, so the PNGs can be deleted); the compare uses it when present
//   env RUNS=n: how many runs of differing frames to list
'use strict';
const fs = require('fs'), path = require('path');
const dir = process.argv[2];
function fnv(rgb) { let h = 0x811C9DC5 | 0; for (let i = 0; i < rgb.length; i++) { h ^= rgb[i]; h = Math.imul(h, 0x01000193); } return (h >>> 0).toString(16).padStart(8, '0'); }
if (process.argv[3] === 'hash') {
  const { readPng } = require('./pngcmp.js');
  const out = [];
  for (const f of fs.readdirSync(dir)) { const m = /^sh_(\d+)\.png$/.exec(f); if (m) { try { out.push([+m[1], fnv(readPng(path.join(dir, f)).rgb)]); } catch (e) { /* a truncated last picture */ } } }
  out.sort((a, b) => a[0] - b[0]);
  fs.writeFileSync(path.join(dir, 'sh.hash'), out.map((x) => x.join(' ')).join('\n') + '\n');
  console.log(`${dir}: ${out.length} hashes`);
  process.exit(0);
}
const from = +process.argv[3], to = +process.argv[4], off = process.argv[5] === undefined ? 7 : +process.argv[5];
const rs = {};
for (const l of fs.readFileSync(path.join(dir, 'chain.log'), 'utf8').split('\n')) { const p = l.split(' '); if (p.length >= 4) rs[+p[0]] = { screen: p[1], stage: p[2], h: p[3] }; }
const sh = {};
const hf = path.join(dir, 'sh.hash');
if (fs.existsSync(hf)) { for (const l of fs.readFileSync(hf, 'utf8').split('\n')) { const p = l.split(' '); if (p.length === 2) sh[+p[0]] = p[1]; } }
else { const { readPng } = require('./pngcmp.js'); for (let f = from; f <= to; f++) { const png = path.join(dir, `sh_${f}.png`); if (fs.existsSync(png)) sh[f] = fnv(readPng(png).rgb); } }
let bad = 0, ok = 0, firstBad = null, runs = [];
for (let f = from; f <= to; f++) {
  if (sh[f] === undefined) continue;
  const mine = rs[f - off];
  if (!mine) continue;
  if (sh[f] === mine.h) ok++; else { bad++; if (firstBad === null) firstBad = f; const last = runs[runs.length - 1]; if (last && last[1] === f - 1) last[1] = f; else runs.push([f, f, mine.screen]); }
}
console.log(`${dir}: frames ${from}-${to} (offset ${off}): ${ok} identical, ${bad} differ` + (bad ? `; first difference at frame ${firstBad}; runs: ` + runs.slice(0, +(process.env.RUNS || 12)).map(r => `${r[0]}-${r[1]}(${r[2]})`).join(' ') : ''));
process.exit(bad ? 1 : 0);
