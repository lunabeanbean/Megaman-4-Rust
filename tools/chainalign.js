// For Mesen screenshots sh_<f>.png in DIR, print which Rust frame (from DIR/chain.log hashes) shows the identical picture: node tools/chainalign.js DIR FROM TO
'use strict';
const fs = require('fs'), path = require('path');
const { readPng } = require('./pngcmp.js');
const [dir, from, to] = [process.argv[2], +process.argv[3], +process.argv[4]];
const byHash = {};
for (const l of fs.readFileSync(path.join(dir, 'chain.log'), 'utf8').split('\n')) { const p = l.split(' '); if (p.length >= 4) (byHash[p[3]] = byHash[p[3]] || []).push(+p[0]); }
function fnv(rgb) { let h = 0x811C9DC5 | 0; for (let i = 0; i < rgb.length; i++) { h ^= rgb[i]; h = Math.imul(h, 0x01000193); } return (h >>> 0).toString(16).padStart(8, '0'); }
let prev = null, out = [];
for (let f = from; f <= to; f++) {
  const png = path.join(dir, `sh_${f}.png`); if (!fs.existsSync(png)) continue;
  const js = (byHash[fnv(readPng(png).rgb)] || []);
  const near = js.filter(j => Math.abs(j - (f - 1)) <= 40).map(j => j - (f - 1));
  out.push(`${f}:${near.length ? near.join('/') : (js.length ? 'far' + (js[0] - (f - 1)) : 'none')}`);
}
console.log(out.join(' '));
