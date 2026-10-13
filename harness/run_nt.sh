#!/bin/sh
# NT survey: each recipe is recorded with NTEVERY (console nametable every 4 rows) and compared by game_test.sh (nt_r<seed>.txt). Seeds 22100+.
export MM4_ANYY=1
rec() { # seed stage rom frames env...
  seed=$1; stage=$2; rom=$3; frames=$4; shift 4
  env STAGE=$stage KEEP=1 NTEVERY=4 NTDUMP=30 JOBS=1 "$@" sh game_test.sh $rom $frames $seed > /tmp/nt_$seed.rec 2>&1
}
rec 22101 4 mm4_orig.nes 3000 TPSECT=4,5,36,0 TPCUR=12 TELEPORT=5,144,100 TPWAIT=8 B=0.5 RIGHT=0.8 A=0.6
rec 22102 4 mm4_orig.nes 3500 TPSECT=8,12,40,6 TPCUR=31 TELEPORT=12,128,60 TPWAIT=8 B=0.4 RIGHT=0.6
rec 22103 8 mm4_orig.nes 3500 B=0.4 RIGHT=0.6
rec 22104 1 mm4_s1_match.nes 3000 B=0.4 RIGHT=0.7
rec 22105 2 mm4_orig.nes 2500 TPDOORS=16,17,18,19 TPCUR=38 TPSECT=8,21,56,0 TELEPORT=21,40,139 TPWAIT=8 RIGHT=0.7 B=0.85
rec 22106 5 mm4_orig.nes 3000 B=0.4 RIGHT=0.7
rec 22107 3 mm4_orig.nes 3000 B=0.4 RIGHT=0.7
rec 22108 6 mm4_s6_match.nes 3000 B=0.4 RIGHT=0.7
rec 22109 7 mm4_orig.nes 3000 B=0.4 RIGHT=0.7
rec 22110 0 mm4_orig.nes 3000 B=0.4 RIGHT=0.7
rec 22111 9 mm4_orig.nes 3000 B=0.4 RIGHT=0.6
rec 22112 10 mm4_orig.nes 3000 B=0.4 RIGHT=0.6
rec 22113 11 mm4_orig.nes 3000 B=0.4 RIGHT=0.6 DOWN=0.4
rec 22114 12 mm4_orig.nes 3000 B=0.4 RIGHT=0.6 UP=0.4
rec 22115 13 mm4_orig.nes 3000 B=0.4 RIGHT=0.6
echo done > /tmp/nt_all.done
