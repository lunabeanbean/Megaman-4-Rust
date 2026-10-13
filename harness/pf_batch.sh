#!/bin/sh
# Runs ppu_frames.lua recordings in parallel. Usage: sh pf_batch.sh jobs.txt   (from harness/)
# Each non-empty line of jobs.txt: <out file name> [VAR=value ...]  (PF_* variables of ppu_frames.lua; values without spaces)
ROM="C:\Users\joele\Desktop\MegaMan4 Project\Mega Man 4 (USA).nes"
n=0
while read -r name rest; do
  [ -z "$name" ] && continue
  case "$name" in \#*) continue;; esac
  n=$((n+1))
  ( env PF_OUT="$name" $rest powershell -ExecutionPolicy Bypass -File rt.ps1 ppu_frames.lua "$ROM" > "/tmp/pfb_$name.log" 2>&1 ) &
done < "$1"
wait
for f in $(awk '$1!="" && $1 !~ /^#/ {print $1}' "$1"); do echo "$f: $(cat /tmp/pfb_$f.log | tr '\n' ' ') $(ls -la $f 2>/dev/null | awk '{print $5}')"; done
