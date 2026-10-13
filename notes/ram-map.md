# RAM map (Mega Man 4, NES)

Derived by reading the disassembly (bank38_3F.asm). Confidence: **C** = confirmed by
code or by a test in Mesen, **L** = likely (inferred from usage), **?** = guess.
Verify L/? entries in Mesen (Debugger > Watch, or a write breakpoint) before relying on them.

## Object slots
Per-object arrays are spaced `$18` apart, indexed by slot X. **Slot 0 = Mega Man.**
Enemies, shots and items use the other slots. Slot N lives at `base + N`.

| Address | Meaning | Conf |
|---|---|---|
| `$0300` | Object type id (`$01` = Mega Man; `$81` seen during the spawn animation) | C |
| `$0318` | X sub-pixel | C |
| `$0330` | X position, pixel | C |
| `$0348` | X position, screen/page (high byte) | C |
| `$0360` | Y sub-pixel | C |
| `$0378` | Y position, pixel (wraps at `$F0`, then `$0390` changes) | C |
| `$0390` | Y position, screen (high byte) | C |
| `$03A8` | X speed setting, sub-pixel (player: run speed `$4C`, stays set when idle) | C |
| `$03C0` | X speed setting, pixel (player: `$01`) | C |
| `$03D8` | Y speed, sub-pixel (see notes/physics-traces.md) | C |
| `$03F0` | Y speed, pixel. **Player: signed, positive = UP, negative = down** (enemy code may differ) | C |
| `$0408` | Collision/hitbox id (`& $3F` indexes table `$F9F4`) | L |
| `$0420` | Flags; bit0 = facing/direction for movement | L |
| `$0468`, `$0480`, `$0498`, `$04B0`, `$04C8` | Per-object scratch/timers (meaning varies by object type) | ? |
| `$0528` | Object flags (`$90` = active; bit6 = horizontal flip) | L |
| `$0540`, `$0570` | Animation frame / timer (cleared by `code_3FF446`) | L |
| `$0558` | Animation / sprite id (set by `code_3FF446`) | L |
| `$0588`, `$05A0` | Sprite/attribute and related per-object data (enemy init writes `$A8..$B1` here) | ? |

Movement code: `code_3FF22D` (move right), `code_3FF24B` (left), `code_3FF269` (down),
`code_3FF289` (up). Each adds speed to position with carry into the screen byte.

## Zero page
| Address | Meaning | Conf |
|---|---|---|
| `$14` | **Controller 1 buttons newly pressed this frame** (`held & ~previous`, code_3EC3AB). Bits: A=`$80`, B=`$40`, Select=`$20`, Start=`$10`, Up=`$08`, Down=`$04`, Left=`$02`, Right=`$01` | C |
| `$15` | Controller 2 buttons held | C |
| `$16`, `$17` | **Buttons currently held** for P1 / P2 (the raw read). Walking, the variable jump height and the A-release check use `$16`; jumping uses `$14` (new press) | C |
| `$22` | **Current scene/stage ID.** Stages are low numbers (`$0E` is checked as the final stage); non-stage screens use `$10`+. Observed in Mesen (Luna): `$10` = Capcom/license screens AND stage select, `$11` = title/password screen, `$13` = story intro cutscene, `$02` = boss-intro cutscene after picking a stage (so `$02` is the chosen stage's ID). Still to map: which robot master is which ID. | C |
| `$65` | Y scroll written to PPU (`$2005` second write); copy of `$FA` made in NMI | C |
| `$66` | X scroll written to PPU (`$2005` first write); copy of `$FC` made in NMI | C |
| `$67` | Low 2 bits = nametable select (OR'd into `$2000`) | C |
| `$FE` | PPUMASK (`$2001`) mirror | C |
| `$FF` | PPUCTRL (`$2000`) mirror | C |
| `$30` | **Player state** (0 = ground, 2 = slide; index into the state-handler table at `$3C80FA`) | C |
| `$99` | Gravity per frame (sub-pixel; `$40` normal, `$15` in some mode) | C |
| `$9A` | NMI/update mode flag (`0` = skip drawing path) | ? |
| `$93`, `$97` | Frame-sync / task-switch arguments used by `code_3FFF1C`, `code_3FFCE3` | ? |

## Health and energy meters (`$B0`+)
Value `$80 | units`. `$9C` = full (28 units). Bit 7 appears to mean "present/unlocked".
| Address | Meaning | Conf |
|---|---|---|
| `$B0` | **Mega Man life** (set to `$9C` at stage start, `$3EC687`) | C |
| `$B1` | Weapon energy slot (set `$9C` in `code_3EC53D` area) | L |
| `$B2`..`$BE` | Other weapon energy slots (stage-select refill loop at `$398D2E` fills those with bit 7 set) | L |
| `$A1` | Set to `2` when the game starts; probably extra lives | ? |

## Hardware / mapper
MMC3. `$8000/$8001` bank select, `$E000` IRQ disable, `$C000/$C001` IRQ latch (`$5C` = scanline).
Shadow of last bank-select value in `$F2`/`$F3`.

## Open questions
- Full list of player states for `$30` (walk, jump in air, slide, hurt, ladder, charge shot ...).
- Run-speed constant location (`$C78D` writes it every frame).
- Lives counter, E-tank count, screen scroll position within the stage (separate from `$65/$66`).

## Player extras (see notes/player-states.md)
`$30` state, `$31` facing (1 right, 2 left), `$34` slide timer, `$0558` anim id.

## Camera (see notes/camera.md)
`$FC` camera X low, `$F9` camera X high, `$FA` camera Y, `$FD` nametable bits, `$2E` camera screen index, `$2D` last screen,
`$27/$28` follow threshold, `$2F` scroll direction. Camera X = player X - `$80`, max 8 px/frame.
