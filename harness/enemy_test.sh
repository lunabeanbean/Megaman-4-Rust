#!/bin/sh
# Object/enemy differential test: record the real game with random inputs on a ROM variant, replay the object system in Rust, compare every frame.
#   sh enemy_test.sh <rom in this folder, e.g. mm4_only00.nes> <frames> <seed...|from to>
# Env: TELEPORT=page,x,y starts Mega Man somewhere else (see gen_ptrace.js); DOWN/UP as in random_trace.js.
# Mega Man is made invincible in the recording (the hurt state is not ported), so enemies can touch him freely.
# Passing seeds delete their traces; failing ones keep ptrace_r<seed>.txt/otrace_r<seed>.txt:
#   ../port/target/release/mm4.exe --stage 2 --rom <rom> --verify-obj ptrace_r<seed>.txt
ROMNAME=$1; FRAMES=$2; shift 2
ROM=$(cygpath -w "$PWD/$ROMNAME")
MM4=../port/target/release/mm4.exe
if [ $# -eq 2 ] && [ "$2" -gt "$1" ] && [ "$2" -ge 10 ]; then SEEDS=$(seq "$1" "$2"); else SEEDS="$@"; fi
fail=0
for seed in $SEEDS; do
  OBJ=1 INVINCIBLE=1 node random_trace.js "$seed" "$FRAMES" >/dev/null || exit 2
  powershell -ExecutionPolicy Bypass -File rt.ps1 "ptrace_r$seed.lua" "$ROM" >/dev/null
  out=$($MM4 --stage 2 --rom "$ROMNAME" --verify-obj "ptrace_r$seed.txt" 2>&1 | tail -n +2)
  if ! echo "$out" | grep -q -E "^(all .* frames identical|stopped at)"; then
    echo "seed $seed: FAIL"; echo "$out"; fail=1
  else
    summary=$(echo "$out" | grep -E "^(all|stopped|routines)" | tr '\n' ' ')
    echo "seed $seed: ok - $summary"; rm -f "ptrace_r$seed.lua" "ptrace_r$seed.txt" "otrace_r$seed.txt"
  fi
done
exit $fail
