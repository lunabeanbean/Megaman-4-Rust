# Mega Man 4 (NES) - rewritten in Rust

The game logic of Mega Man 4 rewritten in Rust and checked against the real game, frame by frame, running in the Mesen emulator.
All 15 stages, every boss and enemy, all special weapons, the pause menu, the title / password / stage-select / cutscene screens, the ending and the sound
requests behave like the original. **The repository contains no game data**: the program reads your own ROM when it starts.

> **Disclaimer:** This is an unofficial fan project. It is not affiliated with, endorsed by, or connected to Capcom. Mega Man is a trademark of
> Capcom Co., Ltd. This repository contains no game data (no ROM, graphics, music or text). You must supply your own legally obtained copy of
> Mega Man 4 (USA). It is provided for preservation, research and educational purposes.

## What you need
- Your own dump of **Mega Man 4 (USA)** as a `.nes` file (iNES, 512 KB, mapper 4/MMC3). Dump it from a cartridge you own.
- Windows for the prebuilt download (other systems may work if you build from source; only Windows was tested).
- Rust (https://rustup.rs), only if you want to build from source.

## Play (Windows, prebuilt download)
1. Download `mm4-rust-port.zip` from the **Releases** page of this repository and unzip it.
2. Copy your ROM into the unzipped folder, next to `Play.bat`. Any `.nes` file with "Mega Man 4" in its name is found automatically.
3. Double-click `Play.bat`. It starts from power-on: copyright screen, story, title, stage select.
   - `Play.bat 5` jumps straight into stage 5 (stages are numbered 0-14).
   - `Play_AllWeapons.bat [stage]` starts with every weapon unlocked.
4. If it says "No ROM found", the file is in the wrong place or has a different name. Either rename it to `Mega Man 4 (USA).nes`, or run
   `bin\mm4.exe --rom "C:\path\to\your.nes"`, or set the environment variable `MM4_ROM` to the path.

## Build from source
1. Copy your ROM into the repository folder (or the `port/` folder). Any file named like `Mega Man 4 (USA).nes` is found automatically;
   otherwise pass `--rom PATH` or set the environment variable `MM4_ROM`.
2. From `port/`:

       cargo run --release                 # from power-on: copyright, story, title, stage select
       cargo run --release -- --stage 5    # straight into a stage (0-14)
       cargo run --release -- --mute       # without sound

   On Windows you can double-click `port/play.bat` (or `play_weapons.bat` to start with every weapon).

## Controls
Arrows / WASD move, Z / Space / K jump (Down + jump slides), X / J shoot (hold to charge), Enter = Start (pause menu), Tab = Select,
Q / E switch weapon, Esc quits.

## How it was made
1. The game's disassembly was used to write down how everything works (`notes/`).
2. Each mechanic was rewritten in Rust (`port/mm4-core`, no window or sound code) and compared against recordings of the real game made with random
   inputs in Mesen, frame by frame, until nothing differed (`harness/`, `STATUS.md`, `PLAN.md`).
3. `port/mm4` is a thin window / keyboard / sound-card shell around the library.

## Known limits
- The sound driver and the sound chip are rewritten/modelled in Rust, but the driver still runs the original sound code on a small 6502 core
  (the sound *requests* from the game are all Rust and verified). Porting the driver itself is not done.
- A few single-frame timing effects of the console's picture (scroll edge columns while scrolling, some palette-animation timing) are not reproduced exactly,
  and the ending / pod-room sounds were written from the disassembly without a recording to compare. See `STATUS.md`.
- The test tools in `harness/` need Mesen and your ROM; recordings are not part of the repository.

## License
MIT for the code and notes (`LICENSE`). Mega Man is a Capcom trademark; no Capcom data is included.
