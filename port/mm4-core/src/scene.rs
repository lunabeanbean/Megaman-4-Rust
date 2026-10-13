//! A small model of the way the game's presentation code (bank `$39` and friends) drives the PPU, used by the cutscene screens.
//!
//! The game's scenes are straight-line code that calls `code_3FFF1C` to wait one video frame. `SceneRunner` runs such a script as ordinary Rust code:
//! every `yield_frame` records one snapshot of the picture (palette, nametable and pattern RAM as the PPU holds them, OAM as the game's RAM `$0200` holds it:
//! what a Mesen recording sees at the end of that frame) and then does what the NMI does: upload the palette when `$18` is set, apply the text buffer when `$19` is set.
//! Scene objects (Mega Man's big face, Rush, ...) go through `SceneObjs`, the draw pass of `code_3EDB3A` for objects that are not in a scrolling stage.
use crate::anim::AnimTable;
use crate::oam::{self, Sprite};
use crate::rom::Rom;
use crate::screen::Screen;
use std::cell::Cell;

thread_local! {
    /// `$95` as the latest scene frame left it (the stage that follows a scene starts from it).
    static LAST_P95: Cell<u8> = Cell::new(0);
}

/// A scene that counts `$95` itself (not through the object passes) reports its final value.
pub fn set_last_p95(v: u8) {
    LAST_P95.with(|c| c.set(v));
}

/// `$95` at the end of the scene run most recently on this thread.
pub fn last_p95() -> u8 {
    LAST_P95.with(|c| c.get())
}

pub const SLOTS: usize = 24;

/// One object slot (`$0300`, `$0528`, `$0330`, `$0348`, `$0378`, `$0390`, `$0558`, `$0540`, `$0570`, `$05B8` + the slot).
#[derive(Clone, Copy, Default, Debug)]
pub struct SObj {
    pub ty: u8,
    pub flags: u8,
    pub xpx: u8,
    pub xpg: u8,
    pub ypx: u8,
    pub ypg: u8,
    pub aid: u8,
    pub aframe: u8,
    pub atick: u8,
    pub t5b8: u8,
    /// `$0468, $0480, $0498, $04B0, $04C8, $04E0, $04F8, $0510` (+ slot): the free registers the scene scripts use (`s[0]` is a timer).
    pub s: [u8; 8],
    /// `$0318`, `$0360`: sub-pixel parts of X and Y; `$03A8`/`$03C0`: X speed (fraction, pixels); `$03D8`/`$03F0`: Y speed.
    pub xsub: u8,
    pub ysub: u8,
    pub xvl: u8,
    pub xvh: u8,
    pub yvl: u8,
    pub yvh: u8,
}

pub struct SceneObjs {
    pub o: [SObj; SLOTS],
    /// `$95`: counts draw passes; its parity picks the slot order.
    pub p95: u8,
    tables: Vec<Option<AnimTable>>,
}

impl SObj {
    /// One slot from the recorder's `OBJ` bytes (24 per slot: `$0300 $0528 $0348 $0330 $0318 $0390 $0378 $0360 $0558 $0540 $0570 $05B8 $03A8 $03C0 $03D8 $03F0 $0468..$0510`).
    pub fn from_dump(b: &[u8]) -> SObj {
        let mut s = [0u8; 8];
        s.copy_from_slice(&b[16..24]);
        SObj { ty: b[0], flags: b[1], xpg: b[2], xpx: b[3], xsub: b[4], ypg: b[5], ypx: b[6], ysub: b[7], aid: b[8], aframe: b[9], atick: b[10], t5b8: b[11], xvl: b[12], xvh: b[13], yvl: b[14], yvh: b[15], s }
    }
}

impl SceneObjs {
    pub fn new() -> SceneObjs {
        SceneObjs { o: [SObj::default(); SLOTS], p95: 0, tables: vec![None; 256] }
    }

    /// `code_3FF22D`: X += speed (16-bit with the page).
    pub fn move_x_plus(&mut self, x: usize) {
        let o = &mut self.o[x];
        let (v, c) = o.xsub.overflowing_add(o.xvl);
        o.xsub = v;
        let (v, c2) = o.xpx.overflowing_add(o.xvh);
        let (v, c3) = v.overflowing_add(c as u8);
        o.xpx = v;
        if c2 || c3 {
            o.xpg = o.xpg.wrapping_add(1);
        }
    }

    /// `code_3FF24B`: X -= speed.
    pub fn move_x_minus(&mut self, x: usize) {
        let o = &mut self.o[x];
        let (v, b) = o.xsub.overflowing_sub(o.xvl);
        o.xsub = v;
        let (v, b2) = o.xpx.overflowing_sub(o.xvh);
        let (v, b3) = v.overflowing_sub(b as u8);
        o.xpx = v;
        if b2 || b3 {
            o.xpg = o.xpg.wrapping_sub(1);
        }
    }

    /// `code_3FF2A7`: Y -= speed (`$03D8/$03F0`: positive = up), wrapping into the next screen at `$F0`, then gravity `$99` lowers the speed (terminal speed `-7`).
    pub fn move_y(&mut self, x: usize, gravity: u8) {
        let o = &mut self.o[x];
        let (v, b) = o.ysub.overflowing_sub(o.yvl);
        o.ysub = v;
        let (y, b2) = o.ypx.overflowing_sub(o.yvh);
        let (y, b3) = y.overflowing_sub(b as u8);
        o.ypx = y;
        let carry = !(b2 || b3);
        if o.yvh & 0x80 != 0 {
            if o.ypx >= 0xF0 {
                o.ypx = o.ypx.wrapping_add(0x10);
                o.ypg = o.ypg.wrapping_add(1);
            }
        } else if !carry {
            o.ypx = o.ypx.wrapping_sub(0x10);
            o.ypg = o.ypg.wrapping_sub(1);
        }
        let (v, b) = o.yvl.overflowing_sub(gravity);
        o.yvl = v;
        o.yvh = o.yvh.wrapping_sub(b as u8);
        if o.yvh & 0x80 != 0 && o.yvh < 0xF9 {
            o.yvh = 0xF9;
            o.yvl = 0;
        }
    }

    /// `code_3EC438`: remove every object but slot 0 (the loop stops at X = 1).
    pub fn clear_all(&mut self) {
        for x in 1..SLOTS {
            let o = &mut self.o[x];
            o.ty = 0;
            o.t5b8 = 0;
        }
    }

    /// `code_3FF452`: object `y` becomes a copy of object `x` (position, mirror flag, registers cleared) with animation `anim`; its type is left alone.
    pub fn copy_obj(&mut self, x: usize, y: usize, anim: u8) {
        let src = self.o[x];
        let d = &mut self.o[y];
        d.xpx = src.xpx;
        d.xpg = src.xpg;
        d.ypx = src.ypx;
        d.ypg = src.ypg;
        d.flags = (src.flags & 0x40) | 0x90;
        d.s = [0; 8];
        d.aid = anim;
        d.aframe = 0;
        d.atick = 0;
    }

    /// `code_3FF446`: start animation `id` (frame and tick 0).
    pub fn set_anim(&mut self, x: usize, id: u8) {
        let o = &mut self.o[x];
        o.aid = id;
        o.aframe = 0;
        o.atick = 0;
    }

    fn table(&mut self, rom: &Rom, ty: u8) -> &AnimTable {
        if self.tables[ty as usize].is_none() {
            self.tables[ty as usize] = Some(AnimTable::load(rom, ty as usize));
        }
        self.tables[ty as usize].as_ref().unwrap()
    }

    /// `code_3EDB3A` (without the rain and the HUD meters): the objects in the order `$95` picks, animation ticked, parts appended to the sprite list.
    pub fn draw_pass(&mut self, rom: &Rom) -> Vec<Sprite> {
        self.p95 = self.p95.wrapping_add(1);
        let order: Vec<usize> = if self.p95 & 1 == 0 { (0..SLOTS).collect() } else { (0..SLOTS).rev().collect() };
        let mut out = Vec::new();
        for x in order {
            if self.o[x].ty == 0 {
                continue;
            }
            if let Some(parts) = self.draw_object(rom, x) {
                oam::append(&mut out, &parts);
            }
        }
        out
    }

    /// `code_3EDB72`.. for a camera at 0: scene-fixed (`$0528` bit 4 clear) or on-screen objects; animation tick (`code_3EDC67`); the sprite parts.
    fn draw_object(&mut self, rom: &Rom, x: usize) -> Option<Vec<Sprite>> {
        let o = self.o[x];
        if o.flags & 0x10 == 0 {
            self.o[x].flags |= 0x80;
            if self.o[x].flags & 4 != 0 {
                return None;
            }
        } else {
            // camera-relative objects: only the on-screen case occurs in scenes (page 0)
            if o.xpg == 0 && o.ypg == 0 {
                self.o[x].flags |= 0x80;
                if self.o[x].flags & 4 != 0 {
                    return None;
                }
            } else {
                self.o[x].flags &= 0x7F;
            }
        }
        let o = self.o[x];
        if o.aid == 0 {
            return None;
        }
        let ty = o.ty;
        let (last, dur) = self.table(rom, ty).header(o.aid);
        let mut t5b8 = o.t5b8;
        let mut atick = o.atick;
        let mut aframe = o.aframe;
        if t5b8 != 0 {
            t5b8 = t5b8.wrapping_sub(1);
            if t5b8 & 0x7F == 0 {
                t5b8 = 0;
            }
        }
        if t5b8 & 0x80 == 0 {
            let t = atick & 0x7F;
            atick = atick.wrapping_add(1);
            if t == dur {
                atick &= 0x80;
                let old = aframe & 0x7F;
                aframe = aframe.wrapping_add(1);
                if old == last {
                    aframe = 0;
                }
            }
        }
        {
            let m = &mut self.o[x];
            m.t5b8 = t5b8;
            m.atick = atick;
            m.aframe = aframe;
        }
        if self.o[x].flags & 0x80 == 0 {
            return None;
        }
        if t5b8 != 0 && t5b8 & 0x80 == 0 && (t5b8 >> 1) & 1 != 0 {
            return None; // blink
        }
        let (aid, af) = (self.o[x].aid, self.o[x].aframe & 0x7F);
        if self.table(rom, ty).sprite(aid, af) == 0 {
            let m = &mut self.o[x];
            m.ty = 0;
            return None;
        }
        let o = self.o[x];
        let table = self.table(rom, ty).clone();
        Some(oam::object_parts(&table, o.aid, o.aframe, o.flags & 0x40 != 0, o.xpx, o.ypx, 0, o.flags & 0x20 != 0))
    }
}

impl Default for SceneObjs {
    fn default() -> SceneObjs {
        SceneObjs::new()
    }
}

/// The runner: PPU side (`ppu`), the RAM the NMI copies from, and the recorded frames.
/// Live link of a scene script running on its own thread: every finished frame goes out, the buttons for the next frame come in.
pub struct Link {
    pub tx: std::sync::mpsc::Sender<Screen>,
    pub rx: std::sync::mpsc::Receiver<u8>,
}

pub struct SceneRunner {
    /// Palette, nametable (one of the four, the displayed one) and pattern RAM as the PPU holds them; `oam` is the RAM list `$0200`.
    pub ppu: Screen,
    /// `$0600`: the palette the NMI uploads when `f18` is set.
    pub ram_pal: [u8; 32],
    /// `$0620`: the palette the fades work towards.
    pub target: [u8; 32],
    pub f18: bool,
    /// `$0780..`: records the NMI writes into the nametable when `f19` is set.
    pub buf: Vec<u8>,
    pub f19: bool,
    pub frames: Vec<Screen>,
    /// `$92` (counts frames).
    pub p92: u8,
    /// `$9A`: while it is not zero the NMI leaves the OAM, the text buffer and the palette alone (it counts the frames since the object pass finished).
    pub p9a: u8,
    pub objs: SceneObjs,
    /// `$64`: palettes the fades leave alone (bit n = palette 7 - n).
    pub fade_mask: u8,
    /// `$F0` (scroll / interrupt mode), `$F1` (scanline of the first interrupt), `$FC` (scroll), `$69` (scroll of the top band in the scroll modes).
    pub f0: u8,
    pub f1: u8,
    pub fc: u8,
    pub x69: u8,
    /// `$6A`: bit 0 = nametable at the left edge of the top band when it scrolls with `$69`.
    pub x6a: u8,
    /// `$FA`: vertical scroll.
    pub fa: u8,
    /// Scenes that never end stop at this many frames (checks only).
    pub limit: usize,
    /// The ROM, for the palette task (set by scenes that use palette animations).
    pub rom: Option<Rom>,
    /// `$16`: the buttons held in the current frame (live scenes only; recordings run without input).
    pub held: u8,
    /// Set when the scene runs live: frames are sent out instead of being kept.
    pub link: Option<Link>,
    /// `$06F0`, `$06F4`, `$06F8` (four slots each): the palette animation task (`code_20B800`, runs once per frame after the main thread).
    pub anim_id: [u8; 4],
    pub anim_idx: [u8; 4],
    pub anim_tick: [u8; 4],
    /// Sound requests made since the last frame boundary (they go with the next snapshot).
    pub snd: Vec<crate::sound::Req>,
}

impl SceneRunner {
    pub fn new(ppu: Screen, p92: u8, p95: u8) -> SceneRunner {
        let mut objs = SceneObjs::new();
        objs.p95 = p95;
        SceneRunner { ppu, ram_pal: [0x0F; 32], target: [0x0F; 32], f18: false, buf: Vec::new(), f19: false, frames: Vec::new(), p92, p9a: 0, objs, fade_mask: 0, f0: 0, f1: 0x88, fc: 0, x69: 0, x6a: 0, fa: 0, limit: usize::MAX, rom: None, held: 0, link: None, anim_id: [0; 4], anim_idx: [0; 4], anim_tick: [0; 4], snd: Vec::new() }
    }

    /// `JSR code_3FF6BE` from a scene script.
    pub fn snd(&mut self, id: u8, ret: u16) {
        self.snd.push(crate::sound::Req { music: false, id, ret });
    }

    /// `JSR code_3FF6BC` from a scene script (music / stop).
    pub fn snd_music(&mut self, id: u8, ret: u16) {
        self.snd.push(crate::sound::Req { music: true, id, ret });
    }

    /// `code_3FFF1C`: one video frame ends. The snapshot is what a recording sees; then the NMI uploads the palette and the text buffer.
    pub fn yield_frame(&mut self) {
        LAST_P95.with(|c| c.set(self.objs.p95));
        // the scroll registers as the NMI will send them (the picture of the next frame): top band `$FC` or `$69`, then the interrupt bands
        let top = if matches!(self.f0, 3 | 6 | 7) || self.f0 >= 9 { self.x69 } else { self.fc };
        self.ppu.scroll_x = top;
        self.ppu.nt_base = self.x6a & 1;
        self.ppu.scroll_y = self.fa;
        self.ppu.snd = std::mem::take(&mut self.snd);
        self.ppu.bands = match self.f0 {
            0x0A => vec![(0, top), (self.f1 as u16 + 1, 0)],
            0x0B => vec![(0, top), (self.f1 as u16 + 1, self.fc), (self.f1 as u16 + 1 + 0x29, 0)],
            _ => Vec::new(),
        };
        if let Some(l) = &self.link {
            // live: hand the picture to the window and wait for the next frame's buttons; a closed channel ends the script
            if l.tx.send(self.ppu.clone()).is_err() {
                std::panic::resume_unwind(Box::new("scene closed"));
            }
            match l.rx.recv() {
                Ok(b) => self.held = b,
                Err(_) => std::panic::resume_unwind(Box::new("scene closed")),
            }
        } else {
            self.frames.push(self.ppu.clone());
        }
        self.ppu.snd.clear();
        self.palette_task();
        if self.p9a != 0 {
            self.p92 = self.p92.wrapping_add(1);
            return;
        }
        if self.f18 {
            for i in 0..32 {
                self.ppu.pal[i] = self.ram_pal[i];
                if i >= 16 && i & 3 == 0 {
                    self.ppu.pal[i - 16] = self.ram_pal[i]; // $3F10/14/18/1C mirror $3F00/04/08/0C
                }
            }
            self.f18 = false;
        }
        if self.f19 {
            let b = std::mem::take(&mut self.buf);
            self.ppu.apply_buffer(&b);
            self.f19 = false;
        }
        self.p92 = self.p92.wrapping_add(1);
    }

    /// `code_20B800` (bank `$20`, run by the second thread once per frame): palette animations. Slot `x` (0..3) with `$06F0+x` bit 7 set steps the colours 1-3 of
    /// background palette `x` through the rows of table `$B8C7` named by animation `$BD2C/$BD6C[id]`.
    fn palette_task(&mut self) {
        let rom = match self.rom.clone() {
            Some(r) => r,
            None => return,
        };
        if self.anim_id.iter().all(|&a| a & 0x80 == 0) {
            return;
        }
        let t = |a: u16| rom.cpu_byte(0x20, 0x20, a);
        let mut f61 = (self.f18 as u8) & 1; // the task keeps only bit 0 of `$18`, then adds one per animation step
        let was = self.f18;
        let _ = was;
        for x in (0..4usize).rev() {
            if self.anim_id[x] & 0x80 == 0 {
                continue;
            }
            let id = (self.anim_id[x] & 0x7F) as u16;
            let ptr = t(0xBD2C + id) as u16 | (t(0xBD6C + id) as u16) << 8;
            let tick = self.anim_tick[x];
            self.anim_tick[x] = tick.wrapping_add(1);
            if tick != t(ptr + 1) {
                continue;
            }
            self.anim_tick[x] = 0;
            let idx = self.anim_idx[x];
            self.anim_idx[x] = idx.wrapping_add(1);
            if idx == t(ptr) {
                self.anim_idx[x] = 0;
            }
            let entry = t(ptr + 2 + self.anim_idx[x] as u16);
            if entry == 0 {
                self.anim_id[x] = 0;
                continue;
            }
            let row = 0xB8C7u16 + entry as u16 * 3;
            for k in 0..3u16 {
                let v = t(row + k);
                if v & 0x80 != 0 {
                    self.ram_pal[0x10] = v & 0x3F;
                    self.target[0x10] = v & 0x3F;
                    break;
                }
                if v < 0x40 {
                    self.ram_pal[x * 4 + 1 + k as usize] = v;
                    self.target[x * 4 + 1 + k as usize] = v;
                }
            }
            f61 += 1;
            if self.anim_id[x] == 0x87 {
                for k in 0..4 {
                    self.ram_pal[0x18 + k] = self.ram_pal[8 + k];
                    self.target[0x18 + k] = self.ram_pal[8 + k];
                }
            }
        }
        self.f18 = f61 != 0;
    }

    /// `code_3FFF15`: wait `n` frames (`n` = 0 waits 256).
    pub fn wait(&mut self, n: u32) {
        for _ in 0..(if n == 0 { 256 } else { n }) {
            self.yield_frame();
        }
    }

    /// `$0780..` + `$19 = $FF`.
    pub fn set_buffer(&mut self, b: &[u8]) {
        self.buf = b.to_vec();
        self.f19 = true;
    }

    /// `code_3EC421` + `code_3EDB3A` + a frame (`code_3FFCE3`): the object draw pass fills the OAM from sprite 1 on, then the frame ends.
    pub fn draw_objects(&mut self, rom: &Rom) {
        self.draw_objects_at(rom, 4);
    }

    /// `code_3FFCE7` with `$97 = start`: the OAM is cleared from byte `start` on (what is before stays, e.g. fixed decoration sprites) and the objects are drawn there.
    pub fn draw_objects_at(&mut self, rom: &Rom, start: usize) {
        if start == 4 {
            self.ppu.oam[0] = 0xF8;
        }
        for i in (start / 4)..64 {
            self.ppu.oam[i * 4] = 0xF8;
        }
        let parts = self.objs.draw_pass(rom);
        for (i, p) in parts.iter().enumerate() {
            let o = start + i * 4;
            if o >= 256 {
                break;
            }
            self.ppu.oam[o..o + 4].copy_from_slice(&[p.y, p.tile, p.attr, p.x]);
        }
        self.p9a = 0;
        self.yield_frame();
        self.p9a = self.p9a.wrapping_add(1);
    }

    /// The palette bytes of `code_3EC451` / `code_3EC457`: `target` lowered by `level`, negative values black, palettes whose `fade_mask` bit is set left as they are.
    fn faded(&self, level: u8) -> [u8; 32] {
        let mut p = self.target; // masked palettes are copied unchanged from `$0620`
        let mut mask = self.fade_mask;
        let mut y = 0x1Fi32;
        while y >= 0 {
            if y & 3 == 3 {
                let c = mask & 1;
                mask >>= 1;
                if c != 0 {
                    y -= 4;
                    continue;
                }
            }
            let v = self.target[y as usize];
            let d = v.wrapping_sub(level);
            p[y as usize] = if d & 0x80 == 0 { d } else { 0x0F };
            y -= 1;
        }
        p
    }

    /// `code_3EC451`: fade in towards `target` (levels `$30,$20,$10,0`, each held for four frames).
    pub fn fade_in(&mut self) {
        let mut level = 0x30u8;
        loop {
            self.ram_pal = self.faded(level);
            self.f18 = true;
            self.wait(4);
            level = level.wrapping_add(0xF0);
            if level == 0x50 {
                break;
            }
            if level & 0x80 != 0 {
                break;
            }
        }
        self.fade_mask = 0;
    }

    /// `code_3EC457`: fade out (levels `$10,$20,$30,$40`).
    pub fn fade_out(&mut self) {
        let mut level = 0x10u8;
        loop {
            self.ram_pal = self.faded(level);
            self.f18 = true;
            self.wait(4);
            level = level.wrapping_add(0x10);
            if level == 0x50 {
                break;
            }
            if level & 0x80 != 0 {
                break;
            }
        }
        self.fade_mask = 0;
    }
}

/// The pictures of a fade-out (`code_3EC457`) of the frozen screen `base`: `held` frames unchanged, then four frames at each of three levels (`$10, $20, $30` subtracted from every
/// palette entry, negative = black), then `black` black frames (the fourth step and the scene change; three of them still carry the sprites).
pub fn fade_out_screens(base: &Screen, held: usize, black: usize) -> Vec<Screen> {
    let mut quiet = base.clone();
    quiet.snd.clear(); // the requests of the last frame of the scene were already made
    let base = &quiet;
    let mut v = Vec::new();

    for _ in 0..held {
        v.push(base.clone());
    }
    for level in [0x10u8, 0x20, 0x30] {
        let mut s = base.clone();
        for p in s.pal.iter_mut() {
            let d = p.wrapping_sub(level);
            *p = if d & 0x80 == 0 { d } else { 0x0F };
        }
        for _ in 0..4 {
            v.push(s.clone());
        }
    }
    // the first three black frames still carry the sprites; then the scene code clears the OAM and resets the scroll
    let mut s = base.clone();
    s.pal = [0x0F; 32];
    for i in 0..black {
        if i == 3 {
            for k in 0..64 {
                s.oam[k * 4] = 0xF8;
            }
            s.scroll_x = 0;
        }
        v.push(s.clone());
    }
    v
}

/// A scene script running on its own thread: `tick` hands it the buttons held in the next frame and returns the picture it produces (`None` once the script is over).
pub struct LiveScene {
    rx: std::sync::mpsc::Receiver<Screen>,
    tx: std::sync::mpsc::Sender<u8>,
    first: bool,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl LiveScene {
    pub fn start<F: FnOnce(&mut SceneRunner) + Send + 'static>(mut runner: SceneRunner, script: F) -> LiveScene {
        let (tx_frame, rx_frame) = std::sync::mpsc::channel();
        let (tx_held, rx_held) = std::sync::mpsc::channel();
        runner.link = Some(Link { tx: tx_frame, rx: rx_held });
        let handle = std::thread::spawn(move || {
            // a closed channel (the window went away) unwinds the script; nothing to report
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| script(&mut runner)));
        });
        LiveScene { rx: rx_frame, tx: tx_held, first: true, handle: Some(handle) }
    }

    pub fn tick(&mut self, held: u8) -> Option<Screen> {
        if !self.first && self.tx.send(held).is_err() {
            return None;
        }
        self.first = false;
        match self.rx.recv() {
            Ok(s) => Some(s),
            Err(_) => {
                if let Some(h) = self.handle.take() {
                    let _ = h.join();
                }
                None
            }
        }
    }
}
