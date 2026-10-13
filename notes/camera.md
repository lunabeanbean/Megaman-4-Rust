# Camera / scrolling

Measured with `harness/scroll.lua` (writes `scroll_out.txt`, `scroll2_out.txt`) plus the scroll code at `$3EC96x`-`$3ECA9x`
(`bank38_3F.asm` lines ~20560-20720). Conf: **C** = measured, **L** = read from code, **?** = guess.

## What reaches the screen
Every frame the NMI handler (`$3EC000`, line 19437) copies the game's scroll variables to the PPU shadow values:
| Game variable | Copied to | Meaning | Conf |
|---|---|---|---|
| `$FC` | `$66` -> `$2005` (first write) | **Camera X, low byte** | C |
| `$FA` | `$65` -> `$2005` (second write) | Camera Y scroll | C (Y never changed in the test stretch) |
| `$FD` | `$67` (low 2 bits OR'd into `$2000`) | Nametable select | L |
| `$F9` | (not copied directly) | **Camera X, high byte (screen/page)** | C |
| `$2E` | | Camera screen index; compared with `$2D` (**last scrollable screen of the stage**) | L |
Some stages/modes (`$F0` = 3, 6, 7 or >= 9) use `$69`/`$6A` for `$66`/`$67` instead (alternate scroll, probably boss rooms or split screens) | L

## Horizontal follow rule (stage `$02`, 60 fps)
- Camera X = **player world X - `$80`**, clamped at 0 on the left: the player stays about 128 px from the left edge of the screen.
  Measured: player `$00AF` -> camera `$2F`, player `$012B` -> camera `$AB`. | C
- Scrolling is capped at **8 px per frame** (`$3ECA38`: `LDA #$08`), more than the 1.3 px/frame run or 2.5 px/frame slide. | L
- `$27`/`$28` hold the follow threshold (set to `$80` at stage start, `$3EC64C`); the code compares player X with it
  (`$3EC9D3`) and scrolls right if the player is beyond it, left if below it. | L
- When the camera reaches screen `$2D` (the last one) it stops, and `$FC` is forced to 0 unless `$3F == $50` (special end-of-stage case). | L
- `$2F` / `$10` = scroll direction for this frame (1 = right, 2 = left); `$25` = previous camera low byte, used to find
  8/16-pixel boundaries where a new column of the map has to be streamed in. | L

## Written from
All `$F9`/`$FC` changes in the run came from `$3ECA45` (scroll right) and `$3ECA00`-`$3ECA0D` (scroll left) i.e. one routine,
`code_3EC96x` .. `code_3ECA79` (not yet named). The map-streaming code after `$3ECA79` is the next thing to read.

## For a port
Camera = clamp(playerWorldX - 128, 0, lastScreen*256 + ...), moving at most 8 px/frame, nametable and column streaming
driven by the camera's 8-px boundaries. Vertical scrolling and special rooms still need traces (a vertical stage and a boss door).

## Screen-edge clamp of the player (code_3ECC13, found by random tests 2026-10-07)
Whenever the camera X low byte (`$FC`) is 0 (camera exactly on a page boundary), the player's X low byte (`$0330`) is forced into `$10..$F0`
(sub-pixels and page untouched). This is what stops Mega Man at the left edge of stage 2 (px `$10`, there is no terrain wall) and keeps him
off the right edge of a locked screen. A second copy of the left clamp sits in the camera code at `$3ECA20` (when the camera would go below 0).
Ported in `camera.rs` (`Camera::update` takes the player and applies it after the follow step).

## Sections and vertical transitions (ported 2026-10-07 in `camera.rs`; verified: 12 random seeds x 1,200 frames through the stage-2 ladder at x=464 into section 1)
State: `$2B` = section index (low nibble) | flags (`$80`/`$40` vertical section direction bits, `$20` first-screen flag), `$2D` = last screen index of the section, `$2E` = screen index (= page - start page), tables `$B530` (sections: low nibble = last screen, high bits = flags) and `$B540` (4-byte entries: page, direction flags, target section, target page; first byte `$80+` ends it; stage 2's table is empty).
Trigger (code_3ECC28, after the camera follow each frame, only on Y screen 0): player Y >= `$E8` (down, `$01`=4) or, only in the ladder state, Y < 8 (up, `$01`=8).
- Vertical section (`$2B & $C0`): `(dir<<4) & $2B` set = next screen (+1 page, or next section when on the last screen), else previous.
- Otherwise code_3ECD24 (needs `$FC` == 0): next/previous section is entered only if its flag bits allow this direction; failing that the `$B540` table is tried; failing that nothing happens.
- Transition loop (code_3ECDD5, 120 iterations, game logic frozen): `$FA` += 2 (down) or -= 2 (up, wrapping through `$F0`), the player's Y moves the opposite way by 1.75 px (`$1C0`, mod 240), animation keeps ticking; the first iteration runs in the trigger frame (after that frame's normal player update); one more idle frame follows the last iteration. The player keeps his pixel X but his page becomes the camera's page.
- Stage 2: climbing off the top of the ladder at x=464 (section 0 = 2 screens, `$21`) enters section 1 (`$80`, one screen at page 2); the player appears at the bottom of it and keeps climbing.
Not ported: horizontal section chaining at the end of a section (code_3ECAFF), `$20` first-screen loading, tall stages (Y screen != 0), nametable streaming.

## Horizontal section chaining (code_3ECAFF, ported + verified 2026-10-07, session 5)
- Trigger (inside the camera follow, code_3ECA53): the camera has just reached the section's last screen (`$2E == $2D`, `$FC` forced to 0) **and** Mega Man's X low byte is above `$F0` (it is then clamped to `$F0`). Without that, nothing happens (and `$2F`/dir is not touched).
  **Second trigger (found with the real stage-2 route):** `$3F` == `$50` also fires it (no clamp of X). `$3F` is zeroed at the start of every logic frame (code_3EC7BE) and set by the right-hand wall probe (code_3FF032: `$3F = $45` = type of probe 0) whenever Mega Man walks into a solid tile: stage 2 section 7 ends in type-5 wall tiles (page 20, x `$F0+`), walking into them is what chains 7 -> 8. Port: `Player::wall_type`.
  Otherwise it only fires if he can physically walk past `$F0` on the last screen (stage 2's real first section has a wall there; the chain used by the real game is section 6/7 -> 7 -> 8 -> 9, flags `$26`, `$30`, `$30`).
- code_3ECAFF: refused if state `$30` = 4 and the next section has flag `$10`. If the next section's flags `& $E0 == $20` it is entered; else the `$B540` door table is searched for the current page: entry flag `$20` = door, target page (`$B543`) goes to `$2A`, `$9E/$9F = $B300[target]`, camera/player page = target - 1, section = `$B542` (door variant: ported, **untested**, stage 2's table is empty).
- code_3ECB1B: `$2B = (flags & $30) | section`, `$2D` = length, `$2E = 0`, dir `$2F = 1`; section flag `$10` runs code_3EE682 (draws door tiles, `$95` = 0, 16 frames -> `$95` = 16; **ported only as the `$95` value**, untested).
- Loop (code_3ECB6E, same shape as the vertical loop): wipes object slots 1-23 once (code_3EC438: type, `$05A0`, `$05B8`, `$0450` = 0, `$0438` = `$FF`), then 128 iterations of camera +2 px and player +0.5 px (sub-pixel `$80`), one video frame each, game logic frozen. After it: flag `$10` sections run code_3EE751 unless the page is `$CC03[stage]`.
- The spawner runs only after the loop (the loop lives inside the camera routine), so the new section's first enemies appear then. Vertical transitions wipe the slots the same way.
- Draw pass: every draw call counts Mega Man's `$05B8` (blink timer copied from `$3C`) down, so after a 128-frame loop he is no longer blinking.
- Vertical sections (`$2B & $C0`) never scroll sideways (code_3EC972 jumps to code_3ECC13).
- Test: `harness/patch_rom.js` makes `mm4_chain_ne.nes` / `mm4_chain.nes` (stage 2 section 1 byte `$B531 = $21`, screen 1 = layout 0 so there is no wall): `RIGHT=0.9 B=0.3 sh game_test.sh mm4_chain.nes 1100 431 432 433 434` all identical (original enemies, shooting).


## Real stage-2 chain 7 -> 8 (verified 2026-10-07, session 6)
Section table of stage 2 (`$B530`): 0:$21 (pages 0-1), 1:$80 (2), 2:$A1 (3-4), 3:$41 (5-6), 4:$62 (7-9), 5:$80 (10), 6:$A2 (11-13), 7:$26 (14-20), 8:$30 (21), 9:$30 (22, boss `79`). Recorded with `TPDOORS=16,17,18,19 TPCUR=27 TPSECT=7,14,39,6 TELEPORT=17,64,80 TPWAIT=8 RIGHT=0.9 B=0.3 sh game_test.sh mm4_orig.nes 3000 <seeds>`: 7 -> 8 chain (128-frame scroll, object wipe, `$95` handling) and section 8 identical to the game for ~1100-1270 frames (seeds 681, 708), until the boss door (state `$09`). Teleports without `TPSECT` leave the game with the section variables of section 0, so the camera then behaves differently from the port (that was an early false alarm).
Gate state (`$0670+page`) is only cleared at stage init (and `$0680-$06BF` there), **not** on death/restart: the port keeps `Stage::door` across restarts, as the code does (not recorded).
