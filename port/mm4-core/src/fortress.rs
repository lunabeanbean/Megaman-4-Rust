//! The fortress map screens of the second half (bank `$39`): Dr. Cossack's castle (`code_398107` the first time, `code_39833C` after each cleared stage of his) and Wily's
//! fortress (`code_3983E8`). Scene `$12`, graphics set `$3E` / `$3F`, lightning flashes, the lights of the cleared stages and the blinking one of the next stage.
//! Checked frame by frame against Mesen (`mm4 --verify-fortress`, `harness/pfm_*.txt`).
use crate::gfx;
use crate::rom::Rom;
use crate::scene::SceneRunner;
use crate::screen::Screen;
use crate::stage::Stage;

fn b39(rom: &Rom, a: u16) -> u8 {
    rom.cpu_byte(0x39, 0x3A, a)
}

/// `code_398627`: append the sprite lists of the first stages (one per set bit of `bits`, from bit 0 while the bits are set) at OAM byte `$97`; returns (`$97`, `$02`).
fn add_lists(r: &mut SceneRunner, rom: &Rom, mut bits: u8, mut p97: usize, first: usize) -> (usize, usize) {
    let mut y2 = first;
    loop {
        let carry = bits & 1;
        bits >>= 1;
        if carry == 0 {
            return (p97, y2);
        }
        let ptr = b39(rom, 0x8B53 + y2 as u16) as u16 | (b39(rom, 0x8B5B + y2 as u16) as u16) << 8;
        let mut y = 0u16;
        loop {
            if b39(rom, ptr + y) == 0 {
                break;
            }
            for k in 0..4 {
                r.ppu.oam[p97 + k] = b39(rom, ptr + y + k as u16);
            }
            y += 4;
            if b39(rom, ptr + y) == 1 {
                y += 1;
            }
            p97 += 4;
        }
        y2 += 1;
    }
}

/// `code_3986B7`: `x` frames of the blinking tiles of the light of the next stage (`$AA` low bits count the cleared ones).
fn blink(r: &mut SceneRunner, rom: &Rom, aa: u8, x: u32, p95: &mut u8) {
    for _ in 0..x {
        let b = ((*p95 >> 3) & 1) as u16;
        let mut y = 0u16;
        let mut a = aa;
        loop {
            let carry = a & 1;
            a >>= 1;
            if carry == 0 {
                break;
            }
            y += 1;
            if y == 7 {
                break;
            }
        }
        let e = y * 4;
        let mut ox = b39(rom, 0x86FC + e) as usize;
        let mut cnt = b39(rom, 0x86FD + e) as i32;
        let mut ty = b39(rom, 0x86FE + e + b) as u16;
        loop {
            r.ppu.oam[ox + 1] = b39(rom, 0x871C + ty);
            ox += 4;
            ty += 1;
            cnt -= 1;
            if cnt < 0 {
                break;
            }
        }
        r.yield_frame();
        *p95 = p95.wrapping_add(1);
    }
}

/// `code_39866B`: the sprites of list `y` one by one, a four-frame blink after each (unless the next byte is a marker `1`).
fn add_next(r: &mut SceneRunner, rom: &Rom, aa: u8, y2: usize, mut p97: usize, p95: &mut u8) {
    let ptr = b39(rom, 0x8B53 + y2 as u16) as u16 | (b39(rom, 0x8B5B + y2 as u16) as u16) << 8;
    let mut y = 0u16;
    loop {
        if b39(rom, ptr + y) == 0 {
            return;
        }
        for k in 0..4 {
            r.ppu.oam[p97 + k] = b39(rom, ptr + y + k as u16);
        }
        y += 4;
        if b39(rom, ptr + y) == 1 {
            y += 1;
        } else {
            r.snd(0x1D, 0x86A1); // code_39869E
            blink(r, rom, aa, 4, p95);
        }
        p97 += 4;
    }
}

/// Which map screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapKind {
    /// `code_398107`: Dr. Cossack's castle, entered from the stage select (lightning first).
    CossackFirst,
    /// `code_39833C`: the same castle after a cleared stage of his.
    CossackAfter,
    /// `code_3983E8`: Wily's fortress.
    Wily,
}

/// `code_39821A`: six flickers of the backdrop colour (ten frames each).
fn flicker(r: &mut SceneRunner, rom: &Rom) {
    let mut k = 6u8;
    while k != 0 {
        r.ram_pal[0x10] = b39(rom, 0x889F + (k & 1) as u16);
        r.f18 = true;
        r.wait(0x0A);
        k -= 1;
    }
}

/// A map scene from the first frame of scene `$12` (the column builder) to the end of the last blink; `aa` = `$AA`.
pub fn map_scene(rom: &Rom, kind: MapKind, aa: u8, p92: u8, p95: u8) -> Vec<Screen> {
    let scene = Stage::load(rom, 0x12, 5);
    let mut s = Screen::new();
    let (tiles, attrs) = scene.nametable(if kind == MapKind::Wily { 2 } else { 0 });
    s.nt[..960].copy_from_slice(&tiles);
    s.nt[960..].copy_from_slice(&attrs);
    gfx::load_set(rom, 0x55, &mut s.chr);
    gfx::load_set(rom, 0x10, &mut s.chr);
    gfx::load_set(rom, if kind == MapKind::Wily { 0x3F } else { 0x3E }, &mut s.chr);
    s.pal = [0x0F; 32];
    let mut r = SceneRunner::new(s, p92, p95);
    r.wait(32);
    match kind {
        MapKind::CossackFirst => {
            // the template sprites, then 96 frames of lightning: the backdrop and the palettes flash white for the first three frames of every 32
            for i in 0..0xA0usize {
                r.ppu.oam[i] = b39(rom, 0x88F8 + i as u16);
            }
            r.snd_music(0xF0, 0x814D); // code_39814A
            let mut p = 0u8;
            for _ in 0..0x60 {
                if p & 0x1F == 0 {
                    r.snd(0x1C, 0x8162); // code_39815F: thunder
                }
                let base: u16 = if (p & 0x1F) < 3 { 0x10 } else { 0 };
                for x in 0..4u16 {
                    let y = base + x;
                    let v = [b39(rom, 0x87EE + y), b39(rom, 0x87F2 + y), b39(rom, 0x87F6 + y), b39(rom, 0x87FA + y)];
                    let x = x as usize;
                    for (k, &c) in v.iter().enumerate() {
                        r.ram_pal[k * 8 + x] = c;
                        r.ram_pal[k * 8 + 4 + x] = c;
                    }
                }
                r.f18 = true;
                r.yield_frame();
                p = p.wrapping_add(1);
            }
            for i in 0..32 {
                r.target[i] = b39(rom, 0x87CE + i as u16);
            }
            for i in (0x30..0x100).step_by(4) {
                r.ppu.oam[i] = 0xF8;
            }
            r.fade_in();
            r.snd_music(0x3C, 0x81BD); // code_3981BA
            r.wait(0);
            r.wait(0);
        }
        MapKind::CossackAfter => {
            for i in 0..0x30usize {
                r.ppu.oam[i] = b39(rom, 0x88F8 + i as u16);
            }
            for i in 0..32 {
                r.target[i] = b39(rom, 0x87CE + i as u16);
            }
            r.snd_music(0x3C, 0x838F); // code_39838C
            r.yield_frame();
            r.fade_in();
            r.wait(0);
            r.wait(0);
        }
        MapKind::Wily => {
            r.yield_frame(); // (the column builder of screen 2 takes one frame more)
            for i in 0..32 {
                r.target[i] = b39(rom, 0x880E + i as u16);
            }
            r.snd_music(0x3B, 0x8432); // code_39842F
            r.yield_frame();
            r.fade_in();
            r.wait(0xF0); // X is $F0 when the routine returns

            r.wait(0);
        }
    }
    flicker(&mut r, rom);
    let mut p95 = 0u8; // the flicker counted `$95` down to zero
    let (pal_src, copy8, sprites): (u16, bool, Vec<(usize, u16)>) = match kind {
        MapKind::CossackFirst => (0x87AE, true, (0..0x20).map(|i| (0x30 + i, 0x8998 + i as u16)).collect()),
        MapKind::CossackAfter => (0x87AE, false, (0..0x20).map(|i| (0x30 + i, 0x8998 + i as u16)).collect()),
        MapKind::Wily => {
            let n: u16 = if aa == 0x7F { 0x2C } else { 0x1C };
            (0x882E, false, (0..n as usize).map(|i| (0x28 + i, 0x89E0 + i as u16)).collect())
        }
    };
    let n = if kind == MapKind::Wily { 32 } else { 16 };
    for y in (0..n).rev() {
        let a = b39(rom, pal_src + y as u16);
        r.ram_pal[y] = a;
        r.target[y] = a;
        if copy8 && y < 8 {
            r.ram_pal[16 + y] = a;
            r.target[16 + y] = a;
        }
    }
    if kind == MapKind::CossackAfter {
        for y in 16..32 {
            let a = b39(rom, 0x87AE + y as u16);
            r.ram_pal[y] = a;
            r.target[y] = a;
        }
    }
    r.f18 = true;
    for (o, src) in sprites {
        r.ppu.oam[o] = b39(rom, src);
    }
    let (bits, first, p97) = if kind == MapKind::Wily { (aa >> 4, 4usize, 0x54usize) } else { (aa & 0x0F, 0usize, 0x50usize) };
    let (p97, y2) = add_lists(&mut r, rom, bits, p97, first);
    blink(&mut r, rom, aa, 0xB4, &mut p95);
    add_next(&mut r, rom, aa, y2, p97, &mut p95);
    blink(&mut r, rom, aa, if kind == MapKind::CossackFirst { 0xF0 } else { 0xB4 }, &mut p95);
    crate::scene::set_last_p95(p95);
    r.frames
}

/// Compare with a `ppu_frames.lua` recording: the map scene (`$22 = $12`) starts at its first frame.
pub fn verify(rom: &Rom, path: &str, which: &str) -> bool {
    use crate::pfrec;
    let frames = match pfrec::load(path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("{}", e);
            return false;
        }
    };
    let start = match frames.iter().find(|f| f.r("scene") == 0x12) {
        Some(f) => f.n,
        None => {
            eprintln!("{}: no scene $12 found", path);
            return false;
        }
    };
    let first = frames.iter().find(|f| f.n == start).unwrap();
    let aa = first.r("aa") as u8;
    let kind = match which {
        "after" => MapKind::CossackAfter,
        "wily" => MapKind::Wily,
        _ => MapKind::CossackFirst,
    };
    let mine = map_scene(rom, kind, aa, first.r("p92") as u8, first.r("p95") as u8);
    let (mut shown, mut bad, mut checked) = (0, 0, 0);
    for (t, ours) in mine.iter().enumerate() {
        let f = match frames.iter().find(|f| f.n == start + t as u32) {
            Some(f) => f,
            None => break,
        };
        let vis = pfrec::visible(ours);
        let d = pfrec::compare(f, ours, vis, vis);
        checked += 1;
        if !pfrec::report(path, f, t as u32, &d, &mut shown) {
            bad += 1;
        }
    }
    println!("{}: {} frames compared, {} differ", path, checked, bad);
    bad == 0
}
