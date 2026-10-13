//! Stage 14 final scene, second half: after the machine's first defeat (`$AC41`) it settles, fights again (`$AD16`), falls apart (`$ADD6`) and leaves (`$AE16`).
use super::*;

impl World {
    pub(super) fn run_wily14b(&mut self, x: usize, ptr: u16) -> bool {
        if self.f6 != 0x35 {
            return false;
        }
        match ptr {
            0xAC41 => self.wily_ac41(x),
            0xACC5 => self.wily_acc5(x),
            0xAD16 => self.wily_ad16(x),
            0xADD6 => self.wily_add6(x),
            0xAE16 => self.wily_ae16(x),
            0xAE4F => self.wily_ae4f(x),
            0xAE61 => self.wily_ae61(x),
            _ => return false,
        }
        true
    }

    /// `$AC41` (the replacement of `BC` when its hit points are gone): full hit points again, no hitbox; explosions `3C` and the PPU buffer of the machine's
    /// damaged picture (`$AEDF`: copied only for the graphics, `s[0]` = index after its `$80` terminator); on the second visit it becomes `$ACC5`.
    fn wily_ac41(&mut self, x: usize) {
        self.o[x].hp = 0x1C;
        self.o[x].hitbox = 0;
        if self.o[x].s[0] == 0 {
            if let Some(y) = self.find_slot() {
                self.child_here(x, y, 0x62);
                self.o[y].ty = 0x3C;
                self.o[y].hitbox = 0;
            }
        }
        let mut y = self.o[x].s[0] as u16;
        loop {
            let b = self.ai_byte(0xAEDF + y);
            y += 1;
            if b == 0x80 {
                break;
            }
        }
        self.carry = true; // CMP #$80 leaves the carry set
        self.o[x].s[0] = y as u8;
        self.o[x].s[1] = self.o[x].s[1].wrapping_add(1);
        if self.o[x].s[1] != 2 {
            return;
        }
        self.o[x].ptr = 0xACC5;
        self.set_anim(23, 0x93);
        self.set_anim(x, 0x94);
        let o = &mut self.o[x];
        o.xpx = o.xpx.wrapping_sub(0x10);
        o.ypx = o.ypx.wrapping_add(8);
        o.hitbox = 0xCD;
        self.bf = 0;
        self.kill_slot(21);
        self.kill_slot(20);
    }

    /// `$ACC5`: Mega Man is frozen again; the machine settles at Y `$70` while the meter refills; then the second phase starts.
    fn wily_acc5(&mut self, x: usize) {
        if self.fd9c(x) != 9 {
            return;
        }
        self.scroll6a = 2;
        let ypx = self.o[x].ypx;
        if ypx == 0x70 {
            if self.p95 & 7 == 0 {
                self.snd(0x29, 0xACEE); // code_35ACEB
                self.bf = self.bf.wrapping_add(1);
                if self.bf == 0x1C {
                    self.player_cmd = Some(0);
                    self.p.state = 0;
                    let o = &mut self.o[x];
                    o.ptr = 0xAD16;
                    o.dir = 2;
                    o.s[0] = 1;
                    o.s[1] = 0x28;
                }
            }
        } else if ypx > 0x70 {
            self.o[x].ypx = ypx.wrapping_sub(1);
        } else {
            self.o[x].ypx = ypx.wrapping_add(1);
        }
        self.wily_aba6(x);
    }

    /// `$AD16`: the second phase: the machine slides between X `$50` and `$90`, turning at random intervals, and every 40 frames throws a `BF`.
    fn wily_ad16(&mut self, x: usize) {
        self.o[x].s[0] = self.o[x].s[0].wrapping_sub(1);
        if self.o[x].s[0] == 0 {
            let sum = self.rng[0] as u16 + self.rng[3] as u16 + self.carry as u16;
            self.rng[3] = sum as u8;
            self.carry = sum > 0xFF;
            let i = (sum as u8 & 7) as u16;
            self.o[x].s[0] = self.ai_byte(0xAF8A + i);
            self.o[x].dir ^= 3;
        }
        self.move_h(x);
        self.o[x].flags = 0x90;
        let xpx = self.o[x].xpx;
        let (lim, clamp) = if self.o[x].dir & 1 != 0 { (0x90u8, 0x90 < xpx) } else { (0x50u8, 0x50 >= xpx) };
        if clamp {
            self.o[x].xpx = lim;
            self.o[x].dir ^= 3;
        }
        self.o[x].s[1] = self.o[x].s[1].wrapping_sub(1);
        if self.o[x].s[1] == 0 {
            self.o[x].s[1] = 0x28;
            let straight = self.o[x].xpx < self.p.xpx || self.dist_y(x) < 0x30;
            if let Some(y) = self.find_slot() {
                self.child_at(23, y, 0x67, 0x95);
                if straight {
                    self.o[y].xvs = 0;
                    self.o[y].xvp = 2;
                    self.o[y].dir = 2;
                } else {
                    let bits = self.aim_fb8c(y, 0x00, 0x02);
                    self.o[y].dir = bits;
                }
                let k = &mut self.o[y];
                k.flags = 0x90;
                k.ty = 0xBF;
                k.hitbox = 0x8D;
            }
        }
        self.wily_aba6(x);
    }

    /// `$ADD6`: the machine drops (speed 0.2) throwing explosions every 8 frames until Y `$88`.
    fn wily_add6(&mut self, x: usize) {
        self.o[x].yvs = 0x33;
        self.o[x].yvp = 0;
        self.down(x);
        self.wily_aba6(x);
        if self.p95 & 7 == 0 {
            if let Some(y) = self.find_slot() {
                self.child_here(23, y, 0x62);
                self.o[y].ty = 0x3C;
                self.o[y].hitbox = 0;
            }
        }
        if self.o[x].ypx == 0x88 {
            self.o[x].ptr = 0xAE16;
        }
    }

    /// `$AE16`: the machine is gone: all enemies vanish, a burst of 16 particles, and what is left of the object rises off the top (`$AE4F`).
    fn wily_ae16(&mut self, x: usize) {
        self.scroll69 = 0;
        self.scroll6a = 0;
        self.fa = 0;
        self.kill_slot(23);
        for y in (8..=21).rev() {
            self.kill_slot(y);
        }
        self.set_anim(x, 0x97);
        self.o[x].xpx = self.o[23].xpx;
        self.o[x].yvs = 0;
        self.o[x].yvp = 1;
        self.wily_burst();
        self.o[x].ptr = 0xAE4F;
        self.wily_ae4f(x);
    }

    fn wily_ae4f(&mut self, x: usize) {
        self.up(x);
        if self.o[x].ypg == 0 {
            return;
        }
        self.o[x].ptr = 0xAE61;
        self.wily_ae61(x);
    }

    /// `$AE61`: Mega Man (free again) starts his victory walk and the object is removed.
    fn wily_ae61(&mut self, x: usize) {
        if self.p.state >= 3 {
            return;
        }
        self.despawn(x);
        self.snd_music(0x0C, 0xAE6F); // code_35AE6C
        self.p.state = 0x0B;
        self.player_cmd = Some(0x0B);
        for y in 1..=4 {
            self.kill_slot(y);
        }
    }

    /// `$AE7C`: the 16 death particles `2E` of the machine (code_3FFB7C: slots 23 down to 5).
    fn wily_burst(&mut self) {
        self.snd_music(0xF0, 0xAE81); // code_35AE7C
        self.snd(0x25, 0xAE86);
        for k in (0..16u16).rev() {
            let Some(y) = (5..24usize).rev().find(|&s| self.o[s].ty == 0) else { break };
            self.child_here(23, y, 0x19);
            let (a, b, c, d) = (self.ai_byte(0xAF94 + k), self.ai_byte(0xAFA4 + k), self.ai_byte(0xAFB4 + k), self.ai_byte(0xAFC4 + k));
            let p = &mut self.o[y];
            p.flags = 0x90;
            p.ty = 0x2E;
            p.hitbox = 0;
            p.xvs = a;
            p.xvp = b;
            p.yvs = c;
            p.yvp = d;
        }
    }
}
