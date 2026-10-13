//! The opening story (bank `$1B`, entered from the title routine with `JSR $8000` -> `code_1B8E36`): CAPCOM's logo fades out, then pictures of the city and its robots with
//! text pages that fade in and out. Scene `$13` (screens `$2A` = 5, 0, 1, 2, 3, ...), graphics sets `$55`, `$56`, `$57`; Start skips it (`code_1B9251`).
//! The script is straight-line code driving `SceneRunner`; the helpers are the game's: `wait_obj` = `code_1B9332`, `type_page` = `code_1B92D0`, `print_text` = `code_1B9388`.
//! Checked frame by frame against Mesen (`mm4 --verify-story harness/pfo_1.txt`).
use crate::gfx;
use crate::rom::Rom;
use crate::scene::SceneRunner;
use crate::stage::Stage;

fn b1b(rom: &Rom, a: u16) -> u8 {
    rom.cpu_byte(0x1B, 0x1C, a)
}

/// `code_1B87F1` / `code_1B87FB`: scene `$13` screen `screen` with graphics set `set`: rendering off, graphics load (its extra frames), 32 column frames, scroll cleared.
fn scene_load(r: &mut SceneRunner, rom: &Rom, screen: usize, set: usize) {
    let st = Stage::load(rom, 0x13, 5);
    let (tiles, attrs) = st.nametable(screen);
    r.wait(gfx::load_frames(rom, set) as u32);
    gfx::load_set(rom, set, &mut r.ppu.chr);
    r.ppu.nt[..960].copy_from_slice(&tiles);
    r.ppu.nt[960..].copy_from_slice(&attrs);
    r.wait(32);
    r.fc = 0;
}

/// `code_1B8866` with `y`: the 32 palette bytes at `$8875 + y` become the fade target.
fn load_palette(r: &mut SceneRunner, rom: &Rom, y: u16) {
    for i in 0..32 {
        r.target[i] = b1b(rom, 0x8875 + y + i as u16);
    }
}

/// The start of a new picture: fade out, OAM cleared (`$97 = 4`, `code_3EC421`), objects removed (`code_3EC438`), one frame.
fn next_picture(r: &mut SceneRunner) {
    r.p9a = 0;
    r.fade_out();
    for i in 0..64 {
        r.ppu.oam[i * 4] = 0xF8;
    }
    r.objs.clear_all();
    r.yield_frame();
}

/// Start was pressed: the script jumps to its last lines (`code_1B9251`).
pub struct Skip;

/// `code_1B9251`: Start held ends the story.
fn skip(r: &SceneRunner) -> Result<(), Skip> {
    if r.held & 0x10 != 0 {
        Err(Skip)
    } else {
        Ok(())
    }
}

/// `code_1B9332`: `x` frames, each with one object pass; in the scroll modes (`$F0` >= 10) the picture slides; Start ends the wait.
fn wait_obj(r: &mut SceneRunner, rom: &Rom, x: u32) {
    for _ in 0..x {
        if r.objs.o[3].ty != 0 {
            letters(r, rom);
        }
        if r.f0 >= 0x0A {
            r.x69 = r.x69.wrapping_sub(0x10);
            if r.f0 == 0x0B {
                r.fc = r.fc.wrapping_add(8);
                let (v, b) = r.objs.o[0].xpx.overflowing_sub(8);
                r.objs.o[0].xpx = v;
                if b {
                    r.objs.o[0].ty = 0;
                }
                let (v, b) = r.objs.o[1].xpx.overflowing_sub(8);
                r.objs.o[1].xpx = v;
                if b {
                    r.objs.o[1].ty = 0;
                }
            }
        }
        r.draw_objects(rom);
        if r.held & 0x10 != 0 {
            break;
        }
    }
    r.p9a = 0;
}

/// `code_1B9279`: the two flying letters (objects 2 and 3) move right by 8; a timer holds each after it restarts; the sparkle colours alternate.
fn letters(r: &mut SceneRunner, rom: &Rom) {
    for x in 2..4usize {
        let mut move_it = true;
        if r.objs.o[x].s[0] != 0 {
            r.objs.o[x].s[0] -= 1;
            if r.objs.o[x].s[0] != 0 {
                move_it = false;
            } else {
                r.objs.set_anim(x, 0x0D);
            }
        }
        if move_it {
            let (v, c) = r.objs.o[x].xpx.overflowing_add(8);
            r.objs.o[x].xpx = v;
            if c {
                r.objs.o[x].xpx = 0;
                r.objs.set_anim(x, 0);
                r.objs.o[x].s[0] = 0x1E;
            }
        }
    }
    let y = ((r.objs.p95 >> 1) & 1) as u16 * 4;
    for k in 0..4u16 {
        r.ram_pal[0x12 + 4 * k as usize] = b1b(rom, 0x950B + y + k);
    }
    r.f18 = true;
}

/// `code_1B93F1`: the text colour (`$060D`) fades to black: five steps of ten frames.
fn fade_text_out(r: &mut SceneRunner, rom: &Rom) {
    let mut s = 0u8;
    r.target[0x0D] = 0x30;
    loop {
        let (v, b) = r.target[0x0D].overflowing_sub(s);
        r.ram_pal[0x0D] = if b { 0x0F } else { v };
        r.f18 = true;
        wait_obj(r, rom, 0x0A);
        if r.held & 0x10 != 0 {
            break;
        }
        s += 0x10;
        if s == 0x50 {
            break;
        }
    }
}

/// `code_1B93C2`: the text colour fades in again: five steps of ten frames.
fn fade_text_in(r: &mut SceneRunner, rom: &Rom) {
    let mut s = 0x40u8;
    r.target[0x0D] = 0x30;
    loop {
        let (v, b) = r.target[0x0D].overflowing_sub(s);
        r.ram_pal[0x0D] = if b { 0x0F } else { v };
        r.f18 = true;
        wait_obj(r, rom, 0x0A);
        if r.held & 0x10 != 0 || s == 0 {
            break;
        }
        s -= 0x10;
    }
}

/// `code_1B9388`: text `y` of the table `$97AC/$97B9` goes into the three-line record template `$942D` (line breaks `$FD`, end `$FF`) and is flushed next frame.
fn print_text(r: &mut SceneRunner, rom: &Rom, y: usize) {
    let ptr = b1b(rom, 0x97AC + y as u16) as u16 | (b1b(rom, 0x97B9 + y as u16) as u16) << 8;
    let mut buf: Vec<u8> = (0..=0x51u16).map(|i| b1b(rom, 0x942D + i)).collect();
    let (mut p, mut line, mut x) = (0u16, 0usize, 3usize);
    loop {
        let c = b1b(rom, ptr + p);
        if c == 0xFF {
            break;
        }
        p += 1;
        if c == 0xFD {
            x = b1b(rom, 0x9480 + line as u16) as usize;
            line += 1;
            continue;
        }
        buf[x] = c;
        x += 1;
    }
    r.set_buffer(&buf);
}

/// `code_1B92D0`: a text page: the old text fades out, 60 frames, the new text is printed and fades in, then `x` frames.
fn type_page(r: &mut SceneRunner, rom: &Rom, y: usize, x: u32) {
    fade_text_out(r, rom);
    wait_obj(r, rom, 0x3C);
    if r.held & 0x10 != 0 {
        return;
    }
    print_text(r, rom, y);
    fade_text_in(r, rom);
    wait_obj(r, rom, x);
}

/// `code_1B92F0`: object `x` appears (type `$60`, animation `$9482 + x`, position from `$9485` / `$9488`).
fn init_obj(r: &mut SceneRunner, rom: &Rom, x: usize) {
    r.objs.o[x].ty = 0x60;
    r.objs.o[x].flags = 0x80;
    r.objs.set_anim(x, b1b(rom, 0x9482 + x as u16));
    r.objs.o[x].ypx = b1b(rom, 0x9485 + x as u16);
    r.objs.o[x].xpx = b1b(rom, 0x9488 + x as u16);
    r.objs.o[x].ypg = 0;
    r.objs.o[x].xpg = 0;
    r.snd(0x37, 0x8EE8 + 13 * x as u16); // code_1B9316: `JMP code_3FF6BE`, so the request returns to the caller (the JSRs at $8EE5, $8EF2, $8EFF)
}

/// `code_1B9319`: four times 20 frames.

fn wait_four(r: &mut SceneRunner, rom: &Rom) {
    for _ in 0..4 {
        r.snd(0x37, 0x9322); // code_1B931F
        if r.held & 0x10 != 0 {
            return;
        }
        wait_obj(r, rom, 0x14);

    }
}

/// Wait until object `x`'s animation frame index reaches `frame` (one object frame at a time, no Start check).
fn wait_frame(r: &mut SceneRunner, rom: &Rom, x: usize, frame: u8) {
    loop {
        wait_obj(r, rom, 1);
        if r.objs.o[x].aframe == frame {
            break;
        }
    }
}

/// Like `wait_frame`, with the Start check after every frame (`code_1B9251`).
fn wait_frame_skip(r: &mut SceneRunner, rom: &Rom, x: usize, frame: u8) -> Result<(), Skip> {
    loop {
        wait_obj(r, rom, 1);
        skip(r)?;
        if r.objs.o[x].aframe == frame {
            return Ok(());
        }
    }
}

/// `wait_obj` followed by the Start check.
fn wait_skip(r: &mut SceneRunner, rom: &Rom, x: u32) -> Result<(), Skip> {
    wait_obj(r, rom, x);
    skip(r)
}

/// `code_1B8E36`: the whole story. On Start the script jumps to its last lines.
pub fn play(r: &mut SceneRunner, rom: &Rom) {
    r.rom = Some(rom.clone());
    if script(r, rom).is_ok() {
        r.fade_out(); // code_1B922E (a skip jumps past it)
    }
    // code_1B9231: the scroll mode and the palette animations end, 60 more frames (a held Start ends them at once)
    r.f0 = 0;
    r.anim_id = [0; 4];
    for i in 0..32 {
        r.target[i] = r.ram_pal[i];
    }
    wait_obj(r, rom, 0x3C);
}

fn script(r: &mut SceneRunner, rom: &Rom) -> Result<(), Skip> {
    next_picture(r);
    scene_load(r, rom, 5, 0x55);
    load_palette(r, rom, 0x60);
    let rec: Vec<u8> = (0..=10u16).map(|i| b1b(rom, 0x9422 + i)).collect();
    r.set_buffer(&rec);
    r.yield_frame();
    wait_skip(r, rom, 0x78)?;
    r.snd_music(0x0A, 0x8E75); // code_1B8E72
    r.fade_in();
    wait_skip(r, rom, 0x78)?;
    r.fade_out();
    r.yield_frame();
    scene_load(r, rom, 0, 0x55);
    load_palette(r, rom, 0x80);
    r.yield_frame();
    print_text(r, rom, 0);
    r.fade_in();
    wait_skip(r, rom, 0x3C)?;
    fade_text_in(r, rom);
    wait_skip(r, rom, 0xB4)?;
    type_page(r, rom, 1, 0x3C);
    for (k, v) in [0xBAu8, 0xB9, 0xB8, 0xB7].iter().enumerate() {
        r.anim_id[3 - k] = *v;
        r.anim_idx[3 - k] = 0;
        r.anim_tick[3 - k] = 0;
    }
    wait_skip(r, rom, 0x78)?;
    type_page(r, rom, 2, 0xB4);
    type_page(r, rom, 3, 0x3C);
    init_obj(r, rom, 0);
    wait_skip(r, rom, 0x28)?;
    init_obj(r, rom, 1);
    wait_skip(r, rom, 0x28)?;
    init_obj(r, rom, 2);
    wait_four(r, rom);
    type_page(r, rom, 4, 0x14);
    for _ in 0..3 {
        wait_four(r, rom);
    }
    type_page(r, rom, 5, 0x14);
    for _ in 0..3 {
        wait_four(r, rom);
    }
    picture_two(r, rom)?;
    picture_three(r, rom)?;
    picture_four(r, rom)
}

/// The second picture (screen 1, set `$56`): the pages 6 and 7 and the flashing palette.
fn picture_two(r: &mut SceneRunner, rom: &Rom) -> Result<(), Skip> {
    next_picture(r);
    scene_load(r, rom, 1, 0x56);
    load_palette(r, rom, 0xA0);
    for x in 0..2usize {
        r.objs.o[x].ty = 0x60;
        r.objs.o[x].flags = 0x80;
        r.objs.o[x].ypg = 0;
        r.objs.o[x].xpg = 0;
        r.objs.o[x].xpx = 0x80;
    }
    r.objs.set_anim(0, 0x33);
    r.objs.o[0].ypx = 0x47;
    r.objs.set_anim(1, 0x07);
    r.objs.o[1].ypx = 0x77;
    r.draw_objects(rom);
    r.p9a = 0;
    wait_skip(r, rom, 0x60)?;
    r.fade_in();
    type_page(r, rom, 6, 0xB4);
    type_page(r, rom, 7, 0x14);
    r.objs.set_anim(0, 0x08);
    wait_frame(r, rom, 0, 7);
    r.objs.set_anim(0, 0x38);
    let (mut p8, mut n9) = (0u16, 0u8);
    loop {
        let mut y = p8;
        for x in 0..32usize {
            r.ram_pal[x] = b1b(rom, 0x948B + y);
            y += 1;
        }
        r.f18 = true;
        if y == 0x60 {
            y = 0;
        }
        p8 = y;
        wait_skip(r, rom, 1)?;
        n9 = n9.wrapping_sub(1);
        if n9 == 0 {
            return Ok(());
        }
    }
}

/// The third picture (screen 2, set `$56`): the boss portrait and its colour cycle.
fn picture_three(r: &mut SceneRunner, rom: &Rom) -> Result<(), Skip> {
    next_picture(r);
    scene_load(r, rom, 2, 0x56);
    load_palette(r, rom, 0xC0);
    r.objs.o[0].ty = 0x60;
    r.objs.o[0].flags = 0x80;
    r.objs.set_anim(0, 0x34);
    r.objs.o[0].ypx = 0x8B;
    r.objs.o[0].xpx = 0x80;
    r.objs.o[0].ypg = 0;
    r.objs.o[0].xpg = 0;
    r.draw_objects(rom);
    r.p9a = 0;
    wait_skip(r, rom, 0x78)?;
    r.fade_in();
    wait_skip(r, rom, 0xF0)?;
    r.objs.set_anim(0, 0x09);
    wait_frame_skip(r, rom, 0, 0x21)?;
    r.objs.set_anim(0, 0x37);
    wait_skip(r, rom, 0x28)?;
    let mut y = 0u16;
    loop {
        for x in 0..8usize {
            let v = b1b(rom, 0x94EB + y);
            r.ram_pal[x] = v;
            r.target[x] = v;
            r.ram_pal[0x10 + x] = v;
            r.target[0x10 + x] = v;
            y += 1;
        }
        r.f18 = true;
        wait_skip(r, rom, 0x0A)?;
        if y == 0x20 {
            return Ok(());
        }
    }
}

/// The last picture (screen 3, set `$57`): Rock and Roll, Rush, the sparkles; ends with the picture sliding away.
fn picture_four(r: &mut SceneRunner, rom: &Rom) -> Result<(), Skip> {
    r.fade_out();
    for i in 0..64 {
        r.ppu.oam[i * 4] = 0xF8;
    }
    r.objs.clear_all();
    r.yield_frame();
    wait_skip(r, rom, 0x3C)?;
    wait_skip(r, rom, 0x78)?;
    scene_load(r, rom, 3, 0x57);
    load_palette(r, rom, 0xE0);
    for x in 0..4usize {
        r.objs.o[x].ty = 0x60;
        r.objs.o[x].flags = 0x80;
        r.objs.o[x].ypg = 0;
        r.objs.o[x].xpg = 0;
    }
    r.objs.set_anim(0, 0x01);
    r.objs.o[0].ypx = 0x4F;
    r.objs.o[0].xpx = 0x7C;
    r.objs.set_anim(1, 0x05);
    r.objs.o[1].ypx = 0x7F;
    r.objs.o[1].xpx = 0x80;
    r.objs.set_anim(2, 0x0D);
    r.objs.o[2].xpx = 0x40;
    r.objs.set_anim(3, 0x0D);
    r.objs.o[3].xpx = 0xC0;
    r.objs.o[2].ypx = 0x2B;
    r.objs.o[3].ypx = 0x2B;
    r.objs.o[2].s[0] = 0;
    r.objs.o[3].s[0] = 0;
    r.draw_objects(rom);
    r.p9a = 0;
    r.fade_in();
    r.snd_music(0x43, 0x914D); // code_1B914A
    type_page(r, rom, 8, 0xB4);
    type_page(r, rom, 9, 0xB4);
    r.anim_id[0] = 0xBB;
    r.anim_id[1] = 0xBC;
    r.anim_id[3] = 0xBD;
    for k in 0..4 {
        r.anim_idx[k] = 0;
        r.anim_tick[k] = 0;
    }
    r.objs.o[2].ty = 0;
    r.objs.o[3].ty = 0;
    for k in 0..4u16 {
        r.ram_pal[0x12 + 4 * k as usize] = b1b(rom, 0x950B + k);
    }
    r.f18 = true;
    r.objs.set_anim(0, 0x02);
    r.f1 = 0x80;
    r.f0 = 0x0A;
    loop {
        wait_skip(r, rom, 1)?;
        if r.anim_id[0] & 0x80 == 0 {
            break;
        }
    }
    r.objs.set_anim(0, 0x03);
    type_page(r, rom, 10, 0xB4);
    type_page(r, rom, 11, 0xB4);
    type_page(r, rom, 12, 0xB4);
    wait_skip(r, rom, 0x3C)?;
    r.objs.set_anim(0, 0x04);
    wait_frame_skip(r, rom, 0, 7)?;
    r.objs.set_anim(0, 0x35);
    r.objs.set_anim(1, 0x06);
    wait_frame_skip(r, rom, 1, 2)?;
    r.objs.set_anim(1, 0x36);
    wait_skip(r, rom, 0x78)?;
    r.snd_music(0xF6, 0x9225); // code_1B9222
    r.f0 = 0x0B;
    wait_obj(r, rom, 0xB4);
    Ok(())
}

/// Compare with a boot recording (`ppu_frames.lua`, no input): copyright, CAPCOM and then the story, from the first frame with `$22 = $10` (frame 6).
pub fn verify(rom: &Rom, path: &str) -> bool {
    use crate::pfrec;
    let frames = match pfrec::load(path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("{}", e);
            return false;
        }
    };
    let start = match frames.iter().find(|f| f.r("scene") == 0x10) {
        Some(f) => f.n,
        None => return false,
    };
    let mut r = crate::boot_screens::boot_runner(rom);
    play(&mut r, rom);
    let mine = r.frames;
    if std::env::var("MM4_PAL13").is_ok() {
        let mut prev = 0xFFu8;
        let mut out = Vec::new();
        for (t, s) in mine.iter().enumerate() {
            if s.pal[13] != prev {
                out.push(format!("{}:{:x}", t, s.pal[13]));
                prev = s.pal[13];
            }
        }
        println!("{}", out.join(" "));
    }
    let (mut shown, mut bad, mut checked) = (0, 0, 0);
    for (t, ours) in mine.iter().enumerate() {
        if t == 0 {
            continue;
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::LiveScene;
    use crate::screen::Screen;

    /// The live (threaded) run and the recording run produce the same pictures; Start ends the story early.
    #[test]
    fn live_equals_recorded_and_start_skips() {
        let rom = match Rom::load("../../Mega Man 4 (USA).nes") {
            Ok(r) => r,
            Err(_) => return,
        };
        let mut rec = crate::boot_screens::boot_runner(&rom);
        play(&mut rec, &rom);
        let rom2 = rom.clone();
        let mut blank = Screen::new();
        blank.pal = [0x0F; 32];
        let mut live = LiveScene::start(crate::scene::SceneRunner::new(blank, 1, 0), move |r| {
            crate::boot_screens::boot(r, &rom2);
            play(r, &rom2);
        });
        for t in 0..1200 {
            let s = live.tick(0).expect("live scene ended early");
            let q = &rec.frames[t];
            assert!(s.pal == q.pal && s.oam == q.oam && s.nt[..] == q.nt[..], "frame {} differs", t);
        }
        // a second live run: Start from frame 800 on (the story proper begins after the 2 x 120 + ~100 frames of the logos)
        let rom3 = rom.clone();
        let mut blank = Screen::new();
        blank.pal = [0x0F; 32];
        let mut live2 = LiveScene::start(crate::scene::SceneRunner::new(blank, 1, 0), move |r| {
            crate::boot_screens::boot(r, &rom3);
            play(r, &rom3);
        });
        let mut n = 0;
        loop {
            let held = if n >= 800 { 0x10 } else { 0 };
            if live2.tick(held).is_none() {
                break;
            }
            n += 1;
            assert!(n < 3000, "the story did not end after Start");
        }
        assert!(n > 800 && n < 1200, "ended after {} frames", n);
    }
}
