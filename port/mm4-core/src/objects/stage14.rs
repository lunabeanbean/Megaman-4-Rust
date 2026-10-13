//! Stage 14 (the Wily castle with the teleporter pods): the pod `74` (AI `$B3`, bank `$35` `$A797`).
use super::*;

impl World {
    pub(super) fn run_stage14(&mut self, x: usize, ptr: u16) -> bool {
        if self.f6 != 0x35 {
            return false;
        }
        match ptr {
            0xA797 => self.pod_a797(x),
            _ => return false,
        }
        true
    }

    /// code_3C8B07: the ninth pod in slot 23 (`$17`): position from Mega Man (code_3FF452), then X = `$80`, Y = `$44`, type `$B3`, hitbox `$1C`; the other
    /// fields (sub-pixels, speeds, pointer) stay as the slot had them.
    pub(crate) fn make_final_pod(&mut self, px: i32, py: i32) {
        let flags = (self.o[0].flags & 0x40) | 0x90;
        let c = &mut self.o[0x17];
        c.xpg = (px >> 16) as u8;
        c.ypg = (py >> 16) as u8 as i8 as u8;
        c.flags = flags;
        c.s = [0; 8];
        c.aid = 0x73;
        c.aframe = 0;
        c.atick = 0;
        c.hitbox = 0x1C;
        c.ty = 0xB3;
        c.xpx = 0x80;
        c.ypx = 0x44;
    }

    /// The extra draw pass inside the scene switch (code_3FCE3) also animates the ninth pod, which already exists then.
    pub(crate) fn draw_final_pod(&mut self, cam: Cam) {
        if self.o[0x17].ty == 0xB3 {
            self.finish_object(0x17, cam);
        }
    }

    /// `$A797`: touching the pod (while Mega Man is in state 0) puts him into state `$0F` at the pod's position; in the pod room (page 9) `$AB` = the
    /// index of the pod in the table `$A7E7` (Y, X pairs) and `$0149` the pod's placement-list index.
    fn pod_a797(&mut self, x: usize) {
        self.o[x].flags = 0x90;
        if !self.touches_player(x) || self.p.state != 0 {
            return;
        }
        let (xpx, ypx) = (self.o[x].xpx, self.o[x].ypx);
        self.pod_req = Some((xpx, ypx));
        if self.cam_page == 9 {
            self.c149 = self.o[x].idx;
            let mut ab = 0u8;
            loop {
                let k = 2 * ab as u16;
                if ypx == self.ai_byte(0xA7E7 + k) && xpx == self.ai_byte(0xA7E8 + k) {
                    break;
                }
                ab = ab.wrapping_add(1);
                if ab == 0 {
                    break;
                }
            }
            self.ab = ab;
        }
    }
}
