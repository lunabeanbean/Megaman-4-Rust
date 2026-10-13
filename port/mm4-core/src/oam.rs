//! Sprite generation: the part of the draw pass (`code_3EDB3A`) that turns objects into OAM entries, plus the HUD meters.
//!
//! Every object has an animation id and a frame index. The animation record (see `anim.rs`) names a sprite id per frame; the sprite id selects a
//! definition `[parts-1, offset-table index, (tile, attribute) per part]` (pointer tables `$B800/$BA00` or `$B900/$BB00`, chosen by bit 7 of the
//! record's first byte) and an offset table `$BC00/$BD00` (one entry per mirror: index + 1 when the object is flipped) holding `(dy, dx)` per part.
//! A part that falls outside 0..255 is dropped without using an OAM slot. At most 63 sprites fit after the reserved sprite 0.
use crate::anim::AnimTable;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sprite {
    pub y: u8,
    pub tile: u8,
    pub attr: u8,
    pub x: u8,
}

/// Parts of one object at screen position (`x13`, `y12`). `ef` is `$EF` (sprites below this line are drawn behind the background),
/// `behind` the object's own priority flag (`$0528` bit 5).
pub fn object_parts(t: &AnimTable, aid: u8, aframe: u8, flip: bool, x13: u8, y12: u8, ef: u8, behind: bool) -> Vec<Sprite> {
    object_parts_sid(t, aid, aframe, None, flip, x13, y12, ef, behind, None, 0)
}

/// Same, but a given sprite id replaces the one the animation frame names (the boss's hit flash uses a fixed silhouette, code_3EDD08).
pub fn object_parts_sid(t: &AnimTable, aid: u8, aframe: u8, sprite: Option<u8>, flip: bool, x13: u8, y12: u8, ef: u8, behind: bool, clip: Option<u8>, maxy: u8) -> Vec<Sprite> {
    let rec = t.record(aid);
    let sid = sprite.unwrap_or_else(|| t.byte(rec.wrapping_add(2 + (aframe & 0x7F) as u16))) as u16;
    let (lo_tab, hi_tab) = if t.byte(rec) & 0x80 == 0 { (0xB800u16, 0xBA00u16) } else { (0xB900u16, 0xBB00u16) };
    let def = t.byte(lo_tab + sid) as u16 | (t.byte(hi_tab + sid) as u16) << 8;
    let parts = t.byte(def) as usize;
    let xi = t.byte(def.wrapping_add(1)) as u16 + flip as u16;
    let off = t.byte(0xBC00 + xi) as u16 | (t.byte(0xBD00 + xi) as u16) << 8; // the game keeps this minus 2 and indexes from 2
    let mut prio = if behind { 0x20u8 } else { 0 };
    let mut out = Vec::new();
    for k in 0..=parts {
        let tile = t.byte(def.wrapping_add(2 + 2 * k as u16));
        let attr = t.byte(def.wrapping_add(3 + 2 * k as u16));
        let dy = t.byte(off.wrapping_add(2 * k as u16));
        let dx = t.byte(off.wrapping_add(1 + 2 * k as u16));
        let (oy, cy) = y12.overflowing_add(dy);
        // the wire (type `$0B`, code_3EDD97): parts at or below the line `$0468` are not drawn (`maxy` != 0)
        let visible_y = if maxy != 0 { if cy { dy >= 0xC0 } else { oy < 0xF8 && oy < maxy } } else if dy & 0x80 != 0 { cy } else { !cy };
        if !visible_y {
            continue;
        }
        if let Some(c) = clip {
            // code_3EDDFB (type $88): sprites at or above `$038F` + 3 (the boss body) are not drawn
            if c.wrapping_add(3) >= oy {
                continue;
            }
        }
        if ef != 0 && ef < oy {
            prio = 0x20;
        }
        let (ox, cx) = x13.overflowing_add(dx);
        let visible_x = if dx & 0x80 != 0 { cx } else { !cx };
        if !visible_x {
            continue;
        }
        out.push(Sprite { y: oy, tile, attr: (attr ^ if flip { 0x40 } else { 0 }) | prio, x: ox });
    }
    out
}

/// One life/energy meter (`code_3EDF00`): seven tiles from y `$48` up to `$18`, tile `$80 + units in that cell` (0..4).
pub fn hud_meter(index: usize, units: u8) -> Vec<Sprite> {
    let attr = [1u8, 0, 2][index];
    let x = [0x18u8, 0x10, 0x28][index];
    let mut left = units;
    let mut y = 0x48u8;
    let mut out = Vec::new();
    loop {
        let (take, rest) = if left >= 4 { (4, left - 4) } else { (left, 0) };
        left = rest;
        out.push(Sprite { y, tile: 0x80 | take, attr, x });
        y = y.wrapping_sub(8);
        if y == 0x10 {
            break;
        }
    }
    out
}

/// Put `parts` into `oam` (at most 63 entries after the reserved sprite 0).
pub fn append(oam: &mut Vec<Sprite>, parts: &[Sprite]) {
    for p in parts {
        if oam.len() >= 63 {
            return;
        }
        oam.push(*p);
    }
}
