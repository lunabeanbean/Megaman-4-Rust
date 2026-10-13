//! Access to the owner's ROM (iNES file with a 16-byte header; 8 KB PRG banks).
use std::fs;

const HEADER: usize = 0x10;
pub const BANK_SIZE: usize = 0x2000;

#[derive(Clone)]
pub struct Rom {
    data: Vec<u8>,
}

impl Rom {
    pub fn load(path: &str) -> Result<Rom, String> {
        let data = fs::read(path).map_err(|e| format!("cannot read ROM '{}': {}", path, e))?;
        if data.len() < HEADER + 0x40 * BANK_SIZE || &data[0..4] != b"NES\x1a" {
            return Err(format!("'{}' does not look like the Mega Man 4 ROM (need a 512 KB iNES file)", path));
        }
        Ok(Rom { data })
    }

    /// One 8 KB PRG bank.
    pub fn bank(&self, n: usize) -> &[u8] {
        let start = HEADER + n * BANK_SIZE;
        &self.data[start..start + BANK_SIZE]
    }

    /// Read a CPU address with PRG banks `lo` mapped at $8000 and `hi` at $A000; $C000-$FFFF are fixed to banks $3E/$3F.
    pub fn cpu_byte(&self, lo: usize, hi: usize, addr: u16) -> u8 {
        let a = addr as usize;
        match a {
            0x8000..=0x9FFF => self.bank(lo)[a - 0x8000],
            0xA000..=0xBFFF => self.bank(hi)[a - 0xA000],
            0xC000..=0xDFFF => self.bank(0x3E)[a - 0xC000],
            _ => self.bank(0x3F)[a - 0xE000],
        }
    }
}
