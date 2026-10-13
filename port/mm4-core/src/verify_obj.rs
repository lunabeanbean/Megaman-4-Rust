//! Compare the Rust object system with a trace of the real game (harness/gen_ptrace.js with OBJ=1: `ptrace_*.txt` + `otrace_*.txt`).
//! Mega Man's position/state and the camera come from the player trace (the object code only reads them), everything else is simulated.
use crate::camera::Camera;
use crate::game::Game;
use crate::objects::{Cam, Obj, PlayerView, World};
use crate::oam::Sprite;
use crate::player::{Buttons, Player};
use crate::rom::Rom;
use crate::stage::Stage;

const NAMES: [&str; 30] = [
    "ty", "xpg", "xpx", "xsub", "ypg", "ypx", "ysub", "xvs", "xvp", "yvs", "yvp", "hitbox", "dir", "idx", "hp", "s0", "s1", "s2", "s3",
    "s4", "s5", "s6", "s7", "flags", "aframe", "aid", "atick", "ptr_lo", "ptr_hi", "t5b8",
];

struct PRow {
    t: i32,
    view: PlayerView,
    cam: Cam,
    pressed: u8,
    held: u8,
    state: u8,
    facing: u8,
    x: i32,
    y: i32,
    vy: i32,
    anim: (u8, u8),
    flip: bool,
    life: u8,
    /// `$0300` of Mega Man (0 = not drawn)
    ptype: u8,
    /// `$3C` (hit invincibility), if the trace has the column.
    inv: Option<u8>,
}

struct ORow {
    t: i32,
    rng: [u8; 4],
    p95: u8,
    c9e: u8,
    c9f: u8,
    dir: u8,
    f92: u8,
    objs: Vec<(usize, Obj)>,
}

fn hex(s: &str) -> u8 {
    u8::from_str_radix(s, 16).unwrap_or_else(|_| panic!("bad hex '{}'", s))
}

fn parse_p(line: &str) -> Option<PRow> {
    let f: Vec<&str> = line.split_whitespace().collect();
    if f.len() < 20 {
        return None;
    }
    Some(PRow {
        t: f[0].parse().ok()?,
        view: PlayerView { xpg: hex(f[5]), xpx: hex(f[6]), ypg: hex(f[8]), ypx: hex(f[9]), state: hex(f[3]), anim: hex(f[17]), falling: hex(f[11]) & 0x80 != 0, life: hex(f[13]), flip: hex(f[19]) & 0x40 != 0 },
        cam: Cam { hi: hex(f[15]), lo: hex(f[16]) },
        pressed: hex(f[1]),
        held: hex(f[2]),
        state: hex(f[3]),
        facing: hex(f[4]),
        x: ((hex(f[5]) as i32) << 16) | ((hex(f[6]) as i32) << 8) | hex(f[7]) as i32,
        y: (((hex(f[8]) as i8 as i32) * 240 + hex(f[9]) as i32) << 8) | hex(f[10]) as i32,
        vy: (((((hex(f[11]) as u16) << 8) | hex(f[12]) as u16) as i16) as i32),
        anim: (hex(f[17]), hex(f[18])),
        flip: hex(f[19]) & 0x40 != 0,
        life: hex(f[13]),
        ptype: hex(f[14]),
        inv: if f.len() >= 21 { Some(hex(f[20])) } else { None },
    })
}

fn parse_o(line: &str) -> Option<ORow> {
    let f: Vec<&str> = line.split_whitespace().collect();
    if f.len() < 14 || f[1] != "G" {
        return None;
    }
    // t G E4 E6 E5 95 9E 9F 2F FD 2E E7 [92] O n ...
    let g = |i: usize| hex(f[2 + i]);
    let opos = f.iter().position(|s| *s == "O")?;
    let f92 = if opos >= 13 { g(10) } else { 0 };
    let n: usize = f.get(opos + 1)?.parse().ok()?;
    if f.len() < opos + 2 + n * 31 {
        return None; // truncated last line of an interrupted recording
    }
    let mut objs = Vec::new();
    for k in 0..n {
        let base = opos + 2 + k * 31;
        let slot = hex(f[base]) as usize;
        let fields: Vec<u8> = f[base + 1..base + 31].iter().map(|s| hex(s)).collect();
        objs.push((slot, Obj::from_fields(&fields)));
    }
    Some(ORow { t: f[0].parse().ok()?, rng: [g(0), g(2), g(1), g(9)], p95: g(3), c9e: g(4), c9f: g(5), dir: g(6), f92, objs })
}

fn compare(w: &World, o: &ORow) -> Option<String> {
    let mut bad: Vec<String> = Vec::new();
    if w.rng != o.rng {
        bad.push(format!("rng game {:02X?} port {:02X?}", o.rng, w.rng));
    }
    if w.p95 != o.p95 || w.c9e != o.c9e || w.c9f != o.c9f {
        bad.push(format!("p95/c9e/c9f game {:02X}/{:02X}/{:02X} port {:02X}/{:02X}/{:02X}", o.p95, o.c9e, o.c9f, w.p95, w.c9e, w.c9f));
    }
    for slot in 1..24 {
        let game = o.objs.iter().find(|(s, _)| *s == slot).map(|(_, ob)| *ob);
        let port = if w.o[slot].ty != 0 { Some(w.o[slot]) } else { None };
        match (game, port) {
            (None, None) => {}
            (Some(g), None) => bad.push(format!("slot {:02X}: game has type {:02X} at {:02X}:{:02X}, port has nothing", slot, g.ty, g.xpg, g.xpx)),
            (None, Some(q)) => bad.push(format!("slot {:02X}: port has type {:02X} at {:02X}:{:02X}, game has nothing", slot, q.ty, q.xpg, q.xpx)),
            (Some(g), Some(q)) => {
                let (mut gf, mut qf) = (g.fields(), q.fields());
                    // `$0588` is dead data until the AI loop has initialised the pointer (`$05A0` bit 7 set): ignore it before that
                    if gf[28] & 0x80 == 0 {
                        gf[27] = 0;
                    }
                    if qf[28] & 0x80 == 0 {
                        qf[27] = 0;
                    }
                let diffs: Vec<String> =
                    (0..30).filter(|&k| gf[k] != qf[k]).map(|k| format!("{} game {:02X} port {:02X}", NAMES[k], gf[k], qf[k])).collect();
                if !diffs.is_empty() {
                    bad.push(format!("slot {:02X} (type {:02X}): {}", slot, g.ty, diffs.join(", ")));
                }
            }
        }
    }
    if bad.is_empty() {
        None
    } else {
        let mut msg = bad.iter().map(|b| format!("  {}", b)).collect::<Vec<_>>().join("
");
        if !w.unported.is_empty() {
            msg += &format!("
  unported AI routines reached: {:04X?}", w.unported);
        }
        Some(msg)
    }
}

/// One game frame of the object system with Mega Man's data from the trace row `p` / `o`.
fn frame(w: &mut World, stage: &Stage, p: &PRow, o: &ORow) {
    w.cam_page = p.cam.hi;
    w.cam_now = p.cam;
    w.ai_phase(stage, p.view, &mut |_, _, _| {});
    w.spawn_phase(stage, p.cam, o.dir);
    w.draw_phase(p.cam);
}

/// Compare a whole trace. Rows are logged at the end of each logic frame (see harness/gen_ptrace.js, OBJ=1).
///
/// The random generator (`$E4-$E7`) is stepped by its own task once per video frame, *after* the frame's logic, but the scheduler
/// sometimes skips or doubles a step when a frame runs long. The number of steps between two rows (normally 1) is therefore tried
/// as 1, 0, 2, 3 and the one that reproduces the row is taken; the logic itself (including everything the AI does with the
/// generator) must still match exactly, so a wrong random value is still caught.
/// Compare the whole game (player + camera + objects) with a trace: inputs come from the trace rows, everything else is simulated.
/// The random task's steps between rows are found by trying 1, 0, 2, 3 (see `run`).
pub fn run_game(rom: &Rom, stage: Stage, ptrace: &str, otrace: &str) -> bool {
    debug_column(&stage);
    let ptext = std::fs::read_to_string(ptrace).unwrap_or_else(|e| panic!("cannot read {}: {}", ptrace, e));
    let otext = std::fs::read_to_string(otrace).unwrap_or_else(|e| panic!("cannot read {}: {}", otrace, e));
    let prows: Vec<PRow> = ptext.lines().filter_map(parse_p).collect();
    let orows: Vec<ORow> = otext.lines().filter_map(parse_o).collect();
    assert!(prows.len() == orows.len(), "trace length differs: {} vs {}", prows.len(), orows.len());
    let initial: Option<Vec<(usize, Obj)>> = otext.lines().find(|l| l.starts_with("S ")).map(|l| {
        let f: Vec<&str> = l.split_whitespace().skip(1).collect();
        f.chunks(31).map(|c| (hex(c[0]) as usize, Obj::from_fields(&c[1..31].iter().map(|s| hex(s)).collect::<Vec<u8>>()))).collect()
    });
    let n = prows.len();
    let first = &prows[0];
    let mut p = Player::new(first.x, first.y, first.vy, first.state, first.facing);
    p.flip = first.flip;
    p.behind = stage.id == 1; // the stage-1 spawn set it before the recording started (code_3C86F1)
    p.life = first.life;
    p.inv = first.inv.unwrap_or(0);
    p.anim.id = first.anim.0;
    p.anim.frame = first.anim.1;
    if first.anim.0 != 0 {
        if let Some(k) = prows.iter().position(|r| r.anim.1 != first.anim.1) {
            p.anim.sync_tick(&stage.anims, k);
        }
    }
    let cam_x = ((first.cam.hi as i32) << 8) | first.cam.lo as i32;
    let mut cam = if cam_x < 0x200 { Camera::new(cam_x, first.x >> 8, &stage) } else { Camera::in_section(cam_x, first.x >> 8, &stage) };
    // MM4_SECT2B=hex: the recording forced $2B to this value (a teleport into a section in the state it has after its entry scroll)
    if let Ok(v) = std::env::var("MM4_SECT2B") {
        cam.sect = u8::from_str_radix(&v, 16).expect("MM4_SECT2B");
    }
    let mut g = Game::new(rom, stage, p, cam);
    g.camera.dir = orows[0].dir;
    if let Some(all) = &initial {
        for (slot, o) in all {
            g.world.o[*slot] = *o;
        }
    }
    for (slot, o) in &orows[0].objs {
        g.world.o[*slot] = *o;
    }
    if let Ok(d) = std::env::var("MM4_DOORS") {
        for p in d.split(',').filter_map(|p| p.parse::<usize>().ok()) {
            g.stage.door[p].set(0x7F); // shutters closed before the recording started (harness TPDOORS)
        }
    }
    g.snap_pics = std::env::var("MM4_SNAPS").is_ok();
    g.force_invincible = std::env::var("MM4_HURT").is_err(); // the recordings keep $3C non-zero unless made without INVINCIBLE
    if let Some(w) = std::env::var("MM4_WEAPON").ok().and_then(|v| v.parse::<u8>().ok()) {
        // the recording poked $A0 and the energy byte $B0+$A0 (harness POKE=A0=..,B?=..)
        g.player.weapon = w;
        if std::env::var("MM4_HUD").is_ok() {
            g.player.w131 = 0x80 | w; // the recording poked $0131 too
        }
        g.world.weapon = w;
        g.progress.weapons[w as usize] = std::env::var("MM4_WEN").ok().and_then(|v| u8::from_str_radix(&v, 16).ok()).unwrap_or(0x9C);
    }
    g.world.boss_hp = std::env::var("MM4_BOSSHP").ok().and_then(|v| v.parse().ok());
    // hidden stage-14 variables the recording poked at its start (POKE=AC=FF,AB=3,149=19): $AC, $AB, $0149
    for (var, f) in [("MM4_AC", 0usize), ("MM4_AB", 1), ("MM4_C149", 2)] {
        if let Ok(v) = std::env::var(var) {
            let v = u8::from_str_radix(&v, 16).expect(var);
            match f {
                0 => g.world.ac = v,
                1 => g.world.ab = v,
                _ => g.world.c149 = v,
            }
        }
    }
    g.world.frame92 = orows[0].f92;
    g.world.rng = orows[0].rng;
    g.world.p95 = orows[0].p95;
    g.world.c9e = orows[0].c9e;
    g.world.c9f = orows[0].c9f;
    // entries inside the spawner window that have no live object were defeated (or collected) before the first row: remember them
    {
        let (lo, hi) = (orows[0].c9f as usize, orows[0].c9e as usize);
        for i in lo..hi {
            if !(8..24).any(|s| g.world.o[s].idx == i as u8) {
                let mask = rom.cpu_byte(0x3E, 0x3F, 0xFCBF + (i & 7) as u16);
                g.world.defeated[i >> 3] |= mask;
            }
        }
    }
    // scroll registers and the column-renderer counter at the first row (xtrace_*.txt: $1B $1C $F0 $F1 $FA $FD $013E $013F), if recorded
    if let Ok(t) = std::fs::read_to_string(otrace.replace("otrace_", "xtrace_")) {
        if let Some(l) = t.lines().next() {
            let v: Vec<u8> = l.split_whitespace().skip(1).map(hex).collect();
            if v.len() >= 8 {
                let w = &mut g.world;
                (w.p1b, w.p1c, w.f0, w.f1, w.fa, w.fd, w.w13e, w.w13f) = (v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7]);
            }
            // the player's water counters `$43`/`$44` and the sprite priority line `$EF` (recorded for every stage but 4)
            if v.len() == 10 || v.len() == 11 || v.len() == 16 || v.len() == 80 {
                g.player.set_water(v[8], v[9]);
            }
            if v.len() == 11 || v.len() == 16 || v.len() == 80 {
                g.world.ef_override = Some(v[10]);
            }
            if v.len() == 16 || v.len() == 80 {
                // stage 8 ice state: $A3, $A5, $A6, $A7, $A8
                g.player.a3 = v[11];
                g.player.a56 = (v[13] as u16) << 8 | v[12] as u16;
                g.player.a78 = (v[15] as u16) << 8 | v[14] as u16;
            }
            // stage 4: the bar slots `$0120-$0123` and the bar bitmap `$0680-$06BF` (the recorder adds them when STAGE=4)
            if v.len() >= 12 {
                g.world.b120 = [v[8], v[9], v[10], v[11]];
            }
            if v.len() == 76 {
                for i in 0..64 {
                    g.stage.blocks[i].set(v[12 + i]);
                }
            }
            if v.len() == 80 {
                for i in 0..64 {
                    g.stage.blocks[i].set(v[16 + i]);
                }
            }
        }
    }

    let dump_slot: Option<usize> = std::env::var("MM4_DUMP").ok().and_then(|v| usize::from_str_radix(&v, 16).ok());
    // the OAM the game had at the end of each logic frame (atrace_*.txt), if recorded
    let atrace_path = otrace.replace("otrace_", "atrace_");
    let oam_rows: Vec<Vec<Sprite>> = std::fs::read_to_string(&atrace_path)
        .map(|t| {
            t.lines()
                .map(|l| {
                    let hexs = l.split_whitespace().nth(1).unwrap_or("");
                    let bytes: Vec<u8> = (0..hexs.len() / 2).map(|i| u8::from_str_radix(&hexs[2 * i..2 * i + 2], 16).unwrap_or(0)).collect();
                    (1..64).filter(|&k| bytes.len() >= k * 4 + 4 && bytes[k * 4] != 0xF8).map(|k| Sprite { y: bytes[k * 4], tile: bytes[k * 4 + 1], attr: bytes[k * 4 + 2], x: bytes[k * 4 + 3] }).collect()
                })
                .collect()
        })
        .unwrap_or_default();
    let xrows: Vec<Vec<u8>> = std::fs::read_to_string(otrace.replace("otrace_", "xtrace_")).map(|t| t.lines().map(|l| l.split_whitespace().skip(1).map(hex).collect()).collect()).unwrap_or_default();
    let cam_trace = std::env::var("MM4_CAM_TRACE").is_ok();
    // MM4_ANYY=1: keep comparing while Mega Man is on another Y screen (vertical sections); the default stops there
    let any_y = std::env::var("MM4_ANYY").is_ok();
    // sound requests the game made (harness/sndreq_snippet.lua, strace_*.txt: "<row> <C|E> <id> <return address>"; a music request logs a C line and then an E line)
    let strace: Option<std::collections::BTreeMap<i64, Vec<(char, u8, u16)>>> = std::fs::read_to_string(ptrace.replace("ptrace_", "strace_")).ok().map(|t| {
        let mut m: std::collections::BTreeMap<i64, Vec<(char, u8, u16)>> = std::collections::BTreeMap::new();
        for l in t.lines() {
            let p: Vec<&str> = l.split_whitespace().collect();
            if p.len() == 4 && (p[1] == "C" || p[1] == "E") {
                if let (Ok(tag), Ok(id), Ok(ret)) = (p[0].parse::<i64>(), u8::from_str_radix(p[2], 16), u16::from_str_radix(p[3], 16)) {
                    m.entry(tag).or_default().push((p[1].chars().next().unwrap(), id, ret));
                }
            }
        }
        m
    });
    let snd_shift: i64 = std::env::var("MM4_SND_SHIFT").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
    if let Some(sm) = &strace {
        // the music playing before the first row (the recording starts mid-game): `$D9` is the last C line before it
        if let Some(e) = sm.get(&-1).and_then(|v| v.iter().rev().find(|e| e.0 == 'C')) {
            g.world.d9 = e.1;
        }
    }
    let (mut snd_ok, mut snd_bad) = (0usize, 0usize);
    macro_rules! snd_summary {
        () => {
            if strace.is_some() {
                println!("sound requests: {} rows with requests identical, {} rows differ", snd_ok, snd_bad);
            }
        };
    }
    let mut ok = 0;
    for i in 1..n {
        let (r, o) = (&prows[i], &orows[i]);
        if std::env::var("MM4_PROG").is_ok() { eprintln!("row {}", r.t); }
        // the console uploads a frame's column/row builds in the next vblank: its nametable dump of row r equals the port's state after row r-1, i.e. before this row's step
        if let Ok(f) = std::env::var("MM4_NTLOG") {
            // gen_ptrace NTEVERY: the console's nametable (NT0) every n rows; report the rows where the ring differs
            static LOG: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
            let b = LOG.get_or_init(|| std::fs::read(&f).unwrap_or_default());
            // positions whose console value differs from the port's in 3 consecutive dumps (single dumps differ transiently while a column is being built)
            static PERSIST: std::sync::Mutex<(Vec<(usize, u8, u8, u8)>, Vec<usize>)> = std::sync::Mutex::new((Vec::new(), Vec::new()));
            let mut o = 0;
            while o + 4100 <= b.len() {
                let row = b[o] as i64 | (b[o + 1] as i64) << 8 | (b[o + 2] as i64) << 16;
                if row == r.t as i64 {
                    let d = &b[o + 4..o + 1028];
                    let mut st = PERSIST.lock().unwrap();
                    let mut now: Vec<(usize, u8, u8, u8)> = Vec::new();
                    for i in 0..960 {
                        if d[i] != g.nt.tile[i] {
                            let prev = st.0.iter().find(|p| p.0 == i && p.1 == d[i] && p.2 == g.nt.tile[i]).map_or(0, |p| p.3);
                            now.push((i, d[i], g.nt.tile[i], prev + 1));
                        }
                    }
                    let long: Vec<String> = now.iter().filter(|p| p.3 >= 3).map(|p| format!("({},{}):{:02X}/{:02X}x{}", p.0 / 32, p.0 % 32, p.1, p.2, p.3)).collect();
                    if !long.is_empty() && !long.iter().all(|x| st.1.iter().any(|&k| k == x.len())) || (!long.is_empty() && std::env::var("MM4_NTLOG_ALL").is_ok()) {
                        println!("NT row {}: {} persistent tiles (row,col:console/port x dumps): {}", row, long.len(), long.iter().take(std::env::var("MM4_NTLOG_MAX").ok().and_then(|v| v.parse().ok()).unwrap_or(12)).cloned().collect::<Vec<_>>().join(" "));
                    }
                    st.0 = now;
                }
                o += 4100;
            }
        }
        if prows[i - 1].state == 8 && g.stage_clear_due() {
            println!("stage clear at t={}: the state-8 countdown ended exactly when the game's did; the game goes on into the stage-end code ($8003, not ported); {} frames matched", r.t, ok);
            snd_summary!();
print_routines(&g);
            return true;
        }
        if !matches!(r.state, 0..=9 | 0x0A | 0x0B | 0x0C | 0x0F | 0x10 | 0x11) || (r.view.ypg != 0 && !(matches!(r.state, 8 | 0x11) && r.view.ypg == 0xFF) && !(any_y && !matches!(r.state, 7))) {
            println!("stopped at t={}: the game left the ported states or screen 0 (state ${:02X}, Y screen {}); {} frames matched", r.t, r.state, r.view.ypg, ok);
            snd_summary!();
print_routines(&g);
            return true;
        }
        let b = Buttons { held: r.held, pressed: r.pressed };
        let mut matched = None;
        let mut first_try: Option<(Game, String)> = None;
        let d92 = o.f92.wrapping_sub(orows[i - 1].f92) as usize;
        let mut ks = vec![d92, 1, 0, 2, 3];
        if d92 > 20 {
            ks.extend((1..=10).map(|d| d92 - d)); // a long scene change: the sound start makes the random task skip a few video frames
        }
        ks.dedup();
        // When a logic frame is long (many objects) a video frame boundary can fall inside it: then the AI saw the random generator and the
        // frame counter `$92` of before that boundary. Try: all `k` steps before the frame (normal), or only `j` of them before and the rest
        // after it, and `$92` as logged or one less.
        // If none of those match, the video-frame boundary may also have fallen between two objects (or between an object's AI and its shot test):
        // `pos` = 2 * slot (before its AI) or 2 * slot + 1 (after its AI), one step there, `j` before the frame, the rest after it.
        let positions: Vec<Option<usize>> = std::iter::once(None).chain((2..48).map(Some)).collect();
        'search: for pos in positions {
          for k in ks.clone() {
            if pos.is_some() && k == 0 {
                continue;
            }
            for j in (0..=k.min(24)).rev() {
                if pos.is_some() && j >= k {
                    continue;
                }
                for f92 in [o.f92, o.f92.wrapping_sub(1)] {
                    let mut t = g.fork();
                    t.world.nmi_pos = pos;
                    if cam_trace {
                        t.camera.x = ((prows[i - 1].cam.hi as i32) << 8) | prows[i - 1].cam.lo as i32;
                        t.camera.force_x = Some(((r.cam.hi as i32) << 8) | r.cam.lo as i32);
                    }
                    t.world.frame92 = f92;
                    t.step(b, j);
                    let mut snd_acc = t.snd_out.clone();
                    // a section change runs its 120-iteration scroll loop inside this one logic frame
                    let mut guard = 0;
                    while t.camera.busy() {
                        t.step(Buttons::default(), 0);
                        snd_acc.extend(t.snd_out.iter().copied());
                        guard += 1;
                        if guard > 2000 {
                            panic!("scroll loop does not end: trans {:?} door_wait {} kind {} sect {:02X} x {:04X}", t.camera.busy(), t.camera.door_wait, t.camera.door_kind, t.camera.sect, t.camera.x);
                        }
                    }
                    t.snd_out = snd_acc;
                    let used = t.world.loop_steps + pos.is_some() as usize;
                    for _ in 0..(k - j).saturating_sub(used) {
                        t.world.rng_step();
                        t.world.flush_blocks(&t.stage);
                    }
                    if k == 0 {
                        // the random task can skip a video frame (sound start) while the column renderer still runs
                        t.world.col_due = t.world.col_due.saturating_add(d92 as u8);
                    }
                    t.world.apply_col_steps();
                    if t.game_over {
                        println!("stopped at t={}: game over (no lives left; the continue screens are not ported); {} frames matched", r.t, ok);
                        snd_summary!();
print_routines(&g);
                        return true;
                    }
                    if t.restarted {
                        t.world.rng = o.rng; // the restart takes an unknown number of video frames
                    }
                    t.world.frame92 = o.f92;
                    match compare_game(&t, r, o, xrows.get(i)) {
                        None => {
                            matched = Some(t);
                            break 'search;
                        }
                        Some(msg) => {
                            if std::env::var("MM4_TRIES").is_ok() {
                                eprintln!("t={} try pos={:?} k={} j={} f92={:02X}: {}", r.t, pos, k, j, f92, msg.replace(char::from(10), " | "));
                            }
                            if first_try.is_none() {
                                first_try = Some((t, msg));
                            }
                        }
                    }
                }
            }
          }
        }
        match matched {
            Some(t) => {
                g = t;
                if let Some(sm) = &strace {
                    // the sound requests of this logic frame against the console's (a music request is a C line plus an E line)
                    let mut mine: Vec<(char, u8, u16)> = Vec::new();
                    for q in &g.snd_out {
                        if q.music {
                            mine.push(('C', q.id, q.ret));
                        }
                        mine.push(('E', q.id, q.ret));
                    }
                    let want = sm.get(&(r.t as i64 + snd_shift)).cloned().unwrap_or_default();
                    if mine == want {
                        snd_ok += (!want.is_empty()) as usize;
                    } else {
                        snd_bad += 1;
                        if snd_bad <= std::env::var("MM4_SND_MAX").ok().and_then(|v| v.parse().ok()).unwrap_or(8) {
                            let f = |v: &Vec<(char, u8, u16)>| v.iter().map(|(k, i, r)| format!("{}{:02X}@{:04X}", k, i, r)).collect::<Vec<_>>().join(" ");
                            println!("SOUND differs at t={}: game [{}] port [{}]", r.t, f(&want), f(&mine));
                        }
                    }
                }
            }
            None => {
                let (t, msg) = first_try.unwrap();
                println!("MISMATCH at t={} (pressed {:02X} held {:02X}) after {} matching frames\n{}", r.t, r.pressed, r.held, ok, msg);
                if !t.world.contacts.is_empty() {
                    println!(
                        "  port: objects touching Mega Man: {:?} (types {:02X?}), inv={} state={}",
                        t.world.contacts,
                        t.world.contacts.iter().map(|&s| t.world.o[s].ty).collect::<Vec<_>>(),
                        t.player.inv,
                        t.player.state
                    );
                }
                if let Some(sl) = dump_slot {
                    let gm = o.objs.iter().find(|(s, _)| *s == sl).map(|(_, ob)| ob.fields());
                    println!("  slot {:02X} game {:02X?}\n  slot {:02X} port {:02X?}", sl, gm, sl, t.world.o[sl].fields());
                }
                return false;
            }
        }
        // the frame in which the auto-walk (state $10) ends draws extra sprites in the game that are not explained yet: not compared
        if let Some(game_spr) = oam_rows.get(i).filter(|_| r.ptype != 0 && prows[i - 1].state != 0x10) {
            let mut port_spr = g.sprites();
            port_spr.retain(|s| s.y != 0xF8); // Y = $F8 hides a sprite; the recording filters those out too
            if &port_spr != game_spr {
                println!("MISMATCH at t={} after {} matching frames: sprites (OAM) differ", r.t, ok);
                println!("  port player: inv={} state={} anim=({:02X},{}) y={:06X} flip={}", g.player.inv, g.player.state, g.player.anim.id, g.player.anim.frame, g.player.y, g.player.flip);
                println!("  game: {:?}", game_spr.iter().map(|s| (s.y, s.x, s.tile, s.attr)).collect::<Vec<_>>());
                println!("  port: {:?}", port_spr.iter().map(|s| (s.y, s.x, s.tile, s.attr)).collect::<Vec<_>>());
                return false;
            }
        }
        if std::env::var("MM4_NTWATCH").is_ok() && r.t >= 28 && r.t <= 36 {
            eprintln!("row {} nt(12,2)={:02X} c24 {:02X} c2a {:02X} cam {:04X}", r.t, g.nt.tile[12 * 32 + 2], g.camera.c24, g.camera.c2a, g.camera.x);
        }
        if let (Ok(dir), true) = (std::env::var("MM4_SHOTDIR"), !g.snaps.is_empty()) {
            for (k, f) in g.snaps.iter().enumerate() {
                let mut bytes = format!("P6
{} {}
255
", crate::stage::SCREEN_W, crate::stage::SCREEN_H).into_bytes();
                for px in f {
                    bytes.extend_from_slice(&[(px >> 16) as u8, (px >> 8) as u8, *px as u8]);
                }
                std::fs::write(format!("{}/rs_{}_{}.ppm", dir, r.t, k), bytes).unwrap();
                std::fs::write(format!("{}/rs_{}_{}.pal", dir, r.t, k), g.snap_pals[k].iter().map(|b| format!("{:02X}", b)).collect::<String>()).unwrap();
                std::fs::write(format!("{}/rs_{}_{}.nt", dir, r.t, k), &g.snap_nts[k]).unwrap();
                let c = g.snap_cams[k];
                std::fs::write(format!("{}/rs_{}_{}.cam", dir, r.t, k), format!("{:02X} {:02X} {:04X}", c.0, c.1, c.2)).unwrap();
            }
            g.snaps.clear();
            g.snap_pals.clear();
            g.snap_nts.clear();
            g.snap_cams.clear();
        }
        if let Ok(dir) = std::env::var("MM4_SHOTDIR") {
            // MM4_SHOTS=a-b: the picture of every row in a..b as PPM (compare with the screenshots of gen_ptrace.js SHOTS through tools/shotcmp.js)
            let (sa, sb) = std::env::var("MM4_SHOTS").ok().and_then(|v| { let mut it = v.split('-'); Some((it.next()?.parse::<i64>().ok()?, it.next()?.parse::<i64>().ok()?)) }).unwrap_or((0, 0));
            if (r.t as i64) >= sa && (r.t as i64) <= sb {
                let st = g.stage.clone();
                let img = st.render_idx().2;
                let mut frame = vec![0u32; crate::stage::SCREEN_W * crate::stage::SCREEN_H];
                crate::flowrun::render_play_with(&mut frame, &img, &st, &mut g, true, false);
                let mut bytes = format!("P6
{} {}
255
", crate::stage::SCREEN_W, crate::stage::SCREEN_H).into_bytes();
                for px in &frame {
                    bytes.extend_from_slice(&[(px >> 16) as u8, (px >> 8) as u8, *px as u8]);
                }
                std::fs::write(format!("{}/rs_{}.ppm", dir, r.t), bytes).unwrap();
            }
        }
        ok += 1;
        if let (Ok(f), Ok(row)) = (std::env::var("MM4_NTINIT"), std::env::var("MM4_NTROW")) {
            // the console's nametable (NT0 from chain/gen_ptrace NTDUMP) replaces the ring at that row: recordings made after a teleport start with a stale nametable
            if row.parse::<i64>().ok() == Some(r.t as i64) {
                let b = std::fs::read(f).expect("cannot read the nametable dump");
                g.nt.tile.copy_from_slice(&b[..960]);
                g.nt.attr.copy_from_slice(&b[960..1024]);
                g.nt.shadow.copy_from_slice(&b[960..1024]);
                if b.len() >= 1026 {
                    // the console's builder column `$24` and page `$2A` at that row (the teleport poked them and the camera moved on)
                    g.camera.c24 = b[1024];
                    g.camera.c2a = b[1025];
                }
            }
        }
    }
    println!("all {} frames identical to the game (player, camera and {} objects alive at the end)", ok, g.world.o.iter().filter(|o| o.ty != 0).count());
    snd_summary!();
print_routines(&g);
    if !g.world.unported.is_empty() {
        println!("note: unported AI routines were reached: {:04X?}", g.world.unported);
    }
    true
}

/// Player + camera comparison (as in verify.rs) followed by the object comparison.
fn compare_game(g: &Game, r: &PRow, o: &ORow, x: Option<&Vec<u8>>) -> Option<String> {
    let p = &g.player;
    let mut bad: Vec<String> = Vec::new();
    if std::env::var("MM4_ANIM").is_ok() {
        eprintln!("t={} port state={} anim=({:02X},{},{}) game anim={:?} hdr={:?} y={:06X} gamey={:06X} vy={:04X}", r.t, p.state, p.anim.id, p.anim.frame, p.anim.tick, r.anim, g.stage.anims.header(p.anim.id), p.y, r.y, p.vy as u16);
    }
    if p.x != r.x || p.y != r.y || p.vy != r.vy || p.state != r.state || p.facing != r.facing || g.camera.x != (((r.cam.hi as i32) << 8) | r.cam.lo as i32) {
        bad.push(format!(
            "player game x={:06X} y={:06X} vy={:04X} state={} facing={} cam={:02X}{:02X}\n  player port x={:06X} y={:06X} vy={:04X} state={} facing={} cam={:04X}",
            r.x, r.y, r.vy as u16, r.state, r.facing, r.cam.hi, r.cam.lo, p.x, p.y, p.vy as u16, p.state, p.facing, g.camera.x
        ));
    }
    let (id, frame) = r.anim;
    if p.life != r.life {
        bad.push(format!("player life game {:02X} port {:02X}", r.life, p.life));
    }
    let frame_check = std::env::var("MM4_CAM_TRACE").is_err(); // after a teleport the freeze counter `$AD` of the recording is unknown
    if p.anim.id != id || (frame_check && matches!(id, 0x01 | 0x0D..=0x0F | 0x13 | 0x33) && p.anim.frame != frame) {
        bad.push(format!("player anim (id,frame) game ({:02X},{}) port ({:02X},{})", id, frame, p.anim.id, p.anim.frame));
    }
    // the column-renderer counter `$1B` and `$F0` (the stage-4 machine waits for the first to reach 0 and sets the second)
    if let Some(v) = x {
        if v.len() >= 3 && (g.world.p1b != v[0] || g.world.f0 != v[2]) {
            bad.push(format!("$1B/$F0 game {:02X}/{:02X} port {:02X}/{:02X}", v[0], v[2], g.world.p1b, g.world.f0));
        }
    }
    if let Some(m) = compare(&g.world, o) {
        bad.push(m);
    }
    if bad.is_empty() {
        None
    } else {
        Some(bad.join("\n"))
    }
}

fn print_routines(g: &Game) {
    let cov: Vec<String> = g.world.stats.iter().map(|(p, n)| format!("{:04X}x{}", p, n)).collect();
    println!("routines run: {}", cov.join(" "));
}

pub fn run(rom: &Rom, stage: &Stage, ptrace: &str, otrace: &str) -> bool {
    let ptext = std::fs::read_to_string(ptrace).unwrap_or_else(|e| panic!("cannot read {}: {}", ptrace, e));
    let otext = std::fs::read_to_string(otrace).unwrap_or_else(|e| panic!("cannot read {}: {}", otrace, e));
    let prows: Vec<PRow> = ptext.lines().filter_map(parse_p).collect();
    let orows: Vec<ORow> = otext.lines().filter_map(parse_o).collect();
    let initial: Option<Vec<(usize, Obj)>> = otext.lines().find(|l| l.starts_with("S ")).map(|l| {
        let f: Vec<&str> = l.split_whitespace().skip(1).collect();
        f.chunks(31).map(|c| (hex(c[0]) as usize, Obj::from_fields(&c[1..31].iter().map(|s| hex(s)).collect::<Vec<u8>>()))).collect()
    });
    assert!(prows.len() == orows.len(), "trace length differs: {} vs {}", prows.len(), orows.len());
    let n = prows.len();
    let dump_slot: Option<usize> = std::env::var("MM4_DUMP").ok().and_then(|v| usize::from_str_radix(&v, 16).ok());
    let dump_from: i32 = std::env::var("MM4_DUMP_FROM").ok().and_then(|v| v.parse().ok()).unwrap_or(0);

    let mut w = World::new(rom);
    let first = &orows[0];
    if let Some(all) = &initial {
        for (slot, o) in all {
            w.o[*slot] = *o;
        }
    }
    for (slot, o) in &first.objs {
        w.o[*slot] = *o;
    }
    w.rng = first.rng;
    w.p95 = first.p95;
    w.c9e = first.c9e;
    w.c9f = first.c9f;

    // sound requests the game made (harness/sndreq_snippet.lua, strace_*.txt: "<row> <C|E> <id> <return address>"); a music request logs a C line and then an E line
    let strace: Option<std::collections::BTreeMap<i64, Vec<(u8, u16)>>> = std::fs::read_to_string(ptrace.replace("ptrace_", "strace_")).ok().map(|t| {
        let mut m: std::collections::BTreeMap<i64, Vec<(u8, u16)>> = std::collections::BTreeMap::new();
        for l in t.lines() {
            let p: Vec<&str> = l.split_whitespace().collect();
            if p.len() == 4 && (p[1] == "C" || p[1] == "E") {
                if let (Ok(tag), Ok(id), Ok(ret)) = (p[0].parse::<i64>(), u8::from_str_radix(p[2], 16), u16::from_str_radix(p[3], 16)) {
                    m.entry(tag).or_default().push((id | 0, ret));
                    if p[1] == "C" {
                        m.get_mut(&tag).unwrap().last_mut().unwrap().1 = ret; // the E line that follows has the same id and return address
                    }
                }
            }
        }
        m
    });
    let snd_shift: i64 = std::env::var("MM4_SND_SHIFT").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
    let (mut snd_ok, mut snd_bad) = (0usize, 0usize);
    let mut ok = 0;
    let mut contact_at = None;
    for i in 1..n {
        let (p, o) = (&prows[i], &orows[i]);
        assert!(p.t == o.t);
        if p.state > 3 && !matches!(p.state, 6 | 8 | 9 | 0x0A | 0x0B) {
            println!("stopped at t={}: Mega Man is in state ${:02X} (death / respawn is not ported); {} frames matched", p.t, p.state, ok);
            return true;
        }
        let mut matched = None;
        let mut first_try: Option<(World, String)> = None;
        let d92 = o.f92.wrapping_sub(orows[i - 1].f92) as usize;
        let mut ks = vec![d92, 1, 0, 2, 3];
        ks.dedup();
        for k in ks {
            let mut t = w.clone();
            t.frame92 = o.f92;
            t.invincible = true;
            for _ in 0..k {
                t.rng_step();
                t.flush_blocks(stage);
            }
            frame(&mut t, stage, p, o);
            match compare(&t, o) {
                None => {
                    matched = Some(t);
                    break;
                }
                Some(msg) => {
                    if first_try.is_none() {
                        first_try = Some((t, msg));
                    }
                }
            }
        }
        match matched {
            Some(t) => w = t,
            None => {
                let (t, msg) = first_try.unwrap();
                if let Some(sl) = dump_slot {
                    let g = o.objs.iter().find(|(s, _)| *s == sl).map(|(_, ob)| ob.fields());
                    println!("t={} game {:02X?}\n      port {:02X?}", o.t, g, t.o[sl].fields());
                }
                println!("MISMATCH at t={} after {} matching frames\n{}", o.t, ok, msg);
                return false;
            }
        }
        if let Some(sl) = dump_slot {
            if p.t > dump_from {
                let g = o.objs.iter().find(|(s, _)| *s == sl).map(|(_, ob)| ob.fields());
                println!("t={} game {:02X?}\n      port {:02X?}", o.t, g, w.o[sl].fields());
            }
        }
        if !w.contacts.is_empty() && contact_at.is_none() {
            contact_at = Some(p.t);
            println!("note: an object touches Mega Man at t={} (damage is not ported; the comparison continues)", p.t);
        }
        ok += 1;
    }
    println!("all {} frames identical to the game ({} objects seen alive at the end)", ok, w.o.iter().filter(|o| o.ty != 0).count());
    let cov: Vec<String> = w.stats.iter().map(|(p, n)| format!("{:04X}x{}", p, n)).collect();
    println!("routines run: {}", cov.join(" "));
    if !w.unported.is_empty() {
        println!("note: unported AI routines were reached: {:04X?}", w.unported);
    }
    true
}

/// Debug: the tiles of world column `col` of `page` as the builder would write them (MM4_NTCOL=page,col).
pub fn debug_column(st: &Stage) {
    if let Ok(v) = std::env::var("MM4_NTCOL") {
        let p: Vec<usize> = v.split(',').filter_map(|x| x.parse().ok()).collect();
        let (tiles, _) = crate::nametable::NameTable::column(st, p[0] as u8, p[1]);
        println!("column page {} col {}: {}", p[0], p[1], tiles.iter().enumerate().map(|(i, t)| format!("{}:{:02X}", i, t)).collect::<Vec<_>>().join(" "));
    }
}
