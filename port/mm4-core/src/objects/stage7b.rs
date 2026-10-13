//! More stage 7 enemies in bank `$3B`: the shielded thrower (`46`/`4B`, AI `$69`), the cannon (`48`, AI `$62`), the emitter (`54`, AI `$BD`) and the walker (`68`, AI `$9C`).
use super::*;

impl World {
    /// Dispatch for these states; returns false for an address that is not one of them.
    pub(super) fn run_stage7b(&mut self, st: &Stage, x: usize, ptr: u16) -> bool {
        match ptr {
            0xAC95 => self.ai_ac95(x),
            0xACD9 => self.ai_acd9(x),
            0xAD6A | 0xABCF => {
                self.move_h(x);
                self.move_gravity(x);
            }
            0xAACC => self.ai_aacc(x),
            0xAB70 => self.ai_ab70(x),
            0xB8C4 => self.ai_b8c4(x),
            0xB94E => self.ai_b94e(x),
            0xB2A3 => self.ai_b2a3(x),
            0xB2C5 => self.ai_b2c5(st, x),
            0xB2F7 => self.ai_b2f7(st, x),
            _ => return false,
        }
        true
    }

    /// `$3BA436`: the hitbox is vulnerable (bit 6) every frame except when a shot touches the enemy from the front.
    fn shield(&mut self, x: usize) {
        self.o[x].hitbox |= 0x40;
        if let Some(y) = self.shot_hits(x) {
            let (e, s) = (self.o[x], self.o[y]);
            let enemy_right = ((e.xpg as u16) << 8 | e.xpx as u16) >= ((s.xpg as u16) << 8 | s.xpx as u16);
            let facing_right = e.flags & 0x40 != 0;
            if enemy_right != facing_right {
                self.o[x].hitbox &= 0xBF;
            }
        }
    }

    /// AI `$69` resting state (`$3BAC95`, list types `46`/`4B`): shielded thrower. After its wait it starts the throw when Mega Man is in front of it.
    fn ai_ac95(&mut self, x: usize) {
        self.shield(x);
        if self.o[x].s[0] != 0 {
            self.o[x].s[0] -= 1;
            if self.o[x].s[0] != 0 {
                return;
            }
        }
        let o = self.o[x];
        let me = (o.xpg as u16) << 8 | o.xpx as u16;
        let pl = (self.p.xpg as u16) << 8 | self.p.xpx as u16;
        let go = if o.flags & 0x40 != 0 { pl < me } else { me < pl };
        if go {
            self.o[x].ptr = 0xACD9;
            self.set_anim(x, 0x73);
        }
    }

    /// AI `$69` throwing state (`$3BACD9`): on tick 5 of animation frame 8 it throws a ball (`$6A`) whose speed depends on the distance, at frame 9 it rests again.
    fn ai_acd9(&mut self, x: usize) {
        self.shield(x);
        if self.o[x].atick != 5 {
            return;
        }
        match self.o[x].aframe {
            8 => {
                let d = self.dist_x(x);
                let k: u16 = if d >= 0x40 {
                    0
                } else if d >= 0x30 {
                    1
                } else {
                    2
                };
                let Some(y) = self.find_slot() else { return };
                self.carry = false;
                let left = self.o[x].flags & 0x40 != 0;
                let c = &mut self.o[y];
                c.ty = 0x6A;
                c.hitbox = 0x8F;
                c.dir = if left { 2 } else { 1 };
                self.child_at(x, y, if left { 0x44 } else { 0x45 }, 0x18);
                let (a, b, cc, dd) = (self.ai_byte(0xAD5E + k), self.ai_byte(0xAD61 + k), self.ai_byte(0xAD64 + k), self.ai_byte(0xAD67 + k));
                let c = &mut self.o[y];
                c.xvs = a;
                c.xvp = b;
                c.yvs = cc;
                c.yvp = dd;
            }
            9 => {
                self.o[x].s[0] = 0x40;
                self.o[x].ptr = 0xAC95;
                self.set_anim(x, 0x72);
            }
            _ => {}
        }
    }

    /// AI `$62` closed state (`$3BAACC`, list type `48`): a cannon that shoots from time to time; a hit opens it up.
    fn ai_aacc(&mut self, x: usize) {
        self.o[x].hp = 3;
        self.face_player(x);
        self.face_flag(x);
        if self.shot_hits(x).is_some() {
            self.o[x].ptr = 0xAB70;
            self.set_anim(x, 0x95);
            return;
        }
        if self.o[x].aframe == 0 {
            if self.o[x].s[0] == 0 {
                self.o[x].s[0] = 0x40;
                return;
            }
            self.o[x].s[0] -= 1;
            if self.o[x].s[0] == 0 {
                self.o[x].aframe += 1;
            } else {
                self.o[x].atick = 0;
            }
            return;
        }
        if self.o[x].atick != 3 {
            return;
        }
        match self.o[x].aframe {
            4 => self.aacc_shoot(x),
            8 => {
                self.o[x].aframe = 0;
                self.o[x].atick = 0;
                self.o[x].s[0] = 0x40;
            }
            _ => {}
        }
    }

    /// `$3BAB21`: a bullet (`$63`) whose speed table row grows with the distance to Mega Man.
    fn aacc_shoot(&mut self, x: usize) {
        let d = self.dist_x(x);
        let mut k: u16 = 4;
        while k > 0 {
            if d >= self.ai_byte(0xABB6 + k) {
                break;
            }
            k -= 1;
        }
        let Some(y) = self.find_slot() else { return };
        self.carry = false;
        let dir = self.o[x].dir;
        let c = &mut self.o[y];
        c.ty = 0x63;
        c.hitbox = 0x8F;
        c.dir = dir;
        self.child_at(x, y, 0x3D + (dir & 1), 0x96);
        let (a, b, cc, dd) = (self.ai_byte(0xABBB + k), self.ai_byte(0xABC0 + k), self.ai_byte(0xABC5 + k), self.ai_byte(0xABCA + k));
        let c = &mut self.o[y];
        c.xvs = a;
        c.xvp = b;
        c.yvs = cc;
        c.yvp = dd;
    }

    /// AI `$62` opened state (`$3BAB70`): vulnerable for a while (hitbox `$8D`), invulnerable and frozen on animation frame 2 for 64 frames, closes at frame `$0B`.
    fn ai_ab70(&mut self, x: usize) {
        self.o[x].hp = 3;
        if self.o[x].s[1] != 0 {
            self.o[x].s[1] -= 1;
            if self.o[x].s[1] != 0 {
                self.o[x].atick = 0;
                return;
            }
            self.o[x].aframe += 1;
        }
        self.o[x].hitbox = 0x8D;
        match self.o[x].aframe {
            2 => {
                self.o[x].hitbox = 0;
                self.o[x].s[1] = 0x40;
                self.o[x].atick = 0;
            }
            0x0B => {
                self.o[x].hitbox = 0xCD;
                self.o[x].ptr = 0xAACC;
                self.set_anim(x, 0x94);
            }
            _ => {}
        }
    }

    /// AI `$BD` (`$3BB8C4`, list type `54`): an emitter. A 16-bit countdown (`$0468:$0480`) spawns a drifting object (`$BE`) at the screen edge.
    fn ai_b8c4(&mut self, x: usize) {
        if self.w13c != 0 {
            return;
        }
        let (s1, borrow) = self.o[x].s[1].overflowing_sub(1);
        let s0 = self.o[x].s[0].wrapping_sub(borrow as u8);
        self.o[x].s[1] = s1;
        self.o[x].s[0] = s0;
        if s0 & 0x80 == 0 {
            return;
        }
        self.o[x].s[0] = 2;
        self.o[x].s[1] = 0x2C;
        let Some(y) = self.find_slot() else { return };
        self.carry = false;
        let cam = self.cam_now;
        let right = self.o[x].ty & 1 != 0;
        self.o[y].ty = 0xBE;
        self.o[y].hitbox = 0xD1;
        self.o[y].hp = 1;
        self.child_here(x, y, 0x1C);
        let py = self.p.ypx;
        let c = &mut self.o[y];
        c.ypx = py;
        c.xvs = 0x80;
        c.yvs = 0x80;
        c.xvp = 0;
        c.yvp = 0;
        c.s[0] = 0x1E;
        let (dir, add) = if right { (0x0A, 0xF8u8) } else { (0x09, 0x08u8) };
        let (px, carry) = cam.lo.overflowing_add(add);
        c.dir = dir;
        c.xpx = px;
        c.xpg = cam.hi.wrapping_add(carry as u8);
    }

    /// AI `$BE` (`$3BB94E`): drifts diagonally; every 30 frames the vertical direction alternates (up, down, down, up).
    fn ai_b94e(&mut self, x: usize) {
        self.move_h(x);
        self.move_v(x);
        self.o[x].s[0] = self.o[x].s[0].wrapping_sub(1);
        if self.o[x].s[0] != 0 {
            return;
        }
        self.o[x].s[1] = self.o[x].s[1].wrapping_add(1);
        let i = (self.o[x].s[1] & 3) as u16;
        let v = self.ai_byte(0xB973 + i);
        self.o[x].dir = (self.o[x].dir & 3) | v;
        self.o[x].s[0] = 0x1E;
    }

    /// AI `$9C` idle state (`$3BB2A3`, list type `68`): wakes up when Mega Man is within 64 px.
    fn ai_b2a3(&mut self, x: usize) {
        if self.dist_x(x) >= 0x40 {
            return;
        }
        let o = &mut self.o[x];
        o.yvs = 0;
        o.yvp = 0;
        o.flags &= 0xFB;
        o.ptr = 0xB2C5;
    }

    /// AI `$9C` falling state (`$3BB2C5`): falls to the floor, plays its landing animation for 20 frames, then faces Mega Man and starts walking.
    fn ai_b2c5(&mut self, st: &Stage, x: usize) {
        if self.o[x].s[0] != 0 {
            self.o[x].atick = 0;
            self.o[x].s[0] -= 1;
            if self.o[x].s[0] != 0 {
                return;
            }
            self.face_player(x);
            self.face_flag(x);
            self.o[x].ptr = 0xB2F7;
            return;
        }
        if self.vertical_collide(st, x, 6) {
            self.set_anim(x, 0x61);
            self.o[x].s[0] = 0x14;
        }
    }

    /// AI `$9C` walking state (`$3BB2F7`): on every second animation frame (tick 4) it steps 4 px; it turns around at a ledge or a wall.
    fn ai_b2f7(&mut self, st: &Stage, x: usize) {
        let f = self.o[x].aframe as u16;
        self.o[x].hitbox = self.ai_byte(0xB376 + f);
        if self.o[x].atick != 4 || f & 1 == 0 {
            return;
        }
        let o = &mut self.o[x];
        let right = o.dir & 1 != 0;
        let (px, c) = if right { o.xpx.overflowing_add(4) } else { o.xpx.overflowing_sub(4) };
        o.xpx = px;
        if right {
            o.xpg = o.xpg.wrapping_add(c as u8);
        } else {
            o.xpg = o.xpg.wrapping_sub(c as u8);
        }
        let (types, _) = self.vprobe(st, x, 0x18);
        let i = if right { 2 } else { 0 };
        if types.get(i).copied().unwrap_or(0) & 1 == 0 {
            self.turn_around(x);
            return;
        }
        let id = (self.ai_byte(0xB36E + f) | ((self.o[x].dir >> 1) & 1)) as usize;
        let (types, _) = self.hprobe(st, x, id);
        if types.iter().any(|t| t & 1 != 0) {
            self.turn_around(x);
        }
    }
}
