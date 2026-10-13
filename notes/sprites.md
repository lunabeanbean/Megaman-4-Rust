# Sprites (OAM) - ported 2026-10-07 in `port/mm4-core/src/oam.rs`, `game.rs::sprites`, `stage.rs::draw_sprites`

Verified: `mm4 --verify-game` compares the **visible OAM entries of the real game** (logged at `$C7C6` into `atrace_*.txt`) with the Rust list every frame, in order
(HUD meter, Mega Man incl. blinking while invincible, shots, effects, enemies, flicker order). Random runs with shooting are identical.

## How an object becomes sprites (`code_3EDB3A` - `3EDE41`)
1. The draw pass visits all 24 slots; ascending order and the HUD meters first when `$95` is even after its increment, descending order and the HUD last when odd (this alternation is the NES sprite flicker).
2. An object is drawn when, after the despawn/visibility rules (notes/objects.md) and its animation tick, flag bit 7 is set, flag bit 2 is clear, its flash timer `$05B8` does not have bit 1 set (the flash blinks at 15 Hz), and the current animation frame's sprite id is not 0.
   Screen position: `x = $0330 - $FC` (objects with flag bit 4 clear use `$0330` directly), `y = $0378`.
3. Animation record (bank `$DFFA + type`, tables `$BE00/$BF00`): `[last frame, duration, sprite id per frame...]`. The record's first byte bit 7 picks the pointer tables `$B800/$BA00` (clear) or `$B900/$BB00` (set); the sprite id gives a **definition** `[parts-1, offset index, (tile, attribute) x parts]`.
   The sprite bank window is **two banks**: the type's bank at `$A000` and the bank before it at `$8000` (definitions can sit in either).
4. Offsets: `$BC00/$BD00` indexed by `offset index + (1 if the object is flipped)` point at `(dy, dx)` pairs, one per part. Position = base + offset (8-bit); a part whose add wraps the wrong way (negative offset without carry / positive with carry) is dropped and uses no OAM entry. Attribute = definition attribute XOR `$40` when flipped, OR `$20` for "behind background" (the object's own `$0528` bit 5, or every part below the line `$EF` = `$C87B + stage` once one part is below it).
5. OAM index starts at 1 (entry 0 is reserved for the status-bar split), at most 63 sprites; the rest are dropped.
6. HUD meters (`code_3EDF00`): life meter drawn when `$0130` bit 7 is set: seven tiles from y `$48` down to `$18` at x `$18`, tile `$80 + units in that cell (0-4)`, attribute 1 (second meter x `$10` attr 0, third x `$28` attr 2).

## Graphics data
Sprite tiles live in CHR-RAM `$1000-$1FFF` (pattern table 1) loaded by the graphics script (`gfx.rs` = `code_3EDA05`). Sprite palettes: Mega Man's two from `$C853`, the stage's two from the start of the graphics script (`GfxSet.palette`). Palette fades/commands and per-weapon palettes are not ported (colours may differ in stages that change palettes).
The window draws sprites with `Stage::draw_sprites` (priority: lower OAM index on top, behind-background sprites only over the backdrop colour); `--boxes` shows hitbox placeholders instead; `mm4 --shot out.ppm --frames N` writes a headless screenshot after a scripted run (`node tools/ppm2png.js` converts it).

## Not ported
8x16 sprites (unused so far), the special clipping mode of type `$0B`, the type-`$88` special case, the weapon-pickup flash sprites (`code_3EDF5F`), palette-flash variants of types `$24/$2B/$52`.
