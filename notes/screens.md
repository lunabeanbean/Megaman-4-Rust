# Screens drawn from the ROM (session 21)

How: a model (`mm4-core/src/*_screen.rs`) rebuilds nametable / palette / OAM / pattern RAM exactly the way the bank-`$39` code does and is compared per frame with a Mesen recording.
- Recorder: `harness/ppu_frames.lua` (env PF_*, see its header), parallel batches `sh harness/pf_batch.sh jobs.txt` (one line per recording: `out.txt VAR=value ...`), reader `node tools/pf.js file info|png|diff`, Rust reader `pfrec.rs` (`compare` checks palette, OAM, the displayed nametable and the pattern RAM of the tiles actually used).
- Stage select recorder: `harness/select_frames.lua` (title -> select with the menu script, `SF_A9`, `SF_WALK`).
- Forcing a stage clear: `PF_HIJACK=720` pokes `$30 = 8`, `$0133 = 1`; scene numbers seen: `$11` = stage-clear/weapon-get/password scenes, `$10` = menu scenes, `$02` etc. = stage id while the stage-end code runs.
- Frame timeline conventions: a screen's frame 0 is the first frame with the new `$22`; palette fades (`code_3EC451`) = four steps of four frames (`$30,$20,$10,0` subtracted, negative -> `$0F`), the first step visible two frames after the buffer/palette is written; NMI buffers (`$0780`, `$19`) reach the nametable one frame after they are built.
- Pattern RAM leftovers: the intro loads set `$55` first (Mega Man's sprite tiles at `$1000-$1FFF`), so tiles a menu set does not overwrite are Mega Man's, then the previous scene's.
- The column builder (`code_3ECFB1`, 32 frames with a black palette) is not modelled: the nametable is final from the start; comparisons skip the nametable/pattern RAM while the palette is all black.
- Bank `$39` map: stage select `code_39809D/$80B7`, cursor `code_398237`, scene loader `code_398495` (`$884E` set per `$10`), weapon get `code_398CBC..`, stage clear menu `code_398E78..398EDE`, game over `code_398EF7`, text printer `code_399281` (typed, 10 frames per character) / `code_3992C9` (whole buffer), stage intro `code_398F93`, password grid `code_399886`, Cossack/Wily map `code_39833C/3983E8`.
- Texts: tile = `$80 + letter index` (A = `$80`), `$9A` = space, records `[hi, lo, count-1, tiles...]`, `$FF` ends (table `$952E`, starts `$9529`).
