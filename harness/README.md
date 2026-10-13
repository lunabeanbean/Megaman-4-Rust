# Harness

Headless Mesen runs that log RAM per frame, so nobody has to watch the emulator.

Run: `powershell -ExecutionPolicy Bypass -File rt.ps1 trace.lua` (from this folder).
- Mesen is at `C:\Users\joele\Downloads\Mesen\Mesen.exe` (Mesen 2.x; its data is in `Documents\MesenCE`).
- Needs `"AllowIoOsAccess": true` in `Documents\MesenCE\settings.json` (set this session;
  backup of the original: `settings.json.bak`). Set it back to `false` if you stop using scripts.
- Lua scripts must use absolute paths with forward slashes for output files. Relative paths land in `Documents\MesenCE`.
- `emu.stop(0)` must be called from inside a callback, not at load time.
- `probe.lua` just logs scene changes. `trace.lua` navigates the menus, enters a stage and runs a scripted input sequence,
  writing `trace_out.txt` (only lines where something changed).
- Edit the `seq` table in `trace.lua` to script other inputs. The ROM path is set in `rt.ps1`.

## Testing a ROM edit
xkas patches the output file in place and does NOT write an iNES header, so copy first:
    cp mm4.nes mm4_test.nes && ./xkas.exe -o mm4_test.nes assemble.asm      (from megaman4-disassembly/)
then: `powershell -ExecutionPolicy Bypass -File rt.ps1 trace.lua "<full path to mm4_test.nes>"` and compare `trace_out.txt`
with the baseline (`trace_base.txt`). `writes.lua` logs the instruction address (PC after the write) for every write to the
slot-0 speed bytes; edit its address list to trace other RAM.

## Player traces for the Rust port
`node harness/gen_ptrace.js <name> <frames> "<lua seq>"` writes `ptrace_<name>.lua`; `sh make_traces.sh` runs the four standard traces
(a, b: original ROM; c, d: `mm4_noenemy.nes`, stage 2 with the enemy list removed using tools/stage.js so nothing hurts the player).
Check the Rust physics against them from `port/`: `target/release/mm4.exe --stage 2 --verify ../harness/ptrace_d.txt`.

## Whole-game and object traces (session 3)
`OBJ=1 node gen_ptrace.js <name> <frames> "<seq>"` (normally called through `random_trace.js`) logs at the **end of every logic frame** (breakpoint `$C7C6`):
`ptrace_<name>.txt` (player row, 21 columns, last = `$3C`), `otrace_<name>.txt` (first line `S` = all slots incl. stale bytes, then per frame `t G <E4 E6 E5 $95 $9E $9F $2F $FD $2E E7 $92> O n <slot + 30 bytes>*n`) and
`atrace_<name>.txt` (the 256 OAM bytes). The buttons are captured at `$C740` (the player code's view). Env: `INVINCIBLE=1`, `TELEPORT=page,x,y`, `WAITF=n`, `TPWAIT=n` (settle frames after a teleport, default 300), `TPSECT=sect,start,$2B,$2D` (also write the section variables so the camera follows the real section rules; then the port uses its own camera instead of `MM4_CAM_TRACE`), `TPCUR=hex` (spawner cursors `$9E/$9F` after the teleport, = `$B300[camera page]`), `TPDOORS=16,17,..` (stage 2 gate shutters already closed, `$0670+page = $7F`; the port gets `MM4_DOORS`), `KEEP=1` (game_test.sh keeps passing traces), `MM4_TRIES=1` (verifier prints why every rng-step split failed), `B=0.4` (probability of holding B in a random segment), `DOWN`, `UP`.
- `sh game_test.sh <rom> <frames> <seeds...>`: record + replay the **whole game** in Rust (`--verify-game`), compare player, camera, every object slot, random state, OAM list. `NOINV=1` lets enemies hurt (hurt/death/respawn are checked); a teleport sets `MM4_CAM_TRACE`.
- `sh enemy_test.sh`: same with `--verify-obj` (player taken from the trace).  `sh random_test.sh`: player only (no enemies).
- `node make_enemy_rom.js 2 <out.nes> 00,3B` builds a ROM whose stage keeps only some placement-list types (needs `tools/stage.js`); `mm4_noenemy.nes` has none.
- Never edit a script or rebuild the exe while a batch runs (sh reads scripts incrementally, the batch uses the exe). `rt.ps1` kills every Mesen: one recording at a time.

## Session 7 additions
- `JOBS=n` (game_test.sh): n seeds at the same time; `rt.ps1` only stops the Mesen it started.
- `BOSSHP=n`: the stage boss (`$7E`) gets n hit points the video frame after it appears (recorder: `gen_ptrace.js`; replay: `MM4_BOSSHP`, applied by `World::ai_phase`).
- `verified_routines.txt`: routine addresses seen in passing recordings (feeds `tools/checklist.js`). `slotlog.js <otrace> <ptrace|-> [from] [to]` prints the object slots per frame.
