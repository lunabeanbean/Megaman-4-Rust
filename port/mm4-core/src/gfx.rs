//! Emulation of the game's graphics loader (code_3EDA05, see notes/graphics.md).
//! Mega Man 4 has no CHR ROM: tiles are copied from PRG banks into 8 KB of CHR-RAM.
use crate::rom::Rom;

pub struct GfxSet {
    pub palette: Vec<u8>,
}

/// Apply graphics set `id` to `chr` (8 KB) and return the palette bytes the script carries.
pub fn load_set(rom: &Rom, id: usize, chr: &mut [u8]) -> GfxSet {
    // The script lives in bank $28 at $B800/$B870; only the $A000 window matters for it.
    let script = |addr: usize| rom.cpu_byte(0x28, 0x28, addr as u16);
    let mut p = ((script(0xB870 + id) as usize) << 8) | script(0xB800 + id) as usize;
    let mut palette = Vec::new();
    for _ in 0..8 {
        let v = script(p);
        p += 1;
        if v >= 0x80 {
            break;
        }
        palette.push(v);
    }
    loop {
        let bank = script(p) as usize;
        if bank >= 0x80 {
            break;
        }
        let pages = script(p + 1) as usize;
        let src_high = script(p + 2) as usize;
        let ppu_high = script(p + 3) as usize;
        p += 4;
        for i in 0..pages * 256 {
            chr[((ppu_high << 8) + i) & 0x1FFF] = rom.cpu_byte(bank, bank + 1, ((src_high << 8) + i) as u16);
        }
    }
    GfxSet { palette }
}

/// Video frames the graphics loader (`code_3EDA05`) takes. It copies straight into the PPU with rendering off, 14 cycles a byte (+15 per page), while every
/// NMI steals part of the frame: about 27000 cycles of loader work fit in a frame. `start` = cycles of the frame already used when the loader begins (about 3000
/// right after a yield, more after other work such as clearing the sprites). Seen in the recordings: set `$10` = 5376 bytes 2 frames (3 after a sprite clear),
/// `$4C` = 5888 bytes 3, `$17` = 4352 bytes 2, `$55` = 6912 bytes 3, `$57` = 6144 bytes 3, `$16` = 8192 bytes 4.
pub fn load_frames_from(rom: &Rom, id: usize, start: usize) -> usize {
    let script = |addr: usize| rom.cpu_byte(0x28, 0x28, addr as u16);
    let mut p = ((script(0xB870 + id) as usize) << 8) | script(0xB800 + id) as usize;
    for _ in 0..8 {
        let v = script(p);
        p += 1;
        if v >= 0x80 {
            break;
        }
    }
    let mut cycles = start;
    loop {
        let bank = script(p) as usize;
        if bank >= 0x80 {
            break;
        }
        cycles += script(p + 1) as usize * (256 * 14 + 15);
        p += 4;
    }
    cycles / 27000
}

/// `load_frames_from` for a loader that starts right after a yield.
pub fn load_frames(rom: &Rom, id: usize) -> usize {
    load_frames_from(rom, id, 3000)
}

/// The section's animated-graphics script (`code_3EDA8B` starts it, `code_3EDABB` runs it): records `[bank, tiles, src high, dst high]` in bank `$29` at `$B800/$B860 + id`
/// (palette bytes first, as in `load_set`), copied to CHR-RAM three tiles per call, one call per video frame.
#[derive(Clone, Debug, Default)]
pub struct GfxStream {
    script: usize,
    idx: usize,
    left: usize,
    bank: usize,
    src: usize,
    dst: usize,
}

impl GfxStream {
    /// `code_3EDA8B`: returns the stream and the palette bytes of the script (`$0618+`, `$0638+`).
    pub fn start(rom: &Rom, id: usize) -> (GfxStream, Vec<u8>) {
        let sb = |addr: usize| rom.cpu_byte(0x29, 0x29, addr as u16);
        let script = ((sb(0xB860 + id) as usize) << 8) | sb(0xB800 + id) as usize;
        let mut pal = Vec::new();
        let mut idx = 0;
        while idx < 8 {
            let v = sb(script + idx);
            idx += 1;
            if v >= 0x80 {
                break;
            }
            pal.push(v);
        }
        (GfxStream { script, idx, ..Default::default() }, pal)
    }

    /// `code_3EDABB`: copy up to three tiles of the current record (the next record starts when one is used up).
    pub fn step(&mut self, rom: &Rom, chr: &mut [u8]) -> bool {
        if self.script == 0 {
            return false;
        }
        let sb = |addr: usize| rom.cpu_byte(0x29, 0x29, addr as u16);
        if self.left == 0 {
            let bank = sb(self.script + self.idx);
            if bank >= 0x80 {
                return false;
            }
            self.bank = bank as usize;
            self.left = match sb(self.script + self.idx + 1) { 0 => 256, n => n as usize };
            self.src = (sb(self.script + self.idx + 2) as usize) << 8;
            self.dst = (sb(self.script + self.idx + 3) as usize) << 8;
            self.idx += 4;
        }
        let mut copied = 0;
        loop {
            for _ in 0..16 {
                chr[(self.dst + copied) & 0x1FFF] = rom.cpu_byte(self.bank, self.bank + 1, (self.src + copied) as u16);
                copied += 1;
            }
            self.left -= 1;
            if self.left == 0 || copied == 0x30 {
                break;
            }
        }
        self.src += copied;
        self.dst += copied;
        true
    }

    /// Run the whole script at once (the stage load and the restart: the loader finishes before the first frame is shown).
    pub fn finish(&mut self, rom: &Rom, chr: &mut [u8]) {
        while self.step(rom, chr) {}
    }
}
