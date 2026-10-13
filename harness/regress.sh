#!/bin/sh
# Re-verify the kept recordings (no Mesen needed, only the exe): `sh regress.sh`. Each group needs the env it was recorded with.
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
# stage 2, plain
for s in 911 912 913; do [ -f "ptrace_r$s.txt" ] && check 2 mm4_orig.nes $s; done
# stage 2, enemies can hurt (seed 812 was the long-frame rng bug)
export MM4_HURT=1
for s in 812; do [ -f "ptrace_r$s.txt" ] && check 2 mm4_orig.nes $s; done
# stage 2 boss fights (gate shutters closed, boss hp 3)
export MM4_DOORS=16,17,18,19 MM4_BOSSHP=3
for s in 751 752 753 771 772 773; do [ -f "ptrace_r$s.txt" ] && check 2 mm4_orig.nes $s; done
unset MM4_HURT MM4_DOORS MM4_BOSSHP
# stage 5 from the level start / page 4 (invincible), then the vertical shaft (needs MM4_ANYY)
for s in 901 921 922 923 931 932 933 934 935 936 941 942 943 951 952 953; do [ -f "ptrace_r$s.txt" ] && check 5 mm4_orig.nes $s; done
export MM4_ANYY=1
for s in 961 962 963 981 982 983 1001 1002 1003 1011 1012 1013 1031 1041 1042 1043 1051 1052 1053; do [ -f "ptrace_r$s.txt" ] && check 5 mm4_orig.nes $s; done
export MM4_HURT=1
for s in 991 992 993; do [ -f "ptrace_r$s.txt" ] && check 5 mm4_orig.nes $s; done
# stage 5 enemy 6E alone (page 18, hurt on)
for s in 1301 1302 1303; do [ -f "ptrace_r$s.txt" ] && check 5 mm4_only6e.nes $s; done
# stage 5 boss (section 8, boss hp 6 / 10 with hurt on)
unset MM4_HURT; export MM4_ANYY=1 MM4_BOSSHP=6
for s in 1401 1402 1403; do [ -f "ptrace_r$s.txt" ] && check 5 mm4_orig.nes $s; done
export MM4_HURT=1 MM4_BOSSHP=10
for s in 1411 1412 1413; do [ -f "ptrace_r$s.txt" ] && check 5 mm4_orig.nes $s; done
# stage 5 sections 4-7 and the climb of section 6 (hurt on; 1511-1513 end in the boss room: boss hp 8)
export MM4_ANYY=1 MM4_HURT=1; unset MM4_BOSSHP
for s in 1501 1502 1503 1521 1522 1523 1541 1551 1552 1553; do [ -f "ptrace_r$s.txt" ] && check 5 mm4_orig.nes $s; done
for s in 1561 1562 1563; do [ -f "ptrace_r$s.txt" ] && check 5 mm4_orig.nes $s; done
export MM4_BOSSHP=8
for s in 1511 1512 1513; do [ -f "ptrace_r$s.txt" ] && check 5 mm4_orig.nes $s; done
# stage 5 section 3 pages 14-16 (recorded with $2B forced to $23, the state after the entry scroll)
export MM4_SECT2B=23 MM4_HURT=1 MM4_ANYY=1; unset MM4_BOSSHP
for s in 1591 1592 1593 1601 1602 1603; do [ -f "ptrace_r$s.txt" ] && check 5 mm4_orig.nes $s; done
unset MM4_SECT2B
# stage 3 start with only the ported enemies (conveyor floor $C0, vertical warp at page 6), invincible
unset MM4_HURT MM4_ANYY
for s in 2011 2012 2013; do [ -f "ptrace_r$s.txt" ] && check 3 mm4_s3_match.nes $s; done
# stage 3 enemies one by one: 03 alone (inv 2101-3, hurt 2111-3), 13 alone (hurt), 67 alone in section 4 (hurt, $2B=$64)
for s in 2101 2102 2103; do [ -f "ptrace_r$s.txt" ] && check 3 mm4_s3_only03.nes $s; done
export MM4_HURT=1
for s in 2111 2112 2113; do [ -f "ptrace_r$s.txt" ] && check 3 mm4_s3_only03.nes $s; done
for s in 2201 2202 2203; do [ -f "ptrace_r$s.txt" ] && check 3 mm4_s3_only13.nes $s; done
export MM4_ANYY=1 MM4_SECT2B=64
for s in 2301 2302 2303; do [ -f "ptrace_r$s.txt" ] && check 3 mm4_s3_only67.nes $s; done
# stage 3 weapon capsule 7D (state $11, restart at checkpoint 1) and enemy 01 (carries Mega Man), $2B=$21; invincible then hurt
export MM4_SECT2B=21; unset MM4_HURT
for s in 2411 2412 2413; do [ -f "ptrace_r$s.txt" ] && check 3 mm4_orig.nes $s; done
export MM4_HURT=1
for s in 2421 2422 2423 2471 2472 2473; do [ -f "ptrace_r$s.txt" ] && check 3 mm4_orig.nes $s; done
export MM4_SECT2B=65
for s in 2441 2442 2443; do [ -f "ptrace_r$s.txt" ] && check 3 mm4_orig.nes $s; done
# stage 3 boss 84 (section 8, $2B=$38; boss hp 6 invincible, 10 and 14 with hurt on)
export MM4_SECT2B=38; unset MM4_HURT; export MM4_BOSSHP=6
for s in 2501 2502 2503; do [ -f "ptrace_r$s.txt" ] && check 3 mm4_orig.nes $s; done
export MM4_HURT=1 MM4_BOSSHP=10
for s in 2511 2512 2513; do [ -f "ptrace_r$s.txt" ] && check 3 mm4_orig.nes $s; done
export MM4_BOSSHP=14
for s in 2521 2522 2523; do [ -f "ptrace_r$s.txt" ] && check 3 mm4_orig.nes $s; done
unset MM4_SECT2B MM4_BOSSHP MM4_HURT MM4_ANYY
# stage 1 (session 14): start with the ported types only (wind on pages 0-7, footsteps, water), then each enemy alone, then sections of the original rom
unset MM4_HURT MM4_SECT2B MM4_BOSSHP; export MM4_ANYY=1
for s in 3001 3002 3003; do [ -f "ptrace_r$s.txt" ] && check 1 mm4_s1_match.nes $s; done
for s in 3101 3102 3103; do [ -f "ptrace_r$s.txt" ] && check 1 mm4_s1_only05.nes $s; done
for s in 3201 3202 3203; do [ -f "ptrace_r$s.txt" ] && check 1 mm4_s1_only14.nes $s; done
export MM4_HURT=1
for s in 3111 3112 3113; do [ -f "ptrace_r$s.txt" ] && check 1 mm4_s1_only05.nes $s; done
for s in 3211 3212 3213; do [ -f "ptrace_r$s.txt" ] && check 1 mm4_s1_only14.nes $s; done
# frog 11 and ceiling crawler 47 in section 2 ($2B=$62)
export MM4_SECT2B=62; unset MM4_HURT
for s in 3311 3312 3313 3321 3322 3323; do [ -f "ptrace_r$s.txt" ] && check 1 mm4_s1_only11.nes $s; done
for s in 3401 3402 3403; do [ -f "ptrace_r$s.txt" ] && check 1 mm4_s1_only47.nes $s; done
export MM4_HURT=1
for s in 3331 3332 3333; do [ -f "ptrace_r$s.txt" ] && check 1 mm4_s1_only11.nes $s; done
for s in 3411 3412 3413; do [ -f "ptrace_r$s.txt" ] && check 1 mm4_s1_only47.nes $s; done
# water jumper 4D in section 10 ($2B=$2A; water splash $6F, bubbles $66, respawn in water)
export MM4_SECT2B=2a; unset MM4_HURT
for s in 3501 3502 3503 3521 3522 3523; do [ -f "ptrace_r$s.txt" ] && check 1 mm4_s1_only4d.nes $s; done
export MM4_HURT=1
for s in 3511 3512 3513; do [ -f "ptrace_r$s.txt" ] && check 1 mm4_s1_only4d.nes $s; done
# big machine 1E in section 5 ($2B=$05; seeds 3641-3 end with its destruction, boss hp 3) and section 8 ($48)
export MM4_SECT2B=5; unset MM4_HURT
for s in 3601 3602 3603 3611 3612 3613 3621 3622 3623 3631 3632 3633; do [ -f "ptrace_r$s.txt" ] && check 1 mm4_s1_only1e.nes $s; done
export MM4_BOSSHP=3
for s in 3641 3642 3643; do [ -f "ptrace_r$s.txt" ] && check 1 mm4_s1_only1e.nes $s; done
export MM4_HURT=1
for s in 3651 3652 3653; do [ -f "ptrace_r$s.txt" ] && check 1 mm4_s1_only1e.nes $s; done
export MM4_SECT2B=48; unset MM4_HURT MM4_BOSSHP
for s in 3661 3662 3663; do [ -f "ptrace_r$s.txt" ] && check 1 mm4_s1_only1e.nes $s; done
# stage 1 boss $8D (section 11, $2B=$3B): hp 6 invincible, hp 10 and 14 hurt (incl. death and respawn)
export MM4_SECT2B=3b MM4_BOSSHP=6
for s in 3701 3702 3703; do [ -f "ptrace_r$s.txt" ] && check 1 mm4_orig.nes $s; done
export MM4_HURT=1 MM4_BOSSHP=10
for s in 3711 3712 3713; do [ -f "ptrace_r$s.txt" ] && check 1 mm4_orig.nes $s; done
export MM4_BOSSHP=14
for s in 3721 3722 3723; do [ -f "ptrace_r$s.txt" ] && check 1 mm4_orig.nes $s; done
unset MM4_BOSSHP MM4_HURT
# original rom, teleports into the sections (shaft 1, section 3/6/7/9 and the end of section 0)
export MM4_SECT2B=41
for s in 3811 3812 3813 3971 3972 3973; do [ -f "ptrace_r$s.txt" ] && check 1 mm4_orig.nes $s; done
export MM4_SECT2B=63
for s in 3901 3902 3903 3981 3982 3983; do [ -f "ptrace_r$s.txt" ] && check 1 mm4_orig.nes $s; done
export MM4_SECT2B=66
for s in 3921 3922 3923; do [ -f "ptrace_r$s.txt" ] && check 1 mm4_orig.nes $s; done
export MM4_SECT2B=27
for s in 3931 3932 3933; do [ -f "ptrace_r$s.txt" ] && check 1 mm4_orig.nes $s; done
export MM4_SECT2B=69
for s in 3941 3942 3943; do [ -f "ptrace_r$s.txt" ] && check 1 mm4_orig.nes $s; done
export MM4_SECT2B=20
for s in 3961 3962 3963; do [ -f "ptrace_r$s.txt" ] && check 1 mm4_orig.nes $s; done
# ---- stage 7 (session 15) ----
unset MM4_SECT2B MM4_HURT MM4_BOSSHP; export MM4_ANYY=1
# start of the stage with the ported types only / the original rom (invincible), single-enemy roms 2E/2F and 54
for s in 4001 4002 4003; do [ -f "ptrace_r$s.txt" ] && check 7 mm4_s7_match.nes $s; done
for s in 4101 4102 4103; do [ -f "ptrace_r$s.txt" ] && check 7 mm4_s7_only2e.nes $s; done
for s in 4201 4202 4203; do [ -f "ptrace_r$s.txt" ] && check 7 mm4_s7_only54.nes $s; done
for s in 5001 5002 5003; do [ -f "ptrace_r$s.txt" ] && check 7 mm4_orig.nes $s; done
export MM4_HURT=1
for s in 4111 4112 4113; do [ -f "ptrace_r$s.txt" ] && check 7 mm4_s7_only2e.nes $s; done
for s in 4211 4212 4213; do [ -f "ptrace_r$s.txt" ] && check 7 mm4_s7_only54.nes $s; done
for s in 5011 5012 5013; do [ -f "ptrace_r$s.txt" ] && check 7 mm4_orig.nes $s; done
unset MM4_HURT
# enemy 48 alone: section 0 page 4 -> page 5 ($2B=$20), page 5 ($2B=$81, plain and hurt)
export MM4_SECT2B=20
for s in 4311 4312 4313; do [ -f "ptrace_r$s.txt" ] && check 7 mm4_s7_only48.nes $s; done
export MM4_SECT2B=81
for s in 4321 4322 4323; do [ -f "ptrace_r$s.txt" ] && check 7 mm4_s7_only48.nes $s; done
export MM4_HURT=1
for s in 4331 4332 4333; do [ -f "ptrace_r$s.txt" ] && check 7 mm4_s7_only48.nes $s; done
unset MM4_HURT
# section 0 end -> section 1 -> 2 (original rom, plain and hurt, $2B=$20)
export MM4_SECT2B=20
for s in 5301 5302 5303; do [ -f "ptrace_r$s.txt" ] && check 7 mm4_orig.nes $s; done
export MM4_HURT=1
for s in 5311 5312 5313; do [ -f "ptrace_r$s.txt" ] && check 7 mm4_orig.nes $s; done
unset MM4_HURT
# page 16 (section 5, ladder warp back to page 12)
export MM4_SECT2B=05
for s in 4411 4412 4413; do [ -f "ptrace_r$s.txt" ] && check 7 mm4_orig.nes $s; done
# section 6 pages 17-21 (enemies 46/4B/68/48, $2B=$a6): plain, then hurt
export MM4_SECT2B=a6
for s in 4421 4422 4423 4431 4432 4433 4441 4442 4443 4451 4452 4453 4461 4462 4463; do [ -f "ptrace_r$s.txt" ] && check 7 mm4_orig.nes $s; done
export MM4_HURT=1
for s in 4611 4612 4613 4621 4622 4623 4631 4632 4633 4641 4642 4643; do [ -f "ptrace_r$s.txt" ] && check 7 mm4_orig.nes $s; done
unset MM4_HURT
# section 2 pages 6-10 ($2B=$a2), plain and hurt
export MM4_SECT2B=a2
for s in 5101 5102 5103 5111 5112 5113 5121 5122 5123 5131 5132 5133 5141 5142 5143; do [ -f "ptrace_r$s.txt" ] && check 7 mm4_orig.nes $s; done
export MM4_HURT=1
for s in 5201 5202 5203 5211 5212 5213 5221 5222 5223 5231 5232 5233; do [ -f "ptrace_r$s.txt" ] && check 7 mm4_orig.nes $s; done
unset MM4_HURT
# section 3 (page 11, $2B=$83) and section 4 (pages 12-15, $2B=$a4)
export MM4_SECT2B=83
for s in 5241 5242 5243; do [ -f "ptrace_r$s.txt" ] && check 7 mm4_orig.nes $s; done
export MM4_SECT2B=a4
for s in 5251 5252 5253 5261 5262 5263 5271 5272 5273 5281 5282 5283; do [ -f "ptrace_r$s.txt" ] && check 7 mm4_orig.nes $s; done
# section 7 pages 22-25 ($2B=$67): plain, then hurt
export MM4_SECT2B=67
for s in 4801 4802 4803 4821 4822 4823 4841 4842 4843 4911 4912 4913; do [ -f "ptrace_r$s.txt" ] && check 7 mm4_orig.nes $s; done
export MM4_HURT=1
for s in 4811 4812 4813 4831 4832 4833 4851 4852 4853 4921 4922 4923; do [ -f "ptrace_r$s.txt" ] && check 7 mm4_orig.nes $s; done
unset MM4_HURT
# section 7 after its entry scroll ($2B=$27): page 25 -> door -> boss room $71 (plain, hurt incl. death/game over, boss hp 4 -> stage clear)
export MM4_SECT2B=27
for s in 4931 4932 4933; do [ -f "ptrace_r$s.txt" ] && check 7 mm4_orig.nes $s; done
export MM4_HURT=1
for s in 4941 4942 4943; do [ -f "ptrace_r$s.txt" ] && check 7 mm4_orig.nes $s; done
unset MM4_HURT
export MM4_BOSSHP=4
for s in 4951 4952 4953; do [ -f "ptrace_r$s.txt" ] && check 7 mm4_orig.nes $s; done
# boss room from its start ($2B=$38): boss hp 6, 2 (stage clear), 3 hurt
export MM4_SECT2B=38 MM4_BOSSHP=6
for s in 4701 4702 4703; do [ -f "ptrace_r$s.txt" ] && check 7 mm4_orig.nes $s; done
export MM4_BOSSHP=2
for s in 4711 4712 4713; do [ -f "ptrace_r$s.txt" ] && check 7 mm4_orig.nes $s; done
export MM4_HURT=1 MM4_BOSSHP=3
for s in 4721 4722 4723; do [ -f "ptrace_r$s.txt" ] && check 7 mm4_orig.nes $s; done
unset MM4_HURT MM4_BOSSHP MM4_SECT2B
# ---- stage 0 (session 16) ----
unset MM4_HURT MM4_BOSSHP MM4_SECT2B; export MM4_ANYY=1
for s in 6001 6002 6003; do [ -f "ptrace_r$s.txt" ] && check 0 mm4_s0_match.nes $s; done
for s in 8001 8002 8003; do [ -f "ptrace_r$s.txt" ] && check 0 mm4_orig.nes $s; done
for s in 6201 6202 6203; do [ -f "ptrace_r$s.txt" ] && check 0 mm4_s0_only15.nes $s; done
export MM4_HURT=1
for s in 8011 8012 8013; do [ -f "ptrace_r$s.txt" ] && check 0 mm4_orig.nes $s; done
for s in 6211 6212 6213; do [ -f "ptrace_r$s.txt" ] && check 0 mm4_s0_only15.nes $s; done
# section 1 (pages 4-6): enemy 04 and 36 alone, then the original ROM
export MM4_SECT2B=21; unset MM4_HURT
for s in 6101 6102 6103; do [ -f "ptrace_r$s.txt" ] && check 0 mm4_s0_only04.nes $s; done
for s in 6301 6302 6303; do [ -f "ptrace_r$s.txt" ] && check 0 mm4_s0_only36.nes $s; done
for s in 8101 8102 8103; do [ -f "ptrace_r$s.txt" ] && check 0 mm4_orig.nes $s; done
export MM4_HURT=1
for s in 6111 6112 6113; do [ -f "ptrace_r$s.txt" ] && check 0 mm4_s0_only04.nes $s; done
for s in 6311 6312 6313; do [ -f "ptrace_r$s.txt" ] && check 0 mm4_s0_only36.nes $s; done
for s in 8111 8112 8113; do [ -f "ptrace_r$s.txt" ] && check 0 mm4_orig.nes $s; done
# section 4 (pages 11-15)
export MM4_SECT2B=24; unset MM4_HURT
for s in 6121 6122 6123; do [ -f "ptrace_r$s.txt" ] && check 0 mm4_s0_only04.nes $s; done
for s in 8201 8202 8203; do [ -f "ptrace_r$s.txt" ] && check 0 mm4_orig.nes $s; done
export MM4_HURT=1
for s in 6131 6132 6133; do [ -f "ptrace_r$s.txt" ] && check 0 mm4_s0_only04.nes $s; done
for s in 6321 6322 6323; do [ -f "ptrace_r$s.txt" ] && check 0 mm4_s0_only36.nes $s; done
for s in 8211 8212 8213; do [ -f "ptrace_r$s.txt" ] && check 0 mm4_orig.nes $s; done
# section 3 (pages 8-10, climb)
export MM4_SECT2B=a3; unset MM4_HURT
for s in 6401 6402 6403; do [ -f "ptrace_r$s.txt" ] && check 0 mm4_s0_only2D.nes $s; done
for s in 8301 8302 8303; do [ -f "ptrace_r$s.txt" ] && check 0 mm4_orig.nes $s; done
export MM4_HURT=1
for s in 6411 6412 6413 6421 6422 6423; do [ -f "ptrace_r$s.txt" ] && check 0 mm4_s0_only2D.nes $s; done
for s in 8311 8312 8313; do [ -f "ptrace_r$s.txt" ] && check 0 mm4_orig.nes $s; done
# section 6 (pages 17-24, platforms and walkers)
export MM4_SECT2B=a6; unset MM4_HURT
for s in 6501 6502 6503 6511 6512 6513; do [ -f "ptrace_r$s.txt" ] && check 0 mm4_s0_sec6.nes $s; done
for s in 8401 8402 8403 8411 8412 8413; do [ -f "ptrace_r$s.txt" ] && check 0 mm4_orig.nes $s; done
export MM4_HURT=1
for s in 655211 655212 655213; do [ -f "ptrace_r$s.txt" ] && check 0 mm4_s0_sec6.nes $s; done
for s in 8421 8422 8423 8441 8442 8443 8451 8452 8453; do [ -f "ptrace_r$s.txt" ] && check 0 mm4_orig.nes $s; done
# page 25 -> boss room, page 7, page 16, the cavern
export MM4_SECT2B=37; unset MM4_HURT
for s in 8501 8502 8503; do [ -f "ptrace_r$s.txt" ] && check 0 mm4_orig.nes $s; done
export MM4_SECT2B=82
for s in 8601 8602 8603; do [ -f "ptrace_r$s.txt" ] && check 0 mm4_orig.nes $s; done
export MM4_HURT=1
for s in 8611 8612 8613; do [ -f "ptrace_r$s.txt" ] && check 0 mm4_orig.nes $s; done
export MM4_SECT2B=85; unset MM4_HURT
for s in 8701 8702 8703; do [ -f "ptrace_r$s.txt" ] && check 0 mm4_orig.nes $s; done
export MM4_HURT=1
for s in 8711 8712 8713; do [ -f "ptrace_r$s.txt" ] && check 0 mm4_orig.nes $s; done
export MM4_SECT2B=6a; unset MM4_HURT
for s in 8801 8802 8803; do [ -f "ptrace_r$s.txt" ] && check 0 mm4_orig.nes $s; done
export MM4_HURT=1
for s in 8811 8812 8813; do [ -f "ptrace_r$s.txt" ] && check 0 mm4_orig.nes $s; done
# boss room ($2B=$38): boss hp 6 (7002 = stage clear), 9 and 17; hurt on for some
export MM4_SECT2B=38; unset MM4_HURT; export MM4_BOSSHP=6
for s in 7001 7002 7003; do [ -f "ptrace_r$s.txt" ] && check 0 mm4_orig.nes $s; done
export MM4_HURT=1
for s in 7011 7012 7013; do [ -f "ptrace_r$s.txt" ] && check 0 mm4_orig.nes $s; done
export MM4_BOSSHP=17
for s in 7021 7022 7023; do [ -f "ptrace_r$s.txt" ] && check 0 mm4_orig.nes $s; done
export MM4_BOSSHP=9; unset MM4_HURT
for s in 8901 8902 8903; do [ -f "ptrace_r$s.txt" ] && check 0 mm4_orig.nes $s; done
export MM4_HURT=1
for s in 8911 8912 8913; do [ -f "ptrace_r$s.txt" ] && check 0 mm4_orig.nes $s; done
unset MM4_HURT MM4_BOSSHP MM4_SECT2B
# stage 0 cavern: the ladder down on page 11 (door table), pages 28-30 (platforms, items), a5/hurt variants
export MM4_SECT2B=24; unset MM4_HURT
for s in 9001 9002 9003; do [ -f "ptrace_r$s.txt" ] && check 0 mm4_orig.nes $s; done
export MM4_SECT2B=6a
for s in 9101 9102 9103 9111 9112 9113 9121 9122 9123; do [ -f "ptrace_r$s.txt" ] && check 0 mm4_orig.nes $s; done
export MM4_HURT=1
for s in 9201 9202 9203 9211 9212 9213; do [ -f "ptrace_r$s.txt" ] && check 0 mm4_orig.nes $s; done
export MM4_SECT2B=a6
for s in 9301 9302 9303; do [ -f "ptrace_r$s.txt" ] && check 0 mm4_orig.nes $s; done
export MM4_SECT2B=2a; unset MM4_HURT
for s in 9401 9402 9403 9411 9412 9413; do [ -f "ptrace_r$s.txt" ] && check 0 mm4_orig.nes $s; done
unset MM4_HURT MM4_BOSSHP MM4_SECT2B
# stage 0 boss with 20 hit points (flash attacks at 16, 8 and 4; 9503 = full defeat -> stage clear)
export MM4_SECT2B=38 MM4_BOSSHP=20; unset MM4_HURT
for s in 9501 9502 9503; do [ -f "ptrace_r$s.txt" ] && check 0 mm4_orig.nes $s; done
export MM4_HURT=1
for s in 9511 9512 9513; do [ -f "ptrace_r$s.txt" ] && check 0 mm4_orig.nes $s; done
unset MM4_HURT MM4_BOSSHP MM4_SECT2B
# stage 4 (sessions 16-17): vertical sections need MM4_ANYY. Bar sliders (single-enemy ROM), machine (18, pages 5/11), boss $75 (boss hp 6), plain starts
export MM4_ANYY=1; unset MM4_HURT MM4_SECT2B MM4_BOSSHP
for s in 10531 10532 10533 10551 10552 10553 10561 10562 10563; do [ -f "ptrace_r$s.txt" ] && check 4 mm4_s4_only08_0e.nes $s; done
for s in 10601 10602 10603; do [ -f "ptrace_r$s.txt" ] && check 4 mm4_s4_only18.nes $s; done
for s in 10611 10612 10613 10651 10652 10653 10801 10802 10803 10911 10912 10913 10931 10932 10933 10951 10952 10953 10971 10972 10973; do [ -f "ptrace_r$s.txt" ] && check 4 mm4_orig.nes $s; done
export MM4_HURT=1
for s in 10521 10522 10523 10541 10542 10543 10571 10572 10573; do [ -f "ptrace_r$s.txt" ] && check 4 mm4_s4_only08_0e.nes $s; done
for s in 10811 10812 10813 10914 10915 10916 10924 10925 10926 10941 10942 10943; do [ -f "ptrace_r$s.txt" ] && check 4 mm4_orig.nes $s; done
export MM4_SECT2B=22; unset MM4_HURT
for s in 10961 10962 10963; do [ -f "ptrace_r$s.txt" ] && check 4 mm4_orig.nes $s; done
export MM4_HURT=1
for s in 10964 10965 10966; do [ -f "ptrace_r$s.txt" ] && check 4 mm4_orig.nes $s; done
export MM4_SECT2B=49
for s in 10954 10955 10956; do [ -f "ptrace_r$s.txt" ] && check 4 mm4_orig.nes $s; done
export MM4_SECT2B=3C MM4_BOSSHP=6; unset MM4_HURT
for s in 10701 10702 10703; do [ -f "ptrace_r$s.txt" ] && check 4 mm4_orig.nes $s; done
export MM4_HURT=1
for s in 10711 10712 10713; do [ -f "ptrace_r$s.txt" ] && check 4 mm4_orig.nes $s; done
unset MM4_HURT MM4_BOSSHP MM4_SECT2B
# stage 6 (session 18): all enemies, the mini boss, the water-level objects 6B/6C, the secret shaft + capsule, the boss $7C
export MM4_ANYY=1; unset MM4_HURT MM4_SECT2B MM4_BOSSHP
chk6() { rom=$1; shift; for s in "$@"; do [ -f "ptrace_r$s.txt" ] && check 6 $rom $s; done; }
chk6 mm4_s6_match.nes 11001 11002 11003
chk6 mm4_s6_a.nes 11011 11012 11013
export MM4_HURT=1
chk6 mm4_s6_a.nes 11021 11022 11023 11161 11162 11163
unset MM4_HURT
export MM4_SECT2B=28
chk6 mm4_s6_b.nes 11111 11112 11113
chk6 mm4_orig.nes 11501 11502 11503 11511 11512 11513 11521 11522 11523
export MM4_HURT=1
chk6 mm4_s6_b.nes 11121 11122 11123 11131 11132 11133
chk6 mm4_orig.nes 11531 11532 11533
unset MM4_HURT
export MM4_SECT2B=29
chk6 mm4_s6_b.nes 11141 11142 11143
chk6 mm4_orig.nes 11471 11472 11473
export MM4_HURT=1
chk6 mm4_s6_b.nes 11151 11152 11153
unset MM4_HURT
export MM4_SECT2B=0
chk6 mm4_s6_c.nes 11201 11202 11203
export MM4_HURT=1
chk6 mm4_orig.nes 11401 11402 11403
unset MM4_HURT
export MM4_SECT2B=45
chk6 mm4_s6_c.nes 11211 11212 11213
export MM4_SECT2B=27
chk6 mm4_orig.nes 11411 11412 11413
export MM4_HURT=1
chk6 mm4_orig.nes 11421 11422 11423
unset MM4_HURT
export MM4_SECT2B=22
chk6 mm4_orig.nes 11431 11432 11433
export MM4_SECT2B=83
chk6 mm4_orig.nes 11441 11442 11443
export MM4_SECT2B=a4
chk6 mm4_orig.nes 11451 11452 11453
export MM4_SECT2B=26 MM4_HURT=1
chk6 mm4_orig.nes 11461 11462 11463
unset MM4_HURT
export MM4_SECT2B=3a
chk6 mm4_orig.nes 11481 11482 11483
export MM4_SECT2B=3b MM4_BOSSHP=6
chk6 mm4_orig.nes 11301 11302 11303
export MM4_HURT=1
chk6 mm4_orig.nes 11311 11312 11313
unset MM4_HURT MM4_BOSSHP
chk6 mm4_orig.nes 11321 11322 11323
unset MM4_HURT MM4_BOSSHP MM4_SECT2B
# stage 8 (session 18): ice floor, slow floor, enemies 02/81/84, boss $87
export MM4_ANYY=1; unset MM4_HURT MM4_SECT2B MM4_BOSSHP
chk8() { rom=$1; shift; for s in "$@"; do [ -f "ptrace_r$s.txt" ] && check 8 $rom $s; done; }
chk8 mm4_s8_match.nes 12011 12012 12013 12021 12022 12023
chk8 mm4_s8_a.nes 12031 12032 12033
export MM4_HURT=1
chk8 mm4_s8_a.nes 12201 12202 12203
chk8 mm4_orig.nes 12221 12222 12223
unset MM4_HURT
chk8 mm4_orig.nes 12211 12212 12213
export MM4_SECT2B=84
chk8 mm4_s8_b.nes 12041 12042 12043 12051 12052 12053 12061 12062 12063
chk8 mm4_orig.nes 12281 12282 12283
export MM4_HURT=1
chk8 mm4_orig.nes 12291 12292 12293
unset MM4_HURT
export MM4_SECT2B=81
chk8 mm4_orig.nes 12231 12232 12233
export MM4_SECT2B=a2
chk8 mm4_orig.nes 12241 12242 12243 12261 12262 12263
export MM4_SECT2B=83
chk8 mm4_orig.nes 12271 12272 12273
export MM4_SECT2B=a5
chk8 mm4_orig.nes 12301 12302 12303
export MM4_SECT2B=36
chk8 mm4_orig.nes 12311 12312 12313
export MM4_SECT2B=37 MM4_BOSSHP=6
chk8 mm4_orig.nes 12081 12082 12083 12091 12092 12093
export MM4_HURT=1
chk8 mm4_orig.nes 12101 12102 12103
unset MM4_HURT MM4_BOSSHP
chk8 mm4_orig.nes 12111 12112 12113 12321 12322 12323 12331 12332 12333
export MM4_HURT=1
chk8 mm4_orig.nes 12341 12342 12343
unset MM4_HURT MM4_SECT2B MM4_BOSSHP
# stage 9 (session 18): block dropper, spikes, boss $95 with scrolling room
export MM4_ANYY=1; unset MM4_HURT MM4_SECT2B MM4_BOSSHP
chk9() { rom=$1; shift; for s in "$@"; do [ -f "ptrace_r$s.txt" ] && check 9 $rom $s; done; }
chk9 mm4_s9_match.nes 13001 13002 13003 13201 13202 13203
export MM4_HURT=1
chk9 mm4_orig.nes 13211 13212 13213
unset MM4_HURT
export MM4_SECT2B=a1
chk9 mm4_s9_a.nes 13011 13012 13013
chk9 mm4_orig.nes 13221 13222 13223
export MM4_HURT=1
chk9 mm4_s9_a.nes 13041 13042 13043
unset MM4_HURT
export MM4_SECT2B=82
chk9 mm4_s9_a.nes 13021 13022 13023
chk9 mm4_orig.nes 13231 13232 13233
export MM4_HURT=1
chk9 mm4_s9_a.nes 13031 13032 13033
chk9 mm4_orig.nes 13241 13242 13243
unset MM4_HURT
export MM4_SECT2B=a3
chk9 mm4_orig.nes 13251 13252 13253
export MM4_SECT2B=84
chk9 mm4_orig.nes 13261 13262 13263
export MM4_SECT2B=85
chk9 mm4_orig.nes 13271 13272 13273
export MM4_SECT2B=a6 MM4_HURT=1
chk9 mm4_orig.nes 13281 13282 13283
unset MM4_HURT
export MM4_SECT2B=37
chk9 mm4_orig.nes 13291 13292 13293
export MM4_SECT2B=38 MM4_BOSSHP=6
chk9 mm4_orig.nes 13101 13102 13103 13111 13112 13113 13301 13302 13303
export MM4_HURT=1
chk9 mm4_orig.nes 13121 13122 13123
unset MM4_HURT MM4_SECT2B MM4_BOSSHP
# stage 10 (session 18): auto-scrolling sky, block crawler, sinking platforms, boss $9E with auto-walk entry
export MM4_ANYY=1; unset MM4_HURT MM4_SECT2B MM4_BOSSHP
chk10() { rom=$1; shift; for s in "$@"; do [ -f "ptrace_r$s.txt" ] && check 10 $rom $s; done; }
chk10 mm4_s10_match.nes 14001 14002 14003
chk10 mm4_s10_a.nes 14011 14012 14013
export MM4_SECT2B=0
chk10 mm4_s10_a.nes 14131 14132 14133 14141 14142 14143 14151 14152 14153
export MM4_SECT2B=a2
chk10 mm4_s10_a.nes 14291 14292 14293 14311 14312 14313 14321 14322 14323 14331 14332 14333
export MM4_SECT2B=34 MM4_BOSSHP=6
chk10 mm4_orig.nes 14401 14402 14403
export MM4_SECT2B=33
chk10 mm4_orig.nes 14460 14461 14462 14510 14511 14512
export MM4_HURT=1
chk10 mm4_orig.nes 14471 14472 14473
unset MM4_HURT MM4_BOSSHP
chk10 mm4_orig.nes 14481 14482 14483
unset MM4_HURT MM4_SECT2B MM4_BOSSHP
# stage 11 (session 18): press boss $92 (player state $0C), plain starts
export MM4_ANYY=1; unset MM4_HURT MM4_SECT2B MM4_BOSSHP
chk11() { rom=$1; shift; for s in "$@"; do [ -f "ptrace_r$s.txt" ] && check 11 $rom $s; done; }
chk11 mm4_s11_match.nes 15001 15002 15003
chk11 mm4_orig.nes 15201 15202 15203
export MM4_HURT=1
chk11 mm4_orig.nes 15211 15212 15213
unset MM4_HURT
export MM4_SECT2B=61
chk11 mm4_orig.nes 15221 15222 15223
export MM4_SECT2B=a3
chk11 mm4_orig.nes 15231 15232 15233
export MM4_SECT2B=66
chk11 mm4_orig.nes 15241 15242 15243
export MM4_SECT2B=39
chk11 mm4_orig.nes 15261 15262 15263
export MM4_SECT2B=3a MM4_BOSSHP=8
chk11 mm4_orig.nes 15101 15102 15103 15271 15272 15273
export MM4_HURT=1
chk11 mm4_orig.nes 15281 15282 15283
unset MM4_HURT
export MM4_BOSSHP=4
chk11 mm4_orig.nes 15291 15292 15293
unset MM4_HURT MM4_SECT2B MM4_BOSSHP
# stage 12 (session 18): block spawner 35, boss $A6 with its rocks, the boss-defeat object $29 (-> stage clear)
export MM4_ANYY=1; unset MM4_HURT MM4_SECT2B MM4_BOSSHP
chk12() { rom=$1; shift; for s in "$@"; do [ -f "ptrace_r$s.txt" ] && check 12 $rom $s; done; }
chk12 mm4_s12_match.nes 16001 16002 16003
chk12 mm4_orig.nes 16301 16302 16303
export MM4_HURT=1
chk12 mm4_orig.nes 16311 16312 16313
unset MM4_HURT
export MM4_SECT2B=84
chk12 mm4_orig.nes 16101 16102 16103
export MM4_SECT2B=a1
chk12 mm4_orig.nes 16321 16322 16323
export MM4_SECT2B=63
chk12 mm4_orig.nes 16331 16332 16333
export MM4_SECT2B=36
chk12 mm4_orig.nes 16351 16352 16353
export MM4_SECT2B=37 MM4_BOSSHP=8
chk12 mm4_orig.nes 16201 16202 16203
export MM4_HURT=1
chk12 mm4_orig.nes 16361 16362 16363
unset MM4_HURT MM4_BOSSHP
chk12 mm4_orig.nes 16371 16372 16373
unset MM4_HURT MM4_SECT2B MM4_BOSSHP
# stage 13 (session 18): boss $AB with shots, moving platforms $AF
export MM4_ANYY=1; unset MM4_HURT MM4_SECT2B MM4_BOSSHP
chk13() { rom=$1; shift; for s in "$@"; do [ -f "ptrace_r$s.txt" ] && check 13 $rom $s; done; }
chk13 mm4_s13_match.nes 17001 17002 17003
chk13 mm4_orig.nes 17201 17202 17203
export MM4_HURT=1
chk13 mm4_orig.nes 17211 17212 17213
unset MM4_HURT
export MM4_SECT2B=62
chk13 mm4_orig.nes 17221 17222 17223
export MM4_SECT2B=a5
chk13 mm4_orig.nes 17231 17232 17233
export MM4_SECT2B=86 MM4_HURT=1
chk13 mm4_orig.nes 17241 17242 17243
unset MM4_HURT
export MM4_SECT2B=a9
chk13 mm4_orig.nes 17251 17252 17253
export MM4_SECT2B=3a
chk13 mm4_orig.nes 17261 17262 17263
export MM4_SECT2B=3b MM4_BOSSHP=8
chk13 mm4_orig.nes 17111 17112 17113
export MM4_HURT=1
chk13 mm4_orig.nes 17271 17272 17273
unset MM4_HURT
export MM4_BOSSHP=3
chk13 mm4_orig.nes 17281 17282 17283
unset MM4_HURT MM4_SECT2B MM4_BOSSHP
# stage 4 machine on page 5 with Mega Man next to the block rows and shooting (B=0.5: a held B never fires): the shots hit slot $13 (13 hits in 21023); the row destruction $A4DE is still not reached (session 22, seeds 21021-21026)
export MM4_ANYY=1 MM4_SECT2B=24; unset MM4_HURT MM4_BOSSHP
for s in 21021 21022 21023 21024 21025 21026; do [ -f "ptrace_r$s.txt" ] && check 4 mm4_orig.nes $s; done
unset MM4_SECT2B
# stage 3 pages 25-31 (session 22): section 7 = pages 26-31 (MM4_SECT2B = 67), section 6 = page 25 (46); the hurt seeds found the conveyor + slide-start bug (rec_s3_c.sh)
export MM4_ANYY=1; unset MM4_HURT MM4_BOSSHP
export MM4_SECT2B=67; for s in 2601 2602 2603 2604 2605 2606 2607; do [ -f "ptrace_r$s.txt" ] && check 3 mm4_orig.nes $s; done
export MM4_SECT2B=46; for s in 2611 2612; do [ -f "ptrace_r$s.txt" ] && check 3 mm4_orig.nes $s; done
export MM4_HURT=1
export MM4_SECT2B=67; for s in 2621 2622 2623 2624 2625 2631 2632 2633 2634 2635 2636 2639; do [ -f "ptrace_r$s.txt" ] && check 3 mm4_orig.nes $s; done
export MM4_SECT2B=46; for s in 2626 2637 2638; do [ -f "ptrace_r$s.txt" ] && check 3 mm4_orig.nes $s; done
unset MM4_HURT MM4_SECT2B MM4_ANYY
# stage 14 (session 18): plain starts only (the boss rush is not ported yet)
export MM4_ANYY=1; unset MM4_HURT MM4_SECT2B MM4_BOSSHP
for s in 18001 18002 18003; do [ -f "ptrace_r$s.txt" ] && check 14 mm4_s14_match.nes $s; done
# stage 14 (session 19): pods + boss rooms (rec_s14_a.sh); the plain starts use mm4_s14_match.nes
unset MM4_SECT2B; for s in 18101 18102 18103; do [ -f "ptrace_r$s.txt" ] && check 14 mm4_orig.nes $s; done
export MM4_ANYY=1 MM4_SECT2B=49 MM4_BOSSHP=6
for s in 18201 18202 18203 18211 18212 18213 18221 18222 18223 18231 18232 18233 18241 18242 18243 18251 18252 18253 18261 18262 18263 18271 18272 18273 18281 18282 18283; do [ -f "ptrace_r$s.txt" ] && check 14 mm4_orig.nes $s; done
unset MM4_BOSSHP; export MM4_SECT2B=49
for s in 18121 18122 18123; do [ -f "ptrace_r$s.txt" ] && check 14 mm4_orig.nes $s; done
# stage 14 boss rooms (session 19, rec_s14_b.sh): section 10 ($2B = 4A), AB = pod index n, 0149 = 23+n, AC = defeated mask
export MM4_ANYY=1 MM4_SECT2B=4A MM4_BOSSHP=6
boss14() { n=$1; ac=$2; shift 2; c=$(printf %x $((23 + n))); for s in "$@"; do [ -f "ptrace_r$s.txt" ] && MM4_AB=$n MM4_C149=$c MM4_AC=$ac check 14 mm4_orig.nes $s; done; }
boss14 1 0 18401 18402 18403
boss14 2 0 18411 18412 18413
boss14 4 0 18421 18422 18423
boss14 5 0 18431 18432 18433
boss14 6 0 18441 18442 18443
boss14 0 0 18461 18462 18463
boss14 3 0 18471 18472 18473
boss14 7 7F 18451 18452 18453 18481 18482 18483
# ninth pod poked into slot 23, AC = FF: the final Wily scene on page 0B (rec_s14_b.sh, last recordings); BOSSHP differs per group
export MM4_SECT2B=49 MM4_AC=FF
wily14() { hp=$1; shift; for s in "$@"; do [ -f "ptrace_r$s.txt" ] && MM4_BOSSHP=$hp check 14 mm4_orig.nes $s; done; }
wily14 6 18497 18501 18502 18503
wily14 4 18511 18512 18513
wily14 2 18521 18522 18523
wily14 1 18531 18532 18533 18534 18535 18536
# the same with Mega Man vulnerable (NOINV=1, defeated bits of the pods poked: POKE ...,102=FF,103=FF): death in the pod room / the machine scene, restart with the ninth pod, X $68 drop (session 22)
export MM4_HURT=1
wily14 2 18541 18542 18543
wily14 4 18544 18545 18546
wily14 1 18547 18548 18549
unset MM4_AC
unset MM4_SECT2B MM4_ANYY MM4_HURT MM4_BOSSHP
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
