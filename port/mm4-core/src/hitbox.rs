//! Collision probe shapes, read from the ROM tables in fixed bank $3E (notes/collision.md).
//!
//! A record is `[last, first, delta...]` (`last + 1` points): the first value is the constant coordinate of the probe line,
//! the following values are cumulative offsets along it. Vertical records (`$D69E/$D6DE`) hold a Y and a list of Xs
//! (feet/head probes); horizontal records (`$D808/$D848`) hold an X and a list of Ys (side probes).
use crate::rom::Rom;

#[derive(Clone)]
pub struct Hitboxes {
    bank: Vec<u8>,
}

/// One probe line: the fixed coordinate and the offsets along the other axis.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Probe {
    pub fixed: i32,
    pub along: Vec<i32>,
}

impl Hitboxes {
    pub fn load(rom: &Rom) -> Hitboxes {
        Hitboxes { bank: rom.bank(0x3E).to_vec() }
    }

    fn record(&self, lo: usize, hi: usize, id: usize) -> Probe {
        let ptr = (((self.bank[hi + id] as usize) << 8) | self.bank[lo + id] as usize) - 0xC000;
        let last = self.bank[ptr] as usize;
        let fixed = self.bank[ptr + 1] as i8 as i32;
        let mut v = self.bank[ptr + 2] as i8 as i32;
        let mut along = vec![v];
        for i in 1..=last {
            v += self.bank[ptr + 2 + i] as i8 as i32;
            along.push(v);
        }
        Probe { fixed, along }
    }

    /// Vertical probe `id`: `fixed` = Y offset, `along` = X offsets.
    pub fn vertical(&self, id: usize) -> Probe {
        self.record(0x169E, 0x16DE, id)
    }

    /// Horizontal probe `id`: `fixed` = X offset, `along` = Y offsets.
    pub fn horizontal(&self, id: usize) -> Probe {
        self.record(0x1808, 0x1848, id)
    }
}
