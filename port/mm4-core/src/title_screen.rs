//! The title screen (bank `$39` `code_398037` .. `code_398070` with `code_398495` for `$10 = 1`): scene `$11` screen 2 ("MEGA MAN IV", the helmet), graphics set `$11`,
//! palette row 1 of `$872E`, the arrow cursor sprite (`$88A1` list, Y `$97` = PRESS START, `$A7` = PASS WORD). No fade-in (the palette is copied at once). Select/Up/Down move
//! the arrow; Start picks the entry; 1200 frames without Start start the intro over. Checked frame by frame against Mesen (`mm4 --verify-title`, `harness/pft_*.txt`).
use crate::gfx;
use crate::rom::Rom;
use crate::screen::Screen;
use crate::select::{BTN_A, BTN_DOWN, BTN_SELECT, BTN_START, BTN_UP};
use crate::stage::Stage;

fn b39(rom: &Rom, a: u16) -> u8 {
    rom.cpu_byte(0x39, 0x3A, a)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TitleResult {
    Stay,
    /// Start on PRESS START: the stage select.
    PressStart,
    /// Start on PASS WORD: the password entry.
    Password,
    /// 1200 frames of waiting: the intro runs again.
    Timeout,
}

pub struct TitleScreen {
    pub screen: Screen,
    target: [u8; 32],
    t: u32,
    /// `$0200`: Y of the arrow.
    pub cursor_y: u8,
    template: Vec<u8>,
    /// The loop's countdown `$11:$10` (`$04B0` = 1200 frames).
    timer: u16,
    /// The scene's music (table `$8879`, scene index 1).
    music: u8,
}

impl TitleScreen {
    /// Frame at which the arrow appears and at which the palette shows (counted from the first frame of the scene).
    pub const SPRITES_AT: u32 = 34;
    pub const PALETTE_AT: u32 = 36;

    pub fn new(rom: &Rom) -> TitleScreen {
        let scene = Stage::load(rom, 0x11, 5);
        let mut s = Screen::new();
        let (tiles, attrs) = scene.nametable(2);
        s.nt[..960].copy_from_slice(&tiles);
        s.nt[960..].copy_from_slice(&attrs);
        gfx::load_set(rom, 0x55, &mut s.chr);
        gfx::load_set(rom, 0x11, &mut s.chr);
        let mut target = [0u8; 32];
        for (i, t) in target.iter_mut().enumerate() {
            *t = b39(rom, 0x872E + 32 + i as u16);
        }
        let mut template = Vec::new();
        let mut p = 0x88A1u16;
        while b39(rom, p) != 0xFF {
            template.push(b39(rom, p));
            p += 1;
        }
        s.pal = [0x0F; 32];
        TitleScreen { screen: s, target, t: 0, cursor_y: 0x97, template, timer: 0x04B0, music: b39(rom, 0x8879 + 1) }
    }

    /// One video frame; `pressed` = `$14`.
    pub fn frame(&mut self, pressed: u8) -> TitleResult {
        let t = self.t;
        self.t += 1;
        self.screen.snd.clear();
        if t == Self::SPRITES_AT {
            self.screen.oam[..self.template.len()].copy_from_slice(&self.template);
            self.cursor_y = self.screen.oam[0];
            self.screen.snd.push(crate::sound::Req { music: true, id: self.music, ret: 0x858D }); // code_398585: the scene's music
        }
        if t == Self::PALETTE_AT {
            self.screen.pal = self.target;
        }
        if t < Self::SPRITES_AT {
            return TitleResult::Stay;
        }
        if pressed & (BTN_START | BTN_A) != 0 {
            self.screen.snd.push(crate::sound::Req { music: false, id: 0x2A, ret: 0x8091 }); // code_39808E
            return if self.cursor_y == 0x97 { TitleResult::PressStart } else { TitleResult::Password };
        }
        if pressed & (BTN_UP | BTN_DOWN | BTN_SELECT) != 0 {
            self.screen.snd.push(crate::sound::Req { music: false, id: 0x2E, ret: 0x805E }); // code_39805B
            self.cursor_y = self.cursor_y.wrapping_add(0x10);
            if self.cursor_y == 0xB7 {
                self.cursor_y = 0x97;
            }
            self.screen.oam[0] = self.cursor_y;
        }
        self.timer = self.timer.saturating_sub(1);
        if self.timer == 0 {
            self.screen.snd.push(crate::sound::Req { music: true, id: 0xF0, ret: 0x8089 }); // code_398086
            return TitleResult::Timeout;
        }

        TitleResult::Stay
    }
}

/// Compare with a `ppu_frames.lua` recording from boot: the scene `$11` starts at the first frame with `$22 = $11`; the title is stepped with the recorded `$14` until it reports a result.
pub fn verify(rom: &Rom, path: &str) -> bool {
    use crate::pfrec;
    let frames = match pfrec::load(path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("{}", e);
            return false;
        }
    };
    let start = match frames.iter().find(|f| f.r("scene") == 0x11) {
        Some(f) => f.n,
        None => {
            eprintln!("{}: no scene $11 found", path);
            return false;
        }
    };
    let mut title = TitleScreen::new(rom);
    let (mut shown, mut bad, mut checked) = (0, 0, 0);
    for f in frames.iter().filter(|f| f.n >= start) {
        let t = f.n - start;
        let r = title.frame(f.r("x14") as u8);
        let vis = pfrec::visible(&title.screen);
        let d = pfrec::compare(f, &title.screen, vis, vis);
        checked += 1;
        if !pfrec::report(path, f, t, &d, &mut shown) {
            bad += 1;
        }
        if r != TitleResult::Stay {
            println!("frame {}: {:?} (stop)", f.n, r);
            break;
        }
    }
    println!("{}: {} frames compared, {} differ", path, checked, bad);
    bad == 0
}
