# Next session plan: presentation and game flow (all stages and all weapons are done)

Paste this to start a new chat: **"Read NEXT-SESSION.md, STATUS.md and PLAN.md in the MegaMan4 Project folder and continue with the plan."**

## Where we are
- Stages 0-14 (eight robot masters, Cossack/Wily stages 8-13 and the Wily castle with the boss rush and the final Wily machine) are ported and frame-exact.
- **Session 20: every special weapon (0-13: buster, Rush Coil/Marine/Jet, Rain Flush, wire, spring platform, Dive/Ring/Drill/Dust/Pharaoh/Flash/Skull) and the pause menu logic are ported and frame-exact** (`notes/weapons.md`).
- `harness/regress.sh` (about 30 minutes, `run_in_background`, `sh regress.sh | tail -3` from `harness/`, needs `port/target/release/mm4.exe`; copy the exe to `mm4_reg.exe` and `sed` the script if you want to keep building meanwhile) = all ok; the weapon group alone is `harness/regress_w.sh` (3 minutes).
- Build: `export PATH="$PATH:/c/Users/joele/.cargo/bin"; cd port; cargo build --release` (builds both crates; `cd port/mm4-core` builds only the library). **Seed numbers are shared by all recordings: never reuse one** (weapons use 91xxx-93xxx).
- Window: `port/play.bat [stage]`, `port/play_weapons.bat [stage]` (all weapons; Q/E switch weapon; Enter = pause menu with the weapon list; Z jump, X shoot, arrows move). The stage-clear, game-over, stage-select and pause screens are placeholders drawn with a 3x5 pixel font.

## Session 21 progress (autonomous loop)
DONE and verified frame by frame: stage select (`select_screen.rs`), password screen / stage-clear menu / game over (`password_screen.rs`); check with `sh harness/regress_screens.sh`. Details: STATUS.md Session 21, `notes/screens.md`.
DONE too: weapon-get (`weapon_get.rs`), boss intro (`boss_intro.rs`).
DONE too: boot screens, title, password entry, fortress maps, Wily flight (`boot_screens.rs`, `title_screen.rs`, `password_entry.rs`, `fortress.rs`, `wily_flight.rs`).
NOT DONE: the ending (bank `$1B` `$8003`, ~7900 frames: Wily castle in the snow, credits on the train; recording `harness/pfe_1.txt`, hijack `PF_STATE=19`), the opening story (bank `$1B` `$8E36`, scene `$13`, ~7000 frames, `harness/pft_1.txt`).
NEXT in order: (old list follows, first items now done) opening story (bank `$1B`) (CAPCOM, title screen), password entry/decode, ending, Cossack/Wily map (scene `$12`, `$AA`), then game flow in the window.

## What is left (nothing here is verified against the game yet unless said)
1. **Presentation, the big one (the pause screen is done, see STATUS Session 20)**: a small PPU model in Rust (nametables + palettes + the game's NMI buffer format `[addr hi, addr lo, count-1, data...]`, `$FF` ends, `code_3EC330`; CHR bank sets; sprite OAM from objects) so the *original* menus can be drawn from the ROM instead of placeholders: title screen, stage select, confirm menu, pause menu (the pause code copies tables like `$9F28` into `$0780`), weapon-get screen, game over, password, ending. The menu *logic* of pause/confirm/select is already verified; what is missing is the drawing and the palette fades. Verify by comparing a PPU screenshot / nametable dump from Mesen against the Rust render at fixed frames.
   Groundwork already there: `harness/ppu_dump.lua` (Mesen script: dumps nametables, palette, 8 KB pattern RAM, OAM and a few registers at chosen frames while the title/menu input of `gen_ptrace.js` runs; also logs `scene.txt` = `$22` per frame: 6-399 title/intro, 0x13, 0x11...), `node tools/render_dump.js ppudump_<frame>.txt out.png` (renders such a dump), `gfx::load_set` (the graphics loader `$3EDA05`: sets 0x10-0x1F/0x30+ are the menu screens' tiles), and `Stage::load(rom, 0x10..0x13)` already renders the menu scenes' backgrounds (scene `$10` screen 0 = the stage-select portraits, screen 1 = CAPCOM PRESENTS; `--stage 16 --ppm out.ppm`). The streamed variant of the graphics loader (`$3EDA8B`/`$3EDABB`, 3 tiles per frame) is modelled only as a frame count (`Game::gfx_chunk_frames`).
   **Stage select, started**: `harness/select_dump.lua` wrote `harness/seldump_572/590/606/622/636/650.txt` (+ `.png` screenshots): its nametable is **exactly** `Stage::load(rom, 0x10, 5).nametable(0)` (0 tile / 0 attribute differences); what is still to port is the sprite list (`$98F9` template, 36 sprites, tile bytes patched per defeated boss by `code_399886` using `$B4..$BD`, `$99AF/$99BA` tables), the palette, the graphics sets (`$12`/`$13` in `code_399A28`/`code_399AEF`, `$22|$18` per stage on leaving), the blinking frame/cursor (`code_39809D`..: cursor `$10/$11`, already verified as logic in `select.rs`) and the "PRESS START" blink. Frame 600 of the title/menu input = the stage select.
2. **Game flow**: title -> (password) -> stage select -> stage -> weapon get -> back; game over/continue (the logic is `code_398EF7`: lives reset to 2, confirm menu, then stage select for stages < 8), the ending after the Wily machine (stage 14 stops at the stage clear), password encode/decode.
3. **HUD/presentation details**: palette fades, text, the stage intro, "READY" blinking.
4. **Audio** (last): the sound engine in bank `$3x` is plain 6502 code writing APU registers; port the music/sfx data tables and the sequencer, output through a small mixer.
5. Small leftovers: stage-3 pages 25-31 never recorded alone, the stage-4 machine's block-row destruction, the final Wily scene past Mega Man's death in hurt mode, the `DE` placement command in stage 14 (ignored), the weapon-get cutscene.
6. Put the project root under git (only `megaman4-disassembly/` is a repo). Never commit ROMs or ripped assets; never push to the upstream remote.

## Recommended order
Ask Luna what she wants to see next; my recommendation: (1) the same method (Mesen PPU dump -> `Stage::nametable` + `gfx::load_set` + the buffer records -> `Screen::render`, compared byte for byte and pixel for pixel) for stage select (scene `$10` screen 0 + the cursor/portrait sprites), confirm/game-over, title, weapon get; (2) the game flow around them; (3) audio.

## Pitfalls (still valid)
See `NEXT-SESSION.old.md` ("Pitfalls learned", "session 16", "session 17"), STATUS.md "Session 19"/"Session 20" and `notes/weapons.md`:
- `MM4_SECT2B` must be the hex `$2B` of the recording's section.
- A verifier mismatch of rng right after a long scene change can mean more skipped NMIs than tried; extra full frames inside one logic frame (life refill, the Marine's graphics stream) must call `draw_phase` + `rng_step` + `loop_steps += 1` like `World::life_refill` / `Game::step` do.
- The recorder's `BOSSHP` hook runs after every video frame; interrupted recordings leave truncated trace lines (re-record them).
- `sed -i 's/.../' regress.sh` style edits hit every matching line; use `wr_group.txt` + the awk recipe in this session's history (or just edit by hand). `sh` reads a running script incrementally: do not edit a script while a batch uses it.
- Quirks of the real code that were *reproduced on purpose*: weapon 6/11 take the highest free shot slot while the buster takes the lowest; the Flash Stopper's timer only counts while the weapon routine runs; the pause menu would loop forever on an empty column (the port stops after 64 steps).

## Ending (session 21 end)
Verified to frame ~11800 (pfe_5). To finish: record with `PF_HIJACK=720 PF_STATE=19 PF_FROM=11500 PF_FRAMES=15500` (Mesen dies after ~4000 recorded frames, so go in pieces), verify the remaining staff pictures and `last_picture` (train, credits scroll, friends arriving). Then item 5 leftovers (stage-3 pages 25-31, stage-4 machine, final Wily scene in hurt mode, `DE` command).
