# Game loop and engine structure

All addresses are CPU addresses; "line" = line in `megaman4-disassembly/bank38_3F.asm`. Conf: C = confirmed by a trace, L = read from code, ? = guess.

## Hardware / mapper (MMC3)
- Banks switched at `$8000` (R6) and `$A000` (R7) via `code_3FFF37` (line 26369): `$F5` = bank for `$8000`, `$F6` = bank for `$A000`.
  `$C000-$FFFF` is fixed (banks `$3E`/`$3F`: NMI, scheduler, shared object code). Kept in sync in `$F2/$F3/$F4` and guarded by `$F7`.
- Bank usage seen: `$38` enemy spawner/enemy code at `$8000`; `$39` stage-select/title/menu logic; `$3A` weapon/shot code (`$8003`);
  `$3C` **Mega Man player code** (`$8000` entry, state table `$80FA`; pause/weapon menu `$9600`); `$20|stage` (`$20`-`$2F`) = stage data at `$A000`
  (and some per-stage code at `$B800`); `$1E/$1F` sound driver (called after NMI).
- IRQ (`$C149`) is used for the mid-screen split (status bar / scroll change): `$5C` latch value, jumps through `$009C`.

## Reset and the task scheduler (L)
- `RESET` (line 26184): standard NES init, clears RAM, sets bank registers, then starts **task 0** at `$C50C` (`$93/$94` = address) and falls into the scheduler.
- A tiny cooperative scheduler runs up to **4 tasks**; task `n` has 4 bytes at `$80+4n`: `[state, wait counter, saved stack pointer, ...]`
  (`code_3FFEA5`, line 26266).
  - state `0` free, `1` waiting (wait counter in `$81+4n`), `2` running, `4` ready, `8` new (start address in `$82/$83`).
- `code_3FFF1C` ("yield / wait N frames", line 26351; callers put N in `$93`, default 1) saves the stack pointer and returns to the scheduler.
  `code_3FFF15` = wait X frames.
- The NMI (`$C000`) decrements the wait counters of waiting tasks and flips them to ready when they hit 0, then rewrites its own return
  address so execution resumes in a trampoline at `$C127` that runs the sound driver (`code_3FFF5C`) before going back to the interrupted code.

So a "frame" = one NMI: tasks run between NMIs, each yielding once per frame.

## NMI (`$C000`, line 19437) every frame (C: `$65/$66/$67` written here)
1. If `$EA` (rendering disabled flag) is set, just reload IRQ latch and leave.
2. Copy scroll: `$FA->$65`, `$FC->$66`, `$FD->$67` (some modes use `$69/$6A`), `$F1->$5C`, `$F0->$5D`.
3. Sprite DMA from `$0200` (`STA $4014` with `$02`).
4. If `$19`/`$1A` set: flush VRAM update buffers (`$0780`+ for name table columns/attributes; `code_3EC330/334`).
5. If `$18` set: upload the 32-byte palette buffer `$0600-$061F` to `$3F00`.
6. Write `$2005` scroll, `$2001` (`$FE`), `$2000` (`$FF | $67&3`); set the MMC3 IRQ latch for the split line.
7. Read controllers happens in `code_3EC37D` (line 19882): `$14`/`$15` held, `$16`/`$17` newly pressed.

## Task 0 / main program (`$C50C`, line 20116)
Sets up task 1 (`$C4D6`: every frame calls the stage bank routine `$B800` with bank `$20` swapped in, then yields), enables IRQ, loads bank `$39`
and calls its entry (`$8000`), then the scene code at `$8006` etc. `$22` is the **scene/stage id** that picks what runs (menus `$10`-`$13`, stages `$00`-`$0E`+).

## In-stage frame loop (`code_3EC6C7`, line 20315, C order confirmed by reading; frame timing not yet profiled)
Each iteration:
1. `code_3FFF1C` - wait for next frame.
2. `code_3EC451` - copy palette `$0620 -> $0600` (fade handling).
3. Pause check: Start (`$14 & $10`) when the state `$30` allows -> bank `$3C` `JSR $9600` (pause / weapon-select menu).
4. **Player update**: bank `$3C`, `JSR $8000` (dispatches on `$30`, see notes/player-states.md).
5. **Weapon/shot update**: bank `$3A`, `JSR $8003`.
6. **Camera**: stage bank, `code_3EC96E` (see notes/camera.md); also streams new map columns.
7. **Enemy spawner**: bank `$38`, `JSR $8000` (see notes/enemy-placement.md).
8. Fixed-bank updates: `code_3FE0FA`, `3FE35C`, `3FE262`, `3FE429` (palette fades `$0135-$0137`), `3FE49C`, `3FE4DD` (stage specific effects/ doors/ water).
9. Copy player speed settings (`$37/$38` -> `$03A8/$03C0`), update `$25` (previous camera X), `$27/$28` (player X) and the scroll/screen counter `$1F`.
10. Build sprites: `code_3EC421` clears OAM from `$97`, `code_3EDB3A` runs the **object update loop** (every object slot with `$0300,x != 0` runs its AI; odd/even frames alternate order via `$95`) and draws sprites into `$0200`.
11. Loop (the end of this loop, death/stage-clear exits and boss handling are further down from line ~20440 and not yet read).

## Key flags
`$19` / `$1A`: pending name-table column / attribute upload; `$18`: palette upload pending; `$EA`: screen off (set by `code_3EC369`, cleared by `code_3EC373`);
`$9A`: nonzero while the game logic runs normally (0 during transitions); `$97`: first OAM byte to clear (sprite 0 reserved).

## Not done
- Exact cycle/NMI budget per frame, exit paths from the stage loop (death, boss defeat, door transitions).
- Contents of bank `$39` scene code (title, stage select, password) and the sound driver.

## Confirmed in session 3 (frame-exact tests, see notes/objects.md and port/mm4-core/src/game.rs)
Order inside one logic frame (`code_3EC6F3` loop), each step verified by trace comparison:
1. prelude of the player code (`$3C80CF`): `$32/$33` shooting-pose timer, charge counter `$35`, `$3C` invincibility countdown
2. player state handler; 3. `$3A8003`: spike death check (`$3D`), then the object AI loop (slots 1-23, with shot/contact tests for slots >= 5, Mega Man's hurt/death applied inside it), then the crush check
4. camera (`code_3EC96E`, sections, vertical transitions run their 120-iteration loop **inside** this frame); 5. enemy spawner; 6. fixed-bank updates, checkpoint counter `$1F` (`$3EC788`)
7. draw pass `code_3EDB3A` (despawn, animation ticks incl. Mega Man's, OAM) 8. `$3EC7C6` = end of the logic frame (the test traces are logged here), then `JSR code_3FFF1C` (wait for the next video frame).
- After the wait the loop checks death (`$30` = 7/8/`$11`/`$13`): `$0133` counts down, at 0 the stage restarts (`code_3EC541`).
- The random generator (`$E4-$E7`) is stepped by task 1 once per video frame; frame counters: `$92` (NMI), `$95` (draw pass).
- Long operations inside a logic frame (item refill, section scroll, stage restart) consume many video frames without logging a row; `$92` tells how many.
