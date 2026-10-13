//! The stage-7 boss (AI `$71`, code in bank `$38`: `$A800`-`$AA05`; defeat and victory reuse the code of the other bosses).
use super::*;

impl World {
    /// Dispatch for the stage-7 boss states; returns false for an address that is not one of them.
    pub(super) fn run_boss71(&mut self, st: &Stage, x: usize, ptr: u16) -> bool {
        match ptr {
            0xA800 => self.boss71_wait(x),
            0xA874 => self.boss71_jump(st, x),
            0xA8E4 => self.boss71_walk(st, x),
            0xA95C => self.boss71_shoot(x),
            0xAA64 => {
                self.move_h(x);
                self.move_v(x);
            }
            _ => return false,
        }
        true
    }

    /// `$A974`: back to the waiting state `$A800` with cleared counters.
    fn boss71_go_wait(&mut self, x: usize) {
        self.face_player(x);
        self.face_flag(x);
        let o = &mut self.o[x];
        o.s[0] = 0;
        o.s[1] = 0;
        o.s[2] = 0;
        o.ptr = 0xA800;
    }

    /// `$A800`: waits for Mega Man to act. Walking or newly pressing B makes it jump toward him (the jump speed depends on the distance),
    /// except that after three of those it shoots instead (`$A95C`), and after three shots it jumps again.
    fn boss71_wait(&mut self, x: usize) {
        if self.o[x].aid == 6 {
            if self.o[x].aframe != 1 {
                return;
            }
            self.o[x].aframe += 1;
            self.o[x].ypx = self.o[x].ypx.wrapping_sub(7);
            self.o[x].ptr = 0xA874;
            self.face_player(x);
            self.face_flag(x);
            return;
        }
        if self.held & 3 == 0 {
            if self.pressed & 0x40 == 0 {
                return;
            }
            self.boss71_jump_pick(x);
            return;
        }
        // $A855
        self.o[x].s[3] = 0;
        self.o[x].s[4] = self.o[x].s[4].wrapping_add(1);
        if self.o[x].s[4] == 3 {
            self.boss71_jump_pick(x);
            return;
        }
        self.o[x].ptr = 0xA95C;
        self.set_anim(x, 5);
    }

    /// `$A836`: count a jump; the third one is a shot series instead. Otherwise pick the jump from the distance (`$AA08`) and start animation 6.
    fn boss71_jump_pick(&mut self, x: usize) {
        self.o[x].s[4] = 0;
        self.o[x].s[3] = self.o[x].s[3].wrapping_add(1);
        if self.o[x].s[3] == 3 {
            self.o[x].s[3] = 0;
            self.o[x].s[4] = self.o[x].s[4].wrapping_add(1);
            self.o[x].ptr = 0xA95C;
            self.set_anim(x, 5);
            return;
        }
        let d = self.dist_x(x);
        let mut y: u16 = 3;
        while y > 0 {
            if d >= self.ai_byte(0xAA08 + y) {
                break;
            }
            y -= 1;
        }
        let (a, b, c, e) = (self.ai_byte(0xAA0C + y), self.ai_byte(0xAA10 + y), self.ai_byte(0xAA14 + y), self.ai_byte(0xAA18 + y));
        let o = &mut self.o[x];
        o.xvs = a;
        o.xvp = b;
        o.yvs = c;
        o.yvp = e;
        self.set_anim(x, 6);
    }

    /// `$A874`: the jump (animation 6: rises and lands on frame 2, frame 4 turns toward Mega Man) and the landing animation (frame 5 drops a static
    /// object `$81`, frame 8 picks a random pause and goes to `$A8E4`).
    fn boss71_jump(&mut self, st: &Stage, x: usize) {
        if self.o[x].aid == 6 {
            match self.o[x].aframe {
                2 => {
                    if self.vertical_collide(st, x, 8) {
                        self.o[x].aframe += 1;
                        return;
                    }
                    self.horizontal_collide(st, x, 0x14);
                    self.o[x].atick = 0;
                }
                4 => {
                    self.face_player(x);
                    self.face_flag(x);
                    self.set_anim(x, 4);
                }
                _ => {}
            }
            return;
        }
        match self.o[x].aframe {
            5 => {
                self.o[x].aframe += 1;
                if let Some(y) = self.find_slot() {
                    self.carry = false;
                    self.o[y].ty = 0x81;
                    self.child_here(x, y, 1);
                    self.o[y].hitbox = 0x94;
                }
            }
            8 => {
                // ADC with the carry the CMP #8 left (set)
                let sum = self.rng[0] as u16 + self.rng[2] as u16 + 1;
                self.rng[2] = sum as u8;
                let k = (self.rng[2] & 7) as u16;
                self.o[x].s[0] = self.ai_byte(0xAA5C + k);
                self.o[x].ptr = 0xA8E4;
            }
            _ => {}
        }
    }

    /// `$A900`: the boss turns vulnerable (hitbox `$F6`) and the first of its `$81` objects (animation 1) is removed (slot 7 if there is none).
    fn boss71_clear(&mut self, x: usize) {
        self.o[x].hitbox = 0xF6;
        let mut y = 0x17;
        while y != 7 {
            let a = self.o[y].aid;
            if a != 0 && a == 1 {
                break;
            }
            y -= 1;
        }
        self.kill_slot(y);
    }

    /// `$A8E4`: pause (counter `$0468`), then a walk toward/away from Mega Man for `$0480` frames, then a rest of 32 frames before it waits again.
    fn boss71_walk(&mut self, st: &Stage, x: usize) {
        if self.o[x].s[0] != 0 {
            self.face_player(x);
            self.face_flag(x);
            self.o[x].atick = 0;
            self.o[x].s[0] -= 1;
            if self.o[x].s[0] == 0 {
                self.o[x].s[2] = 0x20;
                self.set_anim(x, 3);
                self.o[x].xvs = 0;
                self.o[x].xvp = 3;
                let d = self.dist_x(x) >> 2;
                self.o[x].s[1] = d;
                if d != 0 {
                    self.boss71_clear(x);
                } else {
                    self.o[x].aid = self.o[x].aid.wrapping_sub(1);
                }
            } else if self.o[x].s[0] == 0x1E {
                self.boss71_clear(x);
            }
            return;
        }
        if self.o[x].s[1] != 0 {
            if self.horizontal_collide(st, x, 0x14) {
                self.turn_around(x);
            }
            self.o[x].s[1] -= 1;
            if self.o[x].s[1] == 0 {
                self.set_anim(x, 2);
            }
            return;
        }
        self.o[x].s[2] = self.o[x].s[2].wrapping_sub(1);
        if self.o[x].s[2] == 0 {
            self.boss71_go_wait(x);
        }
    }

    /// `$A95C`: the shooting pose; on animation frames 2, 5 and 8 a bullet (`$72`) leaves along the 16-way heading to Mega Man, frame `$0A` ends it.
    fn boss71_shoot(&mut self, x: usize) {
        match self.o[x].aframe {
            2 | 5 | 8 => {}
            0x0A => {
                self.set_anim(x, 2);
                self.boss71_go_wait(x);
                return;
            }
            _ => return,
        }
        self.o[x].aframe += 1;
        self.face_player(x);
        self.face_flag(x);
        let h = self.heading_to_player(x) as u16;
        let Some(y) = self.find_slot() else { return };
        self.carry = false;
        let dir = self.o[x].dir;
        self.child_at(x, y, 0x2F + (dir & 1), 8);
        let (yvs, yvp, xvs, xvp, d) = (self.ai_byte(0xAA3C + h), self.ai_byte(0xAA4C + h), self.ai_byte(0xAA1C + h), self.ai_byte(0xAA2C + h), self.ai_byte(0xF796 + h));
        let c = &mut self.o[y];
        c.ty = 0x72;
        c.hitbox = 0x8F;
        c.yvs = yvs;
        c.yvp = yvp;
        c.xvs = xvs;
        c.xvp = xvp;
        c.dir = if d & 3 == 0 { dir } else { d };
    }
}
