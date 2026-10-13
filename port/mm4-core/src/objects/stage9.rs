//! Stage 9 enemies in bank `$3B`: the block dropper `4E` (AI `$B9`, `$B7C5`) with its blocks `$BA` (`$B860`/`$B892`), the spikes `75`/`76` (AI `$B5`, `$B977`/`$B9B1`).
use super::*;

impl World {
    pub(super) fn run_stage9(&mut self, st: &Stage, x: usize, ptr: u16) -> bool {
        if self.f6 != 0x3B {
            return false;
        }
        match ptr {
            0xB7C5 => self.ai_b7c5(x),
            0xB860 => self.ai_b860(st, x),
            0xB892 => self.ai_b892(st, x),
            0xB977 => self.ai_b977(x),
            0xB9B1 => self.ai_b9b1(x),
            _ => return false,
        }
        true
    }

    /// `$3BB7C5`: every 180 frames, while fewer than three blocks `$BA` exist, it opens (animation frame 3) and drops a block.
    fn ai_b7c5(&mut self, x: usize) {
        if self.o[x].aframe == 0 {
            self.o[x].atick = 0;
            if self.o[x].s[0] != 0 {
                self.o[x].s[0] -= 1;
                if self.o[x].s[0] != 0 {
                    return;
                }
            }
            let n = (8..SLOTS).filter(|&s| self.o[s].ty == 0xBA).count();
            if n >= 3 {
                self.o[x].s[0] = 0xB4;
                return;
            }
            self.o[x].aframe += 1;
            let Some(y) = self.find_slot() else { return };
            let c = &mut self.o[y];
            c.ty = 0xBA;
            c.hitbox = 0xC0;
            c.hp = 1;
            self.child_here(x, y, 0x0E);
            let c = &mut self.o[y];
            let (v, carry) = c.ypx.overflowing_add(0x10);
            c.ypx = v;
            c.ypg = c.ypg.wrapping_add(carry as u8);
            c.xvs = 0;
            c.yvs = 0;
            c.yvp = 0;
            c.xvp = 1;
            c.s[1] = 1;
            c.s[2] = 0x2C;
        } else if self.o[x].atick == 0 && self.o[x].aframe == 3 {
            self.o[x].atick = 0;
            self.o[x].aframe = 0;
            self.o[x].s[0] = 0xB4;
        }
    }

    /// `$3BB8A3`: the block's 16-bit lifetime (`$0480:$0498`); when it runs out it becomes the inert object `$80`.
    fn b8a3(&mut self, x: usize) {
        let o = &mut self.o[x];
        let v = ((o.s[1] as u16) << 8 | o.s[2] as u16).wrapping_sub(1);
        o.s[2] = v as u8;
        o.s[1] = (v >> 8) as u8;
        if o.s[1] & 0x80 == 0 {
            return;
        }
        self.despawn(x);
        self.o[x].ty = 0x80;
        self.set_anim(x, 0x62);
    }

    /// `$3BB860`: the block falls; when it lands the first time it bounces, the second time it faces Mega Man and slides.
    fn ai_b860(&mut self, st: &Stage, x: usize) {
        self.b8a3(x);
        if !self.vertical_collide(st, x, 8) {
            return;
        }
        if self.o[x].s[0] == 0 {
            self.o[x].yvs = 0;
            self.o[x].yvp = 2;
            self.o[x].s[0] += 1;
            return;
        }
        self.face_player(x);
        self.face_flag(x);
        self.o[x].ptr = 0xB892;
        self.set_anim(x, 0x0F);
    }

    /// `$3BB892`: slides along the floor, turning at walls.
    fn ai_b892(&mut self, st: &Stage, x: usize) {
        if self.vertical_collide(st, x, 8) && self.horizontal_collide(st, x, 0x14) {
            self.turn_around(x);
        }
        self.b8a3(x);
    }

    /// The contact test of `$B9CD`: the hitbox only exists for this check, and a touch hurts Mega Man directly (`JSR $81CC`).
    fn b9cd(&mut self, x: usize) {
        if self.p.state < 6 && !self.invincible {
            let y = self.o[x].aid as u16;
            let hb = self.ai_byte(0xB96C + y);
            if hb != 0 {
                self.o[x].hitbox = hb;
                if self.touches_player(x) {
                    let dmg = self.ai_byte(0x8413 + self.o[x].ty as u16);
                    if dmg != 0 && self.p.anim != 0x27 {
                        self.hurt_req = Some((dmg, x));
                    }
                }
            }
        }
        self.o[x].hitbox = 0;
    }

    /// `$3BB977`: a floor spike: waits 90 frames, then plays its pop-up animation (frames 0-8) and turns into the next phase.
    fn ai_b977(&mut self, x: usize) {
        if self.o[x].aframe != 0 {
            if self.o[x].aframe == 8 {
                self.o[x].aid = self.o[x].aid.wrapping_add(1);
                self.o[x].atick = 0;
                self.o[x].aframe = 0;
                self.o[x].ptr = 0xB9B1;
            }
        } else {
            if self.o[x].s[0] == 0 {
                self.o[x].s[0] = 0x5A;
            }
            self.o[x].atick = 0;
            self.o[x].s[0] -= 1;
            if self.o[x].s[0] == 0 {
                self.o[x].aframe += 1;
            }
        }
        self.b9cd(x);
    }

    fn ai_b9b1(&mut self, x: usize) {
        if self.o[x].aframe != 0 {
            self.o[x].ptr = 0xB977;
            self.o[x].aid = self.o[x].aid.wrapping_add(1);
            let a = (self.o[x].aid & 3) | 0x84;
            self.set_anim(x, a);
        }
        self.b9cd(x);
    }
}
