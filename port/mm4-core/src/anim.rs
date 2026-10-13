//! The object animation engine (the part of code_3EDC00 at $3EDC67-$3EDCE0, notes/player-states.md).
//!
//! Each object has an animation id (`$0558`), a frame index (`$0540`) and a tick counter (`$0570`). An animation is a record in the
//! object type's sprite bank: byte 0 = last frame index N (low 7 bits), byte 1 = duration D, then N+1 frame ids. Every frame the
//! drawing code ticks the counter; when the tick (before increment) equals D the counter restarts and the frame index advances,
//! wrapping to 0 after frame N. The game logic reads the frame index (e.g. the walk-start animation $0D blocks movement until
//! its frame index leaves 0), so this is gameplay, not just looks.
use crate::rom::Rom;

/// Player animation ids used by the ported states.
pub const IDLE: u8 = 0x01;
pub const RUN: u8 = 0x04;
pub const AIR: u8 = 0x07;
pub const WALK_START: u8 = 0x0D;
pub const SLIDE: u8 = 0x10;
pub const SPAWN_WAIT: u8 = 0x33;
pub const SPAWN_FALL: u8 = 0x13;
pub const LADDER: u8 = 0x0A;
pub const LADDER_TOP: u8 = 0x14;

#[derive(Clone)]
pub struct AnimTable {
    /// The sprite bank the player's type uses (mapped at $A000 while the object is drawn).
    bank: Vec<u8>,
    /// The bank before it (mapped at $8000 while the object is drawn): sprite definitions may live there.
    prev: Vec<u8>,
}

impl AnimTable {
    /// Table for object type `ty` (0 = Mega Man): the sprite bank number comes from $DFFA+type.
    pub fn load(rom: &Rom, ty: usize) -> AnimTable {
        let bank = rom.cpu_byte(0, 0, 0xDFFA + ty as u16) as usize;
        AnimTable { bank: rom.bank(bank).to_vec(), prev: rom.bank(bank.saturating_sub(1)).to_vec() }
    }

    /// A byte of the 16 KB window ($8000-$BFFF) the draw code sees for this object type.
    pub fn byte(&self, addr: u16) -> u8 {
        if addr >= 0xC000 || addr < 0x8000 {
            0 // outside the window the game would read other banks: treat as empty
        } else if addr >= 0xA000 {
            self.bank[addr as usize - 0xA000]
        } else {
            self.prev[addr as usize - 0x8000]
        }
    }

    /// Address of the record of animation `id`.
    pub fn record(&self, id: u8) -> u16 {
        (self.bank[0x1F00 + id as usize] as u16) << 8 | self.bank[0x1E00 + id as usize] as u16
    }

    /// Sprite id of frame `frame` of animation `id` (0 = the animation ends and the object disappears).
    pub fn sprite(&self, id: u8, frame: u8) -> u8 {
        self.byte(self.record(id).wrapping_add(2 + frame as u16))
    }

    /// (last frame index N, duration D) of animation `id`.
    pub fn header(&self, id: u8) -> (u8, u8) {
        let ptr = self.record(id);
        (self.byte(ptr) & 0x7F, self.byte(ptr.wrapping_add(1)))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Anim {
    pub id: u8,
    pub frame: u8,
    pub tick: u8,
}

impl Anim {
    pub fn new(id: u8) -> Anim {
        Anim { id, frame: 0, tick: 0 }
    }

    /// Set the tick so that the frame index changes `k` rows from now (the trace's phase is unknown at its first row).
    pub fn sync_tick(&mut self, table: &AnimTable, k: usize) {
        let (_, dur) = table.header(self.id);
        self.tick = dur.wrapping_sub((k as u8).wrapping_sub(1));
    }

    /// code_3FF446: switch animation and restart it.
    pub fn set(&mut self, id: u8) {
        *self = Anim::new(id);
    }

    /// One call of the animation tick (runs once per frame after the object's logic).
    pub fn advance(&mut self, table: &AnimTable) {
        if self.id == 0 {
            return;
        }
        let (last, dur) = table.header(self.id);
        let t = self.tick & 0x7F;
        self.tick = self.tick.wrapping_add(1);
        if t == dur {
            self.tick &= 0x80;
            let old = self.frame & 0x7F;
            self.frame = self.frame.wrapping_add(1);
            if old == last {
                self.frame = 0;
            }
        }
    }
}
