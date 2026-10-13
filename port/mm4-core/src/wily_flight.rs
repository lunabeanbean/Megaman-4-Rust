//! The cutscene after Dr. Cossack's last stage (bank `$39` `code_3990C7` .. `code_399280`): a starfield (scene `$12` screen 1, set `$40`) scrolls faster and faster while
//! the saucer rises, brakes, a second object climbs and falls back, and the saucer's last animation ends; then Wily's fortress map follows (`fortress.rs`).
//! Checked frame by frame against Mesen (`mm4 --verify-wilyflight harness/pfm_4.txt`).
use crate::gfx;
use crate::rom::Rom;
use crate::scene::SceneRunner;
use crate::screen::Screen;
use crate::stage::Stage;

fn b39(rom: &Rom, a: u16) -> u8 {
    rom.cpu_byte(0x39, 0x3A, a)
}

/// `code_399254`: the five star sprites move left by twice the speed's high part.
fn shift_stars(r: &mut SceneRunner, v47: u8, v48: u8) {
    let w = ((v48 as u16) << 8 | v47 as u16) << 1;
    let hi = (w >> 8) as u8;
    for k in 0..6usize {
        let o = &mut r.ppu.oam[k * 4 + 3];
        *o = o.wrapping_sub(hi);
    }
}

/// `code_399272`: `n` frames with the objects drawn from OAM byte `$1C`.
fn wait_objects(r: &mut SceneRunner, rom: &Rom, n: u32) {
    for _ in 0..n {
        r.draw_objects_at(rom, 0x1C);
    }
}

/// From the first frame of scene `$12` (`$22 = $12`, column builder) to the jump to Wily's map.
pub fn run(rom: &Rom, p92: u8, p95: u8) -> Vec<Screen> {
    let scene = Stage::load(rom, 0x12, 5);
    let mut s = Screen::new();
    let (tiles, attrs) = scene.nametable(1);
    s.nt[..960].copy_from_slice(&tiles);
    s.nt[960..].copy_from_slice(&attrs);
    gfx::load_set(rom, 0x55, &mut s.chr);
    gfx::load_set(rom, 0x10, &mut s.chr);
    gfx::load_set(rom, 0x40, &mut s.chr);
    s.pal = [0x0F; 32];
    let mut r = SceneRunner::new(s, p92, p95);
    r.wait(33); // (one frame more than the others: the column builder of this screen)
    for i in 0..32 {
        r.target[i] = b39(rom, 0x9391 + i as u16);
    }
    r.objs.o[0].ty = 1;
    r.objs.set_anim(0, 0x8A);
    r.objs.o[0].flags = 0x80;
    r.objs.o[0].xpx = 0x80;
    r.objs.o[0].xpg = 0;
    r.objs.o[0].ypg = 0;
    r.objs.o[0].ypx = 0xF0;
    for i in 0..0x14usize {
        r.ppu.oam[i] = b39(rom, 0x965E + i as u16);
    }
    r.yield_frame();
    r.snd_music(0xF0, 0x913F); // code_39913C
    r.fade_in();
    let (mut v45, mut v46, mut v47, mut v48, mut v49) = (0u8, 0u8, 0u8, 0u8, 0u8);
    let mut fc = 0u8;
    // the saucer rises from y $F0 to $80, one pixel every other frame
    loop {
        if r.objs.p95 & 1 != 0 {
            r.objs.o[0].ypx = r.objs.o[0].ypx.wrapping_sub(1);
            if r.objs.o[0].ypx & 7 == 0 {
                r.snd(0x40, 0x9163); // code_399160
            }
        }
        r.fc = fc;
        r.draw_objects_at(rom, 0x14);
        if r.objs.o[0].ypx == 0x80 {
            break;
        }
    }
    // the starfield accelerates for 256 frames ...
    loop {
        if v48 != 4 {
            v46 = v46.wrapping_add(1);
            let (n, c) = v47.overflowing_add(v46);
            v47 = n;
            v48 = v48.wrapping_add(c as u8);
        }
        let (n, c) = v49.overflowing_add(v47);
        v49 = n;
        fc = fc.wrapping_add(v48).wrapping_add(c as u8);
        shift_stars(&mut r, v47, v48);
        if r.objs.p95 & 0x0F == 0 {
            r.snd(0x40, 0x91A1); // code_39919E
        }
        r.fc = fc;
        r.draw_objects_at(rom, 0x14);
        v45 = v45.wrapping_sub(1);
        if v45 == 0 {
            break;
        }
    }
    // ... and brakes again
    loop {
        let (n, b) = v47.overflowing_sub(v46);
        v47 = n;
        let (n2, b2) = v48.overflowing_sub(b as u8);
        v48 = n2;
        if b2 {
            break;
        }
        v46 = v46.wrapping_sub(1);
        let (n, c) = v49.overflowing_add(v47);
        v49 = n;
        fc = fc.wrapping_add(v48).wrapping_add(c as u8);
        shift_stars(&mut r, v47, v48);
        if r.objs.p95 & 0x0F == 0 {
            r.snd(0x40, 0x91D8); // code_3991D5
        }
        r.fc = fc;
        r.draw_objects_at(rom, 0x14);
    }
    // the saucer opens: a second object (copy of the first, animation $8B) climbs to y $50, waits 180 frames, falls back
    r.objs.set_anim(0, 0x8C);
    {
        let o0 = r.objs.o[0];
        let o1 = &mut r.objs.o[1];
        *o1 = o0;
        o1.flags = (o0.flags & 0x40) | 0x90;
        o1.aid = 0x8B;
        o1.aframe = 0;
        o1.atick = 0;
        o1.flags = 0x80;
        o1.ty = 1;
    }
    loop {
        r.objs.o[1].ypx = r.objs.o[1].ypx.wrapping_sub(1);
        r.draw_objects_at(rom, 0x14);
        r.objs.p95 = 0;
        if r.objs.o[1].ypx == 0x50 {
            break;
        }
    }
    wait_objects(&mut r, rom, 0xB4);
    loop {
        r.objs.o[1].ypx = r.objs.o[1].ypx.wrapping_add(1);
        r.draw_objects_at(rom, 0x14);
        r.objs.p95 = 0;
        if r.objs.o[1].ypx == 0x80 {
            break;
        }
    }
    r.objs.o[1] = Default::default();
    r.objs.set_anim(0, 0x8A);
    wait_objects(&mut r, rom, 0x3C);
    r.objs.set_anim(0, 0x89);
    loop {
        r.draw_objects_at(rom, 0x14);
        if r.objs.o[0].ty == 0 {
            break;
        }
    }
    r.p9a = 0;
    r.frames
}

/// The cutscene, the fade-out of its last picture and Wily's fortress map (`code_3983E8`, `aa` = `$AA` after the clear).
pub fn chain(rom: &Rom, aa: u8, p92: u8, p95: u8) -> Vec<Screen> {
    let mut v = run(rom, p92, p95);
    let last = v.last().cloned().unwrap_or_else(Screen::new);
    v.extend(crate::scene::fade_out_screens(&last, 1, 5));
    let p = p92.wrapping_add(v.len() as u8);
    v.extend(crate::fortress::map_scene(rom, crate::fortress::MapKind::Wily, aa, p, 0));
    v
}

/// Compare with a `ppu_frames.lua` recording of the stage clear of stage 11 (`PF_POKE=22=0B,AA=07`): scene `$12` starts at its first frame.
pub fn verify(rom: &Rom, path: &str) -> bool {
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
    let mine = chain(rom, first.r("aa") as u8 | 0x08, first.r("p92") as u8, first.r("p95") as u8);
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

