//! A stage: tile/block/screen tables from the stage's PRG bank (notes/level-format.md) plus real graphics.
use crate::anim::AnimTable;
use crate::gfx;
use crate::hitbox::Hitboxes;
use crate::nes::{decode_tile, NES_RGB};
use crate::rom::Rom;

pub const SCREEN_W: usize = 256;
pub const SCREEN_H: usize = 240;

#[derive(Clone)]
pub struct Stage {
    pub id: usize,
    bank: Vec<u8>,
    /// Fixed banks $3E/$3F ($C000-$FFFF): the terrain lookup can read there when the page number is out of range.
    fixed: Vec<u8>,
    pub chr: Vec<u8>,
    pub bg_palette: [u8; 16],
    /// The four sprite palettes: Mega Man's two from `$C853`, the stage's two from the graphics script.
    pub sprite_palette: [[u8; 4]; 4],
    pub screens: usize,
    pub anims: AnimTable,
    pub hitboxes: Hitboxes,
    /// $0670 + page: shutter state of stage 2 (high nibble 7 = the type-7 tiles of that page are solid). Written by the gate objects.
    pub door: [std::cell::Cell<u8>; 256],
    /// $0680..$06BF: bit set = the destructible block ($90 tile, only while a sway object is active) is broken. Written by the world.
    pub blocks: [std::cell::Cell<u8>; 64],
    /// `$F0 == 4` (mirrored by the world): the $90 tiles obey `blocks`.
    pub sway: std::cell::Cell<bool>,
    /// `$AC` = `$FF` (every robot master down, stage 14): four extra solid tiles around the ninth pod (code_3ED661).
    pub ac_full: std::cell::Cell<bool>,
    /// `$F0 == 5` (a water-level scroll object, AI `$6B`/`$6C`): `Some($013E)`; every tile probe whose Y is below that value sees empty space (code_3ED61A).
    pub lvl5: std::cell::Cell<Option<u8>>,
    /// `$2B & $C0 != 0`: the camera is in a vertical section (one bitmap of `$0680` for the whole section instead of one per page parity).
    pub vsect: std::cell::Cell<bool>,
}

impl Stage {
    /// `screens` = 0 means "guess from the screen-order table" (consecutive layout ids).
    pub fn load(rom: &Rom, id: usize, screens: usize) -> Stage {
        let bank = rom.bank(0x20 | id).to_vec();
        let fixed = [rom.bank(0x3E), rom.bank(0x3F)].concat();
        let mut chr = vec![0u8; 0x2000];
        let gfx_set = gfx::load_set(rom, id, &mut chr);
        let mut bg_palette = [0u8; 16];
        bg_palette.copy_from_slice(&bank[0xB590 - 0xA000..0xB590 - 0xA000 + 16]);
        let screens = if screens > 0 {
            screens
        } else if id == 0 {
            31 // layouts 0-15, 20-30, 16-19 (pages 0-30; page 31 repeats layout 0)
        } else if id == 6 {
            31 // layouts 0-7, 30, 8-29 (pages 0-30; page 31 repeats layout 0x18)
        } else if id == 1 {
            29 // layouts 0-15, 24, 25, 16-20, 27, 28, 21-23, 26: not a consecutive run (pages 0-28), junk after it
        } else {
            let mut n = 1;
            while n < 48 && bank[0xB500 - 0xA000 + n] == bank[0xB500 - 0xA000 + n - 1].wrapping_add(1) {
                n += 1;
            }
            if n < 4 {
                // layouts that are not numbered consecutively (stage 3): take the run of plausible layout ids
                n = (0..48).take_while(|&i| bank[0xB500 - 0xA000 + i] < 0x40).count().max(1);
            }
            n
        };
        let mut sprite_palette = [[0u8; 4]; 4];
        let mm = |i: usize| rom.cpu_byte(0x3E, 0x3F, 0xC853 + i as u16);
        let gp = |i: usize| gfx_set.palette.get(i).copied().unwrap_or(0x0F);
        for k in 0..4 {
            sprite_palette[0][k] = if k == 0 { bg_palette[0] } else { mm(k) };
            sprite_palette[1][k] = if k == 0 { bg_palette[0] } else { mm(4 + k) };
            sprite_palette[2][k] = if k == 0 { bg_palette[0] } else { gp(k) };
            sprite_palette[3][k] = if k == 0 { bg_palette[0] } else { gp(4 + k) };
        }
        Stage { id, bank, fixed, chr, bg_palette, sprite_palette, screens, anims: AnimTable::load(rom, 0), hitboxes: Hitboxes::load(rom), door: std::array::from_fn(|_| std::cell::Cell::new(0)), blocks: std::array::from_fn(|_| std::cell::Cell::new(0)), sway: std::cell::Cell::new(false), ac_full: std::cell::Cell::new(false), lvl5: std::cell::Cell::new(None), vsect: std::cell::Cell::new(false) }
    }

    /// Read a CPU address as the game sees it: $A000-$BFFF is the stage bank, $C000-$FFFF the fixed banks.
    fn at(&self, cpu: usize) -> u8 {
        if cpu >= 0xC000 {
            self.fixed[cpu - 0xC000]
        } else {
            self.bank[cpu - 0xA000]
        }
    }

    /// Draw OAM sprites (8x8, pattern table `$1000`) into a 256x240 frame the way the PPU does: the sprite list is in OAM order (earlier entries are on top) and
    /// only the first eight sprites of a scanline are drawn.
    pub fn draw_sprites(&self, frame: &mut [u32], sprites: &[crate::oam::Sprite], palette: &[u8; 32]) {
        let backdrop = NES_RGB[(palette[0] & 0x3F) as usize];
        let mut tiles: Vec<Option<[u8; 64]>> = vec![None; 256];
        for y in 0..SCREEN_H {
            let mut on_line: Vec<&crate::oam::Sprite> = Vec::new();
            for s in sprites {
                if s.y < 0xEF && (s.y as usize + 1..s.y as usize + 9).contains(&y) {
                    on_line.push(s);
                    if on_line.len() == 8 {
                        break;
                    }
                }
            }
            for s in on_line.iter().rev() {
                let tile = tiles[s.tile as usize].get_or_insert_with(|| decode_tile(&self.chr, 0x1000 + s.tile as usize * 16));
                let pal = &palette[16 + (s.attr & 3) as usize * 4..];
                let py = y - (s.y as usize + 1);
                let sy = if s.attr & 0x80 != 0 { 7 - py } else { py };
                for px in 0..8usize {
                    let sx = if s.attr & 0x40 != 0 { 7 - px } else { px };
                    let c = tile[sy * 8 + sx];
                    if c == 0 {
                        continue;
                    }
                    let x = s.x as usize + px;
                    if x >= SCREEN_W {
                        continue;
                    }
                    let idx = y * SCREEN_W + x;
                    if s.attr & 0x20 != 0 && frame[idx] != backdrop {
                        continue;
                    }
                    frame[idx] = NES_RGB[(pal[c as usize] & 0x3F) as usize];
                }
            }
        }
    }

    /// Read a CPU address in the stage bank ($A000-$BFFF) or the fixed banks ($C000-$FFFF).
    pub fn byte(&self, cpu: usize) -> u8 {
        self.at(cpu)
    }

    /// The backdrop colour (palette entry 0) as RGB.
    pub fn backdrop_rgb(&self) -> u32 {
        NES_RGB[(self.bg_palette[0] & 0x3F) as usize]
    }

    /// A copy whose background palette is lowered by `level` like the game's palette fades do (`code_3EC451`: negative values become black).
    pub fn faded_copy(&self, level: u8) -> Stage {
        let mut s = self.clone();
        let lower = |v: &mut u8| {
            let d = v.wrapping_sub(level);
            *v = if d & 0x80 == 0 { d } else { 0x0F };
        };
        s.bg_palette.iter_mut().for_each(lower);
        for p in s.sprite_palette.iter_mut() {
            p.iter_mut().for_each(lower);
        }
        s
    }

    /// Render the whole stage into one RGB image (0x00RRGGBB), `screens*256` x 240.
    pub fn render_full(&self) -> (usize, usize, Vec<u32>) {
        let w = self.screens * SCREEN_W;
        let mut img = vec![0u32; w * SCREEN_H];
        let tiles: Vec<[u8; 64]> = (0..512).map(|n| decode_tile(&self.chr, n * 16)).collect();
        for s in 0..self.screens {
            let layout = self.at(0xB500 + s) as usize;
            for cell in 0..64 {
                let block = self.at(0xA900 + layout * 64 + cell) as usize;
                for k in 0..4 {
                    let m = self.at(0xA500 + block * 4 + k) as usize;
                    let pal = (self.at(0xA400 + m) & 3) as usize;
                    let mx = (cell % 8) * 2 + (k & 1);
                    let my = (cell / 8) * 2 + (k >> 1);
                    for t in 0..4 {
                        let tile = &tiles[self.at(0xA000 + 0x100 * t + m) as usize];
                        let x0 = s * SCREEN_W + mx * 16 + (t & 1) * 8;
                        let y0 = my * 16 + (t >> 1) * 8;
                        for y in 0..8 {
                            if y0 + y >= SCREEN_H {
                                continue;
                            }
                            for x in 0..8 {
                                let c = self.bg_palette[pal * 4 + tile[y * 8 + x] as usize] & 0x3F;
                                img[(y0 + y) * w + x0 + x] = NES_RGB[c as usize];
                            }
                        }
                    }
                }
            }
        }
        (w, SCREEN_H, img)
    }
}

impl Stage {
    /// Terrain type (0-15, the high nibble of the metatile flags) at world pixel `x` and screen-relative pixel `y` (0..239).
    /// A Y outside the screen counts as empty. The page number is the high byte of X exactly as the game uses it (so X < 0 reads
    /// page $FF, whatever the tables hold there: Mega Man really does bump into it at the left edge of stage 2).
    pub fn terrain(&self, x: i32, y: i32) -> u8 {
        self.terrain_lim(x, y, SCREEN_H as i32)
    }

    /// `terrain` with a different Y limit: a probe above the entity (negative offset) whose 8-bit sum wrapped is looked up even in rows `$F0..$FF`
    /// (code_3ED323: the carry check replaces the `$F0` limit), so those callers pass 256.
    pub fn terrain_lim(&self, x: i32, y: i32, lim: i32) -> u8 {
        if !(0..lim).contains(&y) {
            return 0;
        }
        if let Some(w) = self.lvl5.get() {
            if (y as u8) < w {
                return 0;
            }
        }
        let page = ((x >> 8) & 0xFF) as usize;
        let (xs, ys) = ((x & 0xFF) as usize, y as usize);
        let layout = self.at(0xB500 + page) as usize;
        let block = self.at(0xA900 + layout * 64 + (ys >> 5) * 8 + (xs >> 5)) as usize;
        let k = ((ys >> 4) & 1) * 2 + ((xs >> 4) & 1);
        let meta = self.at(0xA500 + block * 4 + k) as usize;
        let t = self.at(0xA400 + meta) >> 4;
        // Stage 4 (code_3ED599): the pink bar tiles (metatiles $09 and $8C) are solid until their bit in the `$0680` bitmap is set by a bar object.
        if self.id == 4 && (meta == 9 || meta == 0x8C) {
            let idx = (ys & 0xF0) | (xs >> 4);
            let half = if self.vsect.get() { 0 } else { page & 1 };
            let byte = half * 32 + (idx >> 3);
            return if self.blocks[byte & 63].get() & (1 << (idx & 7)) != 0 { 0 } else { 1 };
        }
        // Stage 2 (code_3ED5E6): type 7 tiles are door shutters, solid only while a gate object has set $0670[page] (high nibble).
        // Stage 8 (code_3ED5DA): type-1 (solid) tiles are masked by the same bitmap
        if t == 1 && self.id == 8 {
            let idx = (ys & 0xF0) | (xs >> 4);
            let half = if self.vsect.get() { 0 } else { page & 1 };
            let byte = half * 32 + (idx >> 3);
            return if self.blocks[byte & 63].get() & (1 << (idx & 7)) != 0 { 0 } else { 1 };
        }
        if t == 0 && self.id == 14 && page == 9 && self.ac_full.get() {
            // code_3ED661: (block cell, quarter) pairs at `$D696`
            let cell = (ys >> 5) * 8 + (xs >> 5);
            if matches!((cell, k), (0x0B, 1) | (0x0C, 0) | (0x13, 3) | (0x14, 2)) {
                return 1;
            }
        }
        if t == 7 && self.id == 2 {
            return self.door[page].get() >> 4;
        }
        if t == 9 && self.sway.get() {
            // code_3ED624: a $90 tile listed in the table at $E566/$E577 is empty once its bit in $0680.. is set
            let idx = (ys >> 5) * 8 + (xs >> 5);
            let mut y = self.at(0xE566 + page) as usize;
            loop {
                let v = self.at(0xE577 + y);
                if v & 0x80 != 0 {
                    break;
                }
                if v as usize == idx {
                    let bit = (self.at(0xE578 + y) | k as u8) as usize;
                    if self.blocks[(bit >> 3) & 63].get() & (1 << (bit & 7)) != 0 {
                        return 0;
                    }
                    break;
                }
                y += 2;
            }
        }
        t
    }

    /// The scroll mode of the objects: `$F0` = 4 (sway) or 5 (water level `$013E`).
    pub fn set_scroll_mode(&self, f0: u8, w13e: u8) {
        self.sway.set(f0 == 4);
        self.lvl5.set(if f0 == 5 { Some(w13e) } else { None });
    }

    /// code_3ECEE1 clears $0680..$06BF.
    pub fn reset_blocks(&self) {
        for b in &self.blocks {
            b.set(0);
        }
    }
}

impl Stage {
    /// Section table byte `$B530 + i` (low nibble = last screen index, high bits = flags).
    pub fn sect_byte(&self, i: usize) -> u8 {
        self.at(0xB530 + i)
    }

    /// The section containing world page `page` and the page it starts at. Sections are laid out one after the other (`$B530` low nibble = last screen index),
    /// which matches the checkpoint table of stage 2 (page `$0B` = section 6, page `$15` = section 8).
    pub fn section_of_page(&self, page: i32) -> (u8, i32) {
        if self.id == 0 && page >= 27 {
            return (10, 27); // stage 0: table entry 9 is unused, section 10 is the cavern of pages 27-30 (reached through the door table)
        }
        if self.id == 6 && page >= 25 {
            return if page <= 26 { (13, 25) } else { (14, 27) }; // the secret shaft behind the pit of page 18 (section 12 is unused)
        }
        let mut start = 0;
        for i in 0..16 {
            let n = (self.sect_byte(i) & 0x0F) as i32 + 1;
            if page < start + n {
                return (i as u8, start);
            }
            start += n;
        }
        (0, 0)
    }

    /// Transition table byte `$B540 + i` (4 bytes per entry: page, direction flags, target section, target page; `$80+` ends the table).
    pub fn trans_byte(&self, i: usize) -> u8 {
        self.at(0xB540 + i)
    }

    /// Rightmost camera X inside the first section of the stage. The section table at $B530 holds the section's last
    /// screen in its low nibble (`$2D`); the camera stops there (notes/camera.md). Later sections / doors are not ported yet.
    pub fn first_section_max_cam(&self) -> i32 {
        ((self.at(0xB530) & 0x0F) as i32) * 256
    }
}

impl Stage {
    /// One 4x4-tile block (cell `cell` = row << 3 | column of layout screen `screen`) the way `code_3ED17F` makes it: 16 tile ids (row-major) and the attribute byte
    /// (2 bits per 16x16 quarter, quarter 0 in bits 0-1).
    pub fn block(&self, screen: usize, cell: usize) -> ([u8; 16], u8) {
        let layout = self.at(0xB500 + screen) as usize;
        let block = self.at(0xA900 + layout * 64 + cell) as usize;
        let mut t = [0u8; 16];
        let mut attr = 0u8;
        for k in 0..4usize {
            let m = self.at(0xA500 + block * 4 + k) as usize;
            attr |= (self.at(0xA400 + m) & 3) << (2 * k);
            let (bx, by) = ((k & 1) * 2, (k >> 1) * 2);
            for q in 0..4usize {
                t[(by + (q >> 1)) * 4 + bx + (q & 1)] = self.at(0xA000 + 0x100 * q + m);
            }
        }
        (t, attr)
    }

    /// The nametable (32x30 tile ids, then the 64 attribute bytes) the game's column builder makes from layout screen `screen` of this stage/scene.
    /// Menu scenes (`$22` = `$10..`) are stored like stages: each 16x16 metatile is 2x2 tiles (tables `$A000-$A300`) and carries its palette in the low two flag bits.
    pub fn nametable(&self, screen: usize) -> (Vec<u8>, Vec<u8>) {
        let mut tiles = vec![0u8; 32 * 30];
        let mut attr = vec![0u8; 64];
        let layout = self.at(0xB500 + screen) as usize;
        for cell in 0..64usize {
            let block = self.at(0xA900 + layout * 64 + cell) as usize;
            for k in 0..4usize {
                let m = self.at(0xA500 + block * 4 + k) as usize;
                let pal = self.at(0xA400 + m) & 3;
                let mx = (cell % 8) * 2 + (k & 1);
                let my = (cell / 8) * 2 + (k >> 1);
                for t in 0..4usize {
                    let (tx, ty) = (mx * 2 + (t & 1), my * 2 + (t >> 1));
                    if ty < 30 {
                        tiles[ty * 32 + tx] = self.at(0xA000 + 0x100 * t + m);
                    }
                }
                // attribute byte of the 32x32 area, 2 bits per 16x16 quadrant
                let (ax, ay) = (mx / 2, my / 2);
                let shift = ((my & 1) * 2 + (mx & 1)) * 2;
                if ay < 8 {
                    attr[ay * 8 + ax] = (attr[ay * 8 + ax] & !(3 << shift)) | (pal << shift);
                }
            }
        }
        (tiles, attr)
    }
}

impl Stage {
    /// The whole stage as palette indices (`palette * 4 + colour`, 0..15) instead of RGB: the picture is then coloured with the live palette of each frame.
    pub fn render_idx(&self) -> (usize, usize, Vec<u8>) {
        let w = self.screens * SCREEN_W;
        let mut img = vec![0u8; w * SCREEN_H];
        let tiles: Vec<[u8; 64]> = (0..512).map(|n| decode_tile(&self.chr, n * 16)).collect();
        for s in 0..self.screens {
            let layout = self.at(0xB500 + s) as usize;
            for cell in 0..64 {
                let block = self.at(0xA900 + layout * 64 + cell) as usize;
                for k in 0..4 {
                    let m = self.at(0xA500 + block * 4 + k) as usize;
                    let pal = (self.at(0xA400 + m) & 3) as usize;
                    let mx = (cell % 8) * 2 + (k & 1);
                    let my = (cell / 8) * 2 + (k >> 1);
                    for t in 0..4 {
                        let tile = &tiles[self.at(0xA000 + 0x100 * t + m) as usize];
                        let x0 = s * SCREEN_W + mx * 16 + (t & 1) * 8;
                        let y0 = my * 16 + (t >> 1) * 8;
                        for y in 0..8 {
                            if y0 + y >= SCREEN_H {
                                continue;
                            }
                            for x in 0..8 {
                                img[(y0 + y) * w + x0 + x] = (pal * 4) as u8 + tile[y * 8 + x];
                            }
                        }
                    }
                }
            }
        }
        (w, SCREEN_H, img)
    }
}

/// `p` lowered by `level` like the game's palette fades (`code_3EC451`): a negative result becomes black.
pub fn lower_palette(p: &[u8; 32], level: u8) -> [u8; 32] {
    let mut q = *p;
    for v in q.iter_mut() {
        let d = v.wrapping_sub(level);
        *v = if d & 0x80 == 0 { d } else { 0x0F };
    }
    q
}

/// Colour the indexed stage image at camera `cx` with the background palette (colour 0 of every palette is the universal backdrop `pal[0]`).
pub fn blit_idx(frame: &mut [u32], img: &[u8], cx: usize, pal: &[u8; 32]) {
    let sw = img.len() / SCREEN_H;
    let mut lut = [0u32; 16];
    for (i, c) in lut.iter_mut().enumerate() {
        *c = NES_RGB[(pal[if i & 3 == 0 { 0 } else { i }] & 0x3F) as usize];
    }
    for y in 0..SCREEN_H {
        let src = &img[y * sw + cx..y * sw + cx + SCREEN_W];
        let dst = &mut frame[y * SCREEN_W..(y + 1) * SCREEN_W];
        for (d, &s) in dst.iter_mut().zip(src) {
            *d = lut[s as usize & 15];
        }
    }
}
