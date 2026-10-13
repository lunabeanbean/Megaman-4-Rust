// Patch bytes of one stage's bank in a ROM copy (test variants).
//   node patch_rom.js <in.nes> <out.nes> <stage> <cpuAddrHex>=<byteHex> ...     e.g.  node patch_rom.js a.nes b.nes 2 B531=21
const fs = require('fs');
const [inp, out, stage, ...ps] = process.argv.slice(2);
const r = fs.readFileSync(inp);
for (const p of ps) {
  const [a, v] = p.split('=');
  r[0x10 + (0x20 | parseInt(stage)) * 0x2000 + parseInt(a, 16) - 0xA000] = parseInt(v, 16);
}
fs.writeFileSync(out, r);
console.log('wrote', out);
