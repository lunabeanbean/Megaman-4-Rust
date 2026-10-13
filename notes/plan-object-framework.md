# Plan: object / enemy framework (item 3) - STEPS 0-5 DONE in session 3 (2026-10-07), see STATUS.md and notes/objects.md
(Step 6, more enemy types and a boss with its door, is the open part; all earlier steps were completed with frame-exact verification.)

Written 2026-10-07 at the end of session 3. Start the new chat with: **"Read STATUS.md and notes/plan-object-framework.md, then start at Step 0."**
Luna's working style (see STATUS.md and memory): run and verify things yourself, small verified steps, hard finish line per step, plain-language summaries, update STATUS.md/PLAN.md/notes after each step, never commit ROMs or ripped assets.

## Goal
Port the game's **object system** to Rust (`port/mm4-core`) so enemies, shots and effects behave frame-for-frame like the real game, ending with **one real enemy of stage 2 verified against Mesen traces**, then buster shots and damage. Everything is verified against the real game; unknowns are marked in the notes.

## What we already know (do not re-derive)
- Object slots: `$00` Mega Man, `$01-$07` shots/effects, `$08-$17` enemies/other objects. Per-object arrays are `$18` apart (`$0300` type/AI id, `$0318/$0330/$0348` X sub/px/page, `$0360/$0378/$0390` Y sub/px/screen, `$03A8/$03C0` X speed, `$03D8/$03F0` Y speed, `$0408` hitbox id, `$0420` direction flags, `$0438` spawn-list index, `$0450` (hp or damage?), `$0468-$0510` scratch/timers, `$0528` flags (bit 6 flip), `$0540/$0558/$0570` anim frame/id/tick). See `notes/ram-map.md`.
- Spawner (`code_388000`, `code_388080`) and the placement tables (`$B100/$B180/$B200/$B280` in the stage bank) and type tables (`$817F`... in bank `$38`): `notes/enemy-placement.md` (verified live once).
- Shared helpers already ported: tile probes and push-out (`player.rs`: `point_types`, `vertical_step`, `horizontal_step`), gravity, animation engine (`anim.rs`; per object type bank from `$DFFA+type`), camera/sections (`camera.rs`).
- Movement routines: `code_3FF22D` right, `3FF24B` left, `3FF269` down, `3FF289` up, `3FF2F6` gravity, `3FF0F8` vertical-with-collision (X = slot; the player code only used X = 0, so the `X != 0` branches are new), `3FF3ED/3FF400` horizontal/vertical with collision. Player vertical speed is positive = up; enemy code may differ, read it.
- Harness: `harness/gen_ptrace.js` (+ `random_trace.js`, `random_test.sh`, `rt.ps1`) records the real game; `harness/enemies.lua`, `spawn.lua`, `dump_enemies.ps1` already look at enemy slots. `TELEPORT=page,px,py`, `WAITF=n` (start of log) env vars exist. Trace rows are described at the top of `gen_ptrace.js`.
- The no-enemy ROM `harness/mm4_noenemy.nes` is what the player tests use; **enemy tests must use the real ROM** (`Mega Man 4 (USA).nes`; pass its full path to `rt.ps1`) or a ROM with only the chosen enemy left.

## Steps (each ends with a check that must pass before moving on)

### Step 0 - Recon, no code (read-only, ~1 session)
1. Find the **main loop order** of one frame: where the player update, the object update loop (slots 1-23), the camera code and the draw/animation tick (`code_3FFF1C`) run, and in which order. (We know the camera runs after the player and that the transition loop draws inside the camera code.) Write it into `notes/game-loop.md`.
2. Find the **object update dispatcher**: how `$0300,x` (type/AI id) selects a routine (jump table address), which slots it loops over, and what is skipped (off-screen, `$0528` bits, freeze flags). Note the table address and list the ids used in stage 2 (`harness/dump_enemies.ps1 -Stage 2` -> type list -> `$820F+t`).
3. Find the **despawn** rule (off-screen removal, "defeated" bit in `$0100+`), and the **player-vs-object collision** and **damage** code (hitbox table `$F9F4`, contact damage, invincibility timer, state 6/hurt entry via `$0C`).
4. Choose the **first enemy**: the simplest AI routine among stage 2's types that appears early (small routine, no spawning of sub-objects, no boss logic). Record: type id, AI routine address, hp, damage, hitbox, animations.
Check: notes written; first enemy chosen with its address and size. No Rust yet.

### Step 1 - Object slots and the spawner in Rust (`mm4-core/src/objects.rs`)
- `Object` struct mirroring the arrays above (fixed-point X/Y as in `Player`), `World` with 24 slots, defeated-bit array, spawn cursors `$9E/$9F`.
- Port `code_388000/388080` (sweep as the camera moves, no duplicates, free-slot search `code_3FFB5C`, init from type tables) reading the placement/type tables from the ROM. Commands `type >= $C0` can be ignored for now (palette/CHR changes) but must advance the cursors correctly.
- Trace: extend the harness trace (new generator, e.g. `gen_otrace.js`) to log slots `$08-$17` every frame: `$0300, $0348/$0330/$0318, $0390/$0378/$0360, $0558, $0528, $0438`. Replay the player's recorded **inputs** through player+camera+spawner in Rust.
Check: for a long scripted run along stage 2, the set of spawned slots (index, type, x, y, flags, anim at spawn frame) matches the game on every frame. **Spawn only; objects do not move yet** (compare the spawn frame and position, then ignore the object).

### Step 2 - Make the comparison safe: enemies must not touch Mega Man
Hurt/damage is not ported yet. In the recording Lua keep Mega Man out of trouble: set his invincibility/damage timer every frame (find the address in Step 0) **or** run inputs that dodge, **or** write life back each frame and stop comparing the player after a hit. Prefer the invincibility write if the contact code honours it (then no hurt state happens at all). Document the choice.
Check: a 1,200-frame run with enemies on screen records without the player ever entering state 6.

### Step 3 - One enemy, frame-exact
- Port the chosen AI routine, the object update loop and the object-side collision (X != 0 branches of `vertical_step`/`horizontal_step`, shared helpers parameterised by slot).
- Trace compare slots `$08-$17` every frame (position, speeds, anim id/frame, flags, scratch bytes that the AI uses).
Check: that enemy matches the game for its whole life on-screen on at least 5 recorded runs (different Mega Man inputs, so different spawn/despawn timing).

### Step 4 - Buster shots and enemy damage
Slots 1-7: shot spawn (B button, `$14 & $40`), movement, tile/enemy collision, enemy hp and death (explosion object, drop item roll, set "defeated" bit). Player `$A0`/`$33` weapon state (the ladder and ground code currently ignore it). Random tests gain a B button (`random_trace.js`: `b=true`).
Check: random runs with B pressed match, including enemy kills.

### Step 5 - Player damage, hurt (`06`), death and respawn
Contact damage, invincibility, knockback (state 6 at `$85DD`), life meter `$B0`, death sequence and respawn at the checkpoint (state `0F`, see notes/player-states.md; needs the section/checkpoint tables in `camera.rs`). Remove the Step 2 workaround.
Check: random runs without the workaround match, including a death and respawn.

### Step 6 - More enemy types and a boss
Independent enemy types can be given to helper agents (one type each, same trace check). Then one boss with its door/intro (needs horizontal section handling, `code_3ECAFF`, and the scroll lock).

## Risks and decisions
- The type ids are global (144 entries) but stage 2 only uses a handful; port the stage-2 ones first.
- Enemy code may use the carry flag conventions of the player helpers differently; port helper by helper with a trace check, not by guessing.
- Slots are scanned in order and objects can spawn other objects mid-loop; keep slot order identical to the game.
- Animation tick phase after a spawn is known (set by `code_3FF446`), but the draw/tick order relative to the AI update matters; Step 0.1 must settle it.
- Budget: Step 0 and 1 are the biggest unknowns. Use big runs in the background, ~20 s per recorded seed.
- Ask Luna before: changing the harness ROM, deleting traces she wants, publishing anything.

## Files you will touch
`port/mm4-core/src/{objects.rs (new), lib.rs, verify.rs, player.rs, camera.rs}`, `port/mm4/src/main.rs` (draw enemies as boxes), `harness/gen_otrace.js` (new), `notes/{game-loop,enemy-placement,ram-map}.md`, `STATUS.md`, `PLAN.md`.
