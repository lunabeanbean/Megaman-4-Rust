//! Stage 1 enemies (bank `$3B` code `$96A2`-`$A9xx`, bank `$3D` `$A602`...).
use super::*;

impl World {
    /// Dispatch for the stage-1 enemy states; returns false for an address that is not one of them.
    pub(super) fn run_stage1(&mut self, st: &Stage, x: usize, ptr: u16) -> bool {
        match ptr {
            0x96A2 => self.ai_96a2(x),
            0x9762 => self.move_h(x),
            0x9766 => self.ai_9766(x),
            0x97A5 => {
                self.move_h(x);
                self.move_v(x);
            }
            0x9D6C => self.ai_9d6c(st, x),
            0x9DAA => self.ai_9daa(x),
            0x9DC9 => self.ai_9dc9(st, x),
            0x9E3C => self.move_h(x),
            0x9A76 => {
                self.face_player(x);
                self.face_flag(x);
                let c = self.carry;
                self.ai_9a7c(st, x, c);
            }
            0x9AAA => self.ai_9aaa(st, x),
            0xA9A7 => self.ai_a9a7(st, x),
            0xAA11 => self.ai_aa11(st, x),
            0xAA8D => self.ai_aa8d(st, x),
            0xA93F => self.ai_a93f(st, x),
            0xAE4F => self.ai_ae4f(st, x),
            0xAE94 => self.ai_ae94(st, x),
            0xAEFC => self.ai_aefc(st, x),
            0xAF19 => self.ai_af19(st, x),
            0xA602 => self.ai_a602(x),
            0xA67D => self.ai_a67d(x),
            0xA75F => self.ai_a75f(x),
            0xA7E5 => self.ai_a7e5(st, x),
            0xA846 => self.ai_a846(x),
            0x9FE8 => self.ai_9fe8(x),
            _ => return false,
        }
        true
    }

    /// `$3B973E`: vertical speed drops by `$10` per frame, the object moves by it (no page wrap) and then sideways.
    fn drop_motion(&mut self, x: usize) {
        let o = &mut self.o[x];
        let (s, b) = o.yvs.overflowing_sub(0x10);
        o.yvs = s;
        o.yvp = o.yvp.wrapping_sub(b as u8);
        let (s, c) = o.ysub.overflowing_add(o.yvs);
        o.ysub = s;
        o.ypx = o.ypx.wrapping_add(o.yvp).wrapping_add(c as u8);
        self.move_h(x);
    }

    /// AI `$18` (`$3B96A2`, list type `05`): hops with a falling speed for 0x42 frames and throws three bullets (type `$19`) on the way,
    /// then flies straight (3 px/frame in its direction).
    fn ai_96a2(&mut self, x: usize) {
        if self.o[x].s[0] == 0 {
            let o = &mut self.o[x];
            o.yvs = 0xF0;
            o.yvp = 1;
            o.s[0] = 0x42;
            o.s[2] = 0x10;
            o.s[3] = 3;
            o.s[4] = o.ypx;
        }
        self.o[x].s[0] = self.o[x].s[0].wrapping_sub(1);
        if self.o[x].s[0] == 0 {
            let o = &mut self.o[x];
            o.xvs = 0;
            o.xvp = 3;
            o.ptr = 0x9762;
            self.set_anim(x, 0x2F);
            return;
        }
        if self.o[x].s[3] != 0 {
            self.o[x].s[2] = self.o[x].s[2].wrapping_sub(1);
            if self.o[x].s[2] == 0 {
                self.o[x].s[3] -= 1;
                let Some(y) = self.find_slot() else { return };
                self.o[y].dir = self.o[x].dir;
                self.o[y].ty = 0x19;
                self.child_here(x, y, 0x31);
                self.o[y].ypx = self.o[x].s[4];
                let a = if self.o[x].dir & 1 != 0 { 1u8 } else { 0xFF };
                let (px, c) = a.overflowing_add(self.cam_now.lo);
                self.o[y].xpx = px;
                self.o[y].xpg = self.cam_now.hi.wrapping_add(c as u8);
                self.o[y].hitbox = 0xCE;
                self.o[y].hp = 1;
                let c = &mut self.o[y];
                c.xvs = 0x33;
                c.xvp = 1;
                c.yvs = 0xF0;
                c.yvp = 1;
                self.o[y].s[0] = self.o[x].s[0];
                self.o[x].s[2] = 0x10;
            }
        }
        self.drop_motion(x);
    }

    /// AI `$19` (`$3B9766`, the bullet of `$18`): follows the thrown arc for the parent's remaining time, then flies at Mega Man along one of
    /// 8 headings (tables at `$97AB`-`$97D3`).
    fn ai_9766(&mut self, x: usize) {
        self.o[x].s[0] = self.o[x].s[0].wrapping_sub(1);
        if self.o[x].s[0] != 0 {
            self.drop_motion(x);
            return;
        }
        let h = self.heading_to_player(x);
        self.o[x].dir = self.ai_byte(0x97C3 + h as u16);
        let a = self.ai_byte(0x97D3 + h as u16);
        self.set_anim(x, a);
        let k = (h & 7) as u16;
        let o = &mut self.o[x];
        o.xvs = self.rom.cpu_byte(0x3A, 0x3B, 0x97AB + k);
        o.xvp = self.rom.cpu_byte(0x3A, 0x3B, 0x97B7 + k);
        o.yvs = self.rom.cpu_byte(0x3A, 0x3B, 0x97AF + k);
        o.yvp = self.rom.cpu_byte(0x3A, 0x3B, 0x97BB + k);
        o.ptr = 0x97A5;
    }

    /// AI `$35` first state (`$3A9D6C`, list type `14`): waits on the ceiling until Mega Man is within 60 px, then drops (1.5 px/frame) until it lands.
    fn ai_9d6c(&mut self, st: &Stage, x: usize) {
        if self.o[x].flags & 4 != 0 {
            if self.dist_x(x) >= 0x3C {
                return;
            }
            let o = &mut self.o[x];
            o.flags &= !4;
            o.yvs = 0x80;
            o.yvp = 1;
            return;
        }
        if !self.down_collide(st, x, 0x0A) {
            return;
        }
        self.set_anim(x, 0x5D);
        self.face_player(x);
        self.face_flag(x);
        self.o[x].ptr = 0x9DAA;
    }

    /// `$3A9DAA`: the landing animation plays up to frame 7, then the walking pose starts.
    fn ai_9daa(&mut self, x: usize) {
        if self.o[x].aframe < 7 {
            return;
        }
        self.set_anim(x, 0x60);
        let o = &mut self.o[x];
        o.yvs = 0;
        o.yvp = 0;
        o.ptr = 0x9DC9;
    }

    /// `$3A9DC9`: animation `$60` throws a bullet (type `$36`) on frame 8 and starts the walk (`$5F`) on frame 9; the walk falls, goes sideways
    /// and turns around at walls.
    fn ai_9dc9(&mut self, st: &Stage, x: usize) {
        if self.o[x].aid == 0x5F {
            if !self.vertical_collide(st, x, 0x0E) {
                return;
            }
            if !self.horizontal_collide(st, x, 0x18) {
                return;
            }
            self.turn_around(x);
            return;
        }
        if self.o[x].atick != 2 {
            return;
        }
        match self.o[x].aframe {
            8 => {
                let Some(y) = self.find_slot() else { return };
                self.o[y].ty = 0x36;
                self.o[y].xvs = 0;
                self.o[y].xvp = 2;
                self.o[y].dir = self.o[x].dir;
                let idx = (self.o[x].dir & 1) + 0x11;
                self.child_at(x, y, idx, 0x61);
                self.o[y].hitbox = 0xDA;
                self.o[y].hp = 1;
            }
            9 => {
                self.o[x].ypx = self.o[x].ypx.wrapping_add(8);
                self.o[x].hitbox = 0xC0;
                self.set_anim(x, 0x5F);
            }
            _ => {}
        }
    }

    /// `$3A9A7C`: a hop is chosen (random vertical speed from the tables at `$9AFF`, 1 px/frame sideways, 32 frames of crouching); the code runs
    /// straight on into the main state. `carry` is the carry flag the original has at this point (it enters the random sum).
    fn ai_9a7c(&mut self, st: &Stage, x: usize, carry: bool) {
        let sum = self.rng[2] as u16 + self.rng[0] as u16 + carry as u16;
        self.rng[2] = sum as u8;
        let k = (sum as u8 & 3) as u16;
        let (yvs, yvp) = (self.ai_byte(0x9AFF + k), self.ai_byte(0x9B03 + k));
        let o = &mut self.o[x];
        o.yvs = yvs;
        o.yvp = yvp;
        o.xvs = 0;
        o.xvp = 1;
        o.s[0] = 0x20;
        o.ptr = 0x9AAA;
        self.ai_9aaa(st, x);
    }

    /// AI `$1F` main state (`$3A9AAA`, list type `11`, a frog): crouches (timer `$0468`), jumps (animation frame 2: moves by its speeds until it lands or
    /// hits a wall), and when the landing animation ends (frame 3, tick 9) turns around if it stood still and picks the next hop.
    fn ai_9aaa(&mut self, st: &Stage, x: usize) {
        if self.o[x].s[0] != 0 {
            self.o[x].atick = 0;
            self.o[x].s[0] -= 1;
            if self.o[x].s[0] == 0 {
                self.o[x].aframe = self.o[x].aframe.wrapping_add(1);
            }
            return;
        }
        match self.o[x].aframe {
            2 => {
                self.o[x].atick = 0;
                if self.vertical_collide(st, x, 0x0C) {
                    self.o[x].aframe += 1;
                    return;
                }
                if !self.horizontal_collide(st, x, 0x14) {
                    return;
                }
                self.o[x].xvs = 0;
                self.o[x].xvp = 0;
            }
            3 => {
                if self.o[x].atick != 9 {
                    return;
                }
                if self.o[x].xvp == 0 {
                    self.turn_around(x);
                }
                self.o[x].aframe = 0;
                self.ai_9a7c(st, x, true); // the CMP #9 left the carry set
            }
            _ => {}
        }
    }

    /// code_3FF0C1 like `up_collide`, but also returns the terrain type of every probe point (the original leaves them in `$45`-`$47`).
    fn up_collide_types(&mut self, st: &Stage, x: usize, id: usize) -> (bool, Vec<u8>) {
        self.up(x);
        let (types, y) = self.vprobe(st, x, id);
        let hit = types.iter().any(|t| t & 1 != 0);
        if hit {
            self.push_down(x, y);
        }
        (hit, types)
    }

    /// AI `$61` ceiling state (`$3BA9A7`, list type `47`): crawls along the ceiling (1 px up each frame keeps it glued; it turns at the end of
    /// the ceiling or at a wall; 20 frames of walking, then one standing frame) and drops when Mega Man is within 64 px.
    fn ai_a9a7(&mut self, st: &Stage, x: usize) {
        if self.dist_x(x) < 0x40 {
            self.o[x].ptr = 0xAA11;
            self.land_speed(x);
            self.o[x].dir = 4;
            self.set_anim(x, 0x91);
            return;
        }
        if self.o[x].s[0] == 0 {
            let o = &mut self.o[x];
            o.yvs = 0;
            o.yvp = 1;
            o.s[0] = 0x14;
            if o.dir & 3 == 0 {
                self.face_player(x);
            } else {
                self.o[x].dir = 0;
            }
            return;
        }
        self.o[x].s[0] -= 1;
        let (_, types) = self.up_collide_types(st, x, 0x22);
        let i = ((self.o[x].dir & 1) << 1) as usize;
        let ceiling = types.get(i).copied().unwrap_or(0) & 1 != 0;
        if !(ceiling && !self.horizontal_collide(st, x, 0x2C)) {
            self.o[x].dir ^= 3; // $A9E7: end of the ceiling or a wall: turn
        }
        self.o[x].flags &= 0xBF;
    }

    /// AI `$61` transitions (`$3BAA11`): the drop (dir 4) and the climb back (dir 8) play their animations; frame 1 shifts the body by 8 px,
    /// frame 2 waits for the floor / ceiling, frame 4 hands over to the floor state `$AA8D` or the ceiling state `$A9A7`.
    fn ai_aa11(&mut self, st: &Stage, x: usize) {
        match self.o[x].aframe {
            1 => {
                let y = self.o[x].ypx;
                self.o[x].ypx = if self.o[x].dir == 4 {
                    match y.wrapping_add(8) {
                        0 => y.wrapping_sub(8), // BNE not taken: falls into the other branch
                        v => v,
                    }
                } else {
                    y.wrapping_sub(8)
                };
                self.o[x].aframe += 1;
                self.aa11_wait(st, x);
            }
            2 => self.aa11_wait(st, x),
            4 => {
                if self.o[x].dir == 4 {
                    self.o[x].ptr = 0xAA8D;
                    self.face_player(x);
                    self.set_anim(x, 0x92);
                } else {
                    let o = &mut self.o[x];
                    o.ptr = 0xA9A7;
                    o.yvs = 0;
                    o.yvp = 1;
                    self.set_anim(x, 0x90);
                }
            }
            _ => {}
        }
    }

    /// `$3BAA6D`: hold the animation frame until the body touches the floor (dropping) or the ceiling (climbing).
    fn aa11_wait(&mut self, st: &Stage, x: usize) {
        self.o[x].atick = 0;
        let hit = if self.o[x].dir == 4 { self.vertical_collide(st, x, 0x21) } else { self.up_collide(st, x, 0x22) };
        if hit {
            self.o[x].aframe += 1;
        }
    }

    /// AI `$61` floor state (`$3BAA8D`): walks on the floor, turns at walls, and climbs back up when Mega Man is 64 px away or more.
    fn ai_aa8d(&mut self, st: &Stage, x: usize) {
        if self.dist_x(x) >= 0x40 {
            let o = &mut self.o[x];
            o.ptr = 0xAA11;
            o.yvs = 0;
            o.yvp = 2;
            o.dir = 8;
            self.set_anim(x, 0x93);
            return;
        }
        if !self.vertical_collide(st, x, 0x21) {
            return;
        }
        if self.horizontal_collide(st, x, 0x2C) {
            self.o[x].dir ^= 3;
        }
        self.o[x].flags &= 0xBF;
    }

    /// AI `$70` resting state (`$3BAE4F`, list type `4D`, a jumper at the bottom of the water): walks to Mega Man (turning at walls) and leaps
    /// when he is within 64 px.
    fn ai_ae4f(&mut self, st: &Stage, x: usize) {
        if self.horizontal_collide(st, x, 0x1C) {
            self.turn_around(x);
        }
        self.face_player(x);
        self.face_flag(x);
        if self.dist_x(x) >= 0x40 {
            return;
        }
        let o = &mut self.o[x];
        o.yvs = 0x52;
        o.yvp = 6;
        o.xvs = 0x33;
        o.xvp = 1;
        o.ptr = 0xAE94;
        o.s[0] = 0x60;
        o.s[1] = o.ypx;
        self.set_anim(x, 0x99);
    }

    /// `$3BAE94`: the leap. Moves with gravity; every time the terrain type under it changes (water surface) a splash (type `$80`, animation `$A4`)
    /// is created; after the splash at type `$60` it goes to the falling state, and once it is back below its start height the flag `$0498` is set.
    fn ai_ae94(&mut self, st: &Stage, x: usize) {
        self.move_gravity(x);
        self.move_h(x);
        if self.o[x].s[2] != 0 {
            return;
        }
        let (types, _) = self.vprobe(st, x, 2);
        let cur = (types.iter().fold(0u8, |a, t| a | t) << 4) & 0xE0;
        if cur == self.o[x].s[0] {
            if self.o[x].s[1] < self.o[x].ypx {
                self.o[x].s[2] += 1;
            }
            return;
        }
        self.o[x].s[0] = cur;
        let Some(y) = self.find_slot() else { return };
        self.o[y].ty = 0x80;
        self.o[y].hitbox = 0;
        self.child_here(x, y, 0xA4);
        self.o[y].ypx &= 0xF0;
        self.o[y].flags &= 0xBF;
        if cur == 0x60 {
            self.o[x].ptr = 0xAEFC;
            return;
        }
        self.o[y].ypx = self.o[y].ypx.wrapping_add(0x10);
    }

    /// `$3BAEFC`: falling back; when it is below its start height again it snaps to it, plays animation `$98` and runs on into `$AF19`.
    fn ai_aefc(&mut self, st: &Stage, x: usize) {
        self.move_gravity(x);
        if self.o[x].s[1] >= self.o[x].ypx {
            return;
        }
        self.o[x].ypx = self.o[x].s[1];
        self.set_anim(x, 0x98);
        self.o[x].ptr = 0xAF19;
        self.ai_af19(st, x);
    }

    /// `$3BAF19`: slides along the floor until a wall stops it, then rests again (2 px/frame when it sets off).
    fn ai_af19(&mut self, st: &Stage, x: usize) {
        if !self.horizontal_collide(st, x, 0x1C) {
            return;
        }
        let o = &mut self.o[x];
        o.xvs = 0;
        o.xvp = 2;
        o.ptr = 0xAE4F;
    }

    /// AI `$66` (`$3BA93F`): an air bubble rises (1 px/frame) while the probe above it is pure water ($60) on its own screen, else it vanishes.
    fn ai_a93f(&mut self, st: &Stage, x: usize) {
        let (_, types) = self.up_collide_types(st, x, 0x23);
        if types.iter().fold(0u8, |a, t| a | t) != 6 || self.o[x].ypg != 0 {
            self.despawn(x);
        }
    }

    /// code_3DA7CD: the body is only vulnerable (hitbox bit 6) from animation frame `$0B` on. Returns the carry the original leaves (frame >= `$0B`).
    fn a7cd(&mut self, x: usize) -> bool {
        let o = &mut self.o[x];
        o.hitbox |= 0x40;
        if o.aframe < 0x0B {
            o.hitbox &= 0xBF;
        }
        o.aframe >= 0x0B
    }

    /// Mega Man's state `$30` becomes 9 (waiting) if it was 0 (code_3DA5C5).
    pub(super) fn a5c5(&mut self) {
        if self.p.state == 0 {
            self.p.state = 9;
            self.player_cmd = Some(9);
        }
    }

    /// AI `$2B` first state (`$3DA602`, list type `1E`, a big machine): drops in from above (Mega Man is frozen), stops at Y `$7F` and runs the
    /// animation; at animation frame 3 it sends six screen-update requests (`$1C`/`$1D`, table `$A806`), one whenever the NMI has taken the last.
    fn ai_a602(&mut self, x: usize) {
        self.a5c5();
        self.move_gravity(x);
        if 0x7F >= self.o[x].ypx {
            self.o[x].atick = 0;
            return;
        }
        self.o[x].ypx = 0x7F;
        if self.o[x].aframe != 3 {
            return;
        }
        if self.w1d & 0x80 != 0 {
            self.o[x].atick = 0;
            return;
        }
        self.p1c = 0x1C;
        let k = self.o[x].s[0] as u16;
        self.o[x].s[0] = self.o[x].s[0].wrapping_add(1);
        self.w1d = self.ai_byte(0xA806 + k);
        if self.o[x].s[0] == 6 {
            self.set_anim(x, 0x36);
            self.o[x].xpx = 0xB0;
            self.o[x].ptr = 0xA67D;
            self.player_cmd = Some(0); // code_3DA5CE: $30 = 0
            self.p.state = 0;
            self.o[22].flags &= 0xFB;
            self.o[21].flags &= 0xFB;
            let o = &mut self.o[x];
            o.s[0] = 0x8C;
            o.s[1] = 0;
            o.s[2] = 0;
            // the palette copy ($0604.., $18 = $FF) is only drawing
        }
        self.o[x].atick = 0;
    }

    /// AI `$2B` main state (`$3DA67D`): waits (timer `$0468`), then randomly throws a bomb (type `$2C`) or flies to a new spot.
    fn ai_a67d(&mut self, x: usize) {
        let carry = self.a7cd(x);
        if self.o[x].s[0] != 0 {
            self.o[x].s[0] -= 1;
            return;
        }
        let sum = self.rng[0] as u16 + self.rng[1] as u16 + carry as u16;
        self.rng[0] = sum as u8;
        let (s1, s2) = (self.o[x].s[1], self.o[x].s[2]);
        if sum as u8 & 3 == 0 {
            // $A708
            if s1 == 0 {
                self.o[x].s[2] = 0;
                self.a721(x);
            } else {
                let n = (s2 + 1) & 3;
                self.o[x].s[2] = n;
                if n == 0 {
                    self.a6ac(x);
                } else {
                    self.a721(x);
                }
            }
        } else if s1 == 0 {
            let n = (s2 + 1) & 3;
            self.o[x].s[2] = n;
            if n == 0 {
                self.o[x].s[2] = 0;
                self.a721(x); // $A71C
            } else {
                self.a6ac(x);
            }
        } else {
            self.o[x].s[2] = 0;
            self.a6ac(x);
        }
    }

    /// `$3DA6AC`: throw a bomb (type `$2C`, animation `$39`) from (`$D4`, `$60`); its sideways speed grows with the distance to Mega Man.
    fn a6ac(&mut self, x: usize) {
        self.o[x].s[1] = 0;
        self.o[x].s[0] = 0x8C;
        let Some(y) = self.find_slot() else { return };
        self.child_here(x, y, 0x39);
        self.o[y].xpx = 0xD4;
        self.o[y].ypx = 0x60;
        self.o[y].ty = 0x2C;
        self.o[y].hitbox = 0xCE;
        self.o[y].hp = 2;
        let d = self.dist_x(x);
        let k = if d >= 0x61 { 2 } else if d >= 0x41 { 1 } else { 0 } as u16;
        let (xvs, xvp) = (self.ai_byte(0xA81C + k), self.ai_byte(0xA81F + k));
        let c = &mut self.o[y];
        c.yvs = 0xE5;
        c.yvp = 4;
        c.xvs = xvs;
        c.xvp = xvp;
        c.dir = 2;
    }

    /// `$3DA721`: fly toward Mega Man along the heading (table `$A80C`, only headings 9-15 move) with the speeds of `$A82A`-`$A83F`.
    fn a721(&mut self, x: usize) {
        self.o[x].s[1] = 1;
        let h = self.heading_to_player(x) as u16;
        let d = self.ai_byte(0xA80C + h);
        if d == 0 {
            return;
        }
        let i = h - 9;
        let (yvs, yvp, xvs, xvp) = (self.ai_byte(0xA82A + i), self.ai_byte(0xA831 + i), self.ai_byte(0xA838 + i), self.ai_byte(0xA83F + i));
        let o = &mut self.o[x];
        o.dir = d;
        o.yvs = yvs;
        o.yvp = yvp;
        o.xvs = xvs;
        o.xvp = xvp;
        o.ptr = 0xA75F;
        o.s[0] = 0;
    }

    /// AI `$2B` flying state (`$3DA75F`): moves until it reaches the edge of its area, bounces back, or goes home when it hits the right limit.
    fn ai_a75f(&mut self, x: usize) {
        self.a7cd(x);
        if self.o[x].s[0] != 0 {
            self.o[x].s[0] -= 1;
            return;
        }
        self.move_h(x);
        self.move_v(x);
        self.o[x].flags &= 0xBF;
        if self.o[x].dir & 1 == 0 {
            let o = &mut self.o[x];
            if 0x10 >= o.xpx {
                o.xpx = 0x10;
            } else if 0x10 >= o.ypx {
                o.ypx = 0x10;
            } else if 0xD0 >= o.ypx {
                return;
            } else {
                o.ypx = 0xD0;
            }
            o.s[0] = 0x14;
            o.dir ^= 0x0F;
        } else {
            if 0xB0 >= self.o[x].xpx {
                return;
            }
            let o = &mut self.o[x];
            o.xpx = 0xB0;
            o.ypx = 0x80;
            o.s[0] = 0x8C;
            o.ptr = 0xA67D;
            self.set_anim(x, 0x36);
        }
    }

    /// AI `$2C` (`$3DA7E5`): the thrown bomb; when it lands or hits a wall it turns into an explosion object (`$83`, animation `$62`).
    fn ai_a7e5(&mut self, st: &Stage, x: usize) {
        if !self.vertical_collide(st, x, 0x11) && !self.horizontal_collide(st, x, 0x1A) {
            return;
        }
        self.despawn(x);
        self.set_anim(x, 0x62);
        self.o[x].ty = 0x83;
        self.o[x].hitbox = 0xB7;
    }

    /// AI `$2D` (`$3DA846`, what the machine becomes when it is destroyed): six screen-update requests, then every object from slot 6 up is removed and
    /// slot 22 becomes the burst emitter `$3C`.
    fn ai_a846(&mut self, x: usize) {
        if self.w1d & 0x80 != 0 {
            return;
        }
        self.p1c = self.cam_now.hi;
        let k = self.o[x].s[0] as u16;
        self.o[x].s[0] = self.o[x].s[0].wrapping_add(1);
        self.w1d = self.ai_byte(0xA806 + k);
        if self.o[x].s[0] != 6 {
            return;
        }
        for y in (6..=0x17).rev() {
            self.despawn(y);
        }
        let o = &mut self.o[0x16];
        o.aid = 0x62;
        o.aframe = 0;
        o.atick = 0;
        o.ty = 0x3C;
    }

    /// AI `$3C` (`$3A9FE8`): the burst emitter: every 6 frames a puff (type `$3B`, animation `$62`) at one of five offsets; after the fifth it ends.
    fn ai_9fe8(&mut self, x: usize) {
        if self.o[x].s[0] != 0 {
            self.o[x].s[0] -= 1;
            if self.o[x].s[0] != 0 {
                return;
            }
        }
        let Some(y) = self.find_slot() else { return };
        self.snd(0x28, 0x9FFE); // code_3A9FFB
        self.o[y].ty = 0x3B;
        self.o[y].hitbox = 0;
        let k = self.o[x].s[1] as u16;
        let idx = self.ai_byte(0xA02B + k);
        self.child_at(x, y, idx, 0x62);
        self.o[x].s[0] = 6;
        self.o[x].s[1] += 1;
        if self.o[x].s[1] == 5 {
            self.despawn(x);
        }
    }
}
