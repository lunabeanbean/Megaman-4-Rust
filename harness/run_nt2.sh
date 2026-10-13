export MM4_ANYY=1
rec() { seed=$1; stage=$2; rom=$3; frames=$4; shift 4; env STAGE=$stage KEEP=1 NTEVERY=4 NTDUMP=30 JOBS=1 "$@" sh game_test.sh $rom $frames $seed > /tmp/nt_$seed.rec 2>&1; }
rec 22104 1 mm4_s1_match.nes 3000 B=0.4 RIGHT=0.7
rec 22115 13 mm4_orig.nes 3000 B=0.4 RIGHT=0.6
echo done > /tmp/nt_all2.done
