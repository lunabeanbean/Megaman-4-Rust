//! The screen after a cleared stage (and after a game over): the password grid with the stage-select / continue menu (bank `$39`: `code_398F49`, `code_399886`,
//! `code_398E8F`). Scene `$11` screen 1, graphics set `$17`, palette `$9509`, one text buffer from `$952E` (`code_3992C9`), the 36 grid dots as sprites
//! (`$98FA`, tile 1 = a red dot, chosen from the weapons and items the player owns) and the arrow cursor (`$0290`). Checked frame by frame against Mesen
//! (`mm4 --verify-passwordscreen`, `harness/ppu_frames.lua` with `PF_HIJACK`).
use crate::gfx;
use crate::rom::Rom;
use crate::screen::Screen;
use crate::select::{BTN_A, BTN_DOWN, BTN_SELECT, BTN_START, BTN_UP};
use crate::stage::Stage;

fn b39(rom: &Rom, a: u16) -> u8 {
    rom.cpu_byte(0x39, 0x3A, a)
}

/// Which screen the scene shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PasswordKind {
    /// After a cleared stage (`code_398E78`): the menu is there at once.
    StageClear,
    /// `code_398EF7`: "GAME OVER" for 256 frames, then the menu (stage < 8) or only CONTINUE (`wily`, stage >= 8).
    GameOver { wily: bool },
}

pub struct PasswordScreen {
    pub screen: Screen,
    target: [u8; 32],
    /// `$0290`: Y of the arrow (`$AF` first entry, `$BF` second).
    pub cursor_y: u8,
    /// Video frames since the scene was entered (`$22 = $11`, the first column of the screen is built in frame 0).
    t: u32,
    /// `$0200..$02FF` as `code_399886` leaves it (copied into the picture when the sprites appear).
    final_oam: [u8; 256],
    kind: PasswordKind,
    /// A text buffer written this frame reaches the nametable in the next frame.
    next_buf: Option<Vec<u8>>,
    /// The menu answers buttons (from the frame the cursor is on).
    menu: bool,
}

/// What a frame of the menu decided.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PasswordResult {
    Stay,
    /// Start/A on the first entry (`$0290 != $BF`): refill, then the stage select.
    First,
    /// Start/A on the second entry (`$0290 == $BF`).
    Second,
}

impl PasswordScreen {
    /// `ram_b0` = `$B0..$BF` (weapon energies: bit 7 = owned).
    pub fn new(rom: &Rom, kind: PasswordKind, ram_b0: &[u8; 16]) -> PasswordScreen {
        let text = if matches!(kind, PasswordKind::StageClear) { 1 } else { 4 };
        let scene = Stage::load(rom, 0x11, 5);
        let mut s = Screen::new();
        let (tiles, attrs) = scene.nametable(1);
        s.nt[..960].copy_from_slice(&tiles);
        s.nt[960..].copy_from_slice(&attrs);
        // pattern RAM: what was loaded before (set $55 sprites, the weapon-get scene's set $16) and then set $17
        gfx::load_set(rom, 0x55, &mut s.chr);
        gfx::load_set(rom, 0x16, &mut s.chr);
        gfx::load_set(rom, 0x17, &mut s.chr);
        // code_3992C9: one buffer of text records
        s.apply_buffer(&Self::text_buffer(rom, text));
        let mut target = [0u8; 32];
        for (i, t) in target.iter_mut().enumerate() {
            *t = b39(rom, 0x9509 + i as u16);
        }
        s.pal = [0x0F; 32];
        let mut me = PasswordScreen { screen: s, target, cursor_y: 0xAF, t: 0, final_oam: [0xF8; 256], kind, next_buf: None, menu: false };
        me.dots(rom, ram_b0);
        me
    }

    /// `code_3992C9`: the records of text `x`.
    fn text_buffer(rom: &Rom, x: u8) -> Vec<u8> {
        let mut p = 0x952Eu16 + b39(rom, 0x9529 + x as u16) as u16;
        let mut buf = Vec::new();
        loop {
            let v = b39(rom, p);
            buf.push(v);
            if v == 0xFF {
                break;
            }
            p += 1;
        }
        buf
    }

    /// `code_399886`: the grid. 36 sprites at `$0200`; the owned items switch some of them to the dot tile.
    fn dots(&mut self, rom: &Rom, b0: &[u8; 16]) {
        let o = &mut self.final_oam;
        for y in 1..=0x90usize {
            o[y - 1] = b39(rom, 0x98F9 + y as u16);
        }
        // the number of weapon slots $B4..$BD that are not zero picks one dot
        let count = (0..10).filter(|&y| b0[4 + y] != 0).count();
        let x = b39(rom, 0x99AF + count as u16) as usize;
        o[0x01 + x] = 1;
        let (mut s00, mut s01) = (0usize, 0usize);
        loop {
            let mut s02 = 0usize;
            let first = b0[b39(rom, 0x9A1E + s00 as u16) as usize & 15];
            let second = b0[b39(rom, 0x9A1F + s00 as u16) as usize & 15];
            if first & 0x80 != 0 {
                // BPL not taken: first one owned
                if second & 0x80 == 0 {
                    // second not owned: stays 0
                } else {
                    s02 = 2;
                }
            } else {
                s02 = 1;
                if second & 0x80 == 0 {
                    s02 = 3;
                }
            }
            let idx = s01 | s02;
            let x = b39(rom, 0x99BA + idx as u16) as usize;
            o[0x01 + x] = 1;
            s01 += 4;
            s00 += 2;
            if s00 == 10 {
                break;
            }
        }
    }

    /// The 36 grid tiles (0 = empty, 1 = dot) as `code_399886` set them, for the decoder test.
    pub fn grid_tiles(&self) -> [u8; 36] {
        let mut t = [0u8; 36];
        for (k, v) in t.iter_mut().enumerate() {
            *v = self.final_oam[k * 4 + 1];
        }
        t
    }

    /// The frame at which the sprites (grid and cursor) appear, counted from entering the scene.
    pub const SPRITES_AT: u32 = 49;

    /// One video frame; `pressed` = `$14`. Not to be called in the frame where Start/A is acted on (use the result of the previous call).
    pub fn frame(&mut self, rom: &Rom, pressed: u8) -> PasswordResult {
        let t = self.t;
        self.t += 1;
        self.screen.snd.clear();
        let req = |music: bool, id: u8, ret: u16| crate::sound::Req { music, id, ret };
        if t == Self::SPRITES_AT {
            // code_398E84 (after a clear) / code_398F02 (game over): the screen's music (the frame the grid sprites appear)

            self.screen.snd.push(match self.kind {
                PasswordKind::StageClear => req(true, 0x0D, 0x8E87),
                PasswordKind::GameOver { .. } => req(true, 0x0E, 0x8F05),
            });
        }
        // fade-in: four steps of four frames, the first visible in frame 34
        if t >= 34 {
            let level = match (t - 34) / 4 {
                0 => 0x30u8,
                1 => 0x20,
                2 => 0x10,
                _ => 0x00,
            };
            let mut p = self.target;
            for v in p.iter_mut() {
                let d = v.wrapping_sub(level);
                *v = if d & 0x80 == 0 { d } else { 0x0F };
            }
            self.screen.pal = p;
        }
        if t < Self::SPRITES_AT {
            return PasswordResult::Stay;
        }
        if let Some(buf) = self.next_buf.take() {
            self.screen.apply_buffer(&buf);
        }
        if t < Self::SPRITES_AT {
            return PasswordResult::Stay;
        }
        if t == Self::SPRITES_AT {
            self.screen.oam.copy_from_slice(&self.final_oam);
            if self.kind == PasswordKind::StageClear {
                self.screen.oam[0x90..0x94].copy_from_slice(&[0xAF, 0x02, 0x00, 0x48]);
                self.menu = true;
            }
        }
        if let PasswordKind::GameOver { wily } = self.kind {
            // 256 frames of the dead end, then the same menu as after a clear (stage < 8) or CONTINUE alone (stage >= 8)
            if t == Self::SPRITES_AT + 256 {
                self.screen.snd.push(req(true, 0x0D, 0x8F12)); // code_398F0F
                self.next_buf = Some(Self::text_buffer(rom, if wily { 2 } else { 1 }));
                self.cursor_y = if wily { 0xB7 } else { 0xAF };
                self.screen.oam[0x90..0x94].copy_from_slice(&[self.cursor_y, 0x02, 0x00, 0x48]);
                self.menu = true;
            }
        }
        if !self.menu {
            return PasswordResult::Stay;
        }
        if pressed & (BTN_START | BTN_A) != 0 {
            // code_398ECC: the first entry plays the pick sound twice (once in the menu, once in the refill routine code_398EDE), the second and the wily wait once
            if self.cursor_y != 0xBF && !matches!(self.kind, PasswordKind::GameOver { wily: true }) {
                self.screen.snd.push(req(false, 0x2A, 0x8ED8));
            }
            self.screen.snd.push(req(false, 0x2A, 0x8EE3));
            return if self.cursor_y == 0xBF { PasswordResult::Second } else { PasswordResult::First };
        }
        let wily_wait = matches!(self.kind, PasswordKind::GameOver { wily: true });
        if !wily_wait && pressed & (BTN_DOWN | BTN_UP | BTN_SELECT) != 0 {
            self.screen.snd.push(req(false, 0x2E, 0x8EB4)); // code_398EB1

            self.cursor_y = self.cursor_y.wrapping_add(0x10);
            if self.cursor_y == 0xCF {
                self.cursor_y = 0xAF;
            }
            self.screen.oam[0x90] = self.cursor_y;
        }
        PasswordResult::Stay
    }
}

/// Compare with a `ppu_frames.lua` recording of the stage clear (`PF_HIJACK`): the scene `$11` is entered at the first frame with `$22 = $11` after the weapon-get
/// scene, the password screen is stepped once per recorded frame with the recorded newly-pressed byte `$14`, until Start/A acts.
pub fn verify(rom: &Rom, path: &str, kind: PasswordKind) -> bool {
    use crate::pfrec;
    let frames = match pfrec::load(path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("{}", e);
            return false;
        }
    };
    // the last entry into scene $11 (the first one is the weapon-get scene's own build)
    let mut start = None;
    for w in frames.windows(2) {
        if w[0].r("scene") != 0x11 && w[1].r("scene") == 0x11 {
            start = Some(w[1].n);
        }
    }
    let start = match start {
        Some(s) => s,
        None => {
            eprintln!("{}: no scene $11 found", path);
            return false;
        }
    };
    let first = frames.iter().find(|f| f.n == start).unwrap();
    let mut b0 = [0u8; 16];
    for i in 0..16 {
        b0[i] = first.ram[0x10 + i];
    }
    let mut ps = PasswordScreen::new(rom, kind, &b0);
    let (mut shown, mut bad, mut checked) = (0, 0, 0);
    let mut pending: Option<PasswordResult> = None;
    for f in frames.iter().filter(|f| f.n >= start) {
        let t = f.n - start;
        let pressed = f.r("x14") as u8;
        let r = ps.frame(rom, pressed);
        let _ = pending.take();
        let vis = pfrec::visible(&ps.screen);
        let d = pfrec::compare(f, &ps.screen, vis, vis);
        checked += 1;
        if !pfrec::report(path, f, t, &d, &mut shown) {
            bad += 1;
        }
        if r != PasswordResult::Stay {
            println!("frame {}: {:?} (stop)", f.n, r);
            break;
        }
    }
    println!("{}: {} frames compared, {} differ", path, checked, bad);
    bad == 0
}
