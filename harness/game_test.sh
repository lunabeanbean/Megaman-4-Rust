#!/bin/sh
# Whole-game differential test (player + camera + objects): record the real game with random inputs on a ROM variant, replay the object system in Rust, compare every frame.
#   sh game_test.sh <rom in this folder, e.g. mm4_only00.nes> <frames> <seed...|from to>
# Env: TELEPORT=page,x,y starts Mega Man somewhere else (see gen_ptrace.js); DOWN/UP as in random_trace.js.
# Mega Man is made invincible in the recording unless NOINV=1 is set (then hurt/knock-back is tested too; death still stops the comparison).
# JOBS=n runs n seeds at the same time (several headless Mesen instances; default 1). KEEP=1 keeps the traces of passing seeds.
# BOSSHP=n sets the stage boss's hit points to n when it appears (recorder and replay both do it), to reach its defeat quickly.
# Passing seeds append the routines they ran to verified_routines.txt (used by tools/checklist.js); failing ones keep ptrace_r<seed>.txt/otrace_r<seed>.txt:
#   ../port/target/release/mm4.exe --stage 2 --rom <rom> --verify-game ptrace_r<seed>.txt
ROMNAME=$1; FRAMES=$2; shift 2
# stage 4: also record the bar slots $0120-$0123 and the bar bitmap $0680-$06BF (the verifier starts from the first row)
S4ADDRS=""; if [ "${STAGE:-2}" = 4 ]; then S4ADDRS=$(node -e 'let a=[120,121,122,123];for(let i=0x680;i<0x6C0;i++)a.push(i.toString(16));console.log(","+a.join(","))'); fi
S6ADDRS=",43,44,EF"; [ "${STAGE:-2}" = 4 ] && S6ADDRS=""; [ "${STAGE:-2}" = 8 ] && S6ADDRS=",43,44,EF,A3,A5,A6,A7,A8$(node -e 'let a=[];for(let i=0x680;i<0x6C0;i++)a.push(i.toString(16));console.log(","+a.join(","))')"
export RAMADDRS=${RAMADDRS:-1B,1C,F0,F1,FA,FD,13E,13F$S6ADDRS$S4ADDRS}   # scroll registers at every row (xtrace_*.txt), the verifier starts from the first row
ROM=$(cygpath -w "$PWD/$ROMNAME")
MM4=../port/target/release/mm4.exe
JOBS=${JOBS:-1}
if [ $# -eq 2 ] && [ "$2" -gt "$1" ] && [ "$2" -ge 10 ]; then SEEDS=$(seq "$1" "$2"); else SEEDS="$@"; fi
if [ -n "$TELEPORT" ] && [ -z "$TPSECT" ]; then export MM4_CAM_TRACE=1; fi
if [ -n "$TPSECT" ]; then export MM4_SECT2B=$(printf %x $(echo $TPSECT | cut -d, -f3)); fi
if [ -n "$TPDOORS" ]; then export MM4_DOORS=$TPDOORS; fi
if [ -n "$BOSSHP" ]; then export MM4_BOSSHP=$BOSSHP; fi
if [ -n "$NOINV" ]; then export MM4_HURT=1; else export INVINCIBLE=1; fi
run_seed() {
  seed=$1; log="game_test_$seed.log"
  OBJ=1 node random_trace.js "$seed" "$FRAMES" >/dev/null || { echo "seed $seed: FAIL (trace generator)" > "$log"; return 2; }
  powershell -ExecutionPolicy Bypass -File rt.ps1 "ptrace_r$seed.lua" "$ROM" >/dev/null
  out=$($MM4 --stage ${STAGE:-2} --rom "$ROMNAME" --verify-game "ptrace_r$seed.txt" 2>&1 | tail -n +2)
  if ! echo "$out" | grep -q -E "^(all .* frames identical|stopped at|stage clear at)"; then
    { echo "seed $seed: FAIL"; echo "$out"; } > "$log"; return 1
  fi
  if [ -n "$NTEVERY" ]; then
    ntout=$(MM4_NTLOG="ntlog_r$seed.bin" MM4_NTINIT="ntdump_r$seed.bin" MM4_NTROW="${NTDUMP:-30}" MM4_NTLOG_MAX=20 $MM4 --stage ${STAGE:-2} --rom "$ROMNAME" --verify-game "ptrace_r$seed.txt" 2>&1 | grep "^NT row" | awk -v r="${NTDUMP:-30}" '{x=$3; sub(":","",x); if (x+0>r) print}' | head -${NTSHOW:-6})
    [ -n "$ntout" ] && echo "$ntout" > "nt_r$seed.txt" || echo "NT identical" > "nt_r$seed.txt"
  fi
  summary=$(echo "$out" | grep -E "^(all|stopped|stage clear|routines)" | tr '\n' ' ')
  echo "$out" | grep "^routines" | grep -oE "[0-9A-F]{4}x" | tr -d x >> verified_routines.txt
  echo "seed $seed: ok - $summary" > "$log"
  [ -z "$KEEP" ] && rm -f "ptrace_r$seed.lua" "ptrace_r$seed.txt" "otrace_r$seed.txt" "atrace_r$seed.txt" "xtrace_r$seed.txt" "strace_r$seed.txt"
  return 0
}
n=0
for seed in $SEEDS; do
  run_seed "$seed" &
  n=$((n+1))
  if [ $((n % JOBS)) -eq 0 ]; then wait; fi
done
wait
fail=0
for seed in $SEEDS; do
  cat "game_test_$seed.log"; grep -q "FAIL" "game_test_$seed.log" && fail=1
  rm -f "game_test_$seed.log"
done
exit $fail
