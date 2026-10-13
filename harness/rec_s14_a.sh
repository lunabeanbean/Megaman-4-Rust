#!/bin/sh
# Stage 14 recordings (session 19). Run from harness/. mm4_orig.nes has every type ported. Pod room = page 9, section 9 ($2B = $49 = 73).
export MM4_ANYY=1
G="sh game_test.sh mm4_orig.nes"
# teleport next to each of the nine pods (pod table $A7E7: Y,X = 44,20 84,20 C4,20 C4,70 C4,90 44,E0 84,E0 C4,E0 44,80)
POD="TPSECT=9,9,73,0 TPCUR=15 TPWAIT=8 B=0.3 BOSSHP=6 JOBS=3"
env STAGE=14 KEEP=1 $POD TELEPORT=9,32,50 RIGHT=0.2 sh game_test.sh mm4_orig.nes 3000 18201 18202 18203
env STAGE=14 KEEP=1 $POD TELEPORT=9,32,110 RIGHT=0.2 sh game_test.sh mm4_orig.nes 3000 18211 18212 18213
env STAGE=14 KEEP=1 $POD TELEPORT=9,32,180 RIGHT=0.2 sh game_test.sh mm4_orig.nes 3000 18221 18222 18223
env STAGE=14 KEEP=1 $POD TELEPORT=9,112,180 RIGHT=0.5 sh game_test.sh mm4_orig.nes 3000 18231 18232 18233
env STAGE=14 KEEP=1 $POD TELEPORT=9,144,180 RIGHT=0.5 sh game_test.sh mm4_orig.nes 3000 18241 18242 18243
env STAGE=14 KEEP=1 $POD TELEPORT=9,224,50 RIGHT=0.1 sh game_test.sh mm4_orig.nes 3000 18251 18252 18253
env STAGE=14 KEEP=1 $POD TELEPORT=9,224,110 RIGHT=0.1 sh game_test.sh mm4_orig.nes 3000 18261 18262 18263
env STAGE=14 KEEP=1 $POD TELEPORT=9,224,180 RIGHT=0.1 sh game_test.sh mm4_orig.nes 3000 18271 18272 18273
env STAGE=14 KEEP=1 $POD TELEPORT=9,128,50 RIGHT=0.5 sh game_test.sh mm4_orig.nes 3000 18281 18282 18283
