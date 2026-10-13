//! Stage 13: the boss `6F` (AI `$AB`, bank `$35` `$A487`/`$A4FB`/`$A51C`) with its shots `$AD` (`$A5A8`) and `$AE` (`$A5BD`), and the moving platform `71` (AI `$AF`,
//! `$A5C0`/`$A5FB`/`$A64C`/`$A67B`).
use super::*;

impl World {
    pub(super) fn run_stage13(&mut self, st: &Stage, x: usize, ptr: u16) -> bool {
        if self.f6 != 0x35 {
            return false;
        }
        match ptr {
            0xA487 => self.boss6f_init(x),
            0xA4FB => self.boss6f_meter(x),
            0xA51C => self.boss6f_a51c(x),
            0xA5A8 => self.shot_a5a8(st, x),
            0xA5BD => self.left(x),
            0xA5C0 => self.plat_a5c0(x),
            0xA5FB => self.plat_a5fb(x),
            0xA64C => self.plat_a64c(x),
            0xA67B => self.plat_a67b(x),
            _ => return false,
        }
        true
    }

    fn boss6f_init(&mut self, x: usize) {
        if self.fd9c(x) != 9 {
            return;
        }
        self.o[x].s[0] = self.o[x].s[0].wrapping_add(1);
        if self.o[x].s[0] & 7 != 0 {
            return;
        }
        let y = self.o[x].s[1].wrapping_add(4);
        self.o[x].s[1] = y;
        if y != 0x0C {
            return;
        }
        let o = &mut self.o[x];
        o.s[0] = 0x50;
        o.s[1] = 0;
        o.ptr = 0xA4FB;
        o.flags = 0x90;
        self.p95 = 0;
        self.bf = 0;
        self.meter2 = 0x8F;
        self.o[20].flags = 0x91;
        self.o[19].flags = 0x91;
    }

    fn boss6f_meter(&mut self, x: usize) {
        if self.p95 & 7 != 0 {
            return;
        }
        self.snd(0x29, 0xA506); // code_35A503
        self.carry = true;
        self.bf = self.bf.wrapping_add(1);
        if self.bf != 0x1C {
            return;
        }
        self.o[x].ptr = 0xA51C;
        self.player_cmd = Some(0);
        self.p.state = 0;
        self.boss6f_a51c(x);
    }

    /// `$A51C`: every 80 frames one shot: a flat one `AE` (slow, from the right) or an aimed one `AD` that falls from the top.
    fn boss6f_a51c(&mut self, x: usize) {
        self.o[x].s[0] = self.o[x].s[0].wrapping_sub(1);
        if self.o[x].s[0] != 0 {
            return;
        }
        self.o[x].s[0] = 0x50;
        let Some(y) = self.find_slot() else {
            self.carry = true;
            return;
        };
        self.carry = false;
        let sum = self.rng[0] as u16 + self.rng[2] as u16 + self.carry as u16;
        self.rng[2] = sum as u8;
        self.carry = sum > 0xFF;
        if sum as u8 & 1 == 0 {
            self.child_here(x, y, 0x9D);
            let c = &mut self.o[y];
            c.ypx = 0x6C;
            c.xpx = 0x94;
            c.ty = 0xAE;
            c.hitbox = 0x9A;
            c.xvs = 0;
            c.xvp = 2;
            return;
        }
        self.child_here(x, y, 0x9C);
        let c = &mut self.o[y];
        c.ypx = 0x24;
        c.xpx = 0xAC;
        c.ty = 0xAD;
        c.hitbox = 0x8D;
        c.yvs = 0;
        c.yvp = 4;
        let dx = self.dist_x(y);
        let sp = if self.p.ypx >= 0x68 { 0x3A } else { 0x28 };
        let (q4, q5) = World::div_fc3a(0, dx, 0, sp);
        let c = &mut self.o[y];
        c.xvs = q4;
        c.xvp = q5;
    }

    fn shot_a5a8(&mut self, st: &Stage, x: usize) {
        if self.vertical_collide(st, x, 0x11) {
            self.despawn(x);
            self.set_anim(x, 0x62);
            self.o[x].ty = 0x3B;
        } else {
            self.left(x);
        }
    }

    // ---- platform `71` ---------------------------------------------------------------------------------------------------------------

    /// `$A6D1`: if Mega Man stands on the platform (touching it one pixel higher) he is carried by `$0F` pixels sideways.
    fn plat_carry(&mut self, x: usize, d: u8) {
        let y = self.o[x].ypx;
        self.o[x].ypx = y.wrapping_sub(1);
        let hit = self.touches_player(x);
        self.o[x].ypx = y;
        self.carry = !hit;
        if hit {
            let nx = self.p.xpx.wrapping_add(d);
            self.p.xpx = nx;
            self.player_ops.push(PlayerOp::SetX(nx));
        }
    }

    fn plat_a5c0(&mut self, x: usize) {
        if self.o[x].xpx >= 0x40 {
            self.o[x].s[0] = 4;
        }
        if self.p95 & 1 != 0 {
            return;
        }
        self.o[x].ypx = self.o[x].ypx.wrapping_add(1);
        let k = self.o[x].s[0] as u16;
        if self.o[x].ypx != self.ai_byte(0xA70F + k) || self.o[x].xpx != self.ai_byte(0xA717 + k) {
            return;
        }
        let o = &mut self.o[x];
        o.s[0] = o.s[0].wrapping_add(1);
        o.ptr = 0xA5FB;
        o.s[1] = 0;
    }

    fn plat_a5fb(&mut self, x: usize) {
        if self.p95 & 1 != 0 {
            return;
        }
        let y = self.o[x].s[1] as u16;
        self.o[x].s[1] = self.o[x].s[1].wrapping_add(2);
        let (dy, dx) = (self.ai_byte(0xA71F + y), self.ai_byte(0xA720 + y));
        self.o[x].ypx = self.o[x].ypx.wrapping_add(dy);
        self.o[x].xpx = self.o[x].xpx.wrapping_add(dx);
        self.plat_carry(x, dx);
        let k = self.o[x].s[0] as u16;
        if self.o[x].xpx != self.ai_byte(0xA717 + k) || self.o[x].ypx != self.ai_byte(0xA70F + k) {
            return;
        }
        let o = &mut self.o[x];
        o.s[0] = o.s[0].wrapping_add(1);
        o.ptr = 0xA64C;
        o.s[1] = 0;
    }

    fn plat_a64c(&mut self, x: usize) {
        if self.p95 & 1 != 0 {
            return;
        }
        self.o[x].ypx = self.o[x].ypx.wrapping_sub(1);
        let k = self.o[x].s[0] as u16;
        if self.o[x].ypx != self.ai_byte(0xA70F + k) || self.o[x].xpx != self.ai_byte(0xA717 + k) {
            return;
        }
        let o = &mut self.o[x];
        o.s[0] = o.s[0].wrapping_add(1);
        o.ptr = 0xA67B;
        o.s[1] = 0;
    }

    fn plat_a67b(&mut self, x: usize) {
        if self.p95 & 1 != 0 {
            return;
        }
        let y = self.o[x].s[1] as u16;
        self.o[x].s[1] = self.o[x].s[1].wrapping_add(2);
        let (dy, dx) = (self.ai_byte(0xA759 + y), self.ai_byte(0xA75A + y));
        self.o[x].ypx = self.o[x].ypx.wrapping_add(dy);
        self.o[x].xpx = self.o[x].xpx.wrapping_add(dx);
        self.plat_carry(x, dx);
        let k = self.o[x].s[0] as u16;
        if self.o[x].xpx != self.ai_byte(0xA717 + k) || self.o[x].ypx != self.ai_byte(0xA70F + k) {
            return;
        }
        let o = &mut self.o[x];
        o.ptr = 0xA5C0;
        o.s[0] &= 0xFC;
        o.s[1] = 0;
    }
}
