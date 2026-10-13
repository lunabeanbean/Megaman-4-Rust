# Mega Man 4 port (Rust)

Work in progress. The program reads **your own** ROM at run time (found automatically, see the top-level README) and never contains ROM data.

Run (from this folder):

    cargo run --release                      # stage 2
    cargo run --release -- --stage 5         # another stage (0-14)
    cargo run --release -- --stage 3 --screens 12 --scale 4

Controls: arrows (or WASD) move / climb, Z / Space / K jump (Down + jump slides), **X / J shoot** (hold to charge), **Enter = pause menu** (pick a weapon or an E-tank with the arrows, Enter or Z confirms), Q / E switch weapon directly, Tab = Select, Esc quits.
`play_weapons.bat [stage]` (or `--all-weapons`) starts with every weapon unlocked; `--weapon N` does the same for the headless `--shot`. Weapons: 1 Rush Coil, 2 Rush Jet, 3 Rush Marine (water only), 4 Rain Flush, 5 wire (Up + shoot while standing), 6 balloon/spring platform, 7 Dive Missile, 8 Ring Boomerang, 9 Drill Bomb, 10 Dust Crusher, 11 Pharaoh Shot (hold to charge), 12 Flash Stopper, 13 Skull Barrier.
Mega Man, the enemies, shots, items and the life meter are drawn with the game's own sprites (read from your ROM); the rules (movement, collision, camera,
enemy AI, shooting, damage, death and restart at the checkpoint) are the original game's, verified frame by frame (see below).
Options: `--rom PATH`, `--stage N`, `--screens N` (the screen count is guessed if omitted), `--scale 1|2|4`, `--boxes` (hitbox placeholders instead of sprites),
`--ppm OUT.ppm` (write the stage image and exit), `--shot OUT.ppm [--frames N]` (headless scripted run, writes the last frame),
`--verify TRACE.txt` (player only), `--verify-pause LOG.txt` (pause menu), `--verify-select LOG.txt`, `--pause-shot OUT.ppm`, `--verify-obj TRACE.txt` (objects only), `--verify-game TRACE.txt` (everything incl. the sprite list; see `../STATUS.md`).

Layout: `objects.rs` (enemies, shots, items), `game.rs` (one whole frame), `oam.rs` (sprites), `verify_obj.rs` (trace comparison of everything), `rom.rs` (ROM access), `gfx.rs` (graphics loader emulation), `stage.rs` (stage tables -> image, terrain lookup),
`player.rs` (movement and collision), `camera.rs`, `verify.rs` (trace comparison), `nes.rs` (palette, tiles), `main.rs` (window + 60 fps loop).
Format knowledge: `../notes/` (level-format.md, graphics.md, collision.md, camera.md, player-states.md).

## Checks
- Background: for stages 2, 5, 9 and 12 the image is pixel-identical to `tools/render_stage.js`, which was checked against Mesen (`node tools/compare_ppm.js`).
- Player + camera: `mm4 --stage 2 --verify ../harness/ptrace_d.txt` etc. All four recorded traces (1,911 frames) match the real game exactly
  in X, Y, vertical speed, state and camera. Traces a and b end when the game's Mega Man gets hurt (state not ported yet); c and d run in a ROM
  with the stage's enemies removed (`harness/mm4_noenemy.nes`) and cover the whole script.
- Ported and verified: all 15 stages (enemies, bosses, doors), all special weapons and the pause menu logic (`../STATUS.md`, `../notes/weapons.md`). Everything else (menus, cutscenes, ending, sound requests) is ported too: see ../README.md and ../STATUS.md.
Needs Rust (rustup); the first build downloads the `minifb` crate.

## Layout (since 2026-10-07)
- `mm4-core/` - the game rules as a library with no window/audio code (player, camera, stage/terrain, ROM access, graphics loader, `verify`).
- `mm4/` - thin shell: window, keyboard, drawing, command-line options. Build/run from `port/` as before (`default-members` = `mm4`).
