//! The boss-defeat object of the Cossack/Wily stages (type `$29`, bank `$35` `$A8EE`/`$A927`/`$A948`): all enemies vanish, a burst emitter `3C` plays
//! for 144 frames, then the object disappears and Mega Man starts his victory walk (state `$0B`).
use super::*;

impl World {
    pub(super) fn run_defeat29(&mut self, x: usize, ptr: u16) -> bool {
        if self.f6 != 0x35 {
            return false;
        }
        match ptr {
            0xA8EE => self.defeat29_start(x),
            0xA927 => self.defeat29_wait(x),
            0xA948 => self.defeat29_end(x),
            _ => return false,
        }
        true
    }

    fn defeat29_start(&mut self, x: usize) {
        for y in (8..=0x16).rev() {
            self.kill_slot(y);
        }
        self.p95 = 0;
        self.set_anim(x, 0);
        self.o[x].ptr = 0xA927;
        if let Some(y) = self.find_slot() {
            self.child_here(x, y, 0x62);
            self.o[y].ty = 0x3C;
        }
        self.defeat29_wait(x);
    }

    fn defeat29_wait(&mut self, x: usize) {
        if self.p95 != 0x90 {
            return;
        }
        self.o[x].ptr = 0xA948;
        self.defeat29_end(x);
    }

    fn defeat29_end(&mut self, x: usize) {
        if self.p.state != 0 {
            return;
        }
        self.snd_music(0x0C, 0xA964); // code_35A961
        self.despawn(x);
        self.f1 = 0x88;
        self.scroll69 = 0;
        self.scroll6a = 0;
        self.fd = 0;
        self.f0 = 0;
        self.fa = 0;
        self.player_cmd = Some(0x0B);
        for y in (1..=4).rev() {
            self.kill_slot(y);
        }
    }
}
