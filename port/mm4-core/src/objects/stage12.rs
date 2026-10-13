//! Stage 12: the falling-block spawner `35` (AI `$C3`, bank `$35` `$B3FD`/`$B423`).
use super::*;

impl World {
    pub(super) fn run_stage12(&mut self, x: usize, ptr: u16) -> bool {
        if self.f6 != 0x35 {
            return false;
        }
        match ptr {
            0xB3FD => self.ai_b3fd(x),
            0xB423 => self.ai_b423(x),
            _ => return false,
        }
        true
    }

    /// `$35B3FD`: waits 180 frames, then starts releasing its blocks.
    fn ai_b3fd(&mut self, x: usize) {
        self.o[x].s[0] = self.o[x].s[0].wrapping_add(1);
        if self.o[x].s[0] != 0xB4 {
            return;
        }
        self.o[x].ptr = 0xB423;
        self.o[x].s[0] = 0;
        let y = self.o[x].xpg.wrapping_sub(0x0D) as u16;
        self.o[x].s[1] = self.ai_byte(0xB4C3 + y);
        self.ai_b423(x);
    }

    /// `$35B423`: every 62 frames drops the blocks of the next table row (`$B4C5`: Y, X, continue flag).
    fn ai_b423(&mut self, x: usize) {
        if self.o[x].s[0] != 0 {
            self.o[x].s[0] -= 1;
            return;
        }
        loop {
            let Some(y) = self.find_slot() else {
                self.carry = true;
                return;
            };
            self.snd(0x39, 0xB432); // code_35B42F
            self.carry = true; // the sound call
            self.child_here(x, y, 0x11);
            let c = &mut self.o[y];
            c.ty = 0x60;
            c.hitbox = 0;
            c.flags = 0x92;
            let k = self.o[x].s[1] as u16;
            let (py, px, v) = (self.ai_byte(0xB4C5 + k), self.ai_byte(0xB4C6 + k), self.ai_byte(0xB4C7 + k));
            let c = &mut self.o[y];
            c.ypx = py;
            c.xpx = px;
            self.o[x].s[1] = self.o[x].s[1].wrapping_add(3);
            if v == 0 {
                break;
            }
            if v & 0x80 == 0 {
                continue;
            }
            let k2 = self.o[x].s[1] as u16;
            self.o[x].s[1] = self.ai_byte(0xB4C5 + k2);
            break;
        }
        self.o[x].s[0] = 0x3D;
        self.o[x].s[0] -= 1;
    }
}
