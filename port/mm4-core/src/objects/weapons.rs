//! Mega Man's special weapons (bank `$3A`: the shot AIs `$8BA3-$8FE7`, the spawn code is in `player.rs`). Weapons 7..=10 are fired by code_3C8F98 into slot 1.
use super::*;

impl World {
    /// Dispatch for the special-weapon object AIs; returns false for an address that is not one of them.
    pub(super) fn run_weapons(&mut self, st: &Stage, x: usize, ptr: u16) -> bool {
        match ptr {
            0x8E31 => self.ai_8e31(st, x),
            0x9F85 => {
                // the first call only plays the sound and installs the real routine
                self.snd(0x28, 0x9F8A);
                self.o[x].ptr = 0x9F94;
                self.ai_9f94(x);
            }
            0x9F94 => self.ai_9f94(x),
            0x8CA4 => self.ai_8ca4(st, x),
            0x8F25 => self.ai_8f25(st, x),
            0x8BA3 => self.ai_8ba3(x),
            0x8ED4 => self.ai_8ed4(st, x),
            0x88DE => {
                // the type-4 AI starts the routine of the selected Rush item (table `$8B25`/`$8B28`)
                self.o[x].ptr = self.ai_byte(0x8B25 + self.weapon as u16) as u16 | (self.ai_byte(0x8B28 + self.weapon as u16) as u16) << 8;
                let p = self.o[x].ptr;
                return self.run_weapons(st, x, p);
            }
            0x88F3 => self.rush_fall(st, x),
            0x891A => self.rush_fall2(st, x),
            0x894B => self.rush_fall3(st, x),
            0x8B42 => self.rush_mount(x),
            0x8B95 => {
                // the Marine follows (code_3FFD05 only redraws the palette); when Mega Man's boarding animation ends Rush disappears
                if self.p.anim == 0x29 {
                    self.despawn(x);
                }
            }
            0x8A86 => self.rush_jet(st, x),
            0x8996 => self.rush_land(x),
            0x89E3 => self.rush_wait(x),
            0x8A5D => self.rush_leave(x),
            0x8BFB => self.ai_8bfb(st, x),
            0x8FE7 => {
                // the skull barrier sticks to Mega Man
                let (xpg, xpx, ypg, ypx) = (self.p.xpg, self.p.xpx, self.p.ypg, self.p.ypx);
                let o = &mut self.o[x];
                o.xpg = xpg;
                o.xpx = xpx;
                o.ypg = ypg;
                o.ypx = ypx;
            }
            0x8F40 => self.ai_8f40(st, x),
            0x8E6F => self.ai_8e6f(st, x),
            0x8D9E => self.ai_8d9e(st, x),
            0x8DBC => self.ai_8dbc(st, x),
            0x8DD7 => self.ai_8dd7(st, x),
            _ => return false,
        }
        true
    }

    /// code_3C8F98 (the object part): a missile of weapon 7..=10 appears in slot 1.
    pub(super) fn spawn_missile(&mut self, weapon: u8, x: i32, y: i32, flip: bool) {
        let w = weapon as u16;
        self.set_parent(x, y, flip);
        let dir = if flip { 1 } else { 2 };
        self.o[1].dir = dir;
        let anim = self.rom.cpu_byte(0x3C, 0x3D, 0x9200 + w);
        self.child_at(0, 1, dir & 1, anim);
        let xvs = self.rom.cpu_byte(0x3C, 0x3D, 0x9204 + w);
        let xvp = self.rom.cpu_byte(0x3C, 0x3D, 0x9208 + w);
        let s0 = self.rom.cpu_byte(0x3C, 0x3D, 0x920C + w);
        let c = &mut self.o[1];
        c.ty = weapon;
        c.xvs = xvs;
        c.xvp = xvp;
        c.s[0] = s0;
        c.s[1] = (dir << 2) | 4;
        c.s[2] = 0x58;
        c.s[3] = 2;
        c.yvs = 0;
        c.yvp = 0;
        c.hitbox = 0;
        if weapon == 8 {
            c.flags |= 0x08;
        }
    }

    /// AI `$09` (`$3A8E31`): the missile flies straight until it hits a wall (it vanishes into an explosion), or after a delay until B is pressed again.
    fn ai_8e31(&mut self, st: &Stage, x: usize) {
        let hit = self.horizontal_collide(st, x, 0x0E);
        let on_breakable = hit && self.pt.iter().copied().max() == Some(9);
        self.shot_block_check(st, x);
        if !hit || on_breakable {
            if self.o[x].s[0] != 0 {
                self.o[x].s[0] -= 1;
                return;
            }
            if self.pressed & 0x40 == 0 {
                return;
            }
        }
        // $8E54: explode
        self.despawn(x);
        self.set_anim(x, 0x62);
        let o = &mut self.o[x];
        o.ty = 0x3B;
        o.s[0] = 0;
        o.s[1] = 0;
        o.hitbox = 3;
    }

    /// AI `$3B` (`$3A9F94`): the explosion of weapon 9 lays down a chain of spark objects (type `$80`, animation `$62`) at the offsets of table `$9FDF`.
    fn ai_9f94(&mut self, x: usize) {
        let mut e: i32 = 0;
        if self.o[x].s[0] != 0 {
            self.o[x].s[0] -= 1;
            if self.o[x].s[0] != 0 {
                return;
            }
            e = 1;
        }
        while e >= 0 {
            let Some(y) = self.find_slot() else { return };
            let o = &mut self.o[y];
            o.ty = 0x80;
            o.hitbox = 0;
            o.hp = 0;
            let idx = self.ai_byte(0x9FDF + self.o[x].s[1] as u16);
            self.child_at(x, y, idx, 0x62);
            self.o[x].s[1] += 1;
            e -= 1;
        }
        self.o[x].s[0] = 2;
        if self.o[x].s[1] == 9 {
            self.despawn(x);
        }
    }
}

impl World {
    /// AI `$0A` (`$3A8E6F`): the dust ball (animation `$4A`) flies until its animation reaches frame 4, then bursts into four pieces (animations `$4B-$4E`) that fly diagonally.
    fn ai_8e6f(&mut self, st: &Stage, x: usize) {
        if self.o[x].aid == 0x4A && self.o[x].aframe != 0 {
            if self.o[x].aframe != 4 {
                return;
            }
            for y in (1..=4usize).rev() {
                self.kill_slot(y);
                let anim = self.ai_byte(0x8ECB + y as u16);
                self.child_here(x, y, anim);
                let dir = self.ai_byte(0x8ECF + y as u16);
                let c = &mut self.o[y];
                c.ty = 0x0A;
                c.dir = dir;
                c.xvs = 0x3E;
                c.yvs = 0x3E;
                c.xvp = 4;
                c.yvp = 4;
                c.hitbox = 0;
            }
        }
        self.o[x].atick = 0;
        self.move_h(x);
        self.move_v(x);
        self.shot_block_check(st, x);
        self.o[x].flags &= 0xBF;
        if self.o[x].ypg != 0 {
            self.despawn(x);
        }
    }
}

impl World {
    /// `$3A8D9E` (type `$08`): the ring flies straight; after a pause it turns toward Mega Man.
    fn ai_8d9e(&mut self, st: &Stage, x: usize) {
        self.move_h(x);
        self.move_v(x);
        self.shot_block_check(st, x);
        self.o[x].s[0] = self.o[x].s[0].wrapping_sub(1);
        if self.o[x].s[0] == 0 {
            self.ring_pause(x);
        }
    }

    /// `$3A8DAC`: wait 8 frames, then aim (`$8DBC`).
    fn ring_pause(&mut self, x: usize) {
        self.o[x].s[0] = 8;
        self.o[x].ptr = 0x8DBC;
    }

    /// `$3A8DBC`: after the pause the ring aims at Mega Man and flies on (`$8DD7` steers it for 22 frames at a time).
    fn ai_8dbc(&mut self, st: &Stage, x: usize) {
        self.o[x].s[0] = self.o[x].s[0].wrapping_sub(1);
        if self.o[x].s[0] != 0 {
            return;
        }
        self.o[x].ptr = 0x8DD7;
        let h = self.heading_to_player(x);
        self.o[x].s[0] = 0x16;
        self.ring_fly(st, x, h, true);
    }

    /// `$3A8DD7`: re-aim toward Mega Man unless the heading would reverse an axis of the current one.
    fn ai_8dd7(&mut self, st: &Stage, x: usize) {
        let h = self.heading_to_player(x);
        let s1 = self.o[x].s[1];
        let (a, b) = (s1 & 3, s1 & 0x0C);
        let mut steer = true;
        if a != 0 && h & 3 != 0 {
            if h & 3 != a {
                steer = false;
            } else {
                self.o[x].s[1] = (h & 3) | b;
            }
        }
        if steer && b != 0 && h & 0x0C != 0 {
            if h & 0x0C != b {
                steer = false;
            } else {
                self.o[x].s[1] = (h & 0x0C) | a;
            }
        }
        self.ring_fly(st, x, h, steer);
    }

    /// `$3A8E10` / `$3A8E17`: set the velocity from the heading (when steering), move, and come back into Mega Man's hands.
    fn ring_fly(&mut self, st: &Stage, x: usize, h: u8, steer: bool) {
        if steer {
            self.heading_speed(x, h, 0x10);
        }
        self.move_h(x);
        self.move_v(x);
        self.shot_block_check(st, x);
        self.o[x].s[0] = self.o[x].s[0].wrapping_sub(1);
        if self.o[x].s[0] == 0 {
            self.ring_pause(x);
        }
        if self.touches_player(x) {
            self.despawn(x);
        }
    }
}

impl World {
    /// `$3A8CA4` (type `$07`): a homing missile. Every other frame it looks for the nearest vulnerable enemy (by X distance) and turns toward it.
    fn ai_8ca4(&mut self, st: &Stage, x: usize) {
        let counter = (self.o[x].s[3] as u16) << 8 | self.o[x].s[2] as u16;
        if counter != 0 {
            let c = counter - 1;
            self.o[x].s[2] = c as u8;
            self.o[x].s[3] = (c >> 8) as u8;
            self.o[x].s[0] = self.o[x].s[0].wrapping_sub(1);
            if self.o[x].s[0] == 0 {
                self.missile_seek(x);
            }
        }
        self.move_h(x);
        self.move_v(x);
        self.shot_block_check(st, x);
        if self.o[x].ypg != 0 {
            self.despawn(x);
        }
    }

    /// `$3A8CC5`: pick the target and steer one step (the heading `$0480` moves by one per search, or on through a half-finished turn).
    fn missile_seek(&mut self, x: usize) {
        self.f6 = 0x20 | self.weapon;
        self.o[x].s[0] = 2;
        let mut best = 0xFFu8;
        let mut target = 0usize;
        for y in (8..=0x17usize).rev() {
            let e = self.o[y];
            if e.ty == 0 || e.flags & 0x80 == 0 || e.hitbox & 0x40 == 0 {
                continue;
            }
            if self.rom.cpu_byte(0x3A, 0x20 | self.weapon as usize, 0xB700 + e.ty as u16) == 0 {
                continue;
            }
            let me = self.o[x];
            let mut d = e.xpx.wrapping_sub(me.xpx);
            let page_ok = (e.xpg as u16) << 8 | e.xpx as u16 >= (me.xpg as u16) << 8 | me.xpx as u16;
            if !page_ok {
                d = (!d).wrapping_add(1);
            }
            if d < best {
                best = d;
                target = y;
            }
        }
        if target == 0 {
            return;
        }
        let e = self.o[target];
        let want = self.heading_between(x, e.xpg, e.xpx, e.ypx);
        if want == self.o[x].s[1] {
            return;
        }
        let cur = self.o[x].s[1];
        if cur & 3 != 0 {
            self.o[x].s[1] = cur.wrapping_add(self.o[x].s[4]);
        } else {
            let v = (cur.wrapping_add(8) & 0x0F).wrapping_sub(want) & 0x0F;
            if v == 8 {
                // already pointing there: the heading is kept
            } else if v > 8 {
                self.o[x].s[4] = 0xFF;
                self.o[x].s[1] = cur.wrapping_sub(1);
            } else {
                self.o[x].s[4] = 1;
                self.o[x].s[1] = cur.wrapping_add(1);
            }
        }
        let h = self.o[x].s[1] & 0x0F;
        self.o[x].s[1] = h;
        let anim = self.ai_byte(0x8D8E + h as u16);
        if anim != self.o[x].aid {
            self.set_anim(x, anim);
        }
        self.heading_speed(x, h, 8);
    }
}

impl World {
    /// code_3C9045 (the object part): the held fireball of weapon 11 (type `$0D`, animation `$4F`) appears above Mega Man in `slot`.
    pub(super) fn spawn_held(&mut self, slot: usize, x: i32, y: i32, flip: bool) {
        self.set_parent(x, y, flip);
        self.child_at(0, slot, 0x41, 0x4F);
        let c = &mut self.o[slot];
        c.ty = 0x0D;
        c.hitbox = 5;
        c.s[0] = 0;
    }

    /// code_3C90C2 (the object part): the Pharaoh shot flies off (type `$0C`, animation `$47`/`$48`/`$49` = small/medium/big).
    pub(super) fn spawn_pharaoh(&mut self, slot: usize, anim: u8, x: i32, y: i32, flip: bool, held: u8) {
        self.set_parent(x, y, flip);
        let mut dir = if flip { 1 } else { 2 };
        self.o[slot].dir = dir;
        self.child_at(0, slot, dir & 1, anim);
        dir |= held & 0x0C;
        let k = (dir >> 2) as u16;
        let (xvs, xvp, yvs, yvp) = (
            self.rom.cpu_byte(0x3C, 0x3D, 0x9217 + k),
            self.rom.cpu_byte(0x3C, 0x3D, 0x921A + k),
            self.rom.cpu_byte(0x3C, 0x3D, 0x921D + k),
            self.rom.cpu_byte(0x3C, 0x3D, 0x9220 + k),
        );
        let c = &mut self.o[slot];
        c.ty = 0x0C;
        c.dir = dir;
        c.xvs = xvs;
        c.xvp = xvp;
        c.yvs = yvs;
        c.yvp = yvp;
        c.hitbox = 4;
    }

    /// `$3A8F25` (type `$0C`): the shot flies straight; its hitbox size comes from its animation.
    fn ai_8f25(&mut self, st: &Stage, x: usize) {
        self.o[x].hitbox = self.ai_byte(0x8F9C + self.o[x].aid as u16);
        self.move_h(x);
        self.move_v(x);
        self.shot_block_check(st, x);
        if self.o[x].ypg != 0 {
            self.despawn(x);
        }
    }

    /// `$3A8F40` (type `$0D`): the charging fireball floats 28 px above Mega Man and trails behind him when he walks.
    fn ai_8f40(&mut self, st: &Stage, x: usize) {
        if self.o[x].aframe == 0x0E {
            self.set_anim(x, 0x49);
        }
        let (py, b) = self.p.ypx.overflowing_sub(0x1C);
        self.o[x].ypx = py;
        self.o[x].ypg = self.p.ypg.wrapping_sub(b as u8);
        let pl = (self.p.xpg as u16) << 8 | self.p.xpx as u16;
        if self.o[x].s[0] == 0 {
            if self.dist_pure(x).0 < 8 {
                return;
            }
            self.face_player(x);
            self.o[x].s[1] = self.p.xpx;
            self.o[x].s[2] = self.p.xpg;
            self.o[x].s[0] += 1;
            return;
        }
        let target = (self.o[x].s[2] as u16) << 8 | self.o[x].s[1] as u16;
        if target != pl {
            let right = self.o[x].dir & 1 != 0;
            let c = target >= pl;
            if (right && !c) || (!right && c) {
                self.o[x].s[1] = self.p.xpx;
                self.o[x].s[2] = self.p.xpg;
            }
        }
        self.o[x].xvs = 0x4C; // $37/$38: his running speed
        self.o[x].xvp = 0x01;
        self.move_h(x);
        self.shot_block_check(st, x);
        let target = (self.o[x].s[2] as u16) << 8 | self.o[x].s[1] as u16;
        let me = (self.o[x].xpg as u16) << 8 | self.o[x].xpx as u16;
        let c = target >= me;
        let right = self.o[x].dir & 1 != 0;
        if target == me || (right && !c) || (!right && c) {
            self.o[x].s[0] = 0; // arrived (or overshot)
        }
    }
}

impl World {
    /// `$3A8BA3` (type `$05`): the rain cloud rises (faster after its second animation), leaves the screen and 11 frames later the rain starts.
    fn ai_8ba3(&mut self, x: usize) {
        if self.o[x].aid == 0x36 {
            match self.o[x].aframe {
                5 => return,
                6 => {
                    self.set_anim(x, 0x37);
                    self.o[x].yvp = 8;
                }
                _ => {}
            }
        }
        self.up(x);
        if self.o[x].ypg == 0 {
            return;
        }
        if self.o[x].flags & 4 == 0 {
            self.o[x].ypg = 0;
            self.o[x].flags |= 4;
            self.o[x].s[0] = 0x0B;
        }
        self.o[x].s[0] = self.o[x].s[0].wrapping_sub(1);
        if self.o[x].s[0] != 0 {
            return;
        }
        self.despawn(x);
        if self.w13a == 0 {
            self.w13b = 0;
            self.w13a = 0x3C;
            self.w145 = x as u8;
        }
    }
}

impl World {
    /// code_3FF400: move vertically by the direction flags with a tile probe (`id` for down, `id + 1` for up); true when the object hit the floor / ceiling.
    fn vert_dir_collide(&mut self, st: &Stage, x: usize, id: usize) -> bool {
        let hit = match self.o[x].dir & 0x0C {
            0 => false,
            d if d & 4 != 0 => self.down_collide(st, x, id),
            _ => self.up_collide(st, x, id + 1),
        };
        self.carry = hit;
        hit
    }

    /// `$3A8BFB` (type `$06`): the spring platform. Mega Man landing on it compresses it (animation `$3F`); it then shoots up and disappears when its time runs out or it hits something.
    fn ai_8bfb(&mut self, st: &Stage, x: usize) {
        if self.vert_dir_collide(st, x, 0x0C) {
            self.coil_end(x);
            return;
        }
        if self.o[x].s[1] != 0 {
            self.o[x].s[1] -= 1;
            if self.o[x].s[1] == 0 {
                let o = &mut self.o[x];
                o.dir = 8;
                o.yvs = 0x33;
                o.yvp = 0;
            }
        }
        if self.o[x].s[0] < 0x28 {
            self.o[x].t5b8 = self.o[x].s[0];
        }
        self.o[x].s[0] = self.o[x].s[0].wrapping_sub(1);
        if self.o[x].s[0] == 0 {
            self.coil_end(x);
            return;
        }
        if self.o[x].aid == 0x3F {
            let hb = self.o[x].hitbox;
            self.o[x].hitbox = 0x0D;
            let touching = self.touches_player(x);
            self.o[x].hitbox = hb;
            if touching {
                return;
            }
            self.set_anim(x, 0x3E);
            let o = &mut self.o[x];
            o.hitbox = 0x0D;
            o.yvs = 0x33;
            o.yvp = 0;
            return;
        }
        if !self.touches_player(x) {
            return;
        }
        if self.o[x].ypx < self.p.ypx || self.o[x].ypx.wrapping_sub(self.p.ypx) < 0x17 {
            return;
        }
        if !self.p.falling {
            return;
        }
        self.player_ops.push(PlayerOp::SetY(self.p.ypx.wrapping_add(3)));
        let o = &mut self.o[x];
        o.s[1] = 8;
        o.yvs = 0x80;
        o.yvp = 0;
        o.dir = 4;
        o.hitbox = 0x1E;
        self.set_anim(x, 0x3F);
    }

    /// `$3A8C02`: the platform turns into a puff (type 1, animation `$35`).
    fn coil_end(&mut self, x: usize) {
        self.despawn(x);
        self.o[x].ty = 1;
        self.set_anim(x, 0x35);
    }
}

impl World {
    /// code_3FF2A7: move by the vertical speed (down when it is negative, else up), then apply gravity.
    fn gravity_move(&mut self, x: usize) {
        if self.o[x].yvp & 0x80 != 0 {
            self.fall_move(x);
        } else {
            self.up(x);
        }
        self.gravity_step(x);
    }

    /// `$3A88F3` (Rush Coil, falling): Rush drops from the top of the screen; once he is within 32 px above Mega Man's height he checks for the ground.
    fn rush_fall(&mut self, st: &Stage, x: usize) {
        self.o[x].atick = 0;
        let (types, _) = self.vprobe(st, x, 0x15);
        self.pt = types.clone();
        let solid = types.iter().any(|t| t & 1 != 0);
        if solid || self.p.ypx.wrapping_sub(0x20) >= self.o[x].ypx {
            self.gravity_move(x);
            return;
        }
        if self.vertical_collide(st, x, 0) {
            self.o[x].ptr = 0x8996; // landed
        }
    }

    /// `$3A8996`: once the arrival animation reaches frame 4 Rush turns into his item form.
    fn rush_land(&mut self, x: usize) {
        if self.o[x].aframe != 4 {
            return;
        }
        let w = self.weapon as u16;
        let anim = self.ai_byte(0x8B1C + w);
        self.set_anim(x, anim);
        let hb = self.ai_byte(0x8B1F + w);
        let fl = self.ai_byte(0x8B22 + w);
        let flip = self.p.flip;
        let o = &mut self.o[x];
        o.hitbox = hb;
        o.flags |= fl;
        o.ptr = 0x89E3;
        o.s[0] = 0x2C;
        o.s[1] = 1;
        o.dir = if flip { 1 } else { 2 };
        o.flags |= if flip { 0x40 } else { 0 };
        o.atick = 0;
    }

    /// `$3A89E3`: Rush waits (a 16-bit counter `$0468/$0480`); when Mega Man stands on him the coil launches him.
    fn rush_wait(&mut self, x: usize) {
        let n = ((self.o[x].s[1] as u16) << 8 | self.o[x].s[0] as u16).wrapping_sub(1);
        self.o[x].s[0] = n as u8;
        self.o[x].s[1] = (n >> 8) as u8;
        if n == 0 {
            self.rush_go(x);
            return;
        }
        self.o[x].atick = 0;
        self.o[x].ypx = self.o[x].ypx.wrapping_sub(1);
        let touching = self.touches_player(x);
        self.o[x].ypx = self.o[x].ypx.wrapping_add(1);
        if !touching {
            return;
        }
        let w = self.weapon as u16;
        if w != 3 && self.o[x].aframe | self.p.state != 0 {
            return;
        }
        let next = (self.ai_byte(0x8B2B + w), self.ai_byte(0x8B2E + w));
        if next.1 != 0 {
            // Jet and Marine: Mega Man got on, the item routine takes over
            self.o[x].ptr = next.0 as u16 | (next.1 as u16) << 8;
            self.o[x].s[3] = 0x3C;
            return;
        }
        // the coil: Mega Man's vertical speed becomes 6.93, the bounce flag is set, the energy is paid
        self.o[x].s[1] = 0;
        self.o[x].s[0] = 0x1E;
        self.o[x].aframe += 1;
        self.player_ops.push(PlayerOp::Bounce);
        self.o[x].flags &= 0xFE;
    }

    /// `$3A8973`: Rush leaves (animation `$32`, he rises off the screen).
    fn rush_go(&mut self, x: usize) {
        self.set_anim(x, 0x32);
        let o = &mut self.o[x];
        o.ptr = 0x8A5D;
        o.hitbox = 0;
        o.yvs = 0;
        o.yvp = 0;
        o.flags &= 0xFC;
    }

    /// `$3A8A5D`: from animation frame 4 on Rush accelerates upward and disappears above the screen.
    fn rush_leave(&mut self, x: usize) {
        if self.o[x].aframe != 4 {
            return;
        }
        self.o[x].atick = 0;
        let v = ((self.o[x].yvp as u16) << 8 | self.o[x].yvs as u16).wrapping_add(0x40);
        self.o[x].yvs = v as u8;
        self.o[x].yvp = (v >> 8) as u8;
        self.up(x);
        if self.o[x].ypg != 0 {
            self.despawn(x);
        }
    }
}

impl World {
    /// `$3A891A` (Rush Jet, falling): Rush falls and stays in the air unless the tile under him is water, where he leaves again.
    fn rush_fall2(&mut self, st: &Stage, x: usize) {
        self.o[x].atick = 0;
        self.gravity_move(x);
        let below = self.p.ypx >= self.o[x].ypx;
        if below && self.p.ypx.wrapping_sub(self.o[x].ypx) >= 0x0C {
            return;
        }
        let (types, _) = self.vprobe(st, x, 0x15);
        self.pt = types.clone();
        if types.iter().any(|t| t & 1 != 0) {
            return;
        }
        if types.iter().copied().max() == Some(6) {
            self.rush_go(x); // water
        } else {
            self.o[x].ptr = 0x8996;
        }
    }

    /// `$3A8A86` (Rush Jet): Rush flies with the control pad (he keeps his horizontal direction, the pad steers; Mega Man rides on top) and costs energy every 60 frames.
    fn rush_jet(&mut self, st: &Stage, x: usize) {
        let mut y = 0usize;
        self.o[x].dir &= 3;
        let lr = self.held & 3;
        if lr != 0 && lr != self.o[x].dir {
            y = 2;
        }
        let ud = self.held & 0x0C;
        if ud != 0 {
            self.o[x].dir |= ud;
            y += 1;
        }
        let (xvs, xvp, yvs, yvp) = (
            self.ai_byte(0x8B32 + y as u16),
            self.ai_byte(0x8B36 + y as u16),
            self.ai_byte(0x8B3A + y as u16),
            self.ai_byte(0x8B3E + y as u16),
        );
        let o = &mut self.o[x];
        o.xvs = xvs;
        o.xvp = xvp;
        o.yvs = yvs;
        o.yvp = yvp;
        o.ypx = o.ypx.wrapping_sub(1);
        let touching = self.touches_player(x);
        self.o[x].ypx = self.o[x].ypx.wrapping_add(1);
        if touching && self.o[x].dir & 0x0C != 0 {
            let oy = self.o[x].ypx;
            let go = if self.o[x].dir & 8 != 0 { oy >= 0x1C && oy >= self.p.ypx && oy - self.p.ypx >= 0x0E } else { oy < 0xE8 };
            if go {
                self.vert_dir_collide(st, x, 0x18);
            }
        }
        if self.horizontal_collide(st, x, 0x24) {
            self.rush_go(x);
            return;
        }
        self.o[x].s[3] = self.o[x].s[3].wrapping_sub(1);
        if self.o[x].s[3] == 0 {
            self.player_ops.push(PlayerOp::Pay);
            let cost = [0u8, 2, 1, 1][self.weapon as usize & 3];
            self.wen_cur = 0x80 | (self.wen_cur & 0x7F).saturating_sub(cost);
            if self.wen_cur & 0x7F == 0 {
                self.rush_go(x);
                return;
            }
            self.o[x].s[3] = 0x3C;
        }
        if touching {
            self.carry_player(x);
        }
    }
}

impl World {
    /// `$3A8ED4` (type `$0B`): the wire rises from Mega Man until it hits the ceiling (animation `$3D`) or leaves the screen, then it is pulled back down to him; it disappears when he lets go.
    fn ai_8ed4(&mut self, st: &Stage, x: usize) {
        if self.p.state != 5 {
            self.despawn(x);
            return;
        }
        self.o[4].s[0] = self.p.ypx.wrapping_sub(0x0E);
        if self.vert_dir_collide(st, x, 0x23) {
            self.set_anim(x, 0x3D);
            return;
        }
        if self.frame92 & 7 == 0 {
            self.snd(0x32, 0x8EFA); // code_3A8EF7
        }
        if self.o[x].dir & 8 != 0 {
            if self.o[x].ypg != 0 {
                let o = &mut self.o[x];
                o.ypx = 0;
                o.ypg = 0;
                o.dir = 4;
            }
            return;
        }
        if self.o[x].ypx < self.o[x].s[0] {
            return;
        }
        self.player_cmd = Some(0); // `$30 = 0`: the pulled-in wire releases him
        self.despawn(x);
    }
}

impl World {
    /// `$3A894B` (Rush Marine, falling): like the Jet but it only comes to rest over water.
    fn rush_fall3(&mut self, st: &Stage, x: usize) {
        self.o[x].atick = 0;
        self.gravity_move(x);
        let below = self.p.ypx >= self.o[x].ypx;
        if below && self.p.ypx.wrapping_sub(self.o[x].ypx) >= 0x0C {
            return;
        }
        let (types, _) = self.vprobe(st, x, 0x15);
        self.pt = types.clone();
        if types.iter().any(|t| t & 1 != 0) {
            return;
        }
        if types.iter().copied().max() == Some(6) {
            self.o[x].ptr = 0x8996;
        }
    }

    /// `$3A8B42`: Mega Man touches the waiting Rush Marine and boards it.
    fn rush_mount(&mut self, x: usize) {
        if self.p.state >= 2 || !self.touches_player(x) {
            return;
        }
        let o = self.o[x];
        self.player_ops.push(PlayerOp::Mount { xpg: o.xpg, xpx: o.xpx, ypg: o.ypg, ypx: o.ypx });
        self.o[x].flags |= 4;
        self.o[x].ptr = 0x8B95;
    }
}
