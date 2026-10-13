//! The robot master's introduction before a stage (bank `$39` `code_398F93` .. `code_3990C6`): striped background (scene `$10` screen 2, graphics set `$18 | stage`),
//! the boss poses (object type `$81`, animation `$44 + stage`), the background palette cycles (the palette animation task, ids `$92 + 3 * stage ..`),
//! the sprite palettes fade in, the name is typed, 240 frames later the stage loads. Checked frame by frame against Mesen (`mm4 --verify-bossintro`, `harness/pfi_*.txt`).
use crate::gfx;
use crate::rom::Rom;
use crate::scene::SceneRunner;
use crate::screen::Screen;
use crate::stage::Stage;
use crate::weapon_get::type_text;

fn b39(rom: &Rom, a: u16) -> u8 {
    rom.cpu_byte(0x39, 0x3A, a)
}

/// Run the scene from its first frame (`$22 = $10`, the column builder) up to the end of the 240-frame wait.
pub fn run(rom: &Rom, stage: usize, p92: u8, p95: u8) -> Vec<Screen> {
    let scene = Stage::load(rom, 0x10, 5);
    let mut s = Screen::new();
    let (tiles, attrs) = scene.nametable(2);
    s.nt[..960].copy_from_slice(&tiles);
    s.nt[960..].copy_from_slice(&attrs);
    gfx::load_set(rom, 0x55, &mut s.chr);
    gfx::load_set(rom, 0x10, &mut s.chr);
    gfx::load_set(rom, 0x18 | stage, &mut s.chr);
    s.pal = [0x0F; 32];
    let mut r = SceneRunner::new(s, p92, p95);
    r.rom = Some(rom.clone());
    r.wait(32);
    for i in 0..16 {
        r.target[i] = b39(rom, 0x9596 + stage as u16 * 16 + i as u16);
        r.target[16 + i] = 0x0F;
    }
    // the palette animations of this stage's background
    for k in 0..3usize {
        r.anim_id[1 + k] = (stage * 3 + 0x92 + k) as u8;
        r.anim_idx[1 + k] = 0;
        r.anim_tick[1 + k] = 0;
    }
    r.objs.o[0].ty = 0x81;
    r.objs.set_anim(0, 0x44 + stage as u8);
    r.objs.o[0].flags = 0x90;
    r.objs.o[0].xpg = 0;
    r.objs.o[0].ypg = 0;
    r.objs.o[0].ypx = 0x74;
    r.objs.o[0].xpx = 0x80;
    if stage == 4 {
        // code_3FF452: a second object at the first one's place, animation $0F, then moved
        let o0 = r.objs.o[0];
        let o1 = &mut r.objs.o[1];
        *o1 = o0;
        o1.flags = (o0.flags & 0x40) | 0x90;
        o1.aid = 0x0F;
        o1.aframe = 0;
        o1.atick = 0;
        o1.xpx = 0x7F;
        o1.ypx = 0x63;
        o1.ty = 0x81;
    }
    r.draw_objects(rom);
    r.p9a = 0;
    r.snd_music(0x0F, 0x9055); // code_399052
    r.fade_in();
    // the sprite palettes fade in: four steps of ten frames ($30, $20, $10, 0 subtracted; a borrow gives black)
    let mut level = 0x30u8;
    loop {
        for x in 0..8usize {
            let (v, borrow) = b39(rom, 0x9616 + x as u16).overflowing_sub(level);
            let v = if borrow { 0x0F } else { v };
            r.ram_pal[16 + x] = v;
            r.target[16 + x] = v;
            let (v, borrow) = b39(rom, 0x961E + stage as u16 * 8 + x as u16).overflowing_sub(level);
            let v = if borrow { 0x0F } else { v };
            r.ram_pal[24 + x] = v;
            r.target[24 + x] = v;
        }
        r.f18 = true;
        r.wait(0x0A);
        if level == 0 {
            break;
        }
        level -= 0x10;
    }
    loop {
        r.draw_objects(rom);
        if r.objs.o[0].aframe == 5 {
            break;
        }
    }
    r.p9a = 0;
    type_text(&mut r, rom, stage + 11);
    r.wait(0xF0);
    r.frames
}

/// Compare with a `ppu_frames.lua` recording (`PF_START` before the select, `PF_MENU=1`): the scene starts at the second frame with `$22 = $10` after the select.
pub fn verify(rom: &Rom, path: &str, stage: usize) -> bool {
    use crate::pfrec;
    let frames = match pfrec::load(path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("{}", e);
            return false;
        }
    };
    // frame 32 of the scene (the first object draw) is where `$22` goes back from `$10` to the stage id after the column builder; stages 5 and 7 enter `$10` one frame
    // earlier than the others (the graphics load takes a frame longer), which is invisible, so the scene is aligned on that frame
    let mut left_select = false;
    let mut start = None;
    for w in frames.windows(2) {
        if w[0].r("scene") == 0x10 && w[1].r("scene") != 0x10 {
            if left_select {
                start = Some(w[1].n - 32);
                break;
            }
            left_select = true;
        }
    }
    let start = match start {
        Some(s) => s,
        None => {
            eprintln!("{}: no boss intro scene found", path);
            return false;
        }
    };
    let first = frames.iter().find(|f| f.n == start).unwrap();
    let mine = run(rom, stage, first.r("p92") as u8, first.r("p95") as u8);
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
