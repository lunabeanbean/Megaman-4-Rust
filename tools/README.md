# Tools

## stage.js - stage data <-> JSON (needs Node, no packages)
    node tools/stage.js export  <rom.nes> <stage 0-14> <out.json>
    node tools/stage.js import  <rom.nes> <stage.json> <out-rom.nes>      (writes a new ROM, never modifies the input)
    node tools/stage.js roundtrip <rom.nes> <stage>                        (export + import must give a byte-identical ROM)

The JSON has `metatiles` (256: four 8x8 tile ids, palette 0-3, terrain type 0-15), `blocks` (256 x four metatile ids),
`layouts` (32 x 64 block ids, an 8x8 grid), `screenOrder` (layout id per screen) and `enemies` (`page`, `x`, `y`, `type`).
Meanings are in `notes/level-format.md` and `notes/enemy-placement.md`. Everything outside those fields is left untouched on import.

Status:
- Round trip is byte-identical for all 15 stages (0-14) of the owner's ROM.
- Edit test: removed enemy 0 of stage 2 in the JSON, imported, ran the game in the headless harness: the first live enemy slot
  now held what used to be entry 1 (`listIdx=00`, 01:18, y `BA`). So edits in the JSON reach the game.
- Limits: at most 127 enemies per stage (the table has 128 slots and an `$FF` end marker); enemies should stay in screen (page) order;
  the screen/section tables (`$B530`..) and palettes/CHR are not exposed yet, so you can reshape tiles but not add whole new sections.
- Edits go into the **ROM**, not into the `.asm` source. The asm build (`xkas`) would overwrite them; to keep both, keep the JSON and re-import after each asm build.

## harness/
See `harness/README.md` (headless Mesen runs, stage renderer, enemy dump).

## Graphics tools (see notes/graphics.md)
    node tools/render_stage.js <rom.nes> <stage> <out.png> [screens]     real-art render of a whole stage (pixel-checked against Mesen for stage 2 screen 0)
    node tools/gfx.js <rom.nes> <set id...>                              dump what the game's graphics loader writes to CHR-RAM

## Reading the disassembly (session 3)
    node tools/asm.js 3BA558 60                       compact listing from a CPU address in bank38_3F.asm (address = bank byte + CPU address, e.g. 3C8130, 3FF413)
    node tools/ai_states.js "Mega Man 4 (USA).nes" 50   which routines ("states") an enemy AI id can reach (scans for stores into $0588/$05A0), plus the helpers they call
    node tools/ppm2png.js in.ppm out.png [scale]        PPM -> PNG (for `mm4 --shot`); no dependencies
(Bank 35 and the other `bankNN.asm` files are not covered by asm.js yet.)

## checklist.js (session 7)
    node tools/checklist.js "Mega Man 4 (USA).nes" 2     enemy types of a stage with AI start, port status (unported / ported / matches)
