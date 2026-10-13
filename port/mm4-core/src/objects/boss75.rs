//! The stage-4 boss (AI `$75`, code in bank `$38`: `$AA6A`-`$AB63`) and its boomerang child `$76` (`$AC6C`, `$AC87`, `$ACA6`).
use super::*;

impl World {
    /// Dispatch for the stage-4 boss states; returns false for an address that is not one of them.
    pub(super) fn run_boss75(&mut self, st: &Stage, x: usize, ptr: u16) -> bool {
        match ptr {
            0xAA6A => self.b75_idle(x),
            0xAABB => self.b75_air(st, x),
            0xAB3D => self.b75_land(st, x),
            0xAB63 => self.b75_walk(st, x),
            0xAC6C => {
                self.move_h(x);
                self.move_v(x);
                self.o[x].s[0] = self.o[x].s[0].wrapping_sub(1);
                if self.o[x].s[0] == 0 {
                    self.b76_wait_start(x);
                }
            }
            0xAC87 => self.b76_wait(x),
            0xACA6 => self.b76_steer(x),
            _ => return false,
        }
        true
    }

    /// code_38ABEC: carry set when two or more objects of type `ty` are alive.
    fn count_type(&self, ty: u8) -> bool {
        (8..=0x17usize).filter(|&s| self.o[s].ty != 0 && self.o[s].ty == ty).count() >= 2
    }

    /// `$AA6A`: standing: after the pause (`$0468`) jump (animation `0C`); otherwise throw a boomerang (animation `11`, frame 2 releases it).
    fn b75_idle(&mut self, x: usize) {
        if self.o[x].s[0] != 0 {
            self.o[x].s[0] -= 1;
            if self.o[x].s[0] != 0 {
                return;
            }
            self.o[x].yvs = 0x52;
            self.o[x].yvp = 6;
            self.o[x].ptr = 0xAABB;
            self.set_anim(x, 0x0C);
            return;
        }
        if self.o[x].aid != 0x11 {
            if self.count_type(0x76) {
                self.o[x].s[0] = 0x1E;
                return;
            }
            self.set_anim(x, 0x11);
            return;
        }
        match self.o[x].aframe {
            2 => self.b75_throw(x, true),
            4 => {
                self.set_anim(x, 9);
                self.o[x].s[0] = 0x1E;
            }
            _ => {}
        }
    }

    /// `$AC07` / `$AC0D`: (optionally face Mega Man,) advance the animation and throw a boomerang (`$76`) that flies out and comes back.
    fn b75_throw(&mut self, x: usize, face: bool) {
        if face {
            self.face_player(x);
            self.face_flag(x);
        }
        self.o[x].aframe = self.o[x].aframe.wrapping_add(1);
        let Some(y) = self.find_slot() else { return };
        let dir = self.o[x].dir;
        self.child_at(x, y, (dir & 1) + 0x2B, 0x10);
        let c = &mut self.o[y];
        c.flags |= 0x08;
        c.ty = 0x76;
        c.hitbox = 0x80;
        c.xvs = 0;
        c.xvp = 6;
        c.dir = dir;
        c.s[1] = dir;
        c.s[2] = x as u8;
        let (d, _) = self.dist_pure(x);
        let q = d.wrapping_add(0x20) / 6 + 1; // code_3FFC14 divides
        let c = &mut self.o[y];
        c.s[0] = q;
        c.s[3] = q.wrapping_shl(1);
    }

    /// `$AABB`: in the air (animation `0C` / `0D`); animation frame 5 (after landing) turns toward Mega Man and starts walking.
    fn b75_air(&mut self, st: &Stage, x: usize) {
        let a = self.o[x].aframe;
        if a == 3 || (a == 2 && self.o[x].yvp & 0x80 == 0) {
            self.o[x].atick = 0;
            if self.vertical_collide(st, x, 0x0C) {
                self.o[x].aframe += 1;
            }
            return;
        }
        match a {
            2 => {
                if !self.count_type(0x76) {
                    // carry clear (fewer than two): the animation does not advance
                } else {
                    self.o[x].aframe += 1;
                }
                self.o[x].ptr = 0xAB3D;
                self.set_anim(x, 0x0D);
            }
            1 => {
                self.o[x].aframe += 1;
                self.o[x].ypx = self.o[x].ypx.wrapping_sub(8);
            }
            5 => {
                self.face_player(x);
                self.face_flag(x);
                self.o[x].ptr = 0xAB63;
                let (d, _) = self.dist_pure(x);
                let v = d.wrapping_add(0x20);
                self.o[x].s[1] = v >> 1;
                let carry = v & 1; // LSR leaves bit 0 in the carry; the SBC below uses it
                self.o[x].s[3] = 0;
                let r = self.rng[2].wrapping_sub(self.rng[1]).wrapping_sub(1 - carry);
                self.o[x].s[2] = r & 3;
                if self.o[x].s[2] == 0 {
                    self.o[x].s[2] = 1;
                }
                self.set_anim(x, 0x0A);
            }
            _ => {}
        }
    }

    /// `$AB3D`: coming down (animation `0D`); frame 2 throws a boomerang, frame 4 goes back to the jump state.
    fn b75_land(&mut self, st: &Stage, x: usize) {
        self.vertical_collide(st, x, 0x0C);
        match self.o[x].aframe {
            2 => self.b75_throw(x, true),
            4 => {
                self.o[x].ptr = 0xAABB;
                self.o[x].aid = self.o[x].aid.wrapping_sub(1);
                self.o[x].aframe = 3;
            }
            _ => {}
        }
    }

    /// `$AB63`: walking toward Mega Man; he reacts to the B button (a shot) and ends the walk with a throw.
    fn b75_walk(&mut self, st: &Stage, x: usize) {
        if self.o[x].s[4] != 0 {
            self.face_player(x);
            self.face_flag(x);
            self.o[x].s[4] -= 1;
            if self.o[x].s[4] == 0 {
                self.o[x].ptr = 0xAA6A;
            }
            return;
        }
        let hit = self.horizontal_collide(st, x, 0x14);
        let walking = self.o[x].aid == 0x0A;
        if hit || walking {
            if !walking {
                return self.b75_after(x);
            }
            if hit {
                return self.b75_stop(x);
            }
            self.o[x].s[1] = self.o[x].s[1].wrapping_sub(1);
            if self.o[x].s[1] == 0 {
                return self.b75_stop(x);
            }
            if self.pressed & 0x40 == 0 {
                return;
            }
            self.o[x].s[3] = self.o[x].s[3].wrapping_add(1);
            if self.o[x].s[3] != self.o[x].s[2] {
                return self.b75_after(x);
            }
            if self.count_type(0x76) {
                return;
            }
            let left_of_player = self.o[x].xpx < self.p.xpx;
            if self.o[x].dir & 1 != 0 {
                if left_of_player {
                    self.o[x].aid = self.o[x].aid.wrapping_add(1);
                }
            } else if !left_of_player {
                self.o[x].aid = self.o[x].aid.wrapping_add(1);
            }
            return;
        }
        self.b75_after(x);
    }

    /// `$AB9A`: stop for 32 frames (animation `09`).
    fn b75_stop(&mut self, x: usize) {
        self.o[x].s[4] = 0x20;
        self.set_anim(x, 9);
    }

    /// `$ABD8`: animation frame 6 throws a boomerang, frame 8 goes back to the walking pose.
    fn b75_after(&mut self, x: usize) {
        match self.o[x].aframe {
            6 => self.b75_throw(x, false),
            8 => {
                self.o[x].aid = self.o[x].aid.wrapping_sub(1);
                self.o[x].aframe = 3;
            }
            _ => {}
        }
    }

    /// `$AC77`: wait 8 frames before steering.
    fn b76_wait_start(&mut self, x: usize) {
        self.o[x].s[0] = 8;
        self.o[x].ptr = 0xAC87;
    }

    /// Heading (0..15) from boomerang `x` to its owner (code_3FF70F).
    fn b76_heading(&self, x: usize) -> u8 {
        let t = self.o[self.o[x].s[2] as usize];
        self.heading_between(x, t.xpg, t.xpx, t.ypx)
    }

    /// `$AC87`: after the wait the boomerang starts steering toward its owner.
    fn b76_wait(&mut self, x: usize) {
        self.o[x].s[0] = self.o[x].s[0].wrapping_sub(1);
        if self.o[x].s[0] != 0 {
            return;
        }
        self.o[x].ptr = 0xACA6;
        let h = self.b76_heading(x);
        self.o[x].s[0] = self.o[x].s[3];
        if self.o[x].s[0] != 0 {
            self.heading_speed(x, h, 0x10);
            self.b76_move(x);
            return;
        }
        self.b76_steer_with(x, h);
    }

    /// `$ACA6`: pick the flight direction toward the owner (axis by axis), move, and vanish when it reaches the owner.
    fn b76_steer(&mut self, x: usize) {
        let h = self.b76_heading(x);
        self.b76_steer_with(x, h);
    }

    fn b76_steer_with(&mut self, x: usize, h: u8) {
        let s1 = self.o[x].s[1];
        let (a, b) = (s1 & 3, s1 & 0x0C);
        if a != 0 && h & 3 != 0 {
            if h & 3 != a {
                return self.b76_move(x);
            }
            self.o[x].s[1] = (h & 3) | b;
        }
        if b != 0 && h & 0x0C != 0 {
            if h & 0x0C != b {
                return self.b76_move(x);
            }
            self.o[x].s[1] = (h & 0x0C) | a;
        }
        self.heading_speed(x, h, 0x10);
        self.b76_move(x);
    }

    /// `$ACE9`: move, count down the steering interval, and disappear once it is within 20 px of its owner.
    fn b76_move(&mut self, x: usize) {
        self.move_h(x);
        self.move_v(x);
        self.o[x].s[0] = self.o[x].s[0].wrapping_sub(1);
        if self.o[x].s[0] == 0 {
            self.b76_wait_start(x);
        }
        let t = self.o[self.o[x].s[2] as usize];
        let o = self.o[x];
        let me = (o.xpg as u16) << 8 | o.xpx as u16;
        let you = (t.xpg as u16) << 8 | t.xpx as u16;
        let dx = if me >= you { o.xpx.wrapping_sub(t.xpx) } else { (!o.xpx.wrapping_sub(t.xpx)).wrapping_add(1) };
        if dx >= 0x14 {
            return;
        }
        let dy = if o.ypx >= t.ypx { o.ypx - t.ypx } else { (!(o.ypx.wrapping_sub(t.ypx))).wrapping_add(1) };
        if dy >= 0x14 {
            return;
        }
        self.despawn(x);
    }
}
