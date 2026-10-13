Finish the Mega Man 4 Rust rewrite in the port folder (everything EXCEPT audio). Work autonomously, in order, until every non-audio item below is done and verified, then stop the loop.

START OF EVERY ITERATION: read NEXT-SESSION.md, STATUS.md and PLAN.md in the project folder, plus the relevant file in notes/. Pick the first unfinished non-audio item from "What is left" in NEXT-SESSION.md. Do not redo finished work.

SCOPE, in this order:
1. Presentation: a small PPU model in Rust (nametables, palettes, the NMI buffer format, CHR sets, OAM) so the ORIGINAL screens are drawn from the ROM instead of placeholders. Order: stage select (finish it: sprite list, palette, graphics sets, blinking cursor, PRESS START), confirm menu, game over, title screen, weapon-get screen, password screen, ending.
2. Game flow: title -> (password) -> stage select -> stage -> weapon get -> back; game over / continue; the ending after the final Wily machine; password encode/decode.
3. HUD/presentation details: palette fades, text, the stage intro, READY blinking.
4. Small leftovers listed in NEXT-SESSION.md item 5 (weapon-get cutscene, final Wily scene past death, etc.).
5. Put the project root under git (local only). Never commit ROMs or ripped assets. Never push to the upstream remote.
SKIP ALL AUDIO (sound engine, music, sfx, mixer). Do not start it.

METHOD (same as every earlier session):
- Every mechanic or screen is checked against the real game in Mesen: record a trace or PPU dump (harness/ppu_dump.lua, select_dump.lua, gen_ptrace.js), then compare the Rust output byte for byte / pixel for pixel at fixed frames. Use the fast toolkit (checklist.js, JOBS=n parallel Mesen, BOSSHP hook, teleports).
- Never claim something is done without a passing comparison. If a comparison fails, find the cause in the disassembly and fix it; do not tweak the test to pass.
- Seed numbers are shared by all recordings: never reuse one.
- Before ending an iteration, run the relevant regression (harness/regress_w.sh for weapons; full harness/regress.sh with run_in_background for engine changes, it takes about 30 minutes) and make sure nothing that was ok became bad.

HOUSEKEEPING EVERY ITERATION:
- Update STATUS.md (new "Session" entry), PLAN.md checkboxes and NEXT-SESSION.md so a fresh session can continue from them alone. Add notes/ files for anything new you learn.
- Update the memory file project-mm4-rust-port.md with a one-line summary of the new state.

PACING: if a long job is running (regress, recordings), start it in the background and schedule the next wake-up 20-30 minutes out instead of polling. Otherwise continue immediately with the next piece.

SAFETY: stay inside the project folder; do not delete files you did not create; if you are truly blocked on a decision only Luna can make (for example what a screen should look like beyond the original game), write the question into NEXT-SESSION.md, skip to the next item, and mention it in the final report.

DONE WHEN: all items 1-5 above are finished and verified. Then write a final summary in STATUS.md, tell Luna what exists, how to run it (port/play.bat), and that only audio remains, and stop the loop.
