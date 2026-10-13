//! The stage-select screen of bank `$39` (`code_39809D` .. `code_398237`, `code_398495` for scene `$10`, graphics set `$10`) drawn from the ROM.
//! Built the way the game builds it: nametable = scene `$10` screen 0, palette row `$872E + 64` faded in by `code_3EC451`, the cursor frame sprites (`$88BF`),
//! the eight portraits as sprites (`$8C3F`/`$8C47` lists) or, for a defeated boss, blanked tiles (`$8C4F` records, one frame each), and with every boss down
//! the flashing backdrop (`code_3982FF`) and Dr. Wily's emblem (`$88D8` sprites, `$887D` tiles). The loop (`code_398237`) moves the frame on direction presses
//! and blinks it with `$92 & 8`. Checked frame by frame against Mesen dumps (`mm4 --verify-selectscreen`, `harness/select_frames.lua`).
use crate::gfx;
use crate::rom::Rom;
use crate::screen::Screen;
use crate::select::{BTN_DOWN, BTN_LEFT, BTN_RIGHT, BTN_UP};
use crate::stage::Stage;

/// Bank `$39` window (`$8000-$9FFF` = bank `$39`, `$A000-$BFFF` = bank `$3A`).
fn b39(rom: &Rom, a: u16) -> u8 {
    rom.cpu_byte(0x39, 0x3A, a)
}

pub struct SelectScreen {
    pub screen: Screen,
    /// `$0620`: the palette the fade-in works towards.
    target: [u8; 32],
    /// `$A9`: boss-defeated bits.
    a9: u8,
    /// `$10` (0..2) and `$11` (0, 3, 6).
    pub col: u8,
    pub row: u8,
    /// Video frames since the screen was built (the first frame of the fade-in sequence is 0).
    t: u32,
    /// Buffer records still waiting for their frame (one per defeated boss, applied in bit order).
    pending: Vec<Vec<u8>>,
    /// Portrait sprites appended after the flush frames of the defeated bosses before them: (frame, OAM offset, bytes).
    late: Vec<(u32, usize, Vec<u8>)>,
    /// `$95` while the all-bosses flash runs.
    p95: u8,
    wily_done: bool,
    /// A buffer written this frame reaches the nametable in the next frame's NMI.
    next_buf: Option<Vec<u8>>,
}

impl SelectScreen {
    /// `code_398495` for `$10 = 2`: everything that happens with the screen blanked. The palette starts black.
    pub fn new(rom: &Rom, a9: u8) -> SelectScreen {
        let menu = Stage::load(rom, 0x10, 5);
        let mut s = Screen::new();
        let (tiles, attrs) = menu.nametable(0);
        s.nt[..960].copy_from_slice(&tiles);
        s.nt[960..].copy_from_slice(&attrs);
        // the pattern RAM still holds what the intro loaded before (set $55: Mega Man's sprite tiles at $1000-$1FFF, then the CAPCOM scene $11)
        gfx::load_set(rom, 0x55, &mut s.chr);
        gfx::load_set(rom, 0x11, &mut s.chr);
        gfx::load_set(rom, 0x10, &mut s.chr);
        let mut target = [0u8; 32];
        for (i, t) in target.iter_mut().enumerate() {
            *t = b39(rom, 0x872E + 64 + i as u16);
        }
        // the frame sprites: the list at `$88BF` (pointer `$8856 + 2` / `$885A + 2`), up to the `$FF`
        let mut n = 0usize;
        let mut p = 0x88BFu16;
        while b39(rom, p) != 0xFF {
            s.oam[n] = b39(rom, p);
            n += 1;
            p += 1;
        }
        // the portraits: sprites for a boss still up, a blanked picture (next frame) for a defeated one
        let mut pending = Vec::new();
        let mut late = Vec::new();
        let mut bits = a9;
        for x in 0..8usize {
            let defeated = bits & 1 != 0;
            bits >>= 1;
            if defeated {
                let hi = b39(rom, 0x8C74 + x as u16);
                let lo = b39(rom, 0x8C6C + x as u16);
                let mut rec = Vec::new();
                for k in 0..4u16 {
                    rec.extend_from_slice(&[hi, lo.wrapping_add(0x20 * k as u8), 3, 0, 0, 0, 0]);
                }
                rec.push(0xFF);
                pending.push(rec);
            } else {
                let mut q = b39(rom, 0x8C3F + x as u16) as u16 | (b39(rom, 0x8C47 + x as u16) as u16) << 8;
                let mut bytes = Vec::new();
                while b39(rom, q) != 0xFF {
                    bytes.push(b39(rom, q));
                    q += 1;
                }
                let d = pending.len() as u32;
                let len = bytes.len();
                if d == 0 {
                    s.oam[n..n + len].copy_from_slice(&bytes);
                } else {
                    late.push((d, n, bytes));
                }
                n += len;
            }
        }
        s.pal = [0x0F; 32];
        SelectScreen { screen: s, target, a9, col: 1, row: 3, t: 0, pending, late, p95: 0, wily_done: false, next_buf: None }
    }

    /// `code_3EC451` with `$64 = 0`: every palette byte lowered by `level` (`$30, $20, $10, 0`), values that would go negative become black.
    fn faded(&self, level: u8) -> [u8; 32] {
        let mut p = self.target;
        for v in p.iter_mut() {
            let d = v.wrapping_sub(level);
            *v = if d & 0x80 == 0 { d } else { 0x0F };
        }
        p
    }

    /// The first frame of the loop (`code_3980B7` entered at `$10 = 1, $11 = 3`), counted from the first frame of the screen.
    pub fn loop_start(&self) -> u32 {
        17 + self.a9.count_ones()
    }

    /// The loop reads the buttons (not before the loop starts, not during the all-bosses flash).
    pub fn accepts_input(&self) -> bool {
        self.t >= self.loop_start() && (self.a9 != 0xFF || self.wily_done)
    }

    /// One video frame. `pressed` = the game's newly-pressed byte `$14`, `frame92` = `$92` (blinks the frame). `t = 0` is the frame the screen was built in.
    /// Not to be called in the frame where Start/A leaves the screen.
    pub fn frame(&mut self, rom: &Rom, pressed: u8, frame92: u8) {
        let t = self.t;
        self.t += 1;
        self.screen.snd.clear();
        if t == self.a9.count_ones() {
            // code_398585: scene 2's music, after the one-frame buffer flush of every defeated boss's portrait
            self.screen.snd.push(crate::sound::Req { music: true, id: b39(rom, 0x8879 + 2), ret: 0x858D });

        }
        if let Some(b) = self.next_buf.take() {
            self.screen.apply_buffer(&b);
        }
        // a defeated boss's portrait is blanked by one buffer flush per frame
        if t >= 1 && ((t - 1) as usize) < self.pending.len() {
            let rec = self.pending[(t - 1) as usize].clone();
            self.screen.apply_buffer(&rec);
        }
        for (d, n, bytes) in &self.late {
            if *d == t {
                self.screen.oam[*n..*n + bytes.len()].copy_from_slice(bytes);
            }
        }
        // the fade-in: four steps of four frames, the first one a frame after the last flush
        let first = 2 + self.a9.count_ones();
        if t >= first {
            let u = t - first;
            let level = match u / 4 {
                0 => 0x30,
                1 => 0x20,
                2 => 0x10,
                _ => 0x00,
            };
            self.screen.pal = self.faded(level);
        }
        let start = self.loop_start();
        if t < start {
            return;
        }
        let k = t - start;
        if self.a9 == 0xFF && !self.wily_done {
            // code_3982FF: forty frames of a flashing backdrop (the palette written in a frame shows in the next one), then the emblem
            if k > 0 {
                let c = [0x20u8, 0x0F][(((k - 1) >> 2) & 1) as usize];
                self.screen.pal[0] = c;
                self.screen.pal[16] = c;
            }
            self.p95 = k as u8;
            if k == 40 {
                for i in 0..32 {
                    self.screen.oam[0x20 + i] = b39(rom, 0x88D8 + i as u16);
                }
                let mut buf = Vec::new();
                let mut p = 0x887Du16;
                loop {
                    let v = b39(rom, p);
                    buf.push(v);
                    if v == 0xFF {
                        break;
                    }
                    p += 1;
                }
                self.next_buf = Some(buf);
                self.wily_done = true;
            } else {
                return;
            }
        }
        self.cursor(rom, pressed, frame92);
    }

    /// `code_398237`: the grid cursor and the blinking frame (sprites 0-5).
    fn cursor(&mut self, rom: &Rom, pressed: u8, frame92: u8) {
        let o = &mut self.screen.oam;
        if pressed & (BTN_RIGHT | BTN_LEFT) != 0 || pressed & (BTN_UP | BTN_DOWN) != 0 {
            if pressed & (BTN_RIGHT | BTN_LEFT) != 0 {
                if pressed & BTN_RIGHT != 0 {
                    self.col = (self.col + 1) % 3;
                } else {
                    self.col = if self.col == 0 { 2 } else { self.col - 1 };
                }
            } else if pressed & crate::select::BTN_UP != 0 {
                self.row = if self.row < 3 { 6 } else { self.row - 3 };
            } else {
                self.row = (self.row + 3) % 9;
            }
            let i = (self.col + self.row) as u16;
            let y = b39(rom, 0x885E + i);
            for s in [0usize, 1, 2] {
                o[s * 4] = y;
            }
            for s in [3usize, 4, 5] {
                o[s * 4] = y.wrapping_add(0x28);
            }
            let x = b39(rom, 0x8867 + i);
            o[3] = x;
            o[0x0F] = x;
            o[7] = x.wrapping_add(0x13);
            o[0x13] = x.wrapping_add(0x13);
            o[0x0B] = x.wrapping_add(0x26);
            o[0x17] = x.wrapping_add(0x26);
            self.screen.snd.push(crate::sound::Req { music: false, id: 0x2E, ret: 0x82BF }); // code_3982BC
        }

        if frame92 & 0x08 != 0 {
            for s in 0..6 {
                o[s * 4 + 1] = b39(rom, 0x88C0 + 4 * s as u16);
            }
        } else {
            for s in 0..6 {
                o[s * 4 + 1] = 0x15;
            }
        }
    }
}

/// The menu input of `harness/select_frames.lua` (the title/menu script of `gen_ptrace.js`): buttons held in video frame `f`.
fn held(f: i64, walk: bool) -> u8 {
    if f < 0 {
        return 0;
    }
    if walk && f >= 612 {
        // select_frames.lua SF_WALK: one button (or two) for 3 of every 6 frames
        const WALK: [u8; 16] = [0x01, 0x01, 0x04, 0x02, 0x02, 0x02, 0x08, 0x08, 0x08, 0x08, 0x01, 0x04, 0x04, 0x04, 0x05, 0x0A];
        let (i, w) = (((f - 612) / 6) as usize, (f - 612) % 6);
        return if i < WALK.len() && w < 3 { WALK[i] } else { 0 };
    }
    let m = f % 160;
    if m < 20 {
        0x10
    } else if (60..80).contains(&m) {
        0x80
    } else if (100..110).contains(&m) {
        BTN_RIGHT
    } else if (130..140).contains(&m) {
        BTN_DOWN
    } else {
        0
    }
}

/// Compare with a `select_frames.lua` recording (`F`, `PAL`, `OAM`, `NT0` lines per frame, `CHR` at the end): the screen is built at frame 592 (the first frame
/// with `$10 = 2`), then stepped once per recorded frame with the menu script's newly pressed buttons and the recorded `$92`. Stops before a frame where Start/A leaves.
pub fn verify(rom: &Rom, path: &str, a9: u8, walk: bool) -> bool {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("cannot read {}: {}", path, e);
            return false;
        }
    };
    let bytes = |s: &str| -> Vec<u8> { (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap_or(0)).collect() };
    let mut sel = SelectScreen::new(rom, a9);
    let (mut bad, mut checked, mut stopped) = (0, 0, false);
    struct Rec {
        frame: i64,
        p92: u8,
        pal: Vec<u8>,
        oam: Vec<u8>,
        nt: Vec<u8>,
    }
    let mut recs: Vec<Rec> = Vec::new();
    let mut chr: Vec<u8> = Vec::new();
    for l in text.lines() {
        if let Some(rest) = l.strip_prefix("F ") {
            let frame = rest.split_whitespace().next().and_then(|v| v.parse().ok()).unwrap_or(-1);
            let p92 = rest.split_whitespace().find_map(|w| w.strip_prefix("p92=")).and_then(|v| u8::from_str_radix(v, 16).ok()).unwrap_or(0);
            recs.push(Rec { frame, p92, pal: vec![], oam: vec![], nt: vec![] });
        } else if let Some(v) = l.strip_prefix("PAL ") {
            if let Some(r) = recs.last_mut() {
                r.pal = bytes(v);
            }
        } else if let Some(v) = l.strip_prefix("OAM ") {
            if let Some(r) = recs.last_mut() {
                r.oam = bytes(v);
            }
        } else if let Some(v) = l.strip_prefix("NT0 ") {
            if let Some(r) = recs.last_mut() {
                r.nt = bytes(v);
            }
        } else if let Some(v) = l.strip_prefix("CHR ") {
            chr = bytes(v);
        }
    }
    let mut last_nt: Vec<u8> = Vec::new();
    for r in &recs {
        if !r.nt.is_empty() {
            last_nt = r.nt.clone();
        }
        if r.frame < 592 {
            continue;
        }
        let f = r.frame;
        let pressed = held(f, walk) & !held(f - 1, walk);
        if sel.accepts_input() && pressed & 0x90 != 0 {
            println!("frame {}: Start/A leaves the screen (stop)", f);
            stopped = true;
            break;
        }
        sel.frame(rom, pressed, r.p92);
        checked += 1;
        let ours = &sel.screen;
        let pd = r.pal.iter().zip(ours.pal.iter()).filter(|(a, b)| a != b).count();
        let od: Vec<String> = r.oam.chunks(4).zip(ours.oam.chunks(4)).enumerate().filter(|(_, (a, b))| a != b && !(a[0] >= 0xEF && b[0] >= 0xEF)).map(|(i, (a, b))| format!("#{}:{:02X?}/{:02X?}", i, a, b)).collect();
        let nd = last_nt.iter().zip(ours.nt.iter()).filter(|(a, b)| a != b).count();
        if pd != 0 || !od.is_empty() || nd != 0 {
            bad += 1;
            if bad <= 12 {
                println!("frame {} (t {}): palette {} differ, sprites {} differ {}, nametable {} differ", f, f - 592, pd, od.len(), od.iter().take(4).cloned().collect::<Vec<_>>().join(" "), nd);
            }
        }
    }
    if !chr.is_empty() && !stopped {
        let cd = chr.iter().zip(sel.screen.chr.iter()).filter(|(a, b)| a != b).count();
        println!("pattern RAM: {} of {} bytes differ", cd, chr.len());
        if cd != 0 {
            let mut tiles: Vec<usize> = chr.iter().zip(sel.screen.chr.iter()).enumerate().filter(|(_, (a, b))| a != b).map(|(i, _)| i / 16).collect();
            tiles.dedup();
            println!("  differing tiles (offset/16): {:?}", &tiles[..tiles.len().min(60)]);
        }
        if cd != 0 {
            bad += 1;
        }
    }
    println!("{}: {} frames compared, {} frames differ", path, checked, bad);
    bad == 0
}
