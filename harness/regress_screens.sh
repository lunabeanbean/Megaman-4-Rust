#!/bin/sh
# Screens drawn from the ROM, compared with PPU recordings of the real game (kept in harness/). Run from harness/: sh regress_screens.sh
# Prints one "ok"/"BAD" line per recording. Needs port/target/release/mm4.exe.
EXE=../port/target/release/mm4.exe
ROM="../Mega Man 4 (USA).nes"
chk() { # name, args...
  name=$1; shift
  if $EXE --rom "$ROM" "$@" >/tmp/rs_out.txt 2>&1; then echo "ok  $name"; else echo "BAD $name"; tail -5 /tmp/rs_out.txt; fi
}
# stage select (select_frames.lua): boss-defeated patterns, the all-bosses flash and Wily emblem, a cursor walk over every cell
for v in 00 05 7F A5 FF; do chk "select a9=$v" --verify-selectscreen selv_$v.txt --a9 $v; done
chk "select a9=FF long" --verify-selectscreen selv_FF2.txt --a9 FF
for v in 00 A5; do chk "select walk a9=$v" --verify-selectscreen selw_$v.txt --a9 $v --walk; done
# password / stage-clear menu and game over (ppu_frames.lua with PF_HIJACK: weapon owners, A9 variants, cursor moves; game over with PF_STATE=7 PF_A1=0)
for n in 1 2 3 4; do chk "password screen pfc_$n" --verify-passwordscreen pfc_$n.txt; done
chk "game over (stage < 8) pfg_1" --verify-passwordscreen pfg_1.txt --kind gameover
chk "game over (stage < 8, leave) pfg_2" --verify-passwordscreen pfg_2.txt --kind gameover
chk "game over (stage >= 8) pfg_3" --verify-passwordscreen pfg_3.txt --kind gameover-wily
# weapon-get cutscene of every robot master (stages 1 and 2 add Rush's screen), up to the frame before the password scene: whole timeline, ~600-1100 frames each
for i in 0 1 2 3 4 5 6 7; do chk "weapon get stage $i" --verify-weaponget pfw_$i.txt --wstage $i; done
# boss introduction before each robot master stage (480 frames: palette animation task, typed name, sprite palette fade)
for i in 0 1 2 3 4 5 6 7; do chk "boss intro stage $i" --verify-bossintro pfi_$i.txt --wstage $i; done
# power-on: copyright + CAPCOM PRESENTS (pft_0), title screen: cursor moves, both Start results, the 1200-frame timeout (pft_4..7)
chk "boot screens pft_0" --verify-boot pft_0.txt
for n in 4 5 6 7; do chk "title pft_$n" --verify-title pft_$n.txt; done
# password entry screen (title -> PASS WORD, dots typed with the pad): 6 valid passwords of different weapon sets, 3 wrong ones (7 dots, a dot in a forbidden place, wrong count), error text and retry
for n in 1 2 3 4 5 6 7 8 9 10 11; do chk "password entry pfp_$n" --verify-passwordentry pfp_$n.txt; done
# fortress maps: Cossack castle (first visit with lightning, after cleared stages AA=1/3/7), Wily fortress (AA=0F/1F/3F/7F)
chk "map cossack first pfm_1" --verify-fortress pfm_1.txt --map first
for n in 2 5 6; do chk "map cossack after pfm_$n" --verify-fortress pfm_$n.txt --map after; done
for n in 3 7 8 9 10; do chk "map wily pfm_$n" --verify-fortress pfm_$n.txt --map wily; done
# the cutscene after Dr. Cossack's last stage (saucer, starfield) followed by Wily's fortress map
chk "wily flight + map pfm_4" --verify-wilyflight pfm_4.txt
# the rendered pictures against Mesen screenshots (copyright, CAPCOM, title, select, boss intro, gameplay): 0 differing pixels each
for f in 100 300 1400 1450 1560 1600 1900 2000 2200; do
  $EXE --rom "$ROM" --render-pf pfs_1.txt --frame $((f-1)) --out ../scratch/rs_a.ppm >/dev/null 2>&1
  if node ../tools/pngcmp.js shots_pfs_1_$f.png ../scratch/rs_a.ppm | grep -q "^0 of"; then echo "ok  picture pfs_1 frame $f"; else echo "BAD picture pfs_1 frame $f"; fi
done
for k in 50 100 200 300; do
  $EXE --rom "$ROM" --stage 2 --shot ../scratch/rs_g.ppm --frames $k --idle >/dev/null 2>&1
  if node ../tools/pngcmp.js shots_pfg_s2_$((2302+k)).png ../scratch/rs_g.ppm | grep -q "^0 of"; then echo "ok  picture gameplay frame $k"; else echo "BAD picture gameplay frame $k"; fi
done
# the whole boot + opening story (7000 frames: pages, palette animations, objects, the scanline-split scroll scenes) and its pictures against screenshots
chk "boot + opening story pfo_1" --verify-story pfo_1.txt
for f in 500 1500 3000 4500 6000 6900; do
  $EXE --rom "$ROM" --render-pf pfo_1.txt --frame $((f-1)) --out ../scratch/rs_a.ppm --mix >/dev/null 2>&1
  if node ../tools/pngcmp.js shots_pfo_1_$f.png ../scratch/rs_a.ppm | grep -q "^0 of"; then echo "ok  picture pfo_1 frame $f"; else echo "BAD picture pfo_1 frame $f"; fi
done
# the ending (stage clear after the last Wily machine, PF_STATE=19): castle, saucer, the scrolling credits sky (pfe_1 = frames 0..7383 so far)
chk "ending pfe_1" --verify-ending pfe_1.txt
chk "ending pfe_4 (to the 3rd staff picture)" --verify-ending pfe_4.txt
chk "ending pfe_5 (frames 7900-11800, staff roll)" --verify-ending pfe_5.txt
chk "ending pfe_6 (frames 11700-14400: staff pictures 5-8, last picture)" --verify-ending pfe_6.txt
chk "ending pfe_7 (frames 14400-18000: last picture, friends arrive)" --verify-ending pfe_7.txt
