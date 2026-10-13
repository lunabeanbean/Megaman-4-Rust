//! The whole game as one state machine, one call per video frame: power-on (copyright, CAPCOM, story), title, password entry, stage select, boss intro, the stage,
//! the stage-end scenes, game over, the ending. The window (`mm4`), the headless chain runner (`--chain`) and the chain verifier all drive this same `Flow`.
use crate::flow::{self, Pic};
use crate::rom::Rom;
use crate::{game, player, select, stage};

/// What follows a clip of pictures.
#[derive(Clone, Copy)]
pub enum Next {
    /// The password screen (stage clear or game over; the flag = game over).
    Password(crate::password_screen::PasswordKind, bool),
    Select,
    Play,
    Title,
    Entry,
    /// The ending, with `$92` and `$95` at its start.
    Ending(u8, u8),
}

pub enum Screen {
    Play,
    /// Pictures played one per frame (a fade, a cutscene), then the next screen.
    Frames(Vec<Pic>, usize, Next),
    /// A scene script running live (the opening): its latest picture is kept to show the sprites and scroll the way the console does.
    Live(crate::scene::LiveScene, crate::screen::Screen, Next),
    /// The password entry screen.
    Entry(crate::password_entry::PasswordEntry),
    /// The title screen (PRESS START / PASS WORD).
    Title(crate::title_screen::TitleScreen),
    /// The pause menu logic, its picture (the real screen, built from the ROM) and a frame counter for the blinking cursor.
    Pause(crate::pause::PauseMenu, crate::screen::PauseScreen, u8),
    /// The password screen with its menu; the flag says it is the game-over screen (code_398EF7) instead of the stage-clear one.
    Password(crate::password_screen::PasswordScreen, bool),
    /// Stage select: the grid logic, its picture and the frame counter `$92` that blinks it.
    Select(select::StageSelect, crate::select_screen::SelectScreen, u8),
}

/// Where a `Flow` starts.
pub enum Start {
    /// Power-on: copyright, CAPCOM, the story, the title.
    PowerOn,
    /// Straight into a stage.
    Stage,
}

pub struct Flow {
    pub rom: Rom,
    pub st: stage::Stage,
    pub img: Vec<u8>,
    pub g: game::Game,
    menu_stage: stage::Stage,
    pub screen: Screen,
    prev_menu: u8,
    prev_held: u8,
    /// The window title the shell should show (stage and weapon).
    pub title: String,
    /// The picture of the latest `step`.
    pub frame: Vec<u32>,
    /// Frames played so far.
    pub frames: u64,
    /// The game's frame counter `$92`: one more per frame in the scenes, the stage's own counter while playing.
    pub p92: u8,
    prev_play: bool,
    /// The latest snapshot of the screen shown (the console displays the sprites of the frame before).
    last_snap: crate::screen::Screen,
    /// Buttons of the two frames before (the stage's logic reads the controller one frame late: the state polled in frame f-1 acts in frame f).
    hist: [(u8, u8); 2],
    /// Checks only: the random-generator state to put into the first stage frame (the real game's depends on how many frames the earlier scenes spent loading).
    pub rng_override: Option<[u8; 4]>,
    /// Checks only: force the stage clear after this frame (counted in this flow's frames) if a stage is being played then.
    pub clear_after: Option<u64>,
    /// The same for a death with no lives left.
    pub gameover_after: Option<u64>,
    /// The first frame of a stage shows no sprites yet (the console displays the OAM of the frame before, which was empty).
    first_play: bool,
    /// `$95` the next stage starts with (what the scene before left; None = keep the stage's own).
    p95_next: Option<u8>,
    /// Checks only: set the Cossack/Wily progress byte  at this frame (the recorder poked it).
    pub aa_at: Option<(u64, u8)>,
    /// Test hooks (chain checks): at flow frame F the checkpoint count `$1F` is set / the spike-death flag `$3D` is raised.
    pub cp_at: Option<(u64, u8)>,
    pub kill_at: Option<u64>,
    /// Checks only: video frames (this flow's count) in which the console spent a whole frame without a logic step (a lag frame: the game logic took longer than one frame): the picture stays and the
    /// random-task step is applied with the next logic frame.
    pub lag_at: std::collections::HashSet<u64>,
    lag_rng: usize,
    hold_one: bool,
    /// Checks only: the PPU palette per flow frame taken from a Mesen log (isolates the palette-task timing, which depends on the console's cycle counts).
    pub pal_override: std::collections::HashMap<u64, [u8; 32]>,
    /// The sound system: request queue, driver and APU (see `sound.rs`); every `step` runs one video frame of it and appends the samples to `audio`.
    pub sound: crate::sound::Sound,
    /// Mono samples produced since the shell last took them (`take_audio`).
    pub audio: Vec<f32>,
    /// Sound requests made by the flow itself (transitions, the pause menu) in the frame being run.
    now: Vec<crate::sound::Req>,
    /// The latest `step_screen` ran the stage logic (its `g.snd_out` belongs to this frame).
    play_stepped: bool,
    /// The pause menu's E-tank refill: units still to add and frames to wait before the next unit's sound (the logic refills at once, the sound keeps the real rhythm).
    refill_left: u8,
    refill_wait: u8,
    /// Frames the game stands still after a logic frame that spent extra video frames (item refills), each with the sound requests due in it.
    hold: std::collections::VecDeque<Vec<crate::sound::Req>>,
    /// The sound requests the latest `step` handed to the driver (for logs and checks).
    pub last_reqs: Vec<crate::sound::Req>,
}

impl Flow {
    pub fn new(rom: &Rom, st: stage::Stage, start: Start) -> Flow {
        let img = st.render_idx().2;
        let g = game::Game::new_stage(rom, st.clone());
        let menu_stage = stage::Stage::load(rom, 0x10, 5); // scene `$10` holds the menu screens (pause layout = screen 3)
        let screen = match start {
            Start::PowerOn => {
                // copyright, CAPCOM and the opening story run as a live script (Start skips the story), then the title
                let mut blank = crate::screen::Screen::new();
                blank.pal = [0x0F; 32];
                let runner = crate::scene::SceneRunner::new(blank, 1, 0);
                let rom2 = rom.clone();
                let live = crate::scene::LiveScene::start(runner, move |r| {
                    crate::boot_screens::boot(r, &rom2);
                    crate::intro_story::play(r, &rom2);
                });
                Screen::Live(live, crate::screen::Screen::new(), Next::Title)
            }
            Start::Stage => Screen::Play,
        };
        let mut g = g;
        let now = if matches!(start, Start::Stage) { std::mem::take(&mut g.start_snd) } else { Vec::new() };
        Flow { rom: rom.clone(), st, img, g, menu_stage, screen, prev_menu: 0, prev_held: 0, title: String::new(), frame: vec![0u32; stage::SCREEN_W * stage::SCREEN_H], frames: 0, p92: 0, prev_play: false, last_snap: crate::screen::Screen::new(), hist: [(0, 0); 2], rng_override: None, clear_after: None, gameover_after: None, first_play: false, aa_at: None, cp_at: None, kill_at: None, lag_at: Default::default(), lag_rng: 0, hold_one: false, pal_override: Default::default(), p95_next: None, sound: crate::sound::Sound::new(rom, 44100), audio: Vec::new(), now, play_stepped: false, refill_left: 0, refill_wait: 0, hold: Default::default(), last_reqs
: Vec::new() }
    }

    pub fn is_play(&self) -> bool {
        matches!(self.screen, Screen::Play)
    }

    /// A short name of the current screen (for logs).
    pub fn screen_name(&self) -> &'static str {
        match self.screen {
            Screen::Play => "play",
            Screen::Frames(..) => "frames",
            Screen::Live(..) => "live",
            Screen::Entry(_) => "entry",
            Screen::Title(_) => "title",
            Screen::Pause(..) => "pause",
            Screen::Password(..) => "password",
            Screen::Select(..) => "select",
        }
    }

    /// One video frame: the picture (`self.frame`) and the frame's sound (`self.audio`). `held` = D-pad and A/B as the gameplay reads them, `menu_held` = the menu
    /// buttons (D-pad, A, Start, Select; no B).
    pub fn step(&mut self, held: u8, menu_held: u8) {
        self.play_stepped = false;
        self.step_video(held, menu_held);
        self.run_sound();
    }

    /// Hands the frame's sound requests to the driver and runs one video frame of sound.
    fn run_sound(&mut self) {
        let mut reqs = std::mem::take(&mut self.now);
        if self.refill_left > 0 {
            if self.refill_wait == 0 {
                reqs.push(crate::sound::Req { music: false, id: 0x29, ret: 0x96FB }); // code_3C96F8
                self.refill_left -= 1;
                self.refill_wait = 8;
            }
            self.refill_wait = self.refill_wait.saturating_sub(1);
        }

        match &self.screen {
            Screen::Play => {
                if self.play_stepped {
                    reqs.extend(self.g.snd_out.iter().copied());
                }
            }
            Screen::Live(_, s, _) => reqs.extend(s.snd.iter().copied()),
            Screen::Frames(pics, i, _) => {
                if let Some(Pic::Scene(sc)) = i.checked_sub(1).and_then(|k| pics.get(k)) {
                    reqs.extend(sc.snd.iter().copied());
                }
            }
            Screen::Pause(_, ps, _) => reqs.extend(ps.screen.snd.iter().copied()),
            Screen::Entry(pe) => reqs.extend(pe.screen.snd.iter().copied()),
            Screen::Title(ts) => reqs.extend(ts.screen.snd.iter().copied()),
            Screen::Password(ps, _) => reqs.extend(ps.screen.snd.iter().copied()),
            Screen::Select(_, ss, _) => reqs.extend(ss.screen.snd.iter().copied()),
        }
        self.last_reqs = reqs.clone();
        for q in reqs {
            self.sound.request(q.id);
        }
        self.sound.frame(&mut self.audio);
    }

    /// The samples made since the last call (mono, at the rate given to `set_sample_rate`).
    pub fn take_audio(&mut self) -> Vec<f32> {
        std::mem::take(&mut self.audio)
    }

    /// Rebuilds the sound system for the output device's sample rate (call before the first `step`).
    pub fn set_sample_rate(&mut self, rate: u32) {
        self.sound = crate::sound::Sound::new(&self.rom, rate);
    }

    fn step_video(&mut self, held: u8, menu_held: u8) {
        self.frames += 1;
        self.p92 = self.p92.wrapping_add(1);
        if self.clear_after.map_or(false, |c| self.frames > c) && self.is_play() {
            self.clear_after = None;
            self.g.force_stage_clear();
        }
        if let Some((f, v)) = self.cp_at {
            if self.frames > f {
                self.cp_at = None;
                self.g.checkpoint = v;
            }
        }
        if self.kill_at.map_or(false, |f| self.frames > f) {
            self.kill_at = None;
            self.g.player.state = player::STATE_DEAD;
            self.g.player.dead_timer = 1;
        }
        if let Some((f, v)) = self.aa_at {
            if self.frames > f {
                self.aa_at = None;
                self.g.progress.flags[1] = v;
            }
        }
        if self.gameover_after.map_or(false, |c| self.frames > c) && self.is_play() {
            self.gameover_after = None;
            self.g.force_game_over();
        }
        let rom = self.rom.clone();
        let (h1, m1) = self.hist[0];
        let (h2, m2) = self.hist[1];
        let play = (player::Buttons { held: h1, pressed: h1 & !h2 }, m1 & !m2);
        self.hist = [(held, menu_held), (h1, m1)];
        loop {
            // (a screen change inside the loop re-enters it without showing a picture, with no new button edges, the way the window loop always did)
            let buttons = player::Buttons { held, pressed: held & !self.prev_held };
            let menu_pressed = menu_held & !self.prev_menu;
            self.prev_held = held;
            self.prev_menu = menu_held;
            if self.step_screen(&rom, buttons, menu_pressed, menu_held, play) {
                self.prev_play = self.is_play();
                if self.prev_play {
                    self.p92 = self.g.world.frame92;
                }
                return;
            }
        }
    }

    /// Runs the current screen for this frame: true = a picture was produced, false = the screen changed and the frame must run again.
    fn step_screen(&mut self, rom: &Rom, buttons: player::Buttons, menu_pressed: u8, menu_held: u8, play: (player::Buttons, u8)) -> bool {
        match &mut self.screen {
            Screen::Play => {
                if let Some(reqs) = self.hold.pop_front() {
                    self.now.extend(reqs);
                    return true; // the picture stays
                }
                let g = &mut self.g;
                if g.stage_clear_due() {
                    // the stage-clear countdown ended: flags and weapons (code_398C7C), then the menus
                    let end = g.progress.stage_cleared(rom, g.stage.id);
                    g.last_end = Some(end.clone());
                    let pics = if g.stage.id < 8 {
                        // the stage-end code draws the objects once before the scenes start: `$95` is one higher than the last frame of play left it
                        let pre = fade_out_play(&self.st, &self.img, g, if end.repeat { 62 } else { 2 }, if end.repeat { 6 } else { 8 });
                        flow::clear_pictures(rom, pre, g.stage.id, end.repeat, g.world.frame92, g.world.p95.wrapping_add(1))
                    } else {
                        // stages of the second half: no weapon-get, no password; the map (or the cutscene after the fourth) shows the progress, then the next stage starts
                        let aa = g.progress.flags[1];
                        let mut pics = fade_out_play(&self.st, &self.img, g, 2, 6);
                        let p92 = g.world.frame92.wrapping_add(pics.len() as u8);
                        if aa == 0x7F {
                            // every stage done: the ending (castle, saucer, credits), then back to the title
                            self.screen = Screen::Frames(pics, 0, Next::Ending(p92, g.world.p95.wrapping_add(1)));
                            return false;
                        }
                        let (frames, id) = if aa == 0x0F {
                            (crate::wily_flight::chain(rom, aa, p92, g.world.p95.wrapping_add(1)), 12)
                        } else if aa > 0x0F {
                            (crate::fortress::map_scene(rom, crate::fortress::MapKind::Wily, aa, p92, 0), 8 + aa.trailing_ones() as usize)
                        } else {
                            (crate::fortress::map_scene(rom, crate::fortress::MapKind::CossackAfter, aa, p92, 0), 8 + aa.trailing_ones() as usize)
                        };
                        let last = frames.last().cloned().unwrap_or_else(crate::screen::Screen::new);
                        pics.extend(flow::render_all(&frames));
                        let progress = g.progress.clone();
                        let (st2, img2, g2) = load_stage(rom, id, progress);
                        self.st = st2;
                        self.img = img2;
                        self.g = g2;
                        self.title = format!("Mega Man 4 port - stage {}", id);
                        self.p95_next = Some(crate::scene::last_p95());
                        pics.extend(stage_fade_in(&last, &self.img, &self.st, &mut self.g));
                        self.screen = Screen::Frames(pics, 0, Next::Play);
                        return false;
                    };
                    self.screen = Screen::Frames(pics, 0, Next::Password(crate::password_screen::PasswordKind::StageClear, false));
                    return false;
                }
                if g.game_over {
                    // forced from outside (checks): the death countdown ended with no lives left
                    let wily = g.stage.id >= 8;
                    let pics = fade_out_play(&self.st, &self.img, g, 2, 6);
                    self.screen = Screen::Frames(pics, 0, Next::Password(crate::password_screen::PasswordKind::GameOver { wily }, true));
                    return false;
                }
                if play.1 & select::BTN_START != 0 && g.can_pause() {
                    // code_3EC707: Start opens the weapon menu (the game is frozen meanwhile)
                    let mut energy = [0u8; 15];
                    energy[0] = g.player.life;
                    energy[1..14].copy_from_slice(&g.progress.weapons[1..14]);
                    energy[14] = 0x9C;
                    let info = crate::screen::PauseInfo { energy, lives: g.player.lives, etanks: g.player.etanks };
                    let menu = g.pause_menu();
                    self.now.push(crate::sound::Req { music: false, id: 0x30, ret: 0xC73D }); // code_3EC73A
                    let ps = crate::screen::PauseScreen::open(rom, &self.menu_stage, &info, menu.cursor);
                    self.screen = Screen::Pause(menu, ps, 0);
                    return false;
                }
                if !self.prev_play {
                    g.world.frame92 = self.p92.wrapping_sub(1); // the counter runs on from the scenes before the stage
                    if let Some(v) = self.p95_next.take() {
                        g.world.p95 = v;
                    }
                    if let Some(r) = self.rng_override.take() {
                        g.world.rng = r;
                    }
                }
                g.world.frame92 = g.world.frame92.wrapping_add(1);
                if std::mem::take(&mut self.hold_one) {
                    self.first_play = false;
                    let _ = g.sprites_shown();
                    let pal = g.world.pal.ppu;
                    render_play_with_list(&mut self.frame, &self.img, &self.st, g, &pal, Vec::new());
                    return true;
                }
                if self.lag_at.contains(&(self.frames - 1)) && self.prev_play {
                    // no logic step in this video frame (the palette task, a second thread, only runs when the logic yields): the same picture again
                    self.lag_rng += 1;
                    let pal = g.world.pal.ppu;
                    let pal = self.pal_override.get(&(self.frames - 1)).copied().unwrap_or(pal);
                    render_play_pal_again(&mut self.frame, &self.img, &self.st, g, &pal);
                    return true;
                }
                let steps = 1 + std::mem::take(&mut self.lag_rng);
                g.defer_after_restart = std::env::var("MM4_NODEFER").is_err();
                let pre = if g.player.state == player::STATE_DEAD && g.player.dead_timer <= 1 { Some(g.fork()) } else { None };
                g.step(play.0, steps);
                self.play_stepped = true;
                if g.extra_frames > 0 {
                    // a refill (or another long routine) spent video frames inside this logic frame: the game stands still for them and the sounds keep the console's rhythm
                    // (one tick of the refill every 4 frames)
                    let mut later: Vec<crate::sound::Req> = Vec::new();
                    let mut first = true;
                    g.snd_out.retain(|q| {
                        if q.ret == 0xBC30 && !std::mem::replace(&mut first, false) {
                            later.push(*q);
                            false
                        } else {
                            true
                        }
                    });
                    self.hold = (1..=g.extra_frames).map(|k| if k % 4 == 0 && !later.is_empty() { vec![later.remove(0)] } else { Vec::new() }).collect();
                }
                if g.game_over {
                    // no lives left: the game restarted the stage with 2 lives (code_398EF7); the game-over screen comes first
                    let wily = g.stage.id >= 8;
                    let pics = fade_out_play(&self.st, &self.img, g, 2, 6);
                    self.screen = Screen::Frames(pics, 0, Next::Password(crate::password_screen::PasswordKind::GameOver { wily }, true));
                    return false;
                }
                if g.restarted {
                    if let Some(mut pre_g) = pre {
                        // the death countdown ended: the picture fades out (black for 43 frames while the stage loads), the stage fades in again, then the new life starts
                        let mut pics = fade_out_play(&self.st, &self.img, &mut pre_g, DEATH_HELD, 43);
                        let ps = g.pal_at_restart.clone().unwrap_or_else(|| g.world.pal.clone());
                        pics.extend(fade_in_frames(&self.img, &self.st, g, ps));
                        let rom2 = g.rom().clone();
                        let _ = &rom2;
                        g.bg_late = true;
                        if std::env::var("MM4_NODEFER").is_ok() {
                            self.hold_one = true;
                            g.world.p95 = g.world.p95.wrapping_sub(1);
                        }
                        self.screen = Screen::Frames(pics, 0, Next::Play);
                        return false;
                    }
                }
                // (dying is handled inside the game: 300 frames after death the stage restarts at the checkpoint)
                if let Some(p) = self.pal_override.get(&(self.frames - 1)) {
                    g.world.pal.ppu = *p;
                }
                let first = std::mem::take(&mut self.first_play);
                render_play_with(&mut self.frame, &self.img, &self.st, g, !first, true);
            }
            Screen::Pause(menu, ps, tick) => {
                let life0 = self.g.player.life;
                let r = self.g.pause_step(menu, menu_pressed);
                if let crate::pause::PauseResult::ETank { used: true } = r {
                    // the E-tank refills the meter one unit at a time (code_3C96F4: a sound and 8 frames per unit)
                    self.refill_left = 0x1Cu8.saturating_sub(life0 & 0x1F);
                    self.refill_wait = 0;
                }
                if matches!(r, crate::pause::PauseResult::Weapon(_)) {
                    self.now.push(crate::sound::Req { music: false, id: 0x30, ret: 0x9774 }); // code_3C9771

                    self.title = format!("Mega Man 4 port - stage {} - weapon {}", self.g.stage.id, self.g.player.weapon);
                    self.screen = Screen::Play;
                    return false;
                }
                ps.update(rom, menu.cursor, *tick);
                *tick = tick.wrapping_add(1);
                self.frame.copy_from_slice(&ps.screen.displayed(&self.last_snap).render());
                self.last_snap = ps.screen.clone();
            }
            Screen::Live(live, prev, next) => match live.tick(menu_held) {
                Some(s) => {
                    self.frame.copy_from_slice(&s.displayed(prev).render());
                    *prev = s;
                }
                None => {
                    // the next scene's routine starts with a fade-out of whatever is on the screen (black after a story that ran to its end, the lit scene after a Start skip): 1 + 12 + 4 frames (verified by the chain runs)
                    let next = *next;
                    self.screen = Screen::Frames(flow::render_all(&crate::scene::fade_out_screens(prev, 1, 4)), 0, next);
                    return false;
                }
            },
            Screen::Entry(pe) => {
                match pe.frame(rom, menu_pressed) {
                    crate::password_entry::EntryResult::Stay => {}
                    crate::password_entry::EntryResult::Accepted(energy, a9) => {
                        self.now.extend(pe.screen.snd.drain(..));
                        self.g.progress = crate::progress::Progress::new();
                        self.g.progress.weapons = energy;
                        self.g.progress.flags = [a9, 0];
                        self.screen = Screen::Frames(flow::render_all_after(&crate::scene::fade_out_screens(&pe.screen, 1, 39), &self.last_snap), 0, Next::Select);
                        return false;
                    }
                }
                self.frame.copy_from_slice(&pe.screen.displayed(&self.last_snap).render());
                self.last_snap = pe.screen.clone();
            }
            Screen::Title(ts) => {
                use crate::title_screen::TitleResult;
                match ts.frame(menu_pressed) {
                    TitleResult::Stay => {}
                    TitleResult::Password => {
                        self.now.extend(ts.screen.snd.drain(..));
                        // fade out (orig through the press frame, 12 faded, 6 black), then the entry screen
                        self.screen = Screen::Frames(flow::render_all_after(&crate::scene::fade_out_screens(&ts.screen, 1, 6), &self.last_snap), 0, Next::Entry);
                        return false;
                    }
                    TitleResult::PressStart => {
                        self.now.extend(ts.screen.snd.drain(..));
                        self.g.progress = crate::progress::Progress::new();
                        self.screen = Screen::Frames(flow::render_all_after(&crate::scene::fade_out_screens(&ts.screen, 1, 39), &self.last_snap), 0, Next::Select);
                        return false;
                    }
                    TitleResult::Timeout => {
                        self.now.extend(ts.screen.snd.drain(..));

                        self.screen = Screen::Title(crate::title_screen::TitleScreen::new(rom)); // the intro story would run again
                        return false;
                    }
                }
                self.frame.copy_from_slice(&ts.screen.displayed(&self.last_snap).render());
                self.last_snap = ts.screen.clone();
            }
            Screen::Frames(pics, i, next) => {
                if matches!(next, Next::Play) && *i + 17 >= pics.len() && !self.g.start_snd.is_empty() {
                    // the stage load asks for the stage's music 17 frames before the fade-in ends (measured on the chains)
                    self.now.extend(std::mem::take(&mut self.g.start_snd));
                }
                if *i >= pics.len() {

                    let next = *next;
                    self.screen = match next {
                        Next::Password(kind, over) => Screen::Password(crate::password_screen::PasswordScreen::new(rom, kind, &ram_b0(&self.g)), over),
                        Next::Play => {
                            self.first_play = true;
                            self.now.extend(std::mem::take(&mut self.g.start_snd));
                            Screen::Play
                        }
                        Next::Entry => Screen::Entry(crate::password_entry::PasswordEntry::new(rom)),
                        Next::Title => Screen::Title(crate::title_screen::TitleScreen::new(rom)),
                        Next::Ending(p92, p95) => {
                            let mut blank = crate::screen::Screen::new();
                            blank.pal = [0x0F; 32];
                            let runner = crate::scene::SceneRunner::new(blank, p92, p95);
                            let rom2 = rom.clone();
                            let live = crate::scene::LiveScene::start(runner, move |r| crate::ending::play(r, &rom2, crate::ending::mega_man_after_stage()));
                            Screen::Live(live, crate::screen::Screen::new(), Next::Title)
                        }
                        Next::Select => Screen::Select(select::StageSelect::new(rom), crate::select_screen::SelectScreen::new(rom, self.g.progress.flags[0]), self.p92),
                    };
                    return false;
                }
                if let Pic::Scene(sc) = &pics[*i] {
                    self.last_snap = sc.clone();
                }
                self.frame.copy_from_slice(&pics[*i].render());
                *i += 1;
            }
            Screen::Password(ps, over) => {
                let over = *over;
                match ps.frame(rom, menu_pressed) {
                    crate::password_screen::PasswordResult::Stay => {}
                    crate::password_screen::PasswordResult::First => {
                        self.now.extend(ps.screen.snd.drain(..));
                        self.g.progress.refill(); // code_398EDE
                        self.screen = if over && self.g.stage.id >= 8 {
                            Screen::Play
                        } else {
                            // the confirm menu fades out (16 frames, 40 black frames with the next scene loading), then the stage select comes up
                            Screen::Frames(flow::render_all_after(&crate::scene::fade_out_screens(&ps.screen, 1, 39), &self.last_snap), 0, Next::Select)
                        };
                        return false;
                    }
                    crate::password_screen::PasswordResult::Second => {
                        self.now.extend(ps.screen.snd.drain(..));

                        // the second entry (CONTINUE): refill and play the same stage again
                        self.g.progress.refill();
                        let progress = self.g.progress.clone();
                        self.g = game::Game::new_stage(rom, self.st.clone());
                        self.g.progress = progress;
                        // the menu fades out, the stage loads again (black frames) and fades in like at a stage start
                        let pics = stage_fade_in(&ps.screen, &self.img, &self.st, &mut self.g);
                        self.screen = Screen::Frames(pics, 0, Next::Play);
                        return false;
                    }
                }
                self.frame.copy_from_slice(&ps.screen.displayed(&self.last_snap).render());
                self.last_snap = ps.screen.clone();
            }
            Screen::Select(sel, ss, tick) => {
                let dirs = menu_pressed & (select::BTN_LEFT | select::BTN_RIGHT | select::BTN_UP | select::BTN_DOWN);
                // the picture owns the cursor (it follows the loop the way the game does); the grid logic only decides what Start/A picks
                sel.col = ss.col;
                sel.row = ss.row;
                let next = if ss.accepts_input() { sel.step(menu_pressed & (select::BTN_START | select::BTN_A), self.g.progress.flags[0]) } else { None };
                if let Some(next) = next {
                    self.now.push(crate::sound::Req { music: false, id: 0x2A, ret: 0x80EE }); // code_3980EB: the pick

                    if next < 8 {
                        // load the chosen stage with the same progress (stages without ported content still load and render)
                        let progress = self.g.progress.clone();
                        self.st = stage::Stage::load(rom, next as usize, 0);
                        self.img = self.st.render_idx().2;
                        self.g = game::Game::new_stage(rom, self.st.clone());
                        self.g.progress = progress;
                        self.title = format!("Mega Man 4 port - stage {}", next);
                        let mut pics = flow::render_all_after(&crate::scene::fade_out_screens(&ss.screen, 1, 5), &self.last_snap);
                        let intro = crate::boss_intro::run(rom, next as usize, tick.wrapping_add(pics.len() as u8), 0);
                        let last = intro.last().cloned().unwrap_or_else(crate::screen::Screen::new);
                        pics.extend(flow::render_all(&intro));
                        self.p95_next = Some(crate::scene::last_p95());
                        pics.extend(stage_fade_in(&last, &self.img, &self.st, &mut self.g));
                        self.screen = Screen::Frames(pics, 0, Next::Play);
                        return false;
                    }
                    if next == 8 {
                        // the centre portrait: Dr. Cossack's castle (or Wily's fortress once his four stages are done), then the next stage of the second half
                        let aa = self.g.progress.flags[1];
                        let kind = if aa >= 0x0F { crate::fortress::MapKind::Wily } else { crate::fortress::MapKind::CossackFirst };
                        let id = 8 + aa.trailing_ones() as usize;
                        let mut pics = flow::render_all_after(&crate::scene::fade_out_screens(&ss.screen, 1, 6), &self.last_snap);
                        let map = crate::fortress::map_scene(rom, kind, aa, tick.wrapping_add(pics.len() as u8), 0);
                        let last = map.last().cloned().unwrap_or_else(crate::screen::Screen::new);
                        pics.extend(flow::render_all(&map));
                        let progress = self.g.progress.clone();
                        let (st2, img2, g2) = load_stage(rom, id, progress);
                        self.st = st2;
                        self.img = img2;
                        self.g = g2;
                        self.title = format!("Mega Man 4 port - stage {}", id);
                        self.p95_next = Some(crate::scene::last_p95());
                        pics.extend(stage_fade_in(&last, &self.img, &self.st, &mut self.g));
                        self.screen = Screen::Frames(pics, 0, Next::Play);
                        return false;
                    }
                }
                ss.frame(rom, dirs, *tick);
                *tick = tick.wrapping_add(1);
                self.frame.copy_from_slice(&ss.screen.displayed(&self.last_snap).render());
                self.last_snap = ss.screen.clone();
            }
        }
        true
    }
}

/// The frozen gameplay picture leaving the screen the way the stage-end code does (`code_3EC457` on the stage's palettes, exact): `held` frames, three levels of four frames, `black` frames.
pub fn fade_out_play(st: &stage::Stage, img: &[u8], g: &mut game::Game, held: usize, black: usize) -> Vec<Pic> {
    use std::rc::Rc;
    let rom = g.rom().clone();
    let mut ps = g.world.pal.clone();
    let mut v = Vec::new();
    let mut pic = vec![0u32; stage::SCREEN_W * stage::SCREEN_H];
    render_play_pal(&mut pic, img, st, g, &ps.ppu, true, false);
    let first = Rc::new(pic);
    for k in 0..held {
        if k > 0 {
            ps.frame(&rom);
            let mut p = vec![0u32; stage::SCREEN_W * stage::SCREEN_H];
            render_play_pal(&mut p, img, st, g, &ps.ppu, true, false);
            v.push(Pic::Rgb(Rc::new(p)));
        } else {
            v.push(Pic::Rgb(first.clone()));
        }
    }
    for level in [0x10u8, 0x20, 0x30] {
        for _ in 0..4 {
            ps.fade_frame(&rom, level);
            let mut p = vec![0u32; stage::SCREEN_W * stage::SCREEN_H];
            render_play_pal(&mut p, img, st, g, &ps.ppu, true, false);
            v.push(Pic::Rgb(Rc::new(p)));
        }
    }
    let b = Rc::new(vec![0u32; stage::SCREEN_W * stage::SCREEN_H]);
    for _ in 0..black {
        v.push(Pic::Rgb(b.clone()));
    }
    v
}

/// Load stage `id` and start a game in it with the progress kept.
pub fn load_stage(rom: &Rom, id: usize, progress: crate::progress::Progress) -> (stage::Stage, Vec<u8>, game::Game) {
    let st = stage::Stage::load(rom, id, 0);
    let img = st.render_idx().2;
    let mut g = game::Game::new_stage(rom, st.clone());
    g.progress = progress;
    (st, img, g)
}

/// A scene fading out into a stage: the scene fades (43 black frames while the stage loads), then the stage's palette fades in over its first picture.
pub fn stage_fade_in(last: &crate::screen::Screen, img: &[u8], st: &stage::Stage, g: &mut game::Game) -> Vec<Pic> {
    let mut v = flow::render_all(&crate::scene::fade_out_screens(last, 1, 43));
    let ps = g.world.pal.clone();
    v.extend(fade_in_frames(img, st, g, ps));
    v
}

/// The stage's palette fading in over its first picture (no sprites yet), starting from the palette state `ps` of the stage load; `g.world.pal` gets the state after it.
pub fn fade_in_frames(img: &[u8], st: &stage::Stage, g: &mut game::Game, ps: crate::palette::PalState) -> Vec<Pic> {
    use std::rc::Rc;
    let rom = g.rom().clone();
    let mut g0 = g.fork();
    let mut v = Vec::new();
    let mut ps = ps;
    for _ in 0..PRE_FADE {
        ps.frame(&rom);
    }
    // the stage's own palette fades in over the background: levels $30, $20, $10 for four frames each (the animation task keeps running and its steps show at full brightness)
    for level in [0x30u8, 0x20, 0x10] {
        for _ in 0..4 {
            ps.fade_frame(&rom, level);
            let mut pic = vec![0u32; stage::SCREEN_W * stage::SCREEN_H];
            render_play_pal(&mut pic, img, st, &mut g0, &ps.ppu, false, false);
            v.push(Pic::Rgb(Rc::new(pic)));
        }
    }
    for k in 0..3 {
        if k == 0 {
            ps.fade_frame(&rom, 0);
        } else {
            ps.frame(&rom);
        }
        let mut pic = vec![0u32; stage::SCREEN_W * stage::SCREEN_H];
        render_play_pal(&mut pic, img, st, &mut g0, &ps.ppu, false, false);
        v.push(Pic::Rgb(Rc::new(pic)));
    }
    g.world.pal = ps;
    v
}

/// Task calls between the stage load (palette animation ids written) and the first fade-in frame.
const PRE_FADE: usize = 1;

/// The gameplay picture: the stage background at the camera, then the sprites.
pub fn render_play(frame: &mut [u32], img: &[u8], st: &stage::Stage, g: &mut game::Game) {
    render_play_with(frame, img, st, g, true, false);
}

/// The same, optionally without sprites (the stage fades in over its background before any sprite is drawn).
pub fn render_play_with(frame: &mut [u32], img: &[u8], st: &stage::Stage, g: &mut game::Game, sprites: bool, shown: bool) {
    let pal = g.world.pal.ppu;
    render_play_pal(frame, img, st, g, &pal, sprites, shown);
}

/// The same with an explicit PPU palette (the fades lower it).
pub fn render_play_pal(frame: &mut [u32], img: &[u8], st: &stage::Stage, g: &mut game::Game, pal: &[u8; 32], sprites: bool, shown: bool) {
    let list = if shown { if sprites { g.sprites_shown() } else { g.sprites_shown(); Vec::new() } } else if sprites { g.sprites() } else { Vec::new() };
    render_play_with_list(frame, img, st, g, pal, list);
}

/// The picture from an explicit sprite list.
pub fn render_play_with_list(frame: &mut [u32], img: &[u8], st: &stage::Stage, g: &mut game::Game, pal: &[u8; 32], list: Vec<crate::oam::Sprite>) {
    if g.camera.sect & 0xC0 == 0 {
        let cx = if g.cam_again { g.cam_shown } else if g.bg_late { g.prev_cam_x } else { g.camera.x };
        g.cam_shown = cx;
        g.nt.render(frame, &g.stage, pal, cx as usize);
    } else {
        g.nt.render_v(frame, &g.stage, pal, g.camera.fa as usize);
        let _ = img;
    }
    g.stage.draw_sprites(frame, &list, pal);
    // the left 8 pixels are clipped to the backdrop like on the console ($2001 = $18)
    let bd = crate::nes::NES_RGB[(pal[0] & 0x3F) as usize];
    for y in 0..stage::SCREEN_H {
        for x in 0..8 {
            frame[y * stage::SCREEN_W + x] = bd;
        }
    }
}

/// `$B0..$BF` as the password grid and the weapon menus see them: life, the weapon energies, the buster bar.
pub fn ram_b0(g: &game::Game) -> [u8; 16] {
    let mut b = [0u8; 16];
    b[0] = g.player.life;
    b[1..14].copy_from_slice(&g.progress.weapons[1..14]);
    b[14] = 0x9C;
    b
}

/// The picture of a video frame without a logic step: background (palette as it is now) and the sprites of the frame before.
pub fn render_play_pal_again(frame: &mut [u32], img: &[u8], st: &stage::Stage, g: &mut game::Game, pal: &[u8; 32]) {
    g.cam_again = true;
    let l = g.sprites_again();
    render_play_with_list(frame, img, st, g, pal, l);
    g.cam_again = false;
}

/// Frames the dead stage stays on screen unchanged after the countdown before it fades out.
const DEATH_HELD: usize = 0;
