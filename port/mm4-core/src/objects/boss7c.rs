//! The stage-6 boss (type `$7C`, code in bank `$38`: `$AEC6`-`$AFFD`) and its homing shot `$7D` (`$AFFE`). Defeat and victory reuse the code of the other bosses.
use super::*;

impl World {
    /// Dispatch for the stage-6 boss states; returns false for an address that is not one of them.
    pub(super) fn run_boss7c(&mut self, st: &Stage, x: usize, ptr: u16) -> bool {
        if self.f6 != 0x38 {
            return false;
        }
        match ptr {
            0xAEC6 => self.boss7c_af0c(x),
            0xAEC9 => self.boss7c_aec9(x),
            0xAF29 => self.boss7c_af29(x),
            0xAF94 => self.boss7c_af94(st, x),
            0xAFFE => self.ai_affe(x),
            _ => return false,
        }
        true
    }

    /// `$38AF0C`: start the next move cycle (counts it in `$04B0`) and go to the walk/turn state `$AF94`.
    fn boss7c_af0c(&mut self, x: usize) {
        let o = &mut self.o[x];
        o.s[3] = o.s[3].wrapping_add(1);
        o.s[2] = 0;
        o.ptr = 0xAF94;
        o.hitbox = 0xC2;
        self.set_anim(x, 0x1E);
    }

    /// `$38AFBF`: vulnerable again (hitbox `$E4`), face Mega Man and wait in `$AEC9`.
    fn boss7c_afbf(&mut self, x: usize) {
        self.o[x].hitbox = 0xE4;
        self.o[x].ptr = 0xAEC9;
        self.face_player(x);
        self.face_flag(x);
    }

    /// `$38AEC9`: after the pause: faces Mega Man and either starts a new move cycle (`$AF0C`) or, when fewer than two shots `7D` fly,
    /// the throw (`$AF29`).
    fn boss7c_aec9(&mut self, x: usize) {
        self.o[x].s[0] = self.o[x].s[0].wrapping_sub(1);
        if self.o[x].s[0] != 0 {
            return;
        }
        self.face_player(x);
        self.face_flag(x);
        if self.o[x].s[2] == 3 {
            self.boss7c_af0c(x);
            return;
        }
        if self.o[x].s[3] != 3 {
            let d = self.dist_x(x);
            if d < 0x50 {
                self.boss7c_af0c(x);
                return;
            }
        }
        let n = (8..SLOTS).filter(|&s| self.o[s].ty == 0x7D).count();
        if n >= 2 {
            self.boss7c_af0c(x);
            return;
        }
        let o = &mut self.o[x];
        o.s[2] += 1;
        o.s[3] = 0;
        o.ptr = 0xAF29;
        o.s[1] = 1;
        self.set_anim(x, 0x1D);
    }

    /// `$38AF29`: the throw animation; on frame 2 it releases a homing shot `7D`, on frame 4 it is over (the first time it plays once more).
    fn boss7c_af29(&mut self, x: usize) {
        if self.o[x].s[0] != 0 {
            self.o[x].s[0] -= 1;
            if self.o[x].s[0] != 0 {
                return;
            }
            self.set_anim(x, 0x1D);
        }
        match self.o[x].aframe {
            2 => {
                self.o[x].aframe += 1;
                let Some(y) = self.find_slot() else { return };
                let idx = (self.o[x].dir & 1) + 0x2D;
                self.child_at(x, y, idx, 0x1F);
                let dir = self.o[x].dir;
                let c = &mut self.o[y];
                c.ty = 0x7D;
                c.hitbox = 0xCE;
                c.hp = 1;
                c.s[0] = 0;
                c.s[3] = (dir << 2) | 4;
                c.s[4] = 0xFF;
            }
            4 => {
                self.set_anim(x, 0x24);
                self.o[x].s[0] = 0x1E;
                if self.o[x].s[1] == 0 {
                    self.o[x].s[1] += 1;
                } else {
                    self.boss7c_afbf(x);
                }
            }
            _ => {}
        }
    }

    /// `$38AF94`: the walk cycle: animation `$1E` (turn) -> on frame 6 a step of 8 px and `$26` (slide toward Mega Man) -> `$28` (landing) -> pause.
    fn boss7c_af94(&mut self, st: &Stage, x: usize) {
        match self.o[x].aid {
            0x26 => {
                self.o[x].s[1] = self.o[x].s[1].wrapping_sub(1);
                if self.o[x].s[1] != 0 {
                    let id = 0x0C;
                    if !self.horizontal_collide(st, x, id) {
                        return;
                    }
                }
                self.set_anim(x, 0x28);
            }
            0x1E => {
                if self.o[x].aframe != 6 {
                    return;
                }
                let k = (self.o[x].dir & 1) as u16;
                let step = self.ai_byte(0xAFFC + k);
                self.o[x].xpx = self.o[x].xpx.wrapping_add(step);
                self.set_anim(x, 0x26);
                let d = self.dist_x(x);
                self.o[x].s[1] = d.wrapping_add(0x10) >> 2;
            }
            _ => {
                if self.o[x].aframe == 0 {
                    return;
                }
                self.set_anim(x, 0x24);
                self.o[x].s[0] = 0x1E;
                self.boss7c_afbf(x);
            }
        }
    }

    /// `$38AFFE` (type `7D`): a shot that steers toward Mega Man (every 5 frames, for 255 frames) and then flies straight; gone when it leaves the screen.
    fn ai_affe(&mut self, x: usize) {
        if self.o[x].s[4] != 0 {
            self.o[x].s[4] -= 1;
            if self.o[x].s[0] == 0 {
                self.aim(x);
                let k = self.o[x].s[3];
                let anim = self.b38(0xB035 + k as u16);
                self.set_anim(x, anim);
                self.heading_speed(x, k, 0);
                self.o[x].s[0] = 5;
            }
            self.o[x].s[0] = self.o[x].s[0].wrapping_sub(1);
        }
        self.move_h(x);
        self.move_v(x);
        if self.o[x].ypg != 0 {
            self.despawn(x);
        }
    }
}
