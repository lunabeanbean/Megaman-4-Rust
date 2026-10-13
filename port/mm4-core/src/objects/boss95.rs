//! The stage-9 boss (type `$95`, bank `$3D` `$BA91`-`$BD48`) and its pieces `$96` (`$BD65`/`$BD84`; the inert ones `$99`, `$97` shots fly straight).
//! The boss *is* the horizontal scroll of the room: its X speed moves `$69`/`$6A`, and ten piece objects (slots 13-22, addressed absolutely) are
//! placed relative to that scroll and push Mega Man sideways (code_3DBC41).
use super::*;

impl World {
    pub(super) fn run_boss95(&mut self, x: usize, ptr: u16) -> bool {
        if self.f6 != 0x3D {
            return false;
        }
        match ptr {
            0xBA91 => self.boss95_init(x),
            0xBABD => self.boss95_meter(x),
            0xBB08 => self.boss95_main(x),
            0xBBB3 => self.boss95_dive(x),
            0xBD65 => {
                let d = self.ai_byte(0xBD44 + x as u16);
                let o = &mut self.o[x];
                o.dir = d;
                o.yvs = 0x80;
                o.yvp = 0;
                o.ptr = 0xBD84;
                o.s[0] = 0x60;
                self.ai_bd84(x);
            }
            0xBD84 => self.ai_bd84(x),
            0xBA8B => {
                self.move_h(x);
                self.move_v(x);
            }
            0xBA8A => {}
            _ => return false,
        }
        true
    }

    fn ai_bd84(&mut self, x: usize) {
        self.move_v(x);
        self.o[x].s[0] = self.o[x].s[0].wrapping_sub(1);
        if self.o[x].s[0] == 0 {
            self.o[x].s[0] = 0x60;
            self.o[x].dir ^= 0x0C;
        }
    }

    fn boss95_init(&mut self, x: usize) {
        if self.fd9c(x) != 9 {
            return;
        }
        self.scroll69 = 0;
        self.scroll6a = 0;
        self.sect_req = Some(0x80); // `$2B` = section | $80
        self.p1c = 0x11;
        self.p1b = 0x80;
        self.o[x].ptr = 0xBABD;
    }

    fn boss95_meter(&mut self, x: usize) {
        if self.f0 == 0 {
            if self.p1b != 0 {
                return;
            }
            self.p95 = 0;
            self.bf = 0;
            self.f1 = 0x40;
            self.f0 = 7;
            self.o[x].dir = 1;
            self.meter2 = 0x8F;
            self.boss95_b94(x);
            self.boss95_set_speed(x);
            return;
        }
        if self.p95 & 7 != 0 {
            return;
        }
        self.snd(0x29, 0xBAE9); // code_3DBAE6
        self.bf = self.bf.wrapping_add(1);
        if self.bf != 0x1C {
            return;
        }
        self.o[x].ptr = 0xBB08;
        self.o[22].flags = 0x91;
        self.o[21].flags = 0x91;
        self.player_cmd = Some(0);
        self.p.state = 0;
        self.boss95_main(x);
    }

    /// `$BB94`: pick the next sideways speed index `$0468` from the random generator.
    fn boss95_b94(&mut self, x: usize) {
        let sum = self.rng[0] as u16 + self.rng[2] as u16 + self.carry as u16;
        self.rng[2] = sum as u8;
        self.carry = sum > 0xFF;
        let y = (sum as u8 & 7) as u16;
        self.o[x].s[0] = self.ai_byte(0xBD4F + y);
    }

    /// `$BB08`: slides the room's scroll left/right by its own speed; when the scroll reaches 0 it turns and may dive.
    fn boss95_main(&mut self, x: usize) {
        let (xvs, xvp) = (self.o[x].xvs, self.o[x].xvp);
        let s3;
        if self.o[x].dir & 1 != 0 {
            let (a, c1) = self.o[x].xsub.overflowing_add(xvs);
            self.o[x].xsub = a;
            let (b, c2a) = self.scroll69.overflowing_add(xvp);
            let (b, c2b) = b.overflowing_add(c1 as u8);
            self.scroll69 = b;
            let c2 = c2a || c2b;
            s3 = self.o[x].s[3].wrapping_add(c2 as u8);
        } else {
            let (a, b1) = self.o[x].xsub.overflowing_sub(xvs);
            self.o[x].xsub = a;
            let (b, b2a) = self.scroll69.overflowing_sub(xvp);
            let (b, b2b) = b.overflowing_sub(b1 as u8);
            self.scroll69 = b;
            let borrow = b2a || b2b;
            s3 = self.o[x].s[3].wrapping_sub(borrow as u8);
        }
        self.o[x].s[3] = s3;
        self.scroll6a = s3 & 1;
        if self.scroll6a == 0 {
            self.o[x].s[3] = 0;
        }
        self.bc41(x);
        if self.scroll69 | self.o[x].xsub != 0 {
            return;
        }
        if self.scroll6a != 0 {
            if self.o[x].s[0] != 0 {
                return;
            }
            self.o[x].flags &= 0xFB;
            self.o[x].ptr = 0xBBB3;
            self.o[x].s[1] = 0xFF;
            return;
        }
        self.o[x].s[0] = self.o[x].s[0].wrapping_sub(1);
        if self.o[x].s[0] & 0x80 == 0 {
            self.boss95_set_speed(x);
            return;
        }
        let sum = self.rng[2] as u16 + self.rng[1] as u16 + self.carry as u16;
        self.rng[0] = sum as u8;
        self.carry = sum > 0xFF;
        let k = (sum as u8 & 1) as u16;
        self.o[x].dir = self.ai_byte(0xBD57 + k);
        self.boss95_b94(x);
        self.boss95_set_speed(x);
    }

    fn boss95_set_speed(&mut self, x: usize) {
        let y = self.o[x].s[0] as u16;
        self.o[x].xvs = self.ai_byte(0xBD49 + y);
        self.o[x].xvp = self.ai_byte(0xBD4C + y);
    }

    /// `$BBB3`: the boss is visible at scroll 0: it shoots aimed shots `$97` every 90 frames for `$0480` frames, then hides again.
    fn boss95_dive(&mut self, x: usize) {
        if self.o[x].s[2] == 0 {
            self.o[x].s[2] = 0x5A;
            let mut skip = false;
            if let Some(y) = self.find_slot() {
                self.child_here(x, y, 0x91);
                self.o[y].ty = 0x97;
                self.o[y].hitbox = 0x80;
                let bits = self.aim_fb8c(y, 0x00, 0x02);
                self.o[y].dir = bits;
            } else {
                self.carry = true;
                skip = true;
            }
            if !skip {
                self.o[x].s[2] = self.o[x].s[2].wrapping_sub(1);
            }
        } else {
            self.o[x].s[2] = self.o[x].s[2].wrapping_sub(1);
        }
        self.o[x].s[1] = self.o[x].s[1].wrapping_sub(1);
        if self.o[x].s[1] != 0 {
            return;
        }
        let o = &mut self.o[x];
        o.t5b8 = 0;
        o.flags |= 4;
        o.dir ^= 3;
        o.ptr = 0xBB08;
        o.s[2] = 0;
    }

    /// `$BC41`: place the ten pieces relative to the scroll and push Mega Man out of them.
    fn bc41(&mut self, x: usize) {
        let s3 = self.o[x].s[3];
        let mut last_hurt: Option<(u8, usize)> = None;
        for y in (0..=9usize).rev() {
            let sl = 13 + y;
            let base = self.ai_byte(0xBD5B + y as u16);
            let mut page;
            if y < 6 {
                let (a, b) = base.overflowing_sub(self.scroll69);
                self.o[sl].xpx = a;
                let (r, b1) = 0x11u8.overflowing_sub(s3);
                let (r, b2) = r.overflowing_sub(b as u8);
                page = r;
                self.carry = !(b1 || b2);
            } else {
                let (a, c) = base.overflowing_add(self.scroll69);
                self.o[sl].xpx = a;
                let (r, c1) = 0x11u8.overflowing_add(s3);
                let (r, c2) = r.overflowing_add(c as u8);
                page = r;
                self.carry = c1 || c2;
            }
            if page & 1 == 0 {
                page = 0x10;
            }
            self.o[sl].xpg = page;
            let hb = (self.o[sl].hitbox & 0x7F) as u16;
            let mut h = self.half_height(hb);
            self.carry = self.p.anim >= 0x10;
            if self.p.anim == 0x10 {
                let (v, b) = h.overflowing_sub(8);
                h = v;
                self.carry = !b;
            }
            // vertical overlap
            let (d, b) = self.p.ypx.overflowing_sub(self.o[sl].ypx);
            let dy = if b { (!d).wrapping_add(1) } else { d };
            self.carry = dy >= h;
            if dy >= h {
                continue;
            }
            // horizontal overlap
            let mut left = false;
            let (d0, b0) = self.p.xpx.overflowing_sub(self.o[sl].xpx);
            let (d1, b1) = self.p.xpg.overflowing_sub(self.o[sl].xpg);
            let (d1, b2) = d1.overflowing_sub(b0 as u8);
            let (mut v0, mut v1) = (d0, d1);
            if b1 || b2 {
                left = true;
                v0 = (!v0).wrapping_add(1);
                let c = v0 == 0;
                v1 = (!v1).wrapping_add(c as u8);
            }
            if v1 != 0 {
                continue;
            }
            let w = self.half_width(hb);
            let (t, bw) = w.wrapping_add(1).overflowing_sub(v0);
            if bw {
                self.carry = false;
                continue;
            }
            let mut push = t;
            self.carry = t >= 8;
            if t >= 8 {
                push = 8;
            }
            if left {
                let c = self.carry as u8;
                push = (!push).wrapping_add(1).wrapping_add(c);
            }
            if push == 0 {
                continue;
            }
            if self.p.state == 5 {
                self.p.state = 0;
                self.player_cmd = Some(0);
            }
            let mut px = self.p.xpx;
            if px >= 0xF0 {
                px = 0xF0;
            } else if px < 0x10 {
                px = 0x10;
            }
            let (nx, c) = px.overflowing_add(push);
            self.carry = c;
            let mut hurt = false;
            let fin;
            if nx >= 0xF1 {
                fin = 0xF0;
                hurt = true;
            } else if nx >= 0x10 {
                fin = nx;
                self.carry = true;
            } else {
                fin = 0x10;
                hurt = true;
            }
            self.p.xpx = fin;
            self.player_ops.push(PlayerOp::SetX(fin));
            if hurt && !self.invincible && self.p.state < 6 {
                let slot = hb as usize;
                let ty = self.obj_ram(0x0300 + slot as u16);
                let dmg = self.ai_byte(0x8413 + ty as u16);
                if dmg != 0 && self.p.anim != 0x27 {
                    last_hurt = Some((dmg, slot.min(SLOTS - 1)));
                    self.hurt_req = Some((dmg, slot.min(SLOTS - 1)));
                }
            }
        }
        let _ = last_hurt;
    }

    /// RAM read of the object arrays by absolute address (`$0300 + n`), as the 6502 code sees them.
    pub(super) fn obj_ram(&self, addr: u16) -> u8 {
        let off = addr.wrapping_sub(0x0300) as usize;
        let (arr, slot) = (off / 0x18, off % 0x18);
        if arr >= 30 {
            return 0;
        }
        self.o[slot].fields_ram()[arr]
    }
}
