//! Stage 8 enemies: the wall crawler `02` (AI `$13`, bank `$3A` `$91DF`/`$921E`) and the bobbing flyers `81`/`84` (AI `$CA`/`$C7`, bank `$3B` `$BA92`/`$BAD9`/`$BB20`).
use super::*;

impl World {
    pub(super) fn run_stage8(&mut self, st: &Stage, x: usize, ptr: u16) -> bool {
        match (self.f6, ptr) {
            (_, 0x91DF) => self.ai_91df(st, x),
            (_, 0x921E) => self.ai_921e(st, x),
            (0x3B, 0xBA92) => self.ai_ba92(st, x),
            (0x3B, 0xBAD9) => self.ai_bad9(st, x),
            (0x3B, 0xBB20) => self.ai_bb20(x),
            _ => return false,
        }
        true
    }

    /// `$3A92C7`: look for the wall the crawler follows. Returns the Y register the original leaves (the old animation frame).
    fn c92c7(&mut self, st: &Stage, x: usize) -> u8 {
        let id = (((self.o[x].dir >> 1) & 1) | 0x0C) as usize;
        let (t, _) = self.hprobe(st, x, id);
        self.pt = t;
        let mut found = None;
        if !self.pt.iter().any(|t| t & 1 != 0) {
            let id = self.o[x].s[1] as usize;
            let (t, _) = self.hprobe(st, x, id);
            self.pt = t;
            found = (0..3).find(|&k| self.pt.get(k).map_or(false, |t| t & 1 != 0));
        }
        let y = match found {
            Some(k) => k as u8,
            None => {
                let o = &mut self.o[x];
                o.flags ^= 0x40;
                o.dir ^= 3;
                o.aframe = 0x17;
                0
            }
        };
        self.o[x].s[0] = y;
        let old = self.o[x].aframe;
        self.o[x].aframe = self.ai_byte(0x9364 + y as u16);
        self.o[x].atick = 0;
        old
    }

    /// `$3A931A`: step along the wall by the tables indexed by `y`.
    fn c931a(&mut self, x: usize, y: u8) {
        let y = y as u16;
        let dy = self.ai_byte(0x9397 + y);
        self.o[x].ypx = self.o[x].ypx.wrapping_add(dy);
        let a = if self.o[x].dir & 1 == 0 { self.ai_byte(0x9367 + y) } else { self.ai_byte(0x937F + y) };
        let hi: u8 = if a & 0x80 != 0 { 0xFF } else { 0 };
        let (px, c) = self.o[x].xpx.overflowing_add(a);
        self.o[x].xpx = px;
        self.o[x].xpg = self.o[x].xpg.wrapping_add(hi).wrapping_add(c as u8);
    }

    fn c92bd(&mut self, x: usize) {
        let a = self.ai_byte(0x934F + self.o[x].aframe as u16);
        self.o[x].hitbox = a;
    }

    /// `$3A91DF`: first state: every 8th frame it looks for the wall and takes a step.
    fn ai_91df(&mut self, st: &Stage, x: usize) {
        self.c92bd(x);
        self.face_flag(x);
        self.o[x].atick = 0;
        let a = self.o[x].s[3];
        self.o[x].s[3] = a.wrapping_add(1);
        if a != 8 {
            return;
        }
        self.o[x].s[3] = a & 7;
        self.o[x].s[1] = ((self.o[x].dir & 2) >> 1) | 8;
        self.c92c7(st, x);
        let y = if self.o[x].s[0] != 0 { 0 } else { 0x12 };
        self.c931a(x, y);
        self.o[x].ptr = 0x921E;
    }

    /// `$3A921E`: second state (the walk around corners).
    fn ai_921e(&mut self, st: &Stage, x: usize) {
        self.c92bd(x);
        self.o[x].atick = 0;
        let a = self.o[x].s[3];
        self.o[x].s[3] = a.wrapping_add(1);
        if a != 8 {
            return;
        }
        self.o[x].s[3] = a & 7;
        let f = self.o[x].aframe;
        let mut y: u8 = 8;
        let corner = if f == 0x0A {
            true
        } else {
            y = 10;
            f == 5 || f == 0x0F
        };
        if !corner {
            if f == 0x11 {
                let k = self.o[x].s[0] as u16;
                let nf = self.ai_byte(0x9364 + k);
                self.o[x].aframe = nf;
                let y = if nf == 6 { 6 } else { 0x15 };
                self.c931a(x, y);
                return;
            }
            self.c931a(x, f);
            self.o[x].aframe = f.wrapping_add(1);
            return;
        }
        let mut s1 = y;
        if self.o[x].dir & 1 == 0 {
            s1 += 1;
        }
        self.o[x].s[1] = s1;
        self.o[x].s[2] = self.o[x].s[0];
        let mut y = self.c92c7(st, x);
        let (s0, s2) = (self.o[x].s[0], self.o[x].s[2]);
        if s0 == s2 {
            if s0 == 1 {
                self.o[x].aframe = 0;
            }
        } else if s0 == 0 {
            y = if s2 == 1 { 0x16 } else { 0x11 };
        } else {
            y = 0x14;
            if s2 != 0 {
                y = 0x10;
                if s2 != 1 {
                    y = 0x13;
                    self.o[x].aframe = 0x11;
                }
            } else {
                self.o[x].aframe = 0x11;
            }
        }
        self.c931a(x, y);
    }

    // ---- flyers 81/84 (AI $CA/$C7) --------------------------------------------------------------------------------------------------

    fn bb52(&mut self, x: usize, a: u8) {
        let k = (a & 1) as u16;
        let d = self.ai_byte(0xBB6A + k);
        self.o[x].xpx = self.o[x].xpx.wrapping_add(d);
    }

    /// `$3BBA92`: hover for a while, then dive toward Mega Man's height.
    fn ai_ba92(&mut self, st: &Stage, x: usize) {
        self.o[x].atick = 0;
        if self.o[x].s[0] == 0 {
            self.o[x].s[0] = 0x1E;
        }
        self.o[x].s[0] -= 1;
        if self.o[x].s[0] != 0 {
            return;
        }
        let y = if self.o[x].ypx >= self.p.ypx { 8 } else { 4 };
        let o = &mut self.o[x];
        o.dir = y;
        o.yvs = 0xCC;
        o.yvp = 0;
        o.s[2] = o.s[2].wrapping_add(1);
        let k = (o.s[2] & 3) as u16;
        self.o[x].s[1] = self.ai_byte(0xBB66 + k);
        self.o[x].ptr = 0xBAD9;
        self.ai_bad9(st, x);
    }

    fn ai_bad9(&mut self, st: &Stage, x: usize) {
        self.o[x].atick = 0;
        self.bad9(st, x);
    }

    fn bad9(&mut self, st: &Stage, x: usize) {
        let y = (((self.o[x].ty & 1) << 1) + 0x34) as usize;
        self.dir_v_collide(st, x, y);
        let or = self.pt.iter().fold(0u8, |a, &t| a | t);
        let mut flip = true;
        if or == 2 || or == 4 {
            let py = self.o[x].ypx;
            if py >= 8 && py < 0xE8 {
                flip = false;
            }
        }
        if flip {
            self.o[x].dir ^= 0x0C;
        }
        self.o[x].s[1] = self.o[x].s[1].wrapping_sub(1);
        if self.o[x].s[1] != 0 {
            return;
        }
        self.o[x].ptr = 0xBB20;
        self.o[x].aframe += 1;
        let t = self.o[x].ty;
        self.bb52(x, t);
    }

    /// `$3BBB20`: wing flap after the dive, then back to `$BA92`.
    fn ai_bb20(&mut self, x: usize) {
        if self.o[x].s[1] != 0 {
            self.o[x].atick = 0;
            self.o[x].s[1] -= 1;
            if self.o[x].s[1] != 0 {
                return;
            }
            self.o[x].aframe += 1;
        }
        let f = self.o[x].aframe;
        if f == 6 {
            self.o[x].s[1] = 0x3C;
            return;
        }
        if f != 9 {
            return;
        }
        self.o[x].ptr = 0xBA92;
        self.o[x].aframe = 0;
        self.o[x].atick = 0;
        let a = self.o[x].aid;
        self.bb52(x, a);
    }
}
