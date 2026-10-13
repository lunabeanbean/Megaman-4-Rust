//! Mega Man's movement, ported from the original game logic (notes/physics-traces.md, notes/player-states.md).
//!
//! Positions are fixed point: 1/256 pixel units. `x` is the 24-bit world X ($0348:$0330.$0318), `y` is the
//! screen-linear Y (screens are 240 px tall; $0390:$0378.$0360). `vy` is the signed 16-bit vertical speed
//! ($03F0.$03D8), **positive = up**. Order of operations inside a frame follows the 6502 code:
//! move, apply gravity, then probe tiles and push out (code_3FF0F8 / code_3FF3ED, bank38_3F.asm).
use crate::anim::{self, Anim};
use crate::stage::Stage;

pub const BTN_RIGHT: u8 = 0x01;
pub const BTN_LEFT: u8 = 0x02;
pub const BTN_DOWN: u8 = 0x04;
pub const BTN_UP: u8 = 0x08;
pub const BTN_A: u8 = 0x80;
pub const BTN_B: u8 = 0x40;

pub const GRAVITY: i32 = 0x40;
pub const RUN_SPEED: i32 = 0x014C;
pub const SLIDE_SPEED: i32 = 0x0280;
pub const SLIDE_FRAMES: u8 = 0x1A;
pub const JUMP_SPEED: i32 = 0x0500;
pub const LADDER_SPEED: i32 = 0x014C;
pub const TERMINAL_VY: i32 = -0x0700;

/// `$30` values handled so far.
pub const STATE_GROUND: u8 = 0;
pub const STATE_AIR: u8 = 1;
pub const STATE_SLIDE: u8 = 2;
pub const STATE_LADDER: u8 = 3;
/// Riding the Rush Marine (code_3C84AD).
pub const STATE_MARINE: u8 = 4;
/// Climbing the wire of weapon 5 (code_3C858D).
pub const STATE_WIRE: u8 = 5;
pub const STATE_HURT: u8 = 6;
pub const STATE_DEAD: u8 = 7;
pub const STATE_SPAWN: u8 = 0x0A;
/// Touching a pod of stage 14 (code_3C8A70): the beam animation plays, on its frame 4 the scene switches.
pub const STATE_POD: u8 = 0x0F;
/// State 9 (code_3C86AD): frozen in place, set by the boss-room object while the boss powers up.
pub const STATE_WAIT: u8 = 9;
/// State `0B` (code_3C87A7): the victory walk and jump after the boss is gone; state 8 (code_3C865D): the teleport-out.
pub const STATE_VICTORY: u8 = 0x0B;
pub const STATE_LEAVE: u8 = 8;
/// Picked up a weapon capsule (`$11`, code_3C8775): frozen for 255 frames, then the teleport-out beam.
pub const STATE_ITEM: u8 = 0x11;
/// State `$10` (code_3C8C49): Mega Man walks right by himself until X $40 (the stage-10 boss room entry).
pub const STATE_AUTOWALK: u8 = 0x10;
/// State `$0C` (code_3C8850): caught by the stage-11 press; when its crush animation (slot 22, `$6C`) ends he falls and is hurt for 6 units.
pub const STATE_CRUSH: u8 = 0x0C;

/// An object Mega Man can stand on or bump into (flag bit 1 = solid on all sides, bit 0 = one-way platform): code_3FF7D8 / code_3FF843.
#[derive(Clone, Copy, Debug, Default)]
pub struct Solid {
    pub xpg: u8,
    pub xpx: u8,
    pub ypx: u8,
    /// `$0528`
    pub flags: u8,
    /// Half width (`$F9F4`) and half height (`$F9B4`) of the object's hitbox size.
    pub w: u8,
    pub h: u8,
    /// `$0558`: the object's animation (the Rush Jet is animation `$24`).
    pub aid: u8,
}

/// What the object system tells the player before his update (slots 1-3 hold his shots, slot 5 the dust effect).
#[derive(Clone, Copy, Debug, Default)]
pub struct ObjEnv {
    /// Lowest free shot slot (1..=3) if the scan of `code_3C8D3E` finds one.
    pub shot_slot: Option<usize>,
    /// A big charged shot (animation `$20`/`$23`) is alive: no new shot.
    pub big_shot: bool,
    /// Slot 5 is free (the slide dust effect needs it).
    pub slot5_free: bool,
    /// Slot 1 is free (`$0301 == 0`): the special weapons use it.
    pub slot1_free: bool,
    /// Highest free shot slot among 3..=1 (the first free one of a downward scan) and the animation id / frame of slots 0..=3.
    pub shot_hi: Option<usize>,
    pub slot_aid: [u8; 4],
    /// Animation id of slot 4 (the wire's `$3D` = it has reached the ceiling).
    pub slot4_aid: u8,
    /// Slot 4 is free (`$0304 == 0`): Rush needs it.
    pub slot4_free: bool,
    /// $013C | $013D != 0: the Flash Stopper's time stop runs.
    pub flash_on: bool,
    /// $013A != 0: a rain (Rain Flush or the Toad Man boss) is falling.
    pub rain_on: bool,
    pub slot_aframe: [u8; 4],
    /// Slot 6 is free (the air bubble).
    pub slot6_free: bool,
    /// Stage id (`$22`).
    pub stage: u8,
    /// Slots 8..=23 are all free (the victory jump waits for the sparkles to be gone).
    pub slots_clear: bool,
    /// Solid objects (slots 23 down to 1, the order the game scans them in); only the first `nsolids` entries are valid.
    pub solids: [Solid; 23],
    pub nsolids: usize,
    /// `$F0`, `$013E`, `$013F`: while `$F0` is 4 (a swaying background) the tile probes see the map shifted by `$013E` (code_3ED552).
    pub f0: u8,
    pub w13e: u8,
    pub w13f: u8,
    /// `$EF`: the sprite priority line (stage 8 uses it as a switch for its slow floor).
    pub ef: u8,
    /// `$95`: the frame counter (the ice physics of stage 8 steps on its parity).
    pub p95: u8,
    /// Animation id of slot 22 (the stage-11 press shows `$6C` while it crushes Mega Man).
    pub crush_aid: u8,
}

/// Things the player creates in the object system; applied by the caller after his update.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerEvent {
    /// A buster shot in `slot` (position/direction captured at the moment of the shot).
    Shot { slot: usize, x: i32, y: i32, flip: bool, anim: u8, low: bool },
    /// The dust effect (object type 1) of a slide start.
    /// The water splash (object `$6F`, animation `$2E`) of code_3FF120; `deep` = he is entering the water (it stays at his Y, else 16 px lower).
    Splash { x: i32, y: i32, flip: bool, deep: bool },
    /// The air bubble of code_3C948C (object `$66`, animation `$7E`, slot 6).
    Bubble { x: i32, y: i32, flip: bool },
    SlideDust { x: i32, y: i32, flip: bool, anim: u8 },
    /// Mega Man explodes into 16 particles (object type `$2E`).
    Death { x: i32, y: i32, flip: bool },
    /// The spark effect of a hit (object type 1 in slot 5).
    HitSpark { enemy: usize, anim: u8 },
    /// A weapon shot in slot 1 (weapons 7..=10, code_3C8F98): `weapon` is its object type, `flip` the facing after the turn.
    Missile { weapon: u8, x: i32, y: i32, flip: bool },
    /// Weapon 11: the held (charging) fireball appears in `slot`.
    Held { slot: usize, x: i32, y: i32, flip: bool },
    /// Weapon 11: the held fireball switches to its full-size animation `$49`.
    Grow { slot: usize },
    /// code_3FFCCF on a slot.
    Kill { slot: usize },
    /// Weapon 12: the time stop starts ($013C:$013D = 300) / counts down one frame.
    FlashStart,
    /// Weapon 5: the wire (type `$0B`, animation `$3C`) shoots up from him in slot 4; `idx` = offset table index (`$3F`, `$40` when he faces right).
    Wire { x: i32, y: i32, flip: bool, idx: u8 },
    /// The hurt handler writes his X into slot 4.
    Slot4X { xpg: u8, xpx: u8 },
    /// Rush items (weapons 1..=3, code_3C8DD5): Rush drops into slot 4 (type 4, animation `$13`) from the top of the screen next to him; `dir` = his facing (1 right, 2 left).
    Rush { x: i32, y: i32, flip: bool, dir: u8 },
    /// Weapon 6: a spring platform (type 6, animation $3E) appears in `slot` at the offset `idx`.
    Coil { slot: usize, idx: u8, x: i32, y: i32, flip: bool },
    /// Weapon 4: the rain cloud (type 5, animation $36) appears above him in slot 1.
    Cloud { x: i32, y: i32, flip: bool },
    FlashTick,
    /// Weapon 13: the skull barrier (type `$0F`, animation `$50`) appears around him in slot 1.
    Barrier { x: i32, y: i32, flip: bool },
    /// Weapon 11: the Pharaoh shot flies off from `slot`; `held` = the up/down bits of the buttons.
    Pharaoh { slot: usize, anim: u8, x: i32, y: i32, flip: bool, held: u8 },
}

#[derive(Clone, Debug)]
pub struct Player {
    pub x: i32,
    pub y: i32,
    pub vy: i32,
    /// `$30`: 0 = ground, 1 = air, 2 = slide.
    pub state: u8,
    /// `$31`: facing / walk direction (`BTN_RIGHT` = 1, `BTN_LEFT` = 2, 0 = none).
    pub facing: u8,
    /// `$0528` bit 6: last horizontal move was to the right (sprite flip flag; decides the slide direction when no button is held).
    pub flip: bool,
    /// `$03C0.$03A8`: horizontal speed used by the move routine (running 1.297, sliding 2.5).
    pub speed: i32,
    /// Stage 3 conveyor: $03A8/$03C0 were lowered this frame; the main loop reloads them from $37/$38 afterwards (code_3EC788).
    conveyor: bool,
    /// The lowered speed the conveyor wrote (a later write of the state code, e.g. the slide start, replaces it and then the reload takes that new value).
    conveyor_low: i32,
    /// Stage 8 (code_3C92E7): the walking speed `$03A8/$03C0` was replaced for this frame by 0.5 (2.0 while sliding); the value to restore and the replacement.
    speed_swap: Option<(i32, i32)>,
    /// Stage 8 ice (`$B0` terrain, code_3C938F/3C93DB): `$A3` slide direction (1 right, 2 left), `$A4` standing on ice, `$A5:$A6` the acceleration step,
    /// `$A7:$A8` the slide speed (8.8 px); the highest terrain types of the three points of the last vertical probe are `$45-$47`.
    pub a3: u8,
    a4: bool,
    pub a56: u16,
    pub a78: u16,
    pts: std::cell::Cell<[u8; 3]>,
    /// `$34`: slide frames left.
    pub slide_timer: u8,
    /// `$0558/$0540/$0570`: the walk logic depends on the animation (see `anim.rs`).
    pub anim: Anim,
    /// Last falling-branch feet probe: `$46` (middle point type, nibble form) and `$11` (probe Y). `code_3C81D6` reads them for the ladder-top check.
    feet_center: u8,
    feet_y: i32,
    /// `$33`: 1 while the shooting pose is shown (animation ids are +1), `$32`: frames until it ends.
    pub shoot: u8,
    pub shoot_timer: u8,
    /// `$35`: charge counter (0 = not charging).
    pub charge: u8,
    /// `$36`: palette-restore step after a release (1 frame for a normal release, 7 after a full charge); the charge counter is frozen while it runs.
    pub flash: u8,
    /// Mega Man's sprite palette colours 1-3 (`$0611-$0613`, `$0631-$0633`) as the charge and its release flash set them in this frame (the last write wins).
    pub pal_mm: Option<([u8; 3], bool)>,
    /// `$3C`: invincibility frames after a hit.
    pub inv: u8,
    /// `$AD`: while non-zero the state handler is skipped (the boss's flash attack); cleared by damage.
    pub ad: u8,
    /// An object (`PlayerOp::SetY`) left his pixel Y at $F0 or more on his own page: the game keeps the page byte, so the fall-death check
    /// (code_3EDC14) only sees him below the screen on the next frame.
    pub raw_below: bool,
    /// `$0133:$0134`: frames left until the stage restarts after death.
    pub dead_timer: u16,
    /// `$A1` extra lives, `$A2` E-tanks.
    pub lives: u8,
    pub etanks: u8,
    /// `$0148`: frames he stands still at the start of the victory state.
    pub hold: u8,
    /// State `$11` sets the checkpoint counter `$1F` to 1 every frame once the beam starts (read by the game, which owns the counter).
    pub set_checkpoint1: bool,
    /// State `$0F` reached animation frame 4: the game must switch the scene (pod room <-> boss room).
    pub pod_switch: bool,
    /// `$0420`: the walk direction the victory state follows (1 right, 2 left).
    pub walk_dir: u8,
    /// `$B0`: life meter (bit 7 = present, low 5 bits = units, 28 = full).
    pub life: u8,
    /// `$0468` / `$0480` while hurt: the state he was in and the probe base id (0 standing, 4 sliding).
    hurt_old: u8,
    hurt_probe: u8,
    /// `$3D`: a probe touched spikes this frame.
    pub spike: std::cell::Cell<bool>,
    /// `$41`: the highest terrain type of the last tile probe a ladder routine made (leaving a ladder copies it to `$44`).
    m41: std::cell::Cell<u8>,
    /// `$3F`: terrain type (probe 0) of the last wall Mega Man walked into on his right; type 5 ends a section (code_3ECA57). Zeroed at the start of every logic frame (code_3EC7BE).
    pub wall_type: u8,
    /// What the object system offers this frame (set by the caller).
    pub env: ObjEnv,
    /// `$F9`: the camera page (stage 1 pushes him sideways on the first eight pages, code_3C92C5).
    pub cam_page: u8,
    /// `$0528` bit 5 of slot 0: Mega Man is drawn behind the background (stage 1 sets it when he spawns, code_3C86F1).
    pub behind: bool,
    /// Stage 14 (code_3C8B38): the 11 zones (page, X, Y) where Mega Man is drawn behind the background, and `$AC` (the last one counts only when it is `$FF`).
    pub zones: Vec<(u8, u8, u8)>,
    pub ac: u8,
    /// Slot 0's type `$0300` was zeroed by the draw pass (code_3EDC00): Mega Man was found on another page than the camera (a scene change that kept the old camera low byte) and is neither drawn nor animated any more.
    pub hidden: bool,
    /// `$44`: the highest terrain type of the last water probe (stage 1 water is `$60`); `$43`: frames of gravity pause left in water.
    water_prev: u8,
    water_wait: u8,
    /// `$A0`: the selected weapon (0 = buster), and `$B0..$BF`: the weapon energies (bit 7 = owned, low bits = units; entry 0 is the life meter and unused here).
    pub weapon: u8,
    pub wen: [u8; 16],
    /// `$04C8` (slot 0's scratch byte 4): the slot of weapon 11's charging fireball, 0 = none.
    pub m4c8: u8,
    /// `$0498` (slot 0's scratch byte 2): set by the Rush Coil bounce; while it is set the air handler does not cut the rise short when A is released.
    pub m498: bool,
    /// `$04B0` (slot 0's scratch byte 3): reset by every vertical step, incremented when he lands on a Rush Jet (animation `$24`); while set he cannot walk or slide.
    pub m4b0: u8,
    /// `$0510` (slot 0's scratch byte 7): frames until the Rush Marine's next energy payment.
    pub m510: u8,
    /// The Rush Marine ran out of energy this frame: the game streams its graphics back in extra frames (`Game::step` runs them).
    pub marine_exit: bool,
    solid_aid: std::cell::Cell<u8>,
    /// `$0131`: the weapon meter in the HUD (bit 7 set, low bits = the weapon slot whose energy it shows); the pause menu sets it.
    pub w131: u8,
    /// Events produced during the last `update`.
    pub events: Vec<PlayerEvent>,
    /// Sound requests made by the last update.
    pub snd: Vec<crate::sound::Req>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Buttons {
    pub held: u8,
    pub pressed: u8,
}

impl Player {
    /// A standing (state 0), airborne (1) or sliding (2) Mega Man with the animation he would have had.
    pub fn new(x: i32, y: i32, vy: i32, state: u8, facing: u8) -> Player {
        Player {
            x,
            y,
            vy,
            state,
            facing,
            flip: facing == BTN_RIGHT,
            speed: if state == STATE_SLIDE { SLIDE_SPEED } else { RUN_SPEED },
            slide_timer: 0,
            conveyor: false,
            conveyor_low: 0,
            speed_swap: None,
            a3: 0,
            a4: false,
            a56: 0,
            a78: 0,
            pts: std::cell::Cell::new([0; 3]),
            cam_page: 0,
            behind: false,
            zones: Vec::new(),
            ac: 0,
            hidden: false,
            water_prev: 0,
            water_wait: 0,
            anim: Anim::new(match state {
                STATE_GROUND => anim::IDLE,
                STATE_SLIDE => anim::SLIDE,
                STATE_LADDER => anim::LADDER,
                _ => anim::AIR,
            }),
            feet_center: 0,
            feet_y: 0,
            shoot: 0,
            shoot_timer: 0,
            charge: 0,
            flash: 0,
            pal_mm: None,
            inv: 0,
            ad: 0,
            raw_below: false,
            life: 0x9C,
            lives: 2,
            dead_timer: 0,
            hold: 0,
            set_checkpoint1: false,
            pod_switch: false,
            walk_dir: 0,
            etanks: 0,
            hurt_old: 0,
            hurt_probe: 0,
            env: ObjEnv::default(),
            events: Vec::new(),
            snd: Vec::new(),
            weapon: 0,
            wen: [0; 16],
            m4c8: 0,
            m498: false,
            m4b0: 0,
            m510: 0,
            marine_exit: false,
            solid_aid: std::cell::Cell::new(0),
            w131: 0,
            spike: std::cell::Cell::new(false),
            m41: std::cell::Cell::new(0),
            wall_type: 0,
        }
    }

    /// Mega Man as the stage (re)start creates him (code_3EC65B): the teleport-in beam at world page `page`.
    pub fn spawn_at(page: i32, old: &Player, life: u8, lives: u8, etanks: u8) -> Player {
        // only the pixel bytes are set: the sub-pixel bytes and `$31` keep what he had
        let mut p = Player::new((page << 16) | (0x80 << 8) | (old.x & 0xFF), (0x64 << 8) | (old.y & 0xFF), -0x100, STATE_SPAWN, old.facing);
        p.flip = false;
        p.anim = Anim::new(anim::SPAWN_WAIT);
        p.life = life;
        p.lives = lives;
        p.etanks = etanks;
        p.water_prev = old.water_prev; // $43/$44 are not reset by the stage reload
        p.water_wait = old.water_wait;
        p.a56 = old.a56; // $A5/$A6 are not reset by the spawn (only $A3, $A7, $A8)
        p.zones = old.zones.clone();
        p.ac = old.ac;
        p.wen = old.wen; // $A0 is cleared by the spawn (code_3EC660), the energies stay
        p
    }

    /// The Y the tile probes start from: with `$F0` = 4 the entity's Y is lowered by `$013E` (code_3ED552 for vertical probes, code_3ED566 for
    /// horizontal ones), except near the bottom of the screen; vertical probes on pages other than `$0C` are shifted whenever `$013F` bit 2 is clear.
    fn pys(&self, vertical: bool) -> i32 {
        let e = &self.env;
        if e.f0 != 4 {
            return self.py();
        }
        let limit = if self.state == STATE_SLIDE { 0xB7 } else { 0xB5 };
        let by_height = crate::game::split_y(self.y).1 < limit;
        let apply = if vertical && e.w13f & 4 == 0 && ((self.x >> 16) as u8) != 0x0C { true } else { by_height };
        if apply {
            // the game subtracts in 8 bits: near the top of the screen the result wraps to a large value
            let (pg, px) = crate::game::split_y(self.y);
            if pg == 0 {
                px.wrapping_sub(e.w13e) as i32
            } else {
                self.py() - e.w13e as i32
            }
        } else {
            self.py()
        }
    }

    /// Terrain lookup for a probe point; spikes (type 3) set the death flag `$3D` like code_3ED3A6 / code_3ED4DF do for slot 0.
    fn terr(&self, stage: &Stage, x: i32, y: i32) -> u8 {
        self.terr_lim(stage, x, y, 240)
    }

    fn terr_lim(&self, stage: &Stage, x: i32, y: i32, lim: i32) -> u8 {
        let t = stage.terrain_lim(x, y, lim);
        if t == 3 {
            self.spike.set(true);
        }
        t
    }

    /// code_3A8243: a hazard (spikes, being crushed) kills him: no damage, no life change, the same death sequence.
    pub fn kill_by_hazard(&mut self) {
        self.state = STATE_DEAD;
        self.dead_timer = 300;
        self.snd_music(0xF0, 0x824C); // code_3A8243
        self.snd(0x25, 0x8251);
        self.events.push(PlayerEvent::Death { x: self.x, y: self.y, flip: self.flip });
    }

    /// code_3A805A: is the body probe (`$3E`, `$3F` while sliding) inside solid ground (crushed)?
    /// code_3A805A: crushed inside solid ground. With `$F0` = 4 (a scrolling-background object) the check is skipped near the bottom of the screen.
    pub fn crushed(&self, stage: &Stage, f0: u8) -> bool {
        if f0 == 4 {
            let limit = if self.state == STATE_SLIDE { 0xB7 } else { 0xB5 };
            if crate::game::split_y(self.y).1 >= limit {
                return false;
            }
        }
        let id = if self.state == STATE_SLIDE { 0x3F } else { 0x3E };
        self.horiz_raw(stage, id).0.iter().any(|t| t & 1 != 0)
    }

    pub fn px(&self) -> i32 {
        self.x >> 8
    }
    pub fn py(&self) -> i32 {
        self.y >> 8
    }

    /// Base hitbox id: 0 standing, 4 sliding (the vertical probes use `id` for the feet and `id + 1` for the head,
    /// the horizontal ones `id` for the right side and `id + 1` for the left).
    fn hitbox(&self) -> usize {
        if self.state == STATE_HURT {
            self.hurt_probe as usize
        } else if self.state == STATE_SLIDE {
            4
        } else {
            0
        }
    }

    /// OR of the terrain types along a vertical probe line (code_3ED2FC): `fixed` is the Y offset, `along` the X offsets.
    /// Type `$40` counts as `$20` here, as in code_3ED4C2.
    fn probe_types(&self, stage: &Stage, id: usize) -> u8 {
        let p = stage.hitboxes.vertical(id);
        let y = self.pys(true) + p.fixed;
        let lim = if p.fixed < 0 { 256 } else { 240 };
        p.along.iter().fold(0, |acc, dx| acc | map_type(self.terr_lim(stage, self.px() + dx, y, lim)))
    }

    /// Is any point of vertical probe `id` solid? Also returns the probe's Y.
    fn vertical_solid(&self, stage: &Stage, id: usize) -> (bool, i32) {
        let p = stage.hitboxes.vertical(id);
        let y = self.pys(true) + p.fixed;
        let lim = if p.fixed < 0 { 256 } else { 240 };
        let types: Vec<u8> = p.along.iter().map(|dx| self.terr_lim(stage, self.px() + dx, y, lim)).collect();
        if let Some(v) = self.virtual_floor(y, lim, &types) {
            return (true, v);
        }
        self.m41.set(types.iter().copied().max().unwrap_or(0));
        let mut pts = [0u8; 3];
        for (i, t) in types.iter().take(3).enumerate() {
            pts[i] = *t;
        }
        self.pts.set(pts);
        (types.iter().fold(false, |s, t| s | (t & 1 != 0)), y)
    }

    /// code_3ED3FD (end of every vertical probe): while a sway object is active ($F0 = 4, $013F bit 2 clear) and Mega Man is not on page $0C, a probe that
    /// found nothing at all counts as solid ground when its unshifted Y is at least $C0 (the bottom of the screen is a floor). Returns the Y to push up from.
    fn virtual_floor(&self, probe_y: i32, lim: i32, types: &[u8]) -> Option<i32> {
        let e = &self.env;
        if e.f0 != 4 || e.w13f & 4 != 0 || (self.x >> 16) as u8 == 0x0C || !(0..lim).contains(&probe_y) || types.iter().any(|&t| t != 0) {
            return None;
        }
        let sum = (probe_y as u8).wrapping_add(e.w13e);
        if sum >= 0xC0 {
            Some(sum as i32)
        } else {
            None
        }
    }

    /// Is any point of horizontal probe `id` solid? Also returns the probe's X.
    fn side_solid(&self, stage: &Stage, id: usize) -> (bool, i32) {
        let (types, x) = self.horiz_raw(stage, id);
        (types.iter().any(|t| t & 1 != 0), x)
    }

    /// Verifier hook: `$43` (gravity pause counter) and `$44` (last water probe) as they were at the first recorded row.
    /// The press has caught him (code_3DB8A9): animation 7, falling at 4 px/frame.
    pub fn crushed_by_press(&mut self) {
        self.shoot = 0;
        self.anim.set(7);
        self.vy = -0x400;
    }

    pub fn set_water(&mut self, wait: u8, prev: u8) {
        self.water_wait = wait;
        self.water_prev = prev;
    }

    /// Gravity, as in code_3FF2F6: vy -= $99 with a clamp at -7 px/frame.
    fn gravity(&mut self) {
        if self.water_wait != 0 {
            // code_3FF2F6: in water gravity only acts on every third frame
            self.water_wait -= 1;
            if self.water_wait != 0 {
                return;
            }
        }
        self.vy -= GRAVITY;
        if self.vy < 0 && (self.vy >> 8) < -7 {
            self.vy = TERMINAL_VY;
        }
    }

    /// code_3FF89A: horizontal distance to a solid object: (|dx|, the game's Z flag = the pages differ by an even number).
    fn solid_dx(&self, o: &Solid) -> (u8, bool) {
        let (pxg, pxx) = ((self.x >> 16) as u8, (self.x >> 8) as u8);
        let (d, b1) = pxx.overflowing_sub(o.xpx);
        let (pg, b2) = pxg.overflowing_sub(o.xpg);
        let (pg, b3) = pg.overflowing_sub(b1 as u8);
        if !(b2 || b3) {
            (d, pg & 1 == 0)
        } else {
            let a = (!pg).wrapping_add((d == 0) as u8); // EOR #$FF / ADC #$00 with the carry of the low byte's negation
            ((!d).wrapping_add(1), a & 1 == 0)
        }
    }

    /// code_3FF8BD: (`$12` = the object's half height, shortened while sliding; `$11` = |dy|; carry = Mega Man is at or below the object).
    fn solid_dy(&self, o: &Solid) -> (u8, u8, bool) {
        let sliding = self.anim.id == 0x10;
        let mut h = o.h;
        if sliding {
            h = h.wrapping_sub(2);
        }
        let py = crate::game::split_y(self.y).1;
        let (d, borrow) = py.overflowing_sub(o.ypx);
        let below = !borrow;
        let dy = if below { d } else { (!d).wrapping_add(1) };
        if below && sliding {
            h = h.wrapping_sub(4);
        }
        (h, dy, below)
    }

    /// code_3FF843, after a horizontal move: how deep Mega Man overlaps a solid object (at most 8 px).
    fn solid_h(&self) -> Option<u8> {
        for o in &self.env.solids[..self.env.nsolids] {
            if o.flags & 2 == 0 {
                continue;
            }
            let (dx, same) = self.solid_dx(o);
            if !same {
                continue;
            }
            let (h, dy, _) = self.solid_dy(o);
            if dy >= h || o.w < dx {
                continue;
            }
            return Some((o.w - dx).min(8));
        }
        None
    }

    /// code_3FF7D8, after a vertical move: how deep Mega Man overlaps a solid object (at most 8 px). One-way platforms (bit 0) only count while he falls.
    fn solid_v(&self) -> Option<u8> {
        for o in &self.env.solids[..self.env.nsolids] {
            if o.flags & 3 == 0 {
                continue;
            }
            if o.flags & 1 != 0 && self.vy >= 0 {
                continue;
            }
            let (dx, same) = self.solid_dx(o);
            if !same {
                continue;
            }
            let (h, dy, below) = self.solid_dy(o);
            if below && o.flags & 1 != 0 {
                continue;
            }
            if dx >= o.w || h < dy {
                continue;
            }
            self.solid_aid.set(o.aid);
            return Some((h - dy).min(8));
        }
        None
    }

    /// code_3FF92E: push Mega Man up by `depth` (the raw pixel byte borrows from the screen number).
    fn push_up_raw(&mut self, depth: u8) {
        let (pg, px) = crate::game::split_y(self.y);
        let (npx, b) = px.overflowing_sub(depth);
        let npg = if b { pg.wrapping_sub(1) } else { pg };
        self.y = ((npg as i8 as i32 * 240 + npx as i32) << 8) | (self.y & 0xFF);
    }

    /// An object put his pixel Y at `ypx` (`STA $0378`) and ran code_3FF3C1: the standing ground probe, and if it is solid he is pushed up onto the tile top.
    pub fn ride_ground(&mut self, stage: &Stage, ypx: u8) {
        let (pg, _) = crate::game::split_y(self.y);
        self.y = ((pg as i8 as i32 * 240 + ypx as i32) << 8) | (self.y & 0xFF);
        let id = if self.state == STATE_SLIDE { 4 } else { 0 };
        let (solid, probe_y) = self.vertical_solid(stage, id);
        if solid {
            self.y -= (probe_y & 0x0F) << 8; // code_3ED9E7
        }
    }

    /// code_3FF93F: push Mega Man down by `depth`.
    fn push_down_raw(&mut self, depth: u8) {
        let (pg, px) = crate::game::split_y(self.y);
        let (mut npx, c) = px.overflowing_add(depth);
        let mut npg = pg;
        if c {
            npg = npg.wrapping_add(1);
        } else if npx >= 0xF0 {
            npx = npx.wrapping_add(0x10);
            npg = npg.wrapping_add(1);
        }
        self.y = ((npg as i8 as i32 * 240 + npx as i32) << 8) | (self.y & 0xFF);
    }

    /// The ground probe of a falling step (code_3FF1B0): gravity, then the feet probe; pushes up onto the tile top when it is solid (and sets vy = -1.0).
    fn fall_probe(&mut self, stage: &Stage, id: usize) -> bool {
        self.gravity();
        let p = stage.hitboxes.vertical(id);
        let probe_y = self.pys(true) + p.fixed;
        let types: Vec<u8> = p.along.iter().map(|dx| self.terr(stage, self.px() + dx, probe_y)).collect();
        self.feet_center = types.get(1).copied().unwrap_or(0);
        self.feet_y = probe_y;
        let mut pts = [0u8; 3];
        for (i, t) in types.iter().take(3).enumerate() {
            pts[i] = *t;
        }
        self.pts.set(pts);
        self.m41.set(types.iter().copied().max().unwrap_or(0));
        // code_3FF1B6: a ladder top (type $40 as the highest type under the feet) is a one-way floor for its upper half
        let top = types.iter().copied().max() == Some(4) && (probe_y & 0x0F) < 8;
        let virt = self.virtual_floor(probe_y, 240, &types);
        let probe_y = virt.unwrap_or(probe_y);
        if types.iter().any(|t| t & 1 != 0) || top || virt.is_some() {
            // code_3ED9E7: push up so the feet sit on the tile top, then code_3FF324
            self.y -= (probe_y & 0x0F) << 8;
            self.vy = -0x100;
            true
        } else if stage.id == 3 && types.iter().copied().max() == Some(0xC) {
            // code_3FF1CE: stage 3 terrain $C0 (conveyor) is a floor that is not solid: no push-up, walking speed ($37/$38) - 0.5, vy = -0.13, landed
            let base = if self.state == STATE_SLIDE { SLIDE_SPEED } else { RUN_SPEED };
            self.speed = base - 0x80;
            self.conveyor_low = self.speed;
            self.conveyor = true;
            self.vy = -0x22;
            true
        } else {
            false
        }
    }

    /// code_3C948C (every frame before the state handler): with probe `$38` entirely in water (type $60) he releases an air bubble (object `$66`, slot 6).
    fn bubble_check(&mut self, stage: &Stage) {
        if self.state >= 5 || !self.env.slot6_free || self.probe_types(stage, 0x38) != 6 {
            return;
        }
        self.events.push(PlayerEvent::Bubble { x: self.x, y: self.y, flip: self.flip });
    }

    /// Start of code_3FF0F8 for slot 0: probe 2 looks for water (terrain type $60). Entering or leaving it makes a splash (object $6F, slot 5)
    /// when slot 5 is free; inside water the gravity pause counter `$43` is armed.
    fn water_probe(&mut self, stage: &Stage) {
        let p = stage.hitboxes.vertical(2);
        let y = self.pys(true) + p.fixed;
        let lim = if p.fixed < 0 { 256 } else { 240 };
        let a = p.along.iter().map(|dx| self.terr_lim(stage, self.px() + dx, y, lim)).max().unwrap_or(0);
        let prev = self.water_prev;
        let change = if a == prev {
            false
        } else if a == 6 {
            prev == 0
        } else {
            prev == 6 && a == 0
        };
        if change && self.env.slot5_free {
            self.events.push(PlayerEvent::Splash { x: self.x, y: self.y, flip: self.flip, deep: a == 6 });
            self.snd(0x1E, 0xF12A); // code_3FF127
            return;
        }
        self.water_prev = a;
        if a == 6 {
            if self.water_wait == 0 {
                self.water_wait = 3;
            }
        } else {
            self.water_wait = 0;
        }
    }

    /// code_3FF0F8 for slot 0 with the current hitbox. Returns true when Mega Man landed on something (carry set in the original).
    pub fn vertical_step(&mut self, stage: &Stage) -> bool {
        self.water_probe(stage);
        let id = self.hitbox();
        let old_py = crate::game::split_y(self.y).1;
        self.m4b0 = 0; // code_3FF0F8: STX $04B0
        if self.vy >= 0 {
            self.y -= self.vy; // rising: code_3FF2D8
            if let Some(depth) = self.solid_v() {
                // an object above him: pushed down out of it; if that left him lower than before, the feet probe runs instead of the head probe
                self.push_down_raw(depth);
                let new_py = crate::game::split_y(self.y).1;
                if old_py < new_py {
                    let (solid, probe_y) = self.vertical_solid(stage, id);
                    if solid {
                        self.y -= (probe_y & 0x0F) << 8;
                    }
                } else {
                    self.gravity();
                    let (solid, probe_y) = self.vertical_solid(stage, id + 1);
                    if solid {
                        self.y += (0x10 - (probe_y & 0x0F)) << 8;
                    }
                }
                self.vy = -0x100; // code_3FF228
                return false;
            }
            self.gravity();
            let (solid, probe_y) = self.vertical_solid(stage, id + 1);
            if solid {
                // code_3ED9CD: push down to the next tile boundary, then code_3FF324: vy = -1.0
                self.y += (0x10 - (probe_y & 0x0F)) << 8;
                self.vy = -0x100;
            }
            false
        } else {
            self.y -= self.vy; // falling: code_3FF2B8 (vy is negative)
            if let Some(depth) = self.solid_v() {
                // landed on an object: pushed up out of it, then vy = -2.0 (code_3FF19C)
                if self.solid_aid.get() == 0x24 {
                    self.m4b0 = self.m4b0.wrapping_add(1); // standing on a Rush Jet
                }
                self.push_up_raw(depth);
                let new_py = crate::game::split_y(self.y).1;
                if old_py > new_py {
                    let (solid, probe_y) = self.vertical_solid(stage, id + 1);
                    if solid {
                        self.y += (0x10 - (probe_y & 0x0F)) << 8;
                    }
                } else {
                    self.fall_probe(stage, id);
                }
                self.vy = -0x200;
                return true;
            }
            self.fall_probe(stage, id)
        }
    }

    /// code_3FF3ED: move sideways by `speed`, then push out of a wall. `dir` is `BTN_RIGHT` or `BTN_LEFT`.
    /// Returns true when a wall was hit (carry set in the original).
    fn horizontal_step(&mut self, stage: &Stage, dir: u8) -> bool {
        let id = self.hitbox();
        self.horizontal_step_id(stage, dir, id)
    }

    /// code_3FF37C: an object moves Mega Man sideways by its own X speed (`xvs`/`xvp`) in its direction flags `dir`, with the usual wall probes
    /// (probe 4 while sliding, else 0). His facing is restored.
    pub fn push_h(&mut self, stage: &Stage, xvs: u8, xvp: u8, dir: u8) {
        let (flip, saved) = (self.flip, self.speed);
        self.speed = (xvp as i32) << 8 | xvs as i32;
        let id = if self.state == STATE_SLIDE { 4 } else { 0 };
        let btn = if dir & 3 == 0 { 0 } else if dir & 1 != 0 { BTN_RIGHT } else { BTN_LEFT };
        self.horizontal_step_id(stage, btn, id);
        self.flip = flip;
        self.speed = saved; // the walking code reloads $03A8/$03C0 from its own copy ($37/$38) every frame
    }

    fn horizontal_step_id(&mut self, stage: &Stage, dir: u8, id: usize) -> bool {
        if dir & BTN_RIGHT != 0 {
            self.flip = true; // code_3FF000
            let old = self.px();
            self.x += self.speed;
            if let Some(depth) = self.solid_h() {
                // a solid object: pushed back out of it; then the left-side tile probe if he went backwards overall, else the right-side one
                self.x -= (depth as i32) << 8;
                if old > self.px() {
                    let (solid, probe) = self.side_solid(stage, id + 1);
                    if solid {
                        self.x += (0x10 - (probe & 0x0F)) << 8;
                    }
                } else {
                    self.right_wall(stage, id);
                }
                return true;
            }
            self.right_wall(stage, id)
        } else if dir & BTN_LEFT != 0 {
            self.flip = false; // code_3FF049
            let old = self.px();
            self.x -= self.speed;
            if let Some(depth) = self.solid_h() {
                self.x += (depth as i32) << 8;
                if old >= self.px() {
                    let (solid, probe) = self.side_solid(stage, id + 1);
                    if solid {
                        self.x += (0x10 - (probe & 0x0F)) << 8;
                    }
                } else {
                    self.right_wall(stage, id);
                }
                return true;
            }
            let (solid, probe) = self.side_solid(stage, id + 1);
            if solid {
                self.x += (0x10 - (probe & 0x0F)) << 8; // code_3ED9B7
            }
            solid
        } else {
            false
        }
    }

    /// The right-side wall probe (code_3FF032): returns true and pushes out when the probe is solid.
    fn right_wall(&mut self, stage: &Stage, id: usize) -> bool {
        let (solid, probe) = self.side_solid(stage, id);
        if std::env::var("MM4_DBG5").is_ok() {
            eprintln!("right_wall x={:06X} y={:06X} id={} types={:?} probe={}", self.x, self.y, id, self.horiz_raw(stage, id).0, probe);
        }
        if solid {
            self.wall_type = map_type(self.horiz_raw(stage, id).0[0]); // code_3FF040: $3F = $45
            self.x -= (probe & 0x0F) << 8; // code_3ED99F
        }
        solid
    }

    /// code_3C92C5 (stage 1), called right after the vertical step of the ground/air and slide states (carry = landed): on camera pages 0-7 the
    /// wind pushes him left by 0.5 px while he is in the air; from page 8 the terrain under him (probe $10) decides: $80 pushes right, $A0 left
    /// (0.5 px, through the normal horizontal move) and $C0 pulls down (vy - 0.297).
    fn stage1_current(&mut self, stage: &Stage, landed: bool) {
        if stage.id == 8 {
            // code_3C92E7: while `$EF` is non-zero (pages 0-5) terrain `$C0` under the feet slows the walk to 0.5 px (2.0 px while sliding) for this frame
            if self.env.ef != 0 && self.probe_types(stage, 0x10) == 0xC {
                let new = if self.state == STATE_SLIDE { 0x200 } else { 0x80 };
                if self.speed_swap.is_none() {
                    self.speed_swap = Some((self.speed, new));
                }
                self.speed = new;
            }
            return;
        }
        if stage.id != 1 {
            return;
        }
        let dir = if self.cam_page < 8 {
            if landed {
                return;
            }
            BTN_LEFT
        } else {
            match self.probe_types(stage, 0x10) {
                8 => BTN_RIGHT,
                0xA => BTN_LEFT,
                0xC => {
                    self.vy -= 0x4C;
                    return;
                }
                _ => return,
            }
        };
        self.push_h(stage, 0x80, 0, dir);
    }

    /// One frame: the state handler (`step`), then the animation tick that the drawing code does afterwards.
    pub fn update(&mut self, stage: &Stage, b: Buttons) {
        self.update_logic(stage, b);
        self.anim.advance(&stage.anims);
    }

    /// The player's part of a game frame without the animation tick (the draw pass does that at the end of the frame, after enemies may have hurt him).
    pub fn update_logic(&mut self, stage: &Stage, b: Buttons) {
        if !self.zones.is_empty() {
            self.zone_priority();
        }
        self.events.clear();
        self.raw_below = false;
        self.pal_mm = None;
        self.spike.set(false);
        self.wall_type = 0;
        // code_3C8032: the shooting pose times out, the charge grows while B is held, the invincibility timer runs down
        if self.shoot_timer != 0 {
            self.shoot_timer -= 1;
            if self.shoot_timer == 0 {
                self.anim.id = self.anim.id.wrapping_sub(self.shoot);
                self.shoot = 0;
            }
        }
        if self.flash != 0 {
            if self.weapon == 0 {
                self.pal_mm = Some((release_palette(self.flash), true)); // code_3C8057 (INC $18)
            }
            self.flash = (self.flash + 1) & 7; // the palette restore after a release; the charge counter does not grow meanwhile
        } else if self.charge != 0 && self.state < 4 && b.held & BTN_B != 0 {
            self.charge += 1;
            if self.charge == 0x32 {
                self.snd(0x22, 0x8090); // code_3C808B
            }
            if self.charge >= 0x64 {
                self.charge = 0x5C;
                self.snd(0x27, 0x809F); // code_3C809A
            }
            if self.weapon == 0 {
                // code_3C809F: from a charge of $20 on the outline colour flashes ($926D/$927D/$928D)
                let hi = (self.charge >> 3) & 0xFC;
                if hi != 0 {
                    self.pal_mm = Some((charge_palette((hi | ((self.charge >> 1) & 3)) as usize), true));
                }
            }
        }
        if self.inv != 0 {
            self.inv -= 1;
        }
        if self.ad != 0 {
            self.anim.tick = 0; // code_3C80F2
            self.ad -= 1;
            return;
        }
        self.step(stage, b);
        if let Some((old, new)) = self.speed_swap.take() {
            if self.speed == new {
                self.speed = old; // the main loop reloads $03A8/$03C0 from $37/$38
            }
        }
        if self.conveyor {
            self.conveyor = false;
            if self.speed == self.conveyor_low {
                self.speed += 0x80; // the main loop reloads $03A8/$03C0 from $37/$38
            }
        }
    }

    /// The state handlers (code_3C8130 ground, code_3C82C8 air, code_3C8353 slide, code_3C8214 walking).
    fn step(&mut self, stage: &Stage, b: Buttons) {
        self.bubble_check(stage);
        let dir = b.held & (BTN_LEFT | BTN_RIGHT);
        let dir = if dir == (BTN_LEFT | BTN_RIGHT) { 0 } else { dir };
        if self.state == STATE_DEAD {
            return; // code_3C865C is a bare RTS: the death sequence runs in the main loop
        }
        if self.state == STATE_SLIDE {
            self.slide_step(stage, b);
            return;
        }
        if self.state == STATE_LADDER {
            self.ladder_step(stage, b);
            return;
        }
        if self.state == STATE_WIRE {
            self.wire_step(stage, b);
            return;
        }
        if self.state == STATE_MARINE {
            self.marine_step(stage, b);
            return;
        }
        if self.state == STATE_HURT {
            self.hurt_step(stage);
            return;
        }
        if self.state == STATE_SPAWN {
            self.spawn_step(stage);
            return;
        }
        if self.state == STATE_POD {
            // code_3C8A70: nothing happens until animation frame 4, then the scene change runs (game.rs `pod_switch`)
            if self.anim.frame == 4 {
                self.pod_switch = true;
            }
            return;
        }
        if self.state == STATE_WAIT {
            self.wait_step(stage);
            return;
        }
        if self.state == STATE_VICTORY {
            self.victory_step(stage);
            return;
        }
        if self.state == STATE_LEAVE {
            self.leave_step();
            return;
        }
        if self.state == STATE_CRUSH {
            if self.env.crush_aid == 0x6C {
                return;
            }
            if !self.vertical_step(stage) {
                return;
            }
            self.anim.set(0x11);
            self.state = STATE_HURT;
            self.shoot = 0;
            self.hurt_old = 0;
            self.hurt_probe = 0;
            let units = (self.life & 0x1F).saturating_sub(6);
            self.life = 0x80 | units;
            if units == 0 {
                self.life = 0x80;
                self.state = STATE_GROUND;
                self.spike.set(true); // $3D = $FF: the death check of the next code_3A8003
            }
            return;
        }
        if self.state == STATE_AUTOWALK {
            if self.anim.id != anim::RUN {
                self.anim.set(anim::RUN);
            }
            self.x += self.speed; // code_3FF22D: no wall probes
            if (self.px() & 0xFF) > 0x40 {
                self.x = (self.x & !0xFF00) | (0x40 << 8);
                self.state = STATE_GROUND;
            }
            return;
        }
        if self.state == STATE_ITEM {
            self.item_step(stage);
            return;
        }
        let landed = self.ground_vertical(stage);
        if landed && self.state == STATE_AIR {
            // code_3C8320: landing sets animation 4 (run); falls through to code_3C833D: moves in the same frame
            self.snd(0x23, 0x8329);
            self.m498 = false;
            self.state = STATE_GROUND;
            self.anim.set(anim::RUN + self.shoot);
            self.ice_land(b);
            self.facing = dir;
            self.horizontal_step(stage, dir);
            self.shoot_then_ladder(stage, b);
            return;
        }
        if !landed {
            self.airborne(stage, b, dir);
            return;
        }
        // on the ground (state 0, code_3C8130)
        if b.pressed & BTN_A != 0 {
            if b.held & BTN_DOWN == 0 || self.m4b0 != 0 {
                self.jump(stage, b, dir);
                return;
            }
            if self.start_slide(stage, b) {
                return;
            }
            // a wall right in front: code_3C81D6, falls into the walking code below
        }
        // code_3C81D6: Down while standing on a ladder top climbs down into the ladder
        if b.held & BTN_DOWN != 0 && self.feet_center == 4 {
            self.px_snap();
            self.y = ((self.feet_y & 0xF0) << 8) | (self.y & 0xFF);
            self.vy = LADDER_SPEED;
            self.state = STATE_LADDER;
            self.anim.set(anim::LADDER_TOP + self.shoot);
            self.shoot_then_ladder(stage, b); // code_3C82A9
            return;
        }
        // code_3C820C: riding a Rush Jet he cannot walk
        if self.m4b0 != 0 {
            self.idle();
            self.ice_idle(stage);
            self.shoot_then_ladder(stage, b);
            return;
        }
        // walking, code_3C8214: the animation id decides what a direction press does
        match self.anim.id {
            // running (4/5, landing or after the start step): keep going in the facing direction, reverse, or stop
            4 | 5 => {
                if dir == 0 {
                    self.anim.set(anim::WALK_START + self.shoot); // code_3C8292: stopping plays the start/stop animation
                    self.ice_idle(stage);
                } else if dir & self.facing != 0 {
                    self.walk_move(stage, self.facing);
                    self.footstep();
                } else {
                    self.start_step(stage, dir);
                }
            }
            // start/stop animation: no movement until its frame index leaves 0, then break into the run animation
            0x0D..=0x0F => {
                if self.anim.frame == 0 {
                    // code_3C82A6: nothing happens (a held direction is ignored)
                    self.ice_idle(stage);
                } else if dir == 0 {
                    self.idle();
                    self.ice_idle(stage);
                } else {
                    self.facing = dir;
                    self.anim.set(anim::RUN + self.shoot);
                    self.walk_move(stage, self.facing);
                    self.footstep();
                }
            }
            // standing (animation 1) or anything else: a direction starts a step
            _ => {
                if dir == 0 {
                    self.idle();
                    self.ice_idle(stage);
                } else {
                    self.start_step(stage, dir);
                }
            }
        }
        self.shoot_then_ladder(stage, b); // every walking path ends in code_3C82A9
    }

    /// code_3C8CBD for the buster: B newly pressed starts a shot (and the charge counter), B released after charging fires a charged one.
    fn try_shoot(&mut self, b: Buttons) {
        if self.weapon != 0 {
            self.try_weapon(b);
            return;
        }
        let mut anim_id = 0x18u8;
        if self.charge != 0 {
            if b.held & BTN_B != 0 {
                return; // still charging
            }
            anim_id = 0x21;
            let c = self.charge;
            self.charge = 0;
            self.flash = 7; // code_3C8CE8
            if c < 0x20 {
                return;
            }
            self.flash = if c >= 0x5C { 0 } else if c >= 0x40 { 9 } else { 8 };
            self.pal_mm = Some((release_palette(self.flash), false)); // code_3C8D13: no INC $18, the upload waits for the next frame's step
            if c >= 0x5C {
                anim_id = 0x20;
            }
            self.flash = (self.flash + 1).min(7); // code_3C8D23: INC $36, at most 7
        } else if b.pressed & BTN_B == 0 {
            return;
        }
        self.charge = 1;
        // free shot slot: scan 3,2,1 (the last free one wins); a big charged shot in flight blocks everything
        let Some(slot) = self.env.shot_slot else { return };
        if self.env.big_shot {
            return;
        }
        if self.state == STATE_LADDER && b.held & 3 != 0 {
            self.facing = b.held & 3;
            self.flip = self.facing & 1 != 0; // code_3FF4F6
        }
        self.shoot_timer = 0x10;
        if self.shoot == 0 {
            self.shoot = 1;
            self.anim.id = self.anim.id.wrapping_add(1);
        }
        self.snd(if anim_id == 0x20 { 0x3D } else { 0x21 }, 0x8DB4); // code_3C8DB1: $12 = $21, or $3D for a full charge
        self.events.push(PlayerEvent::Shot { slot, x: self.x, y: self.y, flip: self.flip, anim: anim_id, low: false });
    }

    /// code_3C8CCC with a special weapon selected: B newly pressed fires an uncharged buster shot.
    fn plain_buster(&mut self, b: Buttons) {
        if b.pressed & BTN_B == 0 {
            return;
        }
        self.charge = 1;
        let Some(slot) = self.env.shot_slot else { return };
        if self.env.big_shot {
            return;
        }
        self.shoot_timer = 0x10;
        if self.shoot == 0 {
            self.shoot = 1;
            self.anim.id = self.anim.id.wrapping_add(1);
        }
        self.snd(0x21, 0x8DB4);
        self.events.push(PlayerEvent::Shot { slot, x: self.x, y: self.y, flip: self.flip, anim: 0x18, low: self.weapon == 3 }); // code_3C8DA0: the Marine rider shoots 8 px lower
    }

    /// code_3FFD75: the energy cost of the selected weapon (table `$FD8E`), the owned bit stays set.
    pub fn spend_energy(&mut self) {
        const COST: [u8; 14] = [0, 2, 1, 1, 4, 2, 2, 1, 1, 1, 1, 1, 4, 2];
        let e = &mut self.wen[self.weapon as usize];
        *e = (*e & 0x7F).saturating_sub(COST[self.weapon as usize]) | 0x80;
    }

    /// code_3C8CBD for a special weapon (jump table `$91EB`/`$91F9`). Weapons 7..=10 share code_3C8F98: one shot in slot 1.
    fn try_weapon(&mut self, b: Buttons) {
        match self.weapon {
            7..=10 => {
                if self.wen[self.weapon as usize] & 0x7F == 0 || !self.env.slot1_free || b.pressed & BTN_B == 0 || self.shoot != 0 {
                    return;
                }
                if b.held & 3 != 0 {
                    self.facing = b.held & 3;
                    self.flip = self.facing & 1 != 0; // code_3FF4F6
                }
                self.snd([0x21, 0x19, 0x1A, 0x21][(self.weapon as usize).wrapping_sub(7) & 3], 0x9003); // code_3C9000: table $9220[weapon]
                self.events.push(PlayerEvent::Missile { weapon: self.weapon, x: self.x, y: self.y, flip: self.flip });
                self.spend_energy();
                self.shoot_timer = 0x10;
                self.shoot = 1;
                self.anim.set(self.anim.id.wrapping_add(1));
            }
            11 => self.weapon_pharaoh(b),
            5 => {
                // code_3C8E9D: Up + B while standing shoots a wire up and starts the climb (state 5)
                if self.wen[5] & 0x7F == 0 || !self.env.slot4_free || b.pressed & BTN_B == 0 || b.held & BTN_UP == 0 || self.anim.id != 1 {
                    return;
                }
                if b.held & 3 != 0 {
                    self.facing = b.held & 3;
                    self.flip = self.facing & 1 != 0;
                }
                self.anim.set(0x39);
                self.state = STATE_WIRE;
                let idx = if self.flip { 0x40 } else { 0x3F };
                self.events.push(PlayerEvent::Wire { x: self.x, y: self.y, flip: self.flip, idx });
                self.spend_energy();
            }
            1 | 2 | 3 => {
                // code_3C8DD5: B calls Rush (while he is already there B fires plain buster shots)
                if self.wen[self.weapon as usize] & 0x7F == 0 {
                    return;
                }
                if !self.env.slot4_free || b.pressed & BTN_B == 0 {
                    self.plain_buster(b);
                    return;
                }
                let mut dir = if self.flip { 1 } else { 2 };
                if b.held & 3 != 0 {
                    dir = b.held & 3;
                    self.facing = dir;
                    self.flip = dir & 1 != 0;
                }
                self.events.push(PlayerEvent::Rush { x: self.x, y: self.y, flip: self.flip, dir });
            }
            6 => {
                // code_3C8F17: B puts a platform in front of him (not while the shooting pose runs)
                if self.wen[6] & 0x7F == 0 || b.pressed & BTN_B == 0 || self.shoot_timer != 0 {
                    return;
                }
                let Some(slot) = self.env.shot_hi else { return };
                if b.held & 3 != 0 {
                    self.facing = b.held & 3;
                    self.flip = self.facing & 1 != 0;
                }
                let idx = 0x2B + self.flip as u8;
                self.snd(0x21, 0x8F3A); // code_3C8F37
                self.events.push(PlayerEvent::Coil { slot, idx, x: self.x, y: self.y, flip: self.flip });
                self.shoot_timer = 0x10;
                self.shoot = 2;
                self.anim.set(self.anim.id.wrapping_add(2));
                self.spend_energy();
            }
            4 => {
                // code_3C8E3B: B sends a rain cloud up (not while a rain is falling or slot 1 is busy)
                if self.wen[4] & 0x7F == 0 || self.env.rain_on || b.pressed & BTN_B == 0 || !self.env.slot1_free {
                    return;
                }
                if b.held & 3 != 0 {
                    self.facing = b.held & 3;
                    self.flip = self.facing & 1 != 0;
                }
                self.snd(0x21, 0x8E5E); // code_3C8E5B
                self.events.push(PlayerEvent::Cloud { x: self.x, y: self.y, flip: self.flip });
                self.shoot_timer = 0x10;
                self.shoot = 2;
                self.anim.set(self.anim.id.wrapping_add(2));
                self.spend_energy();
            }
            12 => {
                // code_3C914E: B starts the time stop; while it runs B fires plain buster shots and the timer counts down
                if self.env.flash_on {
                    if b.pressed & BTN_B != 0 {
                        if b.held & 3 != 0 {
                            self.facing = b.held & 3;
                            self.flip = self.facing & 1 != 0;
                        }
                        self.plain_buster(b);
                    }
                    self.events.push(PlayerEvent::FlashTick);
                } else if self.wen[12] & 0x7F != 0 && b.pressed & BTN_B != 0 {
                    self.spend_energy();
                    self.events.push(PlayerEvent::FlashStart);
                    self.snd(0x17, 0x916D); // code_3C916A
                }
            }
            13 => {
                // code_3C91C2: B starts the skull barrier in slot 1
                if self.wen[13] & 0x7F == 0 || b.pressed & BTN_B == 0 || !self.env.slot1_free {
                    return;
                }
                self.events.push(PlayerEvent::Barrier { x: self.x, y: self.y, flip: self.flip });
                self.spend_energy();
            }
            _ => {}
        }
    }

    /// code_3C9045: weapon 11. B pressed: a small shot at once; B held: a fireball grows above him; B released: it flies off (medium / big).
    fn weapon_pharaoh(&mut self, b: Buttons) {
        if self.wen[11] & 0x7F == 0 {
            return;
        }
        if b.pressed & BTN_B != 0 {
            if self.shoot != 0 {
                return;
            }
            let Some(slot) = self.env.shot_hi else { return };
            self.pharaoh_fire(slot, 0x47, b);
        } else if b.held & BTN_B != 0 {
            if self.m4c8 == 0 {
                let Some(slot) = self.env.shot_hi else { return };
                self.m4c8 = slot as u8;
                self.events.push(PlayerEvent::Held { slot, x: self.x, y: self.y, flip: self.flip });
            } else if self.env.slot_aframe[self.m4c8 as usize & 3] == 0x0E {
                self.events.push(PlayerEvent::Grow { slot: self.m4c8 as usize });
            }
        } else {
            if self.m4c8 == 0 {
                return;
            }
            let slot = self.m4c8 as usize;
            self.events.push(PlayerEvent::Kill { slot });
            let anim = if self.env.slot_aid[slot & 3] == 0x49 {
                0x49
            } else if self.env.slot_aframe[slot & 3] >= 0x0B {
                0x48
            } else {
                return;
            };
            self.pharaoh_fire(slot, anim, b);
        }
    }

    /// code_3C90C2: turn toward the pad, show the shooting pose, create the shot in `slot`, pay (a big one costs double).
    fn pharaoh_fire(&mut self, slot: usize, anim: u8, b: Buttons) {
        self.m4c8 = 0;
        if b.held & 3 != 0 {
            self.facing = b.held & 3;
            self.flip = self.facing & 1 != 0;
        }
        self.shoot_timer = 0x10;
        if self.shoot == 0 {
            self.shoot = 2;
            self.anim.set(self.anim.id.wrapping_add(2));
        }
        self.events.push(PlayerEvent::Pharaoh { slot, anim, x: self.x, y: self.y, flip: self.flip, held: b.held & 0x0C });
        self.snd(if anim == 0x49 { 0x3E } else { 0x1B }, 0x914B); // code_3C9148
        self.spend_energy();
        if anim == 0x49 {
            self.spend_energy();
        }
    }

    /// code_3C8CBD + code_3C922B: the common tail of the walking, landing and airborne paths.
    fn shoot_then_ladder(&mut self, stage: &Stage, b: Buttons) {
        self.try_shoot(b);
        self.try_ladder_up(stage, b);
        if self.weapon == 5 && self.anim.id == 1 && b.held & BTN_UP != 0 {
            self.anim.set(0x38); // code_3C82AF: the wire adaptor aims up while Up is held
        }
    }

    /// code_3A81CC: an enemy touches Mega Man. Returns false when the hit does not count (the "teleport" animation `$27`, no damage entry).
    pub fn take_hit(&mut self, damage: u8, enemy: usize) -> bool {
        if self.anim.id == 0x27 || damage == 0 {
            return false;
        }
        self.ad = 0; // code_3A81DB
        let units = self.life & 0x1F;
        if units <= damage {
            // code_3A823F: dead
            self.life = 0;
            self.state = STATE_DEAD;
            self.dead_timer = 300;
            self.snd_music(0xF0, 0x824C); // code_3A8249
            self.snd(0x25, 0x8251);
            self.events.push(PlayerEvent::Death { x: self.x, y: self.y, flip: self.flip });
            return true;
        }
        self.life = 0x80 | (units - damage);
        self.snd(0x24, 0x81F4); // code_3A81F1
        self.hurt_old = self.state;
        self.hurt_probe = if self.state == STATE_SLIDE { 4 } else { 0 };
        self.state = STATE_HURT;
        self.vy = -0x100; // code_3FF324
        self.anim.set(if self.hurt_old == 4 { 0x2B } else { 0x11 });
        self.shoot = 0;
        self.shoot_timer = 0;
        if self.env.slot5_free && self.hurt_old != 4 {
            self.events.push(PlayerEvent::HitSpark { enemy, anim: 0x12 });
        }
        true
    }

    /// State 6 (code_3C85DD): knocked back while the hurt animation plays; after its frame 9 he is invincible for 60 frames.
    fn hurt_step(&mut self, stage: &Stage) {
        if self.hurt_old != 4 {
            self.vertical_step(stage);
            self.speed = 0x0080;
            let flip = self.flip;
            if self.hurt_old != 2 {
                if !self.flip {
                    self.horizontal_step(stage, BTN_RIGHT); // code_3FF000: knocked back to the right (flip becomes true)
                } else {
                    self.horizontal_step(stage, BTN_LEFT);
                }
            }
            self.flip = flip;
            // code_3C8616: the hurt handler copies his X into slot 4 (the Rush object rides along); slot 4 is overwritten even when it is empty
            self.events.push(PlayerEvent::Slot4X { xpg: (self.x >> 16) as u8, xpx: (self.x >> 8) as u8 });
        }
        if self.anim.frame == 9 {
            self.inv = 0x3C;
            match self.hurt_old {
                4 => {
                    self.state = 4;
                    self.anim.set(0x29);
                }
                2 => {
                    self.state = STATE_SLIDE;
                    self.anim.set(anim::SLIDE);
                    self.slide_timer = 0;
                    self.speed = SLIDE_SPEED;
                    // code_3C8646: JMP code_3C83C3: stand up right away when there is room above
                    if self.probe_types(stage, 3) & 1 == 0 {
                        self.end_slide();
                    }
                }
                _ => {
                    self.state = STATE_GROUND;
                    self.anim.set(0);
                    self.speed = RUN_SPEED;
                }
            }
        }
    }

    /// State 9 (code_3C86AD): gravity and the floor probe, standing animation, facing right. Buttons are ignored.
    fn wait_step(&mut self, stage: &Stage) {
        if self.vertical_step(stage) && self.anim.id != anim::IDLE {
            self.anim.set(anim::IDLE);
        }
        self.shoot_timer = 0; // $32 = $33 = 0: a running shooting-pose timer is cancelled
        self.shoot = 0;
        self.facing = BTN_RIGHT;
        self.flip = true; // code_3FF4F6 with $0420 = 1
    }

    /// The boss object gives him state `$0B` (code_35B07A): he stands for 255 frames, then walks to X `$80` facing the way he must go.
    pub fn start_victory(&mut self) {
        self.state = STATE_VICTORY;
        self.behind = false; // code_3C8CB7: $0528 = $D0
        self.hold = 0xFF;
        self.walk_dir = if 0x80 >= (self.px() & 0xFF) { 1 } else { 2 };
    }

    /// State `0B` (code_3C87A7): gravity, then stand (animation 1) while `$0148` counts down, walk (4) to X `$80`, jump (speed 8.0) from there;
    /// when he is falling at Y `$6C` with nothing else alive he gets state 8. While other objects (the sparkles) remain his Y byte stays `$6C`.
    fn victory_step(&mut self, stage: &Stage) {
        self.shoot_timer = 0;
        self.shoot = 0;
        let landed = self.vertical_step(stage);
        let want = if !landed {
            7
        } else if self.hold != 0 {
            1
        } else {
            4
        };
        if want != self.anim.id {
            self.anim.set(want);
        }
        if self.hold != 0 {
            self.hold -= 1;
            return;
        }
        if self.env.stage >= 8 {
            if self.anim.id == 4 {
                self.state = STATE_LEAVE;
            }
            return;
        }
        if self.px() & 0xFF == 0x80 {
            if self.vy >= 0 || 0x6C >= self.py() {
                return;
            }
            let saved = self.y;
            self.y = (self.y & !0xFF00) | (0x6C << 8);
            if !self.env.slots_clear {
                return; // the game leaves Y at $6C here
            }
            self.y = saved;
            if self.anim.id == 4 {
                self.state = STATE_LEAVE;
            }
            return;
        }
        if self.horizontal_step(stage, self.walk_dir & 3) {
            self.vy = JUMP_SPEED;
        }
        let at = self.px() & 0xFF;
        let arrived = if self.walk_dir & 1 != 0 { at >= 0x80 } else { at <= 0x80 };
        if arrived {
            self.x = (self.x & !0xFF00) | (0x80 << 8);
            self.vy = 0x0800;
        }
    }

    /// The capsule object gives him state `$11` (code_3BBC66): `$0148 = $FF`.
    pub fn start_item(&mut self) {
        self.state = STATE_ITEM;
        self.hold = 0xFF;
    }

    /// State `$11` (code_3C8775): `$0133` is kept at `$FF` while he is on screen 0; he stands (animation 1, or 7 in the air) while `$0148` counts
    /// down, then the teleport-out routine of state 8 runs with `$1F = 1`.
    fn item_step(&mut self, stage: &Stage) {
        if crate::game::split_y(self.y).0 == 0 {
            self.dead_timer = (self.dead_timer & 0xFF00) | 0xFF;
        }
        if self.hold == 0 {
            self.leave_step();
            self.set_checkpoint1 = true;
            return;
        }
        self.shoot_timer = 0;
        self.shoot = 0;
        let landed = self.vertical_step(stage);
        let want = if landed { 1 } else { 7 };
        if want != self.anim.id {
            self.anim.set(want);
        }
        self.hold -= 1;
    }

    /// State 8 (code_3C865D): the teleport-out beam (animation `$32`); from its fourth frame on he accelerates upward, and `$0133` is
    /// kept at 120 until he has left screen 0, then the main loop counts it down.
    fn leave_step(&mut self) {
        if self.anim.id != 0x32 {
            self.snd(0x41, 0x867D); // code_3C867A
            self.anim.set(0x32);
            self.flip = false;
            self.vy = 0;
            self.shoot = 0;
            self.shoot_timer = 0;
        }
        if self.anim.frame != 4 || crate::game::split_y(self.y).0 != 0 {
            return;
        }
        self.anim.tick = 0;
        self.vy += 0x40;
        self.y -= self.vy;
        self.dead_timer = 0x78;
    }

    /// State `0A` (code_3C86CE): the stage-start teleport-in. First the waiting beam animation `$33` plays until its frame index is `$19`;
    /// then animation `$13` starts, Y becomes 0 and he falls without collision until Y >= `$40` with nothing solid at probe H2,
    /// then with the normal vertical step until he lands; the landing animation runs to frame 4, then state 0.
    fn spawn_step(&mut self, stage: &Stage) {
        if self.anim.id != anim::SPAWN_FALL {
            if self.anim.frame != 0x19 {
                return;
            }
            self.anim.set(anim::SPAWN_FALL);
            self.flip = true;
            if self.env.stage == 1 {
                // code_3C86F1: $0528 |= $C86B[stage] ($20 for stage 1), cleared again when he spawns on page $1A
                self.behind = (self.x >> 16) != 0x1A;
            }
            self.y &= 0xFF; // $0378 = 0 (subpixel stays)
            if self.env.stage == 14 && (self.x >> 16) == 9 {
                self.x = (self.x & !0xFF00) | (0x68 << 8); // code_3C8713: the pod room drops him at X $68
            }
        }
        let probe = self.point_types(stage, 2);
        if std::env::var("MM4_DBG3").is_ok() {
            eprintln!("spawn probe: {:?} py={:X} x={:X}", probe, self.py(), self.x >> 8);
        }
        self.water_prev = probe.iter().fold(0, |a, t| a | t); // code_3C8725: $44 = $10 of this probe, every frame of the fall
        let free_fall = self.py() < 0x40 || probe.iter().any(|t| t & 1 != 0);
        if free_fall {
            self.y -= self.vy; // code_3FF2A7: move, then gravity, no collision
            self.gravity();
            self.anim.tick = 0;
        } else if self.vertical_step(stage) {
            if self.anim.frame == 0 && self.anim.tick == 1 {
                self.snd(0x33, 0x8752); // code_3C874D
            }
            if self.anim.frame != 4 {
                return; // landing animation still playing (the tick keeps running)
            }
            self.anim.set(anim::IDLE);
            self.state = STATE_GROUND;
            self.a3 = 0; // code_3C8762
            self.a78 = 0;
            self.facing = BTN_RIGHT;
            self.anim.tick = 0;
        } else {
            self.anim.tick = 0;
        }
    }

    /// code_3C8CBD does nothing without a shot (B) and `$33` stays 0, so the ladder code below ignores weapons.
    /// Centre Mega Man on the ladder tile: `$0330 = ($0330 & $F0) | $08` (only the pixel byte; page and subpixel stay).
    fn px_snap(&mut self) {
        let px = ((self.x >> 8) & 0xF0) | 0x08;
        self.x = (self.x & !0xFF00) | (px << 8);
    }

    /// Terrain types at the points of horizontal probe `id` as code_3ED428 reports them (`$45..`): type `$40` is read as `$20`.
    /// If the first point would lie above the screen top (or below its bottom) the game clamps it to row 0 (or `$EF`) and puts
    /// all following points on that same row (`$05` flag). Only screen 0 is handled (`py` is in 0..239).
    fn point_types(&self, stage: &Stage, id: usize) -> Vec<u8> {
        let t: Vec<u8> = self.horiz_raw(stage, id).0.into_iter().map(map_type).collect();
        self.m41.set(t.iter().copied().max().unwrap_or(0));
        t
    }

    /// The raw terrain types at the points of horizontal probe `id` (code_3ED428, with the first-point clamp described at `point_types`) and the probe's X.
    fn horiz_raw(&self, stage: &Stage, id: usize) -> (Vec<u8>, i32) {
        let p = stage.hitboxes.horizontal(id);
        let x = self.px() + p.fixed;
        let base = self.pys(false);
        let first = base + p.along[0];
        let clamp = if first < 0 {
            Some(0)
        } else if first >= 240 {
            Some(0xEF)
        } else {
            None
        };
        (p.along.iter().map(|dy| self.terr(stage, x, clamp.unwrap_or(base + dy))).collect(), x)
    }

    /// code_3C922B: Up held with a ladder tile at the upper or middle probe point grabs the ladder.
    fn try_ladder_up(&mut self, stage: &Stage, b: Buttons) {
        if b.held & BTN_UP == 0 {
            return;
        }
        let t = self.point_types(stage, 2);
        if t[0] == 2 || t[1] == 2 {
            self.state = STATE_LADDER;
            self.px_snap();
            self.vy = LADDER_SPEED;
            self.anim.set(anim::LADDER + self.shoot);
        }
    }

    /// State 3 (code_3C8420): climbing. The vertical speed field holds the climb speed (1.297 px/frame).
    fn ladder_step(&mut self, stage: &Stage, b: Buttons) {
        self.try_shoot(b); // code_3C8420: JSR code_3C8CBD
        if self.shoot != 0 {
            return; // code_3C8423: no climbing while the shooting pose lasts
        }
        let ud = b.held & (BTN_UP | BTN_DOWN);
        if ud == 0 {
            self.anim.tick = 0; // the climb animation freezes
        } else {
            self.facing = ud; // $31 = $0420 = the held Up/Down bits
            let bumped = if ud & BTN_DOWN != 0 {
                // code_3FF088: move down, feet probe (id 0) pushes up
                self.y += self.vy;
                let (solid, probe_y) = self.vertical_solid(stage, 0);
                if solid {
                    self.y -= (probe_y & 0x0F) << 8;
                }
                solid
            } else {
                // code_3FF0C1 with id 1: move up, head probe pushes down
                self.y -= self.vy;
                let (solid, probe_y) = self.vertical_solid(stage, 1);
                if solid {
                    self.y += (0x10 - (probe_y & 0x0F)) << 8;
                }
                solid
            };
            if bumped {
                if ud & BTN_UP != 0 {
                    // code_3C8481 (the test is on $0420, so Up+Down also lands here)
                    self.anim.tick = 0;
                    self.ladder_jump_off(b);
                } else {
                    self.leave_ladder();
                }
                return;
            }
        }
        // code_3C8449: is there still a ladder at the upper/middle/lower probe point?
        let t = self.point_types(stage, 2);
        if !t.iter().any(|&v| v == 2) {
            self.leave_ladder();
            return;
        }
        self.px_snap();
        let want = (if t[0] != 0 || t[1] != 0 { anim::LADDER } else { anim::LADDER_TOP }) + self.shoot;
        if want != self.anim.id {
            self.anim.set(want); // code_3C847E returns straight after switching
            return;
        }
        self.ladder_jump_off(b);
    }

    /// State 4 (code_3C84AD): riding the Rush Marine. The pad moves him in 8 directions (1.3 px/frame, 0.9 on the diagonals) through the water; the weapon energy is paid every 60 frames.
    fn marine_step(&mut self, stage: &Stage, b: Buttons) {
        const XVS: [u8; 16] = [0x00, 0x4C, 0x4C, 0x00, 0x00, 0xEB, 0xEB, 0x00, 0x00, 0xEB, 0xEB, 0x00, 0x00, 0x00, 0x00, 0x00];
        const XVP: [u8; 16] = [0x00, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        const YVS: [u8; 16] = [0x00, 0x00, 0x00, 0x00, 0x4C, 0xEB, 0xEB, 0x00, 0x4C, 0xEB, 0xEB, 0x00, 0x00, 0x00, 0x00, 0x00];
        const YVP: [u8; 16] = [0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        if self.anim.id == 0x27 {
            if self.anim.frame != 4 {
                return; // boarding
            }
            self.anim.set(0x29);
        }
        let d = (b.held & 0x0F) as usize;
        let v = ((YVP[d] as i32) << 8) | YVS[d] as i32;
        self.vy = v; // `$03D8/$03F0` hold the magnitude, the direction flags decide
        let saved_speed = self.speed;
        self.speed = ((XVP[d] as i32) << 8) | XVS[d] as i32;
        if d & 0x0C != 0 {
            if d & 4 != 0 {
                self.y += v; // code_3FF088: move down, the floor pushes him up
                let (solid, probe_y) = self.vertical_solid(stage, 0x1A);
                if solid {
                    self.y -= (probe_y & 0x0F) << 8;
                }
            } else {
                self.y -= v; // code_3FF0C1 (probe id + 1): move up, the ceiling pushes him down
                let (solid, probe_y) = self.vertical_solid(stage, 0x1B);
                if solid {
                    self.y += (0x10 - (probe_y & 0x0F)) << 8;
                }
            }
        }
        if d & 8 != 0 {
            // leaving the water upward: anything but pure water above the feet pushes him back down
            let or = self.probe_or(stage, 2);
            let (_, probe_y) = self.vertical_solid(stage, 2);
            if or != 0x60 {
                self.y += (0x10 - (probe_y & 0x0F)) << 8; // code_3ED9CD
            }
        }
        // keep him inside the screen: pixel Y between $10 and $E0
        let (pg, py) = crate::game::split_y(self.y);
        let cl = py.clamp(0x10, 0xE0);
        if cl != py {
            self.y = ((pg as i8 as i32 * 240 + cl as i32) << 8) | (self.y & 0xFF);
        }
        self.horizontal_step_id(stage, (d & 3) as u8, 0x26);
        self.speed = saved_speed;
        self.plain_buster(b); // code_3C8CCC
        self.m510 = self.m510.wrapping_sub(1);
        if self.m510 == 0 {
            self.m510 = 0x3C;
            self.spend_energy();
            if self.wen[3] & 0x7F == 0 {
                self.anim.set(7);
                self.marine_exit = true;
                self.state = STATE_GROUND;
            }
        }
    }

    /// State 5 (code_3C858D): once the wire has reached the ceiling (its animation `$3D`) he climbs it at 4 px/frame; A or Down lets go.
    fn wire_step(&mut self, stage: &Stage, b: Buttons) {
        if self.env.slot4_aid != 0x3D {
            return;
        }
        let want = 0x3A + self.shoot;
        if want != self.anim.id {
            self.anim.set(want);
            self.vy = 0x0400;
        }
        self.water_prev = self.probe_or(stage, 2);
        self.y -= self.vy; // code_3FF0C1 with probe `$25`: move up, a ceiling pushes him back down
        let (solid, probe_y) = self.vertical_solid(stage, 0x25);
        if solid {
            self.y += (0x10 - (probe_y & 0x0F)) << 8;
            self.plain_buster(b); // code_3C8CCC
        }
        if b.pressed & BTN_A != 0 || b.held & BTN_DOWN != 0 {
            self.events.push(PlayerEvent::Kill { slot: 4 });
            self.state = STATE_GROUND;
        }
    }

    /// The OR of the terrain types (raw `$x0` form) of the points of vertical probe `id` (code_3ED2FC leaves it in `$10`).
    fn probe_or(&self, stage: &Stage, id: usize) -> u8 {
        let p = stage.hitboxes.vertical(id);
        let y = self.pys(true) + p.fixed;
        let lim = if p.fixed < 0 { 256 } else { 240 };
        p.along.iter().fold(0u8, |a, dx| a | (self.terr_lim(stage, self.px() + dx, y, lim) << 4))
    }

    /// code_3C8486: A newly pressed with no Up/Down held drops off the ladder.
    fn ladder_jump_off(&mut self, b: Buttons) {
        if b.pressed & BTN_A != 0 && b.held & (BTN_UP | BTN_DOWN) == 0 {
            self.leave_ladder();
        }
    }

    /// code_3C8492: back to state 0 (the next frame decides between standing and falling), facing from the flip flag.
    fn leave_ladder(&mut self) {
        self.water_prev = self.m41.get(); // code_3C8492: $44 = $41
        self.state = STATE_GROUND;
        self.vy = -0x100; // code_3FF324
        self.facing = if self.flip { BTN_RIGHT } else { BTN_LEFT };
    }

    /// code_3C8149: jump from the ground (or out of a slide): vy = 5.0, then the air handler runs this same frame.
    fn jump(&mut self, stage: &Stage, b: Buttons, dir: u8) {
        self.vy = JUMP_SPEED;
        let landed = self.ground_vertical(stage);
        debug_assert!(!landed);
        self.airborne(stage, b, dir);
    }

    /// code_3C8156: Down + A on the ground. Fails (false) when a wall is 15 px ahead (the original then walks normally).
    fn start_slide(&mut self, stage: &Stage, b: Buttons) -> bool {
        let mut d = if self.flip { BTN_RIGHT } else { BTN_LEFT };
        if b.held & 3 != 0 {
            d = b.held & 3;
        }
        // code_3ED428 with horizontal id 6 (right) / 7 (left): one point 15 px ahead at y+4
        let hid = ((d >> 1) + 6) as usize;
        if self.horiz_raw(stage, hid).0.iter().any(|t| t & 1 != 0) {
            // the probe leaves its Y in $11, which the ladder-top grab that follows reads (code_3C81E2)
            self.feet_y = self.pys(false) + stage.hitboxes.horizontal(hid).along.last().copied().unwrap_or(0);
            return false;
        }
        self.facing = d;
        self.slide_timer = SLIDE_FRAMES;
        self.state = STATE_SLIDE;
        self.speed = SLIDE_SPEED;
        self.anim.set(anim::SLIDE);
        self.y += 2 << 8;
        self.shoot = 0;
        self.shoot_timer = 0;
        if self.env.slot5_free {
            let a = if self.env.stage == 1 && (self.x >> 16) < 8 { 0x31 } else { 0x17 };
            self.events.push(PlayerEvent::SlideDust { x: self.x, y: self.y, flip: self.flip, anim: a });
        }
        self.slide_after_vertical(stage, b);
        true
    }

    /// Slide frame, code_3C8353: vertical step with the slide hitbox, then the common part.
    fn slide_step(&mut self, stage: &Stage, b: Buttons) {
        let landed = self.vertical_step(stage);
        self.stage1_current(stage, landed);
        if !landed {
            // code_3C835D: with nothing under the feet (probe id $39) the slide ends and he falls
            if self.probe_types(stage, 0x39) & 1 == 0 {
                self.end_slide();
                return;
            }
        }
        self.slide_after_vertical(stage, b);
    }

    /// code_3C8368: jump out of the slide, turn, move, count down, and stand up when the timer is out and there is room.
    fn slide_after_vertical(&mut self, stage: &Stage, b: Buttons) {
        if b.pressed & BTN_A != 0 && b.held & BTN_DOWN == 0 && self.probe_types(stage, 3) == 0 {
            self.speed = RUN_SPEED;
            self.state = STATE_AIR;
            let dir = b.held & (BTN_LEFT | BTN_RIGHT);
            let dir = if dir == (BTN_LEFT | BTN_RIGHT) { 0 } else { dir };
            self.jump(stage, b, dir);
            return;
        }
        let d = b.held & (BTN_LEFT | BTN_RIGHT);
        if d != 0 && d & self.facing == 0 {
            self.facing = d;
            self.slide_timer = 0;
        }
        let blocked = self.horizontal_step(stage, self.facing);
        if !blocked && self.slide_timer != 0 {
            self.slide_timer -= 1;
            if self.slide_timer != 0 {
                return;
            }
        }
        if self.probe_types(stage, 3) & 1 != 0 {
            return; // no room to stand up: keep sliding
        }
        self.end_slide();
    }

    /// code_3C83D3: back to the ground state with the run animation.
    fn end_slide(&mut self) {
        self.state = STATE_GROUND;
        self.speed = RUN_SPEED;
        self.anim.set(anim::RUN + self.shoot);
        self.vy = -0x100; // code_3FF324
    }

    /// code_3C824D: face `dir`, play the start animation and take the first step.
    fn start_step(&mut self, stage: &Stage, dir: u8) {
        self.facing = dir;
        self.anim.set(anim::WALK_START + self.shoot);
        self.walk_move(stage, dir);
        self.footstep();
    }

    /// code_3C82C8 (states 0/1): the vertical step plus the stage hooks that follow it: the wind/current of stages 1 and 8 and the ice flag `$A4` of stage 8
    /// (set when he is falling or standing and one of the three probe points is terrain `$B0`).
    fn ground_vertical(&mut self, stage: &Stage) -> bool {
        self.a4 = false;
        let landed = self.vertical_step(stage);
        if stage.id == 8 && self.vy < 0 && self.pts.get().iter().any(|&t| t == 0xB) {
            self.a4 = true;
        }
        self.stage1_current(stage, landed);
        landed
    }

    /// code_3C93C9 + code_3C93D1: stop the ice slide (direction, speed and acceleration back to their idle values).
    fn ice_clear(&mut self) {
        self.a3 = 0;
        self.a78 = 0;
        self.a56 = 1;
    }

    /// code_3C9458: carry out the slide: move by `$A7:$A8` in direction `$A3` (the original swaps the speed variables around code_3FF3ED).
    fn ice_move(&mut self, stage: &Stage, dir0420: u8) {
        if self.a3 == 0 {
            return;
        }
        let (v, d) = (self.a78, self.a3);
        self.push_h(stage, v as u8, (v >> 8) as u8, d);
        self.flip = dir0420 & 1 != 0; // code_3FF4F6: the facing follows the player's own `$0420`, not the slide
    }

    /// code_3C946E (landing): the speed carries over into the slide, with the first acceleration step $1A.
    fn ice_land(&mut self, b: Buttons) {
        let d = b.held & (BTN_LEFT | BTN_RIGHT);
        self.a3 = d;
        if d == 0 {
            self.a78 = 0;
            self.a56 = 1;
            return;
        }
        self.a78 = self.speed as u16;
        self.a56 = 0x1A;
    }

    /// code_3C93DB: a direction is held on the ground. Returns the carry: true = the caller moves normally, false = the slide moved him.
    fn walk_ice(&mut self, stage: &Stage, dir: u8) -> bool {
        if stage.id != 8 || !self.a4 {
            self.ice_clear();
            return true;
        }
        if self.a3 != 0 && self.a3 != dir {
            // against the slide: brake
            let (v, borrow) = self.a78.overflowing_sub(self.a56);
            self.a78 = v;
            if borrow {
                self.ice_clear();
                return false;
            }
            let (n, b2) = self.a56.overflowing_sub(1);
            self.a56 = if b2 { 1 } else { n };
        } else {
            self.a3 = dir;
            if self.env.p95 & 1 != 0 {
                self.a78 = self.a78.wrapping_add(self.a56);
                let max = self.speed as u16;
                if max >= self.a78 {
                    self.a56 = self.a56.wrapping_add(1);
                } else {
                    self.a78 = max;
                }
            }
        }
        self.ice_move(stage, dir);
        false
    }

    /// The walking move of code_3C825D: ice first, the normal horizontal step otherwise.
    fn walk_move(&mut self, stage: &Stage, dir: u8) {
        if self.walk_ice(stage, dir) {
            self.horizontal_step(stage, dir);
        }
    }

    /// code_3C938F: no direction held on the ground: on ice he keeps sliding and slows down every fourth frame.
    fn ice_idle(&mut self, stage: &Stage) {
        if stage.id != 8 || !self.a4 {
            return;
        }
        if self.env.p95 & 3 == 0 {
            let (v, borrow) = self.a78.overflowing_sub(self.a56);
            self.a78 = v;
            if borrow {
                self.ice_clear();
                return;
            }
            let (n, b2) = self.a56.overflowing_sub(1);
            self.a56 = if n == 0 || b2 { 1 } else { n };
        }
        let f = self.facing;
        self.ice_move(stage, f);
    }

    /// code_3C826C: in the rainy part of stage 1 (Mega Man on pages 0-7) every walking frame makes a splash (object 1, animation $2F) when slot 5 is free.
    fn footstep(&mut self) {
        if self.env.stage == 1 && (self.x >> 16) < 8 && self.env.slot5_free {
            self.events.push(PlayerEvent::SlideDust { x: self.x, y: self.y, flip: self.flip, anim: 0x2F });
        }
    }

    /// code_3C8299: standing animation (restarted only if it was not already playing).
    fn idle(&mut self) {
        if self.anim.id != anim::IDLE + self.shoot {
            self.anim.set(anim::IDLE + self.shoot);
        }
    }

    /// code_3C82F5 and code_3C833D: the airborne animation, variable jump height, then horizontal movement.
    fn airborne(&mut self, stage: &Stage, b: Buttons, dir: u8) {
        if self.anim.id != anim::AIR {
            self.anim.set(anim::AIR + self.shoot); // the test compares with 7, the id set includes the shooting offset
            self.state = STATE_AIR; // code_3C82F5: only when the animation had to be changed (a Marine rider dismounts with animation 7 in state 0)
        }
        if self.vy >= 0 {
            if !self.m498 && b.held & BTN_A == 0 {
                self.vy = 0; // A released while rising
            }
        } else {
            self.m498 = false; // code_3C8338
        }
        self.facing = dir;
        self.horizontal_step(stage, dir);
        self.shoot_then_ladder(stage, b); // code_3C833D ends with code_3C8CBD + code_3C922B
    }
}

/// code_3ED4C2: terrain type `$4` (the `$40` nibble form) counts as `$2` for probes.
fn map_type(t: u8) -> u8 {
    if t == 4 {
        2
    } else {
        t
    }
}

impl Player {
    /// code_3C8B38 (stage 14, first thing of every player frame): `$0528` bit 5 (drawn behind the background) is set while he stands inside one of the zones.
    fn zone_priority(&mut self) {
        self.behind = false;
        let (page, px) = ((self.x >> 16) as u8, (self.x >> 8) as u8);
        let ypx = crate::game::split_y(self.y).1;
        for (i, &(zp, zx, zy)) in self.zones.iter().enumerate() {
            if i == 10 && self.ac != 0xFF {
                return;
            }
            if page < zp {
                return;
            }
            if page == zp && px.abs_diff(zx) < 0x18 && ypx.abs_diff(zy) < 0x10 {
                self.behind = true;
                return;
            }
        }
    }
}

/// `$926D`, `$927D`, `$928D` (bank `$3C`): Mega Man's palette colours 1-3 while a charge builds up, indexed by `(charge >> 3 & $FC) | (charge >> 1 & 3)`.
fn charge_palette(i: usize) -> [u8; 3] {
    const A: [u8; 16] = [0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0x22, 0x22, 0x0F, 0x22, 0x0F, 0x22, 0x0F, 0x29, 0x0F, 0x29];
    const B: [u8; 16] = [0x2C, 0x2C, 0x2C, 0x2C, 0x2C, 0x2C, 0x2C, 0x2C, 0x2C, 0x2C, 0x2C, 0x2C, 0x2C, 0x3A, 0x2C, 0x3A];
    const C: [u8; 16] = [0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x2A, 0x11, 0x2A];
    [A[i & 15], B[i & 15], C[i & 15]]
}

/// `$929E + 4 * $36` (bank `$3C`): the palette steps after a release (7 = back to normal, 8/9 = the flash after a half / full charge, 1-7 = the fade of a full charge).
fn release_palette(k: u8) -> [u8; 3] {
    const T: [u8; 40] = [
        0x0F, 0x0F, 0x2C, 0x11, 0x0F, 0x0F, 0x2C, 0x11, 0x0F, 0x37, 0x35, 0x25, 0x0F, 0x0F, 0x2C, 0x11, 0x0F, 0x30, 0x30, 0x30, 0x0F, 0x0F, 0x30, 0x31, 0x0F, 0x0F, 0x30, 0x31, 0x0F, 0x0F, 0x2C,
        0x11, 0x0F, 0x22, 0x2C, 0x11, 0x0F, 0x1A, 0x2C, 0x11,
    ];
    // T starts at $929D
    let y = k as usize * 4 + 1;
    [T[y], T[y + 1], T[y + 2]]
}

impl Player {
    /// `JSR code_3FF6BE`: queue a sound effect (`ret` = return address of the call in the ROM).
    pub fn snd(&mut self, id: u8, ret: u16) {
        self.snd.push(crate::sound::Req { music: false, id, ret });
    }

    /// `JSR code_3FF6BC`: a music / stop request (`$D9` = id, then the same queue).
    pub fn snd_music(&mut self, id: u8, ret: u16) {
        self.snd.push(crate::sound::Req { music: true, id, ret });
    }
}
