// Button script (PF_SEQ text) that types a password into the entry screen: node pwseq.js <first frame> "<r,c r,c ...>" [noend]
// Starts with the cursor on (0,0); moves right/down only (no wrapping), A toggles, then Right until END, then A (unless "noend").
const [start, dots, flag] = [process.argv[2], process.argv[3], process.argv[4]];
const targets = dots.split(' ').filter(Boolean).map((s) => s.split(',').map(Number)).sort((a, b) => a[0] - b[0] || a[1] - b[1]);
let f = +start, seq = [], r = 0, c = 0;
const press = (btn) => { seq.push(`{${f},${f + 2},{${btn}=true}}`); f += 9; };
for (const [tr, tc] of targets) {
  while (tr > r) { press('down'); r++; }
  while (tc > c) { press('right'); c++; }
  while (tc < c) { press('left'); c--; }
  press('a');
}
if (flag !== 'noend') { while (c < 6) { press('right'); c++; } press('a'); }
console.log(seq.join(","));
console.error("next free frame " + f);
