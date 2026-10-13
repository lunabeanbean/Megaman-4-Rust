//! The console's nametable of a horizontally scrolling stage section: with the cartridge's horizontal mirroring (`$A000 = 1`, code_3EC976) the two
//! nametables are one, so the 32 tile columns are a ring and the game's column builder (`code_3ECFB1`) overwrites the column that just left the screen.
//! Tiles and the attribute table are kept like the PPU holds them, so the stale column that shows at the right edge while the fine scroll is not 0 (and the
//! attribute bits that are only rewritten when an odd column is built) come out like the console's.
use crate::nes::{decode_tile, NES_RGB};
use crate::stage::{Stage, SCREEN_H, SCREEN_W};

#[derive(Clone)]
pub struct NameTable {
    /// `tile[row * 32 + column]`, 30 rows.
    pub tile: [u8; 32 * 30],
    /// The attribute table (`$23C0..`): one byte per 32x32 pixels, `attr[(row / 4) * 8 + column / 4]`.
    pub attr: [u8; 64],
    /// The game's RAM copy of the attribute table (`$0640..`): the builders merge into it and upload the result; some objects only change the copy.
    pub shadow: [u8; 64],
}

/// One metatile as the game's block decoder (`code_3ED17F`) leaves it: its index, whether the stage's overlays blank its tiles (stage 4's pink bars and the `$F0` = 4 blocks that
/// are gone according to the `$0680` bitmap) and its palette (stage 2's closed shutters use palette 1).
fn meta_info(st: &Stage, page: u8, a29: usize, k: usize) -> (usize, bool, u8) {
    let layout = st.byte(0xB500 + page as usize) as usize;
    let block = st.byte(0xA900 + layout * 64 + a29) as usize;
    let m = st.byte(0xA500 + block * 4 + k) as usize;
    let mut pal = st.byte(0xA400 + m) & 3;
    let mut blank = false;
    let vertical = st.vsect.get();
    if st.id == 2 && st.byte(0xA400 + m) & 0xF0 == 0x70 && st.door[page as usize].get() & 0x0F != 0 {
        pal = 1; // code_3ED230
    }
    if st.id == 4 && (m == 9 || m == 0x8C) {
        // code_3ED1D7: the bar's bit in the `$0680` bitmap
        let e6 = if vertical { 0 } else { (page as usize & 1) << 5 };
        let t = a29 >> 1;
        let e7 = (k & 2) | e6 | (t & 0xFC) | ((t >> 1) & 1);
        let idx = ((a29 << 1) | (k & 1)) & 7;
        blank = st.blocks[e7 & 63].get() & (1 << idx) != 0;
    }
    if st.sway.get() && (0x0C..0x11).contains(&page) {
        // code_3ED24B: blocks listed at `$E566/$E577` disappear with their bit
        let mut y = st.byte(0xE566 + page as usize) as usize;
        loop {
            let v = st.byte(0xE577 + y);
            if v & 0x80 != 0 {
                break;
            }
            if v as usize == a29 {
                let bit = (st.byte(0xE578 + y) as usize) | k;
                blank = st.blocks[(bit >> 3) & 63].get() & (1 << (bit & 7)) != 0;
                break;
            }
            y += 2;
        }
    }
    (m, blank, pal)
}

impl NameTable {
    pub fn new() -> NameTable {
        NameTable { tile: [0; 32 * 30], attr: [0; 64], shadow: [0; 64] }
    }

    /// The tile numbers and attribute palettes (top/bottom metatile of each of the 8 block rows) of world column `col` (0..31) of page `page`.
    pub fn column(st: &Stage, page: u8, col: usize) -> ([u8; 30], [(u8, u8); 8]) {
        let layout = st.byte(0xB500 + page as usize) as usize;
        let mut tiles = [0u8; 30];
        let mut pals = [(0u8, 0u8); 8];
        let bc = col >> 2;
        let _ = layout;
        for br in 0..8usize {
            for mr in 0..2usize {
                let k = mr * 2 + ((col >> 1) & 1);
                let (m, blank, pal) = meta_info(st, page, br * 8 + bc, k);
                if mr == 0 {
                    pals[br].0 = pal;
                } else {
                    pals[br].1 = pal;
                }
                for tr in 0..2usize {
                    let row = (br * 2 + mr) * 2 + tr;
                    if row < 30 {
                        let t = tr * 2 + (col & 1);
                        tiles[row] = if blank { 0 } else { st.byte(0xA000 + 0x100 * t + m) };
                    }
                }
            }
        }
        (tiles, pals)
    }

    /// `code_3ECFB1`: build column `col` (`$24`) of page `page` (`$2A`) into the ring: 30 tiles, and on an odd column the attribute half of each block row.
    pub fn build_column(&mut self, st: &Stage, page: u8, col: u8) {
        if std::env::var("MM4_NTWATCH").is_ok() { eprintln!("  build_column page {} col {}", page, col); }
        let col = (col & 0x1F) as usize;
        let (tiles, pals) = Self::column(st, page, col);
        for (row, &t) in tiles.iter().enumerate() {
            self.tile[row * 32 + col] = t;
        }
        if col & 1 != 0 {
            let y = (col & 3) >> 1;
            let (keep, new) = if y == 0 { (0xCC, 0x33) } else { (0x33, 0xCC) };
            for (br, &(top, bot)) in pals.iter().enumerate() {
                let bits = top | top << 2 | bot << 4 | bot << 6;
                let i = br * 8 + (col >> 2);
                self.shadow[i] = (self.shadow[i] & keep) | (bits & new);
                self.attr[i] = self.shadow[i];
            }
        }
    }

    /// The whole ring for a camera at world X `cx`: the stage load builds the 32 columns of the page and then the first column of the next one (code_3EC5AA), which
    /// replaces column 0 in the ring.
    pub fn fill(&mut self, st: &Stage, cx: i32) {
        let first = (cx >> 3).max(0);
        for w in first..first + 33 {
            self.build_column(st, (w >> 5) as u8, (w & 31) as u8);
        }
    }

    /// The picture at scroll `cx` (the PPU's X scroll is `cx & 255`; both nametables are this one).
    pub fn render(&self, frame: &mut [u32], st: &Stage, pal: &[u8; 32], cx: usize) {
        let mut tiles: Vec<Option<[u8; 64]>> = vec![None; 256];
        for y in 0..SCREEN_H {
            let row = y >> 3;
            for x in 0..SCREEN_W {
                let wx = (cx + x) & 255;
                let s = wx >> 3;
                let t = self.tile[row * 32 + s] as usize;
                let td = tiles[t].get_or_insert_with(|| decode_tile(&st.chr, t * 16));
                let c = td[(y & 7) * 8 + (wx & 7)] as usize;
                let ab = self.attr[(row >> 2) * 8 + (s >> 2)];
                let quad = ((row >> 1) & 1) * 2 + ((s >> 1) & 1);
                let p = ((ab >> (quad * 2)) & 3) as usize;
                let idx = if c == 0 { 0 } else { p * 4 + c };
                frame[y * SCREEN_W + x] = NES_RGB[(pal[idx] & 0x3F) as usize];
            }
        }
    }
}

impl NameTable {
    /// `code_3ED052` (the vertical scroll's row builder): tile row `row` (`$24`, 0..29) of page `page` (`$2A`): 32 tiles (the first one comes from the next page, the way the game's buffer
    /// is built) and the half of the 8 attribute bytes of its cell row that belongs to this metatile row. With the vertical mirroring of the vertical sections the 30 rows are a ring.
    pub fn build_row(&mut self, st: &Stage, page: u8, row: u8, attr: bool) {
        let row = (row as usize).min(29);
        let (cr, mr, tr) = (row >> 2, (row >> 1) & 1, row & 1);
        for (pg, only_first) in [(page, false), (page.wrapping_add(1), true)] {
            for bc in 0..8usize {
                let mut pals = [0u8; 2];
                for mc in 0..2usize {
                    let (m, blank, pal) = meta_info(st, pg, cr * 8 + bc, mr * 2 + mc);
                    pals[mc] = pal;
                    for tc in 0..2usize {
                        let x = bc * 4 + mc * 2 + tc;
                        if only_first && x != 0 {
                            continue;
                        }
                        self.tile[row * 32 + x] = if blank { 0 } else { st.byte(0xA000 + 0x100 * (tr * 2 + tc) + m) };
                    }
                }
                if !only_first && attr {
                    let bits = pals[0] | pals[1] << 2;
                    let (keep, new) = if mr == 0 { (0xF0, 0x0F) } else { (0x0F, 0xF0) };
                    let i = cr * 8 + bc;
                    self.shadow[i] = (self.shadow[i] & keep) | ((bits | bits << 4) & new);
                    self.attr[i] = self.shadow[i];
                }
            }
        }
    }

    /// The picture with the vertical scroll `fy` (0..239) and no horizontal scroll: the 30 rows wrap.
    pub fn render_v(&self, frame: &mut [u32], st: &Stage, pal: &[u8; 32], fy: usize) {
        let mut tiles: Vec<Option<[u8; 64]>> = vec![None; 256];
        for y in 0..SCREEN_H {
            let wy = (fy + y) % 240;
            let row = wy >> 3;
            for x in 0..SCREEN_W {
                let s = x >> 3;
                let t = self.tile[row * 32 + s] as usize;
                let td = tiles[t].get_or_insert_with(|| decode_tile(&st.chr, t * 16));
                let c = td[(wy & 7) * 8 + (x & 7)] as usize;
                let ab = self.attr[(row >> 2) * 8 + (s >> 2)];
                let quad = ((row >> 1) & 1) * 2 + ((s >> 1) & 1);
                let p = ((ab >> (quad * 2)) & 3) as usize;
                let idx = if c == 0 { 0 } else { p * 4 + c };
                frame[y * SCREEN_W + x] = NES_RGB[(pal[idx] & 0x3F) as usize];
            }
        }
    }
}
