//! The stage-12 boss (type `$A6` = list `6C`, bank `$35` `$A02D`-`$A330`; part `6D` in slot 22, rocks `$AA` = `$A42A`/`$A458`). Slot 22 is addressed absolutely.
use super::*;

impl World {
    pub(super) fn run_boss6c(&mut self, x: usize, ptr: u16) -> bool {
        if self.f6 != 0x35 {
            return false;
        }
        match ptr {
            0xA02D => self.boss6c_init(x),
            0xA046 => self.boss6c_wait(x),
            0xA07B => self.boss6c_fade(x),
            0xA0B4 => self.boss6c_meter(x),
            0xA0D5 => self.boss6c_a0d5(x),
            0xA127 => self.boss6c_a127(x),
            0xA140 => self.boss6c_a140(x),
            0xA190 => self.boss6c_a190(x),
            0xA203 => self.boss6c_a203(x),
            0xA42A => self.rock_a42a(x),
            0xA458 => self.rock_a458(x),
            _ => return false,
        }
        true
    }

    fn boss6c_init(&mut self, x: usize) {
        if self.fd9c(x) != 9 {
            return;
        }
        self.p1c = 0x15;
        self.p1b = 0x80;
        self.o[x].ptr = 0xA046;
        self.boss6c_wait(x);
    }

    fn boss6c_wait(&mut self, x: usize) {
        if self.p1b != 0 {
            return;
        }
        self.p95 = 0;
        self.bf = 0;
        self.o[x].s[2] = self.o[x].s[2].wrapping_add(1);
        self.o[x].ptr = 0xA07B;
        self.fa = 8;
        self.scroll6a = 2;
        self.f1 = 0xC0;
        self.f0 = 9;
        self.o[22].flags = 0x90;
        self.meter2 = 0x8F;
        self.o[x].s[0] = 0x30;
    }

    fn boss6c_fade(&mut self, x: usize) {
        if self.p95 & 7 != 0 {
            return;
        }
        let (a, b) = self.o[x].s[0].overflowing_sub(0x10);
        self.o[x].s[0] = a;
        if !b {
            return;
        }
        self.o[x].s[0] = 0;
        self.o[x].ptr = 0xA0B4;
    }

    fn boss6c_meter(&mut self, x: usize) {
        if self.p95 & 7 != 0 {
            return;
        }
        self.snd(0x29, 0xA0BF); // code_35A0BC
        self.carry = true;
        self.bf = self.bf.wrapping_add(1);
        if self.bf != 0x1C {
            return;
        }
        self.o[x].ptr = 0xA0D5;
        self.player_cmd = Some(0);
        self.p.state = 0;
        self.boss6c_a0d5(x);
    }

    /// `$A331`: a hidden boss (flag bit 2) restores the palette (only the hit timer is game state).
    fn boss6c_a331(&mut self, x: usize) {
        if self.o[x].flags & 4 != 0 {
            self.o[x].t5b8 = 0;
        }
    }

    /// `$A0D5`: the boss builds itself up (6 steps of 4 frames); then it waits for its next move.
    fn boss6c_a0d5(&mut self, x: usize) {
        if self.o[x].s[0] != 0 {
            self.o[x].s[0] -= 1;
            if self.o[x].s[0] != 0 {
                return;
            }
        }
        let a = self.o[x].s[1];
        self.o[x].s[1] = a.wrapping_add(1);
        if a & 3 != 0 {
            return;
        }
        let y = self.o[x].s[2] as u16;
        self.fa = self.ai_byte(0xA340 + y);
        self.o[22].ypx = self.ai_byte(0xA346 + y);
        let fl = self.ai_byte(0xA359 + y);
        self.o[x].flags = (self.o[x].flags & 0xFB) | fl;
        self.col_block = true;
        self.carry = true; // the last CMP #$41 of the PPU buffer code ($A27F)
        self.boss6c_a331(x);
        self.o[x].s[2] = self.o[x].s[2].wrapping_add(1);
        if y != 5 {
            return;
        }
        self.o[x].s[2] = 4;
        self.o[x].ptr = 0xA127;
        self.o[x].hitbox |= 0x40;
    }

    fn boss6c_a127(&mut self, x: usize) {
        let sum = self.rng[0] as u16 + self.rng[1] as u16 + self.carry as u16;
        self.rng[0] = sum as u8;
        self.carry = sum > 0xFF;
        let y = (sum as u8 & 1) as u16;
        self.o[x].s[0] = self.ai_byte(0xA354 + y);
        self.o[x].ptr = 0xA140;
        self.boss6c_a140(x);
    }

    /// `$A140`: pause, then jump toward Mega Man (speed by distance), dropping a dust cloud.
    fn boss6c_a140(&mut self, x: usize) {
        self.o[x].s[0] = self.o[x].s[0].wrapping_sub(1);
        if self.o[x].s[0] != 0 {
            return;
        }
        let d = self.dist_x(x);
        let mut y = 2u16;
        while y != 0 && d < self.ai_byte(0xA356 + y) {
            y -= 1;
        }
        let (a, b, c, e) = (self.ai_byte(0xA37B + y), self.ai_byte(0xA37E + y), self.ai_byte(0xA381 + y), self.ai_byte(0xA384 + y));
        let o = &mut self.o[x];
        o.xvs = a;
        o.xvp = b;
        o.yvs = c;
        o.yvp = e;
        o.ptr = 0xA190;
        self.face_player(x);
        if let Some(y) = self.find_slot() {
            self.child_here(x, y, 0x7E);
            let c = &mut self.o[y];
            c.ypx = 0xB4;
            c.ty = 1;
            c.hitbox = 0;
        }
        self.boss6c_a190(x);
    }

    /// `$A190`: the jump: sideways with limits, gravity; on landing (Y `$AB`) it shakes the room and freezes Mega Man.
    fn boss6c_a190(&mut self, x: usize) {
        self.move_h(x);
        self.o[x].flags &= 0xBF;
        if self.o[x].dir & 1 != 0 {
            if 0xD8 < self.o[x].xpx {
                self.o[x].xpx = 0xD8;
            }
        } else if 0x28 >= self.o[x].xpx {
            self.o[x].xpx = 0x28;
        }
        self.move_gravity(x);
        if 0xAB < self.o[x].ypx {
            self.o[x].ypx = 0xAB;
            self.o[x].ptr = 0xA203;
            self.set_anim(x, 0x7C);
            self.o[x].s[0] = 0x5A;
            if self.p.state == 0 || self.p.state == 2 {
                self.player_ops.push(PlayerOp::Freeze(0x5A));
            }
        }
        // $A1E3: the scroll registers and the part follow the boss
        let bx = self.o[x].xpx;
        self.scroll69 = 0xB8u8.wrapping_sub(bx);
        self.fa = 0xD9u8.wrapping_sub(self.o[x].ypx);
        self.o[22].xpx = bx;
        self.o[22].ypx = self.o[x].ypx.wrapping_sub(0x26);
    }

    /// `$A203`: the landing: the screen shakes for 90 frames, then rocks fall (`$A2CA`) and the room settles.
    fn boss6c_a203(&mut self, x: usize) {
        if self.o[x].s[0] != 0 {
            let old = self.o[x].s[0];
            self.o[x].s[0] = old - 1;
            if old >= 0x1E {
                self.f1 = 0xBE + ((old & 4) >> 1);
                return;
            }
            if self.o[x].aid == 0x7B {
                return;
            }
            self.f1 = 0xC0;
            self.set_anim(x, 0x7B);
            self.boss6c_a2ca(x);
            return;
        }
        self.o[x].hitbox &= 0xBF;
        let a = self.o[x].s[1];
        self.o[x].s[1] = a.wrapping_add(1);
        if a & 3 != 0 {
            return;
        }
        let y = self.o[x].s[2] as u16;
        self.fa = self.ai_byte(0xA340 + y);
        self.o[22].ypx = self.ai_byte(0xA346 + y);
        let fl = self.ai_byte(0xA359 + y);
        self.o[x].flags = (self.o[x].flags & 0xFB) | fl;
        self.col_block = true;
        self.carry = true; // the last CMP #$41 of the PPU buffer code ($A27F)
        self.boss6c_a331(x);
        self.o[x].s[2] = self.o[x].s[2].wrapping_sub(1);
        if self.o[x].s[2] & 0x80 == 0 {
            return;
        }
        self.o[x].s[2] = 1;
        self.o[x].ptr = 0xA0D5;
        let sum = self.rng[0] as u16 + self.rng[1] as u16 + self.carry as u16;
        self.rng[0] = sum as u8;
        self.carry = sum > 0xFF;
        let y = (sum as u8 & 1) as u16;
        self.o[x].s[0] = self.ai_byte(0xA352 + y);
    }

    /// `$A2CA`: four rocks `$AA` fall from the ceiling.
    fn boss6c_a2ca(&mut self, x: usize) {
        let sum = self.rng[0] as u16 + self.rng[1] as u16 + self.carry as u16;
        self.rng[0] = sum as u8;
        self.carry = sum > 0xFF;
        let k = self.ai_byte(0xA377 + (sum as u8 & 3) as u16);
        let (mut e1, mut e2) = (k as i16, 3i16);
        loop {
            let Some(y) = self.find_slot() else { return };
            self.child_here(x, y, 0x86);
            let px = self.ai_byte(0xA35F + e1 as u16);
            let s0 = self.ai_byte(0xA36B + e2 as u16);
            let dir = self.ai_byte(0xA36F + e2 as u16);
            let fl = self.ai_byte(0xA373 + e2 as u16);
            let c = &mut self.o[y];
            c.xpx = px;
            c.s[0] = s0;
            c.dir = dir;
            c.flags = fl;
            c.ty = 0xAA;
            c.ypx = 0;
            c.hp = 1;
            c.hitbox = 0x80;
            c.xvs = 0;
            c.yvs = 0;
            c.yvp = 0;
            c.xvp = 1;
            e1 -= 1;
            e2 -= 1;
            if e2 < 0 {
                break;
            }
        }
    }

    fn rock_a42a(&mut self, x: usize) {
        if self.o[x].s[0] != 0 {
            self.o[x].s[0] -= 1;
            if self.o[x].s[0] != 0 {
                return;
            }
            self.o[x].flags &= 0xFB;
        }
        self.move_gravity(x);
        self.o[x].atick = 0;
        if 0xB9 >= self.o[x].ypx {
            return;
        }
        self.o[x].ypx = 0xB9;
        self.o[x].ptr = 0xA458;
        self.rock_a458(x);
    }

    fn rock_a458(&mut self, x: usize) {
        if self.o[x].aid == 0x89 {
            self.move_h(x);
            return;
        }
        let f = self.o[x].aframe;
        if f == 4 {
            self.set_anim(x, 0x89);
            self.move_h(x);
            return;
        }
        if f != 1 || self.o[x].atick != 4 {
            return;
        }
        self.o[x].ypx = self.o[x].ypx.wrapping_sub(3);
        self.o[x].hitbox = 0xC0;
    }
}
