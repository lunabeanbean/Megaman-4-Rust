//! The object system: enemy spawner, per-object AI loop and the despawn / animation pass (notes/objects.md).
//!
//! Slots: 0 = Mega Man (handled by `player.rs`, only read here), 1-7 shots/effects, 8-23 enemies and other objects. Each slot mirrors
//! the game's RAM arrays byte for byte (`$0300 + 0x18*k`), so the 6502 routines port almost line by line and the
//! trace comparison (harness/gen_ptrace.js with OBJ=1) can check every byte.
//!
//! One game frame runs, in this order (bank38_3F.asm `code_3EC6C7`): player, **AI loop** (`ai_phase`, bank $3A `$8003`), camera,
//! **spawner** (`spawn_phase`, bank $38 `$8000`), then the draw pass (`draw_phase`, `code_3EDB3A`) that despawns objects and ticks
//! their animation. The random generator (`$E4-$E7`) steps once per frame in a separate task (`rng_step`).
use crate::anim::AnimTable;
use crate::player::{ObjEnv, PlayerEvent, Solid};
use crate::rom::Rom;
use crate::stage::Stage;
use std::collections::HashMap;

mod boss7e;
mod boss79;
mod boss84;
mod boss8d;
mod stage0;
mod stage1;
mod stage3;
mod stage3b;
mod stage4;
mod stage4b;
mod boss75;
mod stage7;
mod stage7b;
mod boss71;
mod boss8b;
mod stage6;
mod stage6b;
mod stage8;
mod stage9;
mod stage10;
mod boss9e;
mod boss92;
mod stage12;
mod boss6c;
mod defeat29;
mod stage13;
mod boss95;
mod boss87;
mod boss7c;
mod stage14;
mod wily14;
mod wily14b;
mod weapons;

pub const SLOTS: usize = 24;

/// One object slot; field names give the RAM array in comments.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct Obj {
    pub ty: u8,     // $0300 type / AI id (0 = free)
    pub xpg: u8,    // $0348
    pub xpx: u8,    // $0330
    pub xsub: u8,   // $0318
    pub ypg: u8,    // $0390
    pub ypx: u8,    // $0378
    pub ysub: u8,   // $0360
    pub xvs: u8,    // $03A8 X speed, sub-pixel
    pub xvp: u8,    // $03C0 X speed, pixel
    pub yvs: u8,    // $03D8 Y speed, sub-pixel (positive = up)
    pub yvp: u8,    // $03F0
    pub hitbox: u8, // $0408 (bit 7: hurts Mega Man on contact, bit 6: can be hurt, low 6 bits: size index)
    pub dir: u8,    // $0420 movement flags: 1 right, 2 left, 4 down, 8 up
    pub idx: u8,    // $0438 placement-list index (`$FF` = not from the list)
    pub hp: u8,     // $0450
    pub s: [u8; 8], // $0468 $0480 $0498 $04B0 $04C8 $04E0 $04F8 $0510 scratch
    pub flags: u8,  // $0528
    pub aframe: u8, // $0540
    pub aid: u8,    // $0558
    pub atick: u8,  // $0570
    pub ptr: u16,   // $0588 | $05A0 << 8: address of the routine that runs next frame
    pub t5b8: u8,   // $05B8 hit-flash / freeze timer
}

impl Obj {
    /// The 30 bytes in the order `harness/gen_ptrace.js` logs them.
    pub fn fields(&self) -> [u8; 30] {
        [
            self.ty, self.xpg, self.xpx, self.xsub, self.ypg, self.ypx, self.ysub, self.xvs, self.xvp, self.yvs, self.yvp, self.hitbox,
            self.dir, self.idx, self.hp, self.s[0], self.s[1], self.s[2], self.s[3], self.s[4], self.s[5], self.s[6], self.s[7],
            self.flags, self.aframe, self.aid, self.atick, self.ptr as u8, (self.ptr >> 8) as u8, self.t5b8,
        ]
    }

    /// The 30 bytes in the order of the RAM arrays (`$0300 + 0x18 * k`).
    pub fn fields_ram(&self) -> [u8; 30] {
        [
            self.ty, self.xsub, self.xpx, self.xpg, self.ysub, self.ypx, self.ypg, self.xvs, self.xvp, self.yvs, self.yvp, self.hitbox, self.dir, self.idx, self.hp, self.s[0], self.s[1],
            self.s[2], self.s[3], self.s[4], self.s[5], self.s[6], self.s[7], self.flags, self.aframe, self.aid, self.atick, self.ptr as u8, (self.ptr >> 8) as u8, self.t5b8,
        ]
    }

    pub fn from_fields(f: &[u8]) -> Obj {
        Obj {
            ty: f[0], xpg: f[1], xpx: f[2], xsub: f[3], ypg: f[4], ypx: f[5], ysub: f[6], xvs: f[7], xvp: f[8], yvs: f[9], yvp: f[10],
            hitbox: f[11], dir: f[12], idx: f[13], hp: f[14], s: [f[15], f[16], f[17], f[18], f[19], f[20], f[21], f[22]], flags: f[23],
            aframe: f[24], aid: f[25], atick: f[26], ptr: f[27] as u16 | (f[28] as u16) << 8, t5b8: f[29],
        }
    }
}

/// Effects of picking up an item (applied to the player by the caller).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pickup {
    /// Refill this many life units (the game freezes for 4 frames per unit while the meter fills).
    Life(u8),
    OneUp,
    ETank,
    /// A weapon capsule (animation id >= `$51`): fills weapon slot `$0064 + id - $B0` and freezes Mega Man (state `$11`).
    Capsule(u8),
    /// Weapon-energy item: refill the selected weapon by this many units (same timing as a life item).
    Weapon(u8),
}

/// Something an object's AI does to Mega Man's position; applied by the caller right after that AI, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerOp {
    /// `STA $0378` then code_3FF3C1: his pixel Y becomes this value and his ground probe pushes him up onto the tile top.
    Ground(u8),
    /// `STA $0378`: only his pixel Y becomes this value.
    SetY(u8),
    /// `LDA #n / STA $AD`: Mega Man's own logic is skipped for n frames (any damage clears it).
    Freeze(u8),
    /// code_3FF32F: the sub-pixel X is synchronised with the carrying object (if given) and he moves by its speed (code_3FF37C).
    Carry { xsub: Option<u8>, xvs: u8, xvp: u8, dir: u8 },
    /// `STA $0330`: his pixel X becomes this value.
    SetX(u8),
    /// The Rush Coil launches him: vertical speed 6.93 and the bounce flag `$0498` (the jump cut-off by releasing A is disabled until he falls).
    Bounce,
    /// The Rush Marine takes Mega Man on board (code_3A8B42): state 4, boarding animation `$27`, he is put where Rush is.
    Mount { xpg: u8, xpx: u8, ypg: u8, ypx: u8 },
    /// code_3FFD75: the selected weapon's energy cost is paid.
    Pay,
    /// The stage-11 press catches Mega Man (animation 7, vy = -4.0, no shooting pose).
    Crush,
}

/// What the object code reads about Mega Man (slot 0).
#[derive(Clone, Copy, Default, Debug)]
pub struct PlayerView {
    pub xpg: u8,
    pub xpx: u8,
    pub ypg: u8,
    pub ypx: u8,
    /// `$30`.
    pub state: u8,
    /// `$0558`.
    pub anim: u8,
    /// `$03F0` bit 7: his vertical speed points down.
    pub falling: bool,
    /// `$B0`: life meter (bit 7 + units); the life-item refill needs it.
    pub life: u8,
    /// `$0528` bit 6 of Mega Man: he faces right.
    pub flip: bool,
}

/// Camera X (`$F9:$FC`).
#[derive(Clone, Copy, Default, Debug)]
pub struct Cam {
    pub hi: u8,
    pub lo: u8,
}

#[derive(Clone)]
pub struct World {
    rom: std::rc::Rc<Rom>,
    /// The stage palette (RAM copy, animations, what the PPU shows).
    pub pal: crate::palette::PalState,
    pub o: [Obj; SLOTS],
    /// `$E4-$E7`: the 32-bit shift-register random generator.
    pub rng: [u8; 4],
    /// `$9E` / `$9F`: cursors into the sorted placement list (first entry beyond the right edge / first entry inside the window).
    pub c9e: u8,
    pub c9f: u8,
    /// `$95`: frame counter; its parity decides the order of the draw pass.
    pub p95: u8,
    /// Sound requests of this frame, in the order the game makes them.
    pub snd: Vec<crate::sound::Req>,
    /// `$D9`: the id of the last music request (the boss-room code only asks again when the music differs).
    pub d9: u8,
    /// `$22`: the stage id as the last AI phase saw it.
    pub st_id: u8,
    /// `$0100..$011F`: "entry already defeated" bits of the placement list.
    pub defeated: [u8; 32],
    p: PlayerView,
    /// `$013C | $013D` (set while a boss shutter / transition runs): the dripper AI waits.
    pub flag_13c_13d: bool,
    /// $013C:$013D: the Flash Stopper's time-stop timer (frames left).
    pub w13c: u16,
    /// `$F6`: the bank mapped at `$A000`; the AI loop's `CMP $F6` leaves a carry that a few routines (`ADC` without `CLC`) depend on.
    f6: u8,
    carry: bool,
    anims: HashMap<u8, AnimTable>,
    /// AI routine addresses that were reached but are not ported (the object stays where it is).
    pub unported: Vec<u16>,
    /// Slots whose contact check hit Mega Man this frame (damage is not ported yet).
    pub contacts: Vec<usize>,
    /// Contact damage dealt to Mega Man during this frame's AI loop (applied by the caller).
    pub hit: Option<(u8, usize)>,
    /// Objects that were drawn this frame, in slot order: (slot, screen x, screen y). Mega Man (slot 0) is not part of it.
    pub drawn: Vec<(usize, u8, u8)>,
    /// An item picked up during this frame (applied by the caller).
    pub pickup: Option<Pickup>,
    /// `$92`: the video-frame counter (the item drop roll mixes it in).
    pub frame92: u8,
    /// `$3C != 0`: Mega Man cannot be hurt by contact.
    pub invincible: bool,
    /// How many times each AI routine address ran (coverage of the tests).
    pub stats: std::collections::BTreeMap<u16, u32>,
    /// `$99`: gravity per frame in sub-pixels (0x40 normally).
    pub gravity: u8,
    /// $F9: camera page as the AI loop sees it (the camera moves after the AI loop).
    pub cam_page: u8,
    /// Camera of the frame in progress (`$F9:$FC`); the draw passes that run inside a life-item refill use it.
    pub cam_now: Cam,
    /// Random-generator steps that already happened inside this frame's AI loop (life-item refill waits).
    pub loop_steps: usize,
    /// Life units added by this frame's life-item refill (applied to the player by the caller).
    pub life_add: u8,
    /// Energy byte `$B0+$A0` of the selected weapon as the player left it this frame, and the units a weapon-energy item added.
    pub wen_cur: u8,
    pub wen_add: u8,
    /// A life item was picked up by the AI that just ran and its refill has not run yet (several items can be taken in one frame).
    life_new: bool,
    /// Verifier hook: perform one random-generator step (a video-frame boundary) at this point of the AI loop: `2 * slot` before its AI, `2 * slot + 1`
    /// between its AI and its shot test.
    pub nmi_pos: Option<usize>,
    /// `$F0` (4 = a scrolling-background object is active: the crush check is relaxed near the bottom of the screen), `$F1`, `$FA` (vertical scroll),
    /// `$FD`, `$013E`, `$013F`: written by the scroll objects (AI `$5E`); only `f0` changes game logic.
    pub f0: u8,
    pub f1: u8,
    pub fa: u8,
    pub fd: u8,
    pub w13e: u8,
    /// `$1D`: pending screen-update request (bit 7 set until the NMI takes it).
    pub w1d: u8,
    /// `$14`: buttons newly pressed this frame (the stage-1 boss reacts to B).
    pub pressed: u8,
    /// `$16`: buttons held this frame (the stage-7 boss reacts to Left/Right).
    pub held: u8,
    /// The stage-1 boss's rain: `$013A` (draw frames left), `$013B` (offset, +4 per draw), `$0145` (the boss slot) and the 30 sprites of the last draw pass
    /// (y, tile, attribute, x) that code_3EDF5F puts at the start of the OAM.
    pub w13a: u8,
    pub w13b: u8,
    pub w145: u8,
    pub rain: Vec<(u8, u8, u8, u8)>,
    /// An AI asked for a direct hit on Mega Man (`JSR $8000`): damage and enemy slot; applied right after that AI.
    pub hurt_req: Option<(u8, usize)>,
    pub w13f: u8,
    /// `$1B`: the column-renderer request (the NMI code at `$3FE35C` counts it from 0 up to `$40` and then clears it), `$1C`.
    pub p1b: u8,
    pub p1c: u8,
    /// `$6B != 0` (the refill loop counter): the draw pass skips the animation tick (code_3EDC67 `LDA $6B / BNE`).
    hold_anim: bool,
    /// `$BF`: the boss life meter's fill counter while the boss-room object `$B8` powers it up (units 0..=28).
    pub bf: u8,
    /// `$0132`: boss meter source (bit 7 set = draw the third HUD meter from `$00B0 + (value & $7F)`).
    pub meter2: u8,
    /// `$0146`: the slot that owns the boss meter.
    pub boss_slot: u8,
    /// A state the object code wants Mega Man in (`$30`), applied by the caller after the AI loop.
    pub player_cmd: Option<u8>,
    /// A pod was touched (`$35:A797`): Mega Man starts state `$0F` at the pod's position (X, Y pixel).
    pub pod_req: Option<(u8, u8)>,
    /// `$AB` (pod index) and `$0149` (placement-list index of the touched pod).
    pub ab: u8,
    pub c149: u8,
    /// `$AC`: bit mask of the robot masters defeated in the boss rush (all set = `$FF`).
    pub ac: u8,
    /// `$EF` when it differs from the stage table value: entering a boss-room section (flag `$10`) zeroes it, the boss sets `$B8`. Stays until the stage restarts.
    pub ef_override: Option<u8>,
    /// Test hook (harness BOSSHP): the boss starts with this many hit points instead of `$1C`.
    pub boss_hp: Option<u8>,
    /// Set by an AI that moves Mega Man sideways (code_3FF37C): (X speed sub-pixel, pixel, direction flags), applied right after the AI by the caller.
    pub push_req: Option<(u8, u8, u8)>,
    /// Position changes an AI makes to Mega Man (see `PlayerOp`), applied by the caller right after the AI.
    pub player_ops: Vec<PlayerOp>,
    /// The AI left X holding a stale `$0F` (the NMI's scratch byte, not a slot) instead of its own slot: the game then runs the shot/contact
    /// tests of the AI loop on junk RAM, which finds nothing, so the port skips them for that slot this frame (`ai_b50a`: `LDX $0F` without `STX $0F`).
    x_junk: bool,
    /// Terrain types of the points of the last tile probe an object made (`$45`, `$46`...): bit 0 = solid (the game keeps them as `$10`).
    pub(crate) pt: Vec<u8>,
    /// `$0120..$0123` (bit 7 = a bar object uses this slot, bit 6 = its tiles need a redraw), `$0124..`, `$0128..`, `$012C..` (redraw data, only written).
    pub(crate) b120: [u8; 4],
    pub(crate) b124: [u8; 4],
    pub(crate) b128: [u8; 4],
    pub(crate) b12c: [u8; 4],
    /// `$0140/$0141/$0142`: a shot hit a destructible block (page, block index, sub-tile); the NMI breaks it (code_3FE4DD).
    pub w140: u8,
    pub w141: u8,
    pub w142: u8,
    /// Bits of `$0680..` the NMI set, to be written into the stage by the caller (`flush_blocks`).
    pub block_events: Vec<u8>,
    /// Column-renderer steps (`$1B`) the NMIs of this frame owe: the AI loop of a logic frame always sees the value from the end of the previous one,
    /// they are applied at the end of the frame (`apply_col_steps`).
    pub col_due: u8,
    /// The camera built a tile column in this logic frame (`$19` != 0): the renderer's step of this frame is skipped.
    pub col_block: bool,
    /// PPU buffer records the objects wrote in this logic frame (address, bytes): the nametable gets them (`Game::apply_ppu`); `shadow_or` = changes of the RAM copy of the attribute table only.
    pub ppu_rec: Vec<(u16, Vec<u8>)>,
    pub shadow_or: Vec<(usize, u8)>,
    /// `$69`/`$6A`: the room scroll of the stage-9 boss (9 bits: `$6A` bit 0 is bit 8).
    pub scroll69: u8,
    pub scroll6a: u8,
    /// The object code wants the section byte `$2B` of the camera to become (section | this flag).
    pub sect_req: Option<u8>,
    /// `$A0`: the selected weapon (0 = buster); decides the shot range, the damage table bank and special hit reactions.
    pub weapon: u8,
}

impl World {
    pub fn new(rom: &Rom) -> World {
        World {
            rom: std::rc::Rc::new(rom.clone()),
            pal: crate::palette::PalState::default(),
            o: [Obj { idx: 0xFF, ..Obj::default() }; SLOTS],
            rng: [0; 4],
            c9e: 0,
            c9f: 0,
            p95: 0,
            snd: Vec::new(),
            d9: 0,
            st_id: 0,
            defeated: [0; 32],
            p: PlayerView::default(),
            flag_13c_13d: false,
            w13c: 0,
            f6: 0x3B,
            carry: true,
            anims: HashMap::new(),
            unported: Vec::new(),
            contacts: Vec::new(),
            gravity: 0x40,
            stats: Default::default(),
            frame92: 0,
            hit: None,
            pickup: None,
            drawn: Vec::new(),
            invincible: false,
            cam_page: 0,
            cam_now: Cam::default(),
            loop_steps: 0,
            life_add: 0,
            wen_cur: 0,
            wen_add: 0,
            life_new: false,
            nmi_pos: None,
            f0: 0,
            f1: 0,
            fa: 0,
            fd: 0,
            w13e: 0,
            w1d: 0,
            pressed: 0,
            held: 0,
            w13a: 0,
            w13b: 0,
            w145: 0,
            rain: Vec::new(),
            hurt_req: None,
            w13f: 0,
            p1b: 0,
            p1c: 0,
            hold_anim: false,
            bf: 0,
            meter2: 0,
            boss_slot: 0,
            player_cmd: None,
            pod_req: None,
            ab: 0,
            c149: 0,
            ac: 0,
            ef_override: None,
            boss_hp: None,
            push_req: None,
            player_ops: Vec::new(),
            x_junk: false,
            pt: Vec::new(),
            b120: [0; 4],
            b124: [0; 4],
            b128: [0; 4],
            b12c: [0; 4],
            w140: 0,
            w141: 0,
            w142: 0,
            block_events: Vec::new(),
            col_due: 0,
            col_block: false,
            ppu_rec: Vec::new(),
            shadow_or: Vec::new(),
            scroll69: 0,
            scroll6a: 0,
            sect_req: None,
            weapon: 0,
        }
    }

    // ---- ROM tables --------------------------------------------------------------------------------------------------------

    /// Byte at CPU address `a` with bank `$3A` at `$8000` and bank `$3B` at `$A000` (the AI code and its tables).
    fn ai_byte(&self, a: u16) -> u8 {
        self.rom.cpu_byte(0x3A, self.f6 as usize, a)
    }

    /// Byte at CPU address `a` with bank `$38` at `$8000` (spawner type tables).
    fn t38(&self, a: u16) -> u8 {
        self.rom.cpu_byte(0x38, 0x39, a)
    }

    fn anim_table(&mut self, ty: u8) -> &AnimTable {
        let rom = &self.rom;
        self.anims.entry(ty).or_insert_with(|| AnimTable::load(rom, ty as usize))
    }

    // ---- the random generator ----------------------------------------------------------------------------------------------

    /// Task 1 (`$3EC4D9`): one step of the 32-bit shift register, once per frame.
    /// code_3EC438: slots 1..23 lose type, next-routine high byte, hit timer, hit points and list index (the other bytes stay stale).
    /// code_3ECEE1 (run by every section change): resets the scroll-object state ($F0/$F1/$FA/$FD/$1C, $013C/$013D), the boss meter counter $BF and $013E.
    pub fn section_reset(&mut self) {
        self.b120 = [0; 4]; // code_3ECEE1 clears $0120-$0123
        self.f0 = 0;
        self.f1 = 0x88;
        self.fa = 0;
        self.fd = 0;
        self.p1c = 0;
        self.w13e = 0;
        self.w13a = 0;
        self.w13c = 0;
        self.bf = 0;
    }

    pub fn clear_slots(&mut self) {
        for x in 1..SLOTS {
            let o = &mut self.o[x];
            o.ty = 0;
            o.ptr &= 0x00FF;
            o.t5b8 = 0;
            o.hp = 0;
            o.idx = 0xFF;
        }
    }

    /// The recorder's hook (`BOSSHP`) runs after every video frame: a boss that has just got its full hit points gets `boss_hp` instead.
    pub fn boss_hp_hook(&mut self) {
        if let Some(h) = self.boss_hp {
            // harness hook: the recorder rewrites the boss's hp after the video frame in which it appeared
            for s in 8..SLOTS {
                if (matches!(self.o[s].ty, 0x71 | 0x75 | 0x79 | 0x7C | 0x7E | 0x84 | 0x87 | 0x8B | 0x8D | 0x95 | 0x9E | 0x92 | 0xA6 | 0xAB | 0xBC | 0xC0) && self.o[s].hp == 0x1C) || (self.o[s].ty == 0x2B && self.o[s].hp == 0x10) {
                    self.o[s].hp = h;
                }
            }
        }
    }

    pub fn rng_step(&mut self) {
        let mut carry = ((self.rng[0] & 2) ^ (self.rng[1] & 2)) != 0;
        for b in self.rng.iter_mut() {
            let out = *b & 1 != 0;
            *b = (*b >> 1) | ((carry as u8) << 7);
            carry = out;
        }
        self.request_step();
        self.col_due = self.col_due.saturating_add(1);
        self.block_step();
        self.carry = carry; // the generator task runs separately; the flag is overwritten by the AI loop anyway
    }

    /// code_3FE4DD (NMI): break the block a shot hit, if its table entry is found (otherwise `$0140` stays set for good, like the game).
    fn block_step(&mut self) {
        if self.w140 == 0 {
            return;
        }
        let rd = |s: &World, a: u16| s.rom.cpu_byte(0x3E, 0x3F, a);
        let mut y = rd(self, 0xE566 + self.w140 as u16) as u16;
        loop {
            let v = rd(self, 0xE577 + y);
            if v & 0x80 != 0 {
                return;
            }
            if v == self.w141 {
                break;
            }
            y += 2;
        }
        self.w140 = 0;
        let bit = rd(self, 0xE578 + y) | self.w142;
        self.block_events.push(bit);
        // code_3FE4F3 also blanks the metatile (2x2 tiles) in the nametable: cell `$0141` (8x8 cells of 4x4 tiles), quarter `$0142`
        if std::env::var("MM4_NTWATCH").is_ok() { eprintln!("block break cell {:02X} quarter {}", self.w141, self.w142); }
        let cell = self.w141 as u16;
        let off = [0u16, 2, 0x40, 0x42][(self.w142 & 3) as usize];
        let addr = 0x2000 + ((cell & 0xF8) << 4) + ((cell & 7) << 2) + off;
        self.ppu_put(addr, &[0, 0]);
        self.ppu_put((addr & 0xFF00) | ((addr as u8) | 0x20) as u16, &[0, 0]);
    }

    /// Write the bits the NMI collected into the stage's block table.
    pub fn flush_blocks(&mut self, st: &Stage) {
        for bit in self.block_events.drain(..) {
            let c = &st.blocks[(bit >> 3) as usize & 63];
            c.set(c.get() | (1 << (bit & 7)));
        }
    }

    /// code_3ED17F for the column of blocks `col >> 2` of page `page` (the tile-column builder, code_3ECFB1): in stage 4 every pink-bar metatile
    /// (`$09`/`$8C`, code_3ED1D7) leaves its scratch values in `$E6/$E7`, so the random state is overwritten (by the last bar of the column, blocks
    /// from the top down, the four metatiles of a block in the order 3, 2, 1, 0).
    pub fn column_scribble(&mut self, st: &Stage, page: u8, col: u8, vertical: bool) {
        if st.id != 4 {
            return;
        }
        let layout = st.byte(0xB500 + page as usize) as usize;
        for k in 0..8u8 {
            self.block_scribble(st, layout, (col >> 2) + 8 * k, page, vertical);
        }
    }

    /// The four metatiles of block `a29` (`$29`) of a layout, as code_3ED17F visits them.
    pub fn block_scribble(&mut self, st: &Stage, layout: usize, a29: u8, page: u8, vertical: bool) {
        let block = st.byte(0xA900 + layout * 64 + a29 as usize) as usize;
        for y in (0..4u8).rev() {
            let meta = st.byte(0xA500 + block * 4 + y as usize);
            if meta != 9 && meta != 0x8C {
                continue;
            }
            let e6 = if vertical { 0 } else { (page & 1) << 5 };
            let t = a29 >> 1;
            let e7 = (t & 0xFC) | ((t >> 1) & 1);
            let e7 = (y & 2) | e6 | e7;
            let idx = ((a29 << 1) | (y & 1)) & 7;
            self.rng[2] = 1 << idx;
            self.rng[3] = e7;
        }
    }

    /// The NMI's column renderer (code_3FE35C), only its `$1B` bookkeeping: a request (bit 7 set) starts the count, each video frame draws four
    /// columns (`$1B` += 4) and when it has reached `$40` the next frame ends it (`$1B` = 0).
    fn column_step(&mut self) {
        if self.p1b == 0 {
            return;
        }
        if self.p1b == 0x40 {
            self.p1b = 0;
            return;
        }
        self.p1b = (self.p1b & 0x7F).wrapping_add(4);
    }

    /// code_3FE262 (NMI): takes a pending `$1D` request (the tile/palette writing itself is only drawing).
    fn request_step(&mut self) {
        if self.w1d & 0x80 != 0 {
            self.w1d &= 0x7F;
        }
    }

    // ---- shared helpers (bank $3F) -----------------------------------------------------------------------------------------

    /// code_3FFB6C: the free slot with the highest index in 8..=23, or None (carry set).
    fn find_slot(&self) -> Option<usize> {
        (8..SLOTS).rev().find(|&s| self.o[s].ty == 0)
    }

    /// code_3FF446: start animation `id`.
    fn set_anim(&mut self, x: usize, id: u8) {
        let o = &mut self.o[x];
        o.aid = id;
        o.aframe = 0;
        o.atick = 0;
    }

    /// code_3FF22D: move right by the X speed.
    fn right(&mut self, x: usize) {
        let o = &mut self.o[x];
        let (s, c1) = o.xsub.overflowing_add(o.xvs);
        o.xsub = s;
        let (p, c2) = o.xpx.overflowing_add(o.xvp);
        let (p, c3) = p.overflowing_add(c1 as u8);
        o.xpx = p;
        let mut carry = false;
        if c2 || c3 {
            carry = o.xpg == 0xFF; // ADC #$00 with the carry set
            o.xpg = o.xpg.wrapping_add(1);
        }
        self.carry = carry;
    }

    /// code_3FF24B: move left.
    fn left(&mut self, x: usize) {
        let o = &mut self.o[x];
        let (s, b1) = o.xsub.overflowing_sub(o.xvs);
        o.xsub = s;
        let (p, b2) = o.xpx.overflowing_sub(o.xvp);
        let (p, b3) = p.overflowing_sub(b1 as u8);
        o.xpx = p;
        let mut carry = true;
        if b2 || b3 {
            carry = o.xpg >= 1; // SBC #$00 with the carry clear
            o.xpg = o.xpg.wrapping_sub(1);
        }
        self.carry = carry;
    }

    /// code_3FF269: move down by the Y speed (the pixel part wraps from `$F0` to 0 and bumps the screen).
    fn down(&mut self, x: usize) {
        let o = &mut self.o[x];
        let (s, c1) = o.ysub.overflowing_add(o.yvs);
        o.ysub = s;
        let mut p = o.ypx.wrapping_add(o.yvp).wrapping_add(c1 as u8);
        let mut carry = false;
        if p >= 0xF0 {
            p = p.wrapping_add(0x10); // CMP #$F0 left carry set: ADC #$0F adds 16
            o.ypg = o.ypg.wrapping_add(1);
            carry = true;
        }
        o.ypx = p;
        self.carry = carry;
    }

    /// code_3FF289: move up.
    fn up(&mut self, x: usize) {
        let o = &mut self.o[x];
        let (s, b1) = o.ysub.overflowing_sub(o.yvs);
        o.ysub = s;
        let (p, b2) = o.ypx.overflowing_sub(o.yvp);
        let (p, b3) = p.overflowing_sub(b1 as u8);
        o.ypx = p;
        let mut carry = true;
        if b2 || b3 {
            carry = o.ypx >= 0x10; // SBC #$0F with the carry clear
            o.ypx = o.ypx.wrapping_sub(0x10);
            o.ypg = o.ypg.wrapping_sub(1);
        }
        self.carry = carry;
    }

    /// code_3FF413: move horizontally by the direction flags (also sets the flip flag).
    fn move_h(&mut self, x: usize) {
        match self.o[x].dir & 3 {
            0 => {}
            d if d & 1 != 0 => {
                self.o[x].flags |= 0x40;
                self.right(x);
            }
            _ => {
                self.o[x].flags &= !0x40;
                self.left(x);
            }
        }
    }

    /// code_3FF434: move vertically by the direction flags.
    fn move_v(&mut self, x: usize) {
        match self.o[x].dir & 0x0C {
            0 => {}
            d if d & 4 != 0 => self.down(x),
            _ => self.up(x),
        }
    }

    /// code_3FF4DC: face Mega Man (1 = he is to the right, 2 = left).
    fn face_player(&mut self, x: usize) {
        let o = &mut self.o[x];
        let me = (o.xpg as u16) << 8 | o.xpx as u16;
        let pl = (self.p.xpg as u16) << 8 | self.p.xpx as u16;
        o.dir = if me >= pl { 2 } else { 1 };
        self.carry = me >= pl;
    }

    /// code_3FF6E6: horizontal distance to Mega Man (low byte only).
    fn dist_pure(&self, x: usize) -> (u8, bool) {
        let o = &self.o[x];
        let a = self.p.xpx.wrapping_sub(o.xpx);
        let me = (o.xpg as u16) << 8 | o.xpx as u16;
        let pl = (self.p.xpg as u16) << 8 | self.p.xpx as u16;
        if pl < me {
            ((!a).wrapping_add(1), false)
        } else {
            (a, true)
        }
    }

    fn dist_x(&mut self, x: usize) -> u8 {
        let (d, c) = self.dist_pure(x);
        self.carry = c;
        d
    }

    /// code_3FF6D7: vertical distance to Mega Man (pixel Y only).
    fn dist_y(&self, x: usize) -> u8 {
        let d = self.p.ypx.wrapping_sub(self.o[x].ypx);
        if self.p.ypx < self.o[x].ypx {
            (!d).wrapping_add(1)
        } else {
            d
        }
    }

    /// code_3FF6FD + code_3FF71E: the 16-way heading (0..15) from the object to Mega Man.
    fn heading_to_player(&self, x: usize) -> u8 {
        self.heading_between(x, self.p.xpg, self.p.xpx, self.p.ypx)
    }

    /// code_3FF71E: the 16-way heading from object `x` to the point (page `tpg`, X `tpx`, Y `typ`) (target = Mega Man: `F6FD`, another object: `F70F`).
    fn heading_between(&self, x: usize, tpg: u8, tpx: u8, typ: u8) -> u8 {
        let o = &self.o[x];
        let mut y: u8 = 0;
        let mut dy = typ.wrapping_sub(o.ypx);
        if typ < o.ypx {
            dy = (!dy).wrapping_add(1);
            y = 4;
        }
        let mut dx = tpx.wrapping_sub(o.xpx);
        let me = (o.xpg as u16) << 8 | o.xpx as u16;
        let pl = (tpg as u16) << 8 | tpx as u16;
        if pl < me {
            dx = (!dx).wrapping_add(1);
            y += 2;
        }
        let (small, large) = if dx < dy {
            y += 1;
            (dx, dy)
        } else {
            (dy, dx)
        };
        let mut k = 0u8;
        let mut a = large >> 2;
        if a < small {
            k = 1;
            a = a.wrapping_shl(1);
            if a < small {
                k = 2;
            }
        }
        self.rom.cpu_byte(0x3A, 0x3B, 0xF776 + (y * 4 + k) as u16)
    }

    /// code_3FF7A6: turn the heading `$04B0` one step toward Mega Man, set the move flags from it; returns the heading (the game leaves it in Y).
    fn aim(&mut self, x: usize) -> u8 {
        let target = self.heading_to_player(x);
        let cur = self.o[x].s[3];
        let v = (cur.wrapping_add(8) & 0x0F).wrapping_sub(target) & 0x0F;
        self.carry = v >= 8; // the last SBC #$08 of the routine
        if v == 8 {
            return target;
        }
        let mut h = cur;
        if v > 8 {
            h = h.wrapping_sub(1);
        } else {
            h = h.wrapping_add(1);
        }
        h &= 0x0F;
        self.o[x].s[3] = h;
        self.o[x].dir = self.rom.cpu_byte(0x3A, 0x3B, 0xF796 + h as u16);
        h
    }

    /// code_3FF49C: initialise a child object in slot `y` at an offset from `x` (table index `idx`), animation `anim`.
    fn child_at(&mut self, x: usize, y: usize, idx: u8, anim: u8) {
        let i = idx as u16;
        let ox = self.rom.cpu_byte(0x3A, 0x3B, 0xF50B + i);
        let oxh = self.rom.cpu_byte(0x3A, 0x3B, 0xF573 + i);
        let oy = self.rom.cpu_byte(0x3A, 0x3B, 0xF5DB + i);
        let oyh = self.rom.cpu_byte(0x3A, 0x3B, 0xF643 + i);
        let src = self.o[x];
        let (px, c) = src.xpx.overflowing_add(ox);
        self.o[y].xpx = px;
        self.o[y].xpg = src.xpg.wrapping_add(oxh).wrapping_add(c as u8);
        let (py, c) = src.ypx.overflowing_add(oy);
        self.o[y].ypx = py;
        let (pg, c1) = src.ypg.overflowing_add(oyh);
        let (pg, c2) = pg.overflowing_add(c as u8);
        self.o[y].ypg = pg;
        self.init_child(x, y, anim);
        self.carry = c1 || c2; // the last ADC of code_3FF49C leaves its carry
    }

    /// code_3FF46B: flags, zeroed scratch and the animation of a freshly created child.
    fn init_child(&mut self, x: usize, y: usize, anim: u8) {
        let flags = (self.o[x].flags & 0x40) | 0x90;
        let c = &mut self.o[y];
        c.flags = flags;
        c.s = [0; 8];
        c.aid = anim;
        c.aframe = 0;
        c.atick = 0;
    }

    /// code_3FF95D: does the object touch Mega Man? (the game returns carry clear for a hit)
    pub fn touches_player(&self, x: usize) -> bool {
        let o = &self.o[x];
        if self.p.ypg != 0 || o.ypg != 0 || o.flags & 0x80 == 0 || o.flags & 4 != 0 {
            return false;
        }
        let hb = (o.hitbox & 0x3F) as u16;
        let mut height = self.rom.cpu_byte(0x3A, 0x3B, 0xF9B4 + hb);
        if self.p.anim == 0x10 {
            height = height.wrapping_sub(8); // sliding
        }
        let dx = self.dist_pure(x).0;
        if dx >= self.rom.cpu_byte(0x3A, 0x3B, 0xF9F4 + hb) {
            return false;
        }
        let mut dy = self.p.ypx.wrapping_sub(o.ypx);
        if self.p.ypx < o.ypx {
            dy = (!dy).wrapping_add(1);
        }
        dy < height
    }

    // ---- the AI loop (bank $3A, $8003) ------------------------------------------------------------------------------------

    /// Run every live object's AI once (slots 1..23 in ascending order) with Mega Man at `p`.
    /// Refresh Mega Man's data after the camera code moved him (a section change shifts his page before the spawner runs).
    pub fn set_view(&mut self, p: PlayerView) {
        self.p = p;
    }

    pub fn ai_phase(&mut self, st: &Stage, p: PlayerView, hook: &mut dyn FnMut(&mut World, u8, usize)) {
        self.st_id = st.id as u8;
        self.p = p;
        self.f6 = 0x3B;
        self.contacts.clear();
        self.hit = None;
        self.pickup = None;
        self.life_add = 0;
        self.wen_add = 0;
        self.life_new = false;
        self.player_cmd = None;
        self.boss_hp_hook();
        for x in 1..SLOTS {
            let ty = self.o[x].ty;
            if ty == 0 {
                continue;
            }
            if self.nmi_pos == Some(2 * x) {
                self.nmi_pos = None;
                self.rng_step();
                self.boss_hp_hook(); // the recorder's hook runs after every video frame
                self.flush_blocks(st);
            }
            if self.o[x].ptr & 0x8000 == 0 {
                let lo = self.ai_byte(0x8683 + ty as u16);
                let hi = self.ai_byte(0x8753 + ty as u16);
                self.o[x].ptr = lo as u16 | (hi as u16) << 8;
            }
            if self.o[x].t5b8 & 0x80 == 0 {
                let bank = self.ai_byte(0x85B3 + ty as u16);
                self.carry = bank >= self.f6; // CMP $F6 before the bank switch
                self.f6 = bank;
                self.run_ai(st, x);
                st.set_scroll_mode(self.f0, self.w13e);
                if self.push_req.is_some() || !self.player_ops.is_empty() {
                    hook(self, 0xFF, x);
                }
                if let Some((d, e)) = self.hurt_req.take() {
                    hook(self, d, e);
                }
                self.life_refill(st);
            }
            if self.nmi_pos == Some(2 * x + 1) {
                self.nmi_pos = None;
                self.rng_step();
                self.boss_hp_hook(); // the recorder's hook runs after every video frame
                self.flush_blocks(st);
            }
            if std::mem::take(&mut self.x_junk) {
                continue;
            }
            if x >= 5 {
                self.weapon_phase(x, hook);
            }
        }
    }

    /// The life-item routine (code_3BBC11) refills the meter one unit at a time inside the AI loop: each unit runs a full draw pass
    /// (`$DB3A`: despawns, animation ticks, `$95`) and waits 4 video frames, so the random task steps 4 times per unit before the next object's AI.
    fn life_refill(&mut self, st: &Stage) {
        if !self.life_new {
            return;
        }
        self.life_new = false;
        let (n, weapon) = match self.pickup {
            Some(Pickup::Life(n)) => (n, false),
            Some(Pickup::Weapon(n)) => (n, true),
            _ => return,
        };
        let units = if weapon { 28 - ((self.wen_cur & 0x1F) + self.wen_add).min(28) } else { 28 - ((self.p.life & 0x1F) + self.life_add).min(28) };
        let add = n.min(units);
        if weapon {
            self.wen_add += add;
        } else {
            self.life_add += add;
        }
        for _ in 0..add {
            self.hold_anim = true;
            self.draw_phase(self.cam_now);
            self.hold_anim = false;
            self.snd(0x29, 0xBC30); // code_3BBC2D
            for _ in 0..4 {
                self.rng_step();
                self.flush_blocks(st);
                self.loop_steps += 1;
            }
        }
    }

    fn run_ai(&mut self, st: &Stage, x: usize) {
        *self.stats.entry(self.o[x].ptr).or_insert(0) += 1;
        let ptr = self.o[x].ptr;
        match ptr {
            0xAD70 => self.ai_ad70(st, x),
            0xADC3 => self.ai_adc3(st, x),
            0xADF1 => self.ai_adf1(x),
            0xAD0C => self.ai_ad0c(st, x),
            0xAE4C => self.move_h(x),
            0x88C9 => {
                if self.o[x].aid == 0x4A {
                    self.o[x].atick = 0;
                    self.o[x].aframe = 0;
                }
                self.move_h(x);
                self.move_v(x);
            }
            0xA8C3 => self.ai_a8c3(x),
            0xBB6C => self.ai_bb6c(x),
            0xBC44 => {
                if self.p.state != 4 {
                    self.ai_bc4a(x);
                }
            }
            0xBC4A => self.ai_bc4a(x),
            0xBC7A => {
                // a dropped item that falls first (probe id `$18`), then waits as `$BC4A`
                if self.vertical_collide(st, x, 0x18) {
                    self.o[x].ptr = 0xBC4A;
                }
                self.ai_bc4a(x);
            }
            0xBBC3 => self.ai_bbc3(st, x),
            0x8823 | 0x8824 => {}
            0x8825 => self.ai_8825(st, x),
            0xB9F4 => self.ai_b9f4(x),
            0xA3FD => self.ai_a3fd(st, x),
            0xB622 => self.ai_b622(x),
            0xAEA3 => self.ai_aea3(x),
            0xAEB7 => self.ai_aeb7(x),
            0xA2B9 => self.ai_a2b9(st, x),
            0xA2CD => self.ai_a2cd(x),
            0xA378 => self.ai_a378(x),
            0xA3D5 => self.ai_a3d5(st, x),
            0xA3FA => self.ai_bc4a(x),
            0xB66C => self.ai_b66c(x),
            0xAFB7 => self.ai_afb7(x),
            0xB033 => self.ai_b033(x),
            0xB099 => self.ai_b099(x),
            0xB0D5 => self.ai_b0d5(x),
            0xB684 => self.ai_b684(x),
            0xB6B5 => self.ai_b6b5(x),
            0xBA4B => self.ai_ba4b(st, x),
            0xB37E => self.ai_b37e(st, x),
            0xB3D9 => self.ai_b3d9(st, x),
            0xAFEC => self.ai_afec(x),
            0xB01B => self.ai_b01b(x),
            0xB07D => self.ai_b07d(st, x),
            0xB430 | 0xB434 => self.ai_b434(x),
            0xB475 => self.ai_b475(x),
            0xB50A => self.ai_b50a(x),
            0xB58F => self.move_h(x),
            0xB592 => {
                self.move_gravity(x);
                self.move_h(x);
            }
            0xB598 => self.ai_b598(x),
            0xB5BD => self.ai_b5bd(x),
            0xB61F => self.move_h(x),
            0xA558 => self.ai_a558(x),
            0xA649 => self.ai_a649(x),
            0xA69D => self.ai_a69d(st, x),
            0xA6D3 => self.ai_a6d3(st, x),
            0xA705 => self.ai_a705(x),
            0xA1DE => self.ai_a1de(x),
            0xA223 => self.ai_a223(st, x),
            0xA29F => self.ai_a29f(st, x),
            0xA555 => self.move_h(x),
            0xA6CD => {
                self.move_h(x);
                self.move_v(x);
            }
            0xAFB0 => self.o[x].ypx = self.w13e,
            0x9000 => self.ai_9000(x),
            0x9063 => self.ai_9063(x),
            0x90EA => self.move_v(x),
            0xA7F9 => self.ai_a7f9(st, x),
            0xA84F => self.ai_a84f(st, x),
            p if self.run_weapons(st, x, p) => {}
            p if self.run_stage6(st, x, p) => {}
            p if self.run_boss7c(st, x, p) => {}
            p if self.run_stage8(st, x, p) => {}
            p if self.run_stage9(st, x, p) => {}
            p if self.run_stage10(st, x, p) => {}
            p if self.run_boss9e(st, x, p) => {}
            p if self.run_boss92(x, p) => {}
            p if self.run_stage12(x, p) => {}
            p if self.run_boss6c(x, p) => {}
            p if self.run_defeat29(x, p) => {}
            p if self.run_stage13(st, x, p) => {}
            p if self.run_stage14(x, p) => {}
            p if self.run_wily14(st, x, p) => {}
            p if self.run_wily14b(x, p) => {}
            p if self.run_boss87(st, x, p) => {}
            p if self.run_boss95(x, p) => {}
            p if self.run_stage0(st, x, p) => {}
            p if self.run_stage1(st, x, p) => {}
            p if self.run_stage3(st, x, p) => {}
            p if self.run_stage3b(x, p) => {}
            p if self.run_stage4(st, x, p) => {}
            p if self.run_stage4b(x, p) => {}
            p if self.run_boss75(st, x, p) => {}
            p if self.run_stage7(st, x, p) => {}
            p if self.run_boss79(st, x, p) => {}
            p if self.run_boss84(st, x, p) => {}
            p if self.run_boss8d(st, x, p) => {}
            p if self.run_boss71(st, x, p) => {}
            p if self.run_boss8b(st, x, p) => {}
            p if self.run_boss(st, x, p) => {}
            p => {
                if !self.unported.contains(&p) {
                    self.unported.push(p);
                }
            }
        }
    }

    /// AI $10, first state ($3A9000): hover toward Mega Man for a random time, then start the attack run and drop an egg.
    fn ai_9000(&mut self, x: usize) {
        if self.o[x].s[0] == 0 {
            let sum = self.rng[0] as u16 + self.rng[2] as u16 + self.carry as u16;
            self.rng[0] = sum as u8;
            let k = (self.rng[0] & 7) as u16;
            self.o[x].s[0] = self.ai_byte(0x90A2 + k);
        }
        self.move_h(x);
        self.o[x].s[0] = self.o[x].s[0].wrapping_sub(1);
        if self.o[x].s[0] != 0 {
            return;
        }
        self.o[x].ptr = 0x9063;
        self.o[x].hitbox = 0xC0;
        self.ai_9073(x);
        self.o[x].atick = 0;
        self.o[x].aframe = 0;
        if let Some(y) = self.find_slot() {
            self.o[y].ty = 0x11;
            self.child_at(x, y, 2, 4);
            let c = &mut self.o[y];
            c.dir = 0x08;
            c.yvs = 0;
            c.yvp = 2;
            c.hitbox = 0xCF;
            c.hp = 1;
        }
    }

    /// AI $10, attack run ($3A9063): fly along the heading, re-aim every 8 frames.
    fn ai_9063(&mut self, x: usize) {
        self.move_h(x);
        self.move_v(x);
        self.o[x].s[0] = self.o[x].s[0].wrapping_add(1);
        if self.o[x].s[0] & 7 == 0 {
            self.ai_9073(x);
        }
    }

    /// $3A9073: aim, load the speeds for the heading and pick the facing animation.
    fn ai_9073(&mut self, x: usize) {
        let h = self.aim(x) as u16;
        let (a, b, c, d) = (self.ai_byte(0x90AA + h), self.ai_byte(0x90BA + h), self.ai_byte(0x90CA + h), self.ai_byte(0x90DA + h));
        let o = &mut self.o[x];
        o.xvs = a;
        o.xvp = b;
        o.yvs = c;
        o.yvp = d;
        o.aid = if o.dir >= 8 { 2 } else { 3 };
        o.atick = 0;
    }

    // ---- tile collision for objects (bank $3F: code_3FF0F8, 3FF3ED, 3ED2FC, 3ED428) ---------------------------------------------

    /// code_3ED552 / code_3ED566: how far an object's Y is lowered for its tile probes while a swaying background (`$F0` = 4) is active.
    fn probe_shift(&self, x: usize, vertical: bool) -> u8 {
        if self.f0 != 4 {
            return 0;
        }
        let o = &self.o[x];
        let limit = if self.p.state == 2 { 0xB7 } else { 0xB5 };
        let apply = if vertical && self.w13f & 4 == 0 && o.xpg != 0x0C { true } else { o.ypx < limit };
        if apply {
            self.w13e
        } else {
            0
        }
    }

    /// Terrain types along vertical probe `id` at object `x`: (types, probe Y). An object off screen 0 or a probe outside 0..239 sees nothing.
    fn vprobe(&self, st: &Stage, x: usize, id: usize) -> (Vec<u8>, i32) {
        let o = &self.o[x];
        let p = st.hitboxes.vertical(id);
        let y = o.ypx.wrapping_sub(self.probe_shift(x, true)) as i32 + p.fixed;
        let wx = (o.xpg as i32) << 8 | o.xpx as i32;
        let lim = if p.fixed < 0 { 256 } else { 240 };
        if o.ypg != 0 || !(0..lim).contains(&y) {
            return (vec![0; p.along.len()], y);
        }
        let types: Vec<u8> = p.along.iter().map(|dx| st.terrain_lim(wx + dx, y, lim)).collect();
        // code_3ED3FD: with a sway object active, an empty probe whose unshifted Y is >= $C0 counts as solid ground
        if self.f0 == 4 && self.w13f & 4 == 0 && o.xpg != 0x0C && types.iter().all(|&t| t == 0) {
            let sum = (y as u8).wrapping_add(self.w13e);
            if sum >= 0xC0 {
                return (vec![1; types.len()], sum as i32);
            }
        }
        (types, y)
    }

    /// Terrain types along horizontal probe `id`: (types, probe X; the game keeps its low byte in `$12`).
    fn hprobe(&self, st: &Stage, x: usize, id: usize) -> (Vec<u8>, i32) {
        let o = &self.o[x];
        let p = st.hitboxes.horizontal(id);
        let wx = ((o.xpg as i32) << 8 | o.xpx as i32) + p.fixed;
        let ypx = o.ypx.wrapping_sub(self.probe_shift(x, false)) as i32;
        // code_3ED428: a first probe point below the screen (Y page > 0 or Y >= $F0) reads row $EF, above it (negative page / wrapped) row 0;
        // the whole probe then repeats that one tile.
        let first = p.along.first().copied().unwrap_or(0);
        let clamp = if (o.ypg as i8) < 0 {
            Some(0)
        } else if o.ypg != 0 {
            Some(0xEF)
        } else if ypx + first < 0 {
            Some(0)
        } else if ypx + first >= 0xF0 {
            Some(0xEF)
        } else {
            None
        };
        if let Some(row) = clamp {
            let t = st.terrain(wx, row);
            return (vec![t; p.along.len()], wx);
        }
        (p.along.iter().map(|dy| st.terrain(wx, ypx + dy)).collect(), wx)
    }

    /// code_3FF2F6 for objects: fall faster, at most 7 px/frame.
    /// Returns the carry the routine leaves (SBC of the speed's high byte, or the CMP #$F9 when it is negative).
    fn gravity_step(&mut self, x: usize) -> bool {
        let g = self.gravity;
        let o = &mut self.o[x];
        let (s, b) = o.yvs.overflowing_sub(g);
        o.yvs = s;
        let no_borrow = !(b && o.yvp == 0);
        o.yvp = o.yvp.wrapping_sub(b as u8);
        if o.yvp & 0x80 == 0 {
            return no_borrow;
        }
        let ge = o.yvp >= 0xF9;
        if !ge {
            o.yvp = 0xF9;
            o.yvs = 0;
        }
        ge
    }

    /// code_3FF324: vertical speed = -1.0 (the "standing on the ground" speed).
    fn land_speed(&mut self, x: usize) {
        self.o[x].yvs = 0;
        self.o[x].yvp = 0xFF;
    }

    /// code_3FF2B8: move down by the (negative) vertical speed.
    fn fall_move(&mut self, x: usize) {
        let o = &mut self.o[x];
        let (s, b1) = o.ysub.overflowing_sub(o.yvs);
        o.ysub = s;
        let mut p = o.ypx.wrapping_sub(o.yvp).wrapping_sub(b1 as u8);
        if p >= 0xF0 {
            p = p.wrapping_add(0x10);
            o.ypg = o.ypg.wrapping_add(1);
        }
        o.ypx = p;
    }

    /// code_3ED9E7: after landing, lift the object so its feet sit on the tile top.
    fn push_up(&mut self, x: usize, probe_y: i32) {
        let o = &mut self.o[x];
        let (p, borrow) = o.ypx.overflowing_sub((probe_y & 0x0F) as u8);
        o.ypx = p;
        if borrow {
            o.ypx = o.ypx.wrapping_sub(0x10);
            o.ypg = o.ypg.wrapping_sub(1);
        }
    }

    /// code_3ED9CD: after bumping the head, push the object down to the next tile boundary.
    fn push_down(&mut self, x: usize, probe_y: i32) {
        let o = &mut self.o[x];
        let mut p = o.ypx.wrapping_add(((probe_y & 0x0F) as u8 ^ 0x0F).wrapping_add(1));
        if p >= 0xF0 {
            p = p.wrapping_add(0x10);
            o.ypg = o.ypg.wrapping_add(1);
        }
        o.ypx = p;
    }

    /// code_3FF0F8 for X != 0: move by the vertical speed, apply gravity, probe with hitbox probe `id` (feet) or `id + 1` (head).
    /// Returns true when the object landed (carry set in the original).
    fn vertical_collide(&mut self, st: &Stage, x: usize, id: usize) -> bool {
        if self.o[x].yvp & 0x80 != 0 {
            self.fall_move(x);
            self.gravity_step(x);
            let (types, y) = self.vprobe(st, x, id);
            self.pt = types.clone();
            let top = types.iter().copied().max() == Some(4) && (y & 0x0F) < 8;
            if top || types.iter().any(|t| t & 1 != 0) {
                self.push_up(x, y);
                self.land_speed(x);
                self.carry = true;
                return true;
            }
            self.carry = false;
            false
        } else {
            self.up(x);
            self.gravity_step(x);
            let (types, y) = self.vprobe(st, x, id + 1);
            self.pt = types.clone();
            if types.iter().any(|t| t & 1 != 0) {
                self.push_down(x, y);
                self.land_speed(x);
            }
            self.carry = false;
            false
        }
    }

    /// code_3FF3ED: move sideways by the X speed in the direction of `$0420` and push out of a wall (returns true on a hit).
    fn horizontal_collide(&mut self, st: &Stage, x: usize, id: usize) -> bool {
        let hit = self.horizontal_collide_inner(st, x, id);
        self.carry = hit;
        hit
    }

    fn horizontal_collide_inner(&mut self, st: &Stage, x: usize, id: usize) -> bool {
        let d = self.o[x].dir;
        if d & 3 == 0 {
            return false;
        }
        if d & 1 != 0 {
            self.o[x].flags |= 0x40;
            self.right(x);
            let (types, px) = self.hprobe(st, x, id);
            self.pt = types.clone();
            if types.iter().any(|t| t & 1 != 0) {
                let o = &mut self.o[x];
                let (p, b) = o.xpx.overflowing_sub((px & 0x0F) as u8);
                o.xpx = p;
                if b {
                    o.xpg = o.xpg.wrapping_sub(1);
                }
                return true;
            }
        } else {
            self.o[x].flags &= !0x40;
            self.left(x);
            let (types, px) = self.hprobe(st, x, id + 1);
            self.pt = types.clone();
            if types.iter().any(|t| t & 1 != 0) {
                let o = &mut self.o[x];
                let add = (((px & 0x0F) as u8) ^ 0x0F).wrapping_add(1);
                let (p, c) = o.xpx.overflowing_add(add);
                o.xpx = p;
                if c {
                    o.xpg = o.xpg.wrapping_add(1);
                }
                return true;
            }
        }
        false
    }

    /// code_3FF452: a child at the same position as `x`.
    fn child_here(&mut self, x: usize, y: usize, anim: u8) {
        let src = self.o[x];
        let c = &mut self.o[y];
        c.xpx = src.xpx;
        c.xpg = src.xpg;
        c.ypx = src.ypx;
        c.ypg = src.ypg;
        self.init_child(x, y, anim);
    }

    /// AI $5B ($3DAD0C): a gate trigger. When Mega Man touches it the shutter of its page closes ($0670 = $70), a palette animation runs
    /// ($06F2 = $8E) and when that ends the shutter is final ($7F). Only the $5B variant is ported; $57-$5A spawn objects.
    fn ai_ad0c(&mut self, st: &Stage, x: usize) {
        if self.o[x].aframe == 0 && self.touches_player(x) && self.o[x].s[0] == 0 {
            if self.o[x].ty != 0x5B {
                self.unported.push(0xAD25);
                return;
            }
            let o = &mut self.o[x];
            o.s[0] = o.s[0].wrapping_add(1);
            o.s[1] = o.s[1].wrapping_add(1);
            o.aframe = o.aframe.wrapping_add(1);
            o.atick = 0;
            let pg = o.xpg as usize;
            self.pal.anim[2] = 0x8E;
            self.pal.step[2] = 0;
            self.pal.tick[2] = 0;
            st.door[pg].set(0x70);
            return;
        }
        // code_3DAD91
        if self.o[x].s[0] & 1 != 0 {
            if self.o[x].ty != 0x5B {
                self.unported.push(0xAD98);
                return;
            }
            if self.o[x].aframe != 2 {
                return;
            }
            self.o[x].s[0] = self.o[x].s[0].wrapping_add(1);
        }
        self.o[x].atick = 0;
        if self.o[x].s[1] == 0 {
            return;
        }
        // code_3DADF2
        if self.pal.anim[2] != 0 {
            return;
        }
        let pg = self.o[x].xpg as usize;
        st.door[pg].set(0x7F);
        if self.o[x].xpg == self.cam_page {
            self.o[x].s[1] = 0; // the attribute-table update ($0640, $1E) is visual only
        }
    }

    // ---- enemy AI: stage 2 ------------------------------------------------------------------------------------------------

    /// AI $40 ($3BA1DE): invisible dripper. While Mega Man is within 32 px it drops a water drop every 120 frames.
    fn ai_a1de(&mut self, x: usize) {
        if self.w13c != 0 {
            return;
        }
        if self.dist_x(x) >= 0x20 {
            self.o[x].s[0] = 0;
            return;
        }
        if self.o[x].s[0] != 0 {
            self.o[x].s[0] -= 1;
            return;
        }
        self.o[x].s[0] = 0x78;
        if let Some(y) = self.find_slot() {
            let c = &mut self.o[y];
            c.ty = 0x98;
            c.hitbox = 0xC0;
            c.hp = 1;
            c.yvs = 0;
            c.yvp = 0;
            self.child_here(x, y, 0x67);
        }
    }

    /// AI $98 ($3BA223): the falling drop; splashes into four droplets when it has landed.
    fn ai_a223(&mut self, st: &Stage, x: usize) {
        if self.o[x].aid != 0x6C {
            if self.vertical_collide(st, x, 0x0C) {
                self.set_anim(x, 0x6C);
            }
            return;
        }
        if self.o[x].atick < 5 {
            return;
        }
        self.despawn(x);
        for i in (0..4usize).rev() {
            let Some(y) = self.find_slot() else { return };
            let t = |a: u16| self.ai_byte(a + i as u16);
            let (xvs, xvp, yvs, yvp, dir, anim) = (t(0xA28F), t(0xA293), t(0xA287), t(0xA28B), t(0xA297), t(0xA29B));
            let c = &mut self.o[y];
            c.ty = 0x41;
            c.hitbox = 0xCF;
            c.hp = 1;
            c.xvs = xvs;
            c.xvp = xvp;
            c.yvs = yvs;
            c.yvp = yvp;
            c.dir = dir;
            self.child_here(x, y, anim);
        }
    }

    /// AI $41 ($3BA29F): a splash droplet; vanishes when it touches the ground or a wall.
    fn ai_a29f(&mut self, st: &Stage, x: usize) {
        if self.vertical_collide(st, x, 0x14) || self.horizontal_collide(st, x, 0x1E) {
            self.despawn(x);
        } else {
            self.o[x].flags &= !0x40;
        }
    }

    /// code_3FF4F6: the flip flag follows the facing bit.
    fn face_flag(&mut self, x: usize) {
        let o = &mut self.o[x];
        o.flags = (o.flags & 0xBF) | ((o.dir & 1) << 6);
        self.carry = o.dir & 4 != 0; // the last of the three RORs shifts bit 2 into the carry
    }

    /// AI $50 / $56 and friends, idle state ($3BA558): wait until Mega Man is within 64 px, then pop out and attack.
    fn ai_a558(&mut self, x: usize) {
        if self.o[x].aframe == 0 {
            self.o[x].atick = 0;
            self.face_flag(x);
            if self.dist_x(x) >= 0x40 {
                return;
            }
            self.o[x].aframe += 1;
            self.o[x].hitbox = 0xC0;
            self.land_speed(x);
            return;
        }
        if self.o[x].atick != 4 {
            return;
        }
        match self.o[x].aframe {
            1 => self.o[x].ypx = self.o[x].ypx.wrapping_sub(3),
            4 => {
                let ty = self.o[x].ty;
                if ty != 0x5C {
                    self.shoot_a5fe(x, 2);
                }
                match ty {
                    0x50 => {
                        self.o[x].ptr = 0xA69D;
                        self.o[x].s[0] = 0x40;
                        self.set_anim(x, 0x89);
                    }
                    0x56 => {
                        self.o[x].ptr = 0xA6D3;
                        // ADC with the carry left by CMP #$56 (set)
                        let sum = self.rng[2] as u16 + self.rng[1] as u16 + 1;
                        self.rng[1] = sum as u8;
                        let k = (self.rng[1] & 1) as u16;
                        self.o[x].yvs = self.ai_byte(0xA690 + k);
                        self.o[x].yvp = self.ai_byte(0xA692 + k);
                        self.o[x].xvs = 0xCC;
                        self.o[x].xvp = 0;
                        self.set_anim(x, 0x87);
                    }
                    _ => {
                        self.o[x].ptr = 0xA705;
                        self.o[x].s[2] = 3;
                        self.set_anim(x, 0x88);
                    }
                }
            }
            _ => {}
        }
    }

    /// $3BA5FE / $3BA604: fire bullets (type $51) from `x`; `last` is the highest table index (2 = three bullets, 0 = one).
    fn shoot_a5fe(&mut self, x: usize, last: usize) {
        for i in (0..=last).rev() {
            let Some(y) = self.find_slot() else { return };
            let (a, b, c) = (self.ai_byte(0xA694 + i as u16), self.ai_byte(0xA697 + i as u16), self.ai_byte(0xA69A + i as u16));
            let dir = self.o[x].dir;
            let c_ = &mut self.o[y];
            c_.ty = 0x51;
            c_.hitbox = 0x8F;
            c_.dir = dir;
            self.child_at(x, y, (dir & 1) + 0x39, 0x18);
            let c_ = &mut self.o[y];
            c_.xvs = a;
            c_.yvs = a;
            c_.xvp = b;
            c_.yvp = b;
            c_.dir |= c;
        }
    }

    /// $3BA649: back in the shell after attacking: wait, then go back to the idle state.
    fn ai_a649(&mut self, x: usize) {
        if self.o[x].s[1] != 0 {
            self.o[x].atick = 0;
            self.o[x].s[1] -= 1;
            if self.o[x].s[1] != 0 {
                return;
            }
            self.o[x].ptr = 0xA558;
            self.face_player(x);
            return;
        }
        if self.o[x].atick != 4 {
            return;
        }
        match self.o[x].aframe {
            1 => self.o[x].ypx = self.o[x].ypx.wrapping_add(3),
            3 => {
                self.o[x].hitbox = 0x80;
                self.o[x].s[1] = 0x40;
                self.set_anim(x, 0x86);
            }
            _ => {}
        }
    }

    /// $3BA6B9: go to the "retreat" state.
    fn go_a649(&mut self, x: usize) {
        self.o[x].ptr = 0xA649;
        self.o[x].hitbox = 0xA0;
        self.set_anim(x, 0x8F);
    }

    /// AI $50 attack state ($3BA69D): land, count down, face Mega Man and shoot three bullets.
    fn ai_a69d(&mut self, st: &Stage, x: usize) {
        if !self.vertical_collide(st, x, 0x1D) {
            return;
        }
        if !self.horizontal_collide(st, x, 0x2A) {
            self.o[x].s[0] = self.o[x].s[0].wrapping_sub(1);
            if self.o[x].s[0] != 0 {
                return;
            }
            self.face_player(x);
            self.face_flag(x);
            self.shoot_a5fe(x, 2);
        }
        self.go_a649(x);
    }

    /// AI $56 hop state ($3BA6D3).
    fn ai_a6d3(&mut self, st: &Stage, x: usize) {
        let f = self.o[x].aframe;
        if f & 3 == 0 {
            if f != 0 && self.o[x].atick == 4 {
                self.go_a649(x);
            }
            return;
        }
        if self.vertical_collide(st, x, 0x1D) {
            self.o[x].atick = 0;
            self.o[x].aframe = 4;
            return;
        }
        self.o[x].atick = 0;
        self.horizontal_collide(st, x, 0x2A);
    }

    /// $3BA705: shoot single bullets a few times.
    fn ai_a705(&mut self, x: usize) {
        if self.o[x].s[0] != 0 {
            self.o[x].s[0] -= 1;
            if self.o[x].s[0] != 0 {
                return;
            }
        }
        self.face_player(x);
        self.shoot_a5fe(x, 0);
        self.o[x].s[2] = self.o[x].s[2].wrapping_sub(1);
        if self.o[x].s[2] == 0 {
            self.go_a649(x);
        } else {
            self.o[x].s[0] = 0x28;
        }
    }

    /// code_3FF0C1 for X != 0: move up by the Y speed, probe `id`; a ceiling pushes the object down (returns true).
    fn up_collide(&mut self, st: &Stage, x: usize, id: usize) -> bool {
        self.up(x);
        let (types, y) = self.vprobe(st, x, id);
        self.pt = types.clone();
        if types.iter().any(|t| t & 1 != 0) {
            self.push_down(x, y);
            return true;
        }
        false
    }

    /// code_3FF088 for X != 0: move down by the Y speed, probe `id`; ground pushes the object up (returns true).
    fn down_collide(&mut self, st: &Stage, x: usize, id: usize) -> bool {
        self.down(x);
        let (types, y) = self.vprobe(st, x, id);
        self.pt = types.clone();
        if types.iter().any(|t| t & 1 != 0) {
            self.push_up(x, y);
            return true;
        }
        false
    }

    /// AI $8E ($3BAFEC): hanging bat. Waits (random 60 or 180 frames), then wakes up.
    fn ai_afec(&mut self, x: usize) {
        if self.o[x].s[0] == 0 {
            // SBC with the carry the AI loop's bank CMP left
            let a = self.rng[2].wrapping_sub(self.rng[1]).wrapping_sub(!self.carry as u8);
            self.o[x].s[0] = if a & 1 != 0 { 0xB4 } else { 0x3C };
            return;
        }
        self.o[x].s[0] -= 1;
        if self.o[x].s[0] != 0 {
            return;
        }
        self.o[x].hitbox = 0xC0;
        self.o[x].ptr = 0xB01B;
        self.set_anim(x, 0x6A);
    }

    /// AI $8E swoop ($3BB01B): fly toward Mega Man for 121 frames or until it touches him.
    fn ai_b01b(&mut self, x: usize) {
        if self.o[x].aid != 0x6B {
            if self.o[x].aframe != 2 {
                return;
            }
            self.set_anim(x, 0x6B);
        }
        if self.o[x].s[1] == 0 {
            let h = self.heading_to_player(x) as u16;
            let (a, b, c, d) = (self.ai_byte(0xB0B1 + h), self.ai_byte(0xB0C5 + h), self.ai_byte(0xB0B5 + h), self.ai_byte(0xB0C9 + h));
            let dir = self.rom.cpu_byte(0x3A, 0x3B, 0xF796 + h);
            let o = &mut self.o[x];
            o.xvs = a;
            o.xvp = b;
            o.yvs = c;
            o.yvp = d;
            o.dir = dir;
            o.s[1] = 0x79;
        }
        self.o[x].s[1] = self.o[x].s[1].wrapping_sub(1);
        self.move_h(x);
        self.move_v(x);
        if self.touches_player(x) {
            self.o[x].yvs = 0;
            self.o[x].yvp = 2;
            self.o[x].ptr = 0xB07D;
        }
    }

    /// AI $8E retreat ($3BB07D): fly up until the ceiling, then hang again.
    fn ai_b07d(&mut self, st: &Stage, x: usize) {
        if !self.up_collide(st, x, 0x3A) {
            return;
        }
        if self.o[x].aid != 0x6C {
            self.set_anim(x, 0x6C);
        }
        if self.o[x].aframe != 2 {
            return;
        }
        self.o[x].ptr = 0xAFEC;
        self.o[x].s[0] = 0;
        self.o[x].s[1] = 0;
        self.o[x].hitbox = 0x80;
        self.set_anim(x, 0x69);
    }

    /// AI $B0 ($3BB598): wall turret, waiting.
    fn ai_b598(&mut self, x: usize) {
        self.face_flag(x);
        if self.o[x].s[0] != 0 {
            self.o[x].atick = 0;
            self.o[x].s[0] -= 1;
            if self.o[x].s[0] != 0 {
                return;
            }
        }
        self.o[x].aframe += 1;
        self.o[x].hitbox = 0xC0;
        self.o[x].ptr = 0xB5BD;
    }

    /// AI $B0 ($3BB5BD): open, shoot one bullet, close.
    fn ai_b5bd(&mut self, x: usize) {
        if self.o[x].atick != 6 {
            return;
        }
        match self.o[x].aframe {
            1 => {
                let Some(y) = self.find_slot() else { return };
                let dir = self.o[x].dir;
                let c = &mut self.o[y];
                c.ty = 0xB1;
                c.hitbox = 0x8F;
                c.hp = 0;
                c.dir = dir;
                self.child_at(x, y, (dir & 1) + 0x39, 0x08);
                let c = &mut self.o[y];
                c.xvs = 0x80;
                c.xvp = 1;
            }
            3 => {
                // ADC with the carry left by CMP #$03 (set)
                let k = ((self.rng[3] as u16 + self.rng[1] as u16 + 1) & 1) as u16;
                self.o[x].s[0] = self.ai_byte(0xB61D + k);
                self.o[x].ptr = 0xB598;
                self.o[x].hitbox = 0x80;
            }
            _ => {}
        }
    }

    /// code_3FF6AB: turn around.
    fn turn_around(&mut self, x: usize) {
        self.o[x].dir ^= 3;
        self.o[x].flags ^= 0x40;
    }

    /// AI $48 ($3BA3FD): a shielded walker. Walks until it hits a wall or its timer ends, then plays the turn animation ($77 -> $76),
    /// and turns around on frame 6 of it. Its hitbox is vulnerable (bit 6) every frame except when a shot touches it from the front.
    fn ai_a3fd(&mut self, st: &Stage, x: usize) {
        if self.o[x].aid != 0x77 {
            if self.o[x].s[0] == 0 {
                self.o[x].s[0] = 0x40;
            }
            let mut turn = self.horizontal_collide(st, x, 0x14);
            if !turn {
                self.o[x].s[0] -= 1;
                turn = self.o[x].s[0] == 0;
            }
            if turn {
                self.set_anim(x, 0x77);
            }
        } else {
            if self.o[x].aframe != 6 {
                return;
            }
            self.o[x].s[0] = 0x40;
            self.set_anim(x, 0x76);
            self.turn_around(x);
        }
        self.o[x].hitbox |= 0x40;
        if let Some(y) = self.shot_hits(x) {
            let (e, s) = (self.o[x], self.o[y]);
            let enemy_right = ((e.xpg as u16) << 8 | e.xpx as u16) >= ((s.xpg as u16) << 8 | s.xpx as u16);
            let facing_right = e.flags & 0x40 != 0;
            if enemy_right != facing_right {
                self.o[x].hitbox &= 0xBF;
            }
        }
    }

    /// AI $A7 ($3BB430/$B434): picks the next attack. The state byte $0498 = 2 forces pattern 1, $04B0 = 2 forces pattern 0,
    /// otherwise it is random (the `ADC` takes the carry of the `CMP $04B0`).
    fn ai_b434(&mut self, x: usize) {
        if self.o[x].s[0] != 0 {
            self.o[x].s[0] -= 1;
            if self.o[x].s[0] != 0 {
                return;
            }
        }
        if self.o[x].aframe | self.o[x].atick != 0 {
            return;
        }
        let y: u16 = if self.o[x].s[2] == 2 {
            1
        } else if self.o[x].s[3] == 2 {
            0
        } else {
            let c = (2 >= self.o[x].s[3]) as u8;
            ((self.rng[2] as u16 + self.rng[3] as u16 + c as u16) & 1) as u16
        };
        let aid = self.ai_byte(0xB57B + y);
        let lo = self.ai_byte(0xB57D + y);
        let hi = self.ai_byte(0xB57F + y);
        let cnt = self.ai_byte(0xB581 + y);
        let o = &mut self.o[x];
        o.aid = aid;
        o.ptr = lo as u16 | (hi as u16) << 8;
        o.s[1] = cnt;
    }

    /// $B531: back to the chooser after a pattern: wait 60 frames in animation $60.
    fn ai_b531(&mut self, x: usize) {
        self.o[x].ptr = 0xB434;
        self.o[x].s[0] = 0x3C;
        self.set_anim(x, 0x60);
    }

    /// AI $A7 pattern 0 ($3BB475): on animation frame 3 throws a $A9 (falling, thrown at Mega Man), on frame 5 repeats or finishes.
    fn ai_b475(&mut self, x: usize) {
        self.face_player(x);
        self.face_flag(x);
        if self.o[x].s[0] != 0 {
            self.o[x].s[0] -= 1;
            if self.o[x].s[0] != 0 {
                return;
            }
            self.o[x].aid = self.o[x].aid.wrapping_add(1);
        }
        match self.o[x].aframe {
            3 => {
                self.o[x].aframe += 1;
                let Some(y) = self.find_slot() else { return };
                let dir = self.o[x].dir;
                let c = &mut self.o[y];
                c.ty = 0xA9;
                c.hitbox = 0x8F;
                c.hp = 0;
                c.dir = dir;
                self.child_at(x, y, 0x63 + (dir & 1), 0x63);
                self.o[y].yvs = 0;
                self.o[y].yvp = 4;
                let d = self.dist_x(x);
                let div = if d >= 0x60 { 0x32 } else { 0x28 };
                let (q4, q5) = Self::div_fc3a(0, d, 0, div);
                self.o[y].xvs = q4;
                self.o[y].xvp = q5;
            }
            5 => {
                self.set_anim(x, 0x60);
                self.o[x].s[1] = self.o[x].s[1].wrapping_sub(1);
                if self.o[x].s[1] == 0 {
                    self.o[x].s[3] = 0;
                    self.o[x].s[2] = self.o[x].s[2].wrapping_add(1);
                    self.ai_b531(x);
                } else {
                    self.o[x].s[0] = 0x20;
                }
            }
            _ => {}
        }
    }

    /// AI $A7 pattern 1 ($3BB50A): on animation frame 9 throws a $A8 (straight shot); frame $F repeats or finishes.
    fn ai_b50a(&mut self, x: usize) {
        self.face_player(x);
        self.face_flag(x);
        match self.o[x].aframe {
            9 => {
                self.o[x].aframe += 1;
                let Some(y) = self.find_slot() else { return };
                let dir = self.o[x].dir;
                let c = &mut self.o[y];
                c.ty = 0xA8;
                c.hitbox = 0x8F;
                c.hp = 0;
                c.dir = dir;
                self.child_at(x, y, 0x65 + (dir & 1), 0x08);
                self.o[y].xvs = 0;
                self.o[y].xvp = 4;
                self.x_junk = true; // B578: LDX $0F
            }
            0x0F => {
                self.o[x].s[1] = self.o[x].s[1].wrapping_sub(1);
                if self.o[x].s[1] == 0 {
                    self.o[x].s[2] = 0;
                    self.o[x].s[3] = self.o[x].s[3].wrapping_add(1);
                    self.ai_b531(x);
                } else {
                    self.o[x].aframe = 0;
                    self.o[x].atick = 0;
                }
            }
            _ => {}
        }
    }

    /// AI $B4 ($3BB622): a stationary launcher. While Mega Man is within 32 px it drops a `$B2` projectile every 120 frames.
    fn ai_b622(&mut self, x: usize) {
        if self.w13c != 0 {
            return;
        }
        self.face_flag(x);
        if self.dist_x(x) >= 0x20 {
            return;
        }
        if self.o[x].s[0] != 0 {
            self.o[x].s[0] -= 1;
            return;
        }
        let Some(y) = self.find_slot() else { return };
        let dir = self.o[x].dir;
        let c = &mut self.o[y];
        c.ty = 0xB2;
        c.hitbox = 0xC0;
        c.hp = 1;
        c.dir = dir;
        self.child_here(x, y, 0x9E);
        let c = &mut self.o[y];
        c.yvs = 0;
        c.yvp = 3;
        self.o[x].s[0] = 0x78;
    }

    /// AI $42 ($3BA2B9), first state of the item carrier: drops to the ground, turns at walls, then behaves like $43 ($A2CD).
    fn ai_a2b9(&mut self, st: &Stage, x: usize) {
        self.face_flag(x);
        if !self.vertical_collide(st, x, 0x0E) {
            return;
        }
        if self.horizontal_collide(st, x, 0x22) {
            self.turn_around(x);
        }
        self.ai_a2cd(x);
    }

    /// AI $42/$43 ($3BA2CD): waits until Mega Man is within 32 px, plays animation $59 and on its frame 4 drops an item ($44) that is
    /// chosen by the random state; on frame 7 it rises away ($A378).
    fn ai_a2cd(&mut self, x: usize) {
        self.face_flag(x);
        if self.o[x].aid != 0x59 {
            if self.dist_x(x) >= 0x20 {
                return;
            }
            self.o[x].ptr = 0xA2CD;
            self.set_anim(x, 0x59);
            return;
        }
        match self.o[x].aframe {
            4 => {
                self.o[x].aframe += 1;
                self.o[x].atick = 0;
                let Some(y) = self.find_slot() else { return };
                let parent = self.o[x];
                let c = &mut self.o[y];
                c.ty = 0x44;
                c.idx = parent.idx;
                c.hitbox = 0;
                c.yvs = 0;
                c.xvs = 0;
                c.xvp = 1;
                c.yvp = 4;
                c.dir = parent.dir;
                let e7 = self.rng[1].wrapping_add(self.rng[3]); // ADC with the carry FB6C left clear
                self.rng[3] = e7;
                let k = (e7 & 0x0F) as u16;
                let base: u16 = if parent.ty == 0x43 { 0xA3C5 } else { 0xA3B5 };
                let anim = self.ai_byte(base + k);
                self.child_here(x, y, anim);
                self.o[y].ypx = self.o[y].ypx.wrapping_sub(0x0C);
            }
            7 => {
                let o = &mut self.o[x];
                o.s[0] = 0x50;
                o.ptr = 0xA378;
                o.yvs = 0;
                o.yvp = 0;
                self.set_anim(x, 0x58);
            }
            _ => {}
        }
    }

    /// AI $42/$43 ($3BA378): after dropping the item it hovers 80 frames, then flies up ($6D) with growing speed until it leaves the screen.
    fn ai_a378(&mut self, x: usize) {
        if self.o[x].s[0] != 0 {
            self.o[x].s[0] -= 1;
            if self.o[x].s[0] != 0 {
                return;
            }
            self.o[x].dir = 8;
            self.set_anim(x, 0x6D);
            return;
        }
        if self.o[x].aframe == 4 {
            self.o[x].atick = 0;
        }
        let o = &mut self.o[x];
        let (s, c) = o.yvs.overflowing_add(0x40);
        o.yvs = s;
        o.yvp = o.yvp.wrapping_add(c as u8);
        self.up(x);
        if self.o[x].ypg != 0 {
            self.despawn(x);
        }
    }

    /// AI $44 ($3BA3D5): the dropped item falls; once it has landed it is an ordinary pickup ($BC4A).
    fn ai_a3d5(&mut self, st: &Stage, x: usize) {
        if self.vertical_collide(st, x, 0x0E) {
            self.o[x].ptr = 0xA3FA;
        }
        let hit = self.horizontal_collide(st, x, 0x18);
        self.o[x].flags &= 0xBF;
        if hit {
            self.o[x].dir = 0;
        }
        self.ai_bc4a(x);
    }

    /// AI $5E ($3DAEA3), first state: asks the NMI to draw the section's columns (`$1B` = $80) and follows Mega Man's X.
    fn ai_aea3(&mut self, x: usize) {
        self.p1c = 0x18;
        self.p1b = 0x80;
        self.o[x].ptr = 0xAEB7;
        self.follow_player_x(x);
    }

    /// AI $5E ($3DAEB7): a background-sway object. Once the renderer is idle it sets the scroll mode (`$F0` = 4, `$F1` = $C0) and then moves
    /// up and down by 64 px (speed 0.5, direction flips every 128 frames); its Y is the background's vertical scroll (`$FA`).
    fn ai_aeb7(&mut self, x: usize) {
        if self.f0 == 0 {
            if self.p1b != 0 {
                self.follow_player_x(x);
                return;
            }
            self.w13e = 0;
            self.f1 = 0xC0;
            self.f0 = 4;
            let o = &mut self.o[x];
            o.yvs = 0x80;
            o.yvp = 0;
            o.dir = 8;
            self.w13f = 4;
            self.o[x].s[0] = 0x80;
        }
        self.move_v(x);
        self.o[x].s[0] = self.o[x].s[0].wrapping_sub(1);
        if self.o[x].s[0] == 0 {
            self.o[x].dir ^= 0x0C;
            self.w13f ^= 0x0C;
            self.o[x].s[0] = 0x80;
        }
        let y = self.o[x].ypx;
        self.fa = y;
        self.fd = if y != 0 { 2 } else { 0 };
        self.w13e = if y == 0 {
            0
        } else {
            let (t, c) = y.overflowing_add(0x10);
            (!t).wrapping_add(1).wrapping_add(c as u8)
        };
        self.follow_player_x(x);
    }

    fn follow_player_x(&mut self, x: usize) {
        self.o[x].xpx = self.p.xpx;
        self.o[x].xpg = self.p.xpg;
    }

    /// AI $73 ($3DAFB7): a ceiling emitter. After 60 frames it releases four `$74` pieces, one every 21 frames, rising from the bottom of
    /// the screen to the heights in `$B13C` (each stays for 61/41/21/1 frames, `$B144`), then it is gone.
    fn ai_afb7(&mut self, x: usize) {
        if self.o[x].s[0] != 0x3C {
            self.o[x].s[0] = self.o[x].s[0].wrapping_add(1);
            return;
        }
        if self.o[x].s[1] == 0 {
            let k = self.o[x].s[2] as u16;
            let (d0e, d0f) = (self.ai_byte(0xB13C + k), self.ai_byte(0xB144 + k));
            let Some(y) = self.find_slot() else {
                self.despawn(x);
                return;
            };
            let s2 = self.o[x].s[2];
            self.child_at(x, y, (s2 & 1) + 0x39, s2.wrapping_add(0xA5));
            let c = &mut self.o[y];
            c.flags |= 2;
            c.dir = 8;
            c.s[0] = c.ypx.wrapping_add(d0e);
            c.s[1] = d0f;
            c.ypx = 0xE8;
            c.yvs = 0;
            c.yvp = 2;
            c.ty = 0x74;
            c.hitbox = 0;
            self.o[x].s[1] = 0x15;
            self.o[x].s[2] = s2.wrapping_add(1);
            if s2.wrapping_add(1) == 4 {
                self.despawn(x);
                return;
            }
        }
        self.o[x].s[1] = self.o[x].s[1].wrapping_sub(1);
    }

    /// AI $74 ($3DB033), first state: a piece waits `$0498` frames, then moves vertically. Rising pieces (dir 8) stop at their target height
    /// (`$0468`) and hold there for `$0480` frames; the one with animation `$A8` then turns into the burst (`$A9`), the others vanish.
    /// Falling pieces (dir 4) vanish below the screen.
    fn ai_b033(&mut self, x: usize) {
        if self.o[x].s[2] != 0 {
            self.o[x].s[2] -= 1;
            if self.o[x].s[2] != 0 {
                return;
            }
        }
        self.move_v(x);
        if self.o[x].dir & 8 == 0 {
            if self.o[x].ypg != 0 {
                self.despawn(x);
            }
            return;
        }
        if self.o[x].s[0] < self.o[x].ypx {
            return;
        }
        self.o[x].ypx = self.o[x].s[0];
        self.o[x].s[1] = self.o[x].s[1].wrapping_sub(1);
        if self.o[x].s[1] != 0 {
            return;
        }
        if self.o[x].aid != 0xA8 {
            self.despawn(x);
            return;
        }
        self.set_anim(x, 0xA9);
        let o = &mut self.o[x];
        o.hitbox = 0x14;
        o.ypx = o.ypx.wrapping_sub(8);
        o.xpx = o.xpx.wrapping_sub(8);
        o.ptr = 0xB099;
        o.s[0] = 0x58;
        o.s[1] = 2;
    }

    /// AI $74 ($3DB099): the burst holds on frame 12 of `$A9` while a 16-bit counter (`$0468/$0480`) runs down, then plays `$AA`.
    fn ai_b099(&mut self, x: usize) {
        if self.o[x].aframe != 0x0C {
            return;
        }
        self.o[x].atick = 0;
        let v = ((self.o[x].s[1] as u16) << 8 | self.o[x].s[0] as u16).wrapping_sub(1);
        self.o[x].s[0] = v as u8;
        self.o[x].s[1] = (v >> 8) as u8;
        if v != 0 {
            self.ai_b0d5(x);
            return;
        }
        self.set_anim(x, 0xAA);
        let o = &mut self.o[x];
        o.ptr = 0xB0D5;
        o.s[0] = 0x58;
        o.s[1] = 2;
    }

    /// AI $74 ($3DB0D5): on frame 2 of `$AA` the burst drops four falling pieces (`$B13C/$B140/$B144`: offsets and start delays) and is gone.
    fn ai_b0d5(&mut self, x: usize) {
        if self.o[x].aframe != 2 {
            return;
        }
        for k in (0..4usize).rev() {
            let (d0d, d0e, d0f) = (self.ai_byte(0xB13C + k as u16), self.ai_byte(0xB140 + k as u16), self.ai_byte(0xB144 + k as u16));
            let Some(y) = self.find_slot() else { break };
            self.child_here(x, y, 0xA5 + k as u8);
            let c = &mut self.o[y];
            c.flags |= 2;
            c.dir = 4;
            c.ypx = c.ypx.wrapping_add(d0d);
            c.xpx = c.xpx.wrapping_add(d0e);
            c.s[2] = d0f;
            c.yvs = 0;
            c.yvp = 2;
            c.ty = 0x74;
            c.hitbox = 0;
        }
        self.despawn(x);
    }

    /// AI $B2 ($3BB66C), first state: pick a random rise time from `$B6DD` (`ADC` with the carry the AI loop left).
    fn ai_b66c(&mut self, x: usize) {
        let y = (self.rng[2] as u16 + self.rng[3] as u16 + self.carry as u16) & 3;
        self.o[x].s[0] = self.ai_byte(0xB6DD + y);
        self.o[x].ptr = 0xB684;
    }

    /// AI $B2 ($3BB684): rise for the chosen time, then fall at a fixed speed while swinging sideways.
    fn ai_b684(&mut self, x: usize) {
        self.up(x);
        self.o[x].s[0] = self.o[x].s[0].wrapping_sub(1);
        if self.o[x].s[0] != 0 {
            return;
        }
        let o = &mut self.o[x];
        o.yvs = 0xB3;
        o.yvp = 0;
        o.xvs = 0x59;
        o.xvp = 0;
        o.dir = 2;
        o.s[1] = 0x0B;
        o.ptr = 0xB6B5;
    }

    /// AI $B2 ($3BB6B5): falling and swinging: the horizontal direction changes every 11 frames (`$B6E1`: left, right, right, left).
    fn ai_b6b5(&mut self, x: usize) {
        self.down(x);
        let flags = self.o[x].flags;
        self.move_h(x);
        self.o[x].flags = flags;
        self.o[x].s[1] = self.o[x].s[1].wrapping_sub(1);
        if self.o[x].s[1] != 0 {
            return;
        }
        self.o[x].s[1] = 0x0B;
        self.o[x].s[2] = self.o[x].s[2].wrapping_add(1);
        let y = (self.o[x].s[2] & 3) as u16;
        self.o[x].dir = self.ai_byte(0xB6E1 + y);
    }

    /// AI $46 ($3BB9F4): hopper on the ground: count down, then pick a jump (random, or by distance to Mega Man).
    fn ai_b9f4(&mut self, x: usize) {
        self.face_player(x);
        self.face_flag(x);
        self.o[x].atick = 0;
        if self.o[x].s[0] == 0 {
            self.o[x].s[0] = 0x1F;
        }
        self.o[x].s[0] -= 1;
        if self.o[x].s[0] != 0 {
            return;
        }
        let mut y = 0usize;
        let sum = self.rng[1] as u16 + self.rng[3] as u16 + self.carry as u16;
        if sum & 1 != 0 {
            y = 2;
            let d = self.dist_x(x);
            while d < self.ai_byte(0xBA83 + y as u16) {
                y -= 1;
                if y == 0 {
                    break;
                }
            }
        }
        let t = |a: u16| self.ai_byte(a + y as u16);
        let (yvs, yvp, xvs, xvp) = (t(0xBA8C), t(0xBA8F), t(0xBA86), t(0xBA89));
        let o = &mut self.o[x];
        o.yvs = yvs;
        o.yvp = yvp;
        o.xvs = xvs;
        o.xvp = xvp;
        o.ptr = 0xBA4B;
        o.aframe += 1;
    }

    /// AI $46 jump ($3BBA4B).
    fn ai_ba4b(&mut self, st: &Stage, x: usize) {
        if self.o[x].aid != 0x1F {
            if self.o[x].aframe != 2 {
                return;
            }
            self.o[x].ptr = 0xB9F4;
            self.o[x].s[0] = 0x3D;
            self.set_anim(x, 0x1F);
            return;
        }
        self.o[x].atick = 0;
        if self.vertical_collide(st, x, 0x32) {
            self.set_anim(x, 0x20);
            return;
        }
        self.horizontal_collide(st, x, 0x3C);
    }

    /// AI $A1 ($3BB37E): walker, waiting / turning.
    fn ai_b37e(&mut self, st: &Stage, x: usize) {
        if self.o[x].s[0] != 0 {
            self.o[x].atick = 0;
            self.o[x].s[0] -= 1;
            if self.o[x].s[0] != 0 {
                return;
            }
            let o = &mut self.o[x];
            o.ptr = 0xB3D9;
            o.s[3] = 1;
            o.xvs = 0;
            o.xvp = 2;
            o.hitbox = 0x8D;
            o.s[1] = 0x20;
            self.face_player(x);
            self.face_flag(x);
            self.set_anim(x, 0x68);
            return;
        }
        if !self.vertical_collide(st, x, 0x08) {
            return;
        }
        if self.horizontal_collide(st, x, 0x14) {
            self.turn_around(x);
            return;
        }
        let d = self.dist_x(x);
        if d >= 0x30 {
            return;
        }
        self.o[x].s[0] = 0x0A;
    }

    /// AI $A1 ($3BB3D9): walking fast, turns at walls, goes back to waiting after a few steps.
    fn ai_b3d9(&mut self, st: &Stage, x: usize) {
        if self.o[x].s[2] != 0 {
            self.o[x].s[2] -= 1;
            if self.o[x].s[2] != 0 {
                return;
            }
            self.o[x].s[1] = 0x20;
            self.turn_around(x);
        }
        if !self.vertical_collide(st, x, 0x08) {
            return;
        }
        if self.horizontal_collide(st, x, 0x14) {
            self.turn_around(x);
            return;
        }
        self.o[x].s[1] = self.o[x].s[1].wrapping_sub(1);
        if self.o[x].s[1] != 0 {
            return;
        }
        self.o[x].s[3] = self.o[x].s[3].wrapping_sub(1);
        if (self.o[x].s[3] as i8) >= 0 {
            self.o[x].s[2] = 0x1E;
            return;
        }
        let o = &mut self.o[x];
        o.ptr = 0xB37E;
        o.xvs = 0;
        o.xvp = 1;
        o.hitbox = 0xCD;
        self.face_player(x);
        self.face_flag(x);
        self.set_anim(x, 0x67);
    }

    // ---- Mega Man's shots and effects --------------------------------------------------------------------------------------

    /// What the player's shooting code looks at before it runs (code_3C8D3E scan of slots 3..1 and slot 5).
    pub fn obj_env(&self, stage: u8) -> ObjEnv {
        let mut shot_slot = None;
        let mut big_shot = false;
        for y in (1..=3usize).rev() {
            if self.o[y].ty == 0 {
                shot_slot = Some(y);
            } else if matches!(self.o[y].aid, 0x20 | 0x23) {
                big_shot = true;
            }
        }
        let mut env = ObjEnv { shot_slot, big_shot, slot5_free: self.o[5].ty == 0, slot1_free: self.o[1].ty == 0, slot4_free: self.o[4].ty == 0, slot4_aid: self.o[4].aid, flash_on: self.w13c != 0, rain_on: self.w13a != 0, shot_hi: (1..=3usize).rev().find(|&y| self.o[y].ty == 0), slot_aid: [self.o[0].aid, self.o[1].aid, self.o[2].aid, self.o[3].aid], slot_aframe: [self.o[0].aframe, self.o[1].aframe, self.o[2].aframe, self.o[3].aframe], slot6_free: self.o[6].ty == 0, stage, slots_clear: (8..SLOTS).all(|s| self.o[s].ty == 0), f0: self.f0, w13e: self.w13e, w13f: self.w13f, ef: self.ef_override.unwrap_or(0), p95: self.p95, crush_aid: self.o[22].aid, ..Default::default() };
        // code_3FF7D8 / code_3FF843 scan slots 23 down to 1 for objects Mega Man collides with (flag bit 0/1, on screen row 0, visible)
        for y in (1..SLOTS).rev() {
            let o = &self.o[y];
            if o.ty != 0 && o.ypg == 0 && o.flags & 4 == 0 && o.flags & 3 != 0 {
                let hb = (o.hitbox & 0x3F) as u16;
                env.solids[env.nsolids] = Solid { xpg: o.xpg, xpx: o.xpx, ypx: o.ypx, flags: o.flags, w: self.half_width(hb), h: self.half_height(hb), aid: o.aid };
                env.nsolids += 1;
            }
        }
        env
    }

    /// Put Mega Man's position into slot 0 (the parent of the objects he creates).
    fn set_parent(&mut self, x: i32, y: i32, flip: bool) {
        let (ypg, ypx) = crate::game::split_y(y);
        let p = &mut self.o[0];
        p.xpg = (x >> 16) as u8;
        p.xpx = (x >> 8) as u8;
        p.ypg = ypg;
        p.ypx = ypx;
        p.flags = (p.flags & !0x40) | if flip { 0x40 } else { 0 };
    }

    /// Create the objects the player asked for this frame (code_3C8D35 ff. and code_3C81B3).
    pub fn apply_player_events(&mut self, events: &[PlayerEvent]) {
        for ev in events {
            match *ev {
                PlayerEvent::Shot { slot, x, y, flip, anim, low } => {
                    self.set_parent(x, y, flip);
                    let dir = if flip { 1 } else { 2 };
                    self.o[slot].dir = dir;
                    self.child_at(0, slot, dir & 1, anim);
                    if low {
                        self.o[slot].ypx = self.o[slot].ypx.wrapping_add(8);
                    }
                    let c = &mut self.o[slot];
                    c.xvs = 0x33;
                    c.xvp = 4;
                    c.ty = 2;
                    c.hitbox = if c.aid == 0x20 { 1 } else { 0 };
                }
                PlayerEvent::Death { x, y, flip } => {
                    self.set_parent(x, y, flip);
                    // code_3A8243: sixteen particles, each in the highest free slot among 5..=23
                    for i in (0..16u16).rev() {
                        let Some(s) = (5..SLOTS).rev().find(|&s| self.o[s].ty == 0) else { break };
                        self.child_here(0, s, 0x19);
                        let (xvs, xvp, yvs, yvp) = (self.ai_byte(0x83D3 + i), self.ai_byte(0x83E3 + i), self.ai_byte(0x83F3 + i), self.ai_byte(0x8403 + i));
                        let c = &mut self.o[s];
                        c.ty = 0x2E;
                        c.hitbox = 0;
                        c.xvs = xvs;
                        c.xvp = xvp;
                        c.yvs = yvs;
                        c.yvp = yvp;
                    }
                }
                PlayerEvent::HitSpark { enemy, anim } => {
                    // the spark appears at the enemy that touched him (code_3A8236 runs with X = that enemy)
                    self.child_here(enemy, 5, anim);
                    self.o[5].ty = 1;
                    self.o[5].hitbox = 0;
                }
                PlayerEvent::Splash { x, y, flip, deep } => {
                    self.set_parent(x, y, flip);
                    self.child_here(0, 5, 0x2E);
                    let mut ypx = self.o[5].ypx;
                    if !deep {
                        ypx = ypx.wrapping_add(0x10);
                    }
                    ypx &= 0xF0;
                    self.o[5].ypx = ypx;
                    self.w13e = ypx;
                    self.o[5].ty = 0x6F;
                }
                PlayerEvent::Bubble { x, y, flip } => {
                    self.set_parent(x, y, flip);
                    self.o[6].hitbox = 0;
                    self.child_here(0, 6, 0x7E);
                    let (py, borrow) = self.o[6].ypx.overflowing_sub(4);
                    self.o[6].ypx = py;
                    if !borrow {
                        let o = &mut self.o[6];
                        o.ty = 0x66;
                        o.flags &= 0xBF;
                        o.yvs = 0;
                        o.yvp = 1;
                    }
                }
                PlayerEvent::Missile { weapon, x, y, flip } => self.spawn_missile(weapon, x, y, flip),
                PlayerEvent::Held { slot, x, y, flip } => self.spawn_held(slot, x, y, flip),
                PlayerEvent::Barrier { x, y, flip } => {
                    self.set_parent(x, y, flip);
                    self.child_here(0, 1, 0x50);
                    self.o[1].ty = 0x0F;
                    self.o[1].hitbox = 6;
                }
                PlayerEvent::Cloud { x, y, flip } => {
                    self.set_parent(x, y, flip);
                    self.child_at(0, 1, 2, 0x36);
                    let c = &mut self.o[1];
                    c.ty = 5;
                    c.yvs = 0;
                    c.yvp = 2;
                    c.hitbox = 0;
                }
                PlayerEvent::Coil { slot, idx, x, y, flip } => {
                    self.set_parent(x, y, flip);
                    self.child_at(0, slot, idx, 0x3E);
                    let c = &mut self.o[slot];
                    c.flags |= 1;
                    c.hitbox = 0x0D;
                    c.ty = 6;
                    c.yvs = 0x33;
                    c.yvp = 0;
                    c.dir = 8;
                    c.s[0] = 0xC8;
                    c.s[1] = 0;
                }
                PlayerEvent::Rush { x, y, flip, dir } => {
                    self.set_parent(x, y, flip);
                    self.o[4].dir = dir;
                    self.child_at(0, 4, (dir & 1) + 0x2B, 0x13);
                    let c = &mut self.o[4];
                    c.flags &= 0xBF;
                    c.ypx = 0;
                    c.ypg = 0;
                    c.yvs = 0;
                    c.yvp = 0;
                    c.ty = 4;
                }
                PlayerEvent::Slot4X { xpg, xpx } => {
                    self.o[4].xpg = xpg;
                    self.o[4].xpx = xpx;
                }
                PlayerEvent::Wire { x, y, flip, idx } => {
                    self.set_parent(x, y, flip);
                    self.child_at(0, 4, idx, 0x3C);
                    let c = &mut self.o[4];
                    c.flags &= 0xBF;
                    c.ty = 0x0B;
                    c.yvs = 0;
                    c.yvp = 4;
                    c.dir = 8;
                    c.s[0] = c.ypx.wrapping_add(2);
                    c.hitbox = 0;
                }
                PlayerEvent::FlashStart => self.w13c = 0x012C,
                PlayerEvent::FlashTick => self.w13c = self.w13c.wrapping_sub(1),
                PlayerEvent::Grow { slot } => self.set_anim(slot, 0x49),
                PlayerEvent::Kill { slot } => self.kill_slot(slot),
                PlayerEvent::Pharaoh { slot, anim, x, y, flip, held } => self.spawn_pharaoh(slot, anim, x, y, flip, held),
                PlayerEvent::SlideDust { x, y, flip, anim } => {
                    self.set_parent(x, y, flip);
                    self.child_here(0, 5, anim);
                    self.o[5].ty = 1;
                    self.o[5].hitbox = 0;
                }
            }
        }
    }

    /// AI $02 ($3A8825): Mega Man's buster shot flies straight; the charged shots change speed with their animation frame.
    fn ai_8825(&mut self, st: &Stage, x: usize) {
        if self.o[x].aid == 0x20 {
            let k = self.o[x].aframe as u16;
            self.o[x].xvs = self.ai_byte(0x88BD + k);
            self.o[x].xvp = self.ai_byte(0x88C3 + k);
            if k == 5 {
                self.set_anim(x, 0x23);
            }
        }
        self.move_h(x);
        self.shot_block_check(st, x);
    }

    /// code_3A884B: with a sway object active a buster shot that touches an unbroken $90 block starts its destruction: it is recorded in
    /// `$0140..$0142`, a decoration object ($80, animation $62) appears on the block's tile and the shot disappears (charged shots go on).
    fn shot_block_check(&mut self, st: &Stage, x: usize) {
        if self.f0 != 4 || self.w140 != 0 {
            return;
        }
        let (types, y) = self.vprobe(st, x, 2);
        if types[0] != 9 {
            return;
        }
        let rel = self.o[x].xpx.wrapping_sub(self.cam_now.lo);
        if rel >= 0xF0 || rel < 0x18 {
            return;
        }
        let p = st.hitboxes.vertical(2);
        let o = self.o[x];
        let wx = ((o.xpg as i32) << 8 | o.xpx as i32) + p.along.last().copied().unwrap_or(0);
        let ys = (y & 0xFF) as usize;
        let xs = (wx & 0xFF) as usize;
        self.w140 = o.xpg;
        self.w141 = ((ys >> 5) * 8 + (xs >> 5)) as u8;
        self.w142 = (((ys >> 4) & 1) * 2 + ((xs >> 4) & 1)) as u8;
        if let Some(c) = self.find_slot() {
            self.child_here(x, c, 0x62);
            let ch = &mut self.o[c];
            ch.ty = 0x80;
            ch.hitbox = 0;
            ch.xpx = (ch.xpx & 0xF0) | 8;
            ch.ypx = (ch.ypx & 0xF0) | 8;
        }
        if self.o[x].ty != 8 && !matches!(self.o[x].aid, 0x20 | 0x23) {
            self.despawn(x);
        }
    }

    // ---- Mega Man's weapon against enemies (bank $3A, code_3A8097-3A83D2, code_3FFA34) ----------------------------------------

    /// Remove object `y` (code_3FFCCF).
    fn kill_slot(&mut self, y: usize) {
        let o = &mut self.o[y];
        o.ptr &= 0x00FF;
        o.ty = 0;
        o.t5b8 = 0;
        o.hp = 0;
        o.idx = 0xFF;
    }

    /// Half extents of hitbox size `hb` as the contact test uses them (width limit `$F9F4`, height limit `$F9B4`).
    pub fn half_width(&self, hb: u16) -> u8 {
        self.rom.cpu_byte(0x3A, 0x3B, 0xF9F4 + hb)
    }

    pub fn half_height(&self, hb: u16) -> u8 {
        self.rom.cpu_byte(0x3A, 0x3B, 0xF9B4 + hb)
    }

    /// code_3FFA34 for the buster: does one of Mega Man's shots (slots 3..1) touch enemy `x`? The routine uses `$E6/$E7` as scratch,
    /// so the random state changes whenever a shot is tested.
    fn shot_hits(&mut self, x: usize) -> Option<usize> {
        let maxslot = self.rom.cpu_byte(0x3E, 0x3F, 0xFABE + self.weapon as u16) as usize;
        for y in (1..=maxslot).rev() {
            let s = self.o[y];
            if s.ty == 0 || s.flags & 0x80 == 0 || s.ty == 3 {
                continue;
            }
            if self.o[x].flags & 0x80 == 0 || self.o[x].flags & 4 != 0 {
                continue;
            }
            let hb2 = (s.hitbox as u16) * 2;
            let z = (self.o[x].hitbox & 0x3F) as u16;
            let e6 = self.rom.cpu_byte(0x3A, 0x3B, 0xFACE + hb2).wrapping_add(self.rom.cpu_byte(0x3A, 0x3B, 0xFADC + z));
            let e7 = self.rom.cpu_byte(0x3A, 0x3B, 0xFACF + hb2).wrapping_add(self.rom.cpu_byte(0x3A, 0x3B, 0xFB1C + z));
            if std::env::var("MM4_DBGSHOT").is_ok() {
                eprintln!("shot_hits x={} y={} shot={:?} e6={:02X} e7={:02X}", x, y, s, e6, e7);
            }
            self.rng[2] = e6;
            self.rng[3] = e7;
            let e = self.o[x];
            let me = (s.xpg as u16) << 8 | s.xpx as u16;
            let you = (e.xpg as u16) << 8 | e.xpx as u16;
            let mut dx = s.xpx.wrapping_sub(e.xpx);
            if me < you {
                dx = (!dx).wrapping_add(1);
            }
            if dx >= e7 {
                continue;
            }
            let mut dy = s.ypx.wrapping_sub(e.ypx);
            if s.ypx < e.ypx {
                dy = (!dy).wrapping_add(1);
            }
            if dy < e6 {
                return Some(y);
            }
        }
        None
    }

    /// code_3A8097 for the buster (weapon 0), run for every object in slots 5..23 after its AI: shot collision and damage, then the
    /// contact test against Mega Man.
    fn weapon_phase(&mut self, x: usize, hook: &mut dyn FnMut(&mut World, u8, usize)) {
        if self.weapon == 4 {
            // the Rain Flush (code_3A80DB): while the rain falls every enemy takes one hit and is then immune until the rain ends
            if self.w13a != 0 && self.w145 < 0x10 && self.o[x].t5b8 == 0 {
                self.f6 = 0x24;
                let dmg = self.rom.cpu_byte(0x3A, 0x24, 0xB700 + self.o[x].ty as u16);
                if dmg != 0 {
                    self.apply_damage(x, dmg);
                    self.o[x].t5b8 = self.w13a;
                }
            }
            self.contact_phase(x, hook);
            return;
        }
        if self.weapon == 0x0C && self.w13c != 0 && self.o[x].flags & 4 == 0 {
            // the Flash Stopper (code_3A80AB): every enemy it can hurt is frozen (hit timer `$82`) for as long as the time stop runs
            self.f6 = 0x2C;
            let dmg = self.rom.cpu_byte(0x3A, 0x2C, 0xB700 + self.o[x].ty as u16);
            let t = self.o[x].t5b8;
            if dmg != 0 && (t == 0 || t == 0x81) {
                self.o[x].t5b8 = 0x82;
            }
        }
        if let Some(y) = self.shot_hits(x) {
            let bank = if self.weapon == 0x0C { 0x20 } else { 0x20 | self.weapon };
            self.f6 = bank; // the weapon's bank is switched in (this also decides the carry the next AI dispatch sees)
            let ty = self.o[x].ty as u16;
            let dmg = self.rom.cpu_byte(0x3A, bank as usize, 0xB700 + ty);
            let vulnerable = self.o[x].hitbox & 0x40 != 0;
            if self.weapon == 0x0D {
                // the skull barrier (code_3A811B): a harmful enemy shot is wiped out together with the barrier
                if (self.o[x].hitbox & 0x40) | dmg == 0 {
                    if self.o[x].hitbox & 0x80 != 0 {
                        self.kill_slot(y);
                        return;
                    }
                } else if dmg & 0x80 != 0 {
                    self.despawn(x);
                    self.kill_slot(y);
                    return;
                }
            }
            if vulnerable || dmg != 0 {
                if dmg != 0 && vulnerable {
                    self.damage_enemy(x, y, dmg);
                } else {
                    self.reflect_shot(y);
                }
            }
        }
        self.contact_phase(x, hook);
    }

    /// code_3A81B2: an object that hurts on contact touches Mega Man.
    fn contact_phase(&mut self, x: usize, hook: &mut dyn FnMut(&mut World, u8, usize)) {
        if self.o[x].hitbox & 0x80 != 0 && self.p.state < 6 && !self.invincible && self.touches_player(x) {
            self.contacts.push(x);
            let dmg = self.ai_byte(0x8413 + self.o[x].ty as u16);
            if dmg != 0 && self.p.anim != 0x27 && self.hit.is_none() {
                self.hit = Some((dmg, x));
                self.p.state = 6; // later objects see him hurt
                hook(self, dmg, x); // Mega Man takes the damage right now: what he creates can still run in this frame's loop
            }
        }
    }

    /// code_3A8162: the shot bounces off (armoured enemy or no damage entry): it turns into a type-3 shot flying back and up.
    fn reflect_shot(&mut self, y: usize) {
        self.snd(0x2B, 0x8167); // code_3A8164
        if self.weapon == 8 || matches!(self.o[y].ty, 0x0B | 0x3B) {
            return;
        }
        self.kill_slot(y);
        if self.weapon == 0x0D {
            return;
        }
        let s = &mut self.o[y];
        s.ty = 3;
        s.dir = (s.dir ^ 3) | 8;
        s.xvs = 0;
        s.yvs = 0;
        s.xvp = 4;
        s.yvp = 4;
        if matches!(s.aid, 0x20 | 0x23) {
            s.aid = 0x22;
            s.aframe = 0;
            s.atick = 0;
        }
    }

    /// code_3A82A4: apply the damage of shot `y` to enemy `x`; at 0 hit points the enemy becomes an explosion (or its replacement type).
    fn damage_enemy(&mut self, x: usize, y: usize, dmg: u8) {
        let mut d = dmg;
        if self.o[y].aid == 0x4A {
            // the dust ball bursts one frame later (its animation frame jumps to 1); a ball that already burst does no damage
            if self.o[y].aframe != 0 {
                return;
            }
            self.o[y].aframe = 1;
            return self.apply_damage(x, d);
        }
        if self.weapon == 8 {
            if self.o[x].t5b8 != 0 {
                return;
            }
            return self.apply_damage(x, d);
        }
        if self.weapon == 9 {
            if self.o[y].aid != 0x62 {
                // the missile turns into its explosion on the enemy
                self.child_here(x, y, 0x62);
                self.kill_slot(y);
                let s = &mut self.o[y];
                s.ty = 0x3B;
                s.s[0] = 0;
                s.s[1] = 0;
                s.hitbox = 3;
            }
            if self.o[x].t5b8 != 0 {
                return;
            }
            return self.apply_damage(x, d);
        }
        let a = self.o[y].aid;
        let mut kill_shot = true;
        if matches!(a, 0x20 | 0x23 | 0x49) {
            d = d.wrapping_add(d.wrapping_shl(1)); // x3 (ASL + ADC)
            if self.o[x].hp < d {
                kill_shot = false;
            }
        }
        if kill_shot {
            self.kill_slot(y);
        }
        self.apply_damage(x, d);
    }

    /// code_3A830F: subtract `d` hit points from enemy `x` (boss meter, flash timer, death).
    fn apply_damage(&mut self, x: usize, d: u8) {
        self.snd(0x26, 0x8314); // code_3A8311
        let is_boss = x == self.boss_slot as usize && self.meter2 & 0x80 != 0;
        if is_boss {
            // code_3A8314: the boss cannot be hurt again while its flash timer runs
            let t = self.o[x].t5b8;
            if t != 0 && (t & 0x80 == 0 || t >= 0x83) {
                return;
            }
        }
        let e = &mut self.o[x];
        let (hp, borrow) = e.hp.overflowing_sub(d);
        e.hp = if borrow { 0 } else { hp };
        e.t5b8 = (e.t5b8 & 0x80) | 0x08;
        if is_boss {
            // code_3A835A: the boss meter shows its hit points; the flash lasts $30 frames
            self.bf = e.hp;
            e.t5b8 = (e.t5b8 & 0x80) | 0x30;
            if e.ty == 0x9E {
                self.bf = e.hp.wrapping_add(0x0E);
            }
        }
        if e.hp != 0 {
            return;
        }
        // code_3A8382: dead
        e.t5b8 = 0;
        let ty = e.ty as u16;
        self.despawn(x);
        self.o[x].flags &= 0xFD;
        let repl = self.ai_byte(0x84E3 + ty);
        if repl != 0 {
            let e = &mut self.o[x];
            e.ty = repl;
            e.hitbox = 0;
            e.s = [0; 8];
        } else {
            self.set_anim(x, 0x19);
            let e = &mut self.o[x];
            e.hitbox = 0;
            e.ty = 0x9D;
            self.snd(0x27, 0x83D2); // code_3A83CF
        }
    }

    fn unported_note(&mut self, addr: u16) {
        if !self.unported.contains(&addr) {
            self.unported.push(addr);
        }
    }

    /// AI $2E ($3DA8C3): a death particle flies in a straight line (16-bit signed speeds) and vanishes when it leaves screen 0.
    fn ai_a8c3(&mut self, x: usize) {
        let o = &mut self.o[x];
        let sx = if o.xvp & 0x80 != 0 { 0xFFu8 } else { 0 };
        let (s, c1) = o.xvs.overflowing_add(o.xsub);
        o.xsub = s;
        let (p, c2) = o.xvp.overflowing_add(o.xpx);
        let (p, c3) = p.overflowing_add(c1 as u8);
        o.xpx = p;
        o.xpg = o.xpg.wrapping_add(sx).wrapping_add((c2 || c3) as u8);
        let sy = if o.yvp & 0x80 != 0 { 0xFFu8 } else { 0 };
        let (s, c1) = o.yvs.overflowing_add(o.ysub);
        o.ysub = s;
        let (p, c2) = o.yvp.overflowing_add(o.ypx);
        let (p, c3) = p.overflowing_add(c1 as u8);
        o.ypx = p;
        o.ypg = o.ypg.wrapping_add(sy).wrapping_add((c2 || c3) as u8);
        if o.ypg != 0 {
            self.despawn(x);
        }
    }

    /// AI `$9D` ($3BBB6C): the explosion. On its fourth frame it may leave an item behind.
    fn ai_bb6c(&mut self, x: usize) {
        if self.o[x].aframe != 4 {
            return;
        }
        if self.meter2 & 0x80 != 0 {
            // $0132 bit 7 (the boss meter is up): no item drops, the explosion just ends
            self.despawn(x);
            return;
        }
        // CMP #$04 left the carry set
        let (a, c1) = {
            let s = self.rng[2] as u16 + self.rng[3] as u16 + 1;
            (s as u8, s > 0xFF)
        };
        let (a, c2) = {
            let d = a as i16 - self.p95 as i16 - !c1 as i16;
            (d as u8, d >= 0)
        };
        self.rng[2] = a;
        let (a, c3) = {
            let d = a as i16 - self.rng[1] as i16 - !c2 as i16;
            (d as u8, d >= 0)
        };
        let e7 = (a as u16 + self.frame92 as u16 + c3 as u16) as u8;
        self.rng[3] = e7;
        let rem = e7 % 50;
        let mut pick = None;
        for k in (0..=4usize).rev() {
            if rem < self.ai_byte(0xBC8D + k as u16) {
                pick = Some(k);
                break;
            }
        }
        match pick {
            None => self.despawn(x),
            Some(k) => {
                let (anim, hb, probe) = (self.ai_byte(0xBC92 + k as u16), self.ai_byte(0xBC97 + k as u16), self.ai_byte(0xBC9C + k as u16));
                self.set_anim(x, anim);
                let o = &mut self.o[x];
                o.hitbox = hb;
                o.s[0] = probe;
                o.s[1] = 0xFF;
                self.land_speed(x);
                self.o[x].ptr = 0xBBC3;
            }
        }
    }

    /// code_3BBBDE: the effect of an item, chosen by its animation id.
    fn pickup_effect(&mut self, aid: u8) {
        self.pickup = match aid {
            0x1A | 0x1B => Some(Pickup::Life(self.ai_byte(0xBC87 + aid as u16))),
            0x1C | 0x1D => if self.weapon == 0 { None } else { Some(Pickup::Weapon(self.ai_byte(0xBC87 + aid as u16))) },
            0x1E => {
                self.snd(0x1F, 0xBBF6); // code_3BBBF3
                Some(Pickup::OneUp)
            }
            0x1F => {
                self.snd(0x1F, 0xBC06); // code_3BBC03
                Some(Pickup::ETank)
            }
            _ => {
                self.unported_note(0xBBDE);
                None
            }
        };
        self.life_new = matches!(self.pickup, Some(Pickup::Life(_) | Pickup::Weapon(_)));
    }

    /// AI `$C1` ($3BBC4A): an item placed in the stage; touching it applies the effect and marks the placement entry as taken.
    fn ai_bc4a(&mut self, x: usize) {
        if !self.touches_player(x) {
            return;
        }
        if self.o[x].ty != 0xC2 {
            // code_3FFC9E
            let i = self.o[x].idx as usize;
            let mask = self.rom.cpu_byte(0x3E, 0x3F, 0xFCBF + (i & 7) as u16);
            if i < 256 {
                self.defeated[(i >> 3) & 31] |= mask;
            }
        }
        let aid = self.o[x].aid;
        self.despawn(x);
        if aid < 0x51 {
            self.pickup_effect(aid);
        } else {
            // code_3BBC66: $0064 + animation id = $9C (weapon slot $B0 + n), sound, $0148 = $FF, state $11
            self.pickup = Some(Pickup::Capsule((0x64u8.wrapping_add(aid)).wrapping_sub(0xB0)));
            self.snd_music(0x3A, 0xBC70); // code_3BBC6D
        }
    }

    /// AI item ($3BBBC3): falls, blinks out after a while, is picked up on contact (pickup effects are not ported: the item just vanishes).
    fn ai_bbc3(&mut self, st: &Stage, x: usize) {
        let id = self.o[x].s[0] as usize;
        self.vertical_collide(st, x, id);
        if self.touches_player(x) {
            // code_3BBBDE: the item vanishes and its effect (chosen by animation id) is applied
            let aid = self.o[x].aid;
            self.despawn(x);
            self.pickup_effect(aid);
            return;
        }
        let t = self.o[x].s[1];
        if t < 0x3C {
            self.o[x].t5b8 = t;
        }
        self.o[x].s[1] = t.wrapping_sub(1);
        if self.o[x].s[1] == 0 {
            self.despawn(x);
        }
    }

    /// AI $6D ($3BAD70): leaping fish. Swims toward Mega Man; when its first two probe points touch ground it stops.
    fn ai_ad70(&mut self, st: &Stage, x: usize) {
        if self.o[x].s[0] == 0 {
            self.o[x].s[0] = 8;
            let h = self.aim(x) as u16;
            let (a, b, c, d) = (self.ai_byte(0x90AA + h), self.ai_byte(0x90BA + h), self.ai_byte(0x90CA + h), self.ai_byte(0x90DA + h));
            let o = &mut self.o[x];
            o.xvs = a;
            o.xvp = b;
            o.yvs = c;
            o.yvp = d;
            return;
        }
        self.o[x].s[0] -= 1;
        self.move_v(x);
        let (types, _) = self.vprobe(st, x, 0x28);
        if types[0] & 1 != 0 && types[1] & 1 != 0 {
            let o = &mut self.o[x];
            o.ptr = 0xADC3;
            o.yvs = 0;
            o.yvp = 0;
            return;
        }
        self.move_h(x);
    }

    /// AI $6D landing ($3BADC3).
    fn ai_adc3(&mut self, st: &Stage, x: usize) {
        if self.o[x].aid == 0x9A {
            if self.vertical_collide(st, x, 0x29) {
                self.set_anim(x, 0x9B);
            }
            return;
        }
        if self.o[x].aframe != 6 {
            return;
        }
        self.o[x].ptr = 0xADF1;
        self.o[x].s[1] = 0x1F;
        self.set_anim(x, 0x9C);
    }

    /// AI $6D shooting ($3BADF1): turn toward Mega Man, wait, spit a bullet (type $6E), repeat.
    fn ai_adf1(&mut self, x: usize) {
        self.face_player(x);
        self.face_flag(x);
        if self.o[x].aid == 0x9C {
            self.o[x].s[1] = self.o[x].s[1].wrapping_sub(1);
            if self.o[x].s[1] != 0 {
                return;
            }
            self.set_anim(x, 0x9D);
            return;
        }
        match self.o[x].aframe {
            3 => {
                self.o[x].aframe += 1;
                let Some(y) = self.find_slot() else { return };
                let dir = self.o[x].dir;
                let c = &mut self.o[y];
                c.ty = 0x6E;
                c.hitbox = 0x8F;
                c.xvs = 0;
                c.xvp = 2;
                c.dir = dir;
                self.child_at(x, y, (dir & 1) + 0x33, 0x18);
            }
            8 => {
                self.o[x].s[1] = 0x50;
                self.set_anim(x, 0x9C);
            }
            _ => {}
        }
    }

    // ---- boss-room object $B8 (bank $35): drops in, powers the boss meter up and turns into the stage's boss -------------------

    /// code_3FFD9C for a robot-master stage: Mega Man is frozen in state 9 (only from the ground, state 0). Returns true when he is in state 9.
    fn freeze_player(&mut self, _st: &Stage, x: usize) -> bool {
        self.boss_music();
        match self.p.state {
            9 => true,
            0 => {
                self.player_cmd = Some(9);
                self.boss_slot = x as u8;
                true
            }
            _ => false,
        }
    }

    /// code_3FF2A7: move by the vertical speed (positive = up), then gravity, no collision.
    fn move_gravity(&mut self, x: usize) {
        if self.o[x].yvp & 0x80 != 0 {
            self.fall_move(x);
        } else {
            self.up(x);
        }
        self.gravity_step(x);
    }

    /// code_35A89F: index into the boss tables `$A8AC..`: the stage, or in stages 8-14 the robot master picked by the pod (`$A8D4[$AB]`).
    fn boss_idx(&self, st: &Stage) -> u16 {
        if st.id < 8 {
            st.id as u16
        } else {
            self.ai_byte(0xA8D4 + self.ab as u16) as u16
        }
    }

    /// AI $B8 first state ($35A7F9): wait for Mega Man to stand still (state 9), fall in from above the room, land and start the power-up.
    fn ai_a7f9(&mut self, st: &Stage, x: usize) {
        let y = self.boss_idx(st); // code_35A89F
        if !self.freeze_player(st, x) {
            return;
        }
        self.o[x].atick = 0;
        let anim = self.ai_byte(0xA8AC + y);
        if self.o[x].aid != anim {
            self.set_anim(x, anim);
        }
        if self.o[x].ypx < 0x80 {
            self.move_gravity(x);
            return;
        }
        let probe = self.ai_byte(0xA8B4 + y) as usize;
        if !self.vertical_collide(st, x, probe) {
            return;
        }
        self.o[x].ptr = 0xA84F;
        self.p95 = 0;
        self.bf = 0;
        self.meter2 = 0x8F;
        self.ai_a84f(st, x);
    }

    /// Second state ($35A84F): on animation frame 5 every 8th frame add one unit to the boss meter; when full the object becomes the boss.
    fn ai_a84f(&mut self, st: &Stage, x: usize) {
        if self.o[x].aframe != 5 {
            return;
        }
        self.o[x].atick = 0;
        if self.p95 & 7 != 0 {
            return;
        }
        self.snd(0x29, 0xA866); // code_35A863
        self.bf = self.bf.wrapping_add(1);
        if self.bf != 0x1C {
            return;
        }
        let y = self.boss_idx(st);
        self.despawn(x);
        let ty = self.ai_byte(0xA8BC + y);
        let anim = self.ai_byte(0xA8C4 + y);
        let hitbox = self.ai_byte(0xA8CC + y);
        let xvs = self.ai_byte(0xA8DE + y);
        let xvp = self.ai_byte(0xA8E6 + y);
        self.o[x].ty = ty;
        self.set_anim(x, anim);
        let o = &mut self.o[x];
        o.hitbox = hitbox;
        o.xvs = xvs;
        o.xvp = xvp;
        o.hp = 0x1C;
        self.land_speed(x);
        self.player_cmd = Some(0);
    }

    // ---- the spawner (bank $38, $8000) ------------------------------------------------------------------------------------

    /// Walk the placement list as the camera moves and create the objects that enter the 256-pixel window.
    /// `dir` is `$2F` (1 = the camera last moved right, otherwise left).
    pub fn spawn_phase(&mut self, st: &Stage, cam: Cam, dir: u8) {
        let t = |i: usize, base: usize| st.byte(base + i);
        let c0 = cam.lo;
        let c1 = cam.hi;
        let (c2, carry) = c0.overflowing_add(0xFF);
        let c3 = c1.wrapping_add(carry as u8);
        if dir & 1 == 0 {
            // moving left: the left cursor walks back over entries that re-enter at the left edge
            loop {
                let y = self.c9f as usize;
                if y == 0 {
                    break;
                }
                let pg = t(y - 1, 0xB100);
                let px = t(y - 1, 0xB180);
                if pg < c1 || (pg == c1 && px < c0) {
                    break;
                }
                self.spawn_entry(st, y - 1);
                self.c9f = self.c9f.wrapping_sub(1);
                if self.c9f == 0 {
                    break;
                }
            }
            let mut y = self.c9e as usize;
            while y != 0 {
                let pg = t(y - 1, 0xB100);
                let px = t(y - 1, 0xB180);
                if pg < c3 || (pg == c3 && px < c2) {
                    break;
                }
                y -= 1;
            }
            self.c9e = y as u8;
            return;
        }
        loop {
            let y = self.c9e as usize;
            let pg = t(y, 0xB100);
            let px = t(y, 0xB180);
            if c3 < pg || (c3 == pg && c2 < px) {
                break;
            }
            self.spawn_entry(st, y);
            self.c9e = self.c9e.wrapping_add(1);
            if self.c9e == 0 {
                break;
            }
        }
        let mut y = self.c9f as usize;
        loop {
            let pg = t(y, 0xB100);
            let px = t(y, 0xB180);
            if c1 < pg || (c1 == pg && c0 < px) {
                break;
            }
            y += 1;
            if y == 256 {
                y = 0;
                break;
            }
        }
        self.c9f = y as u8;
    }

    /// code_388080: create the object of placement entry `i` (or run its palette/CHR command, which does not matter here).
    fn spawn_entry(&mut self, st: &Stage, i: usize) {
        let ty = st.byte(0xB280 + i);
        if ty >= 0xC0 {
            self.pal.command(&self.rom, ty);
            return;
        }
        if (8..SLOTS).any(|s| self.o[s].idx == i as u8) {
            return;
        }
        let Some(x) = self.find_slot() else { return };
        let mask = self.rom.cpu_byte(0x3E, 0x3F, 0xFCBF + (i & 7) as u16);
        if self.defeated[i >> 3] & mask != 0 {
            return;
        }
        let tt = ty as u16;
        let speed_idx = self.t38(0x844F + tt) as u16;
        let anim = self.t38(0x832F + tt);
        let o = &mut self.o[x];
        o.idx = i as u8;
        o.xpg = st.byte(0xB100 + i);
        o.xpx = st.byte(0xB180 + i);
        o.ypx = st.byte(0xB200 + i);
        o.flags = self.rom.cpu_byte(0x38, 0x39, 0x817F + tt);
        o.ty = self.rom.cpu_byte(0x38, 0x39, 0x820F + tt);
        o.hitbox = self.rom.cpu_byte(0x38, 0x39, 0x829F + tt);
        o.hp = self.rom.cpu_byte(0x38, 0x39, 0x83BF + tt);
        o.xvs = self.rom.cpu_byte(0x38, 0x39, 0x84DF + speed_idx);
        o.xvp = self.rom.cpu_byte(0x38, 0x39, 0x84E6 + speed_idx);
        self.set_anim(x, anim);
        self.face_player(x);
        let o = &mut self.o[x];
        o.yvs = 0;
        o.yvp = 0xFF; // code_3FF324
        o.ypg = 0;
        o.xsub = 0;
        o.ysub = 0;
        o.ptr = 0;
        o.s = [0; 8];
    }

    // ---- the draw pass: despawn and animation (code_3EDB3A) -----------------------------------------------------------------

    /// Despawn off-screen objects and tick animations. Parity of `$95` only fixes the visiting order, which does not change the result.
    pub fn draw_phase(&mut self, cam: Cam) {
        self.rain_draw();
        self.p95 = self.p95.wrapping_add(1);
        self.drawn.clear();
        for x in 1..SLOTS {
            if self.o[x].ty != 0 {
                if let Some((x13, y12)) = self.finish_object(x, cam) {
                    self.drawn.push((x, x13, y12));
                }
            }
        }
    }

    /// code_3EDF5F (first thing of every draw pass): while `$013A` counts down, 30 rain sprites are placed at the start of the OAM.
    fn rain_draw(&mut self) {
        self.rain.clear();
        if self.w13a == 0 {
            return;
        }
        self.w13a -= 1;
        if self.w13a & 7 == 0 {
            self.snd(0x18, 0xDF73); // code_3EDF70
        }
        let (tile, attr) = if self.w145 < 0x10 { (0x99, 0) } else { (0xFF, 3) };
        for k in 0..30u16 {
            let (y0, x0) = (self.rom.cpu_byte(0x3E, 0x3F, 0xDFBE + 2 * k), self.rom.cpu_byte(0x3E, 0x3F, 0xDFBF + 2 * k));
            self.rain.push((y0.wrapping_add(self.w13b), tile, attr, x0.wrapping_sub(self.w13b)));
        }
        self.w13b = self.w13b.wrapping_add(4);
    }

    /// The animation table (and so the sprite data) of object type `ty`.
    pub fn sprite_table(&mut self, ty: u8) -> &AnimTable {
        self.anim_table(ty)
    }

    /// The AI loop sees Mega Man at a new X after an object pushed him.
    pub fn set_player_x(&mut self, pg: u8, px: u8) {
        self.p.xpg = pg;
        self.p.xpx = px;
    }

    /// ... and at a new Y.
    pub fn set_player_y(&mut self, pg: u8, px: u8) {
        self.p.ypg = pg;
        self.p.ypx = px;
    }

    /// Slots 1..=4 (Mega Man's shots and weapon objects) disappear when the pause menu closes (code_3C9840).
    pub fn clear_weapon_slots(&mut self) {
        for s in 1..=4 {
            self.despawn(s);
        }
    }

    /// code_3EDC00: remove an object.
    fn despawn(&mut self, x: usize) {
        let o = &mut self.o[x];
        o.ptr &= 0x00FF; // $05A0 = 0 (the low byte $0588 stays)
        o.ty = 0;
        o.t5b8 = 0;
        o.hp = 0;
        o.idx = 0xFF;
    }

    /// code_3EDB72: visibility test and animation tick for one object.
    fn finish_object(&mut self, x: usize, cam: Cam) -> Option<(u8, u8)> {
        let o = self.o[x];
        let mut x13 = o.xpx;
        let y12 = o.ypx;
        if o.flags & 0x10 == 0 {
            self.o[x].flags |= 0x80;
            if self.o[x].flags & 4 != 0 {
                return None;
            }
        } else {
            let (d13, b1) = o.xpx.overflowing_sub(cam.lo);
            let (t01, b2) = o.xpg.overflowing_sub(cam.hi);
            let (t01, b3) = t01.overflowing_sub(b1 as u8);
            let carry = !(b2 || b3); // C after SBC $F9
            x13 = d13;
            let mut t00 = d13;
            // outcome: 0 = visible (draw), 1 = keep without drawing, 2 = candidate for despawn
            let outcome;
            if t01 == 0 {
                if o.ypg == 0 {
                    outcome = 0;
                } else if o.ypg & 0x80 != 0 {
                    outcome = 1;
                } else {
                    outcome = 2;
                }
            } else if o.flags & 8 == 0 {
                outcome = 2;
            } else {
                let mut c = carry;
                if !carry {
                    t00 = (!d13).wrapping_add(1);
                    c = d13 == 0;
                }
                if o.ty < 0x20 || o.ty >= 0x24 {
                    outcome = if t00 >= 0x30 { 2 } else { 1 };
                } else {
                    let (a, _) = if t01 & 0x80 != 0 { (!t01 as u16 + 1 + c as u16, 0) } else { (t01 as u16, 0) };
                    let a = a as u8;
                    outcome = if a < 2 || o.dir != 0 { 1 } else { 2 };
                }
            }
            match outcome {
                0 => {
                    self.o[x].flags |= 0x80;
                    if self.o[x].flags & 4 != 0 {
                        return None;
                    }
                }
                1 => {
                    self.o[x].flags &= 0x7F;
                }
                _ => {
                    let keep = matches!(o.aid, 0x55 | 0x71) || matches!(o.ty, 0x26 | 0x96 | 0x99 | 0xBB | 0xBC | 0x4D);
                    if keep {
                        self.o[x].flags &= 0x7F;
                    } else {
                        self.despawn(x);
                        return None;
                    }
                }
            }
        }
        // animation tick (code_3EDC67)
        let o = self.o[x];
        if o.aid == 0 {
            return None;
        }
        let (last, dur) = self.anim_table(o.ty).header(o.aid);
        let mut t5b8 = o.t5b8;
        let mut atick = o.atick;
        let mut aframe = o.aframe;
        if t5b8 != 0 {
            t5b8 = t5b8.wrapping_sub(1);
            if t5b8 & 0x7F == 0 {
                t5b8 = 0;
            }
        }
        if self.hold_anim {
            t5b8 = o.t5b8;
        } else if t5b8 & 0x80 == 0 {
            let t = atick & 0x7F;
            atick = atick.wrapping_add(1);
            if t == dur {
                atick &= 0x80;
                let old = aframe & 0x7F;
                aframe = aframe.wrapping_add(1);
                if old == last {
                    aframe = 0;
                }
            }
        }
        {
            let o = &mut self.o[x];
            o.t5b8 = t5b8;
            o.atick = atick;
            o.aframe = aframe;
        }
        if self.o[x].flags & 0x80 == 0 {
            return None;
        }
        let t5 = self.o[x].t5b8;
        let is_boss = x == self.boss_slot as usize && self.meter2 & 0x80 != 0; // code_3EDCE7: the boss flashes its palette instead of blinking
        if t5 != 0 && t5 & 0x80 == 0 && !is_boss {
            // types $24/$2B/$52 flash their palette instead of blinking (code_3EDD49-57): drawn as usual, the palette change is not ported
            if !matches!(self.o[x].ty, 0x24 | 0x2B | 0x52) && (t5 >> 1) & 1 != 0 {
                return None; // blink: not drawn this frame, no end check
            }
        }
        let (aid, af) = (self.o[x].aid, self.o[x].aframe & 0x7F);
        if self.anim_table(self.o[x].ty).sprite(aid, af) == 0 {
            let o = &mut self.o[x];
            o.ptr &= 0x00FF;
            o.ty = 0;
            o.idx = 0xFF;
            return None;
        }
        Some((x13, y12))
    }
}

impl World {
    /// code_3ED052 (the vertical scroll's row builder): the 8 blocks of the row `$24` in page `$2A`, then the first of them in the next page.
    pub fn row_scribble(&mut self, st: &Stage, page: u8, c24: u8, vertical: bool) {
        if st.id != 4 {
            return;
        }
        let base = (c24 & 0x1C) << 1;
        let layout = st.byte(0xB500 + page as usize) as usize;
        for k in 0..8u8 {
            self.block_scribble(st, layout, base + k, page, vertical);
        }
        let next = page.wrapping_add(1);
        let layout = st.byte(0xB500 + next as usize) as usize;
        self.block_scribble(st, layout, base, next, vertical);
    }
}

impl World {
    /// Queue a PPU buffer record (the NMI uploads it; the port applies it to the nametable at the end of the logic frame).
    pub fn ppu_put(&mut self, addr: u16, bytes: &[u8]) {
        self.ppu_rec.push((addr, bytes.to_vec()));
    }

    /// A whole PPU buffer as the game builds it in `$0780..` (records `hi lo count-1 data...`, ended by a byte with bit 7 set).
    pub fn ppu_buffer(&mut self, buf: &[u8]) {
        let mut i = 0;
        while i + 2 < buf.len() && buf[i] & 0x80 == 0 {
            let n = buf[i + 2] as usize + 1;
            let end = (i + 3 + n).min(buf.len());
            self.ppu_put((buf[i] as u16) << 8 | buf[i + 1] as u16, &buf[i + 3..end]);
            i += 3 + n;
        }
    }
}

impl World {
    /// The column-renderer steps of the NMIs that came during this logic frame (see `col_due`).
    pub fn apply_col_steps(&mut self) {
        if std::mem::take(&mut self.col_block) {
            self.col_due = self.col_due.saturating_sub(1);
        }
        while self.col_due > 0 {
            self.col_due -= 1;
            self.column_step();
        }
    }
}

impl World {
    /// `JSR code_3FF6BE`: queue a sound effect (`ret` = return address of the call in the ROM).
    pub fn snd(&mut self, id: u8, ret: u16) {
        self.snd.push(crate::sound::Req { music: false, id, ret });
    }

    /// `JSR code_3FF6BC`: a music / stop request (`$D9` = id, then the same queue).
    pub fn snd_music(&mut self, id: u8, ret: u16) {
        self.d9 = id;
        self.snd.push(crate::sound::Req { music: true, id, ret });
    }
}

impl World {
    /// The start of code_3FFD9C: the boss music (`$11`, or `$45` in stage 15) unless it is the one playing already.
    pub(crate) fn boss_music(&mut self) {
        let (id, ret) = if self.st_id == 0x0F { (0x45, 0xFDAB) } else { (0x11, 0xFDB7) };
        if self.d9 != id {
            self.snd_music(id, ret);
        }
    }
}
