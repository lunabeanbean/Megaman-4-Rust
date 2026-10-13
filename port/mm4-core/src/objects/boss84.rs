//! The stage-3 boss (AI `$84`, code in bank `$38`: `$B330`-`$B4FD`) and its children `$85` (bullet, `$B52B`) and `$86` (thrown rock, `$B52E`).
use super::*;

impl World {
    /// Dispatch for the stage-3 boss states; returns false for an address that is not one of them.
    pub(super) fn run_boss84(&mut self, st: &Stage, x: usize, ptr: u16) -> bool {
        match ptr {
            0xB330 => {
                self.o[x].s[3] = 2;
                self.boss84_pick(x, 2);
            }
            0xB33A => self.boss84_idle(x),
            0xB387 => self.boss84_shoot(x),
            0xB3C9 => self.boss84_leap(st, x),
            0xB3F8 => self.boss84_air(st, x),
            0xB52B => self.move_h(x),
            0xB52E => {
                self.move_h(x);
                self.move_v(x);
                if self.o[x].ypg != 0 {
                    self.despawn(x);
                }
            }
            _ => return false,
        }
        true
    }

    /// `$B352`: start animation `$B516[y]`, next state `$B519/$B51C[y]`, vertical speed `$B51F/$B522[y]`, counters `$0480 = $B525[y]`, `$0498 = $B528[y]`, X speed 2.0.
    fn boss84_pick(&mut self, x: usize, y: u8) {
        let k = y as u16;
        let anim = self.b38(0xB516 + k);
        self.set_anim(x, anim);
        let ptr = self.b38(0xB519 + k) as u16 | (self.b38(0xB51C + k) as u16) << 8;
        let (yvs, yvp, s1, s2) = (self.b38(0xB51F + k), self.b38(0xB522 + k), self.b38(0xB525 + k), self.b38(0xB528 + k));
        let o = &mut self.o[x];
        o.ptr = ptr;
        o.yvs = yvs;
        o.yvp = yvp;
        o.s[1] = s1;
        o.s[2] = s2;
        o.xvs = 0;
        o.xvp = 2;
    }

    /// `$B417`: back to idle for 30 frames.
    fn boss84_go_idle(&mut self, x: usize) {
        let o = &mut self.o[x];
        o.ptr = 0xB33A;
        o.s[0] = 0x1E;
        o.s[1] = 0;
        o.s[2] = 0;
        self.set_anim(x, 0x39);
    }

    /// `$B33A`: idle; when the timer runs out choose the next move from the table `$B4FE` (row by distance, column by random number).
    fn boss84_idle(&mut self, x: usize) {
        self.face_player(x);
        self.face_flag(x);
        self.o[x].s[3] = self.o[x].dir;
        self.o[x].s[0] = self.o[x].s[0].wrapping_sub(1);
        if self.o[x].s[0] != 0 {
            return;
        }
        let i = self.boss_choose(x, true);
        let y = self.b38(0xB4FE + i as u16);
        self.boss84_pick(x, y);
    }

    /// `$B387`: shooting pose: on animation frame `$12` a bullet (`$85`) leaves, on frame `$15` the move ends.
    fn boss84_shoot(&mut self, x: usize) {
        self.face_player(x);
        self.face_flag(x);
        match self.o[x].aframe {
            0x12 => {
                self.o[x].aframe += 1;
                let Some(y) = self.find_slot() else { return };
                let dir = self.o[x].dir;
                let c = &mut self.o[y];
                c.ty = 0x85;
                c.hitbox = 0x91;
                c.xvs = 0;
                c.xvp = 6;
                c.dir = dir;
                self.child_at(x, y, 0x2B + (dir & 1), 0x3F);
            }
            0x15 => self.boss84_go_idle(x),
            _ => {}
        }
    }

    /// `$B3C9`: start of a leap: X speed from the distance to Mega Man (`distance / 50` px per frame), then the airborne state in the same frame.
    fn boss84_leap(&mut self, st: &Stage, x: usize) {
        let (d, _) = self.dist_pure(x);
        if d < 0x80 {
            self.o[x].s[1] = 0;
        }
        let (q4, q5) = World::div_fc3a(0, d, 0, 0x32);
        let o = &mut self.o[x];
        o.xvs = q4;
        o.xvp = q5;
        o.ptr = 0xB3F8;
        self.boss84_air(st, x);
    }

    /// `$B476`: while an odd (airborne) animation plays the boss turns to Mega Man: animation `$3D` / `$3B` toggles when his side changed.
    fn boss84_turn(&mut self, x: usize) {
        if self.o[x].aid & 1 == 0 {
            return;
        }
        let (old_dir, old_side) = (self.o[x].dir, self.o[x].s[3]);
        self.face_player(x);
        self.o[x].s[3] = self.o[x].dir;
        if self.o[x].dir != old_side {
            self.o[x].aid = if self.o[x].aid == 0x3D { 0x3B } else { 0x3D };
        }
        self.o[x].dir = old_dir;
    }

    /// `$B3F8`: in the air (jumps `$0498` times); frame 2 of an even animation throws a rock.
    fn boss84_air(&mut self, st: &Stage, x: usize) {
        self.boss84_turn(x);
        if self.vertical_collide(st, x, 0x0A) {
            self.o[x].s[2] = self.o[x].s[2].wrapping_sub(1);
            if self.o[x].s[2] == 0 {
                self.boss84_go_idle(x);
                return;
            }
            let o = &mut self.o[x];
            o.yvs = 0xE5;
            o.yvp = 4;
            o.s[1] = 0x10;
            return;
        }
        if self.horizontal_collide(st, x, 0x32) {
            self.o[x].s[2] = 1;
        } else if self.o[x].s[1] != 0 {
            self.o[x].s[1] -= 1;
            if self.o[x].s[1] == 0 {
                let o = &mut self.o[x];
                o.aid = o.aid.wrapping_add(1);
                o.aframe = 0;
                o.atick = 0;
                return;
            }
        }
        if self.o[x].aid & 1 != 0 {
            return;
        }
        match self.o[x].aframe {
            4 => {
                let o = &mut self.o[x];
                o.aid = o.aid.wrapping_sub(1);
                o.aframe = 0;
                o.atick = 0;
            }
            2 => self.boss84_throw(x),
            _ => {}
        }
    }

    /// `$B4A6`: a rock (`$86`) aimed at Mega Man at 4 px/frame.
    fn boss84_throw(&mut self, x: usize) {
        self.o[x].aframe += 1;
        let Some(y) = self.find_slot() else { return };
        let mut dir = self.o[x].dir;
        let d = self.o[x].aid & 3;
        if d != 0 {
            dir ^= 3;
        }
        let c = &mut self.o[y];
        c.ty = 0x86;
        c.hitbox = 0x8F;
        c.dir = dir;
        self.child_at(x, y, (dir & 1) + d + 0x4C, 0x40);
        let bits = self.aim_fb8c(y, 0x00, 0x04);
        self.o[y].dir |= bits & 0x0C;
    }

    /// code_3FFB8C: speeds for object `x` so that it flies toward Mega Man at `speed` (`sp_hi:sp_lo`) along the longer axis; returns the direction bits (`$0C`).
    pub(super) fn aim_fb8c(&mut self, x: usize, sp_lo: u8, sp_hi: u8) -> u8 {
        let (dx, right) = self.dist_pure(x);
        let mut bits = if right { 1 } else { 2 };
        let o = &self.o[x];
        let (dy, below) = if self.p.ypx >= o.ypx { (self.p.ypx - o.ypx, true) } else { (o.ypx - self.p.ypx, false) };
        bits |= if below { 4 } else { 8 };
        if dy < dx {
            let (q4, q5) = World::div_fc3a(0, dx, sp_lo, sp_hi);
            let (v4, v5) = World::div_fc3a(0, dy, q4, q5);
            let o = &mut self.o[x];
            o.xvs = sp_lo;
            o.xvp = sp_hi;
            o.yvs = v4;
            o.yvp = v5;
        } else {
            let (q4, q5) = World::div_fc3a(0, dy, sp_lo, sp_hi);
            let (v4, v5) = World::div_fc3a(0, dx, q4, q5);
            let o = &mut self.o[x];
            o.yvs = sp_lo;
            o.yvp = sp_hi;
            o.xvs = v4;
            o.xvp = v5;
        }
        bits
    }
}
