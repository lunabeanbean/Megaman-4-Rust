#!/bin/sh
# Re-verify every kept recording (ptrace_r*.txt/otrace_r*.txt) with the current exe; prints one line per seed. Pass the stage with STAGE (default 2).
# Recordings made with MM4_HURT/doors/boss-hp setups need those env vars exported the same way as when they were recorded.
for f in ptrace_r*.txt; do
  s=${f#ptrace_r}; s=${s%.txt}
  [ -f "otrace_r$s.txt" ] || continue
  out=$(../port/target/release/mm4.exe --stage ${STAGE:-2} --rom ${ROM:-mm4_orig.nes} --verify-game "$f" 2>&1 | grep -E "^(all|stopped|stage clear|MISMATCH)" | head -1)
  echo "$s: $out"
done
