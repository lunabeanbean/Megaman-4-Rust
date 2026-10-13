# Level (stage) data format

Derived from the column-streaming code (`code_3ECFB1`, `code_3ED17F`, `code_3ED295`, `code_3ED2CE`, bank38_3F.asm lines ~21420-21850)
and **validated by rendering stage `$02` from the ROM** (`harness/render_stage.ps1` -> `stage_types.png`): the picture is a
recognisable level with a flat start, pits, ladders and stairs, so the layout below is right for tile/collision data.

## Where it lives
- Each stage has its own 8 KB PRG bank, selected by `$F6 = stageId | $20` and mapped at CPU `$A000-$BFFF`.
  ROM file offset of stage S = `0x10 + (0x20 | S) * 0x2000` (iNES header 16 bytes). Stage `$02` = bank `$22` = offset `0x44010`.
  (These banks are in `bank21_34.asm`.)

## Layout of a stage bank (addresses are CPU addresses when the bank is mapped)
| Range | Contents | Conf |
|---|---|---|
| `$A000`..`$A0FF` | Metatile *top-left* 8x8 tile id, indexed by metatile id (0-255) | C |
| `$A100`..`$A1FF` | top-right tile id | C |
| `$A200`..`$A2FF` | bottom-left tile id | C |
| `$A300`..`$A3FF` | bottom-right tile id | C |
| `$A400`..`$A4FF` | Per-metatile flags: **low 2 bits = palette/attribute**, **high nibble = collision/terrain type** (0 = empty, 1 = solid; types 2,3,4,5,7 also used in stage 2: ladder/hazard/slope-like, not yet identified) | C (bits) / ? (meaning) |
| `$A500`..`$A8FF` | **Blocks**, 4 bytes each: four metatile ids (TL, TR, BL, BR) = one 32x32-pixel block | C |
| `$A900`.. | **Screen layouts**, 64 bytes each: an 8x8 grid of block ids, row-major (cell = row*8 + col). Layout `n` is at `$A900 + n*64` | C |
| `$B500`.. | **Screen order table**: byte `s` = layout id used for screen `s` of the stage (`$2A` = current screen index) | C |
| `$B530`.. | Section table (16 entries): low nibble = number of screens in the section (`$2D`), bits 4-5 and the rest = flags; see `code_3ECAFF`-`code_3ECB1B` | L |
| `$B540`.. | Transition table, 4 bytes per entry (screen number, flags, ...), used when a section ends (doors / vertical transitions) | ? |
| `$B550`, `$B560` | Per-section index into the 16-byte palette/CHR sets below and another per-section value | ? |
| `$B590`.. | 16-byte sets copied to `$0620-$062F` (background palette work area), and `$B5A0`.. CHR bank numbers copied to `$06F0-$06F3` | L |
| `$B300`.. | Table indexed by section/screen (loaded into `$9E/$9F`), likely scroll/limit data | ? |
| enemies | Enemy placement tables not located yet (probably later in the same bank, `$B600+`) | - |

A screen is 256 px wide: 8 blocks across, 8 blocks (256 px) down, of which 15 metatile rows (240 px) are visible.
New columns are streamed into the nametable as the camera crosses 8-pixel boundaries (`$24` = nametable column 0-31,
`$2A` = screen index, `$29` = cell index within the layout). VRAM writes are queued in `$0780`+ and attributes in `$0640`.

## Decoding a tile (what a tool must do)
    layout   = bank[0xB500 + screen]
    block    = bank[0xA900 + layout*64 + row*8 + col]            # 32x32 block
    metatile = bank[0xA500 + block*4 + k]    k = 0 TL, 1 TR, 2 BL, 3 BR   # 16x16
    tiles    = bank[0xA000+m], bank[0xA100+m], bank[0xA200+m], bank[0xA300+m]  # TL TR BL BR 8x8
    palette  = bank[0xA400+m] & 3 ;  type = bank[0xA400+m] >> 4

## Tools
`harness/render_stage.ps1 -Stage 2 -Screens 23` draws the collision-type map (each type a different colour) to `harness/stage_types.png`.
Screens count (23 for stage 2) was read off the screen table (ids 0..22 then padding); the real count comes from the section table.

## Still to do (for the level editor)
Enemy/item placement, the exact meaning of terrain types 2-7, section/door transitions (`$B530/$B540`), and the CHR/palette
data to render with real graphics.
