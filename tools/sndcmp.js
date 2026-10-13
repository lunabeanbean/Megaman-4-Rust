// Compares the sound requests of a Mesen chain recording (harness/chain_snd/<name>/snd.log, console frames) with the Rust flow's (mm4 --chain ... --out DIR -> DIR/sound.log, flow frames).
//   node tools/sndcmp.js <mesen snd.log> <rust sound.log> [lag=6]
// Lines: "<frame> <C|E> <id> <return address>". The flow's frame n is console frame n + lag. The two sequences are compared in order; a request matches when kind, id and return address are
// equal; the frame difference is reported (0 = same frame).
const fs = require('fs');
const [a, b, lagArg] = process.argv.slice(2);
const lag = lagArg === undefined ? 6 : +lagArg;
const rd = (f, off) => fs.readFileSync(f, 'utf8').split('\n').filter(l => l.trim()).map(l => { const p = l.split(/\s+/); return { f: +p[0] + off, k: p[1], id: p[2], ret: p[3] }; });
const m = rd(a, 0).filter(e => e.f >= lag);          // requests before the flow starts (console boot) are not ours
const r = rd(b, lag);
let i = 0, j = 0, same = 0, shifted = 0, bad = 0;
const hist = {};
while (i < m.length && j < r.length) {
  const x = m[i], y = r[j];
  if (x.k === y.k && x.id === y.id && x.ret === y.ret) {
    const d = y.f - x.f; hist[d] = (hist[d] || 0) + 1; if (d !== 0 && process.env.SHOWOFF) console.log(`offset ${d}: mesen ${x.f} ${x.k}${x.id}@${x.ret}`);
    if (d === 0) same++; else shifted++;
    i++; j++;
  } else {
    bad++;
    if (bad <= 15) console.log(`differs: mesen ${x.f} ${x.k}${x.id}@${x.ret}   rust ${y.f} ${y.k}${y.id}@${y.ret}`);
    // resynchronise: skip whichever side is earlier
    if (x.f <= y.f) i++; else j++;
  }
}
for (; i < m.length; i++) { bad++; if (bad <= 15) console.log(`only mesen: ${m[i].f} ${m[i].k}${m[i].id}@${m[i].ret}`); }
for (; j < r.length; j++) { bad++; if (bad <= 15) console.log(`only rust: ${r[j].f} ${r[j].k}${r[j].id}@${r[j].ret}`); }
console.log(`mesen ${m.length} requests, rust ${r.length}: ${same} on the same frame, ${shifted} on another frame, ${bad} unmatched; frame offsets (rust - mesen): ${JSON.stringify(hist)}`);
