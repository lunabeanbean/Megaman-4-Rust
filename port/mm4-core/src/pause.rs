//! The pause / weapon-select menu as pure state (no drawing): the loop at `$3C96D7` (bank `$3C`), cursor `$0138`, cell table `$9B71`.
//! The 16 cells are two columns of 8 (cell 7 is the energy-tank entry, cell 15 does not exist); the cell table maps a cell to its weapon slot (`$B0 + n`).
//! Buttons are the game's *newly pressed* byte (`$14`). Verified against Mesen with `PAUSELOG=1` (`mm4 --verify-pause`).
use crate::rom::Rom;

pub const BTN_RIGHT: u8 = 0x01;
pub const BTN_LEFT: u8 = 0x02;
pub const BTN_DOWN: u8 = 0x04;
pub const BTN_UP: u8 = 0x08;
pub const BTN_START: u8 = 0x10;
pub const BTN_A: u8 = 0x80;

/// What the menu decided this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PauseResult {
    Stay,
    /// Start/A on a weapon cell: `$A0` becomes this weapon (0 = buster), `$0131` shows its meter, Mega Man's state `$30` becomes 0 and the game resumes.
    Weapon(u8),
    /// Start/A on the energy-tank cell: `$0131` is cleared; with an E-tank and a damaged life meter the meter refills (the menu stays open and the cursor returns to cell 0).
    ETank { used: bool },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PauseMenu {
    /// `$0138`
    pub cursor: u8,
    /// Table `$9B71`: the weapon slot behind each cell (`$FF` = none).
    cells: [u8; 16],
    /// `$AF`, the byte an empty cell's lookup reads.
    pub af: u8,
}

impl PauseMenu {
    pub fn new(rom: &Rom, cursor: u8) -> PauseMenu {
        let mut cells = [0u8; 16];
        for (i, c) in cells.iter_mut().enumerate() {
            *c = rom.cpu_byte(0x3C, 0x3D, 0x9B71 + i as u16);
        }
        PauseMenu { cursor, cells, af: 0 }
    }

    /// The weapon slot under cell `i`.
    pub fn cell(&self, i: usize) -> u8 {
        self.cells[i]
    }

    /// One loop pass (`$96D7`). `wen` = the weapon slots `$B0..$BF` (bit 7 = owned), `life` = `$B0`, `etanks` = `$A2`.
    pub fn step(&mut self, pressed: u8, wen: &[u8; 16], life: u8, etanks: u8) -> PauseResult {
        if pressed & (BTN_START | BTN_A) != 0 {
            // code_3C9720
            if self.cursor == 7 {
                if life == 0x9C || etanks == 0 {
                    return PauseResult::ETank { used: false };
                }
                self.cursor = 0;
                return PauseResult::ETank { used: true };
            }
            let w = self.cells[self.cursor as usize & 0x0F];
            if w & 0x80 != 0 {
                return PauseResult::Stay; // an empty cell: BMI code_3C96DD
            }
            return PauseResult::Weapon(w);
        }
        self.move_cursor(pressed, wen);
        PauseResult::Stay
    }

    /// code_3C9922: Left/Right switch the column and search upward for an owned weapon; Up/Down step inside the column, skipping empty cells.
    fn move_cursor(&mut self, pressed: u8, wen: &[u8; 16]) {
        // an empty cell holds `$FF`: `LDA $B0,X` wraps inside the zero page and reads `$AF` (never written, so 0)
        let owned = |cell: u8| {
            let w = self.cells[cell as usize & 0x0F];
            if w == 0xFF { self.af & 0x80 != 0 } else { wen[(w & 0x0F) as usize] & 0x80 != 0 }
        };
        if pressed & (BTN_RIGHT | BTN_LEFT) != 0 && self.cursor != 7 {
            let x = (pressed & 3) >> 1; // 0 = Right, 1 = Left
            let step = [0x08u8, 0xF8][x as usize];
            self.cursor = self.cursor.wrapping_add(step) & 0x0F;
            if self.cursor == 7 {
                return;
            }
            let col = self.cursor & 8;
            let mut y = self.cursor;
            for _ in 0..64 {
                self.cursor = y;
                if owned(y) {
                    return;
                }
                if y == col {
                    y = col | 7;
                    if y == 0x0F {
                        y = 0x0E;
                    }
                } else {
                    y -= 1;
                }
            }
            return; // the game would search forever when a column has no owned weapon; the port just stays on the last cell tried
        }
        if pressed & (BTN_DOWN | BTN_UP) != 0 {
            let x = (pressed & 0x0C) >> 3; // 0 = Down, 1 = Up
            let step = [0x01u8, 0xFF][x as usize];
            loop {
                let col = self.cursor & 8;
                self.cursor = (self.cursor.wrapping_add(step) & 7) | col;
                if self.cursor == 0x0F {
                    continue;
                }
                if self.cursor == 7 || owned(self.cursor) {
                    return;
                }
            }
        }
    }
}

/// Replays a `PAUSELOG` recording: rows `P <$14> <$0138> <$A0> <$0131> <$B0..$BF as 16 hex pairs> <$A2> <$AF>`, one per loop pass at `$96D7`
/// (state before the row's buttons are applied). Every row's cursor must be what the previous row's buttons produced.
pub fn verify(rom: &Rom, path: &str) -> bool {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("cannot read {}: {}", path, e);
            return false;
        }
    };
    let hex = |s: &str| u8::from_str_radix(s, 16).unwrap_or(0);
    let mut menu: Option<PauseMenu> = None;
    let mut prev: Option<(u8, [u8; 16], u8, u8)> = None;
    let (mut rows, mut picks) = (0, 0);
    for (n, line) in text.lines().enumerate() {
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.first() != Some(&"P") || f.len() != 23 {
            continue;
        }
        let (pressed, cursor) = (hex(f[1]), hex(f[2]));
        let (a0, w131) = (hex(f[3]), hex(f[4]));
        let mut wen = [0u8; 16];
        for (i, w) in wen.iter_mut().enumerate() {
            *w = hex(f[5 + i]);
        }
        let etanks = hex(f[21]);
        let m = menu.get_or_insert_with(|| PauseMenu::new(rom, cursor));
        m.af = hex(f[22]);
        if let Some((p, pw, pl, pe)) = prev.take() {
            let r = m.step(p, &pw, pl, pe);
            match r {
                PauseResult::Weapon(w) => {
                    picks += 1;
                    if a0 != w || (w != 0) != (w131 & 0x80 != 0) {
                        eprintln!("line {}: picked weapon {}, game has $A0={:02X} $0131={:02X}", n + 1, w, a0, w131);
                        return false;
                    }
                    // the menu closes: the next row belongs to a new pause
                    menu = None;
                    continue;
                }
                PauseResult::ETank { .. } => {
                    if w131 != 0 {
                        eprintln!("line {}: E-tank cell pressed but $0131 = {:02X}", n + 1, w131);
                        return false;
                    }
                }
                PauseResult::Stay => {}
            }
        }
        let m = menu.as_mut().unwrap();
        if m.cursor != cursor {
            eprintln!("line {}: cursor {}, game has {}", n + 1, m.cursor, cursor);
            return false;
        }
        prev = Some((pressed, wen, wen[0], etanks));
        rows += 1;
    }
    println!("all {} rows identical ({} weapon picks)", rows, picks);
    true
}
