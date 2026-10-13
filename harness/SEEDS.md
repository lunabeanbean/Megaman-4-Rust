# Recording seed numbers (shared by ALL recordings: never reuse one, the kept `ptrace_r<seed>.txt` would be overwritten and its regress group breaks)
Before recording, check: `grep -rn "<seed>" regress.sh rec_*.sh` (and `ls ptrace_r<seed>.txt`). Session 22 overwrote 11001-11003 (stage 6) by mistake and had to re-record them
(`STAGE=6 KEEP=1 JOBS=3 B=0.4 RIGHT=0.7 sh game_test.sh mm4_s6_match.nes 3000 11001 11002 11003`).
Used ranges by area (see STATUS.md for the exact seeds): 2xxx stage 3 (2601-2639 = pages 25-31, session 22), 4xxx stage 7, 5xxx/6xxx/7xxx/8xxx/9xxx stage 7/0/1, 10xxx stage 4 (10001-10971),
11xxx stage 6, 12xxx-17xxx stages 8-13, 18xxx stage 14 (18541-18549 = hurt mode in the final Wily scene), 21xxx stage 4 machine (21021-21026), 91xxx-93xxx weapons, 655xxx stage 6.
Free: 22000-29999, 30000 and up.
