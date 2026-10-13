//! Stage 6, bank `$3D`: the water-level scroll objects `6B`/`6C` (AI `$AF2B`), the mini boss `38` (AI `$52`: `$AAAF`/`$AB28`, shots `53` = `$AC29`, falling blocks `54` = `$AC6A`/`$ACE7`)
//! and the death replacement `90` (`$A5D3`).
use super::*;

impl World {
    /// Dispatch for these states (bank `$3D` only: other banks reuse some of the addresses).
    pub(super) fn run_stage6b(&mut self, x: usize, ptr: u16) -> bool {
        if self.f6 != 0x3D {
            return false;
        }
        match ptr {
            0xAF2B => {
                self.p1c = 0x1E;
                self.p1b = 0x80;
                self.o[x].ptr = 0xAF3F;
                self.af78(x);
            }
            0xAF3F => self.ai_af3f(x),
            0xAAAF => self.ai_aaaf(x),
            0xAB28 => self.ai_ab28(x),
            0xAC29 => self.ai_ac29(x),
            0xAC6A => self.ai_ac6a(x),
            0xACE7 => self.ai_ace7(x),
            0xA5D3 => self.ai_a5d3(),
            _ => return false,
        }
        true
    }

    // ---- water-level scroll objects `6B`/`6C` -----------------------------------------------------------------------------------

    /// `$3DAF78`: the object's Y is the background scroll (`$FA`, `$FD`) and, through `$AF87`, the water level.
    fn af78(&mut self, x: usize) {
        let y = self.o[x].ypx;
        self.fa = y;
        self.fd = if y != 0 { 2 } else { 0 };
        self.af87(x);
    }

    /// `$3DAF87`: `$013E` = 0x10 - (Y + 0x10) (8-bit, with the carry quirk of the original), then follow Mega Man's X.
    fn af87(&mut self, x: usize) {
        let y = self.o[x].ypx;
        let a = if y == 0 {
            0
        } else {
            let (t, c) = y.overflowing_add(0x10);
            (!t).wrapping_add(1).wrapping_add(c as u8)
        };
        self.w13e = a.wrapping_add(0x10);
        self.follow_player_x(x);
    }

    /// `$3DAF3F`: once the renderer is idle sets the scroll mode (`$F0` = 5, `$F1` = $78); then moves up and down by 72 px (speed 0.5, flips every 144 frames).
    fn ai_af3f(&mut self, x: usize) {
        if self.f0 == 0 {
            if self.p1b != 0 {
                self.af87(x);
                return;
            }
            self.f1 = 0x78;
            self.f0 = 5;
            let o = &mut self.o[x];
            o.dir = 8;
            o.s[0] = 0x90;
            o.yvs = 0x80;
            o.yvp = 0;
        }
        self.move_v(x);
        self.o[x].s[0] = self.o[x].s[0].wrapping_sub(1);
        if self.o[x].s[0] == 0 {
            self.o[x].dir ^= 0x0C;
            self.o[x].s[0] = 0x90;
        }
        self.af78(x);
    }

    // ---- mini boss `38` (AI `$52`) ----------------------------------------------------------------------------------------------

    /// `$3DAAAF`: drops in from the top of the screen to Y `$6F`, then (animation frame 3) starts its fight: the parts in slots 20-22 become active.
    fn ai_aaaf(&mut self, x: usize) {
        if self.o[x].aframe == 0 {
            self.move_gravity(x);
            if self.o[x].ypx > 0x6F {
                self.o[x].ypx = 0x6F;
                self.o[x].aframe += 1;
            }
            self.o[x].atick = 0;
        }
        if self.o[x].aframe != 3 {
            return;
        }
        self.set_anim(x, 0x80);
        for s in 20..=22 {
            self.o[s].flags &= 0xFB;
        }
        let o = &mut self.o[x];
        o.ptr = 0xAB28;
        o.yvs = 0x20;
        o.yvp = 0;
        o.xvs = 0x80;
        o.xvp = 0;
        o.dir = 9;
        o.s[0] = 0x80;
        o.s[1] = 0x3C;
        self.f1 = 0x40;
        self.f0 = 2;
    }

    /// `$3DAB28`: the mini boss floats up and down (flipping every 128 frames), its parts follow its Y; every 60 frames it throws a homing shot
    /// `53` or (after several) a falling-block emitter `54`.
    fn ai_ab28(&mut self, x: usize) {
        if self.o[x].s[2] != 0 {
            if self.p.xpx < 0x80 {
                self.carry_player(x);
            }
        } else {
            self.move_v(x);
        }
        let y = self.o[x].ypx;
        for s in 20..=22 {
            self.o[s].ypx = y;
        }
        self.carry = y >= 0x2D;
        self.f1 = y.wrapping_sub(0x2D);
        self.o[x].s[0] = self.o[x].s[0].wrapping_sub(1);
        if self.o[x].s[0] == 0 {
            self.o[x].s[0] = 0x80;
            self.o[x].dir ^= 0x0C;
        }
        if self.o[x].s[1] != 0 {
            self.o[x].s[1] -= 1;
            if self.o[x].s[1] == 0 {
                let k = self.o[x].s[4] as usize;
                if k != 0 {
                    self.kill_slot(k);
                }
            }
            return;
        }
        self.o[x].s[4] = 0;
        let sum = self.rng[0] as u16 + self.rng[1] as u16 + self.carry as u16;
        self.rng[1] = sum as u8;
        self.carry = sum > 0xFF;
        if sum as u8 & 3 == 0 {
            if self.o[x].s[2] != 0 {
                self.ab93(x);
            } else {
                self.abdf(x);
            }
        } else if self.o[x].s[2] == 0 {
            self.o[x].s[3] = self.o[x].s[3].wrapping_add(1);
            if self.o[x].s[3] == 5 {
                self.abdf(x);
            } else {
                self.ab98(x);
            }
        } else {
            self.ab93(x);
        }
    }

    fn ab93(&mut self, x: usize) {
        self.o[x].s[3] = 0;
        self.ab98(x);
    }

    /// `$3DAB98`: throw a homing shot (type `53`).
    fn ab98(&mut self, x: usize) {
        self.o[x].s[2] = 0;
        let Some(y) = self.find_slot() else {
            self.carry = true;
            return;
        };
        self.o[x].s[1] = 0x3C;
        self.child_at(x, y, 0x3B, 0x8A);
        let c = &mut self.o[y];
        c.xvs = 0;
        c.yvs = 0;
        c.yvp = 0;
        c.xvp = 4;
        c.ty = 0x53;
        c.dir = 2;
        c.hitbox = 0x80;
        c.s[0] = 4;
        c.s[1] = 0x0C;
    }

    /// `$3DABDF`: release the falling-block emitter (type `54`) plus an inert marker object `80` whose slot is remembered in `$04C8`.
    fn abdf(&mut self, x: usize) {
        let Some(y) = self.find_slot() else {
            self.carry = true;
            return;
        };
        self.o[x].s[2] = 1;
        self.o[x].s[3] = 0;
        self.o[x].s[1] = 0xFF;
        self.child_at(x, y, 0x3C, 0x8D);
        let c = &mut self.o[y];
        c.yvs = 0;
        c.yvp = 4;
        c.ty = 0x54;
        c.hitbox = 0x8E;
        let Some(z) = self.find_slot() else {
            self.carry = true;
            return;
        };
        self.child_here(x, z, 0x82);
        self.o[z].ty = 0x80;
        self.o[z].hitbox = 0;
        self.o[x].s[4] = z as u8;
    }

    /// `$3DAC29` (type `53`): a shot that turns toward Mega Man by one heading step every 5 frames (only while the heading to him is >= 9).
    fn ai_ac29(&mut self, x: usize) {
        if self.o[x].s[0] == 0 {
            let h = self.heading_to_player(x);
            if h >= 9 {
                let cur = self.o[x].s[1];
                if h != cur {
                    if h < cur {
                        self.o[x].s[1] = cur.wrapping_sub(1);
                    } else {
                        self.o[x].s[1] = cur.wrapping_add(1);
                        if self.o[x].s[1] == 0 {
                            self.o[x].s[1] = 0xFF;
                        }
                    }
                }
            }
            let k = self.o[x].s[1];
            self.heading_speed(x, k, 8);
            let a = self.ai_byte(0xACFC + k as u16);
            self.set_anim(x, a);
            self.o[x].s[0] = 5;
        }
        self.move_h(x);
        self.move_v(x);
        self.o[x].s[0] = self.o[x].s[0].wrapping_sub(1);
    }

    /// `$3DAC6A` (type `54`): rises 4 px/frame; above the screen it becomes a row of four falling blocks (X = $14, $3C, $64, $8C).
    fn ai_ac6a(&mut self, x: usize) {
        self.up(x);
        if self.o[x].ypg == 0 {
            return;
        }
        self.set_anim(x, 0x8E);
        let o = &mut self.o[x];
        o.ypx = 0;
        o.ypg = 0;
        o.flags |= 4;
        o.xpx = 0x14;
        o.ptr = 0xACE7;
        o.yvs = 0;
        o.yvp = 1;
        o.s[0] = 0x0A;
        let mut px = 0x14u8;
        for _ in 0..3 {
            let Some(y) = self.find_slot() else { return };
            self.child_here(x, y, 0x8E);
            px = px.wrapping_add(0x28);
            let c = &mut self.o[y];
            c.xpx = px;
            c.ty = 0x54;
            c.hitbox = 0x8E;
            c.ptr = 0xACE7;
            c.yvs = 0;
            c.yvp = 1;
            c.s[0] = 0x0A;
        }
    }

    /// `$3DACE7`: wait 10 frames (inactive), then fall at the vertical speed.
    fn ai_ace7(&mut self, x: usize) {
        if self.o[x].s[0] != 0 {
            self.o[x].s[0] -= 1;
            if self.o[x].s[0] != 0 {
                return;
            }
            self.o[x].flags &= 0xFB;
        }
        self.down(x);
    }

    /// `$3DA5D3` (the replacement of a destroyed mini boss, type `90`): all enemy slots are cleared and a burst emitter `3C` starts in slot 23.
    fn ai_a5d3(&mut self) {
        for y in (6..=0x17).rev() {
            self.kill_slot(y);
        }
        let o = &mut self.o[0x17];
        o.aid = 0x62;
        o.aframe = 0;
        o.atick = 0;
        o.ty = 0x3C;
    }
}
