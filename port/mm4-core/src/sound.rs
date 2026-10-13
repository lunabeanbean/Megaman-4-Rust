//! Game sound: the request queue the game logic fills (`code_3FF6BE`: 8 slots, a request is dropped when its slot is still taken), the per-frame
//! driver update (NMI trampoline `code_3FFF5C`: every pending request goes to `$8003`, then one `$8000`), and the APU that turns the driver's register
//! writes into samples. The driver itself is `sound_ref::RefDriver` for now (the original bank `$1E` code on a small 6502 core).
use crate::apu::Apu;
use crate::rom::Rom;
use crate::sound_ref::RefDriver;

/// One call of the game's sound-request routine: `code_3FF6BE` (a sound effect, `music = false`) or `code_3FF6BC` (stores `$D9`, then the same: music and stop
/// requests). `ret` is the address the call returns to (CPU address of the next instruction); it identifies the call site in the recordings (`strace_*.txt`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Req {
    pub music: bool,
    pub id: u8,
    pub ret: u16,
}

/// CPU cycles per video frame: 29780.5, alternating 29780 and 29781.
const FRAME_CYCLES: u32 = 29780;

pub struct Sound {
    driver: RefDriver,
    apu: Apu,
    queue: [u8; 8],
    head: usize, // `$DB`: next request the update takes
    tail: usize, // `$DA`: where the next request goes
    odd: bool,
    /// Requests made since the last `frame` (for the verifier and the debug log).
    pub requested: Vec<u8>,
    /// APU register writes of the last `frame`.
    pub writes: Vec<(u16, u8)>,
}

impl Sound {
    pub fn new(rom: &Rom, sample_rate: u32) -> Sound {
        let mut s = Sound { driver: RefDriver::new(rom), apu: Apu::new(sample_rate), queue: [0x88; 8], head: 0, tail: 0, odd: false, requested: Vec::new(), writes: Vec::new() };
        // the game's reset code starts the driver (`$F0`-style clear request) before the first frame
        s.driver.play(0xF0, 0);
        s
    }

    /// `code_3FF6BE`: queue a sound id (dropped when the slot is still taken).
    pub fn request(&mut self, id: u8) {
        self.requested.push(id);
        if self.queue[self.tail] == 0x88 {
            self.queue[self.tail] = id;
            self.tail = (self.tail + 1) & 7;
        }
    }

    /// One video frame of sound: the update runs the queued requests and one driver step, then the APU runs for a frame; the samples are appended to `out`.
    pub fn frame(&mut self, out: &mut Vec<f32>) {
        self.writes.clear();
        while self.queue[self.head] != 0x88 {
            let id = self.queue[self.head];
            self.queue[self.head] = 0x88;
            self.head = (self.head + 1) & 7;
            self.driver.writes.clear();
            self.driver.play(id, if id == 0xF6 { 0x14 } else { 0 });
            self.writes.extend(self.driver.writes.iter().copied());
        }
        self.driver.writes.clear();
        self.driver.tick();
        self.writes.extend(self.driver.writes.iter().copied());
        for &(a, v) in &self.writes {
            self.apu.write(a, v);
        }
        let cycles = FRAME_CYCLES + self.odd as u32;
        self.odd = !self.odd;
        self.apu.run(cycles, out);
        self.requested.clear();
    }
}
