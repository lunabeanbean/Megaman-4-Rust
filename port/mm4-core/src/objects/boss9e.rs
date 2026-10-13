//! The stage-10 boss (type `$9E`, bank `$38` `$B82E`-`$B9AF`; its shots `$A0` = `$BC62`). Mega Man walks in by himself (state `$10`), the boss
//! builds its room (PPU writes: only the counters `$04B0`/`$04C8`/`$04E0` and the flag `$19` matter), slides in and then walks along the floor throwing things.
use super::*;

impl World {
    pub(super) fn run_boss9e(&mut self, st: &Stage, x: usize, ptr: u16) -> bool {
        if self.f6 != 0x38 {
            return false;
        }
        match ptr {
            0xB82E => {
                self.player_cmd = Some(0x10);
                self.p.state = 0x10;
                self.o[x].ptr = 0xB842;
            }
            0xB842 => self.boss9e_b842(x),
            0xB858 => self.boss9e_b858(x),
            0xB88F => self.boss9e_b88f(x),
            0xB8B2 => self.boss9e_b8b2(x),
            0xB8D7 => self.boss9e_b8d7(x),
            0xB8F1 => self.boss9e_b8f1(st, x),
            0xB93D => self.boss9e_b93d(x),
            0xBC62 => {
                self.move_h(x);
                self.move_v(x);
                if self.o[x].ypg != 0 {
                    self.despawn(x);
                }
            }
            _ => return false,
        }
        true
    }

    /// `$BC71` / `$BCDA`: one step of the room's tile building every fourth frame (`$04B0` counts 0..4, `$04C8:$04E0` is the PPU address).
    fn boss9e_build(&mut self, x: usize, up: bool) {
        if self.o[x].s[3] == 4 {
            self.carry = true;
            return;
        }
        if self.p95 & 3 != 0 {
            self.carry = self.o[x].s[3] >= 4;
            return;
        }
        self.col_block = true;
        {
            // the two PPU records (code_38BC71 / code_38BCDA): a 2x2 piece of the room at (`$04E0`:`$04C8`), picked by the step `$04B0` from `$BD4E` (up) / `$BD52` (down)
            let step = self.o[x].s[3] as u16;
            let idx = self.ai_byte(if up { 0xBD4E } else { 0xBD52 } + step) as u16;
            let addr = (self.o[x].s[5] as u16) << 8 | self.o[x].s[4] as u16;
            let t = [self.ai_byte(0xBD56 + idx), self.ai_byte(0xBD57 + idx), self.ai_byte(0xBD58 + idx), self.ai_byte(0xBD59 + idx)];
            self.ppu_put(addr, &t[0..2]);
            self.ppu_put((addr & 0xFF00) | ((addr as u8) | 0x20) as u16, &t[2..4]);
        }
        let o = &mut self.o[x];
        o.s[3] += 1;
        let lo = o.s[4];
        if up {
            let (v, c) = lo.overflowing_sub(0x40);
            o.s[4] = v;
            o.s[5] = o.s[5].wrapping_sub(c as u8);
        } else {
            let (v, c) = lo.overflowing_add(0x40);
            o.s[4] = v;
            o.s[5] = o.s[5].wrapping_add(c as u8);
        }
        self.snd(0x2C, if up { 0xBCD9 } else { 0xBD42 }); // code_38BCD6 / code_38BD3F
        self.carry = true; // the sound call leaves the carry set
    }

    fn boss9e_b842(&mut self, x: usize) {
        if self.p.xpx != 0x40 {
            return;
        }
        self.meter2 = 0x8F;
        self.o[x].ptr = 0xB858;
        self.boss9e_b858(x);
    }

    fn boss9e_b858(&mut self, x: usize) {
        if self.fd9c(x) != 9 {
            return;
        }
        if self.p95 & 7 != 0 {
            return;
        }
        self.snd(0x29, 0xB86A); // code_38B867
        self.carry = true;
        self.bf = self.bf.wrapping_add(1);
        if self.bf != 0x1C {
            return;
        }
        let o = &mut self.o[x];
        o.ptr = 0xB88F;
        o.s[3] = 0;
        o.s[4] = 0x5E;
        o.s[5] = 0x21;
        self.p95 = 0;
        self.player_cmd = Some(0);
        self.p.state = 0;
        self.boss9e_b88f(x);
    }

    fn boss9e_b88f(&mut self, x: usize) {
        self.boss9e_build(x, true);
        if self.o[x].s[3] != 4 {
            return;
        }
        let o = &mut self.o[x];
        o.ptr = 0xB8B2;
        o.xpx = 0x18;
        o.xpg = 0x11;
        o.flags = 0x98;
        self.boss9e_b8b2(x);
    }

    fn boss9e_b8b2(&mut self, x: usize) {
        self.move_h(x);
        if self.o[x].xpx != 0xD8 {
            return;
        }
        let o = &mut self.o[x];
        o.s[3] = 0;
        o.s[4] = 0x9E;
        o.s[5] = 0x20;
        o.ptr = 0xB8D7;
        self.p95 = 0;
        self.boss9e_b8d7(x);
    }

    fn boss9e_b8d7(&mut self, x: usize) {
        self.o[x].atick = 0;
        self.boss9e_build(x, false);
        if self.o[x].s[3] == 4 {
            self.o[x].ptr = 0xB8F1;
        }
    }

    /// `$B8F1`: walks along the floor; after its random pause it plays a throwing animation.
    fn boss9e_b8f1(&mut self, st: &Stage, x: usize) {
        if self.o[x].s[0] != 0 {
            self.o[x].atick = 0;
            self.o[x].s[0] -= 1;
            if self.o[x].s[0] != 0 {
                return;
            }
            self.boss9e_b9af(x);
            return;
        }
        if self.horizontal_collide(st, x, 0x38) {
            self.o[x].dir ^= 3;
        }
        self.o[x].flags &= 0xBF;
        self.o[x].s[1] = self.o[x].s[1].wrapping_sub(1);
        if self.o[x].s[1] != 0 {
            return;
        }
        self.o[x].ptr = 0xB93D;
        let sum = self.rng[2] as u16 + self.rng[1] as u16 + self.carry as u16;
        let y = (sum as u8 & 1) as u16;
        self.carry = false; // CLC before the ADC #$5C
        self.set_anim(x, 0x5C + y as u8);
        self.o[x].s[2] = self.ai_byte(0xB9D5 + y);
    }

    fn boss9e_b93d(&mut self, x: usize) {
        if self.o[x].aframe == 0 {
            return;
        }
        let t = self.o[x].atick;
        if t == 0 {
            let first = self.o[x].s[2];
            let mut e = first;
            loop {
                let Some(y) = self.find_slot() else { return };
                let c = &mut self.o[y];
                c.ty = 0xA0;
                c.hitbox = 0x8F;
                c.hp = 0;
                self.child_at(x, y, 0x54 + e, 8);
                let k = e as u16;
                let (a, b, cc, d, f) = (self.ai_byte(0xB9E6 + k), self.ai_byte(0xB9E9 + k), self.ai_byte(0xB9E0 + k), self.ai_byte(0xB9E3 + k), self.ai_byte(0xB9EC + k));
                let c = &mut self.o[y];
                c.yvs = a;
                c.yvp = b;
                c.xvs = cc;
                c.xvp = d;
                c.dir = f;
                if e == 0 {
                    break;
                }
                e -= 1;
                if e == 0 {
                    break;
                }
            }
        } else if t == 8 {
            self.o[x].s[0] = 0x1E;
            self.o[x].ptr = 0xB8F1;
            self.set_anim(x, 0x54);
        }
    }

    /// `$B9AF`: after the pause, the next walking time (`$0480`), direction and speed.
    fn boss9e_b9af(&mut self, x: usize) {
        let sum = self.rng[1] as u16 + self.rng[3] as u16 + self.carry as u16;
        self.rng[1] = sum as u8;
        let c = sum > 0xFF;
        let y = (sum as u8 & 3) as u16;
        let v = self.ai_byte(0xB9D7 + y);
        self.o[x].s[1] = v;
        let sum2 = v as u16 + self.rng[2] as u16 + c as u16;
        self.carry = sum2 > 0xFF;
        let k = (sum2 as u8 & 1) as u16;
        self.o[x].dir = self.ai_byte(0xB9DB + k);
        self.o[x].xvp = self.ai_byte(0xB9DD + k);
        self.o[x].xvs = 0;
    }
}
