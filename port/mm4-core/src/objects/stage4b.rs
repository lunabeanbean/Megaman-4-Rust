//! Stage 4 machine (list type `18`, AI `$24`, bank `$3D` `$A33B`-`$A5CD`): it rises from below the screen, then keeps spawning drifting blocks
//! (type `25`, AI `$A55B`) and pushes / breaks the objects in slots `$11..$15` (the block rows).
use super::*;

impl World {
    pub(super) fn run_stage4b(&mut self, x: usize, ptr: u16) -> bool {
        match ptr {
            0xA33B => self.ai_a33b(x),
            0xA356 => self.ai_a356(x),
            0xA3D7 => self.ai_a3d7(x),
            0xA416 => self.ai_a416(x),
            0xA55B => self.ai_a55b(x),
            _ => return false,
        }
        true
    }


    /// `$A33B`: freeze Mega Man, ask for the screen redraw, wait for it in `$A356`.
    fn ai_a33b(&mut self, x: usize) {
        self.a5c5();
        self.p1b = 0x80;
        self.p1c = 0x1A;
        self.o[x].ptr = 0xA356;
        self.o[x].atick = 0;
    }

    /// `$A40D`: the scroll register `$F1` follows the machine.
    fn a40d(&mut self, x: usize) {
        self.f1 = self.o[x].ypx.wrapping_sub(0x24);
    }

    /// `$A356`: after the redraw the machine climbs in from below (`$F0` = 1), then drops in with the animation `5B` and lands at Y `$44`.
    fn ai_a356(&mut self, x: usize) {
        if self.f0 == 0 {
            if self.p1b != 0 {
                return;
            }
            self.f0 = 1;
            self.a40d(x);
            return;
        }
        self.a5c5();
        if self.o[x].aid != 0x5B {
            let o = &mut self.o[x];
            let (s, b) = o.ysub.overflowing_sub(0x80);
            o.ysub = s;
            o.ypx = o.ypx.wrapping_sub(b as u8);
            if o.ypx == 0x44 {
                self.set_anim(x, 0x5B);
                self.o[x].flags &= !4;
                self.o[x].ypx = 0;
                return;
            }
            self.a40d(x);
            return;
        }
        self.move_gravity(x);
        if 0x44 >= self.o[x].ypx {
            self.o[x].atick = 0;
            return;
        }
        self.o[x].ypx = 0x44;
        if self.o[x].aframe != 3 {
            return;
        }
        self.set_anim(x, 0x0C);
        self.o[22].flags &= !4;
        self.o[x].ptr = 0xA3D7;
        self.a40d(x);
    }

    /// `$A3D7`: a small bounce (3 frames down, 3 up) before the working state.
    fn ai_a3d7(&mut self, x: usize) {
        self.o[x].atick = 0;
        let a = self.o[x].s[1];
        self.o[x].s[1] = a.wrapping_add(1);
        if a < 3 {
            self.o[x].ypx = self.o[x].ypx.wrapping_add(1);
        } else {
            self.o[x].ypx = self.o[x].ypx.wrapping_sub(1);
        }
        self.o[22].ypx = self.o[x].ypx;
        if self.o[x].s[1] == 6 {
            self.o[x].ptr = 0xA416;
            self.o[x].s[1] = 0x3C;
            self.player_cmd = Some(0); // code_3DA5CE
        }
        self.a40d(x);
    }

    /// `$A416`: spawn blocks (at most 2 alive) while the animation is at frame 6 or 8, then look after the block rows in slots `$11..$15`.
    fn ai_a416(&mut self, x: usize) {
        if self.o[x].atick == 0 && matches!(self.o[x].aframe, 6 | 8) {
            let alive = (8..=0x17usize).filter(|&s| self.o[s].ty == 0x25).count();
            if alive != 2 {
                if let Some(y) = self.find_slot() {
                    self.child_at(x, y, 0x10, 0x10);
                    let c = &mut self.o[y];
                    c.ty = 0x25;
                    c.hitbox = 0xCE;
                    c.hp = 1;
                    c.s[0] = 0x40;
                    c.s[1] = 1;
                    c.s[3] = 0x0C;
                    c.dir = 2;
                    c.xvs = 0;
                    c.xvp = 2;
                }
            }
        }
        for s in (0x11..=0x15usize).rev() {
            if self.o[s].aid == 0x17 && self.o[s].aframe == 5 {
                self.a4de(s);
                continue;
            }
            if self.o[s].ypg != 0 || self.o[s].ypx >= 0xB6 {
                self.a525(s);
                continue;
            }
            while let Some(shot) = self.shot_hits(s) {
                let mut dmg = 1; // `$A5B5[$A0]` for the buster
                if self.o[shot].aid == 0x23 {
                    dmg += 1;
                }
                if std::env::var("MM4_COVER").is_ok() {
                    eprintln!("COVER shot hits slot {:02X}", s);
                }
                self.kill_slot(shot);
                self.o[s].s[0] = self.o[s].s[0].wrapping_add(dmg);
                if self.o[s].s[0] >= 2 {
                    self.o[s].s[0] = 0;
                    self.set_anim(s, 0x17);
                    break;
                }
            }
        }
        self.a40d(x);
    }

    /// `$A4DE`: a block row was destroyed: its slot restarts, the rows above it drop by 16 px.
    fn a4de(&mut self, s: usize) {
        if std::env::var("MM4_COVER").is_ok() {
            eprintln!("COVER a4de slot {:02X}", s);
        }
        self.set_anim(s, 0);
        self.o[s].s[1] = 0x78;
        self.o[s].s[2] = 0x10;
        let top = self.o[s].ypx;
        let v = self.o[23].ypx.wrapping_add(0x10);
        self.o[23].ypx = v;
        self.o[22].ypx = v;
        let (y, c) = v.overflowing_add(0x71);
        self.o[s].ypx = y;
        self.o[s].ypg = self.o[s].ypg.wrapping_add(c as u8);
        for k in (0x11..=0x15usize).rev() {
            if self.o[k].ypg == 0 && self.o[k].ypx < top {
                self.o[k].ypx = self.o[k].ypx.wrapping_add(0x10);
            }
        }
    }

    /// `$A525`: after its delay a row slot below the screen edge makes slots `$11..$17` creep up by 1 px.
    fn a525(&mut self, s: usize) {
        if std::env::var("MM4_COVER").is_ok() {
            eprintln!("COVER a525 slot {:02X}", s);
        }
        if self.o[s].s[1] != 0 {
            self.o[s].s[1] -= 1;
            if self.o[s].s[1] != 0 {
                return;
            }
        }
        if self.o[s].ypg != 0 || self.o[s].ypx >= 0xC6 {
            return;
        }
        for k in (0x11..=0x17usize).rev() {
            let o = &mut self.o[k];
            let (p, b) = o.ypx.overflowing_sub(1);
            o.ypx = p;
            o.ypg = o.ypg.wrapping_sub(b as u8);
        }
        self.o[s].s[2] = self.o[s].s[2].wrapping_sub(1);
    }

    /// code_3FFD17: direction flags and speeds from heading `y` (`a` = 0 or 8 picks the speed table).
    pub(super) fn heading_speed(&mut self, x: usize, y: u8, a: u8) {
        self.o[x].dir = self.rom.cpu_byte(0x3A, 0x3B, 0xF796 + y as u16);
        let i = ((y & 7) | a) as u16;
        self.o[x].xvs = self.rom.cpu_byte(0x3A, 0x3B, 0xFD45 + i);
        self.o[x].xvp = self.rom.cpu_byte(0x3A, 0x3B, 0xFD5D + i);
        let j = i ^ 4;
        self.o[x].yvs = self.rom.cpu_byte(0x3A, 0x3B, 0xFD45 + j);
        self.o[x].yvp = self.rom.cpu_byte(0x3A, 0x3B, 0xFD5D + j);
    }

    /// `$A55B` (type `25`): a block that drifts and steers toward Mega Man every 8 frames while its counter `$0468/$0480` runs down.
    fn ai_a55b(&mut self, x: usize) {
        if self.o[x].flags & 0x80 == 0 {
            self.kill_slot(x);
            return;
        }
        let n = (self.o[x].s[1] as u16) << 8 | self.o[x].s[0] as u16;
        if n != 0 {
            let n = n - 1;
            self.o[x].s[0] = n as u8;
            self.o[x].s[1] = (n >> 8) as u8;
            if n as u8 & 7 == 0 {
                self.aim(x);
                let h = self.o[x].s[3];
                self.o[x].aid = self.rom.cpu_byte(0x3A, 0x3D, 0xA5A5 + h as u16);
                self.heading_speed(x, h, 0);
            }
        }
        self.move_h(x);
        self.face_flag(x);
        self.move_v(x);
    }
}
