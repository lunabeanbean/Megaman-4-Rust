//! The stage-2 boss (AI `$7E`, code in bank `$38` at `$A000`-`$BFFF`: `$B045`...). A child module of `objects` so it can use the private helpers.
//!
//! The boss is a chain of states like every other object: each state stores the address of the next one in `ptr`. Names below are the
//! CPU addresses of the states (`ai_bXXX`). `$B7xx`-`$B8xx` hold the defeat sequence.
use super::*;

impl World {
    /// Dispatch for the boss's own states; returns false for an address that is not a boss state.
    pub(super) fn run_boss(&mut self, st: &Stage, x: usize, ptr: u16) -> bool {
        match ptr {
            0xB045 => self.boss_b045(x),
            0xB04D => self.boss_b04d(x),
            0xB06E => self.boss_b06e(st, x),
            0xB087 => self.boss_b087(st, x),
            0xB104 => self.boss_b104(st, x),
            0xB137 => self.boss_b137(st, x),
            0xB146 => self.boss_b146(st, x),
            0xB16B => self.boss_b16b(st, x),
            0xB1C8 => self.boss_b1c8(x),
            0xB1F4 => self.boss_b1f4(st, x),
            0xB2E3 => self.boss_b2e3(x),
            0x9F85 | 0x9F94 => self.ai_9f85(x),
            0xB2AF => self.boss_b2af(st, x),
            0xAFD4 => self.boss_defeat(st, x),
            0xB051 => self.boss_b051(x),
            0xB0F4 => self.sparkle_b0f4(x),
            0xB23C => self.boss_b23c(st, x),
            _ => return false,
        }
        true
    }

    fn set_ptr(&mut self, x: usize, p: u16) {
        self.o[x].ptr = p;
    }

    /// Byte of the boss code bank (`$38` at `$A000`).
    pub(super) fn b38(&self, a: u16) -> u8 {
        self.rom.cpu_byte(0x3A, 0x38, a)
    }

    /// `$B045`: first state after the boss appeared. Sprites are drawn with priority line `$B8` this frame; then the idle choice below.
    fn boss_b045(&mut self, x: usize) {
        self.ef_override = Some(0xB8);
        self.boss_pick(x, 1);
    }

    /// `$B055`: start the animation `$B28F[y]`; the next state follows from the animation id (`$B2A7/$B2AB` indexed by `aid & 3`).
    fn boss_pick(&mut self, x: usize, y: u8) {
        let anim = self.b38(0xB28F + y as u16);
        self.set_anim(x, anim);
        let i = (self.o[x].aid & 3) as u16;
        let p = self.b38(0xB2A7 + i) as u16 | (self.b38(0xB2AB + i) as u16) << 8;
        self.set_ptr(x, p);
    }

    /// `$B71E`/`$B724`: face Mega Man and pick a move: row by distance (`< $40`, `< $80`, far), column by a random number.
    /// Returns the index into the move tables. The `ADC`s use the carry the caller left.
    pub(super) fn boss_choose(&mut self, x: usize, face: bool) -> u8 {
        if face {
            self.face_player(x);
            self.face_flag(x);
        }
        let s1 = self.rng[3] as u16 + self.rng[0] as u16 + self.carry as u16; // LDA $E7 / ADC $E4 / ADC $E5
        let s2 = (s1 as u8) as u16 + self.rng[1] as u16 + (s1 > 0xFF) as u16;
        let col = (s2 as u8) & 7;
        let (d, _) = self.dist_pure(x);
        let mut y = 2u8;
        while y != 0 {
            if d >= self.b38(0xB744 + y as u16) {
                break;
            }
            y -= 1;
        }
        y * 8 + col
    }

    /// `$B04D`: idle; when the timer runs out choose the next move.
    fn boss_b04d(&mut self, x: usize) {
        self.o[x].s[0] = self.o[x].s[0].wrapping_sub(1);
        if self.o[x].s[0] != 0 {
            return;
        }
        let y = self.boss_choose(x, true);
        self.boss_pick(x, y);
    }

    /// `$B06E`: walk toward Mega Man at 1.5 px/frame for `$2A` frames.
    fn boss_b06e(&mut self, st: &Stage, x: usize) {
        self.o[x].xvs = 0x80;
        self.o[x].xvp = 0x01;
        self.o[x].s[1] = 0x2A;
        self.set_ptr(x, 0xB087);
        self.boss_b087(st, x);
    }

    /// `$B087`: walk; at the end turn to Mega Man (animation `$31`), at animation frame 1 throw a `$7F`, at frame 3 resume walking.
    fn boss_b087(&mut self, st: &Stage, x: usize) {
        if self.o[x].s[1] != 0 {
            self.o[x].s[1] -= 1;
            if self.o[x].s[1] == 0 {
                self.o[x].s[2] = self.o[x].dir;
                self.face_player(x);
                self.face_flag(x);
                self.set_anim(x, 0x31);
                return;
            }
            // $B0F9: walk (probe $32); a wall ends the move
            if self.horizontal_collide(st, x, 0x32) {
                self.boss_end_move(x);
            }
            return;
        }
        match self.o[x].aframe {
            1 => {
                self.o[x].aframe += 1;
                let Some(y) = self.find_slot() else { return };
                let dir = self.o[x].dir;
                self.o[y].dir = dir;
                self.child_at(x, y, (dir & 1) + 0x4A, 0x33);
                let c = &mut self.o[y];
                c.ty = 0x7F;
                c.hitbox = 0xC0;
                c.hp = 1;
                c.xvs = 0;
                c.xvp = 4;
                c.s[0] = 0x10;
            }
            3 => {
                self.o[x].dir = self.o[x].s[2];
                self.face_flag(x);
                self.o[x].s[1] = 0x1A;
                self.set_anim(x, 0x30);
            }
            _ => {}
        }
    }

    /// `$B26D`: end of a move: face Mega Man, back to idle for `$1E` frames.
    fn boss_end_move(&mut self, x: usize) {
        self.face_player(x);
        self.face_flag(x);
        self.o[x].s[1] = 0;
        self.o[x].s[2] = 0;
        self.set_ptr(x, 0xB04D);
        self.o[x].s[0] = 0x1E;
        self.set_anim(x, 0x29);
    }

    /// `$B104`: jump toward Mega Man (X speed so that the distance + `$20` takes `$37` frames, Y speed 6.9 px/frame up).
    fn boss_b104(&mut self, st: &Stage, x: usize) {
        let (d, _) = self.dist_pure(x);
        let (q4, q5) = Self::div_fc3a(0, d.wrapping_add(0x20), 0, 0x37); // CLC / ADC #$20
        self.o[x].xvs = q4;
        self.o[x].xvp = q5;
        self.set_ptr(x, 0xB137);
        self.o[x].yvs = 0xEE;
        self.o[x].yvp = 0x06;
        self.boss_b137(st, x);
    }

    /// `$B137`: airborne until landing; sideways with probe `$32`.
    fn boss_b137(&mut self, st: &Stage, x: usize) {
        if self.vertical_collide(st, x, 0x0A) {
            self.boss_end_move(x);
        } else {
            self.horizontal_collide(st, x, 0x32);
        }
    }

    /// `$B146`: wait for animation frame 2, then leap straight up (4.9 px/frame) with hitbox `$B6`.
    fn boss_b146(&mut self, st: &Stage, x: usize) {
        if self.o[x].aframe != 2 {
            return;
        }
        self.set_anim(x, 0x2C);
        self.o[x].hitbox = 0xB6;
        self.o[x].yvs = 0xE5;
        self.o[x].yvp = 0x04;
        self.set_ptr(x, 0xB16B);
        self.boss_b16b(st, x);
    }

    /// `$B16B`: rising / falling animation switch, then the floor probe; landing makes dust and starts sinking.
    fn boss_b16b(&mut self, st: &Stage, x: usize) {
        let aid = self.o[x].aid;
        if 0x2D >= aid {
            if aid != 0x2D {
                if self.o[x].yvp & 0x80 != 0 {
                    self.set_anim(x, 0x2D);
                }
            } else if self.o[x].aframe != 0 {
                self.set_anim(x, 0x2E);
            }
        }
        if self.vertical_collide(st, x, 0x0C) {
            self.set_ptr(x, 0xB1C8);
            self.o[x].yvs = 0;
            self.o[x].yvp = 1;
            self.o[x].s[3] = 0x10;
            self.boss_dust(x);
        }
    }

    /// `$B1AB`: a dust puff (type `$82`, animation `$2B`) at the boss's position.
    fn boss_dust(&mut self, x: usize) {
        let Some(y) = self.find_slot() else { return };
        self.child_here(x, y, 0x2B);
        let c = &mut self.o[y];
        c.ty = 0x82;
        c.hitbox = 0;
        c.hp = 0;
        c.s[0] = 2;
    }

    /// `$B1C8`: after the pause sink into the ground (1 px/frame) until y > `$D0`, then hide.
    fn boss_b1c8(&mut self, x: usize) {
        if self.o[x].s[3] != 0 {
            self.o[x].s[3] -= 1;
            if self.o[x].s[3] != 0 {
                return;
            }
        }
        self.move_down_by_speed(x);
        if 0xD0 >= self.o[x].ypx {
            return;
        }
        self.o[x].ypx = 0xD0;
        self.set_ptr(x, 0xB1F4);
        self.o[x].hitbox = 0;
        self.o[x].s[1] = 0xD2;
    }

    /// `$B1F4`: hidden; at count `$1E` remember where Mega Man is (X clamped to `$20..=$E0`), at 0 jump out of the ground there.
    fn boss_b1f4(&mut self, st: &Stage, x: usize) {
        self.o[x].s[1] = self.o[x].s[1].wrapping_sub(1);
        if self.o[x].s[1] == 0 {
            // $B215
            self.set_ptr(x, 0xB23C);
            self.o[x].xpx = self.o[x].s[2];
            self.o[x].yvs = 0x52;
            self.o[x].yvp = 0x06;
            self.set_anim(x, 0x2C);
            self.o[x].hitbox = 0xF6;
            self.boss_dust(x);
            self.boss_b23c(st, x);
            return;
        }
        if self.o[x].s[1] == 0x1E {
            let px = self.p.xpx;
            self.o[x].s[2] = px.clamp(0x20, 0xE0);
        }
    }

    /// `$B23C`: leaping out of the ground: animation switches, rises, then falls back to the floor.
    pub(super) fn boss_b23c(&mut self, st: &Stage, x: usize) {
        let aid = self.o[x].aid;
        if aid == 0x2A {
            if self.o[x].aframe == 0 {
                return;
            }
            self.set_anim(x, 0x2F);
        } else if aid == 0x2C && self.o[x].yvp & 0x80 != 0 {
            self.set_anim(x, 0x2A);
        }
        if self.o[x].yvp & 0x80 == 0 {
            self.move_gravity(x);
        } else if self.vertical_collide(st, x, 0x0A) {
            self.boss_end_move(x);
        }
    }

    /// AI `$82` (`$B2E3`): the dust puff sheds three `$CC` particles, 8 frames apart, at fixed offsets, then disappears.
    fn boss_b2e3(&mut self, x: usize) {
        if self.o[x].s[1] != 0 {
            self.o[x].s[1] -= 1;
            if self.o[x].s[1] != 0 {
                return;
            }
        }
        let Some(y) = self.find_slot() else { return };
        self.child_here(x, y, 0x2B);
        let i = self.o[x].s[0] as u16;
        let dx = self.b38(0xB32A + i);
        let dy = self.b38(0xB32D + i);
        let c = &mut self.o[y];
        c.ty = 0xCC;
        c.hitbox = 0;
        c.xpx = c.xpx.wrapping_add(dx);
        c.ypx = 0xC0u8.wrapping_sub(dy);
        self.o[x].s[1] = 8;
        self.o[x].s[0] = self.o[x].s[0].wrapping_sub(1);
        if self.o[x].s[0] & 0x80 != 0 {
            self.despawn(x);
        }
    }

    /// AI `$7F` (`$B2AF`): the boss's thrown shot flies sideways; every `$14` frames a random roll (or a wall) makes it burst into a `$83`.
    fn boss_b2af(&mut self, st: &Stage, x: usize) {
        self.o[x].s[0] = self.o[x].s[0].wrapping_sub(1);
        if self.o[x].s[0] == 0 {
            // LDA $E5 / ADC $E6 with the carry the AI loop left
            let a = self.rng[1].wrapping_add(self.rng[2]).wrapping_add(self.carry as u8);
            if a & 3 == 0 {
                self.boss_burst(x);
                return;
            }
            self.o[x].s[0] = 0x14;
        }
        if self.horizontal_collide(st, x, 0x0E) {
            self.boss_burst(x);
        }
    }

    /// `$B2C8`: the shot becomes a type `$83` burst (animation `$62`).
    fn boss_burst(&mut self, x: usize) {
        self.despawn(x);
        self.o[x].ty = 0x83;
        self.o[x].s[0] = 0;
        self.o[x].s[1] = 0;
        self.set_anim(x, 0x62);
        self.o[x].hitbox = 0x94;
    }

    /// AI `$83` (`$3A9F85`, state `$9F94`): a burst. Every other frame it makes one or two `$80` puffs around itself (offsets `$9FDF[n]`), nine in all.
    fn ai_9f85(&mut self, x: usize) {
        if self.o[x].ptr == 0x9F85 {
            self.snd(0x28, 0x9F8A); // code_3A9F87
        }
        self.set_ptr(x, 0x9F94);
        let mut n = 0u8;
        if self.o[x].s[0] != 0 {
            self.o[x].s[0] -= 1;
            if self.o[x].s[0] != 0 {
                return;
            }
            n = 1;
        }
        loop {
            let Some(y) = self.find_slot() else { return };
            let c = &mut self.o[y];
            c.ty = 0x80;
            c.hitbox = 0;
            c.hp = 0;
            let idx = self.rom.cpu_byte(0x3A, 0x3B, 0x9FDF + self.o[x].s[1] as u16);
            self.child_at(x, y, idx, 0x62);
            self.o[x].s[1] += 1;
            if n == 0 {
                break;
            }
            n -= 1;
        }
        self.o[x].s[0] = 2;
        if self.o[x].s[1] == 9 {
            self.despawn(x);
        }
    }

    /// AI `$77` (`$35AFD4`): the boss is dead. Everything except Mega Man's shots disappears, the boss explodes into 16 particles and the
    /// meter goes away; then the object waits (`$B051`) until the room is empty.
    fn boss_defeat(&mut self, st: &Stage, x: usize) {
        self.snd_music(0xF0, 0xAE81); // code_35AE7C
        self.snd(0x25, 0xAE86);
        for y in (6..=23).rev() {
            if y != x {
                self.kill_slot(y);
            }
        }
        // code_35AE7C: 16 `$2E` particles at the position of slot 23, flying out along the tables at `$AF94`
        for i in (0..16u16).rev() {
            let Some(y) = (5..=23).rev().find(|&s| self.o[s].ty == 0) else { break };
            self.child_here(23, y, 0x19);
            let xvs = self.ai_byte(0xAF94 + i);
            let xvp = self.ai_byte(0xAFA4 + i);
            let yvs = self.ai_byte(0xAFB4 + i);
            let yvp = self.ai_byte(0xAFC4 + i);
            let c = &mut self.o[y];
            c.flags = 0x90;
            c.ty = 0x2E;
            c.hitbox = 0;
            c.xvs = xvs;
            c.xvp = xvp;
            c.yvs = yvs;
            c.yvp = yvp;
        }
        self.meter2 = 0;
        if st.id >= 8 {
            // code_35AFFA: slot 5 gets a `C6` object at the boss's place; this slot turns into the exit pod `B3` of the boss room
            // (x $20, y $B4, placement index `$0149`, marked as taken), and `$AC` records the defeated robot master
            self.kill_slot(5);
            self.child_here(x, 5, 0x1A);
            self.o[5].ty = 0xC6;
            self.o[5].hitbox = 0;
            self.despawn(x);
            self.set_anim(x, 0x73);
            let idx = self.c149;
            let ab = self.ab;
            let o = &mut self.o[x];
            o.flags = 0x90;
            o.hitbox = 0x1C;
            o.ty = 0xB3;
            o.xpx = 0x20;
            o.ypx = 0xB4;
            o.idx = idx;
            o.ptr = 0;
            let mask = self.rom.cpu_byte(0x3E, 0x3F, 0xFCBF + (idx & 7) as u16);
            self.defeated[(idx >> 3) as usize & 31] |= mask;
            let bit = self.rom.cpu_byte(0x3E, 0x3F, 0xFCBF + (ab & 7) as u16);
            self.ac |= bit;
            return;
        }
        self.set_anim(x, 0);
        self.set_ptr(x, 0xB051);
        self.boss_b051(x);
    }

    /// `$B051`: wait until Mega Man is on his feet and nothing but this object is left; then he gets the victory state `$0B`.
    fn boss_b051(&mut self, x: usize) {
        let _ = x;
        if self.p.state == 0x0B {
            self.boss_b093(x);
            return;
        }
        if self.p.state >= 3 {
            return;
        }
        for y in (5..=23).rev() {
            let t = self.o[y].ty;
            if t != 0 && t != 0x77 {
                return;
            }
        }
        self.snd_music(0x0C, 0xB076); // code_35B073
        self.player_cmd = Some(0x0B);
        for y in 1..=4 {
            self.kill_slot(y); // code_3FFDDE: his shots are removed
        }
    }

    /// `$B093`: Mega Man has finished his victory jump (falling, X `$80`, Y `$6C`): four sparkles (`$2A`) spawn at the corners `$B17C`
    /// and the object disappears (also when no slot is free).
    fn boss_b093(&mut self, x: usize) {
        if self.p.falling && self.p.xpx == 0x80 && self.p.ypx == 0x6C {
            for k in (0..4usize).rev() {
                let Some(y) = self.find_slot() else { break };
                self.child_here(x, y, 0x19);
                let (sy, sx, dir) = (self.ai_byte(0xB17C + 2 * k as u16), self.ai_byte(0xB17D + 2 * k as u16), self.ai_byte(0xB184 + k as u16));
                let c = &mut self.o[y];
                c.hitbox = 0;
                c.hp = 0;
                c.ptr = 0xB0F4;
                c.ty = 0x2A;
                c.ypx = sy;
                c.xpx = sx;
                c.s[0] = dir;
            }
            self.despawn(x);
        }
    }

    /// AI `$2A` (`$B0F4`): a sparkle follows the offset table at `$B188` (even `$0468`) or `$B213` (odd), mirrored for `$0468 >= 2`; the entry `$80` ends it.
    fn sparkle_b0f4(&mut self, x: usize) {
        let y = self.o[x].s[1] as u16;
        let dir = self.o[x].s[0];
        let base = if dir & 1 == 0 { 0xB188 } else { 0xB213 };
        let (dy, dx) = (self.ai_byte(base + y), self.ai_byte(base + 1 + y));
        if dy == 0x80 {
            self.despawn(x);
            return;
        }
        let (dy, dx) = if dir >= 2 { ((dy ^ 0xFF).wrapping_add(1), (dx ^ 0xFF).wrapping_add(1)) } else { (dy, dx) };
        let o = &mut self.o[x];
        let sum = o.ypx as u16 + dy as u16;
        o.ypx = sum as u8;
        if o.ypx >= 0xF0 {
            // the carry of CMP #$F0 is set here
            if dy & 0x80 != 0 {
                o.ypx = o.ypx.wrapping_sub(0x10);
                o.ypg = o.ypg.wrapping_sub(1);
            } else {
                o.ypx = o.ypx.wrapping_add(0x10);
                o.ypg = o.ypg.wrapping_add(1);
            }
        }
        let sx = if dx & 0x80 != 0 { 0xFF } else { 0 };
        let (p, c) = o.xpx.overflowing_add(dx);
        o.xpx = p;
        o.xpg = o.xpg.wrapping_add(sx).wrapping_add(c as u8);
        o.s[1] = o.s[1].wrapping_add(2);
    }

    /// code_3FFC3A: `($01:$00:00) / ($03:$02:00 >> 8)`: 16-bit quotient in (`$05:$04`), bit by bit as the 6502 does it.
    pub(super) fn div_fc3a(v0: u8, v1: u8, v2: u8, v3: u8) -> (u8, u8) {
        if v0 | v1 | v2 | v3 == 0 {
            return (0, 0);
        }
        let (mut q6, mut r0, mut r1, mut r7) = (0u8, v0, v1, 0u8);
        for _ in 0..16 {
            let c6 = q6 >> 7;
            q6 <<= 1;
            let c0 = r0 >> 7;
            r0 = (r0 << 1) | c6;
            let c1 = r1 >> 7;
            r1 = (r1 << 1) | c0;
            r7 = (r7 << 1) | c1;
            let diff = (r1 as i32 | (r7 as i32) << 8) - (v2 as i32 | (v3 as i32) << 8);
            if diff >= 0 {
                r1 = diff as u8;
                r7 = (diff >> 8) as u8;
                q6 += 1;
            }
        }
        (q6, r0)
    }

    /// code_3FF269: move by the Y speed with the sign of "down" (add), wrapping into the next screen at `$F0`.
    fn move_down_by_speed(&mut self, x: usize) {
        let o = &mut self.o[x];
        let (s, c) = o.ysub.overflowing_add(o.yvs);
        o.ysub = s;
        let (p, c2) = o.ypx.overflowing_add(o.yvp);
        let (p, c3) = p.overflowing_add(c as u8);
        let _ = (c2, c3);
        o.ypx = p;
        if p >= 0xF0 {
            o.ypx = p.wrapping_add(0x10);
            o.ypg = o.ypg.wrapping_add(1);
        }
    }
}
