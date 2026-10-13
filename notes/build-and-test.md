# Build and test

Build (from megaman4-disassembly/):
    ./xkas.exe -o mm4.nes assemble.asm
Output is iNES, body identical to the owner's ROM when unedited.

Backup of the unedited build: ../mm4_baseline_backup.nes (private, do not publish).

## First edit (Phase 1)
bank38_3F.asm line ~20299 (CPU $3EC687, stage start): `LDA #$9C` -> `LDA #$90` (sets $B0 = player life to 16 units instead of 28).
Confirmed in Mesen (Luna): starts with reduced life.
Revert: change back to #$9C, rebuild, `cmp` against the backup.
