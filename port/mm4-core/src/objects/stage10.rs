//! Stage 10 enemies: the block crawler `51`/`52` (AI `$78`, bank `$3B` `$AF35`), the rising/sinking platform `6A` (AI `$5D`, bank `$35` `$B383`) and
//! the falling floor pieces `1C` (AI `$45`, bank `$35` `$B32C`).
use super::*;

impl World {
    pub(super) fn run_stage10(&mut self, st: &Stage, x: usize, ptr: u16) -> bool {
        match (self.f6, ptr) {
            (0x3B, 0xAF35) => self.ai_af35(st, x),
            (0x3B, 0xAF57) => self.ai_af57(st, x),
            (0x3B, 0xAF79) => self.ai_af79(st, x),
            (0x3B, 0xAFC9) => self.ai_afc9(st, x),
            (0x35, 0xB32C) => self.ai_b32c(x),
            (0x35, 0xB354) => self.ai_b354(x),
            (0x35, 0xB383) => self.ai_b383(x),
            (0x35, 0xB39C) => self.ai_b39c(x),
            (0x35, 0xB3B7) => self.ai_b3b7(x),
            _ => return false,
        }
        true
    }

    /// `$3BAF35`: first state: facing sets the direction, slow fall; then as `$AF57`.
    fn ai_af35(&mut self, st: &Stage, x: usize) {
        if self.o[x].s[0] == 0 {
            let y = if self.o[x].flags & 0x40 != 0 { 1 } else { 2 };
            let o = &mut self.o[x];
            o.dir = y | 8;
            o.yvs = 0x4C;
            o.yvp = 1;
            o.s[0] += 1;
        }
        self.ai_af57(st, x);
    }

    /// `$AF57`: crawl along the block's top; at the edge turn the corner (down one step, then `$AF79`).
    fn ai_af57(&mut self, st: &Stage, x: usize) {
        if self.down_collide(st, x, 0x2A) {
            self.move_h(x);
            return;
        }
        let o = &mut self.o[x];
        o.dir ^= 0x0F;
        o.ptr = 0xAF79;
        o.ypx = o.ypx.wrapping_add(2);
        self.ai_af79(st, x);
    }

    fn c_step_x(&mut self, x: usize) {
        if self.o[x].dir & 1 == 0 {
            self.o[x].xpx = self.o[x].xpx.wrapping_sub(2);
        } else {
            self.o[x].xpx = self.o[x].xpx.wrapping_add(2);
        }
    }

    /// `$AF79`: along the side of the block.
    fn ai_af79(&mut self, st: &Stage, x: usize) {
        if self.horizontal_collide(st, x, 0x30) {
            self.move_v(x);
            return;
        }
        if self.o[x].dir & 8 != 0 {
            self.o[x].ptr = 0xAF57;
            self.c_step_x(x);
            return;
        }
        self.o[x].ptr = 0xAFC9;
        self.c_step_x(x);
        self.ai_afc9(st, x);
    }

    /// `$AFC9`: along the underside.
    fn ai_afc9(&mut self, st: &Stage, x: usize) {
        if self.up_collide(st, x, 0x2B) {
            self.move_h(x);
            return;
        }
        let o = &mut self.o[x];
        o.dir ^= 0x0F;
        o.ypx = o.ypx.wrapping_sub(2);
        o.ptr = 0xAF79;
    }

    /// `$35B32C` (type `1C`): when Mega Man stands at X $40 the floor piece starts to fall (the PPU buffer it writes only draws).
    fn ai_b32c(&mut self, x: usize) {
        if self.p.xpx != 0x40 {
            return;
        }
        self.col_block = true;
        let buf: Vec<u8> = (0..0x43u16).map(|i| self.ai_byte(0xB480 + i)).collect(); // the ROM template the AI copies into the PPU buffer
        self.ppu_buffer(&buf);
        let o = &mut self.o[x];
        o.ptr = 0xB354;
        o.s[0] = 0x1F;
        self.set_anim(x, 0xAB);
        self.ai_b354(x);
    }

    fn ai_b354(&mut self, x: usize) {
        if self.o[x].s[0] != 0 {
            self.o[x].s[0] -= 1;
            if self.o[x].s[0] != 0 {
                return;
            }
            self.set_anim(x, 0xAC);
        }
        self.move_gravity(x);
        if self.o[x].ypx < 0xD0 {
            return;
        }
        self.despawn(x);
        self.set_anim(x, 0x62);
        self.o[x].ty = 0x80;
        self.o[x].flags &= 0xFB;
    }

    // ---- `6A`: a platform that sinks while Mega Man stands on it ------------------------------------------------------------------

    fn ai_b383(&mut self, x: usize) {
        let d = self.dist_x(x);
        self.carry = d >= 0x60;
        if d >= 0x60 {
            return;
        }
        self.o[x].flags &= 0xFB;
        self.o[x].ptr = 0xB39C;
        self.ai_b39c(x);
    }

    /// `$35B3E1`: is Mega Man (as if standing) touching the object one pixel higher? Returns the carry (true = not touching).
    fn b3e1(&mut self, x: usize) -> bool {
        let y = self.o[x].ypx;
        self.o[x].ypx = y.wrapping_sub(1);
        let anim = self.p.anim;
        self.p.anim = 1;
        let hit = self.touches_player(x);
        self.p.anim = anim;
        self.o[x].ypx = y;
        !hit
    }

    fn ai_b39c(&mut self, x: usize) {
        if self.o[x].aframe != 0x0B {
            return;
        }
        self.o[x].atick = 0;
        if self.b3e1(x) {
            return;
        }
        self.o[x].ptr = 0xB3B7;
        self.ai_b3b7(x);
    }

    fn ai_b3b7(&mut self, x: usize) {
        self.o[x].atick = 0;
        let o = &mut self.o[x];
        o.yvs = 0x80;
        if !self.b3e1(x) {
            let o = &mut self.o[x];
            o.yvp = 0;
            o.dir = 4;
        } else {
            let o = &mut self.o[x];
            o.yvp = 1;
            o.dir = 8;
        }
        self.move_v(x);
    }
}
