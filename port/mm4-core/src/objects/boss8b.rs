//! The stage-0 boss (AI `$8B`, code in bank `$38`: `$B53C`-`$B71D`; defeat and victory reuse the code of the other bosses).
use super::*;

impl World {
    /// Dispatch for the stage-0 boss states; returns false for an address that is not one of them.
    pub(super) fn run_boss8b(&mut self, st: &Stage, x: usize, ptr: u16) -> bool {
        match ptr {
            0xB53C => self.boss8b_tail(x, 0),
            0xB541 => self.boss8b_wait(x),
            0xB58E => self.boss8b_jump_start(st, x),
            0xB5BE => self.boss8b_air(st, x),
            0xB5DA => self.boss8b_aim(x),
            0xB5F6 => self.boss8b_shoot(x),
            0xB690 => self.boss8b_flash(x),
            0xB718 => {
                self.move_h(x);
                self.move_v(x);
            }
            _ => return false,
        }
        true
    }

    /// `$B575`: start the animation of move `y` (table `$B6C8`); an odd animation id means a jump (`$B58E`), an even one a shot series (`$B5DA`).
    fn boss8b_tail(&mut self, x: usize, y: u8) {
        let anim = self.b38(0xB6C8 + y as u16);
        self.set_anim(x, anim);
        let k = (self.o[x].aid & 1) as u16;
        self.o[x].ptr = self.b38(0xB6E0 + k) as u16 | (self.b38(0xB6E2 + k) as u16) << 8;
    }

    /// `$B625`: back to waiting (30 frames, animation `$34`).
    fn boss8b_settle(&mut self, x: usize) {
        self.o[x].s[0] = 0x1E;
        self.o[x].ptr = 0xB541;
        self.set_anim(x, 0x34);
    }

    /// `$B541`: waits, faces Mega Man and picks the next move. At 16, 8 and 4 hit points left it starts the flash attack (`$B690`) once.
    fn boss8b_wait(&mut self, x: usize) {
        self.o[x].s[0] = self.o[x].s[0].wrapping_sub(1);
        if self.o[x].s[0] != 0 {
            return;
        }
        self.face_player(x);
        self.face_flag(x);
        let hp = self.o[x].hp;
        let special = matches!(hp, 4 | 8 | 0x10);
        if special && hp != self.o[x].s[4] {
            self.o[x].s[4] = hp;
            self.o[x].ptr = 0xB690;
            self.set_anim(x, 0x37);
            return;
        }
        self.carry = if special { true } else { hp >= 0x10 };
        let y = self.boss_choose(x, false);
        self.boss8b_tail(x, y);
    }

    /// `$B58E`: the jump: speeds toward Mega Man (distance / `$37`), then straight into the air state.
    fn boss8b_jump_start(&mut self, st: &Stage, x: usize) {
        self.o[x].yvs = 0xEE;
        self.o[x].yvp = 6;
        let d = self.dist_x(x);
        let (q4, q5) = World::div_fc3a(0, d, 0, 0x37);
        self.o[x].xvs = q4;
        self.o[x].xvp = q5;
        self.o[x].ptr = 0xB5BE;
        self.boss8b_air(st, x);
    }

    /// `$B5BE`: in the air until it lands; a wall stops the rise.
    fn boss8b_air(&mut self, st: &Stage, x: usize) {
        if self.vertical_collide(st, x, 0x13) {
            self.boss8b_settle(x);
            return;
        }
        if self.horizontal_collide(st, x, 0x34) && self.o[x].yvp & 0x80 == 0 {
            self.o[x].yvs = 0;
            self.o[x].yvp = 0;
        }
    }

    /// `$B5DA`: pick where in the shot table the series starts, then run the shooting state at once.
    fn boss8b_aim(&mut self, x: usize) {
        let sum = self.rng[2] as u16 + self.rng[3] as u16 + self.carry as u16;
        self.rng[2] = sum as u8;
        self.carry = sum > 0xFF;
        self.o[x].s[1] = (self.rng[2] & 7) << 2;
        self.o[x].s[2] = 3;
        self.o[x].ptr = 0xB5F6;
        self.boss8b_shoot(x);
    }

    /// `$B5F6`: the shot series: on animation frame 1 it fires a ball (`$8C`), on frame 3 it either fires again (three times) or settles.
    fn boss8b_shoot(&mut self, x: usize) {
        self.face_player(x);
        self.face_flag(x);
        if self.o[x].s[3] != 0 {
            self.o[x].s[3] -= 1;
            if self.o[x].s[3] != 0 {
                return;
            }
            self.set_anim(x, 0x38);
        }
        match self.o[x].aframe {
            1 => {
                self.o[x].aframe += 1;
                let Some(y) = self.find_slot() else { return };
                self.o[y].ty = 0x8C;
                self.o[y].hitbox = 0x8F;
                self.o[y].dir = self.o[x].dir;
                let idx = (self.o[x].dir & 1).wrapping_add(0x50);
                self.child_at(x, y, idx, 0x08);
                let k = (self.b38(0xB6E4 + self.o[x].s[1] as u16) & 3) as u16;
                self.o[y].xvs = self.b38(0xB704 + k);
                self.o[y].xvp = self.b38(0xB708 + k);
                self.o[y].yvs = self.b38(0xB70C + k);
                self.o[y].yvp = self.b38(0xB710 + k);
                self.o[y].dir |= self.b38(0xB714 + k);
                self.o[x].s[1] = self.o[x].s[1].wrapping_add(1);
            }
            3 => {
                self.o[x].s[2] = self.o[x].s[2].wrapping_sub(1);
                if self.o[x].s[2] == 0 {
                    self.boss8b_settle(x);
                    return;
                }
                self.o[x].s[3] = 0x14;
                self.set_anim(x, 0x34);
            }
            _ => {}
        }
    }

    /// `$B690`: the flash attack: on animation frame 8 Mega Man is frozen for 255 frames (`$AD`; any damage ends it); frame 12 ends it.
    fn boss8b_flash(&mut self, x: usize) {
        match self.o[x].aframe {
            8 => {
                if self.o[x].s[5] == 0 {
                    self.o[x].s[5] = 0x12;
                    self.player_ops.push(PlayerOp::Freeze(0xFF));
                }
            }
            0x0C => {
                self.boss8b_settle(x);
                return;
            }
            _ => {}
        }
        // the palette flicker of the flash ($0610, $18) is only visual
        if self.o[x].s[5] != 0 {
            self.o[x].s[5] -= 1;
        }
    }
}
