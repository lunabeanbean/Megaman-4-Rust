// Make a test ROM whose stage keeps only some enemies of one stage.
//   node make_enemy_rom.js <stage> <out.nes> <typeHex,typeHex,...|all>  [base.nes]
// Types are the placement-list types (e.g. 00 = first enemy of stage 2). Commands (>= C0) are always kept.
const fs = require('fs');
const { execFileSync } = require('child_process');
const path = require('path');
const [stage, out, types, base] = process.argv.slice(2);
const root = path.join(__dirname, '..');
const rom = base || path.join(root, 'Mega Man 4 (USA).nes');
const tmp = path.join(__dirname, `_tmp_stage${stage}.json`);
execFileSync('node', [path.join(root, 'tools', 'stage.js'), 'export', rom, stage, tmp], { stdio: 'inherit' });
const j = JSON.parse(fs.readFileSync(tmp, 'utf8'));
const keep = types === 'all' ? null : new Set(types.split(',').map((t) => parseInt(t, 16)));
j.enemies = j.enemies.filter((e) => e.type >= 0xC0 || !keep || keep.has(e.type));
fs.writeFileSync(tmp, JSON.stringify(j));
execFileSync('node', [path.join(root, 'tools', 'stage.js'), 'import', rom, tmp, out], { stdio: 'inherit' });
fs.unlinkSync(tmp);
console.log(`wrote ${out} with ${j.enemies.length} entries`);
