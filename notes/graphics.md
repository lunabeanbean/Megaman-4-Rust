# Graphics: CHR-RAM, the loader, palettes

**Mega Man 4 has no CHR ROM.** The iNES header written by the repo (`20 00 40`) already says 512 KB PRG / 0 CHR. All tile graphics are stored in PRG
banks and copied into CHR-RAM (PPU `$0000-$1FFF`) by a loader, then the MMC3 CHR registers choose which 1 KB pieces are visible.
At stage start the registers were `R0..R5 = 0,2,4,5,6,7` (the reset defaults from `$FFAD`), `chrMode 0`: BG pattern table = `$0000-$0FFF`,
sprites = `$1000-$1FFF` (`PPUCTRL $FF = $88`).

## The loader (`code_3EDA05`, line 22548) - emulated in `tools/gfx.js`
`X` = graphics set id (the stage id is used as the id; `$47` can name a second set, `$C8A7` table). Scripts live in bank `$28`
(mapped at `$A000`): script pointer = `$B800+id` (low) / `$B870+id` (high). Script format:
1. Up to 8 palette bytes (stops at a byte >= `$80`) -> copied to `$0618-$061F` and `$0638-$063F` (sprite palettes 2-3 for this set).
2. Entries `[bank, pages, srcHigh, ppuHigh]`: copy `pages*256` bytes from CPU address `srcHigh*256` (with PRG banks `bank`,`bank+1` mapped at `$8000/$A000`) to PPU `ppuHigh*256`.
3. A first byte >= `$80` ends the list.
Stage `$02` example: 4 KB of background tiles from bank `$10` `$A000` -> PPU `$0000`; sprite sets from banks 0, 4, `$1C`, 6 -> PPU `$1000` and up.
`code_3EDA8B` / `code_3EDABB` load animated graphics (bank `$29`, same pointer scheme at `$B800/$B860`).

**Verified:** emulating set 2 reproduces every byte the game wrote to CHR-RAM in Mesen (100% of the 6 ranges).

## Palettes
- BG palettes (4 x 4 colours) for a stage come from its bank at `$B590 + 20*$48` (`$48` = palette-set index, from the section table; 0 at stage start);
  they are copied to `$0620-$062F` (the working copy, `$0600` is what the NMI uploads).
  Stage 2: `0F 30 10 18 | 0F 37 26 16 | 0F 00 0B 07 | 0F 26 16 07` = what Mesen showed.
- Sprite palettes: `$0630-$0637` from table `$C853`, plus the 8 bytes from the graphics set script (and weapon/boss specifics later).
- Each 16x16 metatile uses BG palette `flags & 3` (flags byte at `$A400+m`), matching the NES attribute granularity.

## Tools
- `node tools/render_stage.js <rom> <stage> <out.png> [screens]` renders a whole stage with its real art. **Screen 0 of stage 2 is pixel-identical (0 of 61,440 pixels differ) to the emulator's nametable.**
  All 15 stages render; the number of screens is guessed from the screen-order table (consecutive ids) when not given, so a few stages
  (e.g. stage 3) come out too short: pass the count explicitly. Not handled yet: palette-set changes after sections, second graphics set `$47`,
  animated tiles and per-scanline CHR changes.
- `tools/gfx.js` (graphics loader emulation), `tools/nes.js` (PNG writer, NES colours, tile decoder), `tools/render_ppu.js` (draws a captured nametable).
- `harness/ppudump.lua` captures CHR, nametables, palette and MMC3 registers from a live game for comparisons.
