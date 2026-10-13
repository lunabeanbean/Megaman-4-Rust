//! The stage-8 boss (type `$87`, bank `$3D` `$B148`-`$B5BF`; its parts `$88`/`$89` are inert, the shots `$8A` fly straight). It floats left and right,
//! drops brick pieces and builds/clears floor blocks through the bitmap `$0696-$0699` (terrain type `$10` of stage 8 is masked by the bitmap `$0680`).
//! Slots 21-23 are the boss and its two parts (the code addresses them absolutely): the parts follow the boss's X, `$038E` (slot 22's Y) is a ram variable.
use super::*;

impl World {
    pub(super) fn run_boss87(&mut self, st: &Stage, x: usize, ptr: u16) -> bool {
        if self.f6 != 0x3D {
            return false;
        }
        match ptr {
            0xB148 => self.boss87_init(x),
            0xB163 => self.boss87_fade(x),
            0xB1F7 => self.boss87_meter(st, x),
            0xB218 => self.boss87_main(st, x),
            0xB326 => {
                self.o[x].ypx = self.o[x].ypx.wrapping_add(2);
                self.o[x].s[3] = self.o[x].s[3].wrapping_sub(1);
                if self.o[x].s[3] == 0 {
                    self.o[x].s[3] = 0x1E;
                    self.o[x].ptr = 0xB342;
                }
                self.boss87_follow();
            }
            0xB342 => {
                self.o[x].s[3] = self.o[x].s[3].wrapping_sub(1);
                if self.o[x].s[3] == 0 {
                    self.o[x].s[3] = 0x10;
                    self.o[x].ptr = 0xB358;
                }
                self.boss87_follow();
            }
            0xB358 => {
                self.o[x].ypx = self.o[x].ypx.wrapping_sub(2);
                self.o[x].s[3] = self.o[x].s[3].wrapping_sub(1);
                if self.o[x].s[3] == 0 {
                    self.o[x].s[3] = 0x78;
                    self.o[x].ptr = 0xB218;
                }
                self.boss87_follow();
            }
            0xB37B => self.boss87_b37b(st, x),
            0xB392 => self.boss87_b392(st, x),
            0xB3C0 => self.boss87_b3c0(st, x),
            0xB49C => self.boss87_b49c(x),
            0xB65C => {}
            0xB5C8 => {
                self.move_h(x);
                self.move_v(x);
            }
            _ => return false,
        }
        true
    }

    /// `$3DFD9C`: wait for Mega Man to stand (state 0 -> 9, the boss owns the meter); returns the state.
    pub(super) fn fd9c(&mut self, x: usize) -> u8 {
        self.boss_music();
        if self.p.state == 0 {
            self.p.state = 9;
            self.player_cmd = Some(9);
            self.boss_slot = x as u8;
        }
        self.p.state
    }

    fn boss87_init(&mut self, x: usize) {
        if self.fd9c(x) == 9 {
            self.p1c = 0x14;
            self.p1b = 0x80;
            self.o[x].ptr = 0xB163;
        }
    }

    /// `$B163`: once the renderer is idle the screen fades in over 4 steps of 16 frames, then the fight starts.
    fn boss87_fade(&mut self, x: usize) {
        if self.f0 == 0 {
            if self.p1b != 0 {
                return;
            }
            self.p95 = 0;
            self.f1 = 0xA0;
            self.f0 = 3;
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
        o.ptr = 0xB1F7;
        o.xvs = 0x80;
        o.xvp = 0;
        o.yvs = 0x40;
        o.yvp = 0;
        o.s[0] = 0x20;
        o.dir = 9;
        o.s[3] = 0x78;
        o.s[4] = 0x5A;
        o.s[5] = 0xFF;
        o.flags = 0x90;
        self.o[21].flags = 0x90;
        self.o[22].flags = 0xB0;
        self.meter2 = 0x8F;
        self.p95 = 0;
        self.bf = 0;
    }

    /// `$B1F7`: the boss meter fills (one unit every 8 frames), then the fight state.
    fn boss87_meter(&mut self, st: &Stage, x: usize) {
        if self.p95 & 7 != 0 {
            return;
        }
        self.snd(0x29, 0xB202); // code_3DB1FF
        self.bf = self.bf.wrapping_add(1);
        if self.bf != 0x1C {
            return;
        }
        self.o[x].ptr = 0xB218;
        self.player_cmd = Some(0);
        self.p.state = 0;
        self.carry = true;
        self.boss87_main(st, x);
    }

    /// `$B2EE`: the parts follow the boss; `$FA`/`$FD` (background scroll) come from its Y.
    fn boss87_follow(&mut self) {
        let bx = self.o[23].xpx;
        self.o[22].xpx = bx;
        self.o[21].xpx = bx;
        self.fd = 0;
        let by = self.o[23].ypx;
        self.o[22].ypx = by.wrapping_sub(0x1C);
        self.o[21].ypx = by.wrapping_sub(0x1C);
        let (a, borrow) = 0x6Cu8.overflowing_sub(by);
        if !borrow {
            self.fa = a;
        } else {
            self.fa = a.wrapping_sub(0x0F).wrapping_sub(1);
            self.fd = 2;
        }
    }

    /// `$B218`: float sideways (turning at the walls), every now and then dive, shoot, or let the blocks change.
    fn boss87_main(&mut self, st: &Stage, x: usize) {
        let mut go_follow = false;
        let mut skip_to_296 = false;
        if self.o[x].s[1] != 0 {
            self.o[x].s[1] -= 1;
            skip_to_296 = true;
        } else {
            self.move_h(x);
            self.move_v(x);
            self.o[x].s[0] = self.o[x].s[0].wrapping_sub(1);
            if self.o[x].s[0] == 0 {
                self.o[x].s[2] += 1;
                if self.o[x].s[2] == 0x0A {
                    self.o[x].s[2] = 0;
                }
                let k = self.o[x].s[2] as u16;
                self.o[x].s[0] = self.ai_byte(0xB5DE + k);
                self.o[x].dir ^= 0x0C;
                let wall = if self.o[x].dir & 1 != 0 { 0xC0 } else { 0x40 };
                if self.o[x].xpx == wall {
                    self.o[x].dir ^= 3;
                    self.o[x].s[1] = 0x15;
                    self.o[x].s[1] -= 1;
                }
                skip_to_296 = true;
            } else {
                // B26F
                if self.o[x].s[5] != 0 {
                    self.o[x].s[5] -= 1;
                    if self.o[x].s[5] != 0 {
                        skip_to_296 = true;
                    } else {
                        self.boss87_b4c4(st, x);
                    }
                }
                if !skip_to_296 {
                    if self.o[x].xpx == self.o[x].s[7] {
                        self.o[x].ptr = 0xB37B;
                        self.o[x].s[5] = 0x1E;
                    }
                    go_follow = true;
                }
            }
        }
        if go_follow {
            self.boss87_follow();
            return;
        }
        if skip_to_296 {
            // B296
            self.o[x].s[3] = self.o[x].s[3].wrapping_sub(1);
            if self.o[x].s[3] == 0 {
                self.o[x].s[3] = 0x78;
                let sum = self.rng[0] as u16 + self.rng[1] as u16 + self.carry as u16;
                self.carry = sum > 0xFF;
                if sum as u8 & 3 == 0 {
                    self.o[x].ptr = 0xB326;
                    self.o[x].s[3] = 0x10;
                    self.boss87_follow();
                    return;
                }
            }
            self.o[x].s[4] = self.o[x].s[4].wrapping_sub(1);
            if self.o[x].s[4] == 0 {
                self.o[x].s[4] = 0x5A;
                if let Some(y) = self.find_slot() {
                    self.carry = false;
                    self.child_here(x, y, 0x32);
                    self.o[y].ty = 0x8A;
                    self.o[y].hitbox = 0x80;
                    let bits = self.aim_fb8c(y, 0x00, 0x02);
                    self.o[y].dir = bits;
                } else {
                    self.carry = true;
                }
            }
        }
        self.boss87_follow();
    }

    fn boss87_b37b(&mut self, st: &Stage, x: usize) {
        self.o[x].s[5] = self.o[x].s[5].wrapping_sub(1);
        if self.o[x].s[5] != 0 {
            return;
        }
        self.o[x].ptr = 0xB392;
        self.o[22].xpx = self.o[22].xpx.wrapping_sub(1);
        self.o[x].s[5] = 0x1F;
        self.boss87_b392(st, x);
    }

    fn boss87_b392(&mut self, st: &Stage, x: usize) {
        if self.o[x].s[5] & 1 != 0 {
            self.o[22].xpx = self.o[22].xpx.wrapping_add(2);
        } else {
            self.o[22].xpx = self.o[22].xpx.wrapping_sub(2);
        }
        self.o[x].s[5] = self.o[x].s[5].wrapping_sub(1);
        if self.o[x].s[5] != 0 {
            return;
        }
        self.o[22].xpx = self.o[22].xpx.wrapping_sub(1);
        self.o[x].ptr = 0xB3C0;
        self.o[x].s[6] = self.o[22].ypx;
        self.boss87_b3c0(st, x);
    }

    fn boss87_b3c0(&mut self, st: &Stage, x: usize) {
        let y = self.o[22].ypx.wrapping_add(4);
        self.o[22].ypx = y;
        if y < 0x80 {
            return;
        }
        for k in (0..=4u16).rev() {
            let Some(y) = self.find_slot() else { break };
            let ty = self.ai_byte(0xB657 + k);
            let (xvs, xvp, yvs, yvp, dir, anim) =
                (self.ai_byte(0xB643 + k), self.ai_byte(0xB648 + k), self.ai_byte(0xB639 + k), self.ai_byte(0xB63E + k), self.ai_byte(0xB64D + k), self.ai_byte(0xB652 + k));
            let c = &mut self.o[y];
            c.ty = ty;
            c.xvs = xvs;
            c.xvp = xvp;
            c.yvs = yvs;
            c.yvp = yvp;
            c.dir = dir;
            self.child_here(x, y, anim);
            self.o[y].ypx = 0xB0;
        }
        let o = &mut self.o[x];
        o.ptr = 0xB49C;
        o.s[5] = 0x14;
        // the PPU buffer of the new bricks (code_3DB40F): four rows of four tiles, 4 rows down from `$B5E8[$03D6]` (the high byte is 22 for the first two rows, 23 for the others), then the block bitmap bits
        {
            let l = self.ai_byte(0xB5E8 + self.o[22].xvp as u16);
            let yb = self.o[22].xvs as u16;
            let mut lo = l;
            for (r, tbl) in [0xB5EBu16, 0xB5EF, 0xB5F3, 0xB5F7].iter().enumerate() {
                let hi: u16 = if r < 2 { 0x22 } else { 0x23 };
                let bytes: Vec<u8> = (0..4u16).map(|k| self.ai_byte(tbl + yb + k)).collect();
                self.ppu_put(hi << 8 | lo as u16, &bytes);
                lo = lo.wrapping_add(0x20);
            }
        }
        let y = self.o[22].s[0] as u16;
        for k in 0..4u16 {
            let v = self.ai_byte(0xB621 + y + k);
            let i = 0x16 + k as usize;
            st.blocks[i].set(st.blocks[i].get() | v);
        }
        self.col_block = true; // `$19` = $FF: the renderer waits this frame
    }

    fn boss87_b49c(&mut self, x: usize) {
        if self.o[x].s[5] != 0 {
            self.o[x].s[5] -= 1;
            if self.o[x].s[5] != 0 {
                return;
            }
        }
        let y = self.o[22].ypx.wrapping_sub(4);
        self.o[22].ypx = y;
        if y == self.o[x].s[6] {
            self.o[x].ptr = 0xB218;
            self.o[x].s[5] = 0xFF;
        }
    }

    /// `$B4C4`: choose where the next blocks go, from the quadrant Mega Man stands in.
    fn boss87_b4c4(&mut self, st: &Stage, x: usize) {
        self.o[22].xvs = 0;
        self.carry = (self.p.xpx >> 5) & 1 != 0; // the last LSR of the quadrant lookup
        let q = self.p.xpx >> 6;
        let bm = |_w: &World, i: usize| st.blocks[0x16 + i].get();
        let mut y: u8;
        match q {
            0 => {
                y = 0;
                if bm(self, 0) & 0x18 != 0 {
                    y = 4;
                    if bm(self, 2) & 0x18 != 0 {
                        self.o[x].s[5] = 0xFF;
                        return;
                    }
                    self.o[22].xvs = 0x10;
                }
                self.o[22].xvp = 0;
                self.o[x].s[7] = 0x41;
            }
            1 | 2 => {
                let base: u8 = if q == 1 { 0 } else { 2 };
                let sum = self.rng[0] as u16 + self.rng[2] as u16 + self.carry as u16;
                self.rng[2] = sum as u8;
                self.carry = sum > 0xFF;
                let i = (sum as u8 & 1) | base;
                let a = self.ai_byte(0xB613 + i as u16);
                y = a;
                let y0 = y as u16;
                let t = |w: &World, off: u16| w.ai_byte(0xB621 + y0 + off);
                let first = (bm(self, 0) & t(self, 0)) | (bm(self, 1) & t(self, 1));
                if first != 0 {
                    self.o[22].xvs = 0x10;
                    y = y.wrapping_add(4);
                    let second = (bm(self, 2) & t(self, 0)) | (bm(self, 3) & t(self, 1));
                    if second != 0 {
                        self.o[x].s[5] = 0xFF;
                        return;
                    }
                }
                let a2 = self.ai_byte(0xB60B + i as u16);
                self.o[x].s[7] = a2;
                self.o[22].xvp = self.ai_byte(0xB60F + i as u16);
            }
            _ => {
                y = 0x10;
                if bm(self, 1) & 0x18 != 0 {
                    y = 0x14;
                    if bm(self, 3) & 0x18 != 0 {
                        self.o[x].s[5] = 0xFF;
                        return;
                    }
                    self.o[22].xvs = 0x10;
                }
                self.o[22].xvp = 2;
                self.o[x].s[7] = 0xBF;
            }
        }
        // $B588: turn toward the target X if needed
        self.o[22].s[0] = y;
        let target = self.o[x].s[7];
        let mut m = 1u8;
        if self.o[x].xpx == target {
            return;
        }
        if self.o[x].xpx > target {
            m = 2;
        }
        if self.o[x].dir & m != 0 {
            return;
        }
        self.o[x].dir ^= 0x0F;
        let k = self.o[x].s[2] as u16;
        let v = self.ai_byte(0xB5DE + k).wrapping_sub(self.o[x].s[0]);
        self.o[x].s[0] = v;
        self.o[x].s[2] = self.ai_byte(0xB617 + k);
    }
}
