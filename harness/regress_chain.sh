#!/bin/sh
# Game-flow chains against the real game: a button script played from power-on in Mesen (harness/chain_rec.lua) and in the Rust flow (mm4 --chain), pictures compared frame by frame.
# Needs harness/chain/<name>/sh.hash from the recordings (see rec_chain.sh); no Mesen needed here. Run from harness/.  Prints one line per chain and a final count.
EXE=../port/target/release/mm4.exe
ROM="../Mega Man 4 (USA).nes"
ok=0; bad=0
# chain NAME FRAMES FROM TO [mm4 options...]   (FROM..TO = console frames compared; the Rust flow starts 6 frames after power-on, a screenshot shows the frame before: offset 7)
chain() {
  name=$1; frames=$2; from=$3; to=$4; shift 4
  mkdir -p chain/$name
  $EXE --rom "$ROM" --chain chain_$name.txt --frames $frames --lag 6 "$@" --shots 0-0 --out chain/$name >/dev/null 2>&1
  out=$(node ../tools/chaincmp.js chain/$name $from $to 7 | tail -1)
  case "$out" in
    *" 0 differ"*) ok=$((ok+1)); echo "ok  $name $from-$to: $(echo "$out" | sed 's/.*offset 7): //')";;
    *) bad=$((bad+1)); echo "BAD $name $from-$to: $out";;
  esac
}
# a: power-on, story skipped by Start, title, stage select, boss intro, stage 2 start, READY, teleport-in, idle, running (until the camera starts to scroll)
chain a 3500 8 2931
# n: the story runs to its end by itself (7063 frames), title, PRESS START, stage select
chain n 7700 7000 7700
# c: PASS WORD entry typing a password, accepted, stage select with those weapons, stage 0 boss intro (stage 0's animated palette is not drawn: compared up to its start)
chain c 4200 8 3088
# b: forced stage clear in stage 2 (hijack), fade, weapon get, stage-clear menu, STAGE SELECT, select, boss intro of stage 6 (compared up to its stage start)
chain b 7000 8 4934 --clear-at 2494
# d/e: forced game over (state 7 poked: the dying sprite of the poke is not drawn: 2501-2514 skipped), menu, STAGE SELECT / CONTINUE (stage fades in again)
chain d 4800 8 2500 --gameover-at 2494
chain d 4800 2515 4790 --gameover-at 2494
chain e 4800 8 2500 --gameover-at 2494
chain e 4800 2515 4790 --gameover-at 2494
# f: all eight weapons by password, centre portrait, Dr. Cossack's castle, stage 8
chain f 6000 8 5990
# g: the same, stage 8 cleared (forced), map after it, stage 9
chain g 8000 8 7990 --clear-at 4494
# h: $AA = 07 poked, stage 11 cleared (forced): the flight to Wily's fortress, the fortress map, stage 12
chain h 10500 8 10490 --aa 2294:07 --clear-at 4494
# q: stage 2, charge shots (full, medium, tap): the charge flash and the palette steps after a release (up to 3683: then an enemy touches Mega Man and the random numbers of the Rust flow differ)
chain q 3700 2900 3683
echo "$ok ok / $bad bad"
