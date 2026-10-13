//! Stage 3 enemies (bank `$3A` code `$90ED`-`$9D48`, bank `$3B` `$B198`...).
use super::*;

impl World {
    /// Dispatch for the stage-3 enemy states; returns false for an address that is not one of them.
    pub(super) fn run_stage3(&mut self, st: &Stage, x: usize, ptr: u16) -> bool {
        match ptr {
            0x93AF => self.ai_93af(x),
            0x942B => self.ai_942b(st, x),
            0x9470 => self.ai_9470(st, x),
            0x9CA2 => {
                self.face_player(x);
                self.ai_9d48(x);
                self.o[x].s[0] = 0x28;
                self.o[x].ptr = 0x9CB7;
                self.ai_9cb7(x); // no RTS: the first state runs straight into the main one
            }
            0x9CB7 => self.ai_9cb7(x),
            0x9D69 => self.move_v(x),
            0x90ED => self.ai_90ed(x),
            0x9124 => self.ai_9124(x),
            0x9155 => self.ai_9155(st, x),
            0x91DC => self.move_h(x),
            _ => return false,
        }
        true
    }

    /// AI `$15` (`$3A93AF`, list type `03`): every ~0x22C frames it releases a bouncing part (type `$14`) from the bottom of the screen.
    fn ai_93af(&mut self, x: usize) {
        if self.w13c != 0 {
            return;
        }
        if self.o[x].s[1] != 0 {
            let o = &mut self.o[x];
            let (v, b) = o.s[0].overflowing_sub(1);
            o.s[0] = v;
            o.s[1] = o.s[1].wrapping_sub(b as u8);
            if o.s[1] != 0 {
                return;
            }
        }
        let Some(y) = self.find_slot() else { return };
        self.o[y].ty = 0x14;
        self.child_here(x, y, 0x15);
        let r = (self.rng[3] as u16 + self.rng[0] as u16) as u8 & 1; // FB6C left the carry clear
        let c = &mut self.o[y];
        c.flags |= 0x20;
        c.ypx = 0xD8;
        c.hitbox = 0x95;
        c.hp = 4;
        c.dir = 0x08;
        c.yvs = 0;
        c.xvs = 0;
        c.yvp = 1;
        c.xvp = 1 + r;
        c.s[1] = 3;
        self.o[x].s[1] = 2;
        self.o[x].s[0] = 0x2C;
    }

    /// `$949B`: after the part has appeared it throws three sparks (type `$80`, animation `$48`), one every 8 frames.
    fn spark_trail(&mut self, x: usize) {
        if self.o[x].s[1] == 0 {
            return;
        }
        if self.o[x].s[2] != 0 {
            self.o[x].s[2] -= 1;
            return;
        }
        self.o[x].s[1] -= 1;
        self.o[x].s[2] = 8;
        let Some(y) = self.find_slot() else { return };
        self.o[y].ty = 0x80;
        self.child_here(x, y, 0x48);
        let k = self.o[x].s[1] as u16;
        let (dy, dx) = (self.ai_byte(0x94DC + k), self.ai_byte(0x94DF + k));
        self.o[y].ypx = 0xC0u8.wrapping_sub(dy);
        self.o[y].xpx = self.o[y].xpx.wrapping_add(dx);
    }

    /// AI `$14` first state (`$3A942B`): the part rises from the bottom edge; once it is high enough it picks a side-to-side run,
    /// below `$D0` going down it vanishes.
    fn ai_942b(&mut self, st: &Stage, x: usize) {
        self.spark_trail(x);
        self.move_v(x);
        if self.o[x].dir & 8 == 0 {
            if self.o[x].ypx >= 0xD0 {
                self.despawn(x);
            }
            return;
        }
        if self.o[x].ypx >= 0xB0 {
            return;
        }
        self.o[x].ptr = 0x9470;
        self.face_player(x);
        self.face_flag(x);
        let k = ((self.rng[2] as u16 + self.rng[0] as u16 + self.carry as u16) as u8 & 3) as u16;
        self.o[x].s[3] = self.ai_byte(0x94E2 + k);
        self.set_anim(x, 0x14);
        self.o[x].hitbox = 0xD5;
        self.ai_9470(st, x);
    }

    /// `$3A9470`: slide toward Mega Man until a wall or the timer stops it, then rise/fall again.
    fn ai_9470(&mut self, st: &Stage, x: usize) {
        if !self.horizontal_collide(st, x, 0x14) {
            self.o[x].s[3] = self.o[x].s[3].wrapping_sub(1);
            if self.o[x].s[3] != 0 {
                return;
            }
        }
        let o = &mut self.o[x];
        o.dir = 4;
        o.ptr = 0x942B;
        o.s[1] = 3;
        o.hitbox = 0x95;
        self.set_anim(x, 0x15);
    }

    /// `$3A9D48`: horizontal speed from the facing (left 2.5, right 4.0 px/frame).
    fn ai_9d48(&mut self, x: usize) {
        let y = (self.o[x].dir & 1) as u16;
        self.o[x].xvs = self.ai_byte(0x9D65 + y);
        self.o[x].xvp = self.ai_byte(0x9D67 + y);
    }

    /// AI `$33` main state (`$3A9CB7`, list type `13`): patrols sideways, turns at the screen edges and drops bursts of 1-3 bombs (type `$34`).
    fn ai_9cb7(&mut self, x: usize) {
        self.move_h(x);
        let y = self.o[x].xpx.wrapping_sub(self.cam_now.lo);
        let right = self.o[x].dir & 1 != 0;
        let (mut carry, reverse) = if right { (y >= 0xF0, y >= 0xF0) } else { (y >= 0x10, y < 0x10) };
        if reverse {
            let s3 = self.o[x].s[3];
            carry = s3 >= 2;
            if s3 != 2 {
                self.o[x].s[3] = s3.wrapping_add(1);
                self.turn_around(x);
                self.ai_9d48(x);
            }
        }
        if self.o[x].s[0] != 0 {
            self.o[x].s[0] -= 1;
            if self.o[x].s[0] != 0 {
                return;
            }
            let sum = self.rng[2] as u16 + self.rng[1] as u16 + carry as u16;
            self.rng[2] = sum as u8;
            self.o[x].s[1] = self.ai_byte(0x9D5B + (self.rng[2] & 7) as u16);
        } else {
            self.o[x].s[2] = self.o[x].s[2].wrapping_sub(1);
            if self.o[x].s[2] != 0 {
                return;
            }
        }
        let Some(y) = self.find_slot() else { return };
        self.o[y].ty = 0x34;
        self.o[y].yvs = 0;
        self.o[y].yvp = 2;
        self.child_here(x, y, 0x57);
        let c = &mut self.o[y];
        c.ypx = c.ypx.wrapping_add(0x0C);
        c.dir = 4;
        c.hitbox = 0x8F;
        self.o[x].s[2] = self.ai_byte(0x9D63 + (self.o[x].dir & 1) as u16);
        self.o[x].s[1] = self.o[x].s[1].wrapping_sub(1);
        if self.o[x].s[1] == 0 {
            self.o[x].s[0] = 0x3C;
        }
    }

    /// code_3FF95D with the object one pixel higher (`DEC $0378,x` ... `INC $0378,x`): is Mega Man standing on / touching its top?
    fn touches_raised(&mut self, x: usize) -> bool {
        self.o[x].ypx = self.o[x].ypx.wrapping_sub(1);
        let t = self.touches_player(x);
        self.o[x].ypx = self.o[x].ypx.wrapping_add(1);
        t
    }

    /// `$3A91A7`: throw part `e` (0 = effect `$80`, 1/2 = flying pieces `$39` to the right/left) from the object.
    fn spawn_91a7(&mut self, x: usize, e: u8) {
        let Some(y) = self.find_slot() else { return };
        let k = e as u16;
        self.o[y].hitbox = 0x8F;
        self.o[y].hp = 0;
        self.o[y].dir = e;
        self.o[y].ty = self.ai_byte(0x91A4 + k);
        let (idx, anim) = (self.ai_byte(0x919E + k), self.ai_byte(0x91A1 + k));
        self.child_at(x, y, idx, anim);
        self.o[y].xvs = 0;
        self.o[y].xvp = 2;
    }

    /// AI `$12` first state (`$3A90ED`, list type `01`): waits; when Mega Man touches it, it ducks (state `$9124`); on animation frame 3 it throws two pieces.
    fn ai_90ed(&mut self, x: usize) {
        if self.touches_raised(x) {
            self.o[x].ptr = 0x9124;
            self.o[x].s[0] = 0x0E;
            self.set_anim(x, 8);
            return;
        }
        if self.o[x].aframe == 3 && self.o[x].atick == 0 {
            self.spawn_91a7(x, 2);
            self.spawn_91a7(x, 1);
        }
    }

    /// `$3A9124`: bob up and down for 14 frames (a piece flies off at the 8th), then become the rolling platform.
    fn ai_9124(&mut self, x: usize) {
        self.o[x].s[0] = self.o[x].s[0].wrapping_sub(1);
        let a = self.o[x].s[0];
        if a == 0 {
            self.o[x].ptr = 0x9155;
            self.o[x].dir = 1;
        } else if a < 8 {
            self.o[x].ypx = self.o[x].ypx.wrapping_sub(1);
            self.set_anim(x, 6);
        } else {
            if a == 8 {
                self.spawn_91a7(x, 0);
            }
            self.o[x].ypx = self.o[x].ypx.wrapping_add(1);
        }
    }

    /// `$3A9155`: the rolling platform: falls to the ground, carries Mega Man standing on it and breaks up against a wall.
    fn ai_9155(&mut self, st: &Stage, x: usize) {
        let standing = self.touches_raised(x);
        self.o[x].s[1] = !standing as u8; // LDA #0 / STA $0480,x ... ROL $0480,x: the carry of the touch test (set = no contact)
        let landed = self.vertical_collide(st, x, 0x13);
        if standing {
            let y = self.o[x].ypx.wrapping_sub(0x17);
            self.player_ops.push(PlayerOp::Ground(y));
            *self.stats.entry(0xF3C1).or_insert(0) += 1; // coverage: code_3FF3C1
        }
        if !landed {
            return;
        }
        if standing {
            self.carry_player(x);
        }
        if self.horizontal_collide(st, x, 0x14) {
            self.o[x].ty = 0x80;
            self.o[x].ptr &= 0x00FF;
            self.set_anim(x, 0x62);
        }
    }

    /// code_3FF32F: the object carries Mega Man along with its own movement.
    pub(super) fn carry_player(&mut self, x: usize) {
        if self.p.state == 5 {
            return;
        }
        let o = self.o[x];
        if o.dir == 0 || (o.xvs | o.xvp) == 0 {
            return;
        }
        let mut xsub = None;
        if o.ty != 0x52 {
            let dist = self.dist_pure(x).0;
            let half = self.rom.cpu_byte(0x3A, 0x3B, 0xF9F4 + (o.hitbox & 0x3F) as u16);
            if half.wrapping_sub(dist) < 4 {
                return;
            }
            xsub = Some(if o.dir & 1 != 0 { o.xsub.wrapping_sub(o.xvs) } else { o.xsub.wrapping_add(o.xvs) });
        }
        *self.stats.entry(0xF32F).or_insert(0) += 1; // coverage: code_3FF32F carries
        self.player_ops.push(PlayerOp::Carry { xsub, xvs: o.xvs, xvp: o.xvp, dir: o.dir });
    }
}
