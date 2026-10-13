//! Stage 0 enemies (bank `$3A` code `$94E6`...`$9E3F`, bank `$3B` `$A030`...`$A534`, bank `$3D` `$A90D`...`$A99A`).
use super::*;

impl World {
    /// Dispatch for the stage-0 states; returns false for an address that is not one of them.
    pub(super) fn run_stage0(&mut self, st: &Stage, x: usize, ptr: u16) -> bool {
        match ptr {
            0x94E6 => self.ai_94e6(x),
            0x9516 => self.ai_9516(st, x),
            0x992B => self.ai_992b(x),
            0x9988 => self.ai_9988(x),
            0x99ED => self.ai_99ed(x),
            0x9A70 => {
                self.move_h(x);
                self.move_v(x);
            }
            0x9E3F => self.ai_9e3f(st, x),
            0x9EE6 => self.ai_9ee6(x),
            0xAA86 => {
                // replacement type $4C of the walker (list type 10): the puff animation; the lights-out state $0135 it also sets is visual only
                if self.o[x].aid != 0x62 {
                    self.set_anim(x, 0x62);
                }
            }
            0x9F1D => self.ai_9f1d(x),
            0xA90D => self.ai_a90d(x),
            0xA927 => self.ai_a927(x),
            0xA980 => self.ai_a980(x),
            0xA99A => self.ai_a99a(x),
            0xA030 => self.ai_a030(st, x),
            0xA0BD => self.ai_a0bd(st, x),
            0xA46D => self.ai_a46d(x),
            0xA477 => self.ai_a477(x),
            0xA4BD => self.ai_a4bd(x),
            0xA534 => self.ai_a534(st, x),
            _ => return false,
        }
        true
    }

    /// Touch test with the object one pixel higher (`DEC $0378,x / JSR F95D / INC $0378,x`); the carry is clear for a hit.
    fn touches_lifted(&mut self, x: usize) -> bool {
        self.o[x].ypx = self.o[x].ypx.wrapping_sub(1);
        let t = self.touches_player(x);
        self.o[x].ypx = self.o[x].ypx.wrapping_add(1);
        self.carry = !t;
        t
    }

    /// The platform pattern shared by `$94E6`, `$A90D` and `$A980`: Mega Man's Y becomes `ypx - dy`, the object carries him and his ground probe
    /// pushes him onto it (`STA $0378 / JSR F32F / JSR F3C1`).
    fn carry_on_top(&mut self, x: usize, dy: u8) {
        let y = self.o[x].ypx.wrapping_sub(dy);
        self.player_ops.push(PlayerOp::SetY(y));
        self.carry_player(x);
        self.player_ops.push(PlayerOp::Ground(y));
        *self.stats.entry(0xF3C1).or_insert(0) += 1; // coverage: code_3FF3C1
    }

    /// `$3A95CB`: wait time of the next move from the random index in `$0480`.
    fn a95cb(&mut self, x: usize) {
        let k = self.o[x].s[1] as u16;
        self.o[x].s[2] = self.ai_byte(0x95F6 + k);
    }

    /// `$3A95C0`: pick the random index (`$E4 += $E6 + carry`, low 3 bits) and the wait time.
    fn a95c0(&mut self, x: usize) {
        let sum = self.rng[2] as u16 + self.rng[0] as u16 + self.carry as u16;
        self.rng[0] = sum as u8;
        self.carry = sum > 0xFF;
        self.o[x].s[1] = self.rng[0] & 7;
        self.a95cb(x);
    }

    /// `$3A95D5`: start a move with the speeds of table index `$0480`.
    fn a95d5(&mut self, x: usize) {
        let k = self.o[x].s[1] as u16;
        self.o[x].yvs = self.ai_byte(0x960E + k);
        self.o[x].yvp = self.ai_byte(0x9616 + k);
        self.o[x].xvs = self.ai_byte(0x95FE + k);
        self.o[x].xvp = self.ai_byte(0x9606 + k);
        self.snd(0x31, 0x95F5); // code_3A95F2
        self.carry = true; // JSR F6BE (sound queue): CMP #$88 on an empty slot leaves the carry set
    }

    /// AI `$16` waiting state (`$3A94E6`, list type `04`): starts when Mega Man touches it.
    fn ai_94e6(&mut self, x: usize) {
        self.o[x].atick = 0;
        let hit = self.touches_lifted(x);
        if !hit {
            return;
        }
        let o = &mut self.o[x];
        o.dir = 1;
        o.s[0] = 0x14;
        o.ptr = 0x9516;
        self.a95c0(x);
        self.o[x].xvs = 0;
        self.o[x].xvp = 0;
    }

    /// AI `$16` moving state (`$3A9516`): a platform that steps through six poses on a timer, moves while Mega Man stands on it and carries him.
    fn ai_9516(&mut self, st: &Stage, x: usize) {
        self.o[x].atick = 0;
        let hit = self.touches_lifted(x);
        self.o[x].s[3] = !hit as u8; // ROL $04B0,x after LDA #0: the carry of the touch test
        if self.o[x].s[0] != 0 {
            self.o[x].s[0] -= 1;
            if self.o[x].s[0] == 0 {
                let mut f = self.o[x].aframe.wrapping_add(1);
                if f == 6 {
                    f = 0;
                }
                self.o[x].aframe = f;
                let y = f as u16;
                self.o[x].ypx = self.o[x].ypx.wrapping_add(self.ai_byte(0x962A + y));
                self.o[x].s[0] = self.ai_byte(0x9630 + y);
                if f >= 4 {
                    if f == 4 {
                        self.a95d5(x);
                        self.o[x].s[2] = self.o[x].s[2].wrapping_sub(1);
                    } else if self.o[x].s[2] == 0 {
                        self.a95cb(x);
                        self.turn_around(x);
                    }
                }
            }
        }
        let f = self.o[x].aframe;
        let landed = self.vertical_collide(st, x, self.ai_byte(0x961E + f as u16) as usize);
        // $957B: no ground -> straight to the horizontal move; ground with pose 4 resets the speed first; other poses skip it
        let mut horizontal = !landed;
        if landed && f == 4 {
            self.o[x].s[0] = 1;
            self.o[x].xvs = 0;
            self.o[x].xvp = 0;
            horizontal = true;
        }
        if horizontal {
            let f = self.o[x].aframe;
            if self.horizontal_collide(st, x, self.ai_byte(0x9624 + f as u16) as usize) {
                self.o[x].xvs = 0;
                self.o[x].xvp = 0;
                self.turn_around(x);
                self.a95c0(x);
            }
        }
        if self.o[x].s[3] == 0 {
            self.carry_on_top(x, 0x13);
        }
    }

    /// AI `$1C` walking state (`$3A992B`, list type `10`): walks (animation `$4C`); on frame 9 it throws a puff (`$80`), after two walks it stops and
    /// starts the throwing animation.
    fn ai_992b(&mut self, x: usize) {
        if self.o[x].aframe | self.o[x].atick == 0 {
            self.o[x].s[0] = self.o[x].s[0].wrapping_add(1);
        }
        if self.o[x].atick == 0 && self.o[x].aframe == 9 {
            let Some(y) = self.find_slot() else { return };
            self.o[y].ty = 0x80;
            self.child_at(x, y, 2, 0x4E);
            let (xf, xd) = (self.o[x].flags, self.o[x].dir);
            self.o[y].flags = xf | ((xd & 1) << 6);
        }
        if self.o[x].s[0] < 2 {
            self.move_h(x);
            self.o[x].flags &= 0xBF;
            return;
        }
        self.o[x].aid = self.o[x].aid.wrapping_add(1);
        self.o[x].ptr = 0x9988;
    }

    /// AI `$1C` throwing state (`$3A9988`): back to walking after animation frame 3; on frame 1 it lobs a bomb (`$1D`).
    fn ai_9988(&mut self, x: usize) {
        if self.o[x].aframe == 3 && self.o[x].atick == 2 {
            self.o[x].ptr = 0x992B;
            self.o[x].s[0] = 0;
            self.set_anim(x, 0x4C);
            return;
        }
        if self.o[x].atick != 0 || self.o[x].aframe != 1 {
            return;
        }
        let Some(y) = self.find_slot() else { return };
        self.o[y].ty = 0x1D;
        self.o[y].xvs = 0;
        self.o[y].xvp = 0;
        self.o[y].yvs = 0;
        self.o[y].yvp = 4;
        self.child_at(x, y, 2, 0x4F);
        self.o[y].s[1] = 0x20;
        self.o[y].hitbox = 0xCE;
        self.o[y].hp = 2;
    }

    /// AI `$1D` (`$3A99ED`): the bomb rises; after its delay it explodes at random into five sparks (`$1E`).
    fn ai_99ed(&mut self, x: usize) {
        // code_3FF2A7
        if self.o[x].yvp & 0x80 != 0 {
            self.fall_move(x);
        } else {
            self.up(x);
        }
        self.carry = self.gravity_step(x);
        if self.o[x].s[1] != 0 {
            self.o[x].s[1] -= 1;
            if self.o[x].s[1] != 0 {
                return;
            }
        }
        self.o[x].s[0] = self.o[x].s[0].wrapping_add(1);
        if self.o[x].s[0] & 7 != 0 {
            return;
        }
        let sum = self.rng[1] as u16 + self.rng[2] as u16 + self.carry as u16;
        self.rng[2] = sum as u8;
        self.carry = sum > 0xFF;
        if self.rng[2] & 1 != 0 {
            return;
        }
        self.despawn(x);
        for e in (0..=4u16).rev() {
            let Some(y) = self.find_slot() else { return };
            self.o[y].ty = 0x1E;
            self.child_here(x, y, 0x50);
            self.o[y].hitbox = 0x8F;
            self.o[y].hp = 0;
            self.o[y].xvs = self.ai_byte(0x9A57 + e);
            self.o[y].xvp = self.ai_byte(0x9A5C + e);
            self.o[y].yvs = self.ai_byte(0x9A61 + e);
            self.o[y].yvp = self.ai_byte(0x9A66 + e);
            self.o[y].dir = self.ai_byte(0x9A6B + e);
        }
    }

    /// `$3A9E9A`: the vertical part of the hopper (`$37`): falls, and lands into its crouch / jump animations.
    fn a9e9a(&mut self, st: &Stage, x: usize) {
        let landed = self.vertical_collide(st, x, 8);
        if !landed {
            self.o[x].atick = 0;
            if self.o[x].s[0] == 0 {
                self.o[x].xvs = 0;
                self.o[x].xvp = 0;
                self.o[x].s[0] += 1;
                self.set_anim(x, 0x2C);
                self.o[x].aframe += 1;
            }
            return;
        }
        if self.o[x].s[0] == 0 {
            return;
        }
        let f = self.o[x].aframe;
        if f == 3 {
            self.o[x].s[0] -= 1;
            self.set_anim(x, 0x2B);
            self.o[x].xvs = 0;
            self.o[x].xvp = 1;
        } else if f < 2 {
            self.o[x].aframe = 2;
        }
    }

    /// AI `$37` (`$3A9E3F`, list type `15`): a hopper that walks, jumps (anim `$2C`) when it meets a wall and turns around in the air.
    fn ai_9e3f(&mut self, st: &Stage, x: usize) {
        if self.o[x].aid == 0x2C && self.o[x].aframe & 1 == 0 {
            return;
        }
        self.a9e9a(st, x);
        if !self.horizontal_collide(st, x, 0x14) {
            if self.o[x].s[0] != 0 {
                return;
            }
            self.o[x].s[1] = self.o[x].s[1].wrapping_add(1) & 0x3F;
            if self.o[x].s[1] == 0 {
                self.face_player(x);
            }
            return;
        }
        if self.o[x].s[0] == 0 {
            self.set_anim(x, 0x2C);
            self.o[x].yvs = 0x25;
            self.o[x].yvp = 4;
            self.o[x].s[0] += 1;
            return;
        }
        if self.o[x].yvp & 0x80 == 0 {
            return;
        }
        self.o[x].dir ^= 3;
        self.o[x].xvs = 0;
        self.o[x].xvp = 0;
    }

    /// Replacement type `$C8` of the hopper's death (`$3A9EE6`): the object becomes an explosion puff (`$80`, anim `$62`) and drops a debris spawner (`$38`).
    fn ai_9ee6(&mut self, x: usize) {
        self.despawn(x);
        self.set_anim(x, 0x62);
        self.o[x].ty = 0x80;
        if let Some(y) = self.find_slot() {
            self.o[y].ty = 0x38;
            self.child_here(x, y, 0x49);
            self.o[y].hitbox = 0;
            self.o[y].yvs = 0;
            self.o[y].yvp = 6;
            self.o[y].s[0] = 0;
            self.o[y].s[1] = 0;
        }
        self.carry = false;
    }

    /// AI `$38` (`$3A9F1D`): rises, hides itself and leaves three puffs (`$80`, anim `$4A`) ten frames apart. (The lights-out state `$0135` it also
    /// touches only drives the palette fade.)
    fn ai_9f1d(&mut self, x: usize) {
        if self.o[x].s[0] != 0 {
            self.o[x].s[0] -= 1;
            if self.o[x].s[0] != 0 {
                return;
            }
        } else {
            self.move_gravity(x);
            if self.o[x].yvp & 0x80 == 0 {
                return;
            }
            self.o[x].flags |= 4;
        }
        let Some(y) = self.find_slot() else { return };
        self.o[y].ty = 0x80;
        self.o[y].hitbox = 0;
        let idx = self.o[x].s[1].wrapping_add(0x1C);
        self.child_at(x, y, idx, 0x4A);
        self.o[x].s[0] = 0x0A;
        self.o[x].s[1] = self.o[x].s[1].wrapping_add(1);
        if self.o[x].s[1] == 3 {
            self.despawn(x);
        }
    }

    /// AI `$2F` waiting state (`$3DA90D`, list type `28`): starts when Mega Man touches it (then runs the moving state at once).
    fn ai_a90d(&mut self, x: usize) {
        if self.touches_lifted(x) {
            self.o[x].ptr = 0xA927;
            self.o[x].dir = 1;
            self.ai_a927(x);
        }
    }

    /// AI `$2F` moving state (`$3DA927`): a platform that slides right while sinking by the steps of table `$AA1F`, then falls.
    fn ai_a927(&mut self, x: usize) {
        let hit = self.touches_lifted(x);
        self.o[x].s[1] = !hit as u8;
        if self.o[x].s[0] != 0x48 {
            self.move_h(x);
            self.o[x].flags &= 0xBF;
            let y = self.o[x].s[0];
            self.o[x].s[0] = y.wrapping_add(1);
            self.o[x].ypx = self.o[x].ypx.wrapping_add(self.ai_byte(0xAA1F + y as u16));
            if y == 0x47 {
                self.o[x].xvs = 0;
                self.o[x].xvp = 0;
            }
        } else {
            if self.o[x].yvp & 0x80 != 0 {
                self.fall_move(x);
            } else {
                self.up(x);
            }
            self.carry = self.gravity_step(x);
        }
        if self.o[x].s[1] == 0 {
            self.carry_on_top(x, 0x14);
        }
    }

    /// AI `$C9` waiting state (`$3DA980`, list type `82`).
    fn ai_a980(&mut self, x: usize) {
        if !self.touches_lifted(x) {
            return;
        }
        self.o[x].ptr = 0xA99A;
        self.o[x].dir = 1;
        self.ai_a99a(x);
    }

    /// AI `$C9` moving state (`$3DA99A`): a platform that moves back and forth with the steps of table `$AA1F`, pausing at each end.
    fn ai_a99a(&mut self, x: usize) {
        if self.o[x].s[2] != 0 {
            self.o[x].s[2] -= 1;
            return;
        }
        let hit = self.touches_lifted(x);
        self.o[x].s[1] = !hit as u8;
        self.move_h(x);
        self.o[x].flags &= 0xBF;
        let at_end;
        if self.o[x].dir & 1 != 0 {
            let y = self.o[x].s[0];
            self.o[x].s[0] = y.wrapping_add(1);
            self.o[x].ypx = self.o[x].ypx.wrapping_add(self.ai_byte(0xAA1F + y as u16));
            at_end = if y == 0x47 { Some(y) } else { None };
        } else {
            let y = self.o[x].s[0];
            self.o[x].s[0] = y.wrapping_sub(1);
            let v = self.ai_byte(0xAA1F + y as u16).wrapping_neg();
            self.o[x].ypx = self.o[x].ypx.wrapping_add(v);
            at_end = if y == 0 { Some(y) } else { None };
        }
        if let Some(y) = at_end {
            self.o[x].s[0] = y;
            self.o[x].dir ^= 3;
            self.o[x].s[2] = 0x0A;
        }
        if self.o[x].s[1] == 0 {
            self.carry_on_top(x, 0x14);
        }
    }

    /// AI `$3D` resting state (`$3BA030`, list type `2D`): hangs hidden. Wakes up when Mega Man comes within 80 px or a shot hits it from below
    /// (the shot disappears); the wake-up animation `$0A` ends in the walking state.
    fn ai_a030(&mut self, st: &Stage, x: usize) {
        if self.o[x].aid == 0x0A {
            if !self.vertical_collide(st, x, 0x0E) {
                self.o[x].atick = 0;
                return;
            }
            if self.o[x].aframe != 8 {
                return;
            }
            self.o[x].s[0] = 0x14;
            self.o[x].ptr = 0xA0BD;
            self.o[x].yvs = 0;
            self.o[x].yvp = 0;
            self.set_anim(x, 0x0B);
            return;
        }
        let d = self.dist_x(x);
        if d >= 0x50 {
            let Some(y) = self.shot_hits(x) else { return };
            if self.o[y].ypx >= self.o[x].ypx {
                return;
            }
            self.kill_slot(y);
        }
        self.set_anim(x, 0x0A);
        self.o[x].hp = 2;
        self.o[x].hitbox = 0xC0;
        self.o[x].ypx = self.o[x].ypx.wrapping_add(8);
        let Some(y) = self.find_slot() else { return };
        self.o[y].ty = 0x80;
        self.o[y].hitbox = 0;
        self.o[y].hp = 0;
        self.child_here(x, y, 0x62);
        self.o[y].ypx = self.o[y].ypx.wrapping_sub(8);
    }

    /// AI `$3D` walking state (`$3BA0BD`): falls, walks and turns at walls; faces Mega Man when its timer ends.
    fn ai_a0bd(&mut self, st: &Stage, x: usize) {
        if self.o[x].s[0] != 0 {
            self.o[x].s[0] -= 1;
            if self.o[x].s[0] != 0 {
                return;
            }
            self.face_player(x);
        }
        if !self.vertical_collide(st, x, 6) {
            return;
        }
        if self.horizontal_collide(st, x, 0x1C) {
            self.turn_around(x);
        }
    }

    /// AI `$4E` (`$3BA46D`, list type `36`): first state, sets the timer and runs the waiting state at once.
    fn ai_a46d(&mut self, x: usize) {
        if self.o[x].s[0] == 0 {
            self.o[x].s[0] = 0x14;
        }
        self.ai_a477(x);
    }

    /// AI `$4E` waiting state (`$3BA477`): faces Mega Man; hops away when he is closer than 4 px, otherwise shoots at random intervals.
    fn ai_a477(&mut self, x: usize) {
        self.o[x].atick = 0;
        self.face_player(x);
        self.face_flag(x);
        let d = self.dist_x(x);
        if d < 4 {
            self.o[x].yvs = 0xA9;
            self.o[x].yvp = 5;
            self.o[x].ptr = 0xA534;
            return;
        }
        self.o[x].s[0] = self.o[x].s[0].wrapping_sub(1);
        if self.o[x].s[0] != 0 {
            return;
        }
        // CMP #4 / BCS left the carry set
        let sum = self.rng[1] as u16 + self.rng[3] as u16 + 1;
        self.rng[3] = sum as u8;
        self.carry = sum > 0xFF;
        if self.rng[3] & 3 == 0 {
            self.o[x].s[0] = 0x40;
            return;
        }
        self.o[x].aframe = self.o[x].aframe.wrapping_add(1);
        self.o[x].ptr = 0xA4BD;
        self.ai_a4bd(x);
    }

    /// AI `$4E` attack state (`$3BA4BD`): plays the aiming animation (`$79`) and shoots on its tick `$10`.
    fn ai_a4bd(&mut self, x: usize) {
        if self.o[x].aid == 0x79 {
            if self.o[x].aframe != 5 {
                return;
            }
            self.set_anim(x, 0x7A);
            // CMP #5 was equal: the carry is set
            let sum = self.rng[2] as u16 + self.rng[0] as u16 + 1;
            self.rng[0] = sum as u8;
            self.carry = sum > 0xFF;
            self.o[x].aframe = self.rng[0] & 7;
            return;
        }
        match self.o[x].atick {
            0x10 => {
                let Some(y) = self.find_slot() else { return };
                self.o[y].ty = 0x4F;
                self.o[y].hitbox = 0x8F;
                self.o[y].xvs = 0;
                self.o[y].xvp = 2;
                self.o[y].dir = self.o[x].dir;
                let bit = self.o[x].dir & 1;
                let idx = self.ai_byte(0xA54D + self.o[x].aframe as u16).wrapping_add(bit);
                self.child_at(x, y, idx, 0x7B);
                self.o[x].s[0] = 0x40;
            }
            0x20 => {
                self.o[x].ptr = 0xA477;
                self.set_anim(x, 0x79);
            }
            _ => {}
        }
    }

    /// AI `$4E` hop state (`$3BA534`): after hopping away it falls; on landing it waits again.
    fn ai_a534(&mut self, st: &Stage, x: usize) {
        self.o[x].atick = 0;
        if !self.vertical_collide(st, x, 0x1C) {
            return;
        }
        self.o[x].ptr = 0xA477;
        self.o[x].s[0] = 0x40;
    }
}
