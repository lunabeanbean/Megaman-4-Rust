//! The stage's live palette: the RAM copy at `$0600` (+ target `$0620`), the section palette sets (`code_3ECEE1`), the palette commands of the placement list
//! (`code_388080`, `$84ED/$851D/$854D`) and the palette-animation task (`code_20B800`, four slots `$06F0..`). `ppu` is what the console's palette holds after the NMI uploads.
use crate::rom::Rom;
use crate::stage::Stage;

#[derive(Clone, Debug)]
pub struct PalState {
    /// `$0600..$061F`: background palettes 0-3, then sprite palettes 0-3.
    pub ram: [u8; 32],
    /// `$0620..$063F`: the fade target.
    pub target: [u8; 32],
    /// The PPU palette (what is displayed).
    pub ppu: [u8; 32],
    /// `$06F0+x` (bit 7 = running), `$06F4+x` (step), `$06F8+x` (tick).
    pub anim: [u8; 4],
    pub step: [u8; 4],
    pub tick: [u8; 4],
    /// `$18`: palette upload requested.
    pub f18: bool,
}

impl Default for PalState {
    fn default() -> Self {
        PalState { ram: [0x0F; 32], target: [0x0F; 32], ppu: [0x0F; 32], anim: [0; 4], step: [0; 4], tick: [0; 4], f18: false }
    }
}

impl PalState {
    /// The palette a new stage starts with: set 0 of the stage's own table (`$B590`), the sprite palettes of the stage, shown at once.
    pub fn for_stage(st: &Stage) -> PalState {
        let mut p = PalState::default();
        p.ram[..16].copy_from_slice(&st.bg_palette);
        for k in 0..4 {
            p.ram[16 + k] = st.sprite_palette[0][k];
            p.ram[20 + k] = st.sprite_palette[1][k];
            p.ram[24 + k] = st.sprite_palette[2][k];
            p.ram[28 + k] = st.sprite_palette[3][k];
        }
        p.target = p.ram;
        p.ppu = p.ram;
        p.mirror();
        p
    }

    fn mirror(&mut self) {
        for i in (16..32).step_by(4) {
            self.ppu[i - 16] = self.ppu[i];
        }
    }

    /// `code_3ECEE1`: the palette set of section `sect & 15` (`$B550`) with its four animation ids (`$B5A0`).
    pub fn section_load(&mut self, st: &Stage, sect: u8) {
        let set = st.byte(0xB550 + (sect & 0x0F) as usize) as usize;
        if set != 0 {
            let y = set * 20;
            for x in 0..16usize {
                let v = st.byte(0xB590 + y + x);
                self.target[x] = v;
                self.ram[x] = v;
                if x < 4 {
                    let a = st.byte(0xB5A0 + y + x);
                    if a != self.anim[x] {
                        self.anim[x] = a;
                        self.step[x] = 0;
                        self.tick[x] = 0;
                    }
                }
            }
        }
        self.ram[0x10] = self.ram[0];
        self.target[0x10] = self.ram[0];
        self.f18 = true;
    }

    /// A palette command of the placement list (`code_388080` with a type >= `$C0`).
    pub fn command(&mut self, rom: &Rom, ty: u8) {
        let t = |a: u16| rom.cpu_byte(0x38, 0x39, a);
        if std::env::var("MM4_PALDBG").is_ok() { eprintln!("pal command {:02X}", ty); }
        let y = (ty & 0x3F) as u16;
        let x = t(0x84ED + y) as usize;
        let a = t(0x851D + y);
        if a & 0x80 != 0 {
            if a == self.anim[x] {
                return;
            }
            self.anim[x] = a;
            self.step[x] = 0;
            self.tick[x] = 0;
            return;
        }
        let y = a as u16 * 4;
        self.anim[x] = 0;
        self.f18 = false;
        let x4 = x * 4;
        if x4 == 0 {
            self.ram[0] = t(0x854D + y);
            self.target[0] = self.ram[0];
        }
        for k in 0..3usize {
            let v = t(0x854E + y + k as u16);
            self.ram[x4 + 1 + k] = v;
            self.target[x4 + 1 + k] = v;
        }
        self.f18 = true;
    }

    /// `code_20B800`, then the NMI's upload (`$18`): one call per frame.
    pub fn frame(&mut self, rom: &Rom) {
        self.task(rom);
        if self.f18 {
            self.ppu = self.ram;
            self.mirror();
            self.f18 = false;
        }
    }

    fn task(&mut self, rom: &Rom) {
        let t = |a: u16| rom.cpu_byte(0x20, 0x20, a);
        let mut f61 = self.f18 as u8;
        self.f18 = false;
        for x in (0..4usize).rev() {
            if self.anim[x] & 0x80 == 0 {
                continue;
            }
            let id = (self.anim[x] & 0x7F) as u16;
            let ptr = t(0xBD2C + id) as u16 | (t(0xBD6C + id) as u16) << 8;
            let tick = self.tick[x];
            self.tick[x] = tick.wrapping_add(1);
            if tick != t(ptr + 1) {
                continue;
            }
            self.tick[x] = 0;
            let s = self.step[x];
            self.step[x] = s.wrapping_add(1);
            if s == t(ptr) {
                self.step[x] = 0;
            }
            let entry = t(ptr + 2 + self.step[x] as u16);
            if entry == 0 {
                self.anim[x] = 0;
                continue;
            }
            let row = 0xB8C7u16 + entry as u16 * 3;
            for k in 0..3u16 {
                let v = t(row + k);
                if v & 0x80 != 0 {
                    self.ram[0x10] = v & 0x3F;
                    self.target[0x10] = v & 0x3F;
                    break;
                }
                if v < 0x40 {
                    self.ram[x * 4 + 1 + k as usize] = v;
                    self.target[x * 4 + 1 + k as usize] = v;
                }
            }
            f61 += 1;
            if self.anim[x] == 0x87 {
                for k in 0..4 {
                    self.ram[0x18 + k] = self.ram[8 + k];
                    self.target[0x18 + k] = self.ram[8 + k];
                }
            }
        }
        self.f18 = f61 != 0;
    }
}

impl PalState {
    /// The stage load (`code_3EC607`): palette set `set` (`$48`, from the checkpoint table `$C8A7+3`) into the target, its four animation ids with cleared counters,
    /// Mega Man's sprite colours from `$C853`; the fade-in then brings the target to the screen, so the picture afterwards shows the target itself.
    pub fn stage_start(&mut self, rom: &Rom, st: &Stage, set: u8) {
        if std::env::var("MM4_PALDBG").is_ok() { eprintln!("stage_start set {}", set); }
        let y = set as usize * 20;
        for x in 0..16usize {
            self.target[x] = st.byte(0xB590 + y + x);
            if x < 4 {
                self.anim[x] = st.byte(0xB5A0 + y + x);
                self.step[x] = 0;
                self.tick[x] = 0;
            }
            if x < 8 {
                self.target[16 + x] = rom.cpu_byte(0x3E, 0x3F, 0xC853 + x as u16);
            }
        }
        self.target[0x10] = self.target[0];
        self.ram = self.target;
        self.ppu = self.target;
        self.f18 = false;
    }
}

impl PalState {
    /// One frame of the stage fades (`code_3EC451`): the RAM palette becomes the target lowered by `level`, uploaded; the animation task then runs as usual.
    pub fn fade_frame(&mut self, rom: &Rom, level: u8) {
        self.ram = crate::stage::lower_palette(&self.target, level);
        self.f18 = true;
        self.frame(rom);
    }
}
