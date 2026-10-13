#!/bin/sh
# Only the special-weapon group of regress.sh and the pause screen (about 3 minutes): sh regress_w.sh
# Prints one line per seed and a final "N ok / M bad" count. Recordings are made by game_test.sh with KEEP=1 (see STATUS.md for the command lines).
MM4=../port/target/release/mm4.exe
ok=0; bad=0
check() { # stage rom seed
  out=$($MM4 --stage "$1" --rom "$2" --verify-game "ptrace_r$3.txt" 2>&1 | grep -E "^(all|stopped|stage clear|MISMATCH)" | head -1)
  case "$out" in
    all*|stopped*|"stage clear"*) ok=$((ok+1)); echo "$1/$3: ok - $(echo "$out" | cut -c1-90)";;
    *) bad=$((bad+1)); echo "$1/$3: BAD - $out";;
  esac
}
unset MM4_HURT MM4_DOORS MM4_BOSSHP MM4_ANYY MM4_CAM_TRACE
# special weapons (session 20, wtest.sh/wbatch.sh): stage 2 with enemies, weapon poked in, HUD meter shown; hurt mode for the 915xx-916xx groups, 3 units of energy for 918xx-919xx
export MM4_HUD=1
wr() { w=$1; en=$2; shift 2; for s in "$@"; do [ -f "ptrace_r$s.txt" ] && MM4_WEAPON=$w MM4_WEN=$en check 2 mm4_orig.nes $s; done; }
wr 1 9C 91011 91012 91013
wr 2 9C 91021 91022 91023
wr 4 9C 91041 91042 91043
wr 6 9C 91061 91062 91063
wr 7 9C 91071 91072 91073
wr 8 9C 91081 91082 91083
wr 9 9C 91091 91092 91093
wr 10 9C 91101 91102 91103
wr 11 9C 91111 91112 91113
wr 12 9C 91121 91122 91123
wr 13 9C 91131 91132 91133
wr 1 83 91811 91812
wr 2 83 91821 91822
wr 9 83 91891 91892
wr 13 83 91931 91932
wr 5 9C 91051 91052 91053 91054 91055 91056 91057 91058 91059
wr 5 83 91851 91852
export MM4_HURT=1
wr 1 9C 91511 91512 91513
wr 2 9C 91521 91522 91523
wr 5 9C 91551 91552 91553
wr 9 9C 91591 91592 91593
wr 11 9C 91611 91612 91613
wr 12 9C 91621 91622 91623
unset MM4_WEAPON MM4_WEN MM4_HUD MM4_HURT
# weapons on a ladder (wbatch2.sh): stage 2, page 1 ladder, no enemies, teleport start
export MM4_CAM_TRACE=1
wl() { w=$1; shift; for s in "$@"; do [ -f "ptrace_r$s.txt" ] && MM4_WEAPON=$w MM4_WEN=9C MM4_HUD=1 check 2 mm4_noenemy.nes $s; done; }
wl 1 93011 93012 93013
wl 2 93021 93022 93023
wl 4 93041 93042 93043
wl 6 93061 93062 93063
wl 7 93071 93072 93073
wl 8 93081 93082 93083
wl 9 93091 93092 93093
wl 10 93101 93102 93103
wl 11 93111 93112 93113
wl 12 93121 93122 93123
wl 13 93131 93132 93133
unset MM4_CAM_TRACE
# Rush Marine (weapon 3, only usable in water): stage 6 start, hurt mode for 915xx, 3 units of energy for 918xx
wm() { w=$1; en=$2; shift 2; for s in "$@"; do [ -f "ptrace_r$s.txt" ] && MM4_WEAPON=$w MM4_WEN=$en MM4_HUD=1 check 6 mm4_orig.nes $s; done; }
export MM4_ANYY=1
wm 3 9C 91031 91032 91033
wm 3 83 91831 91832
export MM4_HURT=1
wm 3 9C 91531 91532 91533
unset MM4_ANYY MM4_HURT
# pause screen (screen.rs): 18 PPU dumps of the real menu while the cursor walks over every cell (pause_walk.lua), nametable + palette + sprites byte for byte
if $MM4 --rom mm4_noenemy.nes --verify-pausescreen "$(ls walk_*.txt | tr '\n' ',' | sed 's/,$//')" >/dev/null 2>&1; then ok=$((ok+1)); echo "pause screen: ok"; else bad=$((bad+1)); echo "pause screen: BAD"; fi
echo "$ok ok / $bad bad"
[ "$bad" -eq 0 ]
