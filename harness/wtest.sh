#!/bin/sh
# Weapon differential test: record the real game with weapon $1 selected (energy $2 hex) under random inputs (B held half the time) on stage 2, replay in Rust.
#   sh wtest.sh <weapon> <energy hex> <frames> <seeds...>        e.g. sh wtest.sh 10 9C 900 2010 2011
W=$1; EN=$2; FR=$3; shift 3
IDX=$(printf '%02X' $((0xB0 + W)))
export POKET=699 POKE=A0=$(printf "%02X" $W),$IDX=$EN,0131=$(printf "%02X" $((0x80 + W))) B=${B:-0.5} KEEP=${KEEP:-1} MM4_WEAPON=$W MM4_WEN=$EN MM4_HUD=1 JOBS=${JOBS:-3}
exec sh game_test.sh ${ROMN:-mm4_orig.nes} $FR "$@"
