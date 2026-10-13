//! One whole game frame: player, objects (AI), camera, spawner, draw pass, random generator, in the order the real game runs them
//! (`code_3EC6C7`, notes/game-loop.md). The window program and the trace verifier both drive this.
use crate::camera::Camera;
use crate::objects::{Cam, Pickup, PlayerView, World};
use crate::oam::{self, Sprite};
use crate::player::{Buttons, Player};
use crate::rom::Rom;
use crate::stage::Stage;

pub struct Game {
    pub stage: Stage,
    pub player: Player,
    pub camera: Camera,
    pub world: World,
    /// The nametable ring of a horizontally scrolling section (see `nametable.rs`).
    pub nt: crate::nametable::NameTable,
    /// Test recordings keep `$3C` non-zero by writing it every frame.
    pub force_invincible: bool,
    /// The section's animated-graphics script (sprite tiles of that section's enemies), three tiles per video frame (`code_3EDA8B` / `code_3EDABB`).
    pub gfx: crate::gfx::GfxStream,
    /// Video frames spent inside this frame's logic beyond the normal one (item refills freeze the game for 4 frames per unit).
    pub extra_frames: usize,
    /// `$1F`: number of checkpoints passed (0..=2).
    pub checkpoint: u8,
    rom: Rom,
    /// Set when the death countdown ran out and the stage was restarted (the window shows a fade in the real game).
    pub restarted: bool,
    /// Set together with `restarted` when the player had no lives left: the real game shows the game-over / continue screens (not ported).
    pub game_over: bool,
    /// Draw passes since Mega Man's last logic update: each one counts his hit-flash timer (`$05B8`, copy of `$3C`) down, so a long
    /// scroll loop leaves him visible.
    draws_since_logic: u32,
    /// The stage-clear countdown (`$0133`, state 8) reached zero: the next frame starts the stage again.
    clear_pending: bool,
    /// The countdown of state `$11` (weapon capsule) reached zero: the stage reloads at checkpoint 1 without a stage clear.
    item_pending: bool,
    /// The row the copy routine of the vertical scroll sends to the PPU in its next iteration.
    row_pending: Option<(u8, u8)>,
    /// The running door drawing: (PPU address, attribute index `$03`, quadrant `$04`) of its next step.
    door: Option<(u16, u8, u8)>,
    /// The OAM shown while a boss-room door is being drawn.
    hold_oam: Option<Vec<Sprite>>,
    /// Checks only (MM4_SNAPS): a picture after every iteration of a scroll loop (the logic frame spends many video frames in it).
    pub snap_pics: bool,
    pub snaps: Vec<Vec<u32>>,
    pub snap_pals: Vec<[u8; 32]>,
    pub snap_nts: Vec<Vec<u8>>,
    pub snap_cams: Vec<(u8, u8, i32)>,
    /// Boss-defeated flags and weapons, updated when a stage is cleared.
    pub progress: crate::progress::Progress,
    /// What the last stage clear changed (read by the shell to show the weapon-get screen).
    pub last_end: Option<crate::progress::StageEnd>,
    /// `$0138`: the pause menu's cursor, kept between pauses.
    pub pause_cursor: u8,
    /// Mega Man's sprite parts of the previous displayed frame (the console shows them one frame late while he teleports in).
    disp_prev: Vec<Sprite>,
    disp_prev_anim: (u8, u8),
    disp_was_spawn: bool,
    disp_prev2: Vec<Sprite>,
    /// The palette state right after the last stage load (a death restart fades the stage in from it).
    pub pal_at_restart: Option<crate::palette::PalState>,
    /// Presentation only: the step that ends the death countdown stops after the restart (the new life's first logic frame comes after the fade-in).
    pub defer_after_restart: bool,
    /// After a death restart the console's scroll register trails the camera by one frame (measured); `prev_cam_x` is the camera of the frame before.
    pub bg_late: bool,
    /// Scroll X of the picture shown last; a lag frame shows it again.
    pub cam_shown: i32,
    pub cam_again: bool,
    pub prev_cam_x: i32,
    /// The sprites `sprites_shown` returned last (a lag frame shows the same OAM again).
    last_shown: Vec<Sprite>,
    /// Sound requests of the logic frame in progress, in the order the game queues them (`sound::Req`); cleared at the start of `step`.
    pub snd_out: Vec<crate::sound::Req>,
    /// Requests the stage load makes (the stage music, code_3EC684); the flow hands them to the sound system when play starts.
    pub start_snd: Vec<crate::sound::Req>,
}

/// Y as the game keeps it ($0390:$0378): screen number and pixel (screens are 240 px tall).
pub fn split_y(y: i32) -> (u8, u8) {
    let px = y >> 8;
    (px.div_euclid(240) as u8, px.rem_euclid(240) as u8)
}

impl Game {
    pub fn new(rom: &Rom, stage: Stage, player: Player, camera: Camera) -> Game {
        let mut player = player;
        if stage.id == 14 {
            player.zones = (0..11).map(|i| (rom.cpu_byte(0x3C, 0x3D, 0x8C28 + i), rom.cpu_byte(0x3C, 0x3D, 0x8C33 + i), rom.cpu_byte(0x3C, 0x3D, 0x8C3E + i))).collect();
        }
        let mut world = World::new(rom);
        world.pal = crate::palette::PalState::for_stage(&stage);
        let mut nt = crate::nametable::NameTable::new();
        nt.fill(&stage, camera.x);
        Game { world, nt, stage, player, camera, force_invincible: false, gfx: Default::default(), extra_frames: 0, checkpoint: 0, rom: rom.clone(), restarted: false, game_over: false, draws_since_logic: 1, clear_pending: false, item_pending: false, row_pending: None, door: None, hold_oam: None, snap_pics: false, snaps: Vec::new(), snap_pals: Vec::new(), snap_nts: Vec::new(), snap_cams: Vec::new(), progress: crate::progress::Progress::new(), last_end: None, pause_cursor: 0, disp_prev: Vec::new(), disp_prev_anim: (0, 0), disp_was_spawn: false, disp_prev2: Vec::new(), last_shown: Vec::new(), snd_out: Vec::new(), start_snd: Vec::new(), pal_at_restart: None, defer_after_restart: false, bg_late: false, prev_cam_x: 0, cam_shown: 0, cam_again: false }
    }

    /// A new game at the start of `stage` (code_3EC541 with no checkpoint): the teleport-in beam at page 0, full life.
    pub fn new_stage(rom: &Rom, stage: Stage) -> Game {
        let life = rom.cpu_byte(0x3E, 0x3F, 0xC688);
        let player = Player::spawn_at(0, &Player::new(0, 0, 0, 0, 0), life, 2, 0);
        let camera = Camera::new(0, 0x80, &stage);
        let mut g = Game::new(rom, stage, player, camera);
        let set = 0;
        g.world.pal.stage_start(rom, &g.stage, set);
        g.world.pal.section_load(&g.stage, 0);
        g.start_anim_gfx(0, true);
        g.world.pal.ppu = g.world.pal.ram;
        g.world.pal.f18 = false;
        let cur = g.stage.byte(0xB300);
        g.world.c9e = cur;
        g.world.c9f = cur;
        if g.stage.id == 8 {
            g.world.ef_override = Some(0x90);
        }
        g.stage_music();
        g
    }

    /// `code_3EDA8B` at the end of `code_3ECEE1`: the animated-graphics script of section `sect` (`$B560`): its sprite palette bytes now, its tiles at once (`instant`: the stage load) or
    /// three per video frame (a section change).
    pub fn start_anim_gfx(&mut self, sect: u8, instant: bool) {
        let id = self.stage.byte(0xB560 + (sect & 0x0F) as usize) as usize;
        let (mut gs, pal) = crate::gfx::GfxStream::start(&self.rom, id);
        for (i, v) in pal.iter().enumerate() {
            self.world.pal.ram[0x18 + i] = *v;
            self.world.pal.target[0x18 + i] = *v;
        }
        if instant {
            gs.finish(&self.rom, &mut self.stage.chr);
        }
        self.gfx = gs;
    }

    /// code_3EC707: Start opens the pause menu in the walking, jumping, climbing and Rush states, but not during a time stop or rain.
    pub fn can_pause(&self) -> bool {
        matches!(self.player.state, 0 | 1 | 3 | 4) && self.world.w13c == 0 && self.world.w13a == 0
    }

    pub fn pause_menu(&self) -> crate::pause::PauseMenu {
        crate::pause::PauseMenu::new(&self.rom, self.pause_cursor)
    }

    /// One pass of the pause-menu loop (code_3C96D7 ff.): moves the cursor or applies the choice. After `Weapon` the game goes on.
    pub fn pause_step(&mut self, menu: &mut crate::pause::PauseMenu, pressed: u8) -> crate::pause::PauseResult {
        use crate::pause::PauseResult;
        let mut wen = self.progress.weapons;
        wen[0] = self.player.life; // `$B0` (the life meter) is the buster's slot and always owned
        let r = menu.step(pressed, &wen, self.player.life, self.player.etanks);
        self.pause_cursor = menu.cursor;
        match r {
            PauseResult::Weapon(w) => {
                self.player.weapon = w;
                self.player.w131 = if w != 0 { 0x80 | w } else { 0 };
                self.player.state = crate::player::STATE_GROUND; // code_3C973A
                self.player.m4c8 = 0;
                self.world.clear_weapon_slots(); // code_3C9840: slots 4..1 are removed
            }
            PauseResult::ETank { used } => {
                self.player.w131 = 0;
                if used {
                    self.player.etanks -= 1;
                    self.player.life = 0x9C; // the real menu refills one unit at a time (8 frames each)
                }
            }
            PauseResult::Stay => {}
        }
        r
    }

    /// code_3EDA8B / code_3EDABB: how many video frames the graphics-set loader needs for set `set` (records `[bank, tiles, src, dst]` after the palette bytes, 3 tiles per frame).
    fn gfx_chunk_frames(&self, set: u8) -> usize {
        let lo = self.rom.cpu_byte(0x3A, 0x29, 0xB800 + set as u16) as u16;
        let hi = self.rom.cpu_byte(0x3A, 0x29, 0xB860 + set as u16) as u16;
        let base = hi << 8 | lo;
        // up to 8 palette bytes, ended early by a byte with bit 7 set (which is skipped)
        let mut k = 8;
        for i in 0..8u16 {
            if self.rom.cpu_byte(0x3A, 0x29, base + i) & 0x80 != 0 {
                k = i + 1;
                break;
            }
        }
        let mut p = base + k;
        let mut frames = 0;
        loop {
            let bank = self.rom.cpu_byte(0x3A, 0x29, p);
            if bank & 0x80 != 0 {
                return frames;
            }
            let tiles = self.rom.cpu_byte(0x3A, 0x29, p + 1) as usize;
            frames += (tiles + 2) / 3;
            p += 4;
        }
    }

    /// Checks only: Mega Man dies with no lives left and the death countdown ends now (the recorder poked state 7, $A1 = 0 and a countdown of 1).
    pub fn force_game_over(&mut self) {
        self.player.lives = 0;
        self.restart();
    }

    /// Checks only: the stage-end code starts now (the recorder poked state 8 and a countdown of 1).
    pub fn force_stage_clear(&mut self) {
        self.clear_pending = true;
    }

    /// The stage-clear countdown of state 8 has just run out: the game moves on to the stage-end code.
    pub fn stage_clear_due(&self) -> bool {
        self.clear_pending
    }

    /// What the object code sees of Mega Man.
    pub fn view(&self) -> PlayerView {
        let (ypg, ypx) = split_y(self.player.y);
        PlayerView {
            xpg: (self.player.x >> 16) as u8,
            xpx: (self.player.x >> 8) as u8,
            ypg,
            ypx,
            state: self.player.state,
            anim: self.player.anim.id,
            falling: self.player.vy < 0,
            life: self.player.life,
            flip: self.player.flip,
        }
    }

    pub fn rom(&self) -> &Rom {
        &self.rom
    }

    pub fn cam(&self) -> Cam {
        Cam { hi: (self.camera.x >> 8) as u8, lo: self.camera.x as u8 }
    }

    /// The sprites of the last frame in OAM order (code_3EDB3A): HUD meters and objects, direction alternating with `$95`.
    pub fn sprites(&mut self) -> Vec<Sprite> {
        if let Some(h) = &self.hold_oam {
            return h.clone(); // the door waits do not run the draw pass: the console keeps showing the OAM of the last one
        }
        self.sprites_impl(false)
    }

    /// The sprites as the console shows them: while Mega Man teleports in (state $0A: the READY text, the falling beam, the landing) his own sprites are the ones of the
    /// frame before (measured against Mesen screenshots; the OAM rows of the recordings are unaffected, so `sprites` stays as it was). Call once per displayed frame.
    pub fn sprites_shown(&mut self) -> Vec<Sprite> {
        let v = self.sprites_impl(true);
        self.last_shown = v.clone();
        v
    }

    /// The same sprites again (a video frame in which no logic step ran).
    pub fn sprites_again(&self) -> Vec<Sprite> {
        self.last_shown.clone()
    }

    fn sprites_impl(&mut self, lag: bool) -> Vec<Sprite> {
        let cam = self.cam();
        let ef = self.world.ef_override.unwrap_or_else(|| self.stage.byte(0xC87B + self.stage.id));
        let mut list: Vec<Vec<Sprite>> = Vec::new(); // per slot, 0..=23
        for _ in 0..24 {
            list.push(Vec::new());
        }
        // Mega Man (slot 0): hidden after death, blinks while invincible
        let p = &self.player;
        let (ypg, ypx) = split_y(p.y);
        let blink = {
            let b = p.inv.saturating_sub(self.draws_since_logic.min(255) as u8);
            b != 0 && (b >> 1) & 1 != 0
        };
        if std::env::var("MM4_DBG8").is_ok() { eprintln!("sprite check ypg {} state {} hidden {} blink {} inv {} dsl {} spr {}", ypg, p.state, p.hidden, blink, p.inv, self.draws_since_logic, self.stage.anims.sprite(p.anim.id, p.anim.frame & 0x7F)); }
        if ypg == 0 && p.state != crate::player::STATE_DEAD && !p.hidden && !blink && self.stage.anims.sprite(p.anim.id, p.anim.frame & 0x7F) != 0 {
            let x13 = ((p.x >> 8) as u8).wrapping_sub(cam.lo);
            list[0] = oam::object_parts(&self.stage.anims, p.anim.id, p.anim.frame, p.flip, x13, ypx, ef, p.behind);
            if std::env::var("MM4_DBG2").is_ok() {
                eprintln!("player sprite: x13={:02X} ypx={:02X} flip={} cam.lo={:02X} parts={:?}", x13, ypx, p.flip, cam.lo, list[0]);
            }
        }
        if lag {
            let spawning = self.player.state == crate::player::STATE_SPAWN;
            let after_spawn = !spawning && std::mem::replace(&mut self.disp_was_spawn, spawning);
            self.disp_was_spawn = spawning;
            let full_lag = spawning || after_spawn;
            let (pid, pframe) = std::mem::replace(&mut self.disp_prev_anim, (self.player.anim.id, self.player.anim.frame));
            if !full_lag && pid == 1 && pid == self.player.anim.id && !self.player.hidden && ypg == 0 && self.stage.anims.sprite(pid, pframe & 0x7F) != 0 {
                let x13 = ((self.player.x >> 8) as u8).wrapping_sub(cam.lo);
                list[0] = oam::object_parts(&self.stage.anims, pid, pframe, self.player.flip, x13, ypx, ef, self.player.behind);
            }
            let cur = std::mem::take(&mut list[0]);
            let older = std::mem::replace(&mut self.disp_prev2, self.disp_prev.clone());
            list[0] = if after_spawn {
                self.disp_prev = cur;
                older // the console still shows the landing pose of the frame before the previous one
            } else if full_lag {
                std::mem::replace(&mut self.disp_prev, cur)
            } else {
                self.disp_prev = cur.clone();
                cur
            };
        }
        let drawn = self.world.drawn.clone();
        for (slot, x13, y12) in drawn {
            let o = self.world.o[slot];
            let t = self.world.sprite_table(o.ty).clone();
            // code_3EDCE7: while the boss's hit-flash timer has bit 1 set it is drawn as the stage's flash silhouette (table $DD5E)
            let flash = slot == self.world.boss_slot as usize && self.world.meter2 & 0x80 != 0 && o.t5b8 != 0 && o.t5b8 & 0x80 == 0 && o.t5b8 & 2 != 0;
            let mut sil = self.rom.cpu_byte(0x3E, 0x3F, (0xDD5E + self.stage.id) as u16);
            if self.stage.id == 14 && cam.hi == 0x0B {
                sil = 0; // code_3EDD06: on page $0B of stage 14 the flash is the palette flash
            }
            let sprite = if flash && sil != 0 { Some(sil) } else { None }; // a zero entry flashes the palette instead (code_3EDE72)
            list[slot] = oam::object_parts_sid(&t, o.aid, o.aframe, sprite, o.flags & 0x40 != 0, x13, y12, ef, o.flags & 0x20 != 0, if o.ty == 0x88 { Some(self.world.o[23].ypx) } else { None }, if o.ty == 0x0B { o.s[0] } else { 0 });
        }
        if std::env::var("MM4_DBG6").is_ok() {
            for (i, l) in list.iter().enumerate() {
                if !l.is_empty() {
                    eprintln!("slot {} ty {:02X} parts {:?}", i, self.world.o[i].ty, l.iter().map(|s| (s.tile, s.attr, s.x, s.y)).collect::<Vec<_>>());
                }
            }
        }
        let mut hud: Vec<Vec<Sprite>> = Vec::new();
        for i in 0..3 {
            hud.push(match i {
                0 => oam::hud_meter(0, self.player.life & 0x7F),
                1 if self.player.w131 & 0x80 != 0 => oam::hud_meter(1, self.player.wen[(self.player.w131 & 0x0F) as usize] & 0x7F),
                2 if self.world.meter2 & 0x80 != 0 => oam::hud_meter(2, self.world.bf & 0x7F), // only $0132 = $8F (`$BF`) is known
                _ => Vec::new(),
            });
        }
        let mut out = Vec::new();
        // the rain starts at OAM entry 0 (the sprite-zero slot, which the recordings do not compare)
        for &(y, tile, attr, x) in self.world.rain.iter().skip(1) {
            out.push(Sprite { y, tile, attr, x });
        }
        // the console displays the OAM in the draw direction of the frame before: the picture reverses the order of the verified list
        if (self.world.p95 & 1 == 0) != lag {
            for h in &hud {
                oam::append(&mut out, h);
            }
            for l in &list {
                oam::append(&mut out, l);
            }
        } else {
            for l in list.iter().rev() {
                oam::append(&mut out, l);
            }
            for h in hud.iter().rev() {
                oam::append(&mut out, h);
            }
        }
        out
    }

    /// A copy for trial runs (the verifier tries different random-task step counts).
    pub fn fork(&self) -> Game {
        Game { nt: self.nt.clone(), stage: self.stage.clone(), player: self.player.clone(), camera: self.camera.clone(), world: self.world.clone(), force_invincible: self.force_invincible, gfx: self.gfx.clone(), extra_frames: self.extra_frames, checkpoint: self.checkpoint, rom: self.rom.clone(), restarted: self.restarted, game_over: self.game_over, draws_since_logic: self.draws_since_logic, clear_pending: self.clear_pending, item_pending: self.item_pending, row_pending: self.row_pending, door: self.door, hold_oam: self.hold_oam.clone(), snap_pics: self.snap_pics, snaps: self.snaps.clone(), snap_pals: self.snap_pals.clone(), snap_nts: self.snap_nts.clone(), snap_cams: self.snap_cams.clone(), progress: self.progress.clone(), last_end: self.last_end.clone(), pause_cursor: self.pause_cursor, disp_prev: self.disp_prev.clone(), disp_prev_anim: self.disp_prev_anim, disp_was_spawn: self.disp_was_spawn, disp_prev2: self.disp_prev2.clone(), last_shown: self.last_shown.clone(), snd_out: self.snd_out.clone(), start_snd: self.start_snd.clone(), pal_at_restart: self.pal_at_restart.clone(), defer_after_restart: self.defer_after_restart, bg_late: self.bg_late, prev_cam_x: self.prev_cam_x, cam_shown: self.cam_shown, cam_again: false }
    }

    /// Run one frame with the given buttons. `rng_steps`: how many times the random task stepped since the previous frame's logic (normally 1;
    /// the task runs after the frame, so its steps are applied at the start of the next one).
    pub fn step(&mut self, b: Buttons, rng_steps: usize) {
        self.snd_out.clear();
        self.gfx.step(&self.rom, &mut self.stage.chr);
        self.prev_cam_x = self.camera.x;
        let sect_before = self.camera.sect;
        self.extra_frames = 0;
        self.restarted = false;
        self.game_over = false;
        if std::mem::take(&mut self.clear_pending) {
            self.last_end = Some(self.progress.stage_cleared(&self.rom, self.stage.id));
            self.restart_after(false); // code_3EC7F6: after the stage-end the same restart as after a death, without losing a life
        }
        if std::mem::take(&mut self.item_pending) {
            self.restart_after(false); // code_3EC7F6 / code_3EC835: state $11 skips the stage-end code and reloads the stage
        }
        if self.force_invincible {
            self.player.inv = 0x3C; // the recordings rewrite `$3C` after every frame
        }
        if std::env::var("MM4_DBG7").is_ok() && self.player.state == crate::player::STATE_DEAD {
            eprintln!("dead timer {} p95 {:02X}", self.player.dead_timer, self.world.p95);
        }
        if self.player.state == crate::player::STATE_DEAD {
            // code_3EC7DD: the death countdown runs between logic frames; at zero the stage restarts and the new life's first frame follows
            self.player.dead_timer = self.player.dead_timer.saturating_sub(1);
            if self.player.dead_timer == 0 {
                self.restart();
                if self.defer_after_restart {
                    self.world.p95 = self.world.p95.wrapping_sub(1); // the console's last countdown pass did not count this one
                    return;
                }
            }
        }
        for _ in 0..rng_steps {
            self.world.rng_step();
            self.world.flush_blocks(&self.stage);
        }
        self.stage.set_scroll_mode(self.world.f0, self.world.w13e);
        self.stage.vsect.set(self.camera.sect & 0xC0 != 0);
        if self.camera.busy() {
            // vertical transition: the game logic is frozen, only the scroll runs and the draw pass keeps ticking animations
            self.camera.transition_step(&mut self.player, &self.stage.anims, &self.stage);
            self.draw_door_step();
            self.scroll_iteration_end();
            self.apply_camera_events();
            if std::env::var("MM4_DBGP").is_ok() { eprintln!("busy step: p95 {:02X} door_wait {} kind {} x {:04X}", self.world.p95, self.camera.door_wait, self.camera.door_kind, self.camera.x); }
            if !std::mem::take(&mut self.camera.door_frame) {
                self.draws_since_logic += 1;
                if self.camera.door_wait == 0 {
                    self.hold_oam = None;
                }
            }
            if self.snap_pics {
                let st = self.stage.clone();
                let img = Vec::new();
                let mut f = vec![0u32; crate::stage::SCREEN_W * crate::stage::SCREEN_H];
                let pal = self.world.pal.ppu;
                let list = self.sprites();
                crate::flowrun::render_play_with_list(&mut f, &img, &st, self, &pal, list);
                self.snaps.push(f);
                self.snap_pals.push(pal);
                self.snap_cams.push((self.camera.c24, self.camera.c2a, self.camera.x));
                let mut nb = self.nt.tile.to_vec();
                nb.extend_from_slice(&self.nt.attr);
                self.snap_nts.push(nb);
            }
            if !self.camera.busy() {
                self.hold_oam = None;
                // the scroll loop lives inside the camera routine: the spawner (next in the frame) only runs once it is over
                let cam = self.cam();
                self.world.set_view(self.view());
                self.world.spawn_phase(&self.stage, cam, self.camera.dir);
            }
        } else {
            let pre_oam = self.sprites_impl(false);
            self.hold_oam = None;
            self.draws_since_logic = 1;
            self.world.loop_steps = 0;
            self.player.env = self.world.obj_env(self.stage.id as u8);
            self.player.cam_page = (self.camera.x >> 8) as u8;
            self.player.ac = self.world.ac;
            self.stage.ac_full.set(self.world.ac == 0xFF);
            self.player.wen = self.progress.weapons;
            self.player.update_logic(&self.stage, b);
            self.collect_snd();
            if let Some((c, up)) = self.player.pal_mm.take() {
                // the charge flash and the palette restore after a release (code_3C8057 / 3C80B2 / 3C8D13): colours 1-3 of Mega Man's sprite palette
                for (i, v) in c.iter().enumerate() {
                    self.world.pal.ram[0x11 + i] = *v;
                    self.world.pal.target[0x11 + i] = *v;
                }
                self.world.pal.f18 |= up;
            }
            self.progress.weapons = self.player.wen;
            self.world.weapon = self.player.weapon;
            self.world.wen_cur = self.player.wen[self.player.weapon as usize];
            if std::mem::take(&mut self.player.marine_exit) {
                // code_3C8539: the Rush Marine's graphics are streamed back, one chunk per video frame; each of those frames is a full frame of the main loop
                let n = self.gfx_chunk_frames(0x4A);
                for _ in 0..n {
                    let cam = self.cam();
                    self.world.draw_phase(cam);
                    self.player.anim.advance(&self.stage.anims);
                    self.world.rng_step();
                    self.world.flush_blocks(&self.stage);
                    self.world.loop_steps += 1;
                    self.draws_since_logic += 1;
                    self.extra_frames += 1;
                }
            }
            if std::mem::take(&mut self.player.pod_switch) {
                self.pod_scene_switch();
            }
            self.world.invincible = self.player.inv != 0 || self.force_invincible;
            // code_3A8003: touching spikes kills at once (unless he is hurt, dying or invincible)
            if self.player.spike.get() && self.player.state < 6 && !self.world.invincible {
                self.player.kill_by_hazard();
            }
            let ev = std::mem::take(&mut self.player.events);
            self.world.apply_player_events(&ev);
            let view = self.view();
            self.world.cam_page = (self.camera.x >> 8) as u8;
            self.world.pressed = b.pressed;
            self.world.held = b.held;
            self.world.cam_now = self.cam();
            let sid = self.stage.id as u8;
            let player = &mut self.player;
            let stage = &self.stage;
            let camera = &mut self.camera;
            self.world.ai_phase(&self.stage, view, &mut |w, dmg, enemy| {
                if dmg == 0xFF {
                    // an object pushes Mega Man sideways (code_3FF37C)
                    let mut raw_y = false; // SetY stored a raw pixel Y >= $F0 on his own page (the game keeps the page byte apart)
                    for op in std::mem::take(&mut w.player_ops) {
                        player.env = w.obj_env(sid);
                        match op {
                            crate::objects::PlayerOp::Ground(ypx) => {
                                if raw_y && ypx >= 0xF0 {
                                    continue; // the ground probe starts below the screen and finds nothing
                                }
                                player.ride_ground(stage, ypx);
                                let (pg, px) = split_y(player.y);
                                w.set_player_y(pg, px);
                            }
                            crate::objects::PlayerOp::Freeze(n) => player.ad = n,
                            crate::objects::PlayerOp::Pay => player.spend_energy(),
                            crate::objects::PlayerOp::Mount { xpg, xpx, ypg, ypx } => {
                                player.state = crate::player::STATE_MARINE;
                                player.m510 = 0x3C;
                                player.shoot = 0;
                                player.shoot_timer = 0;
                                player.anim.set(0x27);
                                player.x = ((xpg as i32) << 16) | ((xpx as i32) << 8) | (player.x & 0xFF);
                                player.y = ((ypg as i8 as i32 * 240 + ypx as i32) << 8) | (player.y & 0xFF);
                                w.set_player_x(xpg, xpx);
                                w.set_player_y(ypg, ypx);
                                camera.last_px = ((xpg as i32) << 8) | xpx as i32; // `STA $27` / `STA $28`: the camera's reference X
                            }
                            crate::objects::PlayerOp::Bounce => {
                                player.vy = 0x06EE;
                                player.m498 = true;
                                player.spend_energy(); // code_3FFD75
                            }
                            crate::objects::PlayerOp::Crush => player.crushed_by_press(),
                            crate::objects::PlayerOp::SetX(px) => {
                                player.x = (player.x & !0xFF00) | ((px as i32) << 8);
                                w.set_player_x((player.x >> 16) as u8, px);
                            }
                            crate::objects::PlayerOp::SetY(ypx) => {
                                let (pg, _) = split_y(player.y);
                                player.y = ((pg as i8 as i32 * 240 + ypx as i32) << 8) | (player.y & 0xFF);
                                raw_y = ypx >= 0xF0;
                                player.raw_below = raw_y;
                                w.set_player_y(pg, ypx);
                            }
                            crate::objects::PlayerOp::Carry { xsub, xvs, xvp, dir } => {
                                if let Some(s) = xsub {
                                    player.x = (player.x & !0xFF) | s as i32;
                                }
                                player.push_h(stage, xvs, xvp, dir);
                                w.set_player_x((player.x >> 16) as u8, (player.x >> 8) as u8);
                            }
                        }
                    }
                    if let Some((xvs, xvp, dir)) = w.push_req.take() {
                        player.env = w.obj_env(sid);
                        player.push_h(stage, xvs, xvp, dir);
                        w.set_player_x((player.x >> 16) as u8, (player.x >> 8) as u8);
                    }
                    return;
                }
                player.events.clear();
                player.env = w.obj_env(sid);
                let took = player.take_hit(dmg, enemy);
                w.snd.extend(std::mem::take(&mut player.snd));
                if took {
                    let ev = std::mem::take(&mut player.events);
                    w.apply_player_events(&ev);
                }
            });
            self.collect_snd();
            self.progress.weapons = self.player.wen; // an object (the Rush Coil) may have paid energy
            if let Some(s) = self.world.player_cmd.take() {
                // the boss-room object freezes / releases Mega Man; the boss-defeat object starts his victory walk
                if s == crate::player::STATE_VICTORY {
                    self.player.start_victory();
                } else {
                    self.player.state = s;
                    if s == crate::player::STATE_AUTOWALK {
                        self.player.shoot = 0; // `$32` = `$33` = 0
                    }
                }
            }
            if let Some((xpx, ypx)) = self.world.pod_req.take() {
                // code_35A797: Mega Man is pulled into the pod (state $0F, beam animation $32, no shooting)
                self.player.state = crate::player::STATE_POD;
                self.player.shoot = 0;
                self.player.x = (self.player.x & !0xFF00) | ((xpx as i32) << 8);
                let (pg, _) = split_y(self.player.y);
                self.player.y = ((pg as i8 as i32 * 240 + ypx as i32) << 8) | (self.player.y & 0xFF);
                self.player.anim.set(0x32);
            }
            if let Some(f) = self.world.sect_req.take() {
                self.camera.sect = (self.camera.sect & 0x0F) | f; // the stage-9 boss sets bit 7 of `$2B`
            }
            // code_3A805A: crushed inside solid ground
            self.player.env = self.world.obj_env(self.stage.id as u8); // the check runs after the AI loop: it sees $013E/$F0 as the objects left them
            // code_3A805A: stage 4 has no crush check
            if self.player.state < 6 && !self.world.invincible && self.stage.id != 4 && self.player.crushed(&self.stage, self.world.f0) {
                self.player.kill_by_hazard();
                let ev = std::mem::take(&mut self.player.events);
                self.world.apply_player_events(&ev);
            }
            // the refill itself (draw pass + 4 video frames per unit) already ran inside the AI loop (World::life_refill)
            let add = self.world.life_add;
            if add != 0 {
                self.player.life = 0x80 | ((self.player.life & 0x1F) + add);
                self.extra_frames += 4 * add as usize;
            }
            let wadd = self.world.wen_add;
            if wadd != 0 {
                let e = &mut self.player.wen[self.player.weapon as usize];
                *e = 0x80 | ((*e & 0x1F) + wadd);
                self.progress.weapons = self.player.wen;
                self.extra_frames += 4 * wadd as usize;
            }
            if let Some(pk) = self.world.pickup {
                match pk {
                    Pickup::Life(_) => {}
                    Pickup::Weapon(_) => {}
                    Pickup::OneUp => self.player.lives = (self.player.lives + 1).min(9),
                    Pickup::ETank => self.player.etanks = (self.player.etanks + 1).min(9),
                    Pickup::Capsule(slot) => {
                        if let Some(w) = self.progress.weapons.get_mut(slot as usize) {
                            *w = 0x9C;
                        }
                        self.player.start_item();
                    }
                }
            }
            self.camera.frame95 = self.world.p95;
            self.camera.update(&mut self.player, &self.stage);
            self.collect_snd();
            if self.camera.door_step.is_some() {
                self.hold_oam = Some(pre_oam);
            }
            self.draw_door_step();
            if let Some((pg, col)) = self.camera.col_build {
                if self.camera.sect & 0xC0 == 0 {
                    if std::env::var("MM4_NTDBG").is_ok() { eprintln!("build pg {} col {} camera x {:04X}", pg, col, self.camera.x); }
                    self.nt.build_column(&self.stage, pg, col);
                }
            }
            if let Some((pg, col)) = self.camera.col_build.take() {
                // the camera built a tile column (`$19` set): the column renderer of this frame does nothing
                self.world.col_block = true;
                self.world.column_scribble(&self.stage, pg, col, self.camera.sect & 0xC0 != 0);
            }
            self.scroll_iteration_end();
            self.apply_camera_events();
            if !self.camera.busy() {
                let cam = self.cam();
                self.world.set_view(self.view());
                self.world.spawn_phase(&self.stage, cam, self.camera.dir);
            }
            // the draw pass ticks Mega Man's animation at the end of the frame (not while he is hidden: `$0528` bit 2 after death)
            if self.player.state != crate::player::STATE_DEAD {
                self.player_draw_pass();
            }
            // code_3EC7DD: state 8 runs the same countdown as death (`$0133` wraps when it was 0)
            if self.player.state == crate::player::STATE_LEAVE || self.player.state == crate::player::STATE_ITEM {
                self.player.dead_timer = self.player.dead_timer.wrapping_sub(1);
                let due = self.player.dead_timer == 0;
                if self.player.state == crate::player::STATE_LEAVE {
                    self.clear_pending = due;
                } else {
                    self.item_pending = due;
                }
            }
            if std::mem::take(&mut self.player.set_checkpoint1) {
                self.checkpoint = 1;
            }
            // code_3EC788: passing the checkpoint page
            let page = (self.player.x >> 16) as u8;
            let i = self.stage.id * 2 + self.checkpoint as usize;
            if self.checkpoint < 2 && page == self.rom.cpu_byte(0x3E, 0x3F, (0xC88B + i) as u16) {
                self.checkpoint += 1;
            }
        }
        // code_3EDC14: below the screen means death by falling
        let (ypg, _) = split_y(self.player.y);
        if ypg != 0 && ypg < 0x80 && !self.player.raw_below && self.player.state != crate::player::STATE_DEAD && !self.camera.busy() {
            self.player.state = crate::player::STATE_DEAD;
            self.player.dead_timer = 300;
            self.collect_snd();
            self.world.d9 = 0xF0;
            self.snd_out.push(crate::sound::Req { music: true, id: 0xF0, ret: 0xDC2A }); // code_3EDC27
            self.snd_out.push(crate::sound::Req { music: false, id: 0x25, ret: 0xDC2F });
        }
        self.draw_bars();
        self.apply_ppu();
        let cam = self.cam();
        self.world.draw_phase(cam);
        self.world.apply_col_steps();
        if std::env::var("MM4_DBG3").is_ok() && self.player.state == crate::player::STATE_SPAWN {
            eprintln!("spawn: y={:06X} vy={:04X} anim=({:02X},{},{})", self.player.y, self.player.vy, self.player.anim.id, self.player.anim.frame, self.player.anim.tick);
        }
        if std::env::var("MM4_DBG5").is_ok() && self.player.state == crate::player::STATE_SPAWN {
            let live: Vec<String> = (1..24).filter(|&s| self.world.o[s].ty != 0).map(|s| format!("{}:{:02X}@{:02X}:{:02X}", s, self.world.o[s].ty, self.world.o[s].xpg, self.world.o[s].xpx)).collect();
            eprintln!("objs {:?} c9e={} c9f={} cam={:X} dir={}", live, self.world.c9e, self.world.c9f, self.camera.x, self.camera.dir);
        }
        // the palette-animation task (code_20B800) is the second thread: it runs once when the logic frame yields
        let rom = self.rom.clone();
        self.world.pal.frame(&rom);
        self.collect_snd();
        if self.camera.sect != sect_before && std::env::var("MM4_DBG").is_ok() {
            eprintln!("sect {:02X} -> {:02X} at camera x {:04X} py {:04X} px {:06X}", sect_before, self.camera.sect, self.camera.x, self.player.y >> 8, self.player.x);
        }
    }

    /// What a section change does to the rest of the game: wipes the objects, reloads the spawner cursors, resets `$95`.
    fn apply_camera_events(&mut self) {
        if std::mem::take(&mut self.camera.clear_objects) {
            self.world.clear_slots();
            self.world.section_reset();
            self.player.m4c8 = 0; // code_3ECF54
            self.world.pal.section_load(&self.stage, self.camera.sect);
            self.start_anim_gfx(self.camera.sect, false);
            // no nametable fill here: the chain scroll builds the new page column by column (the old columns scroll out)
            self.stage.reset_blocks();
            self.stage.set_scroll_mode(0, 0);
            // code_3ECF61 (stage 3): from page $0C on, and in section 3, the sprite priority line `$EF` is cleared
            if self.stage.id == 3 && ((self.camera.x >> 8) >= 0x0C || self.camera.sect & 0x0F == 3) {
                self.world.ef_override = Some(0);
            }
            if self.stage.id == 8 {
                self.world.ef_override = Some(0); // code_3ECF77: stage 8 clears the priority line at every section change
            }
            if self.stage.id == 1 {
                // code_3ECF93: stage 1 takes the priority line from the table at $CFA1 by section
                self.world.ef_override = Some(self.rom.cpu_byte(0x3E, 0x3F, 0xCFA1 + (self.camera.sect & 0x0F) as u16));
            }
        }
        if let Some(c) = self.camera.cursor.take() {
            self.world.c9e = c;
            self.world.c9f = c;
        }
        if let Some(v) = self.camera.p95_reset.take() {
            self.world.ef_override = Some(0); // code_3ECB59: the boss-room section clears the sprite priority line
            self.player.behind = false;
            self.world.p95 = v;
        }
    }

    /// code_3EC541: after death the stage restarts at the last checkpoint (or from the start when no lives are left).
    pub fn restart(&mut self) {
        self.restart_after(true);
    }

    fn restart_after(&mut self, lost_life: bool) {
        let lives = self.player.lives;
        self.game_over = lost_life && lives == 0;
        let (cp, lives) = if !lost_life { (self.checkpoint, lives) } else if lives == 0 { (0, 2) } else { (self.checkpoint, lives - 1) };
        let sid = self.stage.id;
        let tbl = |i: usize| self.rom.cpu_byte(0x3E, 0x3F, (0xC8A7 + cp as usize * 4 + sid * 8 + i) as u16);
        let (page, sect) = if cp == 0 { (0, 0) } else { (tbl(0) as i32, tbl(1)) };
        self.world.clear_slots();
        self.world.section_reset(); // the stage load runs code_3ECEE1
        self.player.m4c8 = 0;
        self.stage.reset_blocks();
        self.stage.set_scroll_mode(0, 0);
        // code_3EC6D2: the stage load sets `$EF` from the table; stage 3 clears it once a checkpoint has been passed (`$1F != 0`)
        self.world.ef_override = if self.stage.id == 3 && cp != 0 {
            Some(0)
        } else if self.stage.id == 1 {
            Some(self.rom.cpu_byte(0x3E, 0x3F, 0xCFA1 + (sect & 0x0F) as u16)) // code_3EC6E3
        } else if self.stage.id == 8 {
            Some(if page < 6 { 0x90 } else { 0 }) // code_3EC6F3: stage 8 draws sprites below line $90 behind the background until page 6
        } else {
            None
        };
        self.world.meter2 = 0; // the boss meter is gone after a restart
        self.world.bf = 0;
        let life = self.rom.cpu_byte(0x3E, 0x3F, 0xC688);
        self.player = Player::spawn_at(page, &self.player, life, lives, self.player.etanks);
        self.camera = Camera::at_section(page << 8, (page << 8) | 0x80, &self.stage, sect);
        let set = if cp == 0 { 0 } else { tbl(3) };
        self.world.pal.stage_start(&self.rom, &self.stage, set);
        self.world.pal.section_load(&self.stage, sect);
        self.start_anim_gfx(sect, true);
        self.nt.fill(&self.stage, self.camera.x);
        self.pal_at_restart = Some(self.world.pal.clone());
        let cur = self.stage.byte(0xB300 + page as usize);
        self.world.c9e = cur;
        self.world.c9f = cur;
        self.world.c9e = cur;
        self.world.c9f = cur;
        if self.stage.id == 14 && page == 9 && self.world.ac == 0xFF {
            // code_3EC697 (stage load): every robot master is down, so the ninth pod stands in the pod room again
            self.world.make_final_pod(self.player.x, self.player.y);
        }
        self.checkpoint = cp;
        if cp == 0 {
            self.world.defeated = [0; 32];
        }
        self.stage_music();
        self.restarted = true;
    }
}

impl Game {
    /// The PPU buffer records the objects queued in this logic frame go into the nametable (a record's bytes run along a row).
    fn apply_ppu(&mut self) {
        for (i, v) in std::mem::take(&mut self.world.shadow_or) {
            self.nt.shadow[i & 63] |= v;
        }
        let vertical = self.camera.sect & 0xC0 != 0;
        for (addr, bytes) in std::mem::take(&mut self.world.ppu_rec) {
            let nt = (addr >> 10) & 3;
            if nt >= 2 || (nt == 1 && vertical) {
                continue; // the other nametable pair is not shown
            }
            let base = (addr & 0x3FF) as usize;
            for (k, &b) in bytes.iter().enumerate() {
                let a = base + k;
                if a < 960 {
                    self.nt.tile[a] = b;
                } else if a < 1024 {
                    self.nt.attr[a - 960] = b;
                    self.nt.shadow[a - 960] = b;
                }
            }
        }
    }

    /// code_3FE0FA / code_3FE12E / code_3FE179 (stage 4, every frame after the objects): the bar sliders' tile requests (`$0120+y` bit 6) draw one metatile (`$012C`) as 2x2 tiles at
    /// (`$0124`:`$0128`) when that spot is inside the part of the ring the column builder has already filled (code_3FE1BF); nothing is drawn while the buffer is busy (`$19`).
    fn draw_bars(&mut self) {
        if self.stage.id != 4 || self.camera.busy() || self.world.col_block {
            return;
        }
        let mut drew = false;
        for y in (0..4usize).rev() {
            if self.world.b120[y] & 0xC0 != 0xC0 {
                continue;
            }
            self.world.b120[y] &= 0xBF;
            let m = self.world.b12c[y] as usize;
            let (hi, lo) = (self.world.b124[y], self.world.b128[y]);
            for (dx, top, bot) in [(0u8, 0xA000usize, 0xA200usize), (1, 0xA100, 0xA300)] {
                let col02 = (lo & 0x1F) | dx;
                if !self.bar_visible(y, col02) {
                    continue;
                }
                let a = (((hi & 3) as usize) << 8) | (lo | dx) as usize;
                let a2 = (((hi & 3) as usize) << 8) | ((lo | dx) | 0x20) as usize;
                if a < 960 {
                    self.nt.tile[a] = self.stage.byte(top + m);
                }
                if a2 < 960 {
                    self.nt.tile[a2] = self.stage.byte(bot + m);
                }
                drew = true;
            }
        }
        let _ = drew;
    }

    /// code_3FE1BF: is column `col02` of bar slot `y` already built into the ring (compared with the camera and the column builder's position)?
    fn bar_visible(&self, y: usize, col02: u8) -> bool {
        if self.camera.sect & 0xC0 != 0 {
            return true;
        }
        let c00 = ((self.camera.x & 0xFF) >> 3) as u8;
        let carry = col02 >= c00;
        let page01 = self.world.b120[y] & 0x3F;
        let f9 = (self.camera.x >> 8) as u8;
        let t = page01 as i32 - f9 as i32 - (!carry) as i32;
        let (c24, c2a) = (self.camera.c24, self.camera.c2a);
        if t < 0 {
            return false;
        }
        if t == 0 {
            if col02 == c00 {
                return false;
            }
            if col02 != c24 {
                return true;
            }
            return page01 == c2a;
        }
        if t != 1 {
            return false;
        }
        if col02 == c24 {
            return page01 == c2a;
        }
        if self.camera.c23 & 1 != 0 {
            return false;
        }
        col02 + 1 == c24
    }

    /// A step of the boss-room door drawing (code_3FE682 = entry door, code_3FE751 = exit door): a metatile of the shutter into the nametable, then its attribute byte.
    fn draw_door_step(&mut self) {
        let Some((kind, step)) = self.camera.door_step.take() else { return };
        if kind == 1 && step == 0 && self.hold_oam.is_none() {
            self.hold_oam = Some(self.sprites_impl(false));
        }
        let st = &self.stage;
        let (ids, attrs, base) = if kind == 0 { (0xB572u16, 0xB578u16, 0xB570u16) } else { (0xB57E, 0xB584, 0xB57C) };
        let (addr, a03, a04) = match (step, self.door) {
            (0, _) | (_, None) => {
                let (aa, qq) = if kind == 0 { (0xB576u16, 0xB577u16) } else { (0xB582, 0xB583) };
                ((st.byte(base as usize) as u16) << 8 | st.byte(base as usize + 1) as u16, st.byte(aa as usize), st.byte(qq as usize))
            }
            (_, Some(d)) => d,
        };
        let m = st.byte((ids + step as u16) as usize) as usize;
        if std::env::var("MM4_DBGP").is_ok() { eprintln!("door kind {} step {} addr {:04X} a03 {:02X} a04 {:02X} meta {:02X}", kind, step, addr, a03, a04, m); }
        let put = |nt: &mut crate::nametable::NameTable, a: u16, t: u8| {
            let a = (a & 0x3FF) as usize;
            if a < 960 {
                nt.tile[a] = t;
            }
        };
        if kind == 0 {
            put(&mut self.nt, addr, st.byte(0xA000 + m));
            put(&mut self.nt, addr + 1, st.byte(0xA100 + m));
            let a2 = (addr & 0xFF00) | ((addr & 0xFF) as u8 | 0x20) as u16;
            put(&mut self.nt, a2, st.byte(0xA200 + m));
            put(&mut self.nt, a2 + 1, st.byte(0xA300 + m));
        } else {
            put(&mut self.nt, addr, st.byte(0xA100 + m));
            let a2 = (addr & 0xFF00) | ((addr & 0xFF) as u8 | 0x20) as u16;
            put(&mut self.nt, a2, st.byte(0xA300 + m));
        }
        let mask = self.rom.cpu_byte(0x3E, 0x3F, 0xE810 + a04 as u16);
        let i = (a03 & 63) as usize;
        self.nt.attr[i] = (self.nt.attr[i] & mask) | st.byte((attrs + step as u16) as usize);
        // the next step: 2 tile rows up (entry door) or down (exit door)
        let (na, n03, n04) = if kind == 0 {
            let n04 = a04 ^ 2;
            (addr.wrapping_sub(0x40), if n04 == 3 { a03.wrapping_sub(8) } else { a03 }, n04)
        } else {
            let n04 = a04 ^ 2;
            (addr.wrapping_add(0x40), if n04 == 0 { a03.wrapping_add(8) } else { a03 }, n04)
        };
        self.door = Some((na, n03, n04));
    }

    /// End of one iteration of the vertical scroll loop (code_3ECE97 and the NMI wait that follows it): stage 4's row builder leaves its scratch
    /// values in `$E6/$E7`, then the NMI steps the random task once (the verifier only supplies the steps that are left over after the loop).
    fn scroll_iteration_end(&mut self) {
        if !std::mem::take(&mut self.camera.nmi_iter) {
            return;
        }
        if let Some((pg, col)) = self.camera.col_build {
            if self.camera.sect & 0xC0 == 0 {
                self.nt.build_column(&self.stage, pg, col);
            }
        }
        if let Some((pg, col)) = self.camera.col_build.take() {
            self.world.column_scribble(&self.stage, pg, col, self.camera.sect & 0xC0 != 0);
        }
        if let Some((pg, c24)) = self.camera.row_build.take() {
            self.world.row_scribble(&self.stage, pg, c24, self.camera.sect & 0xC0 != 0);
            // the buffer's end marker (`$D17B[$23 bit 2]`) leaves the attribute record out of the builder's upload when scrolling up and out of the copy's when scrolling down
            self.nt.build_row(&self.stage, pg, c24, self.camera.c23 & 4 != 0);
            self.row_pending = Some((pg, c24 ^ 1));
        }
        if std::mem::take(&mut self.camera.row_copy) {
            if let Some((pg, r)) = self.row_pending.take() {
                self.nt.build_row(&self.stage, pg, r, self.camera.c23 & 4 == 0);
            }
        }
        self.world.rng_step();
        self.world.flush_blocks(&self.stage);
        self.world.loop_steps += 1;
    }
}

impl Game {
    /// code_3C8A70: after frame 4 of the pod animation the scene changes inside one logic frame (pod room <-> boss rooms): the slots are wiped, the new
    /// section is loaded (code_3FE5C8) and Mega Man appears there with the spawn beam (state `$0A`).
    fn pod_scene_switch(&mut self) {
        if self.world.d9 != 0x16 {
            // code_3C8AEB: the pod-room music
            self.collect_snd();
            self.world.d9 = 0x16;
            self.snd_out.push(crate::sound::Req { music: true, id: 0x16, ret: 0x8AF4 });
        }
        let t = |a: u16| self.rom.cpu_byte(0x3C, 0x3D, a);

        let mut y = (self.camera.sect & 0x0F) as u16;
        if y == 9 && self.world.ab == 8 {
            y = 0x0B;
        }
        let xpx = t(0x8BB9 + y);
        let ypx = t(0x8BC7 + y);
        let mut page = t(0x8B8D + y);
        if page == 0x0A {
            y = self.world.ab as u16 + 0x0E;
            page = t(0x8B8D + y);
        }
        let sect = t(0x8BA3 + y);
        let (mut xpx, mut ypx) = (xpx, ypx);
        if page == 9 {
            // code_3C8AF7: back in the pod room he appears on the pod he used (tables $8BD5 / $8BDD indexed by `$AB`)
            xpx = t(0x8BD5 + self.world.ab as u16);
            ypx = t(0x8BDD + self.world.ab as u16);
        }
        if std::env::var("MM4_DBG").is_ok() {
            eprintln!("pod switch: sect-from={} ab={} page={} sect={} x={:02X} y={:02X}", self.camera.sect, self.world.ab, page, sect, xpx, ypx);
        }
        self.stage.ac_full.set(self.world.ac == 0xFF);
        self.world.clear_slots();
        self.world.section_reset();
        self.player.m4c8 = 0;
        self.stage.reset_blocks();
        self.stage.set_scroll_mode(0, 0);
        self.world.ef_override = Some(0);
        // only these change (the stage reload of a restart would reset much more): `$0330`, `$0348`, `$0378`, state `$0A` with animation `$13`
        self.player.x = ((page as i32) << 16) | ((xpx as i32) << 8) | (self.player.x & 0xFF);
        self.player.y = ((ypx as i32) << 8) | (self.player.y & 0xFF);
        self.player.state = crate::player::STATE_SPAWN;
        self.player.anim = crate::anim::Anim::new(crate::anim::SPAWN_FALL);
        self.camera = Camera::at_scene(page as i32, self.camera.x & 0xFF, xpx as i32, &self.stage, sect);
        self.player_draw_pass(); // code_3FFCE3 draws once more inside this frame: visibility check, animation tick and `$95`
        self.world.p95 = self.world.p95.wrapping_add(1);
        self.draws_since_logic += 1;
        if page == 9 && self.world.ac == 0xFF {
            // code_3C8B07: every robot master is down: the ninth pod appears at ($80, $44) in slot 23 (copy of Mega Man's position, anim $73)
            self.world.make_final_pod(self.player.x, self.player.y);
            let cam = self.cam();
            self.world.draw_final_pod(cam);
        }
        let cur = self.stage.byte(0xB300 + page as usize);
        self.world.c9e = cur;
        self.world.c9f = cur;
        if std::env::var("MM4_DBG4").is_ok() {
            eprintln!("switch done: o23={:02X}/{:04X}/{:02X} ac={:02X} page={} cur={} cam={:X} ab={} c149={} defeated={:02X?} dir={}", self.world.o[23].ty, self.world.o[23].ptr, self.world.o[23].xpx, self.world.ac, page, cur, self.camera.x, self.world.ab, self.world.c149, &self.world.defeated[..4], self.camera.dir);
        }
    }
}

impl Game {
    /// Mega Man\x27s part of the draw pass (code_3EDB72 for slot 0): when his page differs from the camera\x27s (after the low-byte borrow) the slot is wiped
    /// (code_3EDC00: type 0) and he is no longer drawn or animated; otherwise his animation ticks.
    fn player_draw_pass(&mut self) {
        if self.player.hidden {
            return;
        }
        let cam = self.cam();
        let (_, borrow) = ((self.player.x >> 8) as u8).overflowing_sub(cam.lo);
        let t01 = ((self.player.x >> 16) as u8).wrapping_sub(cam.hi).wrapping_sub(borrow as u8);
        if t01 != 0 && !matches!(self.player.anim.id, 0x55 | 0x71) {
            self.player.hidden = true;
            return;
        }
        self.player.anim.advance(&self.stage.anims);
    }
}

impl Game {
    /// Moves the sound requests the parts of the game made so far into `snd_out` (player first, then the objects, then the camera: the order of the frame).
    fn collect_snd(&mut self) {
        let p = std::mem::take(&mut self.player.snd);
        for q in p.iter().filter(|q| q.music) {
            self.world.d9 = q.id;
        }
        self.snd_out.extend(p);
        let w = std::mem::take(&mut self.world.snd);
        self.snd_out.extend(w);
        let c = std::mem::take(&mut self.camera.snd);
        for q in c.iter().filter(|q| q.music) {
            self.world.d9 = q.id;
        }
        self.snd_out.extend(c);
    }
}

impl Game {
    /// code_3EC681: the stage load asks for the stage's music (table `$C85B`).
    fn stage_music(&mut self) {
        let id = self.rom.cpu_byte(0x3E, 0x3F, 0xC85B + self.stage.id as u16);
        self.world.d9 = id;
        self.start_snd.push(crate::sound::Req { music: true, id, ret: 0xC687 });
    }
}
