//! Reader for the per-frame PPU recordings of `harness/ppu_frames.lua` and a helper to compare a modelled `Screen` with one recorded frame.
use crate::screen::Screen;
use std::collections::HashMap;

#[derive(Clone, Default)]
pub struct PfFrame {
    pub n: u32,
    pub reg: HashMap<String, u32>,
    pub pal: Vec<u8>,
    pub oam: Vec<u8>,
    pub buf: Vec<u8>,
    /// `$A0..$BF`.
    pub ram: Vec<u8>,
    /// The first four object slots (24 bytes each, see `ppu_frames.lua`).
    pub obj: Vec<u8>,
    /// The four nametables as last dumped (the recorder writes one only when it changed).
    pub nt: [Vec<u8>; 4],
    pub chr: Vec<u8>,
}

impl PfFrame {
    pub fn r(&self, k: &str) -> u32 {
        self.reg.get(k).copied().unwrap_or(0)
    }
}

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap_or(0)).collect()
}

pub fn load(path: &str) -> Result<Vec<PfFrame>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("cannot read {}: {}", path, e))?;
    let mut frames: Vec<PfFrame> = Vec::new();
    let mut nt: [Vec<u8>; 4] = Default::default();
    let mut chr: Vec<u8> = Vec::new();
    let mut cur: Option<PfFrame> = None;
    for l in text.lines() {
        if let Some(rest) = l.strip_prefix("F ") {
            if let Some(c) = cur.take() {
                frames.push(c);
            }
            let mut it = rest.split_whitespace();
            let n = it.next().and_then(|v| v.parse().ok()).unwrap_or(0);
            let mut f = PfFrame { n, ..Default::default() };
            for kv in it {
                if let Some((k, v)) = kv.split_once('=') {
                    f.reg.insert(k.to_string(), u32::from_str_radix(v, 16).unwrap_or(0));
                }
            }
            cur = Some(f);
        } else if let Some(c) = cur.as_mut() {
            if let Some((tag, hex)) = l.split_once(' ') {
                match tag {
                    "PAL" => c.pal = unhex(hex),
                    "OAM" => c.oam = unhex(hex),
                    "BUF" => c.buf = unhex(hex),
                    "RAM" => c.ram = unhex(hex),
                    "OBJ" => c.obj = unhex(hex),
                    "CHR" => chr = unhex(hex),
                    t if t.starts_with("NT") && t.len() == 3 => {
                        if let Some(i) = t[2..].parse::<usize>().ok().filter(|&i| i < 4) {
                            nt[i] = unhex(hex);
                        }
                    }
                    _ => {}
                }
            }
            c.nt = nt.clone();
            c.chr = chr.clone();
        }
    }
    if let Some(c) = cur.take() {
        frames.push(c);
    }
    Ok(frames)
}

impl PfFrame {
    /// Horizontal scroll the NMI writes for the top of the picture: `$FC`, or `$69` in the scroll modes (`$F0` = 3, 6, 7 and from 9 on: `code_3EC015`).
    pub fn top_scroll(&self) -> u8 {
        let f0 = self.r("f0");
        if matches!(f0, 3 | 6 | 7) || f0 >= 9 {
            self.r("x69") as u8
        } else {
            self.r("fc") as u8
        }
    }

    /// The nametable bits the NMI writes to PPUCTRL for the top band (`$6A` in the scroll modes, else `$FD`, ORed with `$FF`).
    pub fn top_base(&self) -> u8 {
        let f0 = self.r("f0");
        let v = if matches!(f0, 3 | 6 | 7) || f0 >= 9 { self.r("x6a") } else { self.r("fd") };
        ((v | self.r("ff")) & 3) as u8
    }

    /// The scanline interrupt's scroll bands for the story scenes (`$F0` = 10: the lower part is unscrolled; 11: a middle band uses `$FC`, then unscrolled).
    pub fn bands(&self) -> Vec<(u16, u8)> {
        let f1 = self.r("f1") as u16;
        match self.r("f0") {
            0x0A => vec![(0, self.top_scroll()), (f1 + 1, 0)],
            0x0B => vec![(0, self.top_scroll()), (f1 + 1, self.r("fc") as u8), (f1 + 1 + 0x29, 0)],
            _ => Vec::new(),
        }
    }

    /// The recorded frame as a `Screen` (the displayed nametable, palette, OAM RAM, pattern RAM, scroll).
    pub fn to_screen(&self) -> Screen {
        let mut s = Screen::new();
        let nt = &self.nt[(self.r("ff") & 3) as usize];
        s.nt[..nt.len().min(1024)].copy_from_slice(&nt[..nt.len().min(1024)]);
        s.pal.copy_from_slice(&self.pal[..32]);
        s.oam.copy_from_slice(&self.oam[..256]);
        s.chr[..self.chr.len().min(0x2000)].copy_from_slice(&self.chr[..self.chr.len().min(0x2000)]);
        s.scroll_x = self.top_scroll();
        s.scroll_y = self.r("fa") as u8;
        s.nt_base = self.top_base();
        s.bands = self.bands();
        if self.nt[1].len() == 1024 {
            s.nt1 = Some(crate::screen::Shared::new(self.nt[1].clone()));
        }
        s
    }
}

/// The differences between a modelled screen and a recorded frame.
pub struct Diff {
    pub pal_detail: String,
    pub pal: usize,
    pub oam: Vec<String>,
    pub nt: usize,
    pub chr: usize,
    pub scroll: bool,
}

impl Diff {
    pub fn ok(&self) -> bool {
        self.pal == 0 && self.oam.is_empty() && self.nt == 0 && self.chr == 0 && !self.scroll
    }
}

/// `check_nt`: compare the displayed nametable (index from `$FF` bits 0-1); `check_chr`: compare the pattern RAM.
pub fn compare(f: &PfFrame, s: &Screen, check_nt: bool, check_chr: bool) -> Diff {
    let pal = f.pal.iter().zip(s.pal.iter()).filter(|(a, b)| a != b).count();
    let oam: Vec<String> = f.oam.chunks(4).zip(s.oam.chunks(4)).enumerate().filter(|(_, (a, b))| a != b && !(a[0] >= 0xEF && b[0] >= 0xEF)).map(|(i, (a, b))| format!("#{}:{:02X?}/{:02X?}", i, a, b)).collect();
    let nt = if check_nt {
        let real = &f.nt[(f.r("ff") & 3) as usize];
        real.iter().zip(s.nt.iter()).filter(|(a, b)| a != b).count() + real.len().abs_diff(s.nt.len())
    } else {
        0
    };
    // only the tiles the picture can show (the background tiles of the nametable, the 8x8 sprite tiles at $1000): the rest of the pattern RAM is left over from earlier scenes
    let chr = if check_chr {
        let mut used = vec![false; 512];
        for &t in &s.nt[..960] {
            used[t as usize] = true;
        }
        for sp in s.oam.chunks(4) {
            if sp[0] < 0xEF {
                used[256 + sp[1] as usize] = true;
            }
        }
        (0..512).filter(|&t| used[t]).map(|t| f.chr[t * 16..t * 16 + 16].iter().zip(&s.chr[t * 16..t * 16 + 16]).filter(|(a, b)| a != b).count()).sum()
    } else {
        0
    };
    let pal_detail = if pal != 0 { format!("game {} / port {}", f.pal.iter().map(|b| format!("{:02X}", b)).collect::<String>(), s.pal.iter().map(|b| format!("{:02X}", b)).collect::<String>()) } else { String::new() };
    let scroll = f.top_scroll() != s.scroll_x || f.r("fa") as u8 != s.scroll_y;
    Diff { pal_detail, pal, oam, nt, chr, scroll }
}

/// Is anything of the picture visible (a palette that is not all black)?
pub fn visible(s: &Screen) -> bool {
    s.pal.iter().any(|&c| c != 0x0F)
}

/// Print a failing frame; returns whether it was ok.
pub fn report(name: &str, f: &PfFrame, t: u32, d: &Diff, shown: &mut u32) -> bool {
    if d.ok() {
        return true;
    }
    *shown += 1;
    if *shown <= 10 {
        if !d.pal_detail.is_empty() && std::env::var("MM4_PALDIFF").is_ok() {
            println!("  palette {}", d.pal_detail);
        }
        println!("{} frame {} (t {}): palette {} differ, sprites {} differ {}, nametable {} differ, pattern RAM {} differ{}", name, f.n, t, d.pal, d.oam.len(), d.oam.iter().take(4).cloned().collect::<Vec<_>>().join(" "), d.nt, d.chr, if d.scroll { ", scroll differs" } else { "" });
    }
    false
}
