#!/bin/sh
# Records the Mesen side of the game-flow chains (regress_chain.sh compares the Rust flow with it). From harness/:  sh rec_chain.sh [name...]   (default: all)
# Each chain = chain_<name>.txt (button script, console frames) + environment of chain_rec.lua; afterwards the screenshots are reduced to sh.hash and deleted.
ROM="C:\Users\joele\Desktop\MegaMan4 Project\Mega Man 4 (USA).nes"
H="C:/Users/joele/Desktop/MegaMan4 Project/harness"
rec() { # name frames shots [ENV=value ...]
  name=$1; frames=$2; shots=$3; shift 3
  rm -rf chain/$name; mkdir -p chain/$name
  env CH_DIR=chain/$name CH_FRAMES=$frames CH_SHOTS=$shots CH_SCRIPT="$H/chain_$name.txt" "$@" powershell -ExecutionPolicy Bypass -File rt.ps1 chain_rec.lua "$ROM"
  node ../tools/chaincmp.js chain/$name hash && rm -f chain/$name/sh_*.png
}
want() { [ $# -eq 0 ] && return 0; for w in "$@"; do [ "$w" = "$NAME" ] && return 0; done; return 1; }
NAMES="$@"
w() { NAME=$1; shift; if [ -z "$NAMES" ] || echo " $NAMES " | grep -q " $NAME "; then rec "$NAME" "$@"; fi; }
w a 3500 0-3499
w n 7700 7000-7700
w c 4200 1000-4199
w b 7000 2400-6999 CH_HIJACK=2500
w d 4800 2400-4799 CH_HIJACK=2500 CH_STATE=7 CH_POKE=A1=00
w e 4800 2400-4799 CH_HIJACK=2500 CH_STATE=7 CH_POKE=A1=00
w f 6000 1000-5999
w g 8000 4000-7999 CH_HIJACK=4500
w h 10500 4000-10499 CH_EARLY=2300:AA=07 CH_HIJACK=4500
w p 4500 3000-4499 CH_PAL=1 CH_RAM=6F0,6F1,6F2,6F3,6F4,6F5,6F6,6F7,6F8,6F9,2B,135,F9
w q 3700 2900-3699
