//! Mega Man 4 port, step 1: open a window and draw one stage's background from the owner's ROM, with a scrolling camera.
//!
//!   cargo run --release -- [--rom PATH] [--stage N] [--screens N] [--scale 1|2|4] [--ppm OUT.ppm]
//! Arrows or A/D move, Z / Space / K jump, X / J shoot, Esc quits. Sprites come from the ROM (`--boxes` shows hitbox placeholders instead).
//! `--ppm` writes the full stage as an image and exits (used by tools/compare_ppm.js to check against the Node renderer).

mod audio;
use mm4_core::{game, player, rom, select, stage, verify, verify_obj};
use minifb::{Key, Scale, Window, WindowOptions};
use std::io::Write;

fn arg(args: &[String], name: &str) -> Option<String> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let rom_path = arg(&args, "--rom").or_else(find_rom).unwrap_or_else(|| {
        eprintln!("No ROM found. Put your own Mega Man 4 (USA) .nes in this folder or the one above, or give --rom PATH, or set MM4_ROM.");
        std::process::exit(1);
    });
    let stage_id: usize = arg(&args, "--stage").and_then(|s| s.parse().ok()).unwrap_or(2);
    let screens: usize = arg(&args, "--screens").and_then(|s| s.parse().ok()).unwrap_or(0);

    let rom = rom::Rom::load(&rom_path).unwrap_or_else(|e| {
        eprintln!("{}", e);
        std::process::exit(1);
    });
    let st = stage::Stage::load(&rom, stage_id, screens);
    let (w, h, img) = st.render_full();
    let img_idx = st.render_idx().2;
    eprintln!("stage {}: {} screens ({}x{})", st.id, st.screens, w, h);

    if let Some(log) = arg(&args, "--verify-select") {
        std::process::exit(if mm4_core::select::verify(&rom, &log) { 0 } else { 1 });
    }

    if let Some(f) = arg(&args, "--verify-selectscreen") {
        // --verify-selectscreen <selframes file> [--a9 hex]
        let a9 = arg(&args, "--a9").and_then(|v| u8::from_str_radix(&v, 16).ok()).unwrap_or(0);
        std::process::exit(if mm4_core::select_screen::verify(&rom, &f, a9, args.iter().any(|a| a == "--walk")) { 0 } else { 1 });
    }

    if let Some(f) = arg(&args, "--verify-passwordscreen") {
        std::process::exit(if mm4_core::password_screen::verify(&rom, &f, match arg(&args, "--kind").as_deref() { Some("gameover") => mm4_core::password_screen::PasswordKind::GameOver { wily: false }, Some("gameover-wily") => mm4_core::password_screen::PasswordKind::GameOver { wily: true }, _ => mm4_core::password_screen::PasswordKind::StageClear }) { 0 } else { 1 });
    }

    if let Some(f) = arg(&args, "--verify-weaponget") {
        let stage = arg(&args, "--wstage").and_then(|v| v.parse().ok()).unwrap_or(0);
        std::process::exit(if mm4_core::weapon_get::verify(&rom, &f, stage) { 0 } else { 1 });
    }

    if let Some(f) = arg(&args, "--verify-bossintro") {
        let stage = arg(&args, "--wstage").and_then(|v| v.parse().ok()).unwrap_or(2);
        std::process::exit(if mm4_core::boss_intro::verify(&rom, &f, stage) { 0 } else { 1 });
    }

    if let Some(f) = arg(&args, "--render-sound") {
        // --render-sound LOG OUT.wav: replays a recorded driver log through the driver and the APU and writes a mono 44.1 kHz WAV (for listening)
        render_sound(&rom, &f, &arg(&args, "--out").unwrap_or_else(|| "sound.wav".into()));
        return;
    }

    if let Some(f) = arg(&args, "--verify-apu") {
        std::process::exit(if mm4_core::sound_ref::verify_apu(&rom, &f) { 0 } else { 1 });
    }

    if let Some(f) = arg(&args, "--verify-soundref") {
        std::process::exit(if mm4_core::sound_ref::verify_ref(&rom, &f) { 0 } else { 1 });
    }

    if let Some(f) = arg(&args, "--verify-title") {
        std::process::exit(if mm4_core::title_screen::verify(&rom, &f) { 0 } else { 1 });
    }

    if let Some(f) = arg(&args, "--verify-boot") {
        std::process::exit(if mm4_core::boot_screens::verify(&rom, &f) { 0 } else { 1 });
    }

    if let Some(mask) = arg(&args, "--password-dots") {
        // print the grid (row,col of every dot) of the password for a weapon set: bit n of MASK = weapon slot 4+n owned
        let mask = u32::from_str_radix(&mask, 16).unwrap_or(0);
        let mut b0 = [0u8; 16];
        b0[0] = 0x9C;
        for k in 0..10 {
            if mask & (1 << k) != 0 {
                b0[4 + k] = 0x9C;
            }
        }
        let ps = mm4_core::password_screen::PasswordScreen::new(&rom, mm4_core::password_screen::PasswordKind::StageClear, &b0);
        let t = ps.grid_tiles();
        println!("{}", (0..36).filter(|&k| t[k] != 0).map(|k| format!("{},{}", k / 6, k % 6)).collect::<Vec<_>>().join(" "));
        return;
    }

    if let Some(f) = arg(&args, "--render-pf") {
        // --render-pf <recording> --frame N --out x.ppm [--mix]: the picture a recorded frame shows (--mix: sprites and scroll of the frame before, as the PPU displays them)
        let n: u32 = arg(&args, "--frame").and_then(|v| v.parse().ok()).unwrap_or(0);
        let out = arg(&args, "--out").unwrap_or_else(|| "pf.ppm".to_string());
        let frames = mm4_core::pfrec::load(&f).expect("cannot read the recording");
        let i = frames.iter().position(|x| x.n == n).expect("frame not in the recording");
        let mut s = frames[i].to_screen();
        if args.iter().any(|a| a == "--mix") && i > 0 {
            s = s.displayed(&frames[i - 1].to_screen());
        }
        write_ppm(&out, &s.render());
        return;
    }

    if let Some(f) = arg(&args, "--verify-ending") {
        std::process::exit(if mm4_core::ending::verify(&rom, &f) { 0 } else { 1 });
    }

    if let Some(f) = arg(&args, "--verify-story") {
        std::process::exit(if mm4_core::intro_story::verify(&rom, &f) { 0 } else { 1 });
    }

    if let Some(f) = arg(&args, "--verify-wilyflight") {
        std::process::exit(if mm4_core::wily_flight::verify(&rom, &f) { 0 } else { 1 });
    }

    if let Some(f) = arg(&args, "--verify-fortress") {
        let which = arg(&args, "--map").unwrap_or_else(|| "first".to_string());
        std::process::exit(if mm4_core::fortress::verify(&rom, &f, &which) { 0 } else { 1 });
    }

    if let Some(f) = arg(&args, "--verify-passwordentry") {
        std::process::exit(if mm4_core::password_entry::verify(&rom, &f) { 0 } else { 1 });
    }

    if let Some(d) = arg(&args, "--verify-pausescreen") {
        std::process::exit(if mm4_core::screen::verify_pause(&rom, &d.split(',').map(|s| s.to_string()).collect::<Vec<_>>()) { 0 } else { 1 });
    }

    if let Some(log) = arg(&args, "--verify-pause") {
        std::process::exit(if mm4_core::pause::verify(&rom, &log) { 0 } else { 1 });
    }

    if let Some(trace) = arg(&args, "--verify-game") {
        let otrace = trace.replace("ptrace_", "otrace_");
        std::process::exit(if verify_obj::run_game(&rom, st.clone(), &trace, &otrace) { 0 } else { 1 });
    }

    if let Some(trace) = arg(&args, "--verify-obj") {
        let otrace = trace.replace("ptrace_", "otrace_");
        std::process::exit(if verify_obj::run(&rom, &st, &trace, &otrace) { 0 } else { 1 });
    }

    if let Some(trace) = arg(&args, "--verify") {
        std::process::exit(if verify::run(&st, &trace) { 0 } else { 1 });
    }

    if let Some(out) = arg(&args, "--ppm") {
        let mut f = std::fs::File::create(&out).expect("cannot create output file");
        write!(f, "P6\n{} {}\n255\n", w, h).unwrap();
        let mut bytes = Vec::with_capacity(w * h * 3);
        for p in &img {
            bytes.extend_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, *p as u8]);
        }
        f.write_all(&bytes).unwrap();
        eprintln!("wrote {}", out);
        return;
    }

    if let Some(script) = arg(&args, "--chain") {
        // headless run of the whole game flow with a button script (lines: FROM TO BUTTONS, frames FROM..TO, buttons from RLDUABST), pictures written as PPM
        // --chain script.txt --frames N [--shots A-B] --out DIR [--start-stage]   (compare with harness/chain_rec.lua through tools/chaincmp.js)
        let frames: usize = arg(&args, "--frames").and_then(|s| s.parse().ok()).unwrap_or(1000);
        let (sa, sb) = match arg(&args, "--shots") {
            Some(r) => {
                let mut it = r.split('-').map(|v| v.parse::<usize>().unwrap_or(0));
                (it.next().unwrap_or(0), it.next().unwrap_or(0))
            }
            None => (0, frames),
        };
        let dir = arg(&args, "--out").unwrap_or_else(|| "chain_out".to_string());
        std::fs::create_dir_all(&dir).expect("cannot create the output folder");
        let text = std::fs::read_to_string(&script).expect("cannot read the script");
        let mut spans: Vec<(usize, usize, String)> = Vec::new();
        for l in text.lines() {
            let p: Vec<&str> = l.split_whitespace().collect();
            if p.len() == 3 && !l.starts_with('#') {
                spans.push((p[0].parse().unwrap_or(0), p[1].parse().unwrap_or(0), p[2].to_string()));
            }
        }
        let mut flow = mm4_core::flowrun::Flow::new(&rom, st, if args.iter().any(|a| a == "--start-stage") { mm4_core::flowrun::Start::Stage } else { mm4_core::flowrun::Start::PowerOn });
        if std::env::var("MM4_INV").is_ok() {
            flow.g.force_invincible = true;
        }
        if let Ok(tp) = std::env::var("MM4_TP") {
            // checks only: MM4_TP=page,x,y puts Mega Man (standing) at that place of the stage with the camera of its section (needs --start-stage)
            let v: Vec<i32> = tp.split(',').map(|x| x.parse().unwrap_or(0)).collect();
            let g = &mut flow.g;
            g.player = player::Player::new((v[0] << 16) | (v[1] << 8), v[2] << 8, 0, player::STATE_GROUND, player::BTN_RIGHT);
            g.player.life = 0x9C;
            g.force_invincible = true;
            g.camera = mm4_core::camera::Camera::in_section(v[0] << 8, v[1], &g.stage);
            g.start_anim_gfx(g.camera.sect, true);
            let cur = g.stage.byte(0xB300 + v[0] as usize);
            g.world.c9e = cur;
            g.world.c9f = cur;
        }
        if let Some(h) = arg(&args, "--rng") {
            let v: Vec<u8> = (0..4).map(|i| u8::from_str_radix(&h[2 * i..2 * i + 2], 16).unwrap_or(0)).collect();
            flow.rng_override = Some([v[0], v[1], v[2], v[3]]);
        }
        if let Some(c) = arg(&args, "--clear-at").and_then(|v| v.parse::<u64>().ok()) {
            flow.clear_after = Some(c); // frame number in the flow's own count (console frame - lag)
        }
        if let Some(c) = arg(&args, "--gameover-at").and_then(|v| v.parse::<u64>().ok()) {
            flow.gameover_after = Some(c);
        }
        if let Some(v) = arg(&args, "--aa") {
            // --aa FRAME:HEX
            let mut it = v.split(':');
            let f: u64 = it.next().and_then(|x| x.parse().ok()).unwrap_or(0);
            let b = u8::from_str_radix(it.next().unwrap_or("0"), 16).unwrap_or(0);
            flow.aa_at = Some((f, b));
        }
        if let Some(v) = arg(&args, "--cp") {
            // --cp FRAME:N
            let mut it = v.split(':');
            flow.cp_at = Some((it.next().and_then(|x| x.parse().ok()).unwrap_or(0), it.next().and_then(|x| x.parse().ok()).unwrap_or(0)));
        }
        if let Some(v) = arg(&args, "--kill") {
            flow.kill_at = v.parse().ok();
        }
        if let Some(f) = arg(&args, "--lag-log") {
            // --lag-log mesen.log [--lag-from F]: video frames of the Mesen log in which `$95` did not change (no logic step ran) are lag frames of the flow
            let from: usize = arg(&args, "--lag-from").and_then(|s| s.parse().ok()).unwrap_or(0);
            let lagn: usize = arg(&args, "--lag-off").and_then(|s| s.parse().ok()).unwrap_or(6);
            let mut prev: Option<String> = None;
            let mut run: Vec<usize> = Vec::new();
            let mut flush = |run: &mut Vec<usize>, flow: &mut mm4_core::flowrun::Flow| {
                // a long run of unchanged `$95` is a fade or a scene (death, stage load), not a lag frame
                if run.len() <= 3 {
                    for &r in run.iter() {
                        flow.lag_at.insert((r - lagn) as u64);
                    }
                }
                run.clear();
            };
            for l in std::fs::read_to_string(&f).expect("cannot read the log").lines() {
                let p: Vec<&str> = l.split_whitespace().collect();
                if p.len() < 5 || p[0].starts_with('#') {
                    continue;
                }
                let r: usize = p[0].parse().unwrap_or(0);
                if r >= from && prev.as_deref() == Some(p[3]) && r > lagn {
                    run.push(r);
                } else {
                    flush(&mut run, &mut flow);
                }
                prev = Some(p[3].to_string());
            }
            flush(&mut run, &mut flow);
        }
        if let Some(f) = arg(&args, "--pal-log") {
            // --pal-log mesen.log [--pal-off N]: the PPU palette of Mesen row r+N is shown at flow frame r (checks only)
            let off: usize = arg(&args, "--pal-off").and_then(|s| s.parse().ok()).unwrap_or(6);
            for l in std::fs::read_to_string(&f).expect("cannot read the log").lines() {
                let p: Vec<&str> = l.split_whitespace().collect();
                if p.len() < 8 || p[0].starts_with('#') {
                    continue;
                }
                let r: usize = p[0].parse().unwrap_or(0);
                let h = p[p.len() - 1];
                if h.len() == 64 && r >= off {
                    let mut a = [0u8; 32];
                    for i in 0..32 {
                        a[i] = u8::from_str_radix(&h[2 * i..2 * i + 2], 16).unwrap_or(0);
                    }
                    flow.pal_override.insert((r - off) as u64, a);
                }
            }
        }
        let mut wav: Vec<f32> = Vec::new();
        let mut sndlog = String::new();
        let mut log = String::new();
        // the console spends a few frames after power-on before the first scene starts: script frames are the console's, the flow's frame 0 is console frame `lag`
        let lag: usize = arg(&args, "--lag").and_then(|s| s.parse().ok()).unwrap_or(0);
        for n in 0..frames {
            let mut held = 0u8;
            let mut menu = 0u8;
            for (a, b, btn) in &spans {
                if n + lag >= *a && n + lag < *b {
                    for c in btn.chars() {
                        match c {
                            'R' => held |= player::BTN_RIGHT,
                            'L' => held |= player::BTN_LEFT,
                            'D' => held |= player::BTN_DOWN,
                            'U' => held |= player::BTN_UP,
                            'A' => held |= player::BTN_A,
                            'B' => held |= player::BTN_B,
                            'S' => menu |= select::BTN_START,
                            'T' => menu |= select::BTN_SELECT,
                            _ => {}
                        }
                    }
                }
            }
            menu |= held & !player::BTN_B;
            if std::env::var("MM4_INV").ok().and_then(|v| v.parse::<usize>().ok()).is_some_and(|f| n + lag >= f) {
                flow.g.force_invincible = true; // checks only: the recording poked `$3C` every frame from this console frame on
            }
            flow.step(held, menu);
            wav.extend(flow.take_audio());
            for q in &flow.last_reqs {
                if q.music {
                    sndlog.push_str(&format!("{} C {:02X} {:04X}
", n, q.id, q.ret));
                }
                sndlog.push_str(&format!("{} E {:02X} {:04X}
", n, q.id, q.ret));
            }
            if std::env::var("MM4_NTDUMP").ok().and_then(|v| v.parse::<usize>().ok()) == Some(n) {
                // the ring nametable (30 rows x 32 tiles, then the 64 attribute bytes) at this flow frame, for comparing with chain_rec.lua's CH_CHRDUMP
                let mut b: Vec<u8> = flow.g.nt.tile.to_vec();
                b.extend_from_slice(&flow.g.nt.attr);
                std::fs::write(format!("{}/ntrust.bin", dir), b).unwrap();
            }
            let mut h: u32 = 0x811C9DC5;
            for p in &flow.frame {
                for sh in [16u32, 8, 0] {
                    h = (h ^ ((p >> sh) & 255)).wrapping_mul(0x01000193);
                }
            }
            let p = &flow.g.player;
            let pal: String = flow.g.world.pal.ppu.iter().map(|b| format!("{:02X}", b)).collect();
            let camx = format!("{:04X}", flow.g.camera.x & 0xFFFF);
            log.push_str(&format!("{} {} {} {:08x} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {} {} {:06X}
", n, flow.screen_name(), flow.g.stage.id, h, p.anim.id, p.anim.frame, p.anim.tick, p.state, flow.g.world.p95, flow.p92, camx, pal, p.x & 0xFFFFFF));
            if n >= sa && n <= sb {
                write_ppm(&format!("{}/rs_{}.ppm", dir, n), &flow.frame);
            }
        }
        std::fs::write(format!("{}/chain.log", dir), log).unwrap();
        std::fs::write(format!("{}/sound.log", dir), sndlog).unwrap();
        if let Some(p) = arg(&args, "--wav") {
            write_wav(&p, &wav, 44100);
        }
        return;
    }

    if let Some(out) = arg(&args, "--dump-chr") {
        std::fs::write(out, &st.chr).unwrap();
        return;
    }

    if args.iter().any(|a| a == "--sections") {
        // `$B530` (16 section bytes) and `$B300` page cursors of the selected stage
        let b: Vec<String> = (0..16).map(|k| format!("{:02x}", st.byte(0xB530 + k))).collect();
        println!("stage {} sections: {}", st.id, b.join(" "));
        let c: Vec<String> = (0..40).map(|k| format!("{}={:02x}", k, st.byte(0xB300 + k))).collect();
        println!("cursors: {}", c.join(" "));
        for k in 0..16 {
            let id = st.byte(0xB560 + k) as usize;
            let (mut gs, pal) = mm4_core::gfx::GfxStream::start(&rom, id);
            let mut scratch = vec![0u8; 0x2000];
            let mut n = 0;
            while { let before = scratch.clone(); gs.step(&rom, &mut scratch); n += 1; before != scratch && n < 400 } {}
            println!("section {:2}: palette set {:02x} anim gfx {:02x} pal {:02x?} steps {}", k, st.byte(0xB550 + k), id, pal, n - 1);
        }
        if std::env::var("MM4_GFXREC").is_ok() {
            for id in 0..0x60usize {
                let sb = |a: usize| rom.cpu_byte(0x29, 0x29, a as u16);
                let script = ((sb(0xB860 + id) as usize) << 8) | sb(0xB800 + id) as usize;
                let mut i = 0;
                while i < 8 { let v = sb(script + i); i += 1; if v >= 0x80 { break; } }
                let mut s = String::new();
                loop { let b = sb(script + i); if b >= 0x80 { break; } s += &format!(" [bank {:02x} tiles {:02x} src {:02x}00 dst {:02x}00]", b, sb(script + i + 1), sb(script + i + 2), sb(script + i + 3)); i += 4; }
                println!("gfx {:02x}:{}", id, s);
            }
        }
        return;
    }

    if args.iter().any(|a| a == "--pal-anim") {
        // the palette-animation ids of every palette set of every stage (`$B5A0` + 16 * set, 4 bytes; bit 7 = running), for the stage palette task
        for id in 0..15usize {
            let st = stage::Stage::load(&rom, id, 0);
            let mut v = Vec::new();
            for set in 0..10usize {
                let b: Vec<String> = (0..4).map(|k| format!("{:02X}", st.byte(0xB5A0 + set * 20 + k))).collect();
                v.push(b.join(""));
            }
            println!("stage {:2}: {}", id, v.join(" "));
        }
        return;
    }

    if let Some(scr) = arg(&args, "--scene-nt") {
        // print the nametable the scene layout gives for screen N (tiles then attributes, hex), for comparison with a Mesen dump
        let (t, a) = st.nametable(scr.parse().unwrap_or(0));
        println!("NT {}", t.iter().map(|b| format!("{:02X}", b)).collect::<String>());
        println!("AT {}", a.iter().map(|b| format!("{:02X}", b)).collect::<String>());
        return;
    }

    if let Some(out) = arg(&args, "--pause-shot") {
        // render the placeholder pause menu (all weapons owned, cursor on cell 11) to a PPM image
        let mut g = game::Game::new_stage(&rom, st.clone());
        for w in 1..14 {
            g.progress.weapons[w] = 0x80 | (w as u8 * 2);
        }
        g.player.etanks = 3;
        let mut menu = g.pause_menu();
        menu.cursor = 11;
        let mut energy = [0u8; 15];
        energy[0] = 0x9C;
        energy[1..14].copy_from_slice(&g.progress.weapons[1..14]);
        energy[14] = 0x9C;
        let info = mm4_core::screen::PauseInfo { energy, lives: 2, etanks: 3 };
        let ms = stage::Stage::load(&rom, 0x10, 5);
        let mut ps = mm4_core::screen::PauseScreen::open(&rom, &ms, &info, 0);
        ps.update(&rom, menu.cursor, 8);
        let frame = ps.screen.render();
        let mut f = std::fs::File::create(&out).expect("cannot create output file");
        write!(f, "P6
{} {}
255
", stage::SCREEN_W, stage::SCREEN_H).unwrap();
        let mut bytes = Vec::new();
        for p in &frame {
            bytes.extend_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, *p as u8]);
        }
        f.write_all(&bytes).unwrap();
        return;
    }

    if let Some(out) = arg(&args, "--shot") {
        // headless: run a scripted game (hold Right, jump and shoot now and then) and write the last frame as a PPM image
        let frames: usize = arg(&args, "--frames").and_then(|s| s.parse().ok()).unwrap_or(600);
        let mut g = game::Game::new_stage(&rom, st.clone());
        if let Some(w) = arg(&args, "--weapon").and_then(|s| s.parse::<u8>().ok()) {
            g.progress.weapons[w as usize] = 0x9C;
            g.player.weapon = w;
            g.player.w131 = 0x80 | w;
        }
        let mut prev = 0u8;
        for f in 0..frames {
            let mut held = player::BTN_RIGHT;
            if f % 70 < 30 {
                held |= player::BTN_A;
            }
            if f % 25 < 12 {
                held |= player::BTN_B;
            }
            if args.iter().any(|a| a == "--idle") {
                held = 0; // no buttons at all (to compare with a recording that presses nothing)
            }
            let b = player::Buttons { held, pressed: held & !prev };
            prev = held;
            g.world.frame92 = g.world.frame92.wrapping_add(1);
            g.step(b, 1);
        }
        let mut frame = vec![0u32; stage::SCREEN_W * stage::SCREEN_H];
        mm4_core::flowrun::render_play(&mut frame, &img_idx, &st, &mut g);
        let sprites = g.sprites();
        let mut f = std::fs::File::create(&out).expect("cannot create output file");
        write!(f, "P6
{} {}
255
", stage::SCREEN_W, stage::SCREEN_H).unwrap();
        let mut bytes = Vec::new();
        for p in &frame {
            bytes.extend_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, *p as u8]);
        }
        f.write_all(&bytes).unwrap();
        eprintln!("wrote {} (frame {}, player at x={} y={} cam={} anim={:02X}/{}, {} sprites)", out, frames, g.player.px(), g.player.py(), g.camera.x, g.player.anim.id, g.player.anim.frame, sprites.len());
        if std::env::var("MM4_SPRITES").is_ok() {
            for s in &sprites {
                eprintln!("  sprite y={:3} x={:3} tile={:02X} attr={:02X}", s.y, s.x, s.tile, s.attr);
            }
        }
        return;
    }


    if args.iter().any(|a| a == "--audio-test") {
        // opens the sound card and plays the stage music for a few seconds without a window (checks that the device works)
        let Some(a) = audio::AudioOut::start() else {
            println!("no audio device");
            return;
        };
        println!("audio device open at {} Hz", a.rate);
        let mut flow = mm4_core::flowrun::Flow::new(&rom, st, mm4_core::flowrun::Start::Stage);
        flow.set_sample_rate(a.rate);
        let t0 = std::time::Instant::now();
        for n in 0..600u64 {
            flow.step(if n % 40 < 20 { player::BTN_RIGHT } else { 0 }, 0);
            a.push(&flow.take_audio());
            let due = std::time::Duration::from_secs_f64((n + 1) as f64 / 60.0988);
            if let Some(w) = due.checked_sub(t0.elapsed()) {
                std::thread::sleep(w);
            }
        }
        println!("played 600 frames in {:.1} s", t0.elapsed().as_secs_f64());
        return;
    }

    let scale = match arg(&args, "--scale").as_deref() {
        Some("1") => Scale::X1,
        Some("4") => Scale::X4,
        _ => Scale::X2,
    };
    let mut window = Window::new(
        "Mega Man 4 port (arrows move, Z jumps, X shoots, Enter = Start, Esc quits)",
        stage::SCREEN_W,
        stage::SCREEN_H,
        WindowOptions { scale, ..WindowOptions::default() },
    )
    .expect("cannot open window");
    window.set_target_fps(60);

    // without --stage the game starts the way the console does: copyright, CAPCOM, story, title
    let power_on = arg(&args, "--stage").is_none() && !args.iter().any(|a| a == "--all-weapons");
    let mut flow = mm4_core::flowrun::Flow::new(&rom, st, if power_on { mm4_core::flowrun::Start::PowerOn } else { mm4_core::flowrun::Start::Stage });
    let audio = if args.iter().any(|a| a == "--mute") { None } else { audio::AudioOut::start() };
    if let Some(a) = &audio {
        flow.set_sample_rate(a.rate);
    }
    let (mut prev_q, mut prev_e) = (false, false);
    if args.iter().any(|a| a == "--all-weapons") {
        for w in 1..14 {
            flow.g.progress.weapons[w] = 0x9C; // every weapon owned and full (a test shortcut; the real game earns them)
        }
    }
    let mut title = String::new();
    while window.is_open() && !window.is_key_down(Key::Escape) {
        // input: arrows or A/D move, Z / Space / K jump, X / J shoot, Enter = Start, Tab = Select
        let mut held = 0u8;
        if window.is_key_down(Key::Right) || window.is_key_down(Key::D) {
            held |= player::BTN_RIGHT;
        }
        if window.is_key_down(Key::Left) || window.is_key_down(Key::A) {
            held |= player::BTN_LEFT;
        }
        if window.is_key_down(Key::Down) || window.is_key_down(Key::S) {
            held |= player::BTN_DOWN;
        }
        if window.is_key_down(Key::Up) || window.is_key_down(Key::W) {
            held |= player::BTN_UP;
        }
        if window.is_key_down(Key::Z) || window.is_key_down(Key::Space) || window.is_key_down(Key::K) {
            held |= player::BTN_A;
        }
        if window.is_key_down(Key::X) || window.is_key_down(Key::J) {
            held |= player::BTN_B;
        }
        let mut menu_held = held & !player::BTN_B;
        if window.is_key_down(Key::Enter) {
            menu_held |= select::BTN_START;
        }
        if window.is_key_down(Key::Tab) {
            menu_held |= select::BTN_SELECT;
        }
        // Q / E: previous / next owned weapon (a shortcut next to the pause menu)
        let (kq, ke) = (window.is_key_down(Key::Q), window.is_key_down(Key::E));
        if flow.is_play() && ((kq && !prev_q) || (ke && !prev_e)) {
            let g = &mut flow.g;
            let step: i32 = if ke && !prev_e { 1 } else { 13 };
            let mut w = g.player.weapon as i32;
            for _ in 0..14 {
                w = (w + step) % 14;
                if w == 0 || g.progress.weapons[w as usize] & 0x80 != 0 {
                    break;
                }
            }
            g.player.weapon = w as u8;
            g.player.w131 = if w == 0 { 0 } else { 0x80 | w as u8 };
            flow.title = format!("Mega Man 4 port - stage {} - weapon {}", g.stage.id, w);
        }
        prev_q = kq;
        prev_e = ke;

        flow.step(held, menu_held);
        let samples = flow.take_audio();
        if let Some(a) = &audio {
            a.push(&samples);
        }
        if flow.title != title {
            title = flow.title.clone();
            window.set_title(&title);
        }
        if flow.is_play() && args.iter().any(|a| a == "--boxes") {
            draw_boxes(&mut flow.frame, &mut flow.g);
        }
        window.update_with_buffer(&flow.frame, stage::SCREEN_W, stage::SCREEN_H).unwrap();
    }
}

/// Hitbox placeholders over the gameplay picture (--boxes).
fn draw_boxes(frame: &mut Vec<u32>, g: &mut game::Game) {
    let rect = |frame: &mut Vec<u32>, cx0: i32, cy0: i32, hw: i32, hh: i32, fill: u32, edge: u32| {
        for yy in (cy0 - hh)..(cy0 + hh) {
            for xx in (cx0 - hw)..(cx0 + hw) {
                if (0..stage::SCREEN_W as i32).contains(&xx) && (0..stage::SCREEN_H as i32).contains(&yy) {
                    let e = xx == cx0 - hw || xx == cx0 + hw - 1 || yy == cy0 - hh || yy == cy0 + hh - 1;
                    frame[yy as usize * stage::SCREEN_W + xx as usize] = if e { edge } else { fill };
                }
            }
        }
    };
    for (slot, o) in g.world.o.iter().enumerate().skip(1) {
        if o.ty == 0 || o.flags & 0x04 != 0 {
            continue;
        }
        let sx = ((o.xpg as i32) << 8 | o.xpx as i32) - g.camera.x;
        let sy = o.ypx as i32 + o.ypg as i32 * 240;
        let (fill, edge) = match slot {
            1..=4 => (0xFCE060, 0xFFFFFF),
            _ if o.hitbox & 0x80 != 0 => (0xC82800, 0xFC9838),
            _ => (0x58A858, 0xB8F8B8),
        };
        let hb = (o.hitbox & 0x3F) as u16;
        let hw = (g.world.half_width(hb) as i32).clamp(3, 40);
        let hh = (g.world.half_height(hb) as i32).clamp(3, 40);
        rect(frame, sx, sy, hw, hh, fill, edge);
    }
    let (bx, by) = (g.player.px() - g.camera.x, g.player.py());
    let blink = g.player.inv != 0 && g.player.inv & 4 != 0;
    if !blink {
        rect(frame, bx, by, 8, 12, 0x3CBCFC, 0xFFFFFF);
    }
}

fn write_ppm(path: &str, pic: &[u32]) {
    let mut bytes = format!("P6
{} {}
255
", stage::SCREEN_W, stage::SCREEN_H).into_bytes();
    for p in pic {
        bytes.extend_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, *p as u8]);
    }
    std::fs::write(path, bytes).expect("cannot write the picture");
}


/// Replays a snd_rec.lua log (driver calls in order; ticks mark video frames) and writes the mixed audio as a 16-bit mono WAV.
fn render_sound(rom: &mm4_core::rom::Rom, log: &str, out: &str) {
    use mm4_core::{apu::Apu, sound_ref};
    let calls = sound_ref::read_log(log);
    let mut d = sound_ref::RefDriver::new(rom);
    let mut apu = Apu::new(44100);
    let mut samples: Vec<f32> = Vec::new();
    let mut odd = false;
    for c in &calls {
        d.writes.clear();
        match c.play {
            Some((a, y)) => d.play(a, y),
            None => d.tick(),
        }
        for &(a, v) in &d.writes {
            apu.write(a, v);
        }
        if c.play.is_none() {
            apu.run(29780 + odd as u32, &mut samples);
            odd = !odd;
        }
    }
    write_wav(out, &samples, 44100);
    println!("{} samples ({:.1} s) -> {}", samples.len(), samples.len() as f64 / 44100.0, out);
}

/// Mono 16-bit WAV file.
fn write_wav(out: &str, samples: &[f32], rate: u32) {
    let mut w = Vec::new();
    let n = samples.len() as u32 * 2;
    w.extend_from_slice(b"RIFF");
    w.extend_from_slice(&(36 + n).to_le_bytes());
    w.extend_from_slice(b"WAVEfmt ");
    w.extend_from_slice(&16u32.to_le_bytes());
    w.extend_from_slice(&1u16.to_le_bytes());
    w.extend_from_slice(&1u16.to_le_bytes());
    w.extend_from_slice(&rate.to_le_bytes());
    w.extend_from_slice(&(rate * 2).to_le_bytes());
    w.extend_from_slice(&2u16.to_le_bytes());
    w.extend_from_slice(&16u16.to_le_bytes());
    w.extend_from_slice(b"data");
    w.extend_from_slice(&n.to_le_bytes());
    for s in samples {
        w.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32000.0) as i16).to_le_bytes());
    }
    std::fs::write(out, w).unwrap();
}

/// Looks for the player's own ROM (never shipped with the program): `$MM4_ROM`, then a Mega Man 4 `.nes` in the current folder or up to two folders above it.
fn find_rom() -> Option<String> {
    if let Ok(p) = std::env::var("MM4_ROM") {
        return Some(p);
    }
    for dir in [".", "..", "../.."] {
        let Ok(rd) = std::fs::read_dir(dir) else { continue };
        let mut names: Vec<String> = rd.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().to_string()).collect();
        names.sort();
        for n in names {
            let l = n.to_lowercase();
            if l.ends_with(".nes") && (l.contains("mega man 4") || l.contains("megaman4") || l.contains("rockman 4") || l == "mm4.nes") && !l.contains("backup") {
                return Some(format!("{}/{}", dir, n));
            }
        }
    }
    None
}
