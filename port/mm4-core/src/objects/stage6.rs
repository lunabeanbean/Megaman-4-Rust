//! Stage 6 enemies: bank `$3B` `$A729`-`$A93C` (`4A`), `$A952`-`$A9A6` (`45`), `$B0D9`-`$B1xx` (`5D`), `$B6E5`-`$B77D` (`77`), bank `$3D` `$AFA6` (`87`).
//! Stage 6 is the underwater stage: `$013E` is the water surface (set by the scroll objects `$6B`/`$6C`) and several enemies stay below it.
use super::*;

impl World {
    /// Dispatch for the stage-6 states (checked before the older arms: a few addresses also exist in other banks).
    pub(super) fn run_stage6(&mut self, st: &Stage, x: usize, ptr: u16) -> bool {
        let bank = self.f6;
        match (bank, ptr) {
            (0x3B, 0xA729) => self.ai_a729(x),
            (0x3B, 0xA762) => self.ai_a762(st, x),
            (0x3B, 0xA7E5) => self.ai_a7e5_6(st, x),
            (0x3B, 0xA805) => self.ai_a805(st, x),
            (0x3B, 0xA952) => {
                self.o[x].s[0] = self.o[x].ypx;
                self.ai_a958(st, x);
            }
            (0x3B, 0xA958) => self.ai_a958(st, x),
            (0x3B, 0xA978) => self.ai_a978(st, x),
            (0x3B, 0xB0D9) => self.ai_b0d9(x),
            (0x3B, 0xB121) => self.ai_b121(x),
            (0x3B, 0xB6E5) => self.ai_b6e5(x),
            (0x3B, 0xB703) => self.ai_b703(x),
            (0x3B, 0xB761) => self.ai_b761(x),
            (0x3D, 0xAFA6) => {
                let (v, c) = self.w13e.overflowing_add(8);
                self.carry = c;
                self.o[x].ypx = v;
            }
            _ => return self.run_stage6b(x, ptr),
        }
        true
    }

    // ---- enemy 4A (AI $65): shell that jumps out, shoots three bullets and bubbles ----------------------------------------------

    /// `$3BA8F1`: every 120/40 frames (alternating) the enemy releases an air bubble `$66`.
    fn a8f1(&mut self, x: usize) {
        if self.o[x].s[2] != 0 {
            self.o[x].s[2] -= 1;
            return;
        }
        let k = (self.o[x].s[1] & 1) as u16;
        self.o[x].s[2] = self.ai_byte(0xA93D + k);
        self.o[x].s[1] = self.o[x].s[1].wrapping_add(1);
        let Some(y) = self.find_slot() else {
            self.carry = true;
            return;
        };
        self.carry = false;
        self.o[y].ty = 0x66;
        self.o[y].hitbox = 0;
        let dir = self.o[x].dir;
        self.o[y].dir = dir;
        self.child_at(x, y, (dir & 1) + 0x42, 0x7E);
        let c = &mut self.o[y];
        c.flags &= 0xBF;
        c.yvs = 0;
        c.yvp = 1;
    }

    /// `$3BA870`: probe `id` with the water gravity (`$99` = $15). Above the water line (`$013E`) the object is held at it; out of pure water (`$10 != $60`)
    /// it sets `$04C8` = $FF and loses an upward speed. Returns the carry of the probe (landed).
    fn a870(&mut self, st: &Stage, x: usize, id: usize) -> bool {
        self.gravity = 0x15;
        let landed = self.vertical_collide(st, x, id);
        let or = self.pt.iter().fold(0u8, |a, &t| a | t);
        let ypx = self.o[x].ypx;
        let a = if self.w13e >= ypx {
            Some(self.w13e)
        } else if or != 6 {
            Some(ypx)
        } else {
            None
        };
        if let Some(a) = a {
            self.o[x].ypx = a;
            self.o[x].s[4] = 0xFF;
            if self.o[x].yvp & 0x80 == 0 {
                self.land_speed(x);
            }
        }
        self.gravity = 0x40;
        self.carry = landed;
        landed
    }

    /// `$3BA8B5`: jump up with a random sideways speed (the carry in decides the rng subtraction).
    fn a8b5(&mut self, x: usize) {
        self.o[x].yvs = 0x95;
        self.o[x].yvp = 2;
        let borrow = !self.carry as u8;
        let a = self.rng[3].wrapping_sub(self.rng[1]).wrapping_sub(borrow);
        self.carry = self.rng[3] as u16 >= self.rng[1] as u16 + borrow as u16;
        self.rng[1] = a;
        let y = (a & 3) as u16;
        self.o[x].xvs = self.ai_byte(0xA8E9 + y);
        self.o[x].xvp = self.ai_byte(0xA8ED + y);
        self.o[x].ptr = 0xA762;
        self.face_player(x);
        self.face_flag(x);
        self.set_anim(x, 0x7D);
    }

    /// `$3BA8A5`: restart the wait (32 frames) and, with a random bit, jump.
    fn a8a5(&mut self, x: usize) {
        self.o[x].s[0] = 0x20;
        let sum = self.rng[2] as u16 + self.rng[3] as u16 + self.carry as u16;
        self.rng[3] = sum as u8;
        self.carry = sum > 0xFF;
        if self.rng[3] & 1 == 0 {
            self.a8b5(x);
        }
    }

    /// `$3BA729`: waiting in the shell; when Mega Man is within 64 px the shell opens, and on animation frame 3 the enemy jumps.
    fn ai_a729(&mut self, x: usize) {
        self.a8f1(x);
        if self.o[x].aframe == 0 {
            self.o[x].atick = 0;
            self.face_player(x);
            self.face_flag(x);
            let d = self.dist_x(x);
            self.carry = d >= 0x40;
            if d >= 0x40 {
                return;
            }
            self.o[x].aframe += 1;
            self.o[x].hitbox = 0xD1;
        } else if self.o[x].aframe == 3 {
            let y = self.o[x].ypx;
            self.carry = y >= 8;
            self.o[x].ypx = y.wrapping_sub(8);
            self.o[x].s[0] = 0x0C;
            self.a8b5(x);
        }
    }

    /// `$3BA762`: rising/falling after the jump: at animation `$7C` frame 3 the pose changes, otherwise shoots when the countdown ends.
    fn ai_a762(&mut self, st: &Stage, x: usize) {
        self.a8f1(x);
        if self.o[x].aid == 0x7C {
            if self.o[x].aframe == 3 {
                self.set_anim(x, 0x7D);
            }
            return;
        }
        if self.o[x].s[0] != 0 {
            self.o[x].s[0] -= 1;
            if self.o[x].s[0] == 0 {
                let n = (8..SLOTS).filter(|&s| self.o[s].ty == 0x51).count();
                if n < 6 {
                    self.shoot_a5fe(x, 2);
                }
            }
        }
        self.a870(st, x, 0x26);
        if self.o[x].yvp & 0x80 == 0 {
            self.horizontal_collide(st, x, 0x2E);
            return;
        }
        if self.pt.iter().any(|t| t & 1 != 0) {
            let o = &mut self.o[x];
            o.ptr = 0xA7E5;
            o.xvs = 0;
            o.xvp = 0;
            self.set_anim(x, 0x9F);
        } else {
            let o = &mut self.o[x];
            o.ptr = 0xA805;
            o.atick = 0;
            o.aframe = 0;
            o.s[0] = 0x0C;
        }
    }

    /// `$3BA7E5`: landed (anim `$9F`): wait for the landing animation, then go on as `$A805`.
    fn ai_a7e5_6(&mut self, st: &Stage, x: usize) {
        self.a8f1(x);
        if self.o[x].aframe == 3 {
            self.o[x].atick = 0;
        }
        if !self.a870(st, x, 0x0E) {
            return;
        }
        self.o[x].ptr = 0xA805;
        self.ai_a805(st, x);
    }

    /// `$3BA805`: on the ground: counts down for the next jump, falls off ledges, and in animation `$9F` opens fire.
    fn ai_a805(&mut self, st: &Stage, x: usize) {
        self.a8f1(x);
        let aid = self.o[x].aid;
        self.carry = aid >= 0x9F;
        if aid == 0x9F {
            if self.o[x].s[3] != 0 {
                self.o[x].atick = 0;
                self.o[x].s[3] -= 1;
                if self.o[x].s[3] == 0 {
                    let o = &mut self.o[x];
                    o.ptr = 0xA729;
                    o.s[4] = 0;
                    self.set_anim(x, 0x7C);
                }
            } else if self.o[x].aframe == 3 {
                self.o[x].s[3] = 0x40;
                self.o[x].hitbox = 0xA0;
            }
            return;
        }
        self.o[x].atick = 0;
        if self.o[x].s[4] & 0x80 == 0 {
            self.o[x].s[0] = self.o[x].s[0].wrapping_sub(1);
            if self.o[x].s[0] == 0 {
                self.a8a5(x);
                return;
            }
        }
        if self.a870(st, x, 0x26) {
            self.o[x].ypx = self.o[x].ypx.wrapping_add(8);
            self.set_anim(x, 0x9F);
        } else {
            self.horizontal_collide(st, x, 0x2E);
        }
    }

    // ---- enemy 45 (AI $5F): hangs at its spot, drops when Mega Man is near and climbs back ---------------------------------------

    /// `$3BA958`: waiting; when Mega Man is within 64 px it starts to drop (and the same frame moves, `$A978`).
    fn ai_a958(&mut self, st: &Stage, x: usize) {
        let d = self.dist_x(x);
        self.carry = d >= 0x40;
        if d >= 0x40 {
            return;
        }
        let o = &mut self.o[x];
        o.yvs = 0;
        o.yvp = 2;
        o.ptr = 0xA978;
        o.dir = 4;
        self.ai_a978(st, x);
    }

    /// `$3BA978`: falls until it hits something, then rises back to its home height (`$0468`).
    fn ai_a978(&mut self, st: &Stage, x: usize) {
        if self.dir_v_collide(st, x, 0x1F) {
            let o = &mut self.o[x];
            o.dir = 8;
            o.yvp = 1;
            return;
        }
        if self.o[x].dir & 4 != 0 {
            return;
        }
        if self.o[x].s[0] < self.o[x].ypx {
            return;
        }
        self.o[x].ypx = self.o[x].s[0];
        self.o[x].ptr = 0xA958;
    }

    // ---- enemy 5D (AI $91): swimmer ---------------------------------------------------------------------------------------------

    /// `$3BB152`: speeds and animation for heading class `y`.
    fn b152(&mut self, x: usize, y: u8) {
        let y = y as u16;
        let (a, b, c, d, e) = (self.ai_byte(0xB178 + y), self.ai_byte(0xB180 + y), self.ai_byte(0xB188 + y), self.ai_byte(0xB190 + y), self.ai_byte(0xB170 + y));
        let o = &mut self.o[x];
        o.xvs = a;
        o.xvp = b;
        o.yvs = c;
        o.yvp = d;
        self.set_anim(x, e);
    }

    /// `$3BB0D9`: swims along for 48 frames (facing Mega Man while `$0498` of slot 0 is 0), then aims at him.
    fn ai_b0d9(&mut self, x: usize) {
        self.move_v(x);
        self.move_h(x);
        if true {
            self.face_player(x);
            self.face_flag(x);
        }
        self.o[x].s[2] = self.o[x].s[2].wrapping_add(1);
        if self.o[x].s[2] != 0x30 {
            return;
        }
        self.o[x].ptr = 0xB121;
        self.face_player(x);
        self.face_flag(x);
        let h = self.heading_to_player(x);
        let k = h & 7;
        let d = self.rom.cpu_byte(0x3A, 0x3B, 0xF796 + h as u16);
        self.o[x].dir = (d & 0xFC) | self.o[x].dir;
        self.o[x].s[0] = 0x30;
        self.b152(x, k);
        self.ai_b121(x);
    }

    /// `$3BB121`: stays below the water line (+8), moves; after `$0468` frames it slows to the cruising speeds.
    fn ai_b121(&mut self, x: usize) {
        let (a, c) = self.w13e.overflowing_add(8);
        self.carry = c;
        if a >= self.o[x].ypx {
            self.o[x].ypx = a;
            self.set_anim(x, 0x66);
        }
        self.move_v(x);
        self.move_h(x);
        if self.o[x].s[0] != 0 {
            self.o[x].s[0] -= 1;
            if self.o[x].s[0] == 0 {
                self.o[x].dir &= 3;
                self.b152(x, 4);
            }
        }
    }

    // ---- enemy 77 (AI $B6): bobbing mine that turns into a `$83` when Mega Man leaves ---------------------------------------------

    /// `$3BB6E5`: init (drifts up at 0.25 px/frame), then as `$B703`.
    fn ai_b6e5(&mut self, x: usize) {
        let o = &mut self.o[x];
        o.yvs = 0x40;
        o.yvp = 0;
        o.dir = 8;
        o.s[0] = 0x40;
        o.ptr = 0xB703;
        self.ai_b703(x);
    }

    /// `$3BB703`: bobs up and down; when Mega Man hovers near (within 64 px both ways for a while) it may arm itself (`$B761`).
    fn ai_b703(&mut self, x: usize) {
        let mut b737 = true;
        let dy = self.dist_y(x);
        if dy < 0x40 {
            let dx = self.dist_x(x);
            self.carry = dx >= 0x40;
            if dx >= 0x40 {
                b737 = false;
            } else if self.o[x].s[2] != 0 {
                self.o[x].s[2] -= 1;
                if self.o[x].s[2] != 0 {
                    b737 = false;
                } else {
                    let sum = self.rng[1] as u16 + self.rng[2] as u16; // carry is clear after the CMP
                    if sum & 1 == 0 {
                        let o = &mut self.o[x];
                        o.ptr = 0xB761;
                        o.s[0] = 0x40;
                        self.set_anim(x, 0xA8);
                        return;
                    }
                }
            }
        }
        if b737 {
            self.o[x].s[2] = 0x40;
        }
        if self.o[x].s[1] != 0 {
            self.o[x].s[1] -= 1;
            if self.o[x].s[1] != 0 {
                return;
            }
            self.o[x].s[0] = 0x40;
            self.o[x].dir ^= 0x0C;
        }
        self.move_v(x);
        self.o[x].s[0] = self.o[x].s[0].wrapping_sub(1);
        if self.o[x].s[0] == 0 {
            self.o[x].s[1] = 0x1E;
        }
    }

    /// `$3BB761`: armed: after `$0468` frames it turns into the explosion-like object `$83`.
    fn ai_b761(&mut self, x: usize) {
        self.o[x].s[0] = self.o[x].s[0].wrapping_sub(1);
        if self.o[x].s[0] != 0 {
            return;
        }
        self.o[x].s[0] = 0;
        self.o[x].s[1] = 0;
        self.despawn(x);
        self.o[x].ty = 0x83;
        self.o[x].hitbox = 0x94;
        self.set_anim(x, 0x62);
    }
}
