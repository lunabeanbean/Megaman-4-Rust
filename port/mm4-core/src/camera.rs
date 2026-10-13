//! The follow camera and the section / vertical-transition logic (notes/camera.md, code at $3EC96E-$3ECE96).
//!
//! The camera only moves when the player has moved this frame: if he moved right and now stands at or beyond
//! screen x $80, it advances by the same amount (at most 8 px); if he moved left and stands at or before $80 it goes back.
//! Net effect: camera = player X - $80, kept inside the current section (`start * 256 ..= (start + nscreens) * 256`).
//!
//! Sections (`$B530` table, `$2B` = section | flags, `$2D` = last screen index, `$2E` = screen index): when the player's Y leaves the
//! screen at the bottom (`>= $E8`) or, on a ladder, at the top (`< 8`), code_3ECC28 either moves to the next/previous screen of a
//! vertical section or switches section (code_3ECD24), then runs a 120-frame scroll loop (code_3ECDD5) during which the game
//! logic is frozen: the screen scrolls 2 px per frame and the player moves 1.75 px the other way.
//! Not ported: horizontal section chaining at the end of a section (code_3ECAFF), the section-flag `$20` screen loading, and the
//! nametable streaming (graphics only).
use crate::anim::AnimTable;
use crate::player::{Player, STATE_LADDER};
use crate::stage::Stage;

const SCREEN_Y: i32 = 240 << 8;

#[derive(Clone)]
pub struct Camera {
    /// Sound requests made this frame (the boss-room doors).
    pub snd: Vec<crate::sound::Req>,
    pub x: i32,
    pub(crate) last_px: i32,
    /// `$2B`: low nibble = section index, high bits = flags (`$80`/`$40`: vertical section direction bits; `$20`: first-screen flag).
    pub sect: u8,
    /// `$2D`: index of the last screen of the section.
    pub nscreens: i32,
    /// Page number of the section's first screen (`$F9 - $2E`).
    pub start: i32,
    /// `$FA`: vertical scroll offset (0 outside transitions).
    pub fa: u8,
    /// `$2F`: direction of the last horizontal scroll step (1 = right, 2 = left); the enemy spawner reads it every frame.
    pub dir: u8,
    /// Test hook: after the next `update` the camera X is forced to this value (recordings made after a teleport have section state the port cannot know).
    pub force_x: Option<i32>,
    /// Direction of the running transition (`$23`: 4 = down, 8 = up; 0 = the single idle frame after the last iteration).
    trans: Option<u8>,
    /// Set when a section change starts: the game must wipe object slots 1..23 (code_3EC438, called by both scroll loops).
    pub clear_objects: bool,
    /// Door-table chain: the game loads `$B300[target]` into the spawner cursors `$9E/$9F`.
    pub cursor: Option<u8>,
    /// New value of `$95` when the section change runs the tile-drawing helpers (code_3EE682 / code_3EE751 zero it and spend 16 frames).
    pub p95_reset: Option<u8>,
    /// The running transition is the horizontal chain (not a vertical scroll).
    horiz: bool,
    /// The scroll bookkeeping of the tile-column builder: `$24` column in the 32-column ring of the screen edge being built, `$2A` its page,
    /// `$23` the direction of the last build, `$25` the camera X low byte of the previous frame.
    pub c24: u8,
    pub c2a: u8,
    pub c23: u8,
    pub fc25: u8,
    /// A column was built this frame (page, ring column): the NMI-independent builder of stage 4 leaves its scratch values in `$E6/$E7`.
    pub col_build: Option<(u8, u8)>,
    /// `$26`: `$FA` of the previous iteration of the vertical scroll loop; `row_build` = the row builder (code_3ED052) ran in this iteration (page `$2A`, `$24`).
    pub c26: u8,
    pub row_build: Option<(u8, u8)>,
    /// The iteration ran the copy of the second buffer (code_3ED143) instead of the builder: the sibling row (`$24 ^ 1`) of the last build reaches the PPU.
    pub row_copy: bool,
    /// The boss-room doors (code_3FE682 at the start of the section change, code_3FE751 after the scroll): 4 steps of 4 video frames each. `door_wait` = frames still to wait,
    /// `door_kind` 0/1 = entry/exit door, `door_step` = a step to draw in this frame, `door_b_done` = the exit door has been started.
    pub door_wait: u8,
    pub door_kind: u8,
    pub door_step: Option<(u8, u8)>,
    pub door_frame: bool,
    /// The first scroll iteration after the entry door runs in a `transition_step` call, but in the game it belongs to the frame that triggered the section change (no animation tick).
    first_iter_free: bool,
    door_b_done: bool,
    /// A vertical scroll iteration ran (it ends with a wait for the NMI, i.e. one step of the random task).
    pub nmi_iter: bool,
    /// `$95`: the frame counter (stage 10 scrolls by itself on odd frames).
    pub frame95: u8,
}

/// `$24`, `$2A`, `$23`, `$25` as the stage load leaves them for camera X `x` (the right edge column is `x / 8 + 32`).
fn col_state(x: i32) -> (u8, u8, u8, u8) {
    let r = (x >> 3) + 32;
    ((r & 31) as u8, (r >> 5) as u8, 1, (x & 0xFF) as u8)
}

impl Camera {
    /// Camera in the first section of `stage`, at camera X `x`.
    pub fn new(x: i32, player_px: i32, stage: &Stage) -> Camera {
        let b = stage.sect_byte(0);
        Camera { x, last_px: player_px, sect: b & 0xF0, nscreens: (b & 0x0F) as i32, start: 0, fa: 0, dir: 1, force_x: None, trans: None, clear_objects: false, cursor: None, p95_reset: None, horiz: false, c24: col_state(x).0, c2a: col_state(x).1, c23: 1, fc25: col_state(x).3, col_build: None, c26: 0, row_build: None, row_copy: false, door_wait: 0, door_kind: 0, door_step: None, door_frame: false, first_iter_free: false, door_b_done: false, nmi_iter: false, frame95: 0, snd: Vec::new() }
    }

    /// Camera at the first screen of section `sect` (a stage restart at a checkpoint, code_3EC5C4).
    pub fn at_section(x: i32, player_px: i32, stage: &Stage, sect: u8) -> Camera {
        let b = stage.sect_byte(sect as usize);
        let mut flags = b & 0xF0;
        if flags & 0x20 != 0 {
            flags |= sect;
            if flags & 0xC0 != 0 {
                // the restart enters a vertical section through the entry scroll, whose last step masks `$2B &= $2F` (code_3ECE7B)
                flags &= 0x2F;
            }
        }
        Camera { x, last_px: player_px, sect: flags | sect, nscreens: (b & 0x0F) as i32, start: x >> 8, fa: 0, dir: 1, force_x: None, trans: None, clear_objects: false, cursor: None, p95_reset: None, horiz: false, c24: col_state(x).0, c2a: col_state(x).1, c23: 1, fc25: col_state(x).3, col_build: None, c26: 0, row_build: None, row_copy: false, door_wait: 0, door_kind: 0, door_step: None, door_frame: false, first_iter_free: false, door_b_done: false, nmi_iter: false, frame95: 0, snd: Vec::new() }
    }

    /// Camera at world X `x` inside the section that contains it (for recordings that start mid-stage).
    pub fn in_section(x: i32, player_px: i32, stage: &Stage) -> Camera {
        let (sect, start) = stage.section_of_page(x >> 8);
        let b = stage.sect_byte(sect as usize);
        let mut flags = b & 0xF0;
        if flags & 0x20 != 0 && start == 0 {
            flags &= 0x30; // the first screen of the stage
        }
        Camera { x, last_px: player_px, sect: flags | sect, nscreens: (b & 0x0F) as i32, start, fa: 0, dir: 1, force_x: None, trans: None, clear_objects: false, cursor: None, p95_reset: None, horiz: false, c24: col_state(x).0, c2a: col_state(x).1, c23: 1, fc25: col_state(x).3, col_build: None, c26: 0, row_build: None, row_copy: false, door_wait: 0, door_kind: 0, door_step: None, door_frame: false, first_iter_free: false, door_b_done: false, nmi_iter: false, frame95: 0, snd: Vec::new() }
    }

    /// True while a vertical transition runs (the frame's game logic is skipped; call `transition_step` instead).
    pub fn busy(&self) -> bool {
        self.trans.is_some()
    }

    /// Follow the player after this frame's movement. Also applies code_3ECC13: while the camera sits exactly on a screen boundary
    /// the player's X (low byte, `$0330`) is kept inside `$10..=$F0`, which is what stops him at the left edge of the stage; then
    /// the vertical-transition check (code_3ECC28) may start a scroll.
    pub fn update(&mut self, p: &mut Player, stage: &Stage) {
        if self.sect & 0xC0 == 0 {
            if self.follow(p, stage) {
                return; // the chain loop ran inside this routine and returned from all of it
            }
        } else {
            self.last_px = p.px(); // vertical sections never scroll sideways (code_3EC972 jumps to code_3ECC13)
        }
        if self.x & 0xFF == 0 {
            let low = (p.px() & 0xFF).clamp(0x10, 0xF0);
            p.x = (p.x & !0xFF00) | (low << 8);
        }
        self.vertical_check(p, stage);
        if let Some(x) = self.force_x.take() {
            self.x = x;
        }
        self.fc25 = (self.x & 0xFF) as u8; // code_3EC799
    }

    /// Returns true when the camera reached the end of the section with Mega Man past x $F0 and a horizontal chain (code_3ECAFF) started.
    fn follow(&mut self, p: &mut Player, stage: &Stage) -> bool {
        let mut player_px = p.px();
        let mut delta = player_px - self.last_px;
        let mut screen_x = player_px - self.x;
        let mut built = 0u8;
        // code_3EC981: stage 10 scrolls by itself (1 px every other frame) once the camera has left the start of the stage,
        // pushing Mega Man along so that he stays between x $10 and $F0 of the screen
        if stage.id == 10 && (self.x & 0xFF != 0 || self.page() != self.start) {
            if self.frame95 & 1 == 0 {
                return false;
            }
            let sx = ((p.x >> 8) as u8).wrapping_sub(self.x as u8);
            if sx > 0xF0 {
                let d = (sx - 0xF0) as i32;
                p.x -= d << 8;
            } else if sx <= 0x10 {
                let d = (0x10 - sx) as i32;
                p.x += d << 8;
            }
            player_px = p.px();
            delta = 1;
            screen_x = 0x80;
        }
        if delta > 0 && screen_x >= 0x80 {
            self.x += delta.min(8);
            if self.page() - self.start >= self.nscreens {
                // code_3ECA53: arrived on the last screen of the section: camera X low byte forced to 0
                self.x = (self.start + self.nscreens) * 256;
                if p.wall_type == 5 || player_px & 0xFF > 0xF0 {
                    if p.wall_type != 5 {
                        p.x = (p.x & !0xFF00) | (0xF0 << 8);
                    }
                    self.last_px = p.px();
                    if self.chain(p, stage) {
                        return true;
                    }
                } else {
                    self.dir = 1;
                    built = 1;
                }
            } else {
                self.dir = 1;
                built = 1;
            }
        } else if delta < 0 && screen_x <= 0x80 {
            self.x -= (-delta).min(8);
            self.dir = 2;
            built = 2;
        }
        self.x = self.x.clamp(self.start * 256, (self.start + self.nscreens) * 256);
        self.last_px = p.px();
        if built != 0 {
            self.col_check(built);
        }
        false
    }

    /// code_3ECA79: the camera moved: how many 8-pixel columns did it cross? One build of the edge column (`code_3ECFB1`) when it is at least one.
    fn col_check(&mut self, dirv: u8) {
        let fc = (self.x & 0xFF) as u8;
        let diff = fc.wrapping_sub(self.fc25);
        let d = if diff & 0x80 == 0 { diff } else { (!diff).wrapping_add(1) };
        if d == 0 {
            return;
        }
        let fine = if dirv & 1 != 0 { self.fc25 } else { !self.fc25 } & 7;
        if (fine + d) >> 3 == 0 {
            return;
        }
        let same = dirv == self.c23;
        self.c23 = dirv;
        let delta: u8 = if dirv & 1 != 0 { 1 } else { 0xFF };
        if same {
            let s = self.c24.wrapping_add(delta);
            self.c24 = s & 0x1F;
            if s >= 0x20 {
                self.c2a = self.c2a.wrapping_add(delta);
            }
        } else {
            self.c2a = self.c2a.wrapping_add(delta);
        }
        self.col_build = Some((self.c2a, self.c24));
    }

    /// code_3ECAFF: the camera sits on the last screen and Mega Man pushes past x $F0. If the next section is a plain horizontal continuation
    /// (its flags `& $E0 == $20`) or the `$B540` table has a door entry with flag `$20` for this page, enter it and run the 128-frame scroll loop.
    fn chain(&mut self, p: &mut Player, stage: &Stage) -> bool {
        let y = (self.sect & 0x0F) as usize;
        let next = stage.sect_byte(y + 1);
        if p.state == 4 && next & 0x10 != 0 {
            return false;
        }
        let target;
        if next & 0xE0 == 0x20 {
            target = y + 1;
            self.start = self.page() + 1;
        } else {
            // code_3ECACF: door table
            let mut i = 0;
            loop {
                let page = stage.trans_byte(i);
                if page >= 0x80 {
                    return false;
                }
                if page as i32 == self.page() {
                    break;
                }
                i += 4;
            }
            if stage.trans_byte(i + 1) & 0x20 == 0 {
                return false;
            }
            let tp = stage.trans_byte(i + 3);
            self.cursor = Some(stage.byte(0xB300 + tp as usize));
            self.x = ((tp as i32 - 1) << 8) | (self.x & 0xFF); // F9 = target - 1: the loop scrolls into the target page
            p.x = (p.x & 0xFFFF) | (((tp as i32 - 1) & 0xFF) << 16);
            self.start = tp as i32;
            target = stage.trans_byte(i + 2) as usize;
        }
        // code_3ECB1B
        let b = stage.sect_byte(target);
        self.sect = (b & 0x30) | target as u8;
        self.nscreens = (b & 0x0F) as i32;
        self.dir = 1;
        if b & 0x10 != 0 {
            self.p95_reset = Some(0); // code_3FE682 zeroes `$95`; each of its 16 waits adds one (the draw pass at the end of every step does that)
            self.door_kind = 0;
            self.door_wait = 15; // this frame draws step 0, the next 15 wait; the scroll starts after them
            self.door_step = Some((0, 0));
            self.snd(0x2C, 0xE6C9); // code_3FE6C6
            self.first_iter_free = true;
        }
        self.clear_objects = true;
        self.trans = Some(1);
        self.c23 = 1;
        self.horiz = true;
        if self.door_wait == 0 {
            self.iterate(p);
        }
        true
    }

    /// code_3ECC28: Y below `$E8` (any state) or above 8 (ladder only) leaves the screen.
    fn vertical_check(&mut self, p: &mut Player, stage: &Stage) {
        if p.y < 0 || p.y >= SCREEN_Y {
            return; // $0390 != 0: other screens of a tall stage are not handled
        }
        let dir: u8 = if p.py() >= 0xE8 {
            4
        } else if p.py() >= 8 {
            return;
        } else if p.state == STATE_LADDER {
            8
        } else {
            return;
        };
        let was_vert = self.sect & 0xC0 != 0;
        let fwd = (dir << 4) & self.sect != 0;
        let moved = if was_vert { self.vertical_section(dir, stage) } else { self.section_change(dir, stage) };
        if moved {
            // the player keeps his pixel X but lives on the new page (`$0348 = $F9`)
            p.x = (p.x & 0xFFFF) | (self.x >> 8 << 16);
            self.last_px = p.px();
            self.trans = Some(dir);
            self.c23 = dir; // $23 = $01 & $0C
            self.c24 = if dir & 4 != 0 { 0x00 } else { 0x1D }; // code_3ECDD5: $CBFF[$11]
            self.c2a = if !was_vert { (self.x >> 8) as u8 } else if fwd { self.c2a.wrapping_add(1) } else { self.c2a.wrapping_sub(1) }; // $2A = $F9 (door) or INC/DEC $2A
            self.clear_objects = true;
            self.iterate(p); // the first loop iteration runs in the frame that triggered it
        }
    }

    fn page(&self) -> i32 {
        self.x >> 8
    }

    /// Enter section `y` (code_3ECB1B style): `$2B`, `$2D` and the page of its first screen.
    fn set_section(&mut self, stage: &Stage, y: u8, idx: i32) {
        let b = stage.sect_byte(y as usize);
        self.sect = (b & 0xF0) | y;
        self.nscreens = (b & 0x0F) as i32;
        self.start = self.page() - idx;
    }

    /// code_3ECC4F: inside a vertical section the bit in `$2B` says whether this direction leads to the next (+1) or previous (-1) screen.
    fn vertical_section(&mut self, dir: u8, stage: &Stage) -> bool {
        let idx = self.page() - self.start;
        if (dir << 4) & self.sect != 0 {
            // forward (code_3ECC67): next screen, or the next section when this was the last screen
            self.x += 0x100;
            self.dir = 1; // $2F
            if idx == self.nscreens {
                let y = (self.sect.wrapping_add(1)) & 0x0F; // code_3ECC79
                self.set_section(stage, y, 0);
            }
        } else {
            // backward (code_3ECC9F)
            self.x -= 0x100;
            self.dir = 2; // $2F: the spawner now walks its left cursor
            if idx == 0 {
                let y = (self.sect.wrapping_sub(1)) & 0x0F; // code_3ECCB1
                let n = (stage.sect_byte(y as usize) & 0x0F) as i32;
                self.set_section(stage, y, n);
            }
        }
        true
    }

    /// code_3ECD24: not in a vertical section: look at the neighbouring sections' flags, then at the `$B540` transition table.
    fn section_change(&mut self, dir: u8, stage: &Stage) -> bool {
        if self.x & 0xFF != 0 {
            return false;
        }
        let y = (self.sect & 0x0F) as usize;
        let idx = self.page() - self.start;
        // which neighbour is meant: next section (code_3ECD46) or previous (code_3ECD65)
        let forward = if self.nscreens == 0 {
            (stage.sect_byte(y + 1) >> 4) & dir != 0
        } else if idx == 0 {
            false
        } else if idx == self.nscreens {
            true
        } else {
            return false;
        };
        if forward {
            let nb = stage.sect_byte(y + 1);
            if nb & 0xC0 == 0 {
                return self.table_change(dir, stage); // code_3ECCD0
            }
            // code_3ECD92: mask $10 = the neighbour's flags; the bit for this direction must be set
            if (dir << 4) & nb & 0xC0 == 0 {
                return self.table_change(dir, stage);
            }
            self.x += 0x100;
            self.sect = (nb & 0xF0) | (y as u8 + 1);
            self.nscreens = (nb & 0x0F) as i32;
            self.start = self.page();
            self.dir = 1; // $2F = 1 (code_3ECDC8: the caller left 1 in $02)
            true
        } else {
            let cur = stage.sect_byte(y);
            if y == 0 {
                return false; // DEY went below 0
            }
            let pb = stage.sect_byte(y - 1);
            if cur & 0xC0 == 0 && pb & 0x20 != 0 {
                return self.table_change(dir, stage);
            }
            let mut mask = pb & 0xC0;
            if mask == 0 {
                return self.table_change(dir, stage);
            }
            mask ^= 0xC0;
            if (dir << 4) & mask == 0 {
                return self.table_change(dir, stage);
            }
            self.x -= 0x100;
            let n = (pb & 0x0F) as i32;
            self.sect = (pb & 0xF0) | (y as u8 - 1);
            self.nscreens = n;
            self.start = self.page() - n;
            self.dir = 2; // $2F: backward, the spawner walks its left cursor
            true
        }
    }

    /// code_3ECCD0: the `$B540` table, 4 bytes per entry: page, direction flags, target section, target page.
    fn table_change(&mut self, dir: u8, stage: &Stage) -> bool {
        let mut i = 0;
        loop {
            let page = stage.trans_byte(i);
            if page >= 0x80 {
                return false;
            }
            if page as i32 == self.page() {
                break;
            }
            i += 4;
        }
        if (dir << 4) & stage.trans_byte(i + 1) == 0 {
            return false;
        }
        let sec = stage.trans_byte(i + 2);
        let target = stage.trans_byte(i + 3) as i32;
        let b = stage.sect_byte(sec as usize);
        self.sect = (b & 0xF0) | sec;
        self.nscreens = (b & 0x0F) as i32;
        self.x = (target << 8) | (self.x & 0xFF);
        self.start = target;
        self.cursor = Some(stage.byte(0xB300 + target as usize)); // code_3ECD0D: spawner cursors $9E/$9F
        self.dir = 1; // $2F
        true
    }

    /// One frame of the transition loop (code_3ECDEE-code_3ECE7B), including the animation tick of the draw call.
    pub fn transition_step(&mut self, p: &mut Player, anims: &AnimTable, stage: &Stage) {
        if self.door_wait > 0 {
            self.door_wait -= 1;
            self.door_frame = true; // a bare wait: no draw pass, so nothing animates
            self.nmi_iter = true; // every one of these frames steps the NMI's random task
            let f = 15 - self.door_wait;
            if f % 4 == 0 {
                self.door_step = Some((self.door_kind, f / 4));
            }
            if self.door_wait == 0 && self.door_b_done {
                self.door_b_done = false;
                self.horiz = false;
                self.trans = None; // the exit door's last wait ends the camera routine
            }
            return;
        }
        if !std::mem::take(&mut self.first_iter_free) {
            p.anim.advance(anims);
        }
        if self.trans == Some(0) {
            if self.horiz && self.sect & 0x10 != 0 {
                // code_3ECBE4: a section with flag $10 redraws its door tiles (code_3EE751) unless the page is the one in the table at $CC03
                if stage.byte(0xCC03 + stage.id) as i32 != self.page() {
                    self.p95_reset = Some(1);
                    self.door_kind = 1;
                    self.door_wait = 15;
                    self.door_step = Some((1, 0));
                    self.snd(0x2C, 0xE796); // code_3FE793
                    self.door_b_done = true;
                    self.nmi_iter = true;
                    return;
                }
            }
            self.horiz = false;
            self.trans = None; // the frame that triggered the loop ends one frame after its last iteration
            return;
        }
        self.iterate(p);
    }

    /// Scroll 2 px and move the player 1.75 px the other way; the loop ends when `$FA` is back at 0 (120 iterations).
    fn iterate(&mut self, p: &mut Player) {
        let dir = self.trans.expect("iterate outside a transition");
        if dir == 1 {
            // code_3ECB6E: camera +2 px, Mega Man +0.5 px (sub-pixel $80)
            let old25 = self.fc25;
            self.x += 2;
            p.x += 0x80;
            self.nmi_iter = true;
            // code_3ECB94: one column of tiles per 8 px crossed
            let fc = (self.x & 0xFF) as u8;
            let d = fc.wrapping_sub(old25);
            if d != 0 && ((old25 & 7) + d) >> 3 != 0 {
                let n = (self.c24 + 1) & 0x1F;
                self.c24 = n;
                if n == 0 {
                    self.c2a = self.c2a.wrapping_add(1);
                }
                self.col_build = Some((self.c2a, self.c24));
            }
            self.fc25 = fc;
            if self.x & 0xFF == 0 {
                self.trans = Some(0);
                self.last_px = p.px();
            }
            return;
        }
        let old26 = self.c26;
        self.nmi_iter = true;
        if dir & 4 != 0 {
            self.fa = self.fa.wrapping_add(2);
            if self.fa >= 0xF0 {
                self.fa = self.fa.wrapping_add(0x10);
            }
            p.y = (p.y - 0x1C0).rem_euclid(SCREEN_Y);
        } else {
            let borrow = self.fa < 2;
            self.fa = self.fa.wrapping_sub(2);
            if borrow {
                self.fa = self.fa.wrapping_sub(0x10);
            }
            p.y = (p.y + 0x1C0).rem_euclid(SCREEN_Y);
        }
        self.c26 = self.fa;
        self.row_check(dir, old26);
        if self.fa == 0 {
            self.trans = Some(0);
            if self.sect & 0x20 != 0 {
                self.sect &= 0x2F; // code_3ECE7B
                self.c24 = 0;
                self.c2a = self.c2a.wrapping_add(1);
                self.c23 = 1;
            }
            self.last_px = p.px(); // the follow rule compares with the previous frame's position
        }
    }
}

impl Camera {
    /// code_3ECE97: the vertical scroll crossed 8-pixel rows since the previous iteration (`old26` = `$26`): alternately the row builder
    /// (code_3ED052, which runs the block decoder for 8 blocks of page `$2A` and one of the next page) or the copy of its buffer (code_3ED143).
    fn row_check(&mut self, dir: u8, old26: u8) {
        let diff = self.fa.wrapping_sub(old26);
        let d = if diff & 0x80 == 0 { diff } else { (!diff).wrapping_add(1) } & 0x0F;
        if d == 0 {
            return;
        }
        let fine = if dir & 4 != 0 { old26 } else { !old26 } & 7;
        if (fine + d) >> 3 == 0 {
            return;
        }
        let x = ((dir >> 2) & 1) as usize; // $11
        let want = [1u8, 0][x]; // $CC01
        if self.c24 & 1 == want {
            self.row_build = Some((self.c2a, self.c24));
        } else {
            self.row_copy = true;
        }
        self.c24 = self.c24.wrapping_add([0xFF, 0x01][x]); // $CBFD
    }
}

impl Camera {
    /// The scene a pod switch loads (code_3FE5C8 with section `sect` on page `page`): `$2B` = `$20 | sect` for a first-screen section, else the section's
    /// `$C0` direction flags | `sect`; `$2D` = its last screen; the camera keeps its low byte `$FC` (only the page changes).
    pub fn at_scene(page: i32, lo: i32, player_px: i32, stage: &Stage, sect: u8) -> Camera {
        let b = stage.sect_byte(sect as usize);
        let flags = if b & 0x20 != 0 { 0x20 | sect } else { (b & 0xC0) | sect };
        let mut c = Camera::at_section((page << 8) + lo, player_px, stage, sect);
        c.sect = flags;
        c
    }
}

impl Camera {
    /// `JSR code_3FF6BE`: queue a sound effect (`ret` = return address of the call in the ROM).
    pub fn snd(&mut self, id: u8, ret: u16) {
        self.snd.push(crate::sound::Req { music: false, id, ret });
    }

    /// `JSR code_3FF6BC`: a music / stop request (`$D9` = id, then the same queue).
    pub fn snd_music(&mut self, id: u8, ret: u16) {
        self.snd.push(crate::sound::Req { music: true, id, ret });
    }
}
