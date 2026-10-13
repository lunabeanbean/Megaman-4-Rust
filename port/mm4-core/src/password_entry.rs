//! The password entry screen (title menu "PASS WORD", bank `$39` `code_399672` .. `code_39983A`) and the decoder behind its END button (`code_39972D`).
//! The grid is the one the stage-clear screen draws (`password_screen.rs`): 6 x 6 sprites at `$0200` (tile 0 = empty, 1 = red dot), the blinking square cursor is the four
//! sprites at `$0290`, END sits at column 6. A toggles the dot under the cursor; END checks the pattern: exactly six dots, one in the "count" row, one per weapon group, and
//! on success gives the weapon energies (`$B0..$BF`) and the defeated-boss flags (`$A9`) back. Wrong passwords print "PASS WORD ERROR" for 240 frames.
//! Checked against Mesen (`mm4 --verify-passwordentry`, `harness/pfp_*.txt`); `decode` is also checked against `PasswordScreen`'s encoder (round trip in the tests).
use crate::gfx;
use crate::rom::Rom;
use crate::screen::Screen;
use crate::select::{BTN_A, BTN_DOWN, BTN_LEFT, BTN_RIGHT, BTN_UP};
use crate::stage::Stage;

fn b39(rom: &Rom, a: u16) -> u8 {
    rom.cpu_byte(0x39, 0x3A, a)
}

/// `code_39972D`: the password in `tiles` (tile byte of each of the 36 grid sprites) -> (`$B0..$BF`, `$A9`), or `None` for a wrong password.
pub fn decode(rom: &Rom, tiles: &[u8; 36]) -> Option<([u8; 16], u8)> {
    let tile_at = |x: u8| tiles[(x / 4) as usize % 36];
    // exactly six dots
    if tiles.iter().filter(|&&t| t != 0).count() != 6 {
        return None;
    }
    // five positions must be empty
    for y in (0..5u16).rev() {
        if tile_at(b39(rom, 0x99AA + y)) != 0 {
            return None;
        }
    }
    // exactly one dot among the eleven "count" positions: its index is the number of weapons the groups below must add up to
    let mut found = 0u8;
    for y in (0..=10u16).rev() {
        if tile_at(b39(rom, 0x99AF + y)) != 0 {
            if found != 0 {
                return None;
            }
            found = y as u8 | 0x80;
        }
    }
    if found & 0x80 == 0 {
        return None;
    }
    let expected = found & 0x0F;
    let mut energy = [0u8; 16];
    energy[0] = 0x9C;
    let (mut count, mut a9) = (0u8, 0u8);
    let mut y = 0u16;
    while y != 0x14 {
        // code_399801: the four positions of this group
        let (mut n, mut idx) = (0u8, 0u16);
        for k in 0..4u16 {
            if tile_at(b39(rom, 0x99BA + y + k)) != 0 {
                idx = y + k;
                n += 1;
            }
        }
        y += 4;
        if n != 1 {
            return None;
        }
        if idx & 3 == 3 {
            continue; // neither weapon of the pair
        }
        count += if idx & 2 != 0 { 2 } else { 1 };
        a9 |= b39(rom, 0x99CE + idx);
        for k in 0..3u16 {
            let slot = b39(rom, 0x99E2 + idx * 3 + k);
            if slot == 0 {
                break;
            }
            energy[slot as usize & 15] = 0x9C;
        }
    }
    if count != expected {
        return None;
    }
    Some((energy, a9))
}

/// What a frame of the entry screen decided.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EntryResult {
    Stay,
    /// END with a valid password, after the 60 frames the game waits: `$B0..$BF` and `$A9` (`$AA` = 0), then the stage select.
    Accepted([u8; 16], u8),
}

pub struct PasswordEntry {
    pub screen: Screen,
    target: [u8; 32],
    t: u32,
    /// `$10` (row 0..5, 6 = END) and `$11` (column 0..5, 6 = END).
    pub row: u8,
    pub col: u8,
    /// `$95`: blinks the cursor (hidden while bit 2 is set: four frames on, four off).
    p95: u8,
    /// Frames the game spends inside a wait (error message, acceptance): no input, no cursor update.
    block: u32,
    /// The text buffer written this frame reaches the nametable next frame.
    next_buf: Option<Vec<u8>>,
    /// After the error wait the blank text is written and the loop goes on.
    after_error: bool,
    accepted: Option<([u8; 16], u8)>,
    template: Vec<u8>,
}

impl PasswordEntry {
    pub const SPRITES_AT: u32 = 49;

    pub fn new(rom: &Rom) -> PasswordEntry {
        let scene = Stage::load(rom, 0x11, 5);
        let mut s = Screen::new();
        let (tiles, attrs) = scene.nametable(1);
        s.nt[..960].copy_from_slice(&tiles);
        s.nt[960..].copy_from_slice(&attrs);
        gfx::load_set(rom, 0x55, &mut s.chr);
        gfx::load_set(rom, 0x11, &mut s.chr);
        gfx::load_set(rom, 0x17, &mut s.chr);
        s.apply_buffer(&text_buffer(rom, 0));
        let mut target = [0u8; 32];
        for (i, t) in target.iter_mut().enumerate() {
            *t = b39(rom, 0x9509 + i as u16);
        }
        s.pal = [0x0F; 32];
        let template: Vec<u8> = (1..=0xA0u16).map(|y| b39(rom, 0x98F9 + y)).collect();
        PasswordEntry { screen: s, target, t: 0, row: 0, col: 0, p95: 0, block: 0, next_buf: None, after_error: false, accepted: None, template }
    }

    fn tiles(&self) -> [u8; 36] {
        let mut t = [0u8; 36];
        for (k, v) in t.iter_mut().enumerate() {
            *v = self.screen.oam[k * 4 + 1];
        }
        t
    }

    /// `code_39983A`: the cursor sprites (`$0290..$029F`).
    fn cursor(&mut self, rom: &Rom) {
        let o = &mut self.screen.oam;
        if (self.p95 >> 2) & 1 != 0 {
            for s in 0..4 {
                o[0x90 + s * 4] = 0xF8;
            }
            return;
        }
        let y = b39(rom, 0x999A + self.row as u16);
        o[0x90] = y;
        o[0x94] = y;
        o[0x98] = y.wrapping_add(8);
        o[0x9C] = y.wrapping_add(8);
        let x = b39(rom, 0x99A1 + self.col as u16);
        o[0x93] = x;
        o[0x9B] = x;
        o[0x97] = x.wrapping_add(8);
        o[0x9F] = x.wrapping_add(8);
        if self.col == 6 {
            o[0x97] = x.wrapping_add(0x18);
            o[0x9F] = x.wrapping_add(0x18);
        }
    }

    pub fn frame(&mut self, rom: &Rom, pressed: u8) -> EntryResult {
        let t = self.t;
        self.t += 1;
        self.screen.snd.clear();
        if t == Self::SPRITES_AT {
            self.screen.snd.push(crate::sound::Req { music: true, id: 0x0D, ret: 0x967A }); // code_399677 (the frame the grid sprites appear)
        }

        if let Some(b) = self.next_buf.take() {
            self.screen.apply_buffer(&b);
        }
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
            return EntryResult::Stay;
        }
        if t == Self::SPRITES_AT {
            self.screen.oam[..0xA0].copy_from_slice(&self.template);
        }
        if self.block > 0 {
            self.block -= 1;
            if self.block == 0 {
                if let Some(a) = self.accepted.take() {
                    return EntryResult::Accepted(a.0, a.1);
                }
                if self.after_error {
                    self.after_error = false;
                    self.next_buf = Some(text_buffer(rom, 0));
                    self.cursor(rom);
                    self.p95 = self.p95.wrapping_add(1);
                }
            }
            return EntryResult::Stay;
        }
        self.step(rom, pressed)
    }

    /// One pass of the loop at `code_39968D`.
    fn step(&mut self, rom: &Rom, pressed: u8) -> EntryResult {
        let delta = |x: usize| -> u8 { b39(rom, 0x99A8 + x as u16) };
        let mut vertical = true;
        if pressed & (BTN_RIGHT | BTN_LEFT) != 0 {
            self.screen.snd.push(crate::sound::Req { music: false, id: 0x2E, ret: 0x969B }); // code_399698
            let x = (pressed & BTN_RIGHT) as usize;
            let old = self.col;
            self.col = self.col.wrapping_add(delta(x));
            match self.col {
                6 | 0xFF => {
                    self.row = 6;
                    self.col = 6;
                    vertical = false;
                }
                7 => {
                    self.row = 0;
                    self.col = 0;
                    vertical = false;
                }
                5 if old == 6 => {
                    self.row = 0;
                    vertical = false;
                }
                _ => {}
            }
        }
        if vertical && self.row != 6 && pressed & (BTN_UP | BTN_DOWN) != 0 {
            self.screen.snd.push(crate::sound::Req { music: false, id: 0x2E, ret: 0x96E7 }); // code_3996E4
            let x = ((pressed & BTN_DOWN) >> 2) as usize;
            self.row = self.row.wrapping_add(delta(x));
            if self.row == 6 {
                self.row = 0;
            } else if self.row == 0xFF {
                self.row = 5;
            }
        }
        if pressed & BTN_A != 0 {
            if self.col == 6 {
                match decode(rom, &self.tiles()) {
                    Some(a) => {
                        self.screen.snd.push(crate::sound::Req { music: false, id: 0x2F, ret: 0x97E6 }); // code_3997E3
                        self.accepted = Some(a);
                        self.block = 0x3C;
                        return EntryResult::Stay;
                    }
                    None => {
                        // code_39981D: the error text, 240 frames, then the blank text
                        self.screen.snd.push(crate::sound::Req { music: false, id: 0x2D, ret: 0x9822 }); // code_39981F
                        self.row = 0;

                        self.col = 0;
                        self.next_buf = Some(text_buffer(rom, 3));
                        self.block = 0xF0;
                        self.after_error = true;
                        return EntryResult::Stay;
                    }
                }
            }
            let k = (self.row as usize * 6 + self.col as usize) % 36;
            self.screen.oam[k * 4 + 1] ^= 1;
        }
        self.cursor(rom);
        self.p95 = self.p95.wrapping_add(1);
        EntryResult::Stay
    }
}

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

/// Compare with a `ppu_frames.lua` recording of the entry screen (title -> PASS WORD): the scene starts 49 frames before the grid sprites appear; the screen is stepped with the recorded
/// `$14`; the check stops when the entry is accepted.
pub fn verify(rom: &Rom, path: &str) -> bool {
    use crate::pfrec;
    let frames = match pfrec::load(path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("{}", e);
            return false;
        }
    };
    // the first frame with 40 sprites (grid + cursor) is frame 49 of the scene
    let first40 = match frames.iter().find(|f| f.n > 100 && f.oam.chunks(4).filter(|s| s[0] < 0xEF).count() >= 40) {
        Some(f) => f.n,
        None => {
            eprintln!("{}: no entry screen found", path);
            return false;
        }
    };
    let start = first40 - PasswordEntry::SPRITES_AT;
    let mut e = PasswordEntry::new(rom);
    let (mut shown, mut bad, mut checked) = (0, 0, 0);
    for f in frames.iter().filter(|f| f.n >= start) {
        let t = f.n - start;
        let r = e.frame(rom, f.r("x14") as u8);
        let vis = pfrec::visible(&e.screen);
        let d = pfrec::compare(f, &e.screen, vis, vis);
        checked += 1;
        if !pfrec::report(path, f, t, &d, &mut shown) {
            bad += 1;
        }
        if let EntryResult::Accepted(en, a9) = r {
            println!("frame {}: accepted, A9 {:02X}, B0.. {:02X?} (stop)", f.n, a9, en);
            // the game's RAM a few frames later must hold the same values
            if let Some(g) = frames.iter().find(|g| g.n == f.n + 5) {
                let ram_b0: Vec<u8> = g.ram[16..32].to_vec();
                if ram_b0 != en.to_vec() || g.ram[9] != a9 || g.ram[10] != 0 {
                    println!("  RAM differs: game A9 {:02X} AA {:02X} B0.. {:02X?}", g.ram[9], g.ram[10], ram_b0);
                    bad += 1;
                }
            }
            break;
        }
    }
    println!("{}: {} frames compared, {} differ", path, checked, bad);
    bad == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encoder_and_decoder_agree() {
        let rom = match Rom::load("../../Mega Man 4 (USA).nes") {
            Ok(r) => r,
            Err(_) => return,
        };
        // every combination of the ten Robot Master weapons (slots 4..13) owned or not
        for mask in 0u32..1024 {
            let mut b0 = [0u8; 16];
            b0[0] = 0x9C;
            for k in 0..10 {
                if mask & (1 << k) != 0 {
                    b0[4 + k] = 0x9C;
                }
            }
            let ps = crate::password_screen::PasswordScreen::new(&rom, crate::password_screen::PasswordKind::StageClear, &b0);
            let tiles = ps.grid_tiles();
            let (e, _a9) = decode(&rom, &tiles).unwrap_or_else(|| panic!("mask {:03X}: the encoder's password is rejected", mask));
            for k in 4..14 {
                assert_eq!(e[k] & 0x80, b0[k] & 0x80, "mask {:03X} slot {}", mask, k);
            }
        }
    }
}
