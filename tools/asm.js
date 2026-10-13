// Print disassembly starting at a CPU address (with bank): node tools/asm.js 3BA223 [lines=60]
// Compact output: "addr  instruction" (labels shown as @addr). Reads megaman4-disassembly/bank38_3F.asm.
const fs = require('fs'), path = require('path');
const asm = fs.readFileSync(path.join(__dirname, '..', 'megaman4-disassembly', 'bank38_3F.asm'), 'latin1').split(/\r?\n/);
const want = process.argv[2].toUpperCase().padStart(6, '0');
const n = parseInt(process.argv[3] || '60', 10);
let out = 0, started = false;
for (let i = 0; i < asm.length && out < n; i++) {
  const m = /;\s*\$([0-9A-F]{6}) \|/.exec(asm[i]);
  if (!started) { if (m && m[1] === want) started = true; else continue; }
  const lab = /^code_([0-9A-F]{6}):/.exec(asm[i]);
  if (lab) { console.log('@' + lab[1].slice(2) + ':'); continue; }
  if (!m) { if (asm[i].trim().startsWith('db')) { console.log('        ' + asm[i].replace(/;.*/, '').trim()); out++; } continue; }
  console.log(m[1].slice(2) + '  ' + asm[i].replace(/;.*/, '').trim().replace(/code_[0-9A-F]{2}([0-9A-F]{4})/g, '@$1'));
  out++;
}
