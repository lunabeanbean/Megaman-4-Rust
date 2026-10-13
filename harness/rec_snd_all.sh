#!/bin/sh
# Records the sound requests of every game-flow chain (see rec_snd.sh); From harness/:  sh rec_snd_all.sh
sh rec_snd.sh a 3500
sh rec_snd.sh n 7700
sh rec_snd.sh c 4200
sh rec_snd.sh b 7000 CH_HIJACK=2500
sh rec_snd.sh d 4800 CH_HIJACK=2500 CH_STATE=7 CH_POKE=A1=00
sh rec_snd.sh e 4800 CH_HIJACK=2500 CH_STATE=7 CH_POKE=A1=00
sh rec_snd.sh f 6000
sh rec_snd.sh g 8000 CH_HIJACK=4500
sh rec_snd.sh h 10500 CH_EARLY=2300:AA=07 CH_HIJACK=4500
sh rec_snd.sh p 4500 CH_PAL=1 CH_RAM=6F0,6F1,6F2,6F3,6F4,6F5,6F6,6F7,6F8,6F9,2B,135,F9
sh rec_snd.sh q 3700
