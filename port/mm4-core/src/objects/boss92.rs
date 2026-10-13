//! The stage-11 boss (type `$92` = list `5E`, bank `$3D` `$B65D`-`$BA35`): a press that hangs from the ceiling, moves up and down and sideways, throws
//! shots `$94` (`$BA8B`) and crushes Mega Man (player state `$0C`, not ported: the verifier stops there). Its parts are slots 21-23 addressed absolutely.
use super::*;

impl World {
    pub(super) fn run_boss92(&mut self, x: usize, ptr: u16) -> bool {
        if self.f6 != 0x3D {
            return false;
        }
        match ptr {
            0xB65D => self.boss92_init(x),
            0xB678 => self.boss92_fade(x),
            0xB6E1 => self.boss92_meter(x),
            0xB702 => self.boss92_b702(x),
            0xB742 => self.boss92_b742(x),
            0xB778 => self.boss92_b778(x),
            0xB865 => self.boss92_b865(x),
            0xB8DB => self.boss92_b8db(x),
            0xB90A => self.boss92_b90a(x),
            0xB93A => self.boss92_b93a(x),
            0xB9F3 => self.boss92_b9f3(x),
            _ => return false,
        }
        true
    }

    fn boss92_init(&mut self, x: usize) {
        if self.fd9c(x) != 9 {
            return;
        }
        self.p1c = 0x14;
        self.p1b = 0x80;
        self.o[x].ptr = 0xB678;
    }

    fn boss92_fade(&mut self, x: usize) {
        if self.f0 == 0 {
            if self.p1b != 0 {
                return;
            }
            self.p95 = 0;
            self.f1 = 0x90;
            self.f0 = 6;
            self.o[x].s[0] = 0x30;
        }
        if self.p95 & 0x0F != 0 {
            return;
        }
        let (a, b) = self.o[x].s[0].overflowing_sub(0x10);
        self.o[x].s[0] = a;
        if !b {
            return;
        }
        let o = &mut self.o[x];
        o.ptr = 0xB6E1;
        o.s[3] = 0x78;
        o.flags = 0x90;
        self.meter2 = 0x8F;
        self.p95 = 0;
        self.bf = 0;
        self.o[22].flags = 0x90;
        self.o[21].flags = 0x90;
    }

    fn boss92_meter(&mut self, x: usize) {
        if self.p95 & 7 != 0 {
            return;
        }
        self.snd(0x29, 0xB6EC); // code_3DB6E9
        self.bf = self.bf.wrapping_add(1);
        if self.bf != 0x1C {
            return;
        }
        self.carry = true;
        self.o[x].ptr = 0xB702;
        self.player_cmd = Some(0);
        self.p.state = 0;
        self.boss92_b702(x);
    }

    /// `$B94C`: the common tail: at 2 hit points left the boss's defeat starts, then the parts follow and the room scroll is set.
    fn boss92_tail(&mut self, x: usize) {
        if self.o[x].hp == 2 && self.p.state < 2 {
            self.meter2 = 0;
            self.o[x].t5b8 = 0;
            self.o[x].ptr = 0xB9F3;
            self.player_cmd = Some(0x0D);
            self.p.state = 0x0D;
            for y in (1..=4).rev() {
                self.kill_slot(y);
            }
        }
        self.boss92_b9a7(x);
    }

    fn boss92_b9a7(&mut self, x: usize) {
        self.fd &= 0xFD;
        let by = self.o[x].ypx;
        let (a, b) = 0x2Fu8.overflowing_sub(by);
        self.fa = a;
        if b {
            self.fa = a.wrapping_sub(0x0F).wrapping_sub(1);
            self.fd |= 2;
        }
        let bx = self.o[23].xpx;
        self.o[22].xpx = bx;
        self.o[21].xpx = bx;
        let y23 = self.o[23].ypx;
        self.o[21].ypx = y23;
        self.o[22].ypx = y23.wrapping_add(0x28);
    }

    /// `$B702`: wait, then pick the next height (random) and set off toward it.
    fn boss92_b702(&mut self, x: usize) {
        if self.o[x].s[0] != 0 {
            self.o[x].s[0] -= 1;
            if self.o[x].s[0] != 0 {
                return;
            }
            if self.o[22].aid != 0x6E {
                self.o[22].aid = 0x6E;
                self.o[22].aframe = 0;
                self.o[22].atick = 0;
            }
            return;
        }
        let sum = self.rng[0] as u16 + self.rng[1] as u16 + self.carry as u16;
        self.rng[1] = sum as u8;
        let y = (sum as u8 & 3) as u16;
        let target = self.ai_byte(0xBA70 + y);
        self.o[x].s[1] = target;
        let py = self.o[x].ypx;
        self.carry = target >= py;
        if target == py {
            self.boss92_b75c(x);
            return;
        }
        self.o[x].dir = if target >= py { 4 } else { 8 };
        self.o[x].ptr = 0xB742;
        self.boss92_b742(x);
    }

    fn boss92_b742(&mut self, x: usize) {
        self.carry = self.o[x].ypx >= self.o[x].s[1];
        if self.o[x].ypx == self.o[x].s[1] {
            self.boss92_b75c(x);
            return;
        }
        if self.o[x].dir & 8 != 0 {
            self.o[x].ypx = self.o[x].ypx.wrapping_sub(1);
            if self.o[x].ypx == 0 {
                self.o[x].ypx = 1;
            }
        } else {
            self.o[x].ypx = self.o[x].ypx.wrapping_add(1);
        }
        self.boss92_b7ce(x);
    }

    fn boss92_b75c(&mut self, x: usize) {
        let sum = self.rng[0] as u16 + self.rng[1] as u16 + self.carry as u16;
        self.rng[1] = sum as u8;
        let y = (sum as u8 & 1) as u16;
        self.o[x].s[2] = self.ai_byte(0xBA74 + y);
        self.face_player(x);
        self.o[x].ptr = 0xB778;
        self.boss92_b778(x);
    }

    /// `$B778`: sideways walk with turns at the room edges; it may start the drop when Mega Man is below.
    fn boss92_b778(&mut self, x: usize) {
        self.move_h(x);
        let px = self.o[x].xpx;
        self.carry = px >= 0xE0 || px >= 0x28;
        let turn = if px >= 0xE0 {
            self.o[x].dir & 1 != 0
        } else if px >= 0x28 {
            false
        } else {
            self.o[x].dir & 2 != 0
        };
        if turn {
            self.o[x].dir ^= 3;
        }
        self.o[x].s[2] = self.o[x].s[2].wrapping_sub(1);
        if self.o[x].s[2] == 0 {
            self.o[x].s[0] = 0x78;
            self.o[x].ptr = 0xB702;
            self.boss92_b7ce(x);
            return;
        }
        let d = self.dist_x(x);
        self.carry = d >= 0x10;
        if d < 0x10 {
            let sum = self.rng[0] as u16 + self.rng[1] as u16 + self.carry as u16;
            self.rng[0] = sum as u8;
            self.carry = sum > 0xFF;
            if sum as u8 & 7 == 0 {
                self.o[x].ptr = 0xB865;
                return;
            }
        }
        self.boss92_b7ce(x);
    }

    /// `$B7CE`: every `$04B0` frames it throws three shots `$94`; then the crush test (`$B83D`).
    fn boss92_b7ce(&mut self, x: usize) {
        self.o[x].s[3] = self.o[x].s[3].wrapping_sub(1);
        if self.o[x].s[3] == 0 {
            let sum = self.rng[0] as u16 + self.rng[1] as u16 + self.carry as u16;
            self.rng[0] = sum as u8;
            self.carry = sum > 0xFF;
            let k = (sum as u8 & 1) as u16;
            self.o[x].s[3] = self.ai_byte(0xBA76 + k);
            let saved = self.o[x].dir;
            self.face_player(x);
            let d11 = self.o[x].dir;
            let idx = (d11 & 1) | 0x52;
            self.o[x].dir = saved;
            let mut e = 2usize;
            loop {
                let Some(y) = self.find_slot() else { break };
                self.child_at(x, y, idx, 0x18);
                let c = &mut self.o[y];
                c.ty = 0x94;
                c.hitbox = 0x8E;
                let k = e as u16;
                let (a, b, cc, d) = (self.ai_byte(0xBA7E + k), self.ai_byte(0xBA81 + k), self.ai_byte(0xBA84 + k), self.ai_byte(0xBA87 + k));
                let (m, o) = (self.ai_byte(0xBA7B + k), self.ai_byte(0xBA78 + k));
                let c = &mut self.o[y];
                c.xvs = a;
                c.xvp = b;
                c.yvs = cc;
                c.yvp = d;
                c.dir = (d11 & m) | o;
                if e == 0 {
                    break;
                }
                e -= 1;
            }
        }
        self.boss92_b83d(x);
    }

    /// `$B83D`: if the press touches Mega Man (0x28 px below its top) it crushes him.
    fn boss92_b83d(&mut self, x: usize) {
        if self.p.state >= 2 {
            self.boss92_b89c(x);
            return;
        }
        let y = self.o[x].ypx;
        let hb = self.o[x].hitbox;
        self.o[x].ypx = y.wrapping_add(0x28);
        self.o[x].hitbox = 0;
        let hit = self.touches_player(x);
        self.o[x].hitbox = hb;
        self.o[x].ypx = y;
        self.carry = !hit;
        if hit {
            self.boss92_b8a9(x);
        } else {
            self.boss92_tail(x);
        }
    }

    fn boss92_b89c(&mut self, x: usize) {
        self.o[x].ptr = 0xB93A;
        self.boss92_tail(x);
    }

    /// `$B8A9`: the press has caught Mega Man: player state `$0C` (not ported).
    fn boss92_b8a9(&mut self, x: usize) {
        self.o[x].s[0] = 0x1E;
        self.o[x].ptr = 0xB8DB;
        self.o[22].aid = 0x6C;
        self.o[22].aframe = 0;
        self.o[22].atick = 0;
        self.player_cmd = Some(0x0C);
        self.p.state = 0x0C;
        self.player_ops.push(PlayerOp::Crush);
        self.boss92_tail(x);
    }

    /// `$B865`: the drop: down 4 px per frame to the floor of the room.
    fn boss92_b865(&mut self, x: usize) {
        self.o[x].ypx = self.o[x].ypx.wrapping_add(4);
        let lim = if self.o[x].xpx < 0x58 { 0x77 } else { 0x5F };
        if lim >= self.o[x].ypx {
            self.boss92_tail(x);
            return;
        }
        self.o[x].ypx = lim;
        if self.p.state >= 2 {
            self.boss92_b89c(x);
            return;
        }
        let y = self.o[x].ypx;
        self.o[x].ypx = y.wrapping_add(0x28);
        let hit = self.touches_player(x);
        self.o[x].ypx = y;
        if hit {
            self.boss92_b8a9(x);
        } else {
            self.boss92_b89c(x);
        }
    }

    /// `$B9E3`: the crushed Mega Man is carried along under the press.
    fn boss92_carry(&mut self, x: usize) {
        let y = self.o[x].ypx.wrapping_add(0x28);
        let px = self.o[x].xpx;
        self.p.xpx = px;
        self.player_ops.push(PlayerOp::SetY(y));
        self.player_ops.push(PlayerOp::SetX(px));
    }

    fn boss92_b8db(&mut self, x: usize) {
        if self.o[x].s[0] != 0 {
            self.boss92_carry(x);
            self.o[x].s[0] -= 1;
            self.boss92_tail(x);
            return;
        }
        self.o[x].ypx = self.o[x].ypx.wrapping_sub(1);
        self.boss92_carry(x);
        if 0x2F < self.o[x].ypx {
            self.boss92_tail(x);
            return;
        }
        self.o[x].ypx = 0x2F;
        self.o[x].s[0] = 0x3C;
        self.o[x].ptr = 0xB90A;
        self.boss92_tail(x);
    }

    fn boss92_b90a(&mut self, x: usize) {
        if self.o[x].s[0] != 0 {
            self.boss92_carry(x);
            self.o[x].s[0] -= 1;
            self.boss92_tail(x);
            return;
        }
        self.o[22].aid = 0x6D;
        self.o[22].aframe = 0;
        self.o[22].atick = 0;
        self.o[x].ypx = self.o[x].ypx.wrapping_add(4);
        if self.o[x].ypx == 0x37 {
            self.o[x].s[0] = 0x3C;
            self.o[x].ptr = 0xB702;
        }
        self.boss92_tail(x);
    }

    fn boss92_b93a(&mut self, x: usize) {
        let (y, t) = (self.o[x].ypx, self.o[x].s[1]);
        if y == t {
            self.o[x].s[0] = 0x3C;
            self.o[x].ptr = 0xB702;
        } else if y < t {
            self.o[x].ypx = y.wrapping_add(1);
        } else {
            self.o[x].ypx = y.wrapping_sub(1);
            if self.o[x].ypx == 0 {
                self.o[x].ypx = 1;
            }
        }
        self.boss92_tail(x);
    }

    /// `$B9F3`: after the last hit the press rises and slides to the middle (Mega Man is in state `$0D`, not ported).
    fn boss92_b9f3(&mut self, x: usize) {
        if self.o[x].ypx != 0x2F {
            self.o[x].ypx = self.o[x].ypx.wrapping_sub(1);
        } else {
            let px = self.o[x].xpx;
            if px == 0xC0 {
                if self.f0 != 8 {
                    self.f0 = 0;
                }
                if self.o[x].aid == 0x76 {
                    self.o[x].ptr = 0xBA36;
                }
                return;
            }
            self.o[x].xpx = if px < 0xC0 { px.wrapping_add(1) } else { px.wrapping_sub(1) };
        }
        self.boss92_b9a7(x);
    }
}
