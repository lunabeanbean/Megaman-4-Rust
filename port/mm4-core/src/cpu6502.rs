//! A small 6502 core (official opcodes, no decimal mode) used as the reference for the sound driver port: it runs the original driver bytes of bank `$1E`
//! and logs the APU register writes, so `sound.rs` can be checked against it on any sequence of calls (and the core itself against Mesen's recordings).

pub trait Bus {
    fn read(&mut self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, val: u8);
}

#[derive(Clone, Default)]
pub struct Cpu {
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub sp: u8,
    pub pc: u16,
    pub c: bool,
    pub z: bool,
    pub i: bool,
    pub d: bool,
    pub v: bool,
    pub n: bool,
    pub cycles: u64,
}

const NO_ADDR: u16 = 0;

impl Cpu {
    pub fn new() -> Cpu {
        Cpu { sp: 0xFD, i: true, ..Default::default() }
    }

    fn p(&self, brk: bool) -> u8 {
        (self.c as u8) | (self.z as u8) << 1 | (self.i as u8) << 2 | (self.d as u8) << 3 | (brk as u8) << 4 | 0x20 | (self.v as u8) << 6 | (self.n as u8) << 7
    }

    fn set_p(&mut self, p: u8) {
        self.c = p & 1 != 0;
        self.z = p & 2 != 0;
        self.i = p & 4 != 0;
        self.d = p & 8 != 0;
        self.v = p & 0x40 != 0;
        self.n = p & 0x80 != 0;
    }

    fn nz(&mut self, v: u8) -> u8 {
        self.z = v == 0;
        self.n = v & 0x80 != 0;
        v
    }

    fn push<B: Bus>(&mut self, b: &mut B, v: u8) {
        b.write(0x100 + self.sp as u16, v);
        self.sp = self.sp.wrapping_sub(1);
    }

    fn pull<B: Bus>(&mut self, b: &mut B) -> u8 {
        self.sp = self.sp.wrapping_add(1);
        b.read(0x100 + self.sp as u16)
    }

    fn fetch<B: Bus>(&mut self, b: &mut B) -> u8 {
        let v = b.read(self.pc);
        self.pc = self.pc.wrapping_add(1);
        v
    }

    fn fetch16<B: Bus>(&mut self, b: &mut B) -> u16 {
        let lo = self.fetch(b) as u16;
        lo | (self.fetch(b) as u16) << 8
    }

    fn read16_zp<B: Bus>(b: &mut B, zp: u8) -> u16 {
        let lo = b.read(zp as u16) as u16;
        lo | (b.read(zp.wrapping_add(1) as u16) as u16) << 8
    }

    fn adc(&mut self, m: u8) {
        let s = self.a as u16 + m as u16 + self.c as u16;
        self.v = (!(self.a ^ m) & (self.a ^ s as u8) & 0x80) != 0;
        self.c = s > 0xFF;
        self.a = self.nz(s as u8);
    }

    fn cmp(&mut self, r: u8, m: u8) {
        self.c = r >= m;
        self.nz(r.wrapping_sub(m));
    }

    fn branch<B: Bus>(&mut self, b: &mut B, cond: bool) {
        let off = self.fetch(b) as i8;
        if cond {
            self.pc = self.pc.wrapping_add(off as i16 as u16);
        }
    }

    /// Effective address of the addressing mode of opcode `op` (mode from the usual aaabbbcc layout).
    fn addr<B: Bus>(&mut self, b: &mut B, mode: u8) -> u16 {
        match mode {
            0 => { let a = self.pc; self.pc = self.pc.wrapping_add(1); a } // immediate
            1 => self.fetch(b) as u16,                                        // zero page
            2 => self.fetch(b).wrapping_add(self.x) as u16,                   // zp,x
            3 => self.fetch(b).wrapping_add(self.y) as u16,                   // zp,y
            4 => self.fetch16(b),                                             // abs
            5 => self.fetch16(b).wrapping_add(self.x as u16),                 // abs,x
            6 => self.fetch16(b).wrapping_add(self.y as u16),                 // abs,y
            7 => { let z = self.fetch(b).wrapping_add(self.x); Self::read16_zp(b, z) } // (zp,x)
            8 => { let z = self.fetch(b); Self::read16_zp(b, z).wrapping_add(self.y as u16) } // (zp),y
            _ => NO_ADDR,
        }
    }

    /// Runs one instruction.
    pub fn step<B: Bus>(&mut self, b: &mut B) {
        let op = self.fetch(b);
        self.cycles += 2;
        match op {
            0x00 => { // BRK
                self.pc = self.pc.wrapping_add(1);
                let pc = self.pc;
                self.push(b, (pc >> 8) as u8);
                self.push(b, pc as u8);
                let p = self.p(true);
                self.push(b, p);
                self.i = true;
                self.pc = b.read(0xFFFE) as u16 | (b.read(0xFFFF) as u16) << 8;
            }
            0x08 => { let p = self.p(true); self.push(b, p); }
            0x28 => { let p = self.pull(b); self.set_p(p); }
            0x48 => { let a = self.a; self.push(b, a); }
            0x68 => { let v = self.pull(b); self.a = self.nz(v); }
            0x18 => self.c = false,
            0x38 => self.c = true,
            0x58 => self.i = false,
            0x78 => self.i = true,
            0xB8 => self.v = false,
            0xD8 => self.d = false,
            0xF8 => self.d = true,
            0xAA => { self.x = self.nz(self.a); }
            0x8A => { self.a = self.nz(self.x); }
            0xA8 => { self.y = self.nz(self.a); }
            0x98 => { self.a = self.nz(self.y); }
            0xBA => { self.x = self.nz(self.sp); }
            0x9A => self.sp = self.x,
            0xE8 => { self.x = self.nz(self.x.wrapping_add(1)); }
            0xCA => { self.x = self.nz(self.x.wrapping_sub(1)); }
            0xC8 => { self.y = self.nz(self.y.wrapping_add(1)); }
            0x88 => { self.y = self.nz(self.y.wrapping_sub(1)); }
            0xEA => {}
            0x10 => { let c = !self.n; self.branch(b, c) }
            0x30 => { let c = self.n; self.branch(b, c) }
            0x50 => { let c = !self.v; self.branch(b, c) }
            0x70 => { let c = self.v; self.branch(b, c) }
            0x90 => { let c = !self.c; self.branch(b, c) }
            0xB0 => { let c = self.c; self.branch(b, c) }
            0xD0 => { let c = !self.z; self.branch(b, c) }
            0xF0 => { let c = self.z; self.branch(b, c) }
            0x20 => { // JSR
                let t = self.fetch16(b);
                let ret = self.pc.wrapping_sub(1);
                self.push(b, (ret >> 8) as u8);
                self.push(b, ret as u8);
                self.pc = t;
            }
            0x4C => { self.pc = self.fetch16(b); }
            0x6C => { // JMP (ind) with the page-wrap bug
                let p = self.fetch16(b);
                let lo = b.read(p) as u16;
                let hi = b.read((p & 0xFF00) | (p.wrapping_add(1) & 0xFF)) as u16;
                self.pc = lo | hi << 8;
            }
            0x60 => { let lo = self.pull(b) as u16; let hi = self.pull(b) as u16; self.pc = (lo | hi << 8).wrapping_add(1); }
            0x40 => { let p = self.pull(b); self.set_p(p); let lo = self.pull(b) as u16; let hi = self.pull(b) as u16; self.pc = lo | hi << 8; }
            0x24 | 0x2C => { // BIT
                let a = if op == 0x24 { self.fetch(b) as u16 } else { self.fetch16(b) };
                let m = b.read(a);
                self.z = self.a & m == 0;
                self.n = m & 0x80 != 0;
                self.v = m & 0x40 != 0;
            }
            _ => {
                let cc = op & 3;
                let bbb = (op >> 2) & 7;
                let aaa = op >> 5;
                match cc {
                    1 => {
                        // ALU group: ORA AND EOR ADC STA LDA CMP SBC
                        let mode = [7u8, 1, 0, 4, 8, 2, 6, 5][bbb as usize];
                        let ad = self.addr(b, mode);
                        if aaa == 4 {
                            let a = self.a;
                            b.write(ad, a);
                            return;
                        }
                        let m = b.read(ad);
                        match aaa {
                            0 => { self.a = self.nz(self.a | m); }
                            1 => { self.a = self.nz(self.a & m); }
                            2 => { self.a = self.nz(self.a ^ m); }
                            3 => self.adc(m),
                            5 => { self.a = self.nz(m); }
                            6 => { let a = self.a; self.cmp(a, m); }
                            _ => self.adc(!m),
                        }
                    }
                    2 => {
                        // shifts / LDX STX INC DEC
                        let acc = bbb == 2 && aaa < 4;
                        if acc {
                            let a = self.a;
                            self.a = self.shift(aaa, a);
                            return;
                        }
                        let mode = [0u8, 1, 0, 4, 0, 2, 0, 5][bbb as usize];
                        let mode = if (aaa == 4 || aaa == 5) && (bbb == 5 || bbb == 7) { if bbb == 5 { 3 } else { 6 } } else { mode };
                        let ad = self.addr(b, mode);
                        match aaa {
                            4 => { let x = self.x; b.write(ad, x); }
                            5 => { let v = b.read(ad); self.x = self.nz(v); }
                            6 => { let v = b.read(ad).wrapping_sub(1); b.write(ad, v); self.nz(v); }
                            7 => { let v = b.read(ad).wrapping_add(1); b.write(ad, v); self.nz(v); }
                            _ => { let v = b.read(ad); let r = self.shift(aaa, v); b.write(ad, r); }
                        }
                    }
                    0 => {
                        // LDY STY CPY CPX
                        let mode = [0u8, 1, 0, 4, 0, 2, 0, 5][bbb as usize];
                        let ad = self.addr(b, mode);
                        match aaa {
                            4 => { let y = self.y; b.write(ad, y); }
                            5 => { let v = b.read(ad); self.y = self.nz(v); }
                            6 => { let m = b.read(ad); let y = self.y; self.cmp(y, m); }
                            7 => { let m = b.read(ad); let x = self.x; self.cmp(x, m); }
                            _ => panic!("6502: unsupported opcode {:02X} at {:04X}", op, self.pc.wrapping_sub(1)),
                        }
                    }
                    _ => panic!("6502: unsupported opcode {:02X} at {:04X}", op, self.pc.wrapping_sub(1)),
                }
            }
        }
    }

    fn shift(&mut self, aaa: u8, v: u8) -> u8 {
        let r = match aaa {
            0 => { self.c = v & 0x80 != 0; v << 1 }
            1 => { let c = self.c as u8; self.c = v & 0x80 != 0; v << 1 | c }
            2 => { self.c = v & 1 != 0; v >> 1 }
            _ => { let c = (self.c as u8) << 7; self.c = v & 1 != 0; v >> 1 | c }
        };
        self.nz(r)
    }
}
