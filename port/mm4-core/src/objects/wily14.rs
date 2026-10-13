//! Stage 14, page `0B`: the final Wily scene. Controller object `BC` (list type `53`, AI chain in bank `$35` from `$AA52`) in slot 22 moves the machine `BB`
//! (slot 23) and two parts (slots 20, 21); `BF` are its shots.
use super::*;

impl World {
    pub(super) fn run_wily14(&mut self, st: &Stage, x: usize, ptr: u16) -> bool {
        if self.f6 != 0x35 {
            return false;
        }
        match ptr {
            0xAA52 => self.wily_aa52(x),
            0xAA6B => self.wily_aa6b(x),
            0xAAB1 => self.wily_aab1(x),
            0xAAD8 => self.wily_aad8(x),
            0xAB0C => self.wily_ab0c(x),
            0xAC13 => self.wily_ac13(x),
            0xAC3B => {
                self.move_h(x);
                self.move_v(x);
            }
            _ => {
                let _ = st;
                return false;
            }
        }
        true
    }

    /// `$AA52`: wait until Mega Man is frozen (state 9), ask for the screen redraw (`$1B = $80`, `$1C = $0C`).
    fn wily_aa52(&mut self, x: usize) {
        if self.fd9c(x) != 9 {
            return;
        }
        self.p1c = 0x0C;
        self.p1b = 0x80;
        self.o[x].ptr = 0xAA6B;
    }

    /// `$AA6B`: after the redraw the scroll mode `$F0` = 9 starts and the machine drops in from above (speed 0.5 px/frame).
    fn wily_aa6b(&mut self, x: usize) {
        if self.p1b != 0 {
            return;
        }
        self.scroll69 = 0;
        self.scroll6a = 0;
        self.f1 = 0xC0;
        self.f0 = 9;
        let o = &mut self.o[x];
        o.ptr = 0xAAB1;
        o.ypg = 0xFF;
        o.flags = 0x90;
        o.yvs = 0x80;
        o.xvs = 0x80;
        o.yvp = 0;
        o.xvp = 0;
        o.dir = 4;
        self.o[23].ypg = 0xFF;
        self.o[23].flags = 0x90;
        self.o[21].flags = 0x90;
        self.o[20].flags = 0x90;
    }

    /// `$AAB1`: fall until Y `$70` (page 0), then the boss meter fills.
    fn wily_aab1(&mut self, x: usize) {
        self.down(x);
        self.wily_aba6(x);
        if self.o[x].ypg != 0 || self.o[x].ypx != 0x70 {
            return;
        }
        self.p95 = 0;
        self.bf = 0;
        self.meter2 = 0x8F;
        self.o[x].ptr = 0xAAD8;
        self.wily_aad8(x);
    }

    /// `$AAD8`: the meter fills (one unit every 8 frames, 28 units); then Mega Man is free and the machine starts to move and shoot.
    fn wily_aad8(&mut self, x: usize) {
        if self.p95 & 7 != 0 {
            return;
        }
        self.snd(0x29, 0xAAE3); // code_35AAE0
        self.bf = self.bf.wrapping_add(1);
        if self.bf != 0x1C {
            return;
        }
        self.player_cmd = Some(0);
        self.p.state = 0;
        let o = &mut self.o[x];
        o.ptr = 0xAB0C;
        o.s[0] = 0x20;
        o.s[1] = 0x28;
        self.o[21].flags = 0x94;
        self.o[20].flags = 0x94;
    }

    /// `$AB0C`: the machine hovers up and down (direction flags toggled every 64 frames), its arms (slots 20/21, animation) and every `s[1]` frames a shot `BF`.
    fn wily_ab0c(&mut self, x: usize) {
        self.move_v(x);
        if self.o[21].flags & 4 == 0 && self.o[21].aframe == 0x12 {
            self.o[21].flags = 0x94;
            self.o[20].flags = 0x94;
        }
        self.o[x].s[0] = self.o[x].s[0].wrapping_sub(1);
        if self.o[x].s[0] == 0 {
            self.o[x].s[0] = 0x40;
            self.o[x].dir ^= 0x0C;
            if self.o[x].dir & 8 != 0 {
                self.o[21].flags = 0x90;
                self.o[20].flags = 0x90;
                self.o[21].aframe = 0;
                self.o[20].aframe = 0;
                self.o[21].atick = 0;
                self.o[20].atick = 0;
            }
        }
        if self.o[x].s[1] == 0 {
            let y = self.o[x].s[2] as u16;
            self.o[x].s[1] = self.ai_byte(0xAEDC + y);
            if let Some(c) = self.find_slot() {
                self.child_at(23, c, 0x67, 0x95);
                let k = &mut self.o[c];
                k.ty = 0xBF;
                k.hitbox = 0x8D;
                k.xvs = 0xF6;
                k.xvp = 0x01;
                k.yvs = 0x63;
                k.yvp = 0x00;
                k.dir = 6;
                let o = &mut self.o[x];
                o.s[2] = o.s[2].wrapping_add(1);
                if o.s[2] == 3 {
                    o.s[2] = 0;
                }
            }
        }
        self.o[x].s[1] = self.o[x].s[1].wrapping_sub(1);
        self.wily_aba6(x);
    }

    /// `$AC13`: the shot `BF` hangs at the machine's side until its animation reaches frame 7, then it flies off by its direction flags.
    fn wily_ac13(&mut self, x: usize) {
        self.o[x].ypx = self.o[23].ypx.wrapping_add(0x10);
        self.o[x].xpx = self.o[23].xpx.wrapping_sub(0x28);
        if self.o[x].aframe != 7 {
            return;
        }
        self.set_anim(x, 0x96);
        self.o[x].ptr = 0xAC3B;
        self.move_h(x);
        self.move_v(x);
    }

    /// `$ABA6`: the machine (slot 23) and its arms (slots 20/21) follow the controller; `$FA` follows the machine's Y.
    pub(super) fn wily_aba6(&mut self, x: usize) {
        let (xpx, ypx, ypg) = (self.o[x].xpx, self.o[x].ypx, self.o[x].ypg);
        if self.o[x].aid != 0x30 {
            self.o[23].xpx = xpx.wrapping_add(0x30);
            self.o[23].ypx = ypx.wrapping_add(8);
        } else {
            self.o[23].xpx = xpx.wrapping_add(0x20);
            self.o[23].ypx = ypx;
            self.o[23].ypg = ypg;
            let (mut a, c) = ypx.overflowing_add(0x30);
            if c || a >= 0xF0 {
                a = a.wrapping_add(0x10);
            }
            self.o[21].ypx = a;
            self.o[20].ypx = a;
            let bx = self.o[23].xpx;
            self.o[21].xpx = bx;
            self.o[20].xpx = bx.wrapping_add(0x1C);
        }
        self.scroll69 = 0xC0u8.wrapping_sub(self.o[23].xpx);
        let (fa, b) = 0xAFu8.overflowing_sub(self.o[23].ypx);
        self.fa = fa;
        if b {
            self.fa = fa.wrapping_sub(0x10);
            self.scroll6a = 2;
        }
    }
}
