#!/bin/sh
# Weapons on a ladder (stage 2, page 1 ladder, no enemies): seeds 93xxx, KEEP=1. Replay needs MM4_CAM_TRACE=1 (teleport without section data).
for w in 1 2 4 6 7 8 9 10 11 12 13; do
  b=$((93000 + w * 10))
  TELEPORT=1,216,70 TPWAIT=8 ROMN=mm4_noenemy.nes UP=0.5 DOWN=0.3 B=0.4 JOBS=3 sh wtest.sh $w 9C 900 $((b+1)) $((b+2)) $((b+3)) 2>&1 | cut -c1-110
done
