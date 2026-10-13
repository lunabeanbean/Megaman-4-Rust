//! The weapon-get cutscene after a robot master (bank `$39` `code_398CBC` .. `code_398E75`): Mega Man's big face slides to the left, "YOU GOT" and the weapon's
//! name are typed letter by letter, the palette flashes in the weapon's colours; after stage 1 and 2 Rush's own screen follows.
//! Scene `$11` screen 0 with the grid floor, graphics set `$16`, scene object type 1 with animation `$70`. Checked frame by frame against Mesen recordings
//! (`mm4 --verify-weaponget`, `harness/pfw_*.txt`).
use crate::gfx;
use crate::rom::Rom;
use crate::scene::SceneRunner;
use crate::screen::Screen;
use crate::stage::Stage;

fn b39(rom: &Rom, a: u16) -> u8 {
    rom.cpu_byte(0x39, 0x3A, a)
}

/// `code_399281`: type string `y` of the table `$94D9/$94F1`, one tile at a time, ten frames per letter.
pub(crate) fn type_text(r: &mut SceneRunner, rom: &Rom, y: usize) {
    let ptr = b39(rom, 0x94D9 + y as u16) as u16 | (b39(rom, 0x94F1 + y as u16) as u16) << 8;
    let (hi, mut lo) = (b39(rom, ptr), b39(rom, ptr + 1));
    let mut i = 2;
    loop {
        let c = b39(rom, ptr + i);
        if c == 0xFF {
            return;
        }
        r.set_buffer(&[hi, lo, 0, c, 0xFF]);
        if !(0x0B..0x13).contains(&y) {
            r.snd(0x35, 0x92BD); // code_3992BA (the robot masters' names, strings 11..18, are typed silently)
        }
        r.wait(0x0A);

        lo = lo.wrapping_add(1);
        i += 1;
    }
}

/// Run the scene from the first frame with `$22 = $11` for stage `stage` (0-7), `p92`/`p95` = `$92` and `$95` at that moment. Returns one snapshot per video frame
/// up to the frame before the next scene (the password screen) starts.
pub fn run(rom: &Rom, stage: usize, p92: u8, p95: u8) -> Vec<Screen> {
    let scene = Stage::load(rom, 0x11, 5);
    let mut s = Screen::new();
    let (tiles, attrs) = scene.nametable(0);
    s.nt[..960].copy_from_slice(&tiles);
    s.nt[960..].copy_from_slice(&attrs);
    gfx::load_set(rom, 0x55, &mut s.chr);
    gfx::load_set(rom, 0x16, &mut s.chr);
    s.pal = [0x0F; 32];
    let mut r = SceneRunner::new(s, p92, p95);
    // the 32 frames of the column builder (black palette)
    r.wait(32);
    for i in 0..32 {
        r.target[i] = b39(rom, 0x92DF + i as u16);
    }
    r.objs.o[0].ty = 1;
    r.objs.set_anim(0, 0x70);
    r.objs.o[0].flags = 0x90;
    r.objs.o[0].xpg = 0;
    r.objs.o[0].ypg = 0;
    r.objs.o[0].ypx = 0x70;
    r.objs.o[0].xpx = 0x80;
    r.draw_objects(rom);
    r.yield_frame();
    r.snd_music(0x14, 0x8D40); // code_398D3D
    r.fade_in();
    r.p9a = r.p9a.wrapping_add(1);

    // Mega Man's face animates until its frame index is 8
    loop {
        r.draw_objects(rom);
        if r.objs.o[0].aframe == 8 {
            break;
        }
    }
    r.wait(0x1E);
    // it slides left to x = $48 (the tick is reset every frame: the animation stands still)
    loop {
        r.objs.o[0].xpx = r.objs.o[0].xpx.wrapping_sub(1);
        r.draw_objects(rom);
        r.objs.o[0].atick = 0;
        if r.objs.o[0].xpx == 0x48 {
            break;
        }
    }
    r.p9a = 0;
    type_text(&mut r, rom, 0);
    r.wait(0x0A);
    type_text(&mut r, rom, stage + 1);
    // the palette flashes in the weapon's colours: `$95` counts down from $3C (it is also the draw pass counter)
    r.objs.p95 = 0x3C;
    loop {
        let y = if (r.objs.p95 >> 1) & 1 != 0 { 0 } else { (stage as u16 + 1) * 16 };
        for x in 0..16u16 {
            let v = b39(rom, 0x92EF + y + x);
            r.ram_pal[16 + x as usize] = v;
            r.target[16 + x as usize] = v;
        }
        r.f18 = true;
        r.yield_frame();
        r.objs.p95 = r.objs.p95.wrapping_sub(1);
        if r.objs.p95 == 0 {
            break;
        }
    }
    r.wait(0x3C);
    if stage == 1 || stage == 2 {
        rush(&mut r, rom, stage);
    }
    // code_398E78: wait, then the password screen is prepared (code_398F49: fade out, OAM cleared, three more frames with the screen off)
    r.wait(0x3C);
    r.fade_out();
    for i in 0..64 {
        r.ppu.oam[i * 4] = 0xF8;
    }
    r.wait(3);
    r.frames
}

/// `code_398DC5`: after stages 1 and 2 Rush's own screen: the face fades out (the background palettes 1-3 stay), the text frame appears, the dog slides in and
/// "YOU GOT / RUSH JET (or MARINE) / ADAPTOR" is typed.
fn rush(r: &mut SceneRunner, rom: &Rom, stage: usize) {
    r.fade_mask = 0x70;
    r.fade_out();
    for i in 0..64 {
        r.ppu.oam[i * 4] = 0xF8;
    }
    let mut buf = Vec::new();
    let mut p = 0x93B9u16;
    loop {
        let v = b39(rom, p);
        buf.push(v);
        if v == 0xFF {
            break;
        }
        p += 1;
    }
    r.set_buffer(&buf);
    r.yield_frame();
    r.objs.p95 = 0;
    for i in 0..4usize {
        r.ram_pal[3] = b39(rom, 0x938F + (i & 1) as u16);
        r.f18 = true;
        r.wait(0x0A);
    }
    r.wait(0x3C);
    r.objs.o[0].ty = 0x81;
    r.objs.set_anim(0, 0x36);
    r.objs.o[0].xpx = 0x80;
    for i in 0..16 {
        r.target[16 + i] = b39(rom, 0x937F + i as u16);
    }
    r.draw_objects(rom);
    r.p9a = 0;
    r.yield_frame();
    r.fade_mask = 0x70;
    r.fade_in();
    r.wait(0x1E);
    loop {
        r.objs.o[0].xpx = r.objs.o[0].xpx.wrapping_sub(1);
        r.draw_objects(rom);
        if r.objs.o[0].xpx == 0x48 {
            break;
        }
    }
    r.p9a = 0;
    type_text(r, rom, 0);
    r.wait(0x0A);
    type_text(r, rom, if stage == 2 { 9 } else { 10 });
    r.wait(0x0A);
    type_text(r, rom, 0x13);
    r.wait(0x3C);
}

/// Compare with a `ppu_frames.lua` recording of a stage clear (`PF_HIJACK`).
pub fn verify(rom: &Rom, path: &str, stage: usize) -> bool {
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
