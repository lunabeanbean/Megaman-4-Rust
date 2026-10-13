# Enemy placement and spawning

Found in `code_388000`-`code_38817E` (bank38_3F.asm lines 1-200, the start of bank `$38` at `$8000`) and **verified live**:
in a stage `$02` run, the objects that appeared in the enemy slots had the positions of entries 0, 1, 2 and 3 of the table below.

## Placement table (inside each stage's data bank, mapped at `$A000`; see notes/level-format.md)
Up to 128 entries, sorted by X (so the spawner can sweep through them as the camera moves). Index 0 is the first entry.
| Table | Meaning |
|---|---|
| `$B100`+i | X position, screen/page. `$FF` ends the list |
| `$B180`+i | X position, pixel within the screen |
| `$B200`+i | Y position, pixel |
| `$B280`+i | **Enemy type id** (0-`$BF`). Values `$C0`+ are not enemies: they are commands (`& $3F` indexes tables at `$84ED`/`$851D`/`$854D`, changing palettes or CHR banks as the camera passes) |
Stage `$02` has 57 entries (index 0 = page `01`, x `$10`, y `$70`, type `$00`). `harness/dump_enemies.ps1 -Stage N` lists any stage.

## Type tables (shared by every stage, in bank `$38` at `$8000`, 144 entries each)
Type `t` is used to fill a freshly spawned object's slot `x`:
| Table | Goes to | Meaning |
|---|---|---|
| `$817F`+t | `$0528,x` | Object flags (`$90` active, bit 6 = flip, `$D0` seen when flipped) |
| `$820F`+t | `$0300,x` | **Object type/behaviour id** (the AI routine). E.g. list type `$00` -> `$10`, `$3B` -> `$50` in stage 2 |
| `$829F`+t | `$0408,x` | Hitbox id |
| `$832F`+t | animation id (via `code_3FF446`) | Starting animation |
| `$83BF`+t | `$0450,x` | Unknown byte (probably hit points or contact damage) |
| `$844F`+t | index into `$84DF`/`$84E6` | Initial X speed (`$03A8`/`$03C0`) |

## Spawn logic (`code_388000`)
- Camera window: `$00/$01` = camera X (`$FC/$F9`); `$02/$03` = camera X + 255. `$9E` and `$9F` are two cursors into the sorted list
  (`$9F` = first entry still inside the window, `$9E` = first entry beyond its right edge). When the camera moves right
  (`$2F` bit 0) the right cursor advances and entries crossing the right edge are spawned; moving left, the left side does the same.
- `code_388080` spawns one entry: for type < `$C0` it first checks the 16 enemy slots (`$08`-`$17`) for the same list index in `$0438,x`
  (no duplicates), grabs a free slot with `code_3FFB5C`, and refuses if the entry's "defeated" bit is set.
- **`$0438,x` = list index of the entry that created the object** (`$FF` for objects not from the list, e.g. rain drops, shots).
- **`$0100`+(i/8), bit `i & 7`** (mask from `$FCBF`) = entry i already defeated/collected: it won't respawn.
- Slots: `$00` Mega Man, `$01`-`$07` player shots/effects, `$08`-`$17` enemies and other objects.
- The spawn writes X (`$0348/$0330`), Y (`$0378`), type, flags, hitbox, animation, speeds and zeroes the scratch fields
  (`$0468`...`$0510`).

## Notes for an editor / port
Adding an enemy = insert a row into the four tables in X order (keeping them sorted) and keep the `$FF` end marker; the type list
and AI routines are global. Type ids -> names are not mapped yet.
