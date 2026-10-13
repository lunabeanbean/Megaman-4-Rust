#!/bin/sh
# Records only the sound requests of a game-flow chain (chain_<name>.txt, same environment as rec_chain.sh) into chain_snd/<name>/snd.log.
# From harness/:  sh rec_snd.sh <name> <frames> [ENV=value ...]       then:  mm4 --chain chain_<name>.txt ... --out DIR and  node ../tools/sndcmp.js chain_snd/<name>/snd.log DIR/sound.log
ROM="C:\Users\joele\Desktop\MegaMan4 Project\Mega Man 4 (USA).nes"
H="C:/Users/joele/Desktop/MegaMan4 Project/harness"
name=$1; frames=$2; shift 2
rm -rf chain_snd/$name; mkdir -p chain_snd/$name
env CH_DIR=chain_snd/$name CH_FRAMES=$frames CH_SHOTS=0-0 CH_SCRIPT="$H/chain_$name.txt" "$@" powershell -ExecutionPolicy Bypass -File rt.ps1 chain_rec.lua "$ROM"
rm -f chain_snd/$name/sh_*.png
