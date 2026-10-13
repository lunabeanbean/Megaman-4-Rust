//! Stage 4 enemies: bank `$3A` `$97E3`-`$9C8E` (turret `1A`, rock monster `30`/`31`/`32`), bank `$3B` `$ABD5`/`$B780` (drifter `49`, walker `78`).
use super::*;

impl World {
    /// Dispatch for the stage-4 states; returns false for an address that is not one of them.
    pub(super) fn run_stage4(&mut self, st: &Stage, x: usize, ptr: u16) -> bool {
        match ptr {
            0xA000 => self.ai_a000(st, x),
            0xA0CB => self.ai_a0cb(st, x),
            0xA1A4 => self.ai_a1a4(st, x),
            0x97E3 => self.ai_97e3(x),
            0x9823 => self.ai_9823(x),
            0x9925 => {
                self.move_h(x);
                self.move_v(x);
            }
            0x9B07 => self.ai_9b07(x),
            0x9B51 | 0x9C0E => {}
            0x9B56 => self.ai_9b56(x),
            0x9C33 => self.ai_9c33(x),
            0xABD5 => self.ai_abd5(x),
            0xB780 => self.ai_b780(st, x),
            _ => return false,
        }
        true
    }

    /// `$3A9899`: every 8 frames the turret turns its aim one step toward Mega Man (heading -> pose animation and shot direction index `$0498`).
    fn a9899(&mut self, x: usize) {
        if self.o[x].s[4] != 0 {
            self.o[x].s[4] -= 1;
            if self.o[x].s[4] != 0 {
                return;
            }
        }
        let h = self.aim(x);
        if h != 0 {
            self.carry = h >= 8; // CMP #$08
        }
        let facing_right = self.o[x].flags & 0x40 != 0;
        let y = match h {
            0 => {
                if facing_right {
                    1
                } else {
                    0x0F
                }
            }
            8 => {
                if facing_right {
                    7
                } else {
                    9
                }
            }
            h => h,
        };
        self.o[x].s[2] = y;
        self.o[x].aid = self.ai_byte(0x98DD + y as u16);
        self.o[x].s[4] = 8;
    }

    /// AI `$1A` waiting state (`$3A97E3`, list types `06`/`07`): a wall turret that aims; after a pause it fires 1-3 aimed shots.
    fn ai_97e3(&mut self, x: usize) {
        self.a9899(x);
        if self.o[x].s[1] != 0 {
            self.o[x].s[1] -= 1;
            return;
        }
        let sum = self.rng[0] as u16 + self.rng[2] as u16 + self.carry as u16;
        self.rng[2] = sum as u8;
        let mut carry = sum > 0xFF;
        let mut a = self.rng[2] & 3;
        if a != 0 {
            self.o[x].s[0] = self.o[x].s[0].wrapping_add(1);
            a = self.o[x].s[0];
            if a != 3 {
                self.o[x].s[1] = 0x3C;
                return;
            }
            carry = true; // CMP #3 (equal)
        }
        let sum = a as u16 + self.rng[1] as u16 + carry as u16;
        self.carry = sum > 0xFF;
        let y = (sum as u8) & 7;
        self.o[x].s[0] = self.ai_byte(0x98D5 + y as u16);
        self.o[x].s[1] = 0x3C;
        self.o[x].ptr = 0x9823;
        self.ai_9823(x);
    }

    /// AI `$1A` shooting state (`$3A9823`): when the pause ends it plays the shooting pose and fires a shot (`$1B`) in the aimed direction.
    fn ai_9823(&mut self, x: usize) {
        self.a9899(x);
        if self.o[x].s[1] == 0 {
            return;
        }
        self.o[x].s[1] -= 1;
        if self.o[x].s[1] != 0 {
            return;
        }
        self.o[x].aid = self.o[x].aid.wrapping_add(1);
        let Some(y) = self.find_slot() else { return };
        let k = self.o[x].s[2] as u16;
        self.o[y].ty = 0x1B;
        self.o[y].dir = self.ai_byte(0x97C3 + k);
        self.o[y].xvs = self.ai_byte(0x98ED + k);
        self.o[y].xvp = self.ai_byte(0x9901 + k);
        self.o[y].yvs = self.ai_byte(0x98F1 + k);
        self.o[y].yvp = self.ai_byte(0x9905 + k);
        let idx = self.ai_byte(0x9915 + k);
        self.child_at(x, y, idx, 0x18);
        self.o[y].hitbox = 0x8F;
        self.o[x].s[4] = 0x10;
        self.o[x].s[1] = 0x1E;
        self.o[x].s[0] = self.o[x].s[0].wrapping_sub(1);
        if self.o[x].s[0] != 0 {
            return;
        }
        self.o[x].s[1] = 0;
        self.o[x].s[2] = 0;
        self.o[x].ptr = 0x97E3;
    }

    /// AI `$30` (`$3A9B07`, list type `12`): the rock monster's nest: after 121 frames it plays the hatching animation and creates the monster (`$31`).
    fn ai_9b07(&mut self, x: usize) {
        if self.o[x].s[0] == 0 {
            self.o[x].s[0] = 0x79;
        }
        self.o[x].s[0] -= 1;
        if self.o[x].s[0] != 0 {
            return;
        }
        self.set_anim(x, 0x53);
        self.face_player(x);
        self.face_flag(x);
        self.o[x].hitbox = 0xC2;
        let Some(y) = self.find_slot() else { return };
        self.o[y].ty = 0x31;
        self.child_here(x, y, 0x54);
        self.o[y].flags &= 0xBF;
        self.o[y].hitbox = 0x82;
        self.o[y].hp = 0x12;
        self.o[x].ptr = 0x9B51;
    }

    /// `$3A9C8E`: the slot (23..8) holding an object of type `t`; when there is none the routine falls out of its loop with Y = 7 and removes slot 7.
    fn a9c8e(&mut self, t: u8) -> usize {
        for y in (8..=0x17usize).rev() {
            if self.o[y].ty != 0 && self.o[y].ty == t {
                return y;
            }
        }
        self.kill_slot(7);
        7
    }

    /// AI `$31` (`$3A9B56`): the rock monster. On tick 4 of its animation: with frame 0 it breaks into six pieces (`$32`), later it removes itself and
    /// wakes the nest (`$30`) again.
    fn ai_9b56(&mut self, x: usize) {
        if self.o[x].atick != 4 {
            return;
        }
        if self.o[x].aframe != 0 {
            self.despawn(x);
            let y = self.a9c8e(0x30);
            if self.o[y].ty == 0 {
                return;
            }
            self.o[y].aid = 0x52;
            self.o[y].aframe = 0;
            self.o[y].atick = 0;
            self.o[y].hitbox = 0x93;
            self.o[y].s[0] = 0;
            self.o[y].flags &= 0xBF;
            self.o[y].ptr = 0x9B07;
            return;
        }
        self.o[x].flags |= 4;
        self.o[x].ptr = 0x9C0E;
        let sum = self.rng[1] as u16 + self.rng[3] as u16 + self.carry as u16;
        self.rng[3] = sum as u8;
        self.carry = sum > 0xFF;
        let k = (self.rng[3] & 3) as u16;
        self.o[x].s[0] = self.ai_byte(0x9B52 + k);
        for e in (0..=5u16).rev() {
            let Some(y) = self.find_slot() else { return };
            self.o[y].ty = 0x32;
            let idx = self.ai_byte(0x9C0F + e);
            self.child_at(x, y, idx, 0x55);
            self.o[y].flags |= 8;
            self.o[y].s[0] = self.o[x].s[0];
            self.o[y].s[1] = self.o[x].s[0];
            self.o[y].xvs = self.ai_byte(0x9C15 + e);
            self.o[y].xvp = self.ai_byte(0x9C1B + e);
            self.o[y].yvs = self.ai_byte(0x9C21 + e);
            self.o[y].yvp = self.ai_byte(0x9C27 + e);
            self.o[y].dir = self.ai_byte(0x9C2D + e);
            self.o[y].hitbox = 0x8D;
        }
    }

    /// AI `$32` (`$3A9C33`): a flying piece: it flies out for `$0468` frames, turns around for `$0480` frames, then removes itself and makes the hidden
    /// monster (`$31`) reappear.
    fn ai_9c33(&mut self, x: usize) {
        self.o[x].atick = 0;
        self.move_h(x);
        self.move_v(x);
        self.o[x].flags &= 0xBF;
        self.o[x].s[0] = self.o[x].s[0].wrapping_sub(1);
        if self.o[x].s[0] != 0 {
            return;
        }
        if self.o[x].aframe == 0 {
            self.o[x].aframe = 1;
            self.o[x].dir ^= 0x0F;
            self.o[x].s[0] = self.o[x].s[1];
            return;
        }
        self.despawn(x);
        let y = self.a9c8e(0x31);
        if self.o[y].ty == 0 {
            return;
        }
        self.o[y].flags &= 0xFB;
        self.o[y].aframe = 1;
        self.o[y].atick = 0;
        self.o[y].ptr = 0x9B56;
    }

    /// AI `$64` (`$3BABD5`, list type `49`): drifts; every 8 frames it turns its heading toward Mega Man and takes the matching speed.
    fn ai_abd5(&mut self, x: usize) {
        self.o[x].s[0] = self.o[x].s[0].wrapping_add(1);
        if self.o[x].s[0] & 7 != 0 {
            self.move_h(x);
            self.move_v(x);
            return;
        }
        let y = self.aim(x) as u16;
        self.o[x].xvs = self.ai_byte(0xAC01 + y);
        self.o[x].xvp = self.ai_byte(0xAC15 + y);
        self.o[x].yvs = self.ai_byte(0xAC05 + y);
        self.o[x].yvp = self.ai_byte(0xAC19 + y);
    }

    /// AI `$B7` (`$3BB780`, list type `78`, also stage 6): a walker that stops near Mega Man, turns at ledges and walls.
    fn ai_b780(&mut self, st: &Stage, x: usize) {
        self.o[x].xvs = 0xC0;
        self.o[x].xvp = 0;
        if self.dist_y(x) < 8 {
            self.o[x].xvs = 0;
            self.o[x].xvp = 2;
        }
        self.vertical_collide(st, x, 0x0E);
        let i = (self.o[x].dir & 1) as usize;
        let front_solid = self.pt.get(i).copied().unwrap_or(0) & 1 != 0;
        if !front_solid || self.horizontal_collide(st, x, 0x18) {
            self.o[x].dir ^= 3;
        }
        self.o[x].flags &= 0xBF;
    }

    /// `$3DA22E`: byte index into the `$0680` bitmap for tile position `$04B0` (one bitmap per page parity in horizontal sections).
    fn bar_byte(&self, st: &Stage, x: usize) -> usize {
        let half = if st.vsect.get() { 0 } else { (self.o[x].xpg & 1) as usize };
        half * 32 + (self.o[x].s[3] as usize >> 3)
    }

    /// AI `$20..$23` waiting state (`$3DA000`, list types `08`-`0E`): a bar slider. Its position and size come from tables by object type and page; when Mega Man stands
    /// on it and a bar slot is free it starts to slide along the bar (`$A0CB`), erasing the bar tiles it passes.
    fn ai_a000(&mut self, _st: &Stage, x: usize) {
        let base = (self.o[x].ty & 3) << 3;
        let idx = base.wrapping_add(self.ai_byte(0xA257 + self.o[x].xpg as u16)) as u16;
        self.o[x].hitbox = self.ai_byte(0xA271 + idx);
        self.o[x].xpx = self.ai_byte(0xA26D + idx);
        if !self.touches_player(x) {
            return;
        }
        if self.p.ypx >= self.o[x].ypx {
            return;
        }
        let Some(yb) = (0..4usize).rev().find(|&i| self.b120[i] & 0x80 == 0) else { return };
        self.o[x].yvp = yb as u8;
        let a = self.ai_byte(0xA26B + idx);
        let b = self.ai_byte(0xA26C + idx);
        self.o[x].xpx = self.ai_byte(0xA272 + idx);
        self.o[x].s[0] = 0;
        self.o[x].s[4] = b;
        self.b120[yb] = 0x80 | self.o[x].xpg;
        self.o[x].s[1] = 0;
        self.b12c[yb] = 0;
        self.b128[yb] = b;
        self.b124[yb] = a;
        self.o[x].s[2] = self.ai_byte(0xA26E + idx);
        let n = self.ai_byte(0xA270 + idx);
        self.o[x].s[3] = n;
        self.o[x].hp = n;
        self.o[x].s[5] = self.ai_byte(0xA271 + idx);
        self.o[x].s[6] = 0;
        self.o[x].hitbox = 1;
        self.o[x].flags |= 2;
        let dir = self.ai_byte(0xA26F + idx);
        self.o[x].dir = dir;
        self.o[x].s[7] = self.o[x].xpx;
        let t = self.o[x].xpx.wrapping_add(self.ai_byte(0xA334 + (dir & 2) as u16));
        self.o[x].ysub = t;
        self.o[x].yvs = t;
        self.face_flag(x);
        self.o[x].ptr = 0xA0CB;
    }

    /// The step both slider states share (`$3DA128`): next tile, redraw data, next target X (a tile centre), step counter.
    fn bar_advance(&mut self, x: usize) {
        let yd = (self.o[x].dir & 2) as u16;
        self.o[x].s[3] = self.o[x].s[3].wrapping_add(self.ai_byte(0xA337 + yd));
        let d0 = self.ai_byte(0xA333 + yd);
        let d1 = self.ai_byte(0xA334 + yd);
        let b = (self.o[x].yvp & 3) as usize;
        self.b128[b] = self.b128[b].wrapping_add(d0);
        self.o[x].ysub = (self.o[x].xpx.wrapping_add(d1) & 0xF0) | 0x08;
        self.o[x].s[6] = self.o[x].s[6].wrapping_add(1);
        self.b120[b] |= 0x40;
    }

    /// The slider's horizontal move (`$A0D4`-`$A102`): moves with a speed of 4 px/frame unless bit 7 of its flags is set; true when it reached its target.
    fn bar_move(&mut self, x: usize) -> bool {
        let save = self.o[x].xvp;
        if self.o[x].flags & 0x80 == 0 {
            self.o[x].xvp = 4;
        }
        self.move_h(x);
        self.o[x].xvp = save;
        let o = &self.o[x];
        if o.dir & 1 != 0 {
            o.xpx >= o.ysub
        } else {
            o.ysub >= o.xpx
        }
    }

    /// AI `$20..$23` sliding state (`$3DA0CB`): erases the bar tile under it (sets its bit) at every tile it reaches.
    fn ai_a0cb(&mut self, st: &Stage, x: usize) {
        if self.o[x].s[0] != 0 {
            self.o[x].s[0] -= 1;
            return;
        }
        if !self.bar_move(x) {
            return;
        }
        if self.o[x].s[5] == self.o[x].s[6] {
            // $A167: the slide is over, back to the start position and the returning state
            self.o[x].ptr = 0xA1A4;
            let b = (self.o[x].yvp & 3) as usize;
            self.b12c[b] = self.o[x].s[2];
            if self.o[x].aid != 0x2A {
                self.set_anim(x, 0x4B);
            }
            self.b128[b] = self.o[x].s[4];
            self.o[x].xpx = self.o[x].s[7];
            self.o[x].ysub = self.o[x].yvs;
            self.o[x].s[3] = self.o[x].hp;
            self.o[x].s[6] = 0;
            return;
        }
        let m = self.rom.cpu_byte(0x3E, 0x3F, 0xFCBF + (self.o[x].s[3] & 7) as u16);
        let i = self.bar_byte(st, x) & 63;
        st.blocks[i].set(st.blocks[i].get() | m);
        self.bar_advance(x);
    }

    /// AI `$20..$23` returning state (`$3DA1A4`): the same walk back, putting the bar tiles back (clears their bits); at the end it is the waiting state again.
    fn ai_a1a4(&mut self, st: &Stage, x: usize) {
        if self.o[x].s[0] != 0 {
            self.o[x].s[0] -= 1;
            return;
        }
        if !self.bar_move(x) {
            return;
        }
        if self.o[x].s[5] == self.o[x].s[6] {
            // $A204
            self.o[x].flags &= 0xFD;
            self.o[x].dir = 0;
            let b = (self.o[x].yvp & 3) as usize;
            self.b120[b] = 0;
            self.o[x].ptr = 0xA000;
            if self.o[x].aid != 0x2A {
                self.set_anim(x, 0x29);
            }
            return;
        }
        let m = self.rom.cpu_byte(0x3E, 0x3F, 0xFCC7 + (self.o[x].s[3] & 7) as u16);
        let i = self.bar_byte(st, x) & 63;
        st.blocks[i].set(st.blocks[i].get() & m);
        self.bar_advance(x);
    }
}
