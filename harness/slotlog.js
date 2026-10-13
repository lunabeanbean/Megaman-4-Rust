// Prints, per logic frame, the slots of an otrace file in a readable form, only when something other than positions changed.
//   node slotlog.js otrace_x.txt ptrace_x.txt [from] [to]
const fs = require('fs');
const [,, ofile, pfile, fromS, toS] = process.argv;
const from = +(fromS || 0), to = +(toS || 1e9);
const names = ['type','page','x','xsub','ys','y','ysub','xvs','xvp','yvs','yvp','hb','mv','idx','hp','s0','s1','s2','s3','s4','s5','s6','s7','fl','af','aid','at','ptlo','pthi','blink'];
const o = fs.readFileSync(ofile, 'utf8').split('\n');
const p = {};
if (pfile && pfile !== '-') for (const l of fs.readFileSync(pfile, 'utf8').split('\n')) { const f = l.trim().split(/\s+/); if (f.length > 10) p[f[0]] = f; }
let prev = {};
for (const line of o) {
  const f = line.trim().split(/\s+/);
  if (f[0] === 'S' || f.length < 14) continue;
  const t = +f[0];
  if (t < from || t > to) continue;
  const oi = f.indexOf('O');
  const n = parseInt(f[oi + 1], 16) || +f[oi + 1];
  const g = f.slice(2, oi);
  let k = oi + 2;
  const out = [];
  for (let i = 0; i < n; i++) {
    const slot = f[k]; const b = f.slice(k + 1, k + 31); k += 31;
    const key = slot;
    const sig = [b[0], b[22+1-1+0 + 0] ].join();
    const cur = b.join(' ');
    const ptr = b[28] + b[27];
    const info = `s${slot}: ty=${b[0]} pg=${b[1]} x=${b[2]} y=${b[4]}:${b[5]} ptr=${ptr} aid=${b[25]} af=${b[24]} fl=${b[23]} xv=${b[8]}.${b[7]} yv=${b[10]}.${b[9]} s0..7=${b.slice(15,23).join(',')} hp=${b[14]} blink=${b[29]}`;
    out.push(info);
  }
  const pr = p[t];
  console.log(`t=${t} G=${g.join(' ')}${pr ? ' | P st=' + pr[3] + ' fc=' + pr[4] + ' x=' + pr[5] + pr[6] + ' y=' + pr[8] + pr[9] + ' anim=' + pr[17] : ''}\n   ` + out.join('\n   '));
}
