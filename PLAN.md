# Mega Man 4 - Editable Source and Rust Port Plan

> **Fresh session? Read `STATUS.md` first** (handoff: current state, next steps, how to run things).

Goal: turn the finished NES disassembly into something we can freely change, then (optionally) port the game logic to Rust.

## FINISH LINE (decided 2026-10-09 with Luna): finish the classic port and release it. Nothing gets added to this list until it ships.
All 15 stages are done (regress 1009 ok / 0 bad). Left, in order:
- [x] 1. Playable loop: title -> stage select -> stage -> weapon get -> back, start to finish, incl. password entry, game over / continue, Cossack and Wily maps and the flight (session 22: `flowrun.rs` = one state machine; 11 chains from power-on checked against Mesen, `harness/regress_chain.sh`)
- [x] 2. Ending (after the final Wily machine) verified to the last picture (session 22), game over / continue done (chains d/e)
- [x] 3. Special weapons (8 robot-master weapons, Rush, wire, balloon/spring platform, weapon menu logic, energy, E-tank use): done in session 20 (regress_w.sh 111 ok); only the menu graphics are open (item 4)
- [~] 4. HUD and effects (energy meters, text, palette fades): menu scenes, stage fades and the READY blink are exact (chains); the gameplay picture is not yet the console's in palette animations, edge column streaming and tile changes (NEXT-SESSION item 1)
- [ ] 5. Audio
- [~] 6. Small leftovers: stage 3 pages 25-31 done, final Wily scene in hurt mode done; open: stage 4 machine row destruction, ignored CHR/palette placement commands (NEXT-SESSION item 2)
- [~] 7. Release: project root under git (done, local only; no ROM, no assets), README (build/run/what frame-exact means), notes/ as docs, demo clip, post to ROM-hacking / Rust / EmuDev communities
Afterwards (version 2, not now): widescreen, dash, playable characters, connected map, editor-built rooms.

Rules for every session
- One small, testable task per session. Finish it, commit it, write notes, stop.
- Notes live in `notes/` so a fresh session can pick up without re-reading code.
- Never read whole bank files (1+ MB). Use grep / line ranges.
- Verify against the emulator (Mesen), not by feel.
- Keep the original ROM and any ripped assets private. Do not publish them.

## Phase 0 - Setup (done)
- [x] Disassembly cloned to `megaman4-disassembly/`
- [x] Builds with `xkas.exe -o mm4.nes assemble.asm`; body matches the owner's ROM byte for byte
- [x] Baseline commit in `megaman4-disassembly/` (it was already a git clone; local commit `30aef87`, `*.nes` now ignored so the ROM stays out of git; do not push to origin)
- [x] Mesen already installed (Input settings: Configure on Port 1 to set WASD)
- [x] Confirm `megaman4-disassembly/mm4.nes` runs in Mesen (confirmed by Luna)

## Phase 1 - Prove the edit loop (1 short session)
- [x] Change one value (starting health or jump height), rebuild, see it in game (starting life 28 -> 16 units at `$3EC687`, confirmed by Luna in Mesen)
- [x] Write `notes/build-and-test.md`

## Phase 2 - Map the engine (3-5 short sessions)
- [ ] RAM map: player X/Y, speed, health, state, current stage, screen scroll -> `notes/ram-map.md` (first draft written: X/Y, speed, health, stage, PPU scroll, controller done; `$22` = scene/stage ID confirmed by Luna in Mesen; still to verify in Mesen: speed arrays; player state byte `$30` and jump/gravity code found (see `notes/physics-traces.md`); `$30` states first mapped in `notes/player-states.md` (00 ground, 01 air, 02 slide, 06 hurt, 0A spawn confirmed; the rest are guesses); camera/scroll variables mapped in `notes/camera.md`; still missing: lives)
- [x] Main loop and game-state machine -> `notes/game-loop.md` (first pass: scheduler, NMI, in-stage frame order done; exit paths, menus, sound still unread)
- [ ] Object/enemy slot system (how enemies are spawned and updated) (spawning, slots and placement tables done: `notes/enemy-placement.md`; still to do: how each object type's AI runs, type names)
- [ ] Level data format (tile maps, screen layout, enemy placement tables) (tile/screen/collision format decoded and validated by rendering stage `$02`: see `notes/level-format.md`; still to do: enemy placement, terrain type meanings, section transitions)
- [ ] Rename labels in the asm as they are understood

## Phase 3 - Level editor (optional, works with or without a Rust port)
- [x] Extract one stage's data into a readable file format (`tools/stage.js export`, JSON; see `tools/README.md`)
- [x] Tool that reads and writes that format back into the ROM (`tools/stage.js import`; edits the ROM, not the asm)
- [x] Round-trip test: export, import, ROM unchanged (all 15 stages byte-identical; an enemy-removal edit also verified in game)

## Phase 4 - Rust vertical slice (the big one; do in order)
1. [x] Test harness first: run the ROM in an emulator with scripted input, dump player X/Y/state each frame to a file (done early: `harness/`, headless Mesen + Lua, see `harness/README.md`; first measurements in `notes/physics-traces.md`)
2. [x] Rust window, fixed 60 fps loop, draw one extracted background (`port/`: window opens at 60 fps, scrolls stage backgrounds from the ROM; image pixel-identical to the Node renderer for 4 stages; see `port/README.md`)
3. [x] Player: gravity, jump, run, collision against tiles; compare to harness traces (`port/mm4-core/src/player.rs`: identical to the game for 1,911 recorded frames incl. camera plus 100+ random seeds; states 0/1/2/3/0A (ground, air, slide, ladder, spawn); hurt/death not yet, see `notes/collision.md`; playable in the window with a placeholder box)
4. [ ] Scrolling and camera (horizontal chaining `code_3ECAFF` done on a patched ROM, real stage-2 route still to record; follow camera and first-section limit done and verified; sections, doors, vertical scrolling and column streaming not done)
5. [~] Enemies/objects: object system, 9 stage-2 enemy AIs, buster shots, damage, items, hurt, death + respawn all frame-exact (see STATUS.md, notes/objects.md); fish `4C` and gate `41` done in session 4; boss `7E` fight, defeat and victory walk/teleport-out done in sessions 7-8; stage-end flags/weapon unlock done and Mesen-verified in session 9; stages 3 and 5 (all enemies, items, bosses) done in sessions 12-13, stage 1 in session 14, stage 7 in session 15 and stage 0 in session 16 and stage 4 in session 17 (`harness/regress.sh` = 582 ok); stage 6 in session 18 (`harness/regress.sh` = 663 ok), so all eight robot-master stages are done; stages 8-13 (Cossack/Wily) followed in the same session (`harness/regress.sh` = 930 ok); stage 14 (pods, boss rush and the final Wily machine) followed in session 19 (`harness/regress.sh` = 1009 ok): **all 15 stages are done**; still open: the stage-select/weapon-get screens, game over/continue, special weapons, HUD/title, audio (see NEXT-SESSION.md)
6. [ ] Asset pipeline (tiles, palettes, level data) as files, not hard-coded
7. [~] Audio: requests, driver (6502 reference), APU, window output done and checked against Mesen; hand port of the driver left

Stop points: after 2, 3, or any step of 4 there is something usable.

## Model and usage plan (Claude $20 plan)
Sources disagree a little (third-party blogs), so treat numbers as approximate: Pro has a rolling 5-hour window plus weekly caps, and Opus uses allowance roughly 5x faster than Sonnet.
- Default: Sonnet 5.5 (Rust code, harness, builds, label renaming)
- Opus 5.5: only for hard analysis or bugs Sonnet failed on after a couple of tries
- Haiku 4.5: tiny jobs (formatting, simple searches)
- Avoid overnight agent loops; check the usage meter before big tasks
- Commit after every working step so a cutoff loses nothing

## Starting a new chat
Open this folder (`Desktop\MegaMan4 Project`) and say: "Read PLAN.md and continue from the first unchecked item."
Context: the user's name is Luna. The disassembly is in `megaman4-disassembly/`; the owner's ROM is `Mega Man 4 (USA).nes` in this folder (header is NES 2.0, repo output is iNES; bodies are identical). The repo's asm files are huge (up to 1.5 MB), so grep and read line ranges only. Most of `bank00_1A.asm` and `bank21_34.asm` is data; engine code is mainly in `bank38_3F.asm` and the small banks. Almost no labels are named yet.
