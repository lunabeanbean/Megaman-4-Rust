//! Compare the Rust player physics with a trace recorded from the real game (harness/ptrace.lua).
//! Each trace row: t pressed held state30 facing Xpage Xpx Xsub Yscreen Ypx Ysub vyHi vyLo hp type (hex, except t).
use crate::anim;
use crate::camera::Camera;
use crate::player::{Buttons, Player};
use crate::stage::Stage;

struct Row {
    t: i32,
    pressed: u8,
    held: u8,
    state: u8,
    facing: u8,
    anim: Option<(u8, u8)>,
    flip: Option<bool>,
    x: i32,
    y: i32,
    vy: i32,
    yscreen: u8,
    cam: i32,
}

fn parse(line: &str) -> Option<Row> {
    let f: Vec<&str> = line.split_whitespace().collect();
    if f.len() < 17 {
        return None;
    }
    let h = |i: usize| i32::from_str_radix(f[i], 16).unwrap();
    Some(Row {
        t: f[0].parse().ok()?,
        pressed: h(1) as u8,
        held: h(2) as u8,
        state: h(3) as u8,
        facing: h(4) as u8,
        flip: if f.len() >= 20 { Some(h(19) & 0x40 != 0) } else { None },
        anim: if f.len() >= 19 { Some((h(17) as u8, h(18) as u8)) } else { None },
        x: (h(5) << 16) | (h(6) << 8) | h(7),
        y: ((h(8) * 240 + h(9)) << 8) | h(10),
        yscreen: h(8) as u8,
        vy: ((((h(11) << 8) | h(12)) as u16) as i16) as i32,
        cam: (h(15) << 8) | h(16),
    })
}

pub fn run(stage: &Stage, path: &str) -> bool {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {}: {}", path, e));
    let rows: Vec<Row> = text.lines().filter_map(parse).collect();
    let first = &rows[0];
    let mut p = Player::new(first.x, first.y, first.vy, first.state, first.facing);
    if let Some(f) = first.flip {
        p.flip = f;
    }
    if let Some((id, frame)) = first.anim {
        p.anim.id = id;
        p.anim.frame = frame;
        if id == anim::SPAWN_WAIT {
            if let Some(k) = rows.iter().position(|r| r.anim.map_or(false, |(i, f)| i != id || f != frame)) {
                p.anim.sync_tick(&stage.anims, k);
            }
        }
    }
    let mut cam = Camera::new(first.cam, (first.x >> 8) as i32, stage);
    let mut ok = 0;
    for r in &rows[1..] {
        if (r.state > 3 && r.state != 0x0A) || r.yscreen != 0 {
            println!("stopped at t={}: the game left the ported states or screen 0 (state ${:02X}, Y screen {}); {} frames matched", r.t, r.state, r.yscreen, ok);
            return true;
        }
        if cam.busy() {
            cam.transition_step(&mut p, &stage.anims, &stage);
        } else {
            p.update(stage, Buttons { held: r.held, pressed: r.pressed });
            cam.update(&mut p, stage);
        }
        if p.x != r.x || p.y != r.y || p.vy != r.vy || p.state != r.state || p.facing != r.facing || cam.x != r.cam
            || r.anim.map_or(false, |(id, frame)| p.anim.id != id || (matches!(id, 0x0D..=0x0F | 0x13 | 0x33) && p.anim.frame != frame))
        {
            println!("MISMATCH at t={} (pressed {:02X} held {:02X}) after {} matching frames", r.t, r.pressed, r.held, ok);
            println!("  game: x={:06X} y={:06X} vy={:04X} state={}", r.x, r.y, r.vy as u16, r.state);
            println!("  port: x={:06X} y={:06X} vy={:04X} state={}", p.x, p.y, p.vy as u16, p.state);
            println!("  facing: game {} port {}   anim (id,frame): game {:?} port ({:02X},{})", r.facing, p.facing, r.anim, p.anim.id, p.anim.frame);
            println!("  camera: game {:04X} port {:04X}", r.cam, cam.x);
            return false;
        }
        ok += 1;
    }
    println!("all {} frames identical to the game", ok);
    true
}
