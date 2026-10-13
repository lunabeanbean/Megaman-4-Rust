//! The ending (bank `$1B`, entered with `JSR $8003` after the last Wily machine: `code_1B8003`): the saucer over the castle, the scene with the second picture,
//! the credits on the train. Scene `$14`, graphics sets `$4B`, `$4C`, `$4D`, `$60`...; the script is straight-line code driving `SceneRunner`.
//! Checked frame by frame against Mesen (`mm4 --verify-ending harness/pfe_1.txt`, stage clear with `PF_STATE=19`).
use crate::gfx;
use crate::rom::Rom;
use crate::scene::{SObj, SceneRunner};
use crate::stage::Stage;

/// Gravity `$99` of the stage engine.
const GRAVITY: u8 = 0x40;

fn b1b(rom: &Rom, a: u16) -> u8 {
    rom.cpu_byte(0x1B, 0x1C, a)
}

/// `code_1B87F7` / `code_1B87FB`: scene `$14` screen `screen`, graphics set `set`: the graphics load, 32 column frames.
fn scene_load(r: &mut SceneRunner, rom: &Rom, scene: usize, screen: usize, set: usize) {
    let st = Stage::load(rom, scene, 5);
    let (tiles, attrs) = st.nametable(screen);
    r.wait(gfx::load_frames_from(rom, set, 5500) as u32);
    gfx::load_set(rom, set, &mut r.ppu.chr);
    r.ppu.nt[..960].copy_from_slice(&tiles);
    r.ppu.nt[960..].copy_from_slice(&attrs);
    r.wait(32);
    r.fc = 0;
}

/// `code_1B8866` with `y`.
fn load_palette(r: &mut SceneRunner, rom: &Rom, y: u16) {
    for i in 0..32 {
        r.target[i] = b1b(rom, 0x8875 + y + i as u16);
    }
}

/// `$97 = 4`, `code_3EC421`, `code_3EC438` (after a fade-out).
fn clear(r: &mut SceneRunner) {
    for i in 0..64 {
        r.ppu.oam[i * 4] = 0xF8;
    }
    r.objs.clear_all();
}

/// The block streamer of the second scene (`code_3FE35C`, one call per frame): `b1b` = `$1B`, `attr` = the attribute bytes collected in `$0680`.
struct Stream {
    b1b: u8,
    attr: [u8; 64],
}

impl Stream {
    /// `code_3FE35C` with `$1C` = `page`, `$2B` = `$80` (the second nametable): 16 tiles wide, 4 rows per call, then the attribute table.
    fn call(&mut self, r: &mut SceneRunner, st: &Stage, page: usize) {
        let hi = 0x24u8;
        if self.b1b == 0 {
            return;
        }
        if self.b1b == 0x40 {
            self.b1b = 0;
            let mut b = vec![hi | 3, 0xC0, 0x3F];
            b.extend_from_slice(&self.attr);
            b.push(0xFF);
            r.set_buffer(&b);
            self.attr = [0; 64];
            return;
        }
        self.b1b &= 0x7F;
        let idx = self.b1b as usize;
        // the address: the block row (idx >> 3 = 0..7) is four tile rows of 32 bytes
        let base = 0x2400u16 + ((idx >> 3) as u16) * 128 + ((idx & 7) as u16) * 4;
        let mut b = Vec::new();
        let mut rows: [Vec<u8>; 4] = Default::default();
        for k in 0..4usize {
            let (t, a) = st.block(page, idx + k);
            for row in 0..4 {
                rows[row].extend_from_slice(&t[row * 4..row * 4 + 4]);
            }
            self.attr[idx + k] = a;
        }
        for (row, data) in rows.iter().enumerate() {
            let addr = base + 32 * row as u16;
            b.push((addr >> 8) as u8);
            b.push(addr as u8);
            b.push(0x0F);
            b.extend_from_slice(data);
        }
        b.push(0xFF);
        r.set_buffer(&b);
        self.b1b += 4;
    }
}

/// `code_3FE286` for `$29` = `idx` (a block at a time while the picture scrolls): tiles into the nametable of the page's parity, its attribute byte, the last block row cut short.
fn build_block(r: &mut SceneRunner, st: &Stage, page: usize, idx: usize) {
    let (t, a) = st.block(page, idx);
    let nt = if page & 1 != 0 { 0x2400u16 } else { 0x2000 };
    let base = nt + ((idx >> 3) as u16) * 128 + ((idx & 7) as u16) * 4;
    let mut b = Vec::new();
    let rows = if idx & 0x3F >= 0x38 { 2 } else { 4 };
    for row in 0..rows {
        let addr = base + 32 * row as u16;
        b.push((addr >> 8) as u8);
        b.push(addr as u8);
        b.push(3);
        b.extend_from_slice(&t[row * 4..row * 4 + 4]);
    }
    b.push(((nt >> 8) as u8) | 3);
    b.push(0xC0 | idx as u8);
    b.push(0);
    b.push(a);
    b.push(0xFF);
    r.set_buffer(&b);
}

/// `code_1B87C9`: object 2 sways sideways with `$95`.
fn sway_x(r: &mut SceneRunner, rom: &Rom) {
    let y = ((r.objs.p95 >> 4) & 3) as u16;
    r.objs.o[2].xpx = r.objs.o[2].xpx.wrapping_add(b1b(rom, 0x8AB5 + y));
}

/// `code_1B87DD`: object 2 bobs up and down with `$95`.
fn sway_y(r: &mut SceneRunner, rom: &Rom) {
    let y = ((r.objs.p95 >> 4) & 3) as u16;
    r.objs.o[2].ypx = r.objs.o[2].ypx.wrapping_add(b1b(rom, 0x8AB5 + y));
}

/// Object slot 0 as the stage leaves it after the last Wily machine (type 1, the beam animation `$13`, in the middle of the screen); the scenes keep it.
pub fn mega_man_after_stage() -> SObj {
    let b: [u8; 24] = [0x01, 0xD0, 0x00, 0x80, 0x00, 0x00, 0x3D, 0xC0, 0x13, 0x00, 0x01, 0x00, 0x4C, 0x01, 0x40, 0xFA, 0, 0, 0, 0, 0, 0, 0, 0];
    SObj::from_dump(&b)
}

/// `mega`: object slot 0 as the stage left it (the scene keeps it).
pub fn play(r: &mut SceneRunner, rom: &Rom, mega: SObj) {
    r.rom = Some(rom.clone());
    r.objs.o[0] = mega;
    // the stage faded out (frames before the scene starts), then:
    clear(r);
    scene_load(r, rom, 0x14, 0, 0x4B);
    load_palette(r, rom, 0);
    r.objs.set_anim(1, 0x12);
    r.objs.o[1].ty = 0x60;
    r.objs.o[1].flags = 0x90;
    r.objs.o[1].xpg = 0;
    r.objs.o[1].ypg = 0;
    r.objs.o[1].ypx = 0x6F;
    r.objs.o[1].xpx = 0xC8;
    r.draw_objects(rom);
    r.p9a = 0;
    r.objs.p95 = 0;
    r.fade_in();
    // 180 frames of the castle (the animation of object 1 stands still: its tick is reset every frame)
    loop {
        r.draw_objects(rom);
        r.objs.o[1].atick = 0;
        if r.objs.p95 == 0xB4 {
            break;
        }
    }
    // a second object (animation $27) rises from y $60 to $20, swaying
    r.objs.copy_obj(1, 2, 0x27);
    r.objs.o[2].ty = 0x60;
    r.objs.o[2].ypx = 0x60;
    r.objs.p95 = 0;
    loop {
        if r.objs.p95 & 3 == 0 {
            sway_x(r, rom);
            r.objs.o[2].ypx = r.objs.o[2].ypx.wrapping_sub(1);
            if r.objs.o[2].ypx == 0x20 {
                break;
            }
        }
        r.draw_objects(rom);
    }
    loop {
        if r.objs.p95 & 3 == 0 {
            sway_y(r, rom);
            r.objs.o[2].xpx = r.objs.o[2].xpx.wrapping_sub(1);
            if r.objs.o[2].xpx == 0x88 {
                break;
            }
        }
        r.draw_objects(rom);
    }
    loop {
        if r.objs.p95 & 3 == 0 {
            sway_x(r, rom);
            r.objs.o[2].ypx = r.objs.o[2].ypx.wrapping_sub(1);
            if r.objs.o[2].ypx == 0xF8 {
                break;
            }
        }
        r.draw_objects(rom);
    }
    // the saucer lands: object 1 stands (animation $13) for 256 frames, then a second object flies up from $60
    r.objs.set_anim(1, 0x13);
    r.objs.p95 = 0;
    loop {
        r.draw_objects(rom);
        if r.objs.p95 & 0x1F == 0 {
            r.snd(0x28, 0x80DA); // code_1B80D7
        }
        if r.objs.p95 == 0 {
            break;
        }
    }
    r.objs.copy_obj(1, 0, 0x28);
    r.objs.o[0].ypx = 0x60;
    r.objs.o[0].ty = 0x60;
    loop {
        r.objs.o[0].ypx = r.objs.o[0].ypx.wrapping_sub(2);
        if r.objs.o[0].ypx == 0xF8 {
            break;
        }
        r.draw_objects(rom);
    }
    r.objs.set_anim(1, 0x14);
    loop {
        r.draw_objects(rom);
        if r.objs.o[1].aframe == 0x3B {
            break;
        }
    }
    r.snd(0x37, 0x8118); // code_1B8115
    r.objs.set_anim(1, 0x15);
    r.objs.p95 = 0;
    loop {
        r.draw_objects(rom);
        if r.objs.p95 == 0 {
            break;
        }
    }
    scroll_scene(r, rom);
}

/// The scrolling scene: the saucer's trip over the countryside (screens `$0C` and `$0B` down to 0), the names typed on the sky.
fn scroll_scene(r: &mut SceneRunner, rom: &Rom) {
    let st = Stage::load(rom, 0x14, 5);
    r.p9a = 0;
    r.fade_out();
    clear(r);
    r.yield_frame();
    if std::env::var("MM4_DBG").is_ok() {
        eprintln!("scene 2 starts at frame {}", r.frames.len());
    }
    scene_load(r, rom, 0x14, 0x0C, 0x4C);
    load_palette(r, rom, 0x20);
    r.objs.set_anim(1, 0x25);
    {
        let o = &mut r.objs.o[1];
        o.ty = 0x60;
        o.flags = 0x90;
        o.xpg = 0;
        o.ypg = 0;
        o.ypx = 0;
        o.xpx = 0x80;
        o.yvl = 0;
        o.yvh = 0xFF;
    }
    for y in [0usize, 2] {
        r.objs.copy_obj(1, y, b1b(rom, 0x8ABD + y as u16));
        let o = &mut r.objs.o[y];
        o.flags = 0x98;
        o.s[0] = b1b(rom, 0x8ABE + y as u16);
        o.ty = 0x60;
        o.ypx = b1b(rom, 0x8AB9 + y as u16);
        o.xpg = b1b(rom, 0x8ABA + y as u16);
        o.xpx = 0;
        o.xvl = 0;
        o.xvh = 8;
        o.yvl = 0;
        o.yvh = 0;
    }
    r.ppu.nt1 = Some(crate::screen::Shared::new(vec![0; 1024]));
    let mut stream = Stream { b1b: 0x80, attr: [0; 64] };
    loop {
        stream.call(r, &st, 0x0B);
        r.yield_frame();
        if stream.b1b == 0 {
            break;
        }
    }
    if std::env::var("MM4_DBG").is_ok() {
        eprintln!("stream done at frame {}", r.frames.len());
    }
    r.x69 = 0;
    r.x6a = 0;
    let (mut p1c, mut p24, mut p2a);
    p1c = 0x0Bu8;
    p24 = 0x1Fu8;
    r.snd_music(0x0B, 0x81D5); // code_1B81D2
    r.fade_in();
    loop {
        r.objs.o[1].atick = 0;
        r.objs.move_y(1, GRAVITY);
        if r.objs.o[1].ypx > 0x54 {
            r.objs.o[1].ypx = 0x54;
            break;
        }
        r.draw_objects(rom);
    }
    loop {
        r.draw_objects(rom);
        if r.objs.o[1].aframe == 4 {
            break;
        }
    }
    r.objs.set_anim(1, 0x23);
    r.f1 = 0x60;
    r.f0 = 0x0A;
    p2a = 0x0B;
    let (mut p50, mut p51) = (b1b(rom, 0x8AEC), b1b(rom, 0x8AED));
    let mut p52 = 0x8AEEu16;
    let (mut p54, mut p55) = (0u8, 0usize);
    loop {
        if r.x69 | p2a == 0 {
            break;
        }
        if r.objs.o[1].s[0] & 1 == 0 {
            let borrow = r.x69 == 0;
            r.x69 = r.x69.wrapping_sub(1);
            r.x6a = r.x6a.wrapping_sub(borrow as u8) & 1;
            if borrow {
                p2a -= 1;
            }
            // code_1B882C: every fourth pixel one block of the incoming screen
            if r.x69 & 3 == 3 {
                let mut idx = ((p24 >> 3) | ((p24 & 7) << 3)) as usize;
                if idx < 0x18 {
                    build_block(r, &st, p1c as usize, idx);
                }
                idx = 0;
                let _ = idx;
                p24 = p24.wrapping_sub(1) & 0x3F;
                if p24 == 0x3F {
                    p1c -= 1;
                }
            }
            // code_1B86DB: palette steps at fixed scroll positions
            if p55 != 0x14 && p2a == b1b(rom, 0x8989 + p55 as u16) && b1b(rom, 0x8975 + p55 as u16) == r.x69 {
                let base = 0x899D + p55 as u16 * 12;
                p55 += 1;
                for i in 0..4usize {
                    let v = b1b(rom, base + i as u16);
                    r.ram_pal[i] = v;
                    r.target[i] = v;
                }
                for i in 4..12usize {
                    let v = b1b(rom, base + i as u16);
                    r.ram_pal[i + 4] = v;
                    r.target[i + 4] = v;
                }
                r.f18 = true;
            }
        } else {
            // code_1B871F: the typed names
            if p54 != 0 {
                p54 -= 1;
                if p54 == 0 {
                    let b: Vec<u8> = (0..43).map(|i| b1b(rom, 0x8AC1 + i)).collect();
                    r.set_buffer(&b);
                }
            } else {
                let mut a = b1b(rom, p52);
                let mut write = true;
                if a == 0x7F {
                    p50 = b1b(rom, p52 + 1);
                    p51 = b1b(rom, p52 + 2);
                    p52 += 3;
                } else if a == 0xFF {
                    write = false;
                } else if a & 0x80 != 0 {
                    p54 = a;
                    p50 = b1b(rom, p52 + 1);
                    p51 = b1b(rom, p52 + 2);
                    p52 += 3;
                    write = false;
                }
                if write {
                    a = b1b(rom, p52);
                    r.set_buffer(&[p50, p51, 0, a, 0xFF]);
                    p51 = p51.wrapping_add(1);
                    p52 += 1;
                }
            }
        }
        // code_1B87A1: two objects drift sideways
        for x in [0usize, 2] {
            if r.objs.o[x].s[0] == 0 {
                r.objs.move_x_plus(x);
                if r.objs.o[x].xpg == 0 {
                    continue;
                }
                r.objs.o[x].s[0] = 0x8D;
            }
            r.objs.o[x].s[0] -= 1;
            if r.objs.o[x].s[0] == 0 {
                r.objs.o[x].xpx = 0;
                r.objs.o[x].xpg = 0;
            }
        }
        r.objs.p95 = 0xFF;
        r.draw_objects(rom);
        r.objs.o[1].s[0] = r.objs.o[1].s[0].wrapping_add(1);
    }
    staff_roll(r, rom);
}

/// `code_3EDA8B` / `code_3EDABB`: the in-game graphics loader (bank `$29` table `$B800/$B860`): palette bytes into the sprite palettes 2-3, then 48 bytes of pattern data per frame.
struct SetLoader {
    base: u16,
    y: u16,
    cnt: u8,
    bank: usize,
    src: u16,
    dst: u16,
}

impl SetLoader {
    fn table(rom: &Rom, a: u16) -> u8 {
        rom.cpu_byte(0x3A, 0x29, a)
    }

    fn start(r: &mut SceneRunner, rom: &Rom, set: u8) -> SetLoader {
        let base = Self::table(rom, 0xB800 + set as u16) as u16 | (Self::table(rom, 0xB860 + set as u16) as u16) << 8;
        let mut y = 0u16;
        let mut full = true;
        while y < 8 {
            let v = Self::table(rom, base + y);
            y += 1;
            if v & 0x80 != 0 {
                full = false;
                break;
            }
            r.ram_pal[0x18 + (y as usize - 1)] = v;
            r.target[0x18 + (y as usize - 1)] = v;
        }
        if full {
            r.f18 = true;
        }
        SetLoader { base, y, cnt: 0, bank: 0, src: 0, dst: 0 }
    }

    /// One call of `code_3EDABB` (from `code_3FFD05`, once per frame): queue the next 48 bytes.
    fn step(&mut self, r: &mut SceneRunner, rom: &Rom) {
        if self.cnt == 0 {
            let a = Self::table(rom, self.base + self.y);
            if a & 0x80 != 0 {
                return;
            }
            self.bank = a as usize;
            self.cnt = Self::table(rom, self.base + self.y + 1);
            self.src = (Self::table(rom, self.base + self.y + 2) as u16) << 8;
            self.dst = (Self::table(rom, self.base + self.y + 3) as u16) << 8;
            self.y += 4;
        }
        let mut data = Vec::new();
        loop {
            let i = data.len() as u16;
            data.push(rom.cpu_byte(self.bank, self.bank + 1, self.src + i));
            if data.len() % 16 == 0 {
                self.cnt -= 1;
                if self.cnt == 0 || data.len() == 0x30 {
                    break;
                }
            }
        }
        let n = data.len() as u16;
        let mut b = vec![(self.dst >> 8) as u8, self.dst as u8, (n - 1) as u8];
        b.extend_from_slice(&data);
        b.push(0xFF);
        r.set_buffer(&b);
        self.src += n;
        self.dst += n;
    }
}

/// `code_1B8678`: the names of the staff typed one letter per frame.
fn type_staff(r: &mut SceneRunner, rom: &Rom, staff: usize) {
    let p = b1b(rom, 0x8E26 + staff as u16) as u16 | (b1b(rom, 0x8E2E + staff as u16) as u16) << 8;
    let mut y = 0u16;
    loop {
        let hi = b1b(rom, p + y);
        if hi & 0x80 != 0 {
            return;
        }
        let mut lo = b1b(rom, p + y + 1);
        let n = b1b(rom, p + y + 2);
        y += 3;
        for _ in 0..=n {
            r.set_buffer(&[hi, lo, 0, b1b(rom, p + y), 0xFF]);
            r.yield_frame();
            y += 1;
            lo = lo.wrapping_add(1);
        }
    }
}

/// The robot masters' pictures with the staff names (`code_1B8267` on).
fn staff_roll(r: &mut SceneRunner, rom: &Rom) {
    r.objs.p95 = 0;
    while r.objs.p95 != 0x3C {
        r.draw_objects(rom);
    }
    let blank: Vec<u8> = (0..43).map(|i| b1b(rom, 0x8AC1 + i)).collect();
    r.set_buffer(&blank);
    for y in 0..8usize {
        let v = b1b(rom, 0x8AAD + y as u16);
        r.ram_pal[0x18 + y] = v;
        r.target[0x18 + y] = v;
    }
    r.f18 = true;
    r.objs.copy_obj(1, 0, 0x21);
    r.objs.o[0].ty = 0x60;
    r.objs.o[0].xpx = 0;
    r.objs.o[0].ypx = 0xD7;
    loop {
        r.objs.o[0].xpx = r.objs.o[0].xpx.wrapping_add(1);
        r.draw_objects(rom);
        if r.objs.o[0].xpx == 0x54 {
            break;
        }
    }
    r.objs.set_anim(0, 0x22);
    r.objs.p95 = 0;
    while r.objs.p95 != 0x3C {
        r.draw_objects(rom);
    }
    r.objs.set_anim(1, 0x24);
    r.objs.o[1].yvl = 0;
    r.objs.o[1].yvh = 5;
    loop {
        r.objs.move_y(1, GRAVITY);
        if r.objs.o[1].ypx > 0x7C {
            r.objs.o[1].ypx = 0x7C;
        }
        r.draw_objects(rom);
        if r.objs.o[1].ypx == 0x7C {
            break;
        }
    }
    for y in 0..32usize {
        let v = b1b(rom, 0x8A8D + y as u16);
        r.ram_pal[y] = v;
        r.target[y] = v;
    }
    r.f18 = true;
    r.p9a = 0;
    r.snd_music(0xF6, 0x8303); // code_1B8300
    r.wait(0);
    r.fade_out();
    r.f0 = 0;
    r.yield_frame();
    clear(r);
    r.yield_frame();
    scene_load(r, rom, 0x10, 2, 0x10);
    load_palette(r, rom, 0x40);
    r.objs.o[0].ty = 0x81;
    r.objs.set_anim(0, 0x44);
    {
        let o = &mut r.objs.o[0];
        o.flags = 0x90;
        o.xpg = 0;
        o.ypg = 0;
        o.ypx = 0x74;
        o.xpx = 0x50;
    }
    let b: Vec<u8> = (0..9).map(|i| b1b(rom, 0x8E14 + i)).collect();
    r.set_buffer(&b);
    r.fade_in();
    r.snd_music(0x13, 0x8369); // code_1B8366
    for x in 1..4usize {
        r.anim_id[x] = 0x92 + x as u8 - 1;
        r.anim_idx[x] = 0;
        r.anim_tick[x] = 0;
    }
    r.wait(0x96);
    let b: Vec<u8> = (0..9).map(|i| b1b(rom, 0x8E1D + i)).collect();
    r.set_buffer(&b);
    r.yield_frame();
    let mut staff = 0usize;
    loop {
        let set = b1b(rom, 0x8C93 + staff as u16);
        r.p9a += 1;
        let mut loader = SetLoader::start(r, rom, set);
        r.f18 = false;
        r.p9a -= 1;
        for _ in 0..0x78 {
            loader.step(r, rom);
            r.yield_frame();
        }
        r.objs.o[0].flags &= 0xFB;
        r.draw_objects(rom);
        r.p9a = 0;
        let mut level = 0x30u8;
        loop {
            for x in 0..8usize {
                let c = b1b(rom, 0x8C4B + x as u16);
                let a = if c < level { 0x0F } else { c - level };
                r.ram_pal[0x10 + x] = a;
                r.target[0x10 + x] = a;
                let c = b1b(rom, 0x8C53 + staff as u16 * 8 + x as u16);
                let a = if c < level { 0x0F } else { c - level };
                r.ram_pal[0x18 + x] = a;
                r.target[0x18 + x] = a;
            }
            r.f18 = true;
            r.wait(0x0A);
            if level < 0x10 {
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
        type_staff(r, rom, staff);
        r.wait(0xF0);
        staff += 1;
        if staff == 8 {
            last_picture(r, rom);
            return;
        }
        let b: Vec<u8> = (0..0x33).map(|i| b1b(rom, 0x8DE1 + i)).collect();
        r.set_buffer(&b);
        let mut level = 0x10u8;
        while level != 0x50 {
            for y in 0..16usize {
                let c = r.target[0x10 + y];
                r.ram_pal[0x10 + y] = if c < level { 0x0F } else { c - level };
            }
            r.f18 = true;
            r.wait(0x0A);
            level += 0x10;
        }
        for y in 0..3usize {
            r.anim_id[1 + y] = r.anim_id[1 + y].wrapping_add(3);
        }
        r.objs.o[0].flags |= 4;
        r.objs.o[0].aid = r.objs.o[0].aid.wrapping_add(1);
        r.objs.o[0].aframe = 0;
        r.objs.o[0].atick = 0;
        r.draw_objects(rom);
        r.p9a = 0;
    }
}

/// `code_1B86B8`: the sprites of the train drift (three by 3 pixels, eight by 2).
fn drift(r: &mut SceneRunner) {
    for y in [0usize, 4, 8] {
        r.ppu.oam[3 + y] = r.ppu.oam[3 + y].wrapping_add(3);
    }
    for k in 0..8usize {
        r.ppu.oam[0x0F + k * 4] = r.ppu.oam[0x0F + k * 4].wrapping_add(2);
    }
}

/// `code_1B8483` on: the last picture, the credits typed line by line while the picture scrolls up, then Mega Man rides on and his friends join him. The game stays in the
/// last loop for ever; `r.limit` ends it (a recording check, or the window leaving the scene).
fn last_picture(r: &mut SceneRunner, rom: &Rom) {
    r.p9a = 0;
    for x in 1..4usize {
        r.anim_id[x] = 0;
    }
    r.fade_out();
    clear(r);
    scene_load(r, rom, 0x14, 0x0D, 0x4D);
    load_palette(r, rom, 0x60);
    for i in 0..44usize {
        r.ppu.oam[i] = b1b(rom, 0x8BFB + i as u16);
    }
    r.yield_frame();
    r.fade_in();
    let (mut p10, mut p12, mut p13) = (0x955Du16, 1u8, 0u8);
    r.objs.p95 = 0;
    loop {
        if r.objs.p95 & 1 == 0 {
            if p12 == 0x24 && r.fa == 0x90 {
                r.snd_music(0xF6, 0x84E7); // code_1B84E4
                break;
            }
            r.fa += 1;
            if r.fa == 0xF0 {
                r.fa = 0;
            }
            if r.fa & 7 == 0 {
                p13 += 1;
                if p13 == 0x1E {
                    p13 = 0;
                }
                let mut b: Vec<u8> = (0..0x24).map(|i| b1b(rom, 0x8C27 + i)).collect();
                if p13 == b1b(rom, p10) {
                    let mut x = b1b(rom, p10 + 1) as usize;
                    let mut y = 2u16;
                    loop {
                        let c = b1b(rom, p10 + y);
                        if c & 0x80 != 0 {
                            break;
                        }
                        b[3 + x] = c;
                        x += 1;
                        y += 1;
                    }
                    p10 = b1b(rom, 0x9513 + p12 as u16) as u16 | (b1b(rom, 0x9538 + p12 as u16) as u16) << 8;
                    p12 += 1;
                }
                let row = (p13 as u16) << 5;
                b[1] |= row as u8;
                b[0] = 0x20 | (row >> 8) as u8;
                r.set_buffer(&b);
            }
        }
        drift(r);
        r.yield_frame();
        r.objs.p95 = r.objs.p95.wrapping_add(1);
    }
    // Mega Man appears on the right and rides to the left
    r.objs.set_anim(0, 0x5A);
    {
        let o = &mut r.objs.o[0];
        o.flags = 0x90;
        o.ty = 0x80;
        o.xpg = 0;
        o.ypg = 0;
        o.xpx = 0xFC;
        o.ypx = 0x88;
    }
    while r.objs.o[0].xpx != 0xB0 {
        r.objs.o[0].xpx -= 1;
        drift(r);
        r.draw_objects_at(rom, 0x2C);
    }
    r.objs.o[0].s[1] = 6;
    r.objs.o[0].s[0] = 1;
    loop {
        if r.objs.o[0].s[1] != 0 {
            r.objs.o[0].s[0] = r.objs.o[0].s[0].wrapping_sub(1);
            if r.objs.o[0].s[0] == 0 {
                r.objs.o[0].s[0] = 0x28;
                r.objs.set_anim(0, 0x59);
            }
            if r.objs.o[0].aid == 0x59 && r.objs.o[0].aframe == 4 {
                r.objs.o[0].aframe += 1;
                let y = r.objs.o[0].s[1] as usize;
                r.objs.copy_obj(0, y, b1b(rom, 0x866B + y as u16));
                {
                    let o = &mut r.objs.o[y];
                    o.yvl = 0xE5;
                    o.yvh = 4;
                    o.xvl = b1b(rom, 0x865F + y as u16);
                    o.xvh = b1b(rom, 0x8665 + y as u16);
                    o.s[0] = b1b(rom, 0x8671 + y as u16);
                    o.ty = 0x60;
                }
                r.objs.o[0].s[1] -= 1;
                if r.objs.o[0].s[1] == 0 {
                    r.snd_music(0xF0, 0x8612); // code_1B860F
                }
            }
        }
        let x = r.objs.o[0].s[1] as usize + 1;
        r.objs.move_y(x, GRAVITY);
        if r.objs.o[x].yvh & 0x80 != 0 && r.objs.o[x].ypx > 0x70 {
            r.objs.o[x].ypx = 0x70;
            r.objs.o[x].xpx = r.objs.o[x].s[0];
            r.snd(0x23, 0x8633); // code_1B8630

        } else {
            r.objs.move_x_minus(x);
        }
        if r.objs.o[0].aid == 0x59 && r.objs.o[0].aframe == 7 {
            r.objs.set_anim(0, 0x58);
        }
        drift(r);
        r.draw_objects_at(rom, 0x2C);
        if r.frames.len() >= r.limit || (r.link.is_some() && r.held & 0x10 != 0) {
            return;
        }
    }
}

/// Compare with the recording of the ending (`ppu_frames.lua`, `PF_HIJACK` with `PF_STATE=19`): the scene starts at the first frame with `$22 = $14`.
pub fn verify(rom: &Rom, path: &str) -> bool {
    use crate::pfrec;
    let frames = match pfrec::load(path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("{}", e);
            return false;
        }
    };
    // the scene starts at recording frame 18 ($92 = $5D, $95 = $D6 then); a recording that begins later (`PF_FROM`) is compared from its first frame
    let (start, p92, p95, mega) = match frames.iter().find(|f| f.r("scene") == 0x14) {
        Some(f) if f.n <= 18 => {
            let prev = frames.iter().find(|g| g.n + 1 == f.n).unwrap_or(f);
            (f.n, f.r("p92") as u8, f.r("p95") as u8, SObj::from_dump(&prev.obj[..24]))
        }
        Some(_) => (18, 0x5D, 0xD6, mega_man_after_stage()),
        None => return false,
    };
    let mut blank = crate::screen::Screen::new();
    blank.pal = [0x0F; 32];
    let mut r = SceneRunner::new(blank, p92, p95);
    r.limit = frames.iter().map(|f| f.n).max().unwrap_or(0) as usize - start as usize + 1;
    play(&mut r, rom, mega);
    let mine = r.frames;
    let (mut shown, mut bad, mut checked) = (0, 0, 0);
    for (t, ours) in mine.iter().enumerate() {
        let f = match frames.iter().find(|f| f.n == start + t as u32) {
            Some(f) => f,
            None if checked == 0 => continue,
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
