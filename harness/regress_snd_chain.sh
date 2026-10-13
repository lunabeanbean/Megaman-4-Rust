#!/bin/sh
# Sound requests of the game-flow chains against Mesen (chain_snd/<name>/snd.log from rec_snd_all.sh): the Rust flow runs the same button script, tools/sndcmp.js compares the two request lists.
# From harness/:  sh regress_snd_chain.sh [name...]
EXE=../port/target/release/mm4.exe
ROM="../Mega Man 4 (USA).nes"
want() { [ $# -le 1 ] && return 0; n=$1; shift; for w in "$@"; do [ "$w" = "$n" ] && return 0; done; return 1; }
chain() { # name frames [mm4 options...]
  name=$1; frames=$2; shift 2
  want $name $NAMES || return
  [ -f chain_snd/$name/snd.log ] || { echo "$name: no recording (sh rec_snd_all.sh)"; return; }
  mkdir -p /tmp/sndchain/$name
  $EXE --rom "$ROM" --chain chain_$name.txt --frames $frames --lag 6 "$@" --shots 0-0 --out /tmp/sndchain/$name >/dev/null 2>&1
  echo "$name: $(node ../tools/sndcmp.js chain_snd/$name/snd.log /tmp/sndchain/$name/sound.log | tail -${TAILN:-1})"
}
NAMES="$@"
chain a 3500
chain n 7700
chain c 4200
chain b 7000 --clear-at 2494
chain d 4800 --gameover-at 2494
chain e 4800 --gameover-at 2494
chain f 6000
chain g 8000 --clear-at 4494
chain h 10500 --aa 2294:07 --clear-at 4494
chain q 3700
