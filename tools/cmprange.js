// node tools/cmprange.js shotsDir rsDir FROM TO [OFFSET=7]: count differing pixels per frame between Mesen sh_<f>.png and ours rs_<f-OFFSET>.ppm
'use strict';
const { readPng, readPpm } = require('./pngcmp.js');
const [dir, rs, from, to, off] = [process.argv[2], process.argv[3], +process.argv[4], +process.argv[5], +(process.argv[6] || 7)];
let bad = 0, tot = 0, list = [];
for (let f = from; f <= to; f++) {
  let a, b; try { a = readPng(`${dir}/sh_${f}.png`); b = readPpm(`${rs}/rs_${f - off}.ppm`); } catch (e) { continue; }
  let d = 0; for (let i = 0; i < a.rgb.length; i += 3) if (a.rgb[i] !== b.rgb[i] || a.rgb[i + 1] !== b.rgb[i + 1] || a.rgb[i + 2] !== b.rgb[i + 2]) d++;
  tot++; if (d) { bad++; list.push(`${f}:${d}`); }
}
console.log(`${tot} frames, ${bad} differ`); console.log(list.slice(0, 40).join(' '));
