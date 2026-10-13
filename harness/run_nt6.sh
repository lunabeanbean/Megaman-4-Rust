export MM4_ANYY=1
rec() { seed=$1; stage=$2; rom=$3; frames=$4; shift 4; env STAGE=$stage KEEP=1 NTEVERY=4 NTDUMP=30 JOBS=1 "$@" sh game_test.sh $rom $frames $seed > /tmp/nt_$seed.rec 2>&1; }
rec 22501 5 mm4_orig.nes 3000 TPSECT=3,12,163,4 TPCUR=1c TELEPORT=12,40,205 TPWAIT=8 UP=0.9 B=0.6
rec 22502 5 mm4_orig.nes 3000 TPSECT=3,12,35,4 TPCUR=1c TELEPORT=13,128,50 TPWAIT=8 B=0.6
rec 22503 5 mm4_orig.nes 3000 TPSECT=3,12,35,4 TPCUR=1c TELEPORT=15,128,50 TPWAIT=8 B=0.6 RIGHT=0.3
echo done > /tmp/nt_all6.done
