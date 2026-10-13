// List the AI routines (states) reachable from an AI id, by scanning the disassembly for pointer assignments.
//   node tools/ai_states.js <rom> <aiId hex>...        e.g. node tools/ai_states.js "Mega Man 4 (USA).nes" 10 50
// A state = routine whose address is stored in $0588/$05A0 (lo/hi). Output: address, approximate length (lines), pointers it can set.
const fs = require('fs');
const path = require('path');
const rom = fs.readFileSync(process.argv[2]);
const asm = fs.readFileSync(path.join(__dirname, '..', 'megaman4-disassembly', 'bank38_3F.asm'), 'latin1').split(/\r?\n/);
const lines = [];
const byAddr = new Map();
asm.forEach((t, i) => {
  const m = /;\s*\$([0-9A-F]{6}) \|/.exec(t);
  if (m) { const a = parseInt(m[1], 16); byAddr.set(a, lines.length); lines.push({ a, t: t.replace(/;.*/, '').trim() }); }
});
const b3a = 16 + 0x3A * 8192;
const tbl = (base, i) => rom[b3a + base - 0x8000 + i];
function bankOf(ptr, ai) { return ptr < 0xA000 ? 0x3A : tbl(0x85B3, ai); }
function scan(start, bank) {
  const full = (bank << 16) | start;
  let i = byAddr.get(full);
  if (i === undefined) return null;
  const ptrs = [], calls = [];
  let maxT = 0, n = 0;
  for (; i < lines.length; i++, n++) {
    const L = lines[i];
    const br = /^(B[A-Z]{2})\s+code_([0-9A-F]{6})/.exec(L.t);
    if (br) { const tg = parseInt(br[2], 16); if (tg > L.a && tg > maxT) maxT = tg; }
    const jsr = /^JSR code_([0-9A-F]{6})/.exec(L.t);
    if (jsr) calls.push(jsr[1]);
    const sta = /^STA \$0588,x/.exec(L.t);
    if (sta) {
      // look back for LDA #lo, and ahead for STA $05A0 with LDA #hi
      let lo = null, hi = null;
      for (let k = i - 1; k >= Math.max(0, i - 3); k--) { const m = /^LDA #\$([0-9A-F]{2})/.exec(lines[k].t); if (m) { lo = parseInt(m[1], 16); break; } }
      for (let k = i + 1; k < Math.min(lines.length, i + 4); k++) { const m = /^LDA #\$([0-9A-F]{2})/.exec(lines[k].t); if (m) { hi = parseInt(m[1], 16); break; } }
      if (lo !== null && hi !== null) ptrs.push((hi << 8) | lo);
    }
    if ((L.t.startsWith('RTS') || L.t.startsWith('JMP') || L.t.startsWith('RTI')) && L.a >= maxT && !L.t.includes('($')) break;
    if (n > 600) break;
  }
  return { n, ptrs: [...new Set(ptrs)], calls: [...new Set(calls)] };
}
for (const ai of process.argv.slice(3).map((s) => parseInt(s, 16))) {
  const start = (tbl(0x8753, ai) << 8) | tbl(0x8683, ai);
  const seen = new Map();
  const todo = [start];
  const helpers = new Set();
  while (todo.length) {
    const p = todo.pop();
    if (seen.has(p)) continue;
    const r = scan(p, bankOf(p, ai));
    seen.set(p, r);
    if (!r) continue;
    r.ptrs.forEach((q) => todo.push(q));
    r.calls.forEach((c) => helpers.add(c));
  }
  console.log(`AI $${ai.toString(16)}: start $${start.toString(16)} bank $${bankOf(start, ai).toString(16)}, ${seen.size} states, ${[...seen.values()].reduce((s, r) => s + (r ? r.n : 0), 0)} lines`);
  for (const [p, r] of seen) console.log(`   $${p.toString(16)}: ${r ? r.n + ' lines -> ' + r.ptrs.map((q) => '$' + q.toString(16)).join(' ') : '(not found)'}`);
  const local = [...helpers].filter((h) => parseInt(h, 16) < 0x3FF000);
  console.log(`   helper calls: ${[...helpers].map((h) => h.slice(2)).join(' ')}`);
}
