#!/bin/sh
# Stage 14, boss rooms entered directly (session 19): TELEPORT into page 10 (section 10, $2B = $4A = 74) with $AB = pod index n, $0149 = 23 + n (pod list index), $AC = defeated mask.
# Each boss room then runs: fall-in of the room object 79, power-up, boss `$A8BC[$A8D4[$AB]]`, defeat, C6 prize, exit pod B3, back to the pod room (page 9).
export MM4_ANYY=1
BR="STAGE=14 KEEP=1 TPSECT=10,10,74,0 TPCUR=1f TPWAIT=8 B=0.9 BOSSHP=6 JOBS=3 TELEPORT=10,32,176 RIGHT=0.3"
# n=1 (stage 0 boss 8B), n=2 (stage 7 boss... see $A8D4), n=4, n=5, n=6, n=7 (+ $AC = 7F: the last one opens the ninth pod)
run() { n=$1; ac=$2; seeds=$3; c=$(printf %x $((23 + n))); env $BR POKE=AB=$n,149=$c,AC=$ac MM4_AB=$n MM4_C149=$c MM4_AC=$ac sh game_test.sh mm4_orig.nes 5000 $seeds; }
run 1 0 "18401 18402 18403"
run 2 0 "18411 18412 18413"
run 4 0 "18421 18422 18423"
run 5 0 "18431 18432 18433"
run 6 0 "18441 18442 18443"
run 7 7F "18451 18452 18453"
run 0 0 "18461 18462 18463"
run 3 0 "18471 18472 18473"
# the ninth pod (AC = FF after the 7th boss): longer runs, hoping the random walk reaches the pod at (80,44)
run 7 7F "18481 18482 18483"
# ninth pod poked into slot 23 (the pod room after the 7th boss), Mega Man teleported next to it (pod room page 9, $2B = 49)
env MM4_SECT2B=49 STAGE=14 KEEP=1 TPSECT=9,9,73,0 TPCUR=15 TPWAIT=60 B=0.3 BOSSHP=6 JOBS=3 TELEPORT=9,100,68 RIGHT=0.5 POKE=317=B3,35F=09,347=80,38F=44,3A7=00,41F=1C,53F=90,56F=73,59F=97,5B7=A7,AC=FF MM4_AC=FF sh game_test.sh mm4_orig.nes 6000 18491 18492 18493
# the final Wily scene (page 0B): B=shooting probability, RIGHT=0.5 keeps the early walk to the pod; BOSSHP shortens the fight (BC and its replacement C0)
P9="MM4_SECT2B=49 STAGE=14 KEEP=1 TPSECT=9,9,73,0 TPCUR=15 TPWAIT=60 JOBS=3 TELEPORT=9,100,68 RIGHT=0.5 POKE=317=B3,35F=09,347=80,38F=44,3A7=00,41F=1C,53F=90,56F=73,59F=97,5B7=A7,AC=FF MM4_AC=FF"
env $P9 B=0.3 BOSSHP=6 sh game_test.sh mm4_orig.nes 6000 18501 18502 18503
env $P9 B=0.8 BOSSHP=4 sh game_test.sh mm4_orig.nes 12000 18511 18512 18513
env $P9 B=0.6 BOSSHP=1 sh game_test.sh mm4_orig.nes 9000 18531 18532 18533 18534 18535 18536   # 18533 = the whole scene through the stage clear
# hurt mode (session 22): NOINV=1 and the defeated bits of the eight pod entries poked (102=FF,103=FF), as a real run would have them; deaths restart in the pod room with the ninth pod
P9H="MM4_SECT2B=49 STAGE=14 KEEP=1 NOINV=1 TPSECT=9,9,73,0 TPCUR=15 TPWAIT=60 JOBS=3 TELEPORT=9,100,68 RIGHT=0.5 POKE=317=B3,35F=09,347=80,38F=44,3A7=00,41F=1C,53F=90,56F=73,59F=97,5B7=A7,AC=FF,102=FF,103=FF MM4_AC=FF"
env $P9H B=0.6 BOSSHP=2 sh game_test.sh mm4_orig.nes 9000 18541 18542 18543
env $P9H B=0.8 BOSSHP=4 sh game_test.sh mm4_orig.nes 12000 18544 18545 18546
env $P9H B=0.5 BOSSHP=1 sh game_test.sh mm4_orig.nes 14000 18547 18548 18549
