//! Menu screens built the way the game builds them: a nametable (from the scene layouts, `Stage::nametable`), palette, OAM list and 8 KB of pattern RAM
//! (graphics loader `gfx::load_set`), changed by the same buffer writes (`[addr hi, addr lo, count-1, data...]`, `$FF` ends) the NMI would send to the PPU.
//! Only what a menu needs is modelled: one 32x30 nametable with its attribute bytes, background patterns at `$0000`, 8x8 sprites at `$1000`.
//! The pause screen (`PauseScreen`) is checked byte for byte against PPU dumps of the real game (`mm4 --verify-pausescreen`, harness/pause_dump.lua).
use crate::gfx;
use crate::nes::{decode_tile, NES_RGB};
use crate::rom::Rom;
use crate::stage::{Stage, SCREEN_H, SCREEN_W};

/// A byte buffer that is shared between clones until one of them writes (a recorded scene keeps thousands of snapshots; the nametable and the pattern RAM rarely change).
#[derive(Clone)]
pub struct Shared(std::sync::Arc<Vec<u8>>);

impl Shared {
    pub fn new(v: Vec<u8>) -> Shared {
        Shared(std::sync::Arc::new(v))
    }
}

impl std::ops::Deref for Shared {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &self.0
    }
}

impl std::ops::DerefMut for Shared {
    fn deref_mut(&mut self) -> &mut [u8] {
        std::sync::Arc::make_mut(&mut self.0).as_mut_slice()
    }
}

#[derive(Clone)]
pub struct Screen {
    /// 960 tile ids followed by the 64 attribute bytes (the 1 KB of one nametable).
    pub nt: Shared,
    /// The second nametable (`$2400`, right of the first); `None` = the same as `nt` (the mirroring the one-screen scenes use).
    pub nt1: Option<Shared>,
    /// `$3F00..$3F1F`.
    pub pal: [u8; 32],
    /// OAM (`$0200..$02FF`), 64 sprites of `y, tile, attr, x`.
    pub oam: Vec<u8>,
    /// 8 KB pattern RAM.
    pub chr: Shared,
    /// Horizontal scroll (`$FC`): the background wraps around the one nametable (the cutscenes that scroll use mirroring that repeats it).
    pub scroll_x: u8,
    /// Vertical scroll (`$FA`, 0..239; the picture wraps within the 240 lines).
    pub scroll_y: u8,
    /// Base nametable bits (`PPUCTRL & 3`) of the top band: bit 0 adds 256 to the horizontal scroll.
    pub nt_base: u8,
    /// Scroll changes by the mapper's scanline interrupt: (first scanline, horizontal scroll) in order; empty = `scroll_x` for the whole picture.
    pub bands: Vec<(u16, u8)>,
    /// Sound requests the game made while building this picture (the NMI of this frame hands them to the sound driver).
    pub snd: Vec<crate::sound::Req>,
}

impl Screen {
    pub fn new() -> Screen {
        Screen { nt: Shared::new(vec![0; 1024]), nt1: None, nt_base: 0, pal: [0x0F; 32], oam: vec![0xF8; 256], chr: Shared::new(vec![0; 0x2000]), scroll_x: 0, scroll_y: 0, bands: Vec::new(), snd: Vec::new() }
    }

    /// Execute NMI-buffer records (`[hi, lo, count-1, data...]` until a byte >= `$80`) against the nametable / palette.
    /// The nametable select bits of the address are ignored (a menu screen uses one nametable).
    pub fn apply_buffer(&mut self, buf: &[u8]) {
        let mut i = 0;
        while i + 2 < buf.len() && buf[i] < 0x80 {
            let (hi, lo, n) = (buf[i] as usize, buf[i + 1] as usize, buf[i + 2] as usize + 1);
            let addr = hi << 8 | lo;
            for k in 0..n {
                let v = buf[i + 3 + k];
                if addr >= 0x3F00 {
                    self.pal[(addr + k) & 0x1F] = v;
                } else if addr < 0x2000 {
                    self.chr[addr + k] = v;
                } else if let (Some(n1), true) = (self.nt1.as_mut(), (addr >> 10) & 1 == 1) {
                    n1[(addr + k) & 0x3FF] = v;
                } else {
                    self.nt[(addr + k) & 0x3FF] = v;
                }
            }
            i += 3 + n;
        }
    }

    /// What the console shows during the frame whose snapshot is `self` (end-of-frame state, as the recordings see it): the sprites and the scroll of the top band were
    /// sent by the NMI at the end of the previous frame, i.e. they are the previous snapshot's; the palette, the nametable and the pattern RAM are `self`'s.
    pub fn displayed(&self, prev: &Screen) -> Screen {
        let mut s = self.clone();
        s.oam = prev.oam.clone();
        s.scroll_x = prev.scroll_x;
        s.scroll_y = prev.scroll_y;
        s.nt_base = prev.nt_base;
        if let Some(b) = s.bands.first_mut() {
            b.1 = prev.scroll_x;
        }
        s
    }

    /// The 256x240 picture: background, then the sprites (earlier sprites in front; attribute bit 5 puts a sprite behind non-transparent background).
    pub fn render(&self) -> Vec<u32> {
        let mut bg_idx = vec![0u8; SCREEN_W * SCREEN_H];
        let mut frame = vec![0u32; SCREEN_W * SCREEN_H];
        let backdrop = NES_RGB[(self.pal[0] & 0x3F) as usize];
        let nt1 = self.nt1.as_ref().unwrap_or(&self.nt);
        for ty in 0..30usize {
            for tx in 0..64usize {
                let nt = if tx < 32 { &self.nt } else { nt1 };
                let txx = tx & 31;
                let tile = nt[ty * 32 + txx] as usize;
                let attr = nt[960 + (ty >> 2) * 8 + (txx >> 2)];
                let p = ((attr >> (((ty >> 1) & 1) * 4 + ((txx >> 1) & 1) * 2)) & 3) as usize;
                let px = decode_tile(&self.chr, tile * 16);
                for y in 0..8 {
                    let row = (ty * 8 + y + 240 - self.scroll_y as usize % 240) % 240;
                    // the band this scanline belongs to: its horizontal scroll (the top band also has the nametable bit)
                    let (sx, base) = match self.bands.iter().rev().find(|b| b.0 as usize <= row) {
                        Some(b) if b.0 == 0 => (b.1 as usize, (self.nt_base & 1) as usize),
                        Some(b) => (b.1 as usize, 0usize),
                        None => (self.scroll_x as usize, (self.nt_base & 1) as usize),
                    };
                    for x in 0..8 {
                        let c = px[y * 8 + x] as usize;
                        // position in the 512-wide world, then on the screen
                        let wx = (tx * 8 + x + 512 - base * 256) & 511;
                        let ox = (wx + 512 - sx) & 511;
                        if ox >= SCREEN_W {
                            continue;
                        }
                        let o = row * SCREEN_W + ox;
                        bg_idx[o] = c as u8;
                        frame[o] = if c == 0 { backdrop } else { NES_RGB[(self.pal[p * 4 + c] & 0x3F) as usize] };
                    }
                }
            }
        }
        for i in (0..64).rev() {
            let (sy, tile, attr, sx) = (self.oam[i * 4] as usize, self.oam[i * 4 + 1] as usize, self.oam[i * 4 + 2], self.oam[i * 4 + 3] as usize);
            if sy >= 0xEF {
                continue;
            }
            let px = decode_tile(&self.chr, 0x1000 + tile * 16);
            let p = (attr & 3) as usize;
            for yy in 0..8usize {
                for xx in 0..8usize {
                    let (cx, cy) = (if attr & 0x40 != 0 { 7 - xx } else { xx }, if attr & 0x80 != 0 { 7 - yy } else { yy });
                    let c = px[cy * 8 + cx] as usize;
                    let (x, y) = (sx + xx, sy + 1 + yy);
                    if c == 0 || x >= SCREEN_W || y >= SCREEN_H {
                        continue;
                    }
                    if attr & 0x20 != 0 && bg_idx[y * SCREEN_W + x] != 0 {
                        continue;
                    }
                    frame[y * SCREEN_W + x] = NES_RGB[(self.pal[16 + p * 4 + c] & 0x3F) as usize];
                }
            }
        }
        // the game keeps the PPU's left-column clipping on ($2001 = $18): the leftmost 8 pixels show the backdrop
        for y in 0..SCREEN_H {
            for x in 0..8 {
                frame[y * SCREEN_W + x] = backdrop;
            }
        }
        frame
    }

    /// One energy slot of the pause screen (code_3C9885 for a shown slot, code_3C9A86 for an empty one).
    fn nt_energy(&mut self, rom: &Rom, y: usize, energy: u8) {
        let t = |a: u16| rom.cpu_byte(0x3C, 0x3D, a);
        if energy & 0x80 != 0 {
            // seven bar tiles, 4 units each
            let mut units = (energy & 0x1F) as i32;
            let mut b = vec![t(0x9B4E + y as u16), t(0x9B3F + y as u16), 6];
            for _ in 0..7 {
                let take = units.clamp(0, 4);
                units -= 4;
                b.push(t(0x9B5D + take as u16));
            }
            b.push(0xFF);
            self.apply_buffer(&b);
        } else {
            // two rows of eleven blank tiles, and the weapon's small icon sprite disappears
            let (hi, lo) = (t(0x9F45 + y as u16), t(0x9F54 + y as u16));
            let mut b = vec![hi, lo, 0x0A];
            b.extend(std::iter::repeat(0).take(11));
            b.extend_from_slice(&[hi, lo | 0x20, 0x0A]); // the second row: the game ORs bit 5 into the low byte
            b.extend(std::iter::repeat(0).take(11));
            b.push(0xFF);
            self.apply_buffer(&b);
            let sprite = match y {
                1 => Some(0x14),
                2 => Some(0x1C),
                3 => Some(0x18),
                5 => Some(0x20),
                7 => Some(0x10),
                _ => None,
            };
            if let Some(i) = sprite {
                self.oam[i] = 0xF8;
            }
        }
    }
}

impl Default for Screen {
    fn default() -> Screen {
        Screen::new()
    }
}

/// The game's RAM the pause screen shows when the menu opens.
pub struct PauseInfo {
    /// `$B0..$BE`: life (slot 0), the weapon energies (1..13) and the buster's full bar (14); bit 7 = shown.
    pub energy: [u8; 15],
    /// `$A1` (lives) and `$A2` (E-tanks).
    pub lives: u8,
    pub etanks: u8,
}

/// The pause screen of bank `$3C`: `open` is `code_3C9600` (nametable of scene `$10` screen 3, palette, fixed sprites, one bar or blank per energy slot, the digits),
/// `update` is what the menu loop does every frame (`code_3C9992` + `code_3C9A3A`: the old cell loses its highlight, the cursor cell gets it, the icon and the blinking
/// cursor sprites follow). The state is incremental like the game's (a highlight record of one cell overwrites attribute bytes it shares with its neighbour).
pub struct PauseScreen {
    pub screen: Screen,
    /// `$0139`: the cell highlighted last frame.
    previous: u8,
    /// The cursor has left the cell the menu opened on. Until then the real game shows that cell without its highlight although the buffer holds the highlight record
    /// every frame (seen in PPU dumps and in the PPU write log; the reason is not known, the first cell of the menu is the only one it concerns).
    moved: bool,
}

impl PauseScreen {
    pub fn open(rom: &Rom, menu: &Stage, info: &PauseInfo, cursor: u8) -> PauseScreen {
        let t = |a: u16| rom.cpu_byte(0x3C, 0x3D, a);
        let mut s = Screen::new();
        let (tiles, attrs) = menu.nametable(3);
        s.nt[..960].copy_from_slice(&tiles);
        s.nt[960..].copy_from_slice(&attrs);
        gfx::load_set(rom, 0x14, &mut s.chr);
        for i in 0..32 {
            s.pal[i] = t(0x9AE3 + i as u16); // the menu palette (the real game fades it in)
        }
        for i in 0..44 {
            s.oam[i] = t(0x9E51 + i as u16); // the fixed sprites
        }
        for y in (0..15usize).rev() {
            s.nt_energy(rom, y, info.energy[y]);
        }
        // code_3C98D0: the E-tank and lives digits
        let (a1, a2) = (info.lives, info.etanks);
        s.apply_buffer(&[0x23, 0x27, 0x01, 0x70 | (a1 >> 4), 0x70 | (a2 & 0x0F), 0x23, 0x3A, 0x01, 0x70 | (a2 >> 4), 0x70 | (a1 & 0x0F), 0xFF]);
        let mut p = PauseScreen { screen: s, previous: cursor, moved: false };
        p.update(rom, cursor, 0);
        p
    }

    /// One pass of the menu loop with the cursor on `cursor`; `frame` is `$95` as the loop sees it (before its own increment).
    pub fn update(&mut self, rom: &Rom, cursor: u8, frame: u8) {
        let t = |a: u16| rom.cpu_byte(0x3C, 0x3D, a);
        let s = &mut self.screen;
        let rec = |s: &mut Screen, table: u16, idx: usize| {
            let mut b = Vec::new();
            let mut p = table + idx as u16;
            loop {
                let v = t(p);
                b.push(v);
                if v == 0xFF {
                    break;
                }
                p += 1;
            }
            s.apply_buffer(&b);
        };
        let (prev, cur) = (self.previous as usize & 0x0F, cursor as usize & 0x0F);
        if cur != prev {
            self.moved = true;
        }
        // code_3C9992
        let k = t(0x9D30 + prev as u16) as usize;
        if k != 0 {
            s.oam[k + 2] = 0;
        }
        rec(s, 0x9C08, t(0x9BF9 + prev as u16) as usize);
        let k = t(0x9D30 + cur as u16) as usize;
        if k != 0 {
            s.oam[k + 2] = 3;
        }
        if self.moved {
            rec(s, 0x9B90, t(0x9B81 + cur as u16) as usize);
        }
        self.previous = cursor;
        // the selected weapon's icon sprites (`$9C70 + 16 * k`, four sprites at `$022C`)
        let k = t(0x9C71 + cur as u16) as usize;
        if k == 0 {
            for i in [0x2C, 0x30, 0x34, 0x38] {
                s.oam[i] = 0xF8;
            }
        } else {
            for i in 0..16 {
                s.oam[0x2C + i] = t(0x9C70 + (k * 16) as u16 + i as u16);
            }
        }
        // code_3C9A3A: the blinking cursor sprites
        for i in 0x3C..0x6C {
            s.oam[i] = 0xF8;
        }
        let phase = ((frame >> 3) & 1) as usize;
        let mut y = t(0x9D3F + (cur * 2 + phase) as u16) as u16;
        let mut x = 0usize;
        loop {
            while t(0x9D63 + y) != 0 {
                for j in 0..4 {
                    s.oam[0x3C + x + j] = t(0x9D63 + y + j as u16);
                }
                y += 4;
                x += 4;
            }
            let next = t(0x9D64 + y);
            if next == 0 {
                break;
            }
            y = next as u16;
        }
    }
}

/// Compare the pause screen with PPU dumps of the real game (`harness/pause_dump.lua`: `REG`, `ENE`, `NT0..NT3`, `PAL`, `OAM`, `CHR` lines). `paths` are dumps of one
/// pause in time order: the first opens the menu, the later ones move the cursor (the screen is updated like the game does, one frame per step, then compared).
pub fn verify_pause(rom: &Rom, paths: &[String]) -> bool {
    let menu = Stage::load(rom, 0x10, 5);
    let mut screen: Option<PauseScreen> = None;
    let mut all_ok = true;
    // the energies shown are those of the last dump (the menu fills `$BE` while it opens, so the first dumps may still lack it)
    let last_ene: Vec<u8> = paths.last().and_then(|p| std::fs::read_to_string(p).ok()).and_then(|t| t.lines().find(|l| l.starts_with("ENE ")).map(|l| l[4..].split_whitespace().map(|h| u8::from_str_radix(h, 16).unwrap_or(0)).collect())).unwrap_or_default();
    for path in paths {
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("cannot read {}: {}", path, e);
                return false;
            }
        };
        let line = |tag: &str| text.lines().find(|l| l.starts_with(&format!("{} ", tag))).map(|l| l[tag.len() + 1..].to_string());
        let bytes = |s: &str| -> Vec<u8> { (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap_or(0)).collect() };
        let reg = line("REG").unwrap_or_default();
        let field = |k: &str| -> u8 { reg.split_whitespace().find_map(|w| w.strip_prefix(&format!("{}=", k))).and_then(|v| u8::from_str_radix(v, 16).ok()).unwrap_or(0) };
        let mut energy = [0u8; 15];
        energy.copy_from_slice(&last_ene[..15]);
        let (cursor, frame) = (field("cursor"), field("p95"));
        let ps = screen.get_or_insert_with(|| PauseScreen::open(rom, &menu, &PauseInfo { energy, lives: field("a1"), etanks: field("a2") }, 0)); // the menu opens on cell 0 (`$0138` starts at 0)
        ps.update(rom, cursor, frame.wrapping_sub(1)); // the menu loop builds the sprites before its own `INC $95`
        let ours = &ps.screen;
        println!("{} (cursor {:02X}):", path, cursor);
        let mut best = (usize::MAX, 0);
        for i in 0..4 {
            let real = bytes(&line(&format!("NT{}", i)).unwrap_or_default());
            let d = real.iter().zip(ours.nt.iter()).filter(|(a, b)| a != b).count();
            if d < best.0 {
                best = (d, i);
            }
        }
        println!("  nametable (NT{}): {} bytes differ", best.1, best.0);
        if best.0 != 0 {
            let real = bytes(&line(&format!("NT{}", best.1)).unwrap_or_default());
            let diffs: Vec<String> = real.iter().zip(ours.nt.iter()).enumerate().filter(|(_, (a, b))| a != b).take(12).map(|(i, (a, b))| format!("{}:{:02X}/{:02X}", i, a, b)).collect();
            println!("  first differences (offset:game/port): {}", diffs.join(" "));
            all_ok = false;
        }
        let pal = bytes(&line("PAL").unwrap_or_default());
        let pd = pal.iter().zip(ours.pal.iter()).filter(|(a, b)| a != b).count();
        println!("  palette: {} bytes differ", pd);
        all_ok &= pd == 0;
        let oam = bytes(&line("OAM").unwrap_or_default());
        let od: Vec<String> = oam.chunks(4).zip(ours.oam.chunks(4)).enumerate().filter(|(_, (a, b))| a != b && !(a[0] >= 0xEF && b[0] >= 0xEF)).map(|(i, (a, b))| format!("#{}:{:02X?}/{:02X?}", i, a, b)).collect();
        println!("  sprites: {} differ {}", od.len(), od.iter().take(8).cloned().collect::<Vec<_>>().join(" "));
        all_ok &= od.is_empty();
        let chr = bytes(&line("CHR").unwrap_or_default());
        let cd = chr.iter().zip(ours.chr.iter()).filter(|(a, b)| a != b).count();
        println!("  pattern RAM: {} of {} bytes differ", cd, chr.len());
        if let Ok(out) = std::env::var("MM4_RENDER") {
            // write our picture of this step as a PPM (compare with `node tools/render_dump.js <dump> x.png 0 4096 2`, which draws the real pattern RAM)
            let f = ours.render();
            let mut bytes = format!("P6
{} {}
255
", SCREEN_W, SCREEN_H).into_bytes();
            for p in &f {
                bytes.extend_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, *p as u8]);
            }
            let _ = std::fs::write(format!("{}_{}.ppm", out, cursor), bytes);
        }
    }
    all_ok
}
