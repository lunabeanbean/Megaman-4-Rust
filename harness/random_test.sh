#!/bin/sh
# Random differential test: for each seed, record the real game (Mesen, stage 2 without enemies) under random inputs,
# replay them in the Rust port and compare every frame. Run from this folder:  sh random_test.sh 1 2 3   (or: sh random_test.sh 1 20)
# Passing seeds delete their trace files; a failing seed keeps ptrace_r<seed>.lua/.txt so it can be replayed:
#   ../port/target/release/mm4.exe --stage 2 --verify ptrace_r<seed>.txt
NOENEMY='C:\Users\joele\Desktop\MegaMan4 Project\harness\mm4_noenemy.nes'
MM4=../port/target/release/mm4.exe
if [ $# -eq 2 ] && [ "$2" -gt "$1" ] && [ "$2" -ge 10 ]; then SEEDS=$(seq "$1" "$2"); else SEEDS="$@"; fi
fail=0
for seed in $SEEDS; do
  node random_trace.js "$seed" 1200 >/dev/null || exit 2
  powershell -ExecutionPolicy Bypass -File rt.ps1 "ptrace_r$seed.lua" "$NOENEMY" >/dev/null
  out=$($MM4 --stage 2 --verify "ptrace_r$seed.txt" 2>&1 | tail -n +2)
  if echo "$out" | grep -q MISMATCH; then
    echo "seed $seed: FAIL"; echo "$out"; fail=1
  else
    echo "seed $seed: ok - $(echo "$out" | head -1)"; rm -f "ptrace_r$seed.lua" "ptrace_r$seed.txt"
  fi
done
exit $fail
