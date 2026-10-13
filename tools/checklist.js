// Per-stage enemy checklist, generated from the ROM and the port's source.
//   node tools/checklist.js <rom.nes> <stage 0-14>
// For every placement-list type in the stage: how many are placed, the object type (AI id, table $820F), where its AI starts (bank:address) and a status:
//   unported   - the start address has no match arm in port/mm4-core/src/objects*.rs
//   ported     - an arm exists but no passing recording has run it (not in harness/verified_routines.txt)
//   matches    - a recording that matched the real game frame for frame ran that routine
// harness/game_test.sh appends the routines of every passing recording to harness/verified_routines.txt.
const fs = require('fs');
const path = require('path');
const { execFileSync } = require('child_process');
const [, , romPath, stageArg] = process.argv;
if (!romPath || stageArg === undefined) { console.error('usage: node tools/checklist.js <rom.nes> <stage>'); process.exit(2); }
const root = path.join(__dirname, '..');
const rom = fs.readFileSync(romPath);
const tmp = path.join(require('os').tmpdir(), `mm4_stage${stageArg}.json`);
execFileSync('node', [path.join(__dirname, 'stage.js'), 'export', romPath, stageArg, tmp], { stdio: 'ignore' });
const enemies = JSON.parse(fs.readFileSync(tmp, 'utf8')).enemies;

const bankBase = (b) => 16 + b * 0x2000;
const at = (bank, a) => rom[bankBase(bank) + a - 0x8000]; // bank mapped at $8000
const aiType = (lt) => at(0x38, 0x820F + lt);
const startOf = (t) => ({ addr: at(0x3A, 0x8683 + t) | (at(0x3A, 0x8753 + t) << 8), bank: at(0x3A, 0x85B3 + t) });

// addresses with a match arm in the port
const ported = new Set();
const srcDir = path.join(root, 'port', 'mm4-core', 'src');
const files = [path.join(srcDir, 'objects.rs')];
const sub = path.join(srcDir, 'objects');
if (fs.existsSync(sub)) for (const f of fs.readdirSync(sub)) if (f.endsWith('.rs')) files.push(path.join(sub, f));
for (const f of files) {
  for (const line of fs.readFileSync(f, 'utf8').split('\n')) {
    const m = line.match(/^\s*((?:0x[0-9A-Fa-f]{4}\s*\|?\s*)+)(?:=>|if )/);
    if (m) for (const a of m[1].match(/0x[0-9A-Fa-f]{4}/g)) ported.add(parseInt(a, 16));
  }
}
const vfile = path.join(root, 'harness', 'verified_routines.txt');
const verified = new Set(fs.existsSync(vfile) ? fs.readFileSync(vfile, 'utf8').split(/\s+/).filter(Boolean).map((x) => parseInt(x, 16)) : []);

const byType = new Map();
for (const e of enemies) byType.set(e.type, (byType.get(e.type) || 0) + 1);
const rows = [...byType.entries()].sort((a, b) => a[0] - b[0]).map(([lt, n]) => {
  const t = aiType(lt);
  const s = startOf(t);
  const status = verified.has(s.addr) ? 'matches' : ported.has(s.addr) ? 'ported' : 'unported';
  return { lt, n, t, s, status };
});
const hex = (v, w = 2) => v.toString(16).toUpperCase().padStart(w, '0');
console.log(`Stage ${stageArg}: ${enemies.length} placements, ${rows.length} list types`);
console.log('list  count  AI   start       status');
for (const r of rows) console.log(`${hex(r.lt)}    ${String(r.n).padStart(4)}   ${hex(r.t)}   ${hex(r.s.bank)}:${hex(r.s.addr, 4)}   ${r.status}`);
const c = { matches: 0, ported: 0, unported: 0 };
for (const r of rows) c[r.status]++;
console.log(`summary: ${c.matches} matches, ${c.ported} ported but unverified, ${c.unported} unported`);
