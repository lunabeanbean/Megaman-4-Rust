#!/bin/sh
# Directed weapon test: sh wseq.sh <weapon> <energy hex> <frames> <name> "<lua seq>"
W=$1; EN=$2; FR=$3; NAME=$4; SEQ=$5
IDX=$(printf '%02X' $((0xB0 + W)))
export POKET=699 POKE=A0=$(printf "%02X" $W),$IDX=$EN,0131=$(printf "%02X" $((0x80 + W))) INVINCIBLE=1 OBJ=1 RAMADDRS=${RAMADDRS:-1B,1C,F0,F1,FA,FD,13E,13F,43,44,EF}
node gen_ptrace.js $NAME $FR "$SEQ" >/dev/null
ROMW=$(cygpath -w "$PWD/${ROMN:-mm4_orig.nes}")
powershell -ExecutionPolicy Bypass -File rt.ps1 ptrace_$NAME.lua "$ROMW" >/dev/null
MM4_WEAPON=$W MM4_WEN=$EN MM4_HUD=1 ${MM4_EXE:-../port/target/release/mm4.exe} --stage ${STAGE:-2} --rom ${ROMN:-mm4_orig.nes} --verify-game ptrace_$NAME.txt 2>&1 | tail -n +2
