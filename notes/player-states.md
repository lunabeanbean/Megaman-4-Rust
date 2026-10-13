# Player state machine (`$30`)

Each frame the player update (`$3C80DA`) reads `$30`, looks up a handler address in a table
(low bytes at `$3C80FA`, high bytes at `$3C810E`, 20 entries) and jumps to it with `JMP ($0000)`.
Handler addresses below are CPU addresses in bank `$3C` (file: `bank38_3F.asm`, grep `; $3C<addr> |`).

Conf: **C** = seen in a harness run (`harness/states.lua`), **L** = read from the code, **?** = only a guess.

| `$30` | Handler | Meaning | Conf |
|---|---|---|---|
| `00` | `$8130` | Standing / walking on the ground. Pressing A (not Down) writes vy=`$05.00` and goes airborne; Down+A starts a slide | C |
| `01` | `$82C8` | In the air (jump or fall). Gravity applied; releasing A sets vy=`-1.0` | C |
| `02` | `$8353` | **Slide.** Set up by the ground handler: `$34`=`$1A` timer counting down (25 frames seen), X speed set to `$02.80` (2.5 px/frame), anim `$10`, `$31` = direction | C |
| `03` | `$8420` | Reads Up/Down presses (`$16 & $0C`) and `$33`; likely **ladder climbing** | L |
| `04` | `$84AD` | Plays animations `$27`/`$29`; probably ladder entry/exit or a shooting pose on a ladder | ? |
| `05` | `$858D` | Anim `$3A+$33`; likely a special pose (charge/ buster / item pickup) | ? |
| `06` | `$85DD` | **Hurt (knockback).** Seen when an enemy hit Mega Man: life dropped, state lasted about 29 frames, then back to `00`. Sets X speed `$00.80` | C |
| `07` | `$865C` | Empty (just `RTS`): player not controlled | L |
| `08` | `$865D` | Anim `$32`, zeroes speeds and `$33`; a frozen pose (cutscene/teleport/ending) | ? |
| `09` | `$86AD` | Collision-check then anim `$01` (idle); "waiting" state | ? |
| `0A` | `$86CE` | **Spawn / teleport-in** at stage start (anim `$13`); Mega Man falls from the top (vy` -1.0..-7`) until he lands, then `00` | C |
| `0B` | `$87A7` | Landing / ready after teleport; picks anim `$07`, `$04` or `$01` | ? |
| `0C` | `$8850` | Damage check: switches to anim `$11` and sets `$30 = 06` | L |
| `0D` | `$8895` | Collision check, anim `$07` | ? |
| `0E` | `$891A` | Plays a sound (`$0780` = `$28`) and uses `$6C-$6F` tables: stage-clear style sequence | ? |
| `0F` | `$8A70` | Clears `$06F0-$06F3`, calls `$3EC457`; looks like death or teleport-out | ? |
| `10` | `$8C49` | Walks right automatically until X = `$40` (cutscene/ending) | L |
| `11` | `$8775` | Sets a flag in `$0133`/`$1F`, calls the `08` routine | ? |
| `12` | `$8C65` | Collision check then anim `$07`/idle; cutscene idle | ? |
| `13` | `$865D` | Same handler as `08` | L |

## Related bytes
- `$31` = facing/direction: `01` = right, `02` = left (also copied to `$0420`).
- `$34` = slide timer (starts `$19`/`$1A`, counts down to 0). `$32`, `$33` = ladder / animation sub-state, not yet understood.
- `$0558` = animation id, written by `code_3FF446`; handlers compare it to avoid restarting an animation.
- Slide speed `$03C0.$03A8` = `$02.80` (2.5 px/frame); run speed `$01.4C`.

## Re-run
`harness/states.lua` logs `$30/$31/$34` and the controller each time something changes. Edit its `seq` table
to drive other inputs (ladder, shooting, charging) and map the states marked **?**.

## Correction (found while porting)
Earlier text in these notes treated `$14` as "held". It is the **newly pressed** mask; `$16` is the **held** mask (see notes/ram-map.md).
So: jump starts on a new A press (`$14 & $80`), the jump is cut short when A is no longer held (`$16 & $80` = 0), walking uses `$16 & 3`.

## Ground walking details (code_3C8214, verified by random tests)
Walking is driven by the **animation** (`$0558` id, `$0540` frame index, `$0570` tick; engine at $3EDC67, ported in `port/mm4-core/src/anim.rs`). The animation tick runs once per frame *after* the player logic.
An animation record (bank from `$DFFA`+type, pointers $BE00/$BF00): byte0 = last frame index N, byte1 = duration D, then N+1 frame ids; the frame index advances when the tick equals D, wrapping after N.
- Anim 4/5 (run): no direction -> set anim $0D (no move). Direction & facing($31) != 0 -> move. Otherwise (reverse) -> facing = dir, anim $0D, move one step.
- Anim $0D-$0F (start/stop; $0D: N=1, D=5): frame index 0 -> nothing happens (any held direction ignored). Index != 0: no direction -> idle anim 1; direction -> facing = dir, anim 4, move.
- Any other anim (standing = 1): no direction -> anim 1; direction -> facing = dir, anim $0D, move one step.
- So a step is followed by 5 frames of no movement (anim $0D lasts 6 frames), and *stopping* also plays $0D, which blocks a new step for 5 frames. The old note "step + 5 frame delay" was this.
- Landing while a direction is held goes straight to running (no step delay) and moves horizontally in the same frame.
- In the air, left/right move every frame at 1.297 px/frame; with no direction held `$0420`/`$31` become 0 and there is no horizontal move.
- Jump: on the ground frame the code first runs the normal vertical step (vy = -1 stand push), then on an A press sets vy = `$05.00` and runs the vertical step again that frame.
- Releasing A while rising sets vy = 0 (not -1); gravity then takes over on the next frame.

## Slide, state 2 (code_3C8156 start, code_3C8353 frame; verified by random tests with Down held, 2026-10-07)
- **Start**: on the ground, A newly pressed **and Down held** (`$04B0` = 0). Direction = held L/R, else the flip flag (`$0528` bit 6: set by the last move to the right, cleared by a move to the left). Wall test: horizontal probe id 6 (right) / 7 (left) = one point 15 px ahead at y+4; if solid, no slide (the walking code runs instead, no jump).
  Otherwise: `$31` = dir, `$34` = `$1A` (26 frames), `$30` = 2, speed `$02.80` (2.5 px/frame), anim `$10`, **Y += 2**, then the slide frame code (below) runs in the same frame without a vertical step.
- **Frame**: vertical step with hitbox 4 (feet id 4, head id 5; side probes id 4 right / 5 left). If not landed and probe `$39` (y+16, x-14/0/+14) finds nothing solid: slide ends and he falls.
  Then (`code_3C8368`): A newly pressed, Down not held, and nothing at all in probe 3 (y-14, x-14/0/+14): speed back to 1.297, jump (vy 5.0, state 1, standing hitbox).
  Else a held direction opposite to `$31` turns him around and sets `$34` = 0. Move by `$31` at the slide speed; if no wall was hit and `$34` != 0 then `$34`--, and if it is still nonzero the frame ends.
  When `$34` is 0 (or a wall was hit): if probe 3 has a solid tile above he keeps sliding, otherwise the slide ends: state 0, speed 1.297, anim 4, vy = -1.0.
- The horizontal move routine takes its speed from `$03A8/$03C0` (running 1.297 = `$01.4C`).
- Terrain lookup uses the page number as the high byte of X: **X < 0 reads page `$FF`** (bytes from `$B5FF` and the fixed bank), which is a wall in stage 2. This is why a left slide at the stage edge ends early.
Hitbox tables are parsed from the ROM by `port/mm4-core/src/hitbox.rs` (record = `[last, fixed coord, cumulative offsets...]`).

## Ladder, state 3 (code_3C81D6 / code_3C922B / code_3C8420; ported 2026-10-07, verified by random tests next to the ladder at stage 2 x=464)
State `$04` (`$84AD`) is **not** the ladder (it is a free-flight style state with anims `$27/$29` and Y clamped to `$10..$E0`); state **3** is the ladder.
Terrain types on the ladder: `$20` = ladder body, `$40` = ladder top. Probe `H2` (code_3ED428, horizontal id 2: X+0, Y-11/0/+5) returns `$45/$46/$47`, with `$40` read as `$20`.
- **Grab from below/air (code_3C922B)**: called at the end of every ground-walking path (code_3C82A9) and every air frame (code_3C833D). Up held, and `$45` or `$46` is a ladder: state 3, X snapped to the tile centre (`px = (px & $F0) | 8`, subpixel and page untouched), speed field `$03D8/$03F0` = `$014C` (1.297), anim `$0A`. No vertical move that frame.
- **Grab from the top (code_3C81D6)**: on the ground, Down held and the middle feet-probe point (`$46`, vertical probe 0, raw value) is `$40`: Y = probe Y `& $F0` (tile top), X snapped, state 3, anim `$14`, then the Up-grab check runs.
- **Ladder top is a one-way floor** (code_3FF1B6): when falling, if the highest type under the feet is exactly `$40` and the probe Y is in the upper half of the tile (`y & 15 < 8`), he lands. Rising and climbing ignore it.
- **Frame (state 3)**: no Up/Down held: anim tick `$0570` = 0 (animation frozen). Else `$31` = held Up/Down bits (`4`/`8`/`12`), Down wins when both are held: move `1.297` px down (feet probe 0 pushes up) or up (head probe 1 pushes down). A bump while Up is in `$0420` (so also Up+Down): tick = 0 and the A check; a bump going Down: leave the ladder.
  Then probe H2: if none of the three points is a ladder: leave. Else re-snap X; wanted anim = `$0A`, or `$14` if both `$45` and `$46` are empty (only the lowest point is on the ladder); if the anim differs, switch and end the frame (the A check is skipped that frame).
  A newly pressed with no Up/Down held drops off (A check, code_3C8486).
- **Leave (code_3C8492)**: state 0, vy = -1.0, `$31` = right if the flip flag is set else left; the anim stays `$0A/$14` and the ground code replaces it next frame.
- Not handled: weapons on the ladder (`$33`, shots), screens above/below (Y outside 0..239 behaves differently), and the section change that climbing off the top triggers in stage 2 (the verifier stops there).

## Spawn, state `0A` (code_3C86CE; ported + verified 2026-10-07: 200 frames identical, trace from the first frame of the wait animation to state 0)
- Phase 1: anim `$33` (the waiting beam, he sits at X `$80`, Y `$64`) plays until its frame index (`$0540`) is `$19` (25 frames x ~6 ticks), nothing else happens.
- Phase 2 (same frame as the switch): anim `$13`, flip flag set, Y pixel = 0, vy = -1.0 from before. While Y < `$40` or horizontal probe H2 sees something solid: move by vy then gravity, no collision (code_3FF2A7). After that the normal vertical step (hitbox 0). Terminal speed -7 as usual.
- Landing: the anim `$13` frames 0..3 play (tick runs); when the frame index is 4: anim 1, state 0, `$31` = 1. While falling the anim tick is reset to 0 every frame (the frame stays 0).
- Trace trick: the animation phase at the first logged row is unknown, so `verify.rs` finds the row where the frame index next changes and sets the tick from that (`Anim::sync_tick`).
- Repro: `cd harness; WAITF=512 node gen_ptrace.js sp 200 ""` then run it with `rt.ps1` on `mm4_noenemy.nes` and `mm4.exe --stage 2 --verify ptrace_sp.txt`.

## Death / respawn, state `0F` (code at `$3C8A70`, read only, not ported)
Waits for the explosion anim (`$0540` == 4), clears `$06F0-$06F3`, calls the level reload (`$3EC457`, `$3EC438`), loads the checkpoint position from tables (`$8BB9`, `$8BC7`, `$8B8D`, `$8BA3`, indexed by `$2B & 15`), sets camera/page (`$F9`, `$2A`, `$28`, `$27`), then state `0A` + anim `$13` and the same spawn drop. So respawning needs the **section/checkpoint and camera reload** first (section table `$B530`/`$B540`); the code that starts the death (HP 0, pits) is separate and not located yet.

## Shooting and getting hurt (ported 2026-10-07, `player.rs` + `objects.rs`; verified with `harness/game_test.sh`)
- **Buster** (`code_3C8CBD`, weapon 0 only): called from the end of the walking / landing / airborne paths (`$3C82A9`, `$3C834B`) and at the start of the ladder state (`$3C8420`).
  B newly pressed (`$14 & $40`) starts a shot and sets the charge counter `$35 = 1`; while B is held `$35` grows each frame (capped at `$5C` once it reaches `$64`);
  releasing it with `$35 >= $20` fires a charged shot (anim `$21`, or `$20` from `$5C`), with less the counter is just cleared. A shot needs a free slot among 1-3 (the lowest free one is used) and no big charged shot (anim `$20/$23`) in flight.
  Firing sets `$32 = $10` (pose timer) and, if `$33 == 0`, `$33 = 1` and the animation id **+1** (shooting pose); when `$32` runs out the id is decremented by `$33` again.
  Every place that starts a walking/standing/air/ladder animation adds `$33` (`code_3C8204, 823B, 8258, 8295, 829C, 82FD, 8330, 8477, 9267`); the air handler compares with the plain id 7. On the ladder `$33 != 0` stops climbing. Sliding clears `$32/$33`.
  The shot object (type 2, AI `$3A8825`) moves at 4.2 px/frame, spawned at x-15 / x+16 (`$F50B`); see notes/objects.md.
- **Slide dust**: sliding creates a type-1 effect object in slot 5 (anim `$17`) when slot 5 is free.
- **Hurt** (`code_3A81CC` takes the hit, state 6 = `code_3C85DD`): damage from `$3A8413 + type`; `life = 0x80 | (units - damage)`; old state saved in `$0468`, probe base `$0480` (0, or 4 when sliding); `$30 = 6`, vy = -1.0, anim `$11`; a spark effect (anim `$12`, slot 5) appears.
  The state knocks him back at 0.5 px/frame (away from the side he faces), with the normal vertical step, until the hurt animation reaches frame 9; then `$3C = $3C` (60 frames of invincibility) and back to state 0 with animation id 0. Contact is ignored while `$3C != 0`, `$30 >= 6` or the animation is `$27`.
- Death (damage >= life): `$30 = 7`, 16 particle objects (type `$2E`), `$0133 = 300` frames, then the stage restarts at the checkpoint (see STATUS.md for what is ported).
