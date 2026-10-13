//! The stage-5 boss (AI `$79`, code in bank `$38`: `$AD28`-`$AE3F`) and its children `$7A` (thrown ball, `$AE62`) and `$7B` (the fragments, `$AEB8`).
use super::*;

impl World {
    /// Dispatch for the stage-5 boss states; returns false for an address that is not one of them.
    pub(super) fn run_boss79(&mut self, st: &Stage, x: usize, ptr: u16) -> bool {
        match ptr {
            0xAD28 => {
                self.boss79_pick(x, 3);
            }
            0xAD2C => self.boss79_ad2c(x),
            0xAD4D => self.boss79_ad4d(x),
            0xAD92 => self.boss79_ad92(st, x),
            0xADD6 => self.boss79_add6(x),
            0xAE62 => self.ai_ae62(x),
            0xAEB8 => {
                self.move_h(x);
                self.o[x].flags &= 0xBF;
                self.move_v(x);
            }
            _ => return false,
        }
        true
    }

    /// `$AD34`: start animation `$AE42[y]`; the next state follows from the animation id (`$AE5A/$AE5E` indexed by `aid & 3`).
    fn boss79_pick(&mut self, x: usize, y: u8) {
        let anim = self.b38(0xAE42 + y as u16);
        self.set_anim(x, anim);
        let i = (self.o[x].aid & 3) as u16;
        let p = self.b38(0xAE5A + i) as u16 | (self.b38(0xAE5E + i) as u16) << 8;
        self.o[x].ptr = p;
    }

    /// `$AE2E`: back to idle for 30 frames.
    fn boss79_idle(&mut self, x: usize) {
        self.o[x].s[0] = 0x1E;
        self.o[x].ptr = 0xAD2C;
        self.set_anim(x, 0x12);
    }

    /// `$AD2C`: idle; when the timer runs out face Mega Man and choose the next move (`$B71E`).
    fn boss79_ad2c(&mut self, x: usize) {
        self.o[x].s[0] = self.o[x].s[0].wrapping_sub(1);
        if self.o[x].s[0] != 0 {
            return;
        }
        let y = self.boss_choose(x, true);
        self.boss79_pick(x, y);
    }

    /// `$AD4D`: on animation frame `$18` throw a `$7A` at Mega Man; on frame `$1C` go idle.
    fn boss79_ad4d(&mut self, x: usize) {
        match self.o[x].aframe {
            0x1C => self.boss79_idle(x),
            0x18 => {
                self.o[x].aframe += 1;
                let Some(y) = self.find_slot() else { return };
                let dir = self.o[x].dir;
                self.o[y].dir = dir;
                self.child_at(x, y, (dir & 1) + 0x48, 0x17);
                let d = self.dist_x(x);
                let c = &mut self.o[y];
                c.ty = 0x7A;
                c.hitbox = 0x80;
                c.xvs = 0;
                c.xvp = 4;
                c.s[0] = d >> 2;
            }
            _ => {}
        }
    }

    /// `$AD92`: jump toward Mega Man: frame 1 leaps, frame 2 flies until landing, frame 4 ends.
    fn boss79_ad92(&mut self, st: &Stage, x: usize) {
        match self.o[x].aframe {
            1 => {
                self.o[x].aframe += 1;
                self.face_player(x);
                self.face_flag(x);
                let o = &mut self.o[x];
                o.ypx = o.ypx.wrapping_sub(8);
                o.yvs = 0xBD;
                o.yvp = 7;
            }
            4 => self.boss79_idle(x),
            2 => {
                self.o[x].atick = 0;
                if self.vertical_collide(st, x, 0x0C) {
                    self.o[x].aframe += 1;
                } else {
                    self.horizontal_collide(st, x, 0x14);
                }
            }
            _ => {}
        }
    }

    /// `$ADD6`: wait for the start of the suction animation (`$1C`), then pull Mega Man toward the boss for up to 240 frames.
    fn boss79_add6(&mut self, x: usize) {
        if self.o[x].aid != 0x1C {
            self.o[x].hitbox = 0xA3;
            if self.o[x].aframe == 0 {
                return;
            }
            self.o[x].s[1] = 0xF0;
            self.set_anim(x, 0x1C);
            return;
        }
        self.face_player(x);
        self.face_flag(x);
        self.o[x].s[1] = self.o[x].s[1].wrapping_sub(1);
        let end = if self.o[x].s[1] == 0 {
            true
        } else if self.p.state >= 4 {
            self.o[x].s[1] < 0xE0
        } else {
            let o = &mut self.o[x];
            o.dir ^= 3;
            o.xvs = 0;
            o.xvp = 1;
            self.push_req = Some((0, 1, o.dir)); // code_3FF37C: Mega Man is moved by the boss's speed
            return;
        };
        if end {
            let o = &mut self.o[x];
            o.xvs = 0x80;
            o.xvp = 1;
            o.hitbox = 0xE3;
            self.boss79_idle(x);
        }
    }

    /// AI `$7A` (`$AE62`): flies on; when its timer ends it bursts into four `$7B` pieces.
    fn ai_ae62(&mut self, x: usize) {
        self.o[x].s[0] = self.o[x].s[0].wrapping_sub(1);
        if self.o[x].s[0] != 0 {
            self.move_h(x);
            self.o[x].flags &= 0xBF;
            return;
        }
        self.despawn(x);
        for e in (0..4u16).rev() {
            let Some(y) = self.find_slot() else { return };
            let dir = self.b38(0xAEB4 + e);
            let c = &mut self.o[y];
            c.ty = 0x7B;
            c.hitbox = 0x8F;
            c.xvs = 0xD4;
            c.yvs = 0xD4;
            c.xvp = 2;
            c.yvp = 2;
            c.dir = dir;
            let anim = self.b38(0xAEB0 + e);
            self.child_here(x, y, anim);
        }
    }
}
