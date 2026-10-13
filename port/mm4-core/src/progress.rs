//! Game progress that survives stages: boss-defeated flags (`$A9`, `$AA`) and the weapon energies (`$B0..`, bit 7 = owned).
//! Rewritten from `code_398C7C` (bank `$39`, called as `JSR $8003` after the stage-clear countdown). Only the *logic* is here; the
//! weapon-get screen, stage-select menu, text and music are presentation and not ported. Stage 2 verified against a Mesen RAM log (`ENDLOG=1`, harness/endlog_r731.txt).
use crate::rom::Rom;

/// `$9C`: weapon owned, full energy (28 units).
const WEAPON_FULL: u8 = 0x9C;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Progress {
    /// `$A9`, `$AA`: one bit per cleared stage (stages 0-7 in `$A9`, 8-15 in `$AA`).
    pub flags: [u8; 2],
    /// `$B0..$BF`: weapon slots; bit 7 set = owned (a pickup writes `$9C` = owned and full, 28 units; the game starts with every slot 0).
    pub weapons: [u8; 16],
}

/// What clearing a stage changed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StageEnd {
    /// The stage had been cleared before: the game skips the weapon-get screen and goes straight to the stage select.
    pub repeat: bool,
    /// Weapon slots (`$B0 + n`) filled by this clear.
    pub unlocked: Vec<usize>,
}

impl Progress {
    pub fn new() -> Progress {
        Progress { flags: [0; 2], weapons: [0; 16] }
    }

    /// code_398EDE (confirm menu -> stage select): every owned weapon is refilled to full energy.
    pub fn refill(&mut self) {
        for w in self.weapons.iter_mut() {
            if *w & 0x80 != 0 {
                *w = WEAPON_FULL;
            }
        }
    }

    pub fn cleared(&self, stage: usize) -> bool {
        self.flags[stage >> 3 & 1] & (1 << (stage & 7)) != 0
    }

    /// code_398C7C for a stage `stage` (`$22`) that was just finished (stages 0-7 are the Robot Masters).
    pub fn stage_cleared(&mut self, rom: &Rom, stage: usize) -> StageEnd {
        let mut end = StageEnd { repeat: false, unlocked: Vec::new() };
        if stage < 8 && self.cleared(stage) {
            end.repeat = true; // code_398C95 is skipped: `$A9 & bit` already set
            return end;
        }
        self.flags[stage >> 3 & 1] |= 1 << (stage & 7);
        if stage < 8 {
            // code_398D72: the Robot Master's weapon (table `$93B1`, indexed by stage) and, after stages 1 and 2, the Rush item
            let slot = rom.cpu_byte(0x39, 0x3A, 0x93B1 + stage as u16) as usize;
            self.weapons[slot] = WEAPON_FULL;
            end.unlocked.push(slot);
            let extra = match stage {
                1 => Some(3),
                2 => Some(2),
                _ => None,
            };
            if let Some(x) = extra {
                self.weapons[x] = WEAPON_FULL; // code_398DC5
                end.unlocked.push(x);
            }
        }
        end
    }
}

impl Default for Progress {
    fn default() -> Progress {
        Progress::new()
    }
}
