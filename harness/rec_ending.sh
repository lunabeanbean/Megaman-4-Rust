#!/bin/sh
# Re-records the late part of the ending. Mesen's test runner dies after ~100 s of wall time (about 11800 emulated frames), so the long recordings go in
# pieces through savestates (ppu_frames.lua: PF_SAVE=<recording frame> PF_SAVEAS=<name> writes <name>.state/.meta at the first NMI after that frame, PF_LOAD=<name> resumes there).
# From harness/:  sh rec_ending.sh      (then: mm4 --verify-ending pfe_6.txt / pfe_7.txt)
H="PF_HIJACK=720 PF_STATE=19"
printf 'pfs_x.txt %s PF_FROM=11000 PF_FRAMES=11001 PF_SAVE=11000 PF_SAVEAS=pfs_a\n' "$H" > /tmp/je1.txt; sh pf_batch.sh /tmp/je1.txt
printf 'pfe_6.txt %s PF_LOAD=pfs_a PF_FROM=11700 PF_FRAMES=14400 PF_SAVE=14400 PF_SAVEAS=pfs_b\n' "$H" > /tmp/je2.txt; sh pf_batch.sh /tmp/je2.txt
printf 'pfe_7.txt %s PF_LOAD=pfs_b PF_FROM=14400 PF_FRAMES=18000\n' "$H" > /tmp/je3.txt; sh pf_batch.sh /tmp/je3.txt
rm -f pfs_x.txt
