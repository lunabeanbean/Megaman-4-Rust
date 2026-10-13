//! The original sound driver (bank `$1E`) run on the small 6502 core: the reference the Rust port in `sound.rs` is checked against, and the check of the core itself
//! against Mesen's recordings (`harness/snd_rec.lua`: every driver call and every APU register write, in order).
use crate::cpu6502::{Bus, Cpu};
use crate::rom::Rom;

const SENTINEL: u16 = 0x0002;

pub struct RefDriver {
    rom: Rom,
    ram: Box<[u8; 0x800]>,
    sel: u8,
    r7: u8,
    pub writes: Vec<(u16, u8)>,
}

impl Bus for RefDriver {
    fn read(&mut self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1FFF => self.ram[(addr & 0x7FF) as usize],
            0x8000..=0x9FFF => self.rom.bank(0x1E)[(addr - 0x8000) as usize],
            0xA000..=0xBFFF => self.rom.bank(self.r7 as usize)[(addr - 0xA000) as usize],
            0xC000..=0xDFFF => self.rom.bank(0x3E)[(addr - 0xC000) as usize],
            0xE000..=0xFFFF => self.rom.bank(0x3F)[(addr - 0xE000) as usize],
            _ => 0,
        }
    }

    fn write(&mut self, addr: u16, val: u8) {
        match addr {
            0x0000..=0x1FFF => self.ram[(addr & 0x7FF) as usize] = val,
            0x4000..=0x4017 => self.writes.push((addr, val)),
            0x8000..=0x9FFF if addr & 1 == 0 => self.sel = val & 7,
            0x8000..=0x9FFF => {
                if self.sel == 7 {
                    self.r7 = val & 0x3F;
                }
            }
            _ => {}
        }
    }
}

impl RefDriver {
    pub fn new(rom: &Rom) -> RefDriver {
        RefDriver { rom: rom.clone(), ram: Box::new([0; 0x800]), sel: 0, r7: 0x1F, writes: Vec::new() }
    }

    fn call(&mut self, addr: u16, a: u8, y: u8) {
        let mut cpu = Cpu::new();
        cpu.a = a;
        cpu.y = y;
        cpu.sp = 0xF0;
        let ret = SENTINEL.wrapping_sub(1);
        self.write(0x1F0, (ret >> 8) as u8);
        self.write(0x1EF, ret as u8);
        cpu.sp = 0xEE;
        cpu.pc = addr;
        let mut n = 0u32;
        while cpu.pc != SENTINEL {
            cpu.step(self);
            n += 1;
            assert!(n < 2_000_000, "driver call does not return");
        }
        self.r7 = 0x1F;
    }

    /// The per-frame update (`$8000`).
    pub fn tick(&mut self) {
        self.call(0x8000, 0, 0);
    }

    /// A play request (`$8003`): A = sound id, Y as the caller had it.
    pub fn play(&mut self, a: u8, y: u8) {
        self.call(0x8003, a, y);
    }

    pub fn ram(&self) -> &[u8; 0x800] {
        &self.ram
    }
}

/// One driver call as recorded by Mesen with the APU writes it made.
#[derive(Clone, Debug)]
pub struct Call {
    pub frame: u32,
    pub play: Option<(u8, u8)>,
    pub writes: Vec<(u16, u8)>,
}

/// Reads a `snd_rec.lua` log into driver calls.
pub fn read_log(path: &str) -> Vec<Call> {
    let mut v: Vec<Call> = Vec::new();
    for l in std::fs::read_to_string(path).expect("cannot read the sound log").lines() {
        let p: Vec<&str> = l.split_whitespace().collect();
        match p.first().copied() {
            Some("T") => v.push(Call { frame: p[1].parse().unwrap(), play: None, writes: Vec::new() }),
            Some("P") => v.push(Call { frame: p[1].parse().unwrap(), play: Some((u8::from_str_radix(p[2], 16).unwrap(), u8::from_str_radix(p[3], 16).unwrap())), writes: Vec::new() }),
            Some("W") => {
                // writes outside a driver call (the game's own init writes) are attached to a pseudo call
                if v.last().map_or(true, |c| c.frame != p[1].parse::<u32>().unwrap() && false) {
                    v.push(Call { frame: p[1].parse().unwrap(), play: None, writes: Vec::new() });
                }
                if let Some(c) = v.last_mut() {
                    c.writes.push((u16::from_str_radix(p[2], 16).unwrap(), u8::from_str_radix(p[3], 16).unwrap()));
                }
            }
            _ => {}
        }
    }
    v
}

/// Replays the recorded calls on the reference driver and compares the APU writes of each call.
pub fn verify_ref(rom: &Rom, log: &str) -> bool {
    let calls = read_log(log);
    let mut d = RefDriver::new(rom);
    let mut bad = 0;
    let mut writes = 0;
    for (i, c) in calls.iter().enumerate() {
        d.writes.clear();
        match c.play {
            Some((a, y)) => d.play(a, y),
            None => d.tick(),
        }
        writes += d.writes.len();
        if d.writes != c.writes {
            bad += 1;
            if bad <= 5 {
                println!("call {} frame {} {:?}: expected {:X?} got {:X?}", i, c.frame, c.play, c.writes, d.writes);
            }
        }
    }
    println!("{} driver calls, {} APU writes, {} calls differ", calls.len(), writes, bad);
    bad == 0
}

/// Replays a log with the channel-state lines (`S frame p1 vol len period ...`, recorded with SND_STATE=1) through driver + APU on a cycle timeline (video frame f starts at f*29780.5 CPU cycles,
/// the driver runs `MM4_APU_DELTA` cycles into the frame, the game's `$4017` write is at cycle `MM4_APU_PHI`) and counts how often the model's state differs from Mesen's at a frame end.
pub fn verify_apu(rom: &Rom, log: &str) -> bool {
    let env = |k: &str, d: u64| std::env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(d);
    let (delta, phi) = (env("MM4_APU_DELTA", 0), env("MM4_APU_PHI", 0));
    let text = std::fs::read_to_string(log).expect("cannot read the sound log");
    let mut d = RefDriver::new(rom);
    let mut apu = crate::apu::Apu::new(44100);
    let mut sink: Vec<f32> = Vec::new();
    let mut now = 0u64; // cycles the APU has run
    let mut advance = |apu: &mut crate::apu::Apu, sink: &mut Vec<f32>, now: &mut u64, to: u64| {
        if to > *now {
            apu.run((to - *now) as u32, sink);
            sink.clear();
            *now = to;
        }
    };
    let frame_start = |f: u64| (f as f64 * 29780.5) as u64;
    advance(&mut apu, &mut sink, &mut now, phi);
    apu.write(0x4017, 0x40);
    let (mut diffs, mut checked) = (0, 0);
    let names = ["p1vol", "p1len", "p1per", "p2vol", "p2len", "p2per", "tlen", "tlin", "tper", "nvol", "nlen", "nper"];
    let mut pending: Option<(u64, Vec<(u16, u8)>)> = None;
    let mut call: Option<(bool, u8, u8, u64)> = None; // (is_play, a, y, frame)
    let mut seq: Vec<(u64, String)> = Vec::new();
    for l in text.lines() {
        let p: Vec<&str> = l.split_whitespace().collect();
        match p.first().copied() {
            Some("T") | Some("P") | Some("S") => seq.push((p[1].parse().unwrap(), l.to_string())),
            _ => {}
        }
    }
    let _ = (&mut pending, &mut call);
    for (f, l) in &seq {
        let p: Vec<&str> = l.split_whitespace().collect();
        match p[0] {
            "S" => {
                advance(&mut apu, &mut sink, &mut now, frame_start(f + 1));
                let m: Vec<i32> = p[2..].iter().filter_map(|x| x.parse().ok()).collect();
                let mine = apu.dump();
                checked += 1;
                let bad: Vec<usize> = (0..12).filter(|&i| mine[i] != m[i]).collect();
                if !bad.is_empty() {
                    diffs += 1;
                    if diffs <= 12 || std::env::var("MM4_APU_ALL").is_ok() {
                        println!("frame {}: {}", f, bad.iter().map(|&i| format!("{} mesen {} mine {}", names[i], m[i], mine[i])).collect::<Vec<_>>().join(", "));
                    }
                }
            }
            _ => {
                d.writes.clear();
                if p[0] == "P" {
                    d.play(u8::from_str_radix(p[2], 16).unwrap(), u8::from_str_radix(p[3], 16).unwrap());
                } else {
                    d.tick();
                }
                advance(&mut apu, &mut sink, &mut now, frame_start(*f) + delta);
                for &(a, v) in &d.writes {
                    apu.write(a, v);
                }
            }
        }
    }
    println!("{} frames compared, {} differ", checked, diffs);
    diffs == 0
}
