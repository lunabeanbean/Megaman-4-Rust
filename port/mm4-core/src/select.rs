//! The two screens between a cleared stage and the next one, as pure state (no drawing): the confirm menu at `$8EA3` (bank `$39`,
//! cursor sprite Y in `$0290`) and the 3x3 stage-select grid at `$809D`/`$80B7` (cursor in `$10`/`$11`, grid table `$8870`).
//! Buttons are the game's *newly pressed* byte (`$14`). Verified against Mesen with `SELLOG=1` (harness/sellog_*.txt, `mm4 --verify-select`).
use crate::rom::Rom;

pub const BTN_RIGHT: u8 = 0x01;
pub const BTN_LEFT: u8 = 0x02;
pub const BTN_DOWN: u8 = 0x04;
pub const BTN_UP: u8 = 0x08;
pub const BTN_START: u8 = 0x10;
pub const BTN_SELECT: u8 = 0x20;
pub const BTN_A: u8 = 0x80;

/// What the confirm menu decided this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuResult {
    Stay,
    /// Start/A on the first entry: refill the weapons (`code_398EDE`), `$1F = 0`, go to the stage select.
    ToSelect,
    /// Start/A on the second entry: leave to the caller (title/password code, not ported).
    Leave,
}

/// `code_398E8F..8EC9`: two entries, the cursor sprite sits at Y `$AF` or `$BF`; Down, Up and Select all move it to the other one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfirmMenu {
    pub cursor_y: u8,
}

impl ConfirmMenu {
    pub fn new() -> ConfirmMenu {
        ConfirmMenu { cursor_y: 0xAF }
    }

    pub fn step(&mut self, pressed: u8) -> MenuResult {
        if pressed & (BTN_START | BTN_A) != 0 {
            return if self.cursor_y == 0xBF { MenuResult::Leave } else { MenuResult::ToSelect };
        }
        if pressed & (BTN_DOWN | BTN_UP | BTN_SELECT) != 0 {
            self.cursor_y = self.cursor_y.wrapping_add(0x10);
            if self.cursor_y == 0xCF {
                self.cursor_y = 0xAF;
            }
        }
        MenuResult::Stay
    }
}

impl Default for ConfirmMenu {
    fn default() -> ConfirmMenu {
        ConfirmMenu::new()
    }
}

/// `code_39809D`: 3 columns x 3 rows. `col` = `$10` (0..2), `row` = `$11` (0, 3, 6; the grid index is `col + row`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StageSelect {
    pub col: u8,
    pub row: u8,
    /// Table `$8870`: the stage number behind each grid cell (index `col + row`).
    grid: [u8; 9],
}

impl StageSelect {
    pub fn new(rom: &Rom) -> StageSelect {
        let mut grid = [0u8; 9];
        for (i, g) in grid.iter_mut().enumerate() {
            *g = rom.cpu_byte(0x39, 0x3A, 0x8870 + i as u16);
        }
        StageSelect { col: 1, row: 3, grid }
    }

    /// The stage under the cursor (`$8870[$10+$11]`).
    pub fn hovered(&self) -> u8 {
        self.grid[(self.col + self.row) as usize]
    }

    pub fn cell(&self, col: usize, row: usize) -> u8 {
        self.grid[col + row * 3]
    }

    /// One loop pass (`$80B7`). `a9` = the boss-defeated flags; the centre cell (stage 8) only works once all eight bits are set.
    /// Returns the chosen stage number (`$22`) when Start/A picks a selectable cell.
    pub fn step(&mut self, pressed: u8, a9: u8) -> Option<u8> {
        if pressed & (BTN_START | BTN_A) != 0 {
            let s = self.hovered();
            if s & 0x80 != 0 || (s == 8 && a9 != 0xFF) {
                return None;
            }
            return Some(s);
        }
        if pressed & (BTN_RIGHT | BTN_LEFT) != 0 {
            // horizontal input wins over vertical, Right over Left; both wrap
            if pressed & BTN_RIGHT != 0 {
                self.col = (self.col + 1) % 3;
            } else {
                self.col = if self.col == 0 { 2 } else { self.col - 1 };
            }
        } else if pressed & (BTN_UP | BTN_DOWN) != 0 {
            if pressed & BTN_UP != 0 {
                self.row = if self.row < 3 { 6 } else { self.row - 3 };
            } else {
                self.row = (self.row + 3) % 9;
            }
        }
        None
    }
}

/// Replays a `SELLOG` recording: rows `M <$14> <$0290>` (confirm menu, state before the row's buttons are applied), `G <$14> <$10> <$11> <$A9>`
/// (grid) and a final `P <$22>`. Every row's state must be what the previous row's buttons produced.
pub fn verify(rom: &Rom, path: &str) -> bool {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("cannot read {}: {}", path, e);
            return false;
        }
    };
    let hex = |s: &str| u8::from_str_radix(s, 16).unwrap_or(0);
    let mut menu = ConfirmMenu::new();
    let mut sel = StageSelect::new(rom);
    let (mut last_m, mut last_g): (Option<u8>, Option<(u8, u8)>) = (None, None); // pressed byte of the previous row
    let (mut rows, mut picked) = (0, None);
    for (n, line) in text.lines().enumerate() {
        let f: Vec<&str> = line.split_whitespace().collect();
        match f.first().copied() {
            Some("M") if f.len() == 3 => {
                let (pressed, cur) = (hex(f[1]), hex(f[2]));
                if let Some(p) = last_m {
                    menu.step(p);
                }
                if menu.cursor_y != cur {
                    eprintln!("line {}: confirm cursor {:02X}, game has {:02X}", n + 1, menu.cursor_y, cur);
                    return false;
                }
                last_m = Some(pressed);
                rows += 1;
            }
            Some("G") if f.len() == 5 => {
                let (pressed, c, r, a9) = (hex(f[1]), hex(f[2]), hex(f[3]), hex(f[4]));
                if let Some((p, a)) = last_g {
                    sel.step(p, a);
                } else if let Some(p) = last_m.take() {
                    // the first grid row: the menu's last buttons must have been Start/A on the first entry
                    if menu.step(p) != MenuResult::ToSelect {
                        eprintln!("line {}: grid reached but the menu did not leave on {:02X}", n + 1, p);
                        return false;
                    }
                }
                if (sel.col, sel.row) != (c, r) {
                    eprintln!("line {}: grid cursor ({},{}), game has ({},{})", n + 1, sel.col, sel.row, c, r);
                    return false;
                }
                last_g = Some((pressed, a9));
                rows += 1;
            }
            Some("P") if f.len() == 2 => {
                if let Some((p, a)) = last_g {
                    picked = sel.step(p, a);
                }
                if picked != Some(hex(f[1])) {
                    eprintln!("line {}: picked stage {:?}, game loads {:02X}", n + 1, picked, hex(f[1]));
                    return false;
                }
            }
            _ => {}
        }
    }
    println!("all {} rows identical (picked stage: {:?})", rows, picked);
    true
}
