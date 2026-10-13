export MM4_ANYY=1
rec() { seed=$1; stage=$2; rom=$3; frames=$4; shift 4; env STAGE=$stage KEEP=1 NTEVERY=4 NTDUMP=30 JOBS=1 "$@" sh game_test.sh $rom $frames $seed > /tmp/nt_$seed.rec 2>&1; }
rec 22601 10 mm4_orig.nes 5000 TPSECT=4,16,52,0 TPCUR=3b TELEPORT=16,20,100 TPWAIT=8 RIGHT=0.5 B=0.9 BOSSHP=6
echo done > /tmp/nt_all7.done
