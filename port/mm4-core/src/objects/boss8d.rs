//! The stage-1 boss (AI `$8D`, code in bank `$38`: `$B747`-`$B81D`; defeat and victory reuse the code of the other bosses).
use super::*;

impl World {
    /// Dispatch for the stage-1 boss states; returns false for an address that is not one of them.
    pub(super) fn run_boss8d(&mut self, st: &Stage, x: usize, ptr: u16) -> bool {
        match ptr {
            0xB747 => self.boss8d_idle(x),
            0xB7A7 => self.boss8d_jump(st, x),
            0xB7DA => self.boss8d_attack(x),
            _ => return false,
        }
        true
    }

    /// `$B81E`: back to the idle state with animation `$41`.
    fn boss8d_go_idle(&mut self, x: usize) {
        self.o[x].ptr = 0xB747;
        self.set_anim(x, 0x41);
    }

    /// `$B747`: faces Mega Man; after 64 frames it attacks (`$B7DA`). Whenever he newly presses B it jumps toward him.
    fn boss8d_idle(&mut self, x: usize) {
        if self.o[x].s[0] == 0 {
            self.o[x].s[0] = 0x40;
        }
        self.face_player(x);
        self.face_flag(x);
        self.o[x].s[0] -= 1;
        if self.o[x].s[0] == 0 {
            self.o[x].ptr = 0xB7DA;
            self.set_anim(x, 0x43);
            return;
        }
        if self.pressed & 0x40 == 0 {
            return;
        }
        let d = self.dist_x(x);
        let (q4, q5) = World::div_fc3a(0, d, 0, 0x30);
        let o = &mut self.o[x];
        o.yvs = 0;
        o.yvp = 6;
        o.xvs = q4;
        o.xvp = q5;
        o.ptr = 0xB7A7;
        self.set_anim(x, 0x42);
    }

    /// `$B7A7`: the jump: animation frame 1 flies until it lands (a wall stops the rise), frame 3 ends it.
    fn boss8d_jump(&mut self, st: &Stage, x: usize) {
        match self.o[x].aframe {
            1 => {
                self.o[x].atick = 0;
                if self.vertical_collide(st, x, 0x0A) {
                    self.o[x].aframe += 1;
                    return;
                }
                if self.horizontal_collide(st, x, 0x32) && self.o[x].yvp & 0x80 == 0 {
                    self.o[x].yvs = 0;
                    self.o[x].yvp = 0;
                }
            }
            3 => self.boss8d_go_idle(x),
            _ => {}
        }
    }

    /// `$B7DA`: the attack. On animation frame 8 it starts the 128-frame rain (animation `$6E`; the falling-drop drawing `$013A`/`$013B` is only
    /// visual); 8 frames later (counter `$78`) the rain hurts Mega Man directly. A buster shot that touches the boss ends it early.
    fn boss8d_attack(&mut self, x: usize) {
        if self.o[x].s[1] == 0 {
            if self.o[x].aframe == 8 {
                self.set_anim(x, 0x6E);
                self.o[x].s[1] = 0x80;
                self.w145 = x as u8; // $0145
                self.w13b = 0;
                self.w13a = 0x78;
            } else {
                if self.shot_hits(x).is_none() {
                    return;
                }
                self.boss8d_go_idle(x);
                return;
            }
        }
        if self.o[x].s[1] == 0x78 && !self.invincible && self.p.state < 6 {
            let dmg = self.ai_byte(0x8413 + self.o[x].ty as u16);
            if dmg != 0 && self.p.anim != 0x27 {
                self.hurt_req = Some((dmg, x));
            }
        }
        self.o[x].s[1] -= 1;
        if self.o[x].s[1] == 0 {
            self.boss8d_go_idle(x);
        }
    }
}
