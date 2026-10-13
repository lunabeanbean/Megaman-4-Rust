//! Stage 7 enemies in bank `$3B` (`$A0DC`...`$C...`): the wall cannon (list types `2E`/`2F`, AI `$3E`) and its bouncing bullets (`$3F`).
use super::*;

impl World {
    /// Dispatch for the stage-7 states; returns false for an address that is not one of them.
    pub(super) fn run_stage7(&mut self, st: &Stage, x: usize, ptr: u16) -> bool {
        match ptr {
            0xA0DC => self.ai_a0dc(x),
            0xA18D => self.ai_a18d(st, x),
            _ => return self.run_stage7b(st, x, ptr),
        }
        true
    }

    /// code_3FF400: move by the vertical direction flags (down = `F088`, up = `F0C1` with the next probe id); true when it hit something.
    pub(super) fn dir_v_collide(&mut self, st: &Stage, x: usize, id: usize) -> bool {
        let hit = match self.o[x].dir & 0x0C {
            0 => false,
            d if d & 4 != 0 => self.down_collide(st, x, id),
            _ => self.up_collide(st, x, id + 1),
        };
        self.carry = hit;
        hit
    }

    /// AI `$3E` (`$3BA0DC`): the cannon waits a random time, and when Mega Man is within 32 px vertically (and fewer than three of its bullets
    /// exist) it plays its shooting animation; on tick 3 of animation frame 2 it fires a bullet.
    fn ai_a0dc(&mut self, x: usize) {
        if self.o[x].aframe == 0 {
            self.o[x].atick = 0;
            if self.o[x].s[0] != 0 {
                self.o[x].s[0] -= 1;
                if self.o[x].s[0] != 0 {
                    return;
                }
            }
            // ADC with the carry the AI loop left
            let sum = self.rng[0] as u16 + self.rng[2] as u16 + self.carry as u16;
            self.rng[2] = sum as u8;
            let k = (self.rng[2] & 7) as u16;
            self.o[x].s[0] = self.ai_byte(0xA185 + k);
            let dy = self.p.ypx.wrapping_sub(self.o[x].ypx);
            let dy = if self.p.ypx < self.o[x].ypx { (!dy).wrapping_add(1) } else { dy };
            if dy >= 0x20 {
                return;
            }
            let count = (8..=0x17).filter(|&s| self.o[s].ptr >> 8 != 0 && self.o[s].ty == 0x3F).count();
            if count >= 3 {
                return;
            }
            self.o[x].aframe += 1;
        }
        match self.o[x].aframe {
            1 => {}
            2 => {
                if self.o[x].atick == 3 {
                    self.a0dc_shoot(x);
                }
            }
            _ => {
                self.o[x].aframe = 0;
                self.o[x].atick = 0;
            }
        }
    }

    /// `$3BA13A`: a bullet that arcs away from the cannon (up and to the side it faces).
    fn a0dc_shoot(&mut self, x: usize) {
        let Some(y) = self.find_slot() else { return };
        let right = self.o[x].flags & 0x40 != 0;
        let c = &mut self.o[y];
        c.ty = 0x3F;
        c.hitbox = 0xCE;
        c.hp = 1;
        c.yvs = 0x6A;
        c.xvs = 0x6A;
        c.yvp = 1;
        c.xvp = 1;
        c.dir = if right { 0x09 } else { 0x0A };
        let idx = 0x29 + (self.o[y].dir & 1);
        self.child_at(x, y, idx, 0x65);
    }

    /// AI `$3F` (`$3BA18D`): the bullet flies diagonally, bounces off floor/ceiling (flipping the vertical direction) and walls (turning
    /// around), and gets faster on every bounce until its speed reaches 5 px/frame.
    fn ai_a18d(&mut self, st: &Stage, x: usize) {
        if self.o[x].aid == 0x66 && self.o[x].aframe != 0 {
            self.set_anim(x, 0x65);
        }
        if self.dir_v_collide(st, x, 0x16) {
            self.set_anim(x, 0x66);
            self.o[x].dir ^= 0x0C;
        } else {
            if !self.horizontal_collide(st, x, 0x20) {
                return;
            }
            self.turn_around(x);
        }
        if self.o[x].xvp >= 5 {
            return;
        }
        let (s, c) = self.o[x].xvs.overflowing_add(0x66);
        let p = self.o[x].xvp.wrapping_add(c as u8);
        let o = &mut self.o[x];
        o.xvs = s;
        o.yvs = s;
        o.xvp = p;
        o.yvp = p;
    }
}
