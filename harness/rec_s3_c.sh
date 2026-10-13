#!/bin/sh
# Stage 3 pages 25-31 (session 22). Section 7 (pages 26-31): TPSECT=7,26,103,5 (MM4_SECT2B=67), TPCUR per page 26..31 = 2c 2e 30 32 34 34; section 6 (page 25): TPSECT=6,25,70,0 TPCUR=2c (MM4_SECT2B=46).
export MM4_ANYY=1
r7() { page=$1; cur=$2; seed=$3; shift 3; env MM4_SECT2B=67 STAGE=3 KEEP=1 JOBS=1 TPSECT=7,26,103,5 TPCUR=$cur TELEPORT=$page,128,60 TPWAIT=8 B=0.4 RIGHT=0.3 DOWN=0.3 UP=0.2 "$@" sh game_test.sh mm4_orig.nes 3000 $seed; }
r6() { seed=$1; shift; env MM4_SECT2B=46 STAGE=3 KEEP=1 JOBS=1 TPSECT=6,25,70,0 TPCUR=2c TELEPORT=25,128,60 TPWAIT=8 B=0.4 RIGHT=0.3 DOWN=0.3 "$@" sh game_test.sh mm4_orig.nes 3000 $seed; }
r7 26 2c 2601; r7 27 2e 2602; r7 28 30 2603; r7 29 32 2604; r7 30 34 2605; r7 31 34 2606; r7 28 30 2607 DOWN=0.5 UP=0.1; r6 2611; r6 2612 DOWN=0.6
# hurt (NOINV=1): 2621-2626 (RIGHT=0.3 as above), 2631-2639 (RIGHT=0.4); these found the bug where the conveyor's lowered speed outlived the slide start
r7 26 2c 2621 NOINV=1; r7 27 2e 2622 NOINV=1; r7 28 30 2623 NOINV=1; r7 29 32 2624 NOINV=1; r7 31 34 2625 NOINV=1; r6 2626 NOINV=1
