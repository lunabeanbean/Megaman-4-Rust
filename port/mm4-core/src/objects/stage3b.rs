//! Stage 3 enemies in bank `$3B` (`$B198`...): the hiding shooter (list type `67`, AI `$9A`) and its bullets (`$9B`).
use super::*;

impl World {
    /// Dispatch for these states; returns false for an address that is not one of them.
    pub(super) fn run_stage3b(&mut self, x: usize, ptr: u16) -> bool {
        match ptr {
            0xB198 => self.ai_b198(x),
            0xB1D7 => self.ai_b1d7(x),
            0xB29D => {
                self.move_h(x);
                self.move_v(x);
            }
            _ => return false,
        }
        true
    }

    /// AI `$9A` resting state (`$3BB198`): hidden until Mega Man is within 128 px, then it pops up; once the pop-up animation is over it starts shooting.
    fn ai_b198(&mut self, x: usize) {
        if self.o[x].s[0] != 0 {
            self.o[x].s[0] -= 1;
            return;
        }
        if self.o[x].flags & 4 == 0 {
            if self.o[x].aframe >= 6 {
                self.o[x].ptr = 0xB1D7;
            }
            return;
        }
        if self.dist_x(x) >= 0x80 {
            return;
        }
        let o = &mut self.o[x];
        o.flags &= !4;
        o.hitbox = 0xD5;
        o.aframe = 0;
        o.atick = 0;
    }

    /// AI `$9A` attack state (`$3BB1D7`): face Mega Man; on animation frame `$15` fire a bullet at him, on `$1F` wait 64 frames, on `$26` hide again.
    fn ai_b1d7(&mut self, x: usize) {
        self.face_player(x);
        self.face_flag(x);
        if self.o[x].s[1] != 0 {
            self.o[x].atick = 0;
            self.o[x].s[1] -= 1;
            if self.o[x].s[1] != 0 {
                return;
            }
            self.o[x].aframe = self.o[x].aframe.wrapping_add(1);
        }
        if self.o[x].atick != 0 {
            return;
        }
        match self.o[x].aframe {
            0x15 => self.b1d7_shoot(x),
            0x1F => self.o[x].s[1] = 0x40,
            0x26 => {
                let o = &mut self.o[x];
                o.flags |= 4;
                o.hitbox = 0x95;
                o.ptr = 0xB198;
                o.s[0] = 0x1E;
            }
            _ => {}
        }
    }

    /// `$3BB226`: a bullet (type `$9B`) aimed along the 16-way heading to Mega Man; the speeds come from the tables at `$B27D`-`$B295`.
    fn b1d7_shoot(&mut self, x: usize) {
        let h = self.heading_to_player(x);
        let k = (h >> 1) as u16;
        let bits = self.ai_byte(0xF796 + h as u16) & 0x0C;
        self.o[x].dir |= bits;
        let Some(y) = self.find_slot() else { return };
        self.o[y].ty = 0x9B;
        self.o[y].hitbox = 0x8E;
        self.o[y].hp = 3;
        self.o[y].dir = self.o[x].dir;
        let idx = 0x33 + (self.o[x].dir & 1);
        self.child_at(x, y, idx, 0x99);
        let (yvs, yvp, xvs, xvp) = (self.ai_byte(0xB28D + k), self.ai_byte(0xB295 + k), self.ai_byte(0xB27D + k), self.ai_byte(0xB285 + k));
        let c = &mut self.o[y];
        c.yvs = yvs;
        c.yvp = yvp;
        c.xvs = xvs;
        c.xvp = xvp;
    }
}
