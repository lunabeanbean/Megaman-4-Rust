//! The NES sound chip (2A03 APU) from its register writes: two pulse channels, triangle and noise (the game never uses the DMC channel).
//! Runs per CPU cycle; `run` turns a number of cycles into audio samples (box-filtered down to the output rate, with the console's nonlinear mixer).

const LENGTHS: [u8; 32] = [10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14, 12, 16, 24, 18, 48, 20, 96, 22, 192, 24, 72, 26, 16, 28, 32, 30];
const DUTY: [[u8; 8]; 4] = [[0, 1, 0, 0, 0, 0, 0, 0], [0, 1, 1, 0, 0, 0, 0, 0], [0, 1, 1, 1, 1, 0, 0, 0], [1, 0, 0, 1, 1, 1, 1, 1]];
const NOISE_PERIOD: [u16; 16] = [4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 4068];
const TRI_SEQ: [u8; 32] = [15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];

#[derive(Clone, Default)]
struct Envelope {
    start: bool,
    divider: u8,
    decay: u8,
    period: u8,
    looped: bool,
    constant: bool,
}

impl Envelope {
    fn clock(&mut self) {
        if self.start {
            self.start = false;
            self.decay = 15;
            self.divider = self.period;
        } else if self.divider == 0 {
            self.divider = self.period;
            if self.decay > 0 {
                self.decay -= 1;
            } else if self.looped {
                self.decay = 15;
            }
        } else {
            self.divider -= 1;
        }
    }

    fn volume(&self) -> u8 {
        if self.constant { self.period } else { self.decay }
    }
}

#[derive(Clone, Default)]
struct Pulse {
    on: bool,
    ones_complement: bool,
    duty: u8,
    halt: bool,
    env: Envelope,
    sweep_enable: bool,
    sweep_period: u8,
    sweep_negate: bool,
    sweep_shift: u8,
    sweep_divider: u8,
    sweep_reload: bool,
    timer: u16,
    counter: u16,
    phase: u8,
    length: u8,
}

impl Pulse {
    fn target(&self) -> i32 {
        let delta = (self.timer >> self.sweep_shift) as i32;
        if self.sweep_negate {
            self.timer as i32 - delta - self.ones_complement as i32
        } else {
            self.timer as i32 + delta
        }
    }

    fn muted(&self) -> bool {
        self.timer < 8 || self.target() > 0x7FF
    }

    fn clock_sweep(&mut self) {
        if self.sweep_divider == 0 && self.sweep_enable && self.sweep_shift != 0 && !self.muted() {
            self.timer = self.target() as u16;
        }
        if self.sweep_divider == 0 || self.sweep_reload {
            self.sweep_divider = self.sweep_period;
            self.sweep_reload = false;
        } else {
            self.sweep_divider -= 1;
        }
    }

    fn clock_length(&mut self) {
        if !self.halt && self.length > 0 {
            self.length -= 1;
        }
    }

    fn tick(&mut self) {
        if self.counter == 0 {
            self.counter = self.timer;
            self.phase = (self.phase + 1) & 7;
        } else {
            self.counter -= 1;
        }
    }

    fn output(&self) -> u8 {
        if !self.on || self.length == 0 || self.muted() || DUTY[self.duty as usize][self.phase as usize] == 0 {
            0
        } else {
            self.env.volume()
        }
    }
}

#[derive(Clone, Default)]
struct Triangle {
    on: bool,
    control: bool,
    linear_reload: u8,
    linear: u8,
    linear_flag: bool,
    timer: u16,
    counter: u16,
    phase: u8,
    length: u8,
}

impl Triangle {
    fn clock_linear(&mut self) {
        if self.linear_flag {
            self.linear = self.linear_reload;
        } else if self.linear > 0 {
            self.linear -= 1;
        }
        if !self.control {
            self.linear_flag = false;
        }
    }

    fn clock_length(&mut self) {
        if !self.control && self.length > 0 {
            self.length -= 1;
        }
    }

    fn tick(&mut self) {
        if self.counter == 0 {
            self.counter = self.timer;
            if self.length > 0 && self.linear > 0 {
                self.phase = (self.phase + 1) & 31;
            }
        } else {
            self.counter -= 1;
        }
    }

    fn output(&self) -> u8 {
        // ultrasonic periods are held at their last value like the hardware does (no pop)
        TRI_SEQ[self.phase as usize]
    }
}

#[derive(Clone)]
struct Noise {
    on: bool,
    halt: bool,
    env: Envelope,
    short: bool,
    period: u16,
    counter: u16,
    shift: u16,
    length: u8,
}

impl Noise {
    fn tick(&mut self) {
        if self.counter == 0 {
            self.counter = self.period;
            let bit = if self.short { 6 } else { 1 };
            let fb = (self.shift & 1) ^ ((self.shift >> bit) & 1);
            self.shift = (self.shift >> 1) | (fb << 14);
        } else {
            self.counter -= 1;
        }
    }

    fn output(&self) -> u8 {
        if !self.on || self.length == 0 || self.shift & 1 != 0 { 0 } else { self.env.volume() }
    }
}

#[derive(Clone)]
pub struct Apu {
    p: [Pulse; 2],
    t: Triangle,
    n: Noise,
    five_step: bool,
    inhibit_irq: bool,
    frame_cycle: u32, // CPU cycles into the frame sequence
    odd: bool,
    // sample generation
    acc: f64,
    acc_n: u32,
    cyc_per_sample: f64,
    frac: f64,
    hp1: f64,
    hp2: f64,
    prev_in: f64,
    prev_hp1: f64,
    hp_a1: f64,
    hp_a2: f64,
}

impl Apu {
    pub fn new(sample_rate: u32) -> Apu {
        let mut p0 = Pulse::default();
        p0.ones_complement = true;
        let dt = 1.0 / sample_rate as f64;
        let a = |fc: f64| { let rc = 1.0 / (2.0 * std::f64::consts::PI * fc); rc / (rc + dt) };
        Apu {
            p: [p0, Pulse::default()],
            t: Triangle::default(),
            n: Noise { on: false, halt: false, env: Envelope::default(), short: false, period: 4, counter: 0, shift: 1, length: 0 },
            five_step: false,
            inhibit_irq: false,
            frame_cycle: 0,
            odd: false,
            acc: 0.0,
            acc_n: 0,
            cyc_per_sample: 1_789_773.0 / sample_rate as f64,
            frac: 0.0,
            hp1: 0.0,
            hp2: 0.0,
            prev_in: 0.0,
            prev_hp1: 0.0,
            hp_a1: a(90.0),
            hp_a2: a(440.0),
        }
    }

    pub fn write(&mut self, addr: u16, v: u8) {
        match addr {
            0x4000 | 0x4004 => {
                let p = &mut self.p[((addr - 0x4000) >> 2) as usize];
                p.duty = v >> 6;
                p.halt = v & 0x20 != 0;
                p.env.looped = v & 0x20 != 0;
                p.env.constant = v & 0x10 != 0;
                p.env.period = v & 0x0F;
            }
            0x4001 | 0x4005 => {
                let p = &mut self.p[((addr - 0x4001) >> 2) as usize];
                p.sweep_enable = v & 0x80 != 0;
                p.sweep_period = (v >> 4) & 7;
                p.sweep_negate = v & 8 != 0;
                p.sweep_shift = v & 7;
                p.sweep_reload = true;
            }
            0x4002 | 0x4006 => {
                let p = &mut self.p[((addr - 0x4002) >> 2) as usize];
                p.timer = (p.timer & 0x700) | v as u16;
            }
            0x4003 | 0x4007 => {
                let p = &mut self.p[((addr - 0x4003) >> 2) as usize];
                p.timer = (p.timer & 0xFF) | ((v & 7) as u16) << 8;
                if p.on {
                    p.length = LENGTHS[(v >> 3) as usize];
                }
                p.phase = 0;
                p.env.start = true;
            }
            0x4008 => {
                self.t.control = v & 0x80 != 0;
                self.t.linear_reload = v & 0x7F;
            }
            0x400A => self.t.timer = (self.t.timer & 0x700) | v as u16,
            0x400B => {
                self.t.timer = (self.t.timer & 0xFF) | ((v & 7) as u16) << 8;
                if self.t.on {
                    self.t.length = LENGTHS[(v >> 3) as usize];
                }
                self.t.linear_flag = true;
            }
            0x400C => {
                self.n.halt = v & 0x20 != 0;
                self.n.env.looped = v & 0x20 != 0;
                self.n.env.constant = v & 0x10 != 0;
                self.n.env.period = v & 0x0F;
            }
            0x400E => {
                self.n.short = v & 0x80 != 0;
                self.n.period = NOISE_PERIOD[(v & 15) as usize];
            }
            0x400F => {
                if self.n.on {
                    self.n.length = LENGTHS[(v >> 3) as usize];
                }
                self.n.env.start = true;
            }
            0x4015 => {
                self.p[0].on = v & 1 != 0;
                self.p[1].on = v & 2 != 0;
                self.t.on = v & 4 != 0;
                self.n.on = v & 8 != 0;
                if !self.p[0].on { self.p[0].length = 0; }
                if !self.p[1].on { self.p[1].length = 0; }
                if !self.t.on { self.t.length = 0; }
                if !self.n.on { self.n.length = 0; }
            }
            0x4017 => {
                self.five_step = v & 0x80 != 0;
                self.inhibit_irq = v & 0x40 != 0;
                self.frame_cycle = 0;
                if self.five_step {
                    self.clock_quarter();
                    self.clock_half();
                }
            }
            _ => {}
        }
    }

    fn clock_quarter(&mut self) {
        for p in self.p.iter_mut() {
            p.env.clock();
        }
        self.t.clock_linear();
        self.n.env.clock();
    }

    fn clock_half(&mut self) {
        for p in self.p.iter_mut() {
            p.clock_length();
            p.clock_sweep();
        }
        self.t.clock_length();
        if !self.n.halt && self.n.length > 0 {
            self.n.length -= 1;
        }
    }

    /// One CPU cycle.
    fn cycle(&mut self) {
        self.t.tick();
        if self.odd {
            self.p[0].tick();
            self.p[1].tick();
            self.n.tick();
        }
        self.odd = !self.odd;
        self.frame_cycle += 1;
        // the frame sequencer (CPU cycles: 7457, 14913, 22371, 29829 for the 4-step sequence; 5-step adds 37281)
        match (self.five_step, self.frame_cycle) {
            (_, 7457) | (_, 22371) => self.clock_quarter(),
            (_, 14913) => { self.clock_quarter(); self.clock_half(); }
            (false, 29829) => { self.clock_quarter(); self.clock_half(); self.frame_cycle = 0; }
            (true, 37281) => { self.clock_quarter(); self.clock_half(); self.frame_cycle = 0; }
            _ => {}
        }
    }

    fn mix(&self) -> f64 {
        let p = (self.p[0].output() + self.p[1].output()) as f64;
        let pulse = if p == 0.0 { 0.0 } else { 95.88 / (8128.0 / p + 100.0) };
        let t = self.t.output() as f64;
        let n = self.n.output() as f64;
        let tnd = if t + n == 0.0 { 0.0 } else { 159.79 / (1.0 / (t / 8227.0 + n / 12241.0) + 100.0) };
        pulse + tnd
    }

    /// Runs `cycles` CPU cycles and appends the audio samples that complete in that time to `out` (mono, roughly -0.5..0.5).
    pub fn run(&mut self, cycles: u32, out: &mut Vec<f32>) {
        for _ in 0..cycles {
            self.cycle();
            self.acc += self.mix();
            self.acc_n += 1;
            self.frac += 1.0;
            if self.frac >= self.cyc_per_sample {
                self.frac -= self.cyc_per_sample;
                let x = self.acc / self.acc_n as f64;
                self.acc = 0.0;
                self.acc_n = 0;
                // the console's output stage: two high-pass filters (90 Hz, 440 Hz) remove the DC offset
                let y1 = self.hp_a1 * (self.prev_hp1 + x - self.prev_in);
                self.prev_in = x;
                self.prev_hp1 = y1;
                let y2 = self.hp_a2 * (self.hp2 + y1 - self.hp1);
                self.hp1 = y1;
                self.hp2 = y2;
                out.push(y2 as f32 * 1.6);
            }
        }
    }
}

impl Apu {
    /// Channel state as Mesen reports it (for checking the model): volume, length counter, period for both pulses; length, linear counter, period for the triangle; volume, length, period for the noise.
    pub fn dump(&self) -> [i32; 12] {
        let p = |p: &Pulse| [p.env.volume() as i32, p.length as i32, p.timer as i32];
        let a = p(&self.p[0]);
        let b = p(&self.p[1]);
        [a[0], a[1], a[2], b[0], b[1], b[2], self.t.length as i32, self.t.linear as i32, self.t.timer as i32, self.n.env.volume() as i32, self.n.length as i32, self.n.period as i32 - 1]
    }
}
