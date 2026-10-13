//! Glue between the gameplay picture and the presentation scenes: the palette fade of a frozen gameplay frame (the stage fading out when a scene is entered) and the lists
//! of pictures the window plays for a cutscene.
//! The fade of the real game subtracts `$10` per step from every palette entry (`code_3EC457`); the gameplay picture here is already RGB, so each step darkens the
//! picture by a quarter instead (close, not byte-exact: an exact stage fade needs a palette-indexed stage renderer).
use crate::rom::Rom;
use crate::scene;
use crate::screen::Screen;
use std::rc::Rc;

/// One picture of a playing clip: a finished RGB image (shared by the frames of a fade) or a scene snapshot rendered when it is shown.
#[derive(Clone)]
pub enum Pic {
    Rgb(Rc<Vec<u32>>),
    Scene(Screen),
}

impl Pic {
    pub fn render(&self) -> Vec<u32> {
        match self {
            Pic::Rgb(p) => p.as_ref().clone(),
            Pic::Scene(s) => s.render(),
        }
    }
}

/// The picture darkened for fade step `k` (0 = untouched .. 4 = black).
pub fn shade(base: &[u32], k: usize) -> Vec<u32> {
    if k == 0 {
        return base.to_vec();
    }
    if k >= 4 {
        return vec![0; base.len()];
    }
    let f = 4 - k as u32; // 3, 2, 1 quarters left
    base.iter().map(|&p| (((p >> 16 & 0xFF) * f / 4) << 16) | (((p >> 8 & 0xFF) * f / 4) << 8) | ((p & 0xFF) * f / 4)).collect()
}

/// A frozen gameplay frame leaving the screen the way the stage-end code does: `held` frames unchanged, then four steps of four frames, then `black` black frames.
/// (Mesen: the fade shows from the second frame after it starts, i.e. `held` = 2 for the stage clear and the game over.)
pub fn fade_pictures(base: &[u32], held: usize, black: usize) -> Vec<Pic> {
    let mut v = Vec::new();
    let orig = Rc::new(base.to_vec());
    for _ in 0..held {
        v.push(Pic::Rgb(orig.clone()));
    }
    for k in 1..4 {
        let p = Rc::new(shade(base, k));
        for _ in 0..4 {
            v.push(Pic::Rgb(p.clone()));
        }
    }
    let b = Rc::new(vec![0; base.len()]);
    for _ in 0..black {
        v.push(Pic::Rgb(b.clone()));
    }
    v
}

/// The stage-clear cutscene after the stage's own fade-out `pre` (the caller draws it exactly with the stage's palettes): the weapon-get scene
/// (`weapon_get::run`) up to the password scene. `repeat` = the stage was cleared before: no weapon-get, `pre` already holds the 60 frames of waiting.
pub fn clear_pictures(rom: &Rom, mut pre: Vec<Pic>, stage: usize, repeat: bool, p92: u8, p95: u8) -> Vec<Pic> {
    if repeat {
        return pre;
    }
    let frames = crate::weapon_get::run(rom, stage, p92.wrapping_add(pre.len() as u8), p95);
    pre.extend(render_all(&frames));
    pre
}

/// The same for a clip that continues a screen whose last snapshot was `prev` (its first picture shows the sprites of that snapshot).
pub fn render_all_after(frames: &[Screen], prev: &Screen) -> Vec<Pic> {
    (0..frames.len()).map(|i| Pic::Scene(frames[i].displayed(if i > 0 { &frames[i - 1] } else { prev }))).collect()
}

/// A scene's snapshots as pictures.
pub fn render_all(frames: &[Screen]) -> Vec<Pic> {
    (0..frames.len()).map(|i| Pic::Scene(if i > 0 { frames[i].displayed(&frames[i - 1]) } else { frames[i].clone() })).collect()
}

#[allow(dead_code)]
fn _uses(_: &scene::SceneRunner) {}

