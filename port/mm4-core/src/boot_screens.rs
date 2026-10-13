//! The first two screens after power-on (bank `$39` `code_39800C`): the copyright notice and "CAPCOM PRESENTS", each entered through the scene loader
//! `code_398495` (fade out, graphics, 32 frames of column building, fade in) and held for 120 frames. Checked against Mesen (`mm4 --verify-boot harness/pft_0.txt`).
use crate::gfx;
use crate::rom::Rom;
use crate::scene::SceneRunner;
use crate::screen::Screen;
use crate::stage::Stage;

fn b39(rom: &Rom, a: u16) -> u8 {
    rom.cpu_byte(0x39, 0x3A, a)
}

/// `code_398495` for menu scene index `idx` (`$10`) and screen `screen` (`$2A`): returns after the fade-in.
pub fn enter_scene(r: &mut SceneRunner, rom: &Rom, idx: u8, screen: u8) {
    r.fade_out();
    for i in 0..64 {
        r.ppu.oam[i * 4] = 0xF8;
    }
    r.yield_frame();
    let scene_id = b39(rom, 0x884E + idx as u16) as usize;
    let set = if screen == 4 { 0x55 } else { scene_id };
    r.wait(gfx::load_frames(rom, set) as u32);
    gfx::load_set(rom, set, &mut r.ppu.chr);
    let st = Stage::load(rom, scene_id, 5);
    let (tiles, attrs) = st.nametable(screen as usize);
    r.ppu.nt[..960].copy_from_slice(&tiles);
    r.ppu.nt[960..].copy_from_slice(&attrs);
    r.wait(32);
    for i in 0..32 {
        r.target[i] = b39(rom, 0x872E + idx as u16 * 32 + i as u16);
    }
    if idx != 0 {
        let ptr = b39(rom, 0x8856 + idx as u16) as u16 | (b39(rom, 0x885A + idx as u16) as u16) << 8;
        let mut n = 0;
        while b39(rom, ptr + n) != 0xFF {
            r.ppu.oam[n as usize] = b39(rom, ptr + n);
            n += 1;
        }
    }
    r.snd_music(b39(rom, 0x8879 + idx as u16), 0x858D); // code_398585: the scene's music (table $8879)
    r.yield_frame();
    if scene_id == 0x10 {

        r.fade_in();
    }
}

/// The runner after the copyright notice and the "CAPCOM PRESENTS" hold (from the first frame with `$22 = $10`, frame 6 after reset): the opening story continues on it.
pub fn boot_runner(rom: &Rom) -> SceneRunner {
    let mut s = Screen::new();
    s.pal = [0x0F; 32];
    let mut r = SceneRunner::new(s, 1, 0);
    boot(&mut r, rom);
    r
}

/// The two screens themselves (on a runner that is recording or live).
pub fn boot(r: &mut SceneRunner, rom: &Rom) {
    enter_scene(r, rom, 3, 4);
    r.wait(0x78);
    enter_scene(r, rom, 0, 1);
    r.wait(0x78);
}

/// From the first frame with `$22 = $10` (frame 6 after reset) to the end of the "CAPCOM PRESENTS" hold.
pub fn run(rom: &Rom) -> Vec<Screen> {
    boot_runner(rom).frames
}

/// Compare with a boot recording (`ppu_frames.lua`, no input): frame 6 is the first frame with `$22 = $10`.
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
    let mine = run(rom);
    let (mut shown, mut bad, mut checked) = (0, 0, 0);
    for (t, ours) in mine.iter().enumerate() {
        if t == 0 {
            continue; // the very first frame still shows the power-on palette
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
