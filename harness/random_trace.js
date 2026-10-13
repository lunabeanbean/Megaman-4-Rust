// Random-input differential test generator.
//   node harness/random_trace.js <seed> [frames=1200]
// Builds a seeded random button sequence (hold left/right/nothing, tap or hold A, optionally Down, 1-40 frame segments) and hands it to
// gen_ptrace.js, which writes ptrace_r<seed>.lua. Same seed = same inputs. Run it on harness/mm4_noenemy.nes (see random_test.sh).
const { execFileSync } = require('child_process');
const seed = parseInt(process.argv[2], 10);
const frames = parseInt(process.argv[3] || '1200', 10);
if (!Number.isFinite(seed)) { console.error('usage: node random_trace.js <seed> [frames]'); process.exit(2); }
let s = seed >>> 0;
const rnd = () => { s = (s + 0x6D2B79F5) >>> 0; let t = s; t = Math.imul(t ^ (t >>> 15), t | 1); t ^= t + Math.imul(t ^ (t >>> 7), t | 61); return ((t ^ (t >>> 14)) >>> 0) / 4294967296; };
const DOWN = parseFloat(process.env.DOWN || '0'); // probability of holding Down in a segment (0 = never)
const UP = parseFloat(process.env.UP || '0'); // probability of holding Up in a segment (ladders)
const RIGHT = parseFloat(process.env.RIGHT || '0.55'); // probability of moving right in a segment
const B = parseFloat(process.env.B || '0'); // probability of holding B (shoot) in a segment
const pick = (n) => Math.floor(rnd() * n);
const parts = [];
let t = 0;
while (t < frames) {
  const len = 1 + pick(rnd() < 0.3 ? 6 : 40);
  const r = rnd();
  const dir = r < RIGHT ? 'right' : r < RIGHT + (1 - RIGHT) * 0.55 ? 'left' : null; // mostly rightwards so the run covers new ground
  const a = rnd() < 0.5;
  const down = rnd() < DOWN; // Down+A starts a slide
  const up = UP > 0 && !down && rnd() < UP;
  const bb = B > 0 && rnd() < B;
  const btn = [dir ? `${dir}=true` : '', down ? 'down=true' : '', up ? 'up=true' : '', a ? 'a=true' : '', bb ? 'b=true' : ''].filter(Boolean).join(',');
  parts.push(`{${t},${t + len},{${btn}}}`);
  t += len;
}
execFileSync('node', [__dirname + '/gen_ptrace.js', 'r' + seed, String(frames), parts.join(', ')], { stdio: 'inherit' });
