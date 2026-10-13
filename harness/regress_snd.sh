#!/bin/sh
# Sound-request check: replays every recording that has a strace_r<seed>.txt (recorded by gen_ptrace.js OBJ=1 with sndreq_snippet.lua) and compares the sound requests of each
# logic frame with the console's. Recipes come from the run_nt*.sh files (seeds 22100+) plus the older recordings. Prints one line per seed; SNDMAX=n shows n differing rows.
MM4=../port/target/release/mm4.exe
tot_ok=0; tot_bad=0
chk() { # seed stage rom frames env...
  seed=$1; stage=$2; rom=$3; shift 4
  [ -f "strace_r$seed.txt" ] || return
  res=$(
    tele=""; unset MM4_SECT2B MM4_DOORS MM4_BOSSHP MM4_HURT MM4_CAM_TRACE
    for kv in "$@"; do
      case $kv in
        TPSECT=*) export MM4_SECT2B=$(printf %x $(echo ${kv#*=} | cut -d, -f3));;
        TPDOORS=*) export MM4_DOORS=${kv#*=};;
        BOSSHP=*) export MM4_BOSSHP=${kv#*=};;
        NOINV=*) export MM4_HURT=1;;
        TELEPORT=*) tele=1;;
      esac
    done
    [ -n "$tele" ] && [ -z "$MM4_SECT2B" ] && export MM4_CAM_TRACE=1
    export MM4_ANYY=1 MM4_SND_MAX=${SNDMAX:-3}
    $MM4 --stage "$stage" --rom "$rom" --verify-game "ptrace_r$seed.txt" 2>&1 | grep -E "^(SOUND|sound requests|MISMATCH)")
  echo "$stage/$seed: $(echo "$res" | grep -E '^(sound requests|MISMATCH)' | head -1)"
  echo "$res" | grep "^SOUND" | sed 's/^/    /'
}
rec() { chk "$@"; }
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
rec 22104 1 mm4_s1_match.nes 3000 B=0.4 RIGHT=0.7
rec 22115 13 mm4_orig.nes 3000 B=0.4 RIGHT=0.6
rec 22201 0 mm4_orig.nes 4000 TPSECT=8,26,56,0 TPCUR=35 TELEPORT=26,40,100 TPWAIT=8 RIGHT=0.6 B=0.7 BOSSHP=9
rec 22202 11 mm4_orig.nes 4000 TPSECT=10,19,58,0 TPCUR=24 TELEPORT=19,60,100 TPWAIT=8 RIGHT=0.5 B=0.5 BOSSHP=8
rec 22203 13 mm4_orig.nes 5000 TPSECT=11,21,59,0 TPCUR=1a TELEPORT=21,60,100 TPWAIT=8 RIGHT=0.5 B=0.9 BOSSHP=3
rec 22204 4 mm4_orig.nes 3500 TPSECT=12,24,60,0 TPCUR=46 TELEPORT=24,40,100 TPWAIT=8 RIGHT=0.7 B=0.85 BOSSHP=6
rec 22205 9 mm4_orig.nes 6000 TPSECT=8,16,56,0 TPCUR=28 TELEPORT=16,40,100 TPWAIT=8 RIGHT=0.5 B=0.9 BOSSHP=6
rec 22301 8 mm4_orig.nes 2500 TPSECT=2,7,162,5 TPCUR=11 TELEPORT=11,128,200 TPWAIT=8 B=0.3 UP=0.6
rec 22302 8 mm4_orig.nes 2500 TPSECT=3,13,131,1 TPCUR=16 TELEPORT=13,128,200 TPWAIT=8 B=0.3 UP=0.6
rec 22303 8 mm4_orig.nes 2500 TPSECT=4,15,132,0 TPCUR=19 TELEPORT=15,40,60 TPWAIT=8 B=0.3 UP=0.6 RIGHT=0.5
rec 22304 8 mm4_orig.nes 2500 TPSECT=5,16,165,0 TPCUR=1d TELEPORT=16,128,200 TPWAIT=8 B=0.3 UP=0.6
rec 22305 8 mm4_orig.nes 2500 TPSECT=6,17,54,0 TPCUR=20 TELEPORT=17,128,100 TPWAIT=8 B=0.3 RIGHT=0.6
rec 22306 8 mm4_orig.nes 6000 TPSECT=7,18,55,0 TPCUR=20 TELEPORT=18,40,100 TPWAIT=8 RIGHT=0.5 B=0.5
rec 22401 5 mm4_orig.nes 4500 TPSECT=7,22,55,0 TPCUR=27 TELEPORT=22,128,170 TPWAIT=8 BOSSHP=8 RIGHT=0.5 B=0.7
rec 22402 3 mm4_orig.nes 4500 TPSECT=8,32,56,0 TPCUR=34 TELEPORT=32,40,100 TPWAIT=8 RIGHT=0.7 B=0.85 BOSSHP=6
rec 22403 1 mm4_orig.nes 4500 TPSECT=11,27,59,0 TPCUR=2f TELEPORT=27,40,100 TPWAIT=8 RIGHT=0.7 B=0.85 BOSSHP=6
rec 22404 7 mm4_orig.nes 4500 TPSECT=8,26,56,0 TPCUR=3e TELEPORT=26,40,100 TPWAIT=8 RIGHT=0.7 B=0.85 BOSSHP=6
rec 22405 6 mm4_orig.nes 4500 TPSECT=11,24,59,0 TPCUR=46 TELEPORT=24,40,100 TPWAIT=8 RIGHT=0.7 B=0.85 BOSSHP=6
rec 22406 12 mm4_orig.nes 4500 TPSECT=7,20,55,0 TPCUR=22 TELEPORT=20,60,100 TPWAIT=8 RIGHT=0.5 B=0.5 BOSSHP=8
rec 22407 10 mm4_orig.nes 4500 TPSECT=3,15,51,0 TPCUR=24 TELEPORT=15,40,90 TPWAIT=8 RIGHT=0.9 B=0.5 BOSSHP=6
rec 22501 5 mm4_orig.nes 3000 TPSECT=3,12,163,4 TPCUR=1c TELEPORT=12,40,205 TPWAIT=8 UP=0.9 B=0.6
rec 22502 5 mm4_orig.nes 3000 TPSECT=3,12,35,4 TPCUR=1c TELEPORT=13,128,50 TPWAIT=8 B=0.6
rec 22503 5 mm4_orig.nes 3000 TPSECT=3,12,35,4 TPCUR=1c TELEPORT=15,128,50 TPWAIT=8 B=0.6 RIGHT=0.3
rec 22601 10 mm4_orig.nes 5000 TPSECT=4,16,52,0 TPCUR=3b TELEPORT=16,20,100 TPWAIT=8 RIGHT=0.5 B=0.9 BOSSHP=6
