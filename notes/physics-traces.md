# Player physics, measured from the running ROM

Source: `harness/trace.lua` run headless in Mesen (stage id `$02`, slot 0, mm4.nes with the
16-unit-life edit; hp=`$90` in the log confirms that edit). Numbers are per frame (60 fps).
Format of positions: `screen:pixel.subpixel`. Speeds are `$03F0.$03D8` (pixel.sub) = signed 16-bit / 256.

## Horizontal
- Holding right: X advances **`$01.4C` = 1.297 px/frame** (`$03C0.$03A8`). No acceleration: full speed on frame 1.
- Releasing: stops immediately (no sliding while running).
- `$0420` bit0 = facing right (`fl=01`), `fl=02` = facing left (observed after pressing left).
  `fl=00` was seen while jumping straight up after a right press, so the bit is not simply "last direction"; needs more checks.
- `$03C0/$03A8` keep holding the run speed ($01.4C) even when standing still. They are a
  speed setting, not the instantaneous velocity.

## Vertical (player slot 0)
**Sign convention is opposite to what I first assumed:** for the player, `$03F0:$03D8` positive = moving UP, negative = DOWN.
- Jump start (press A on ground): vy = **`+$04.C0` = +4.75 px/frame**.
- Gravity: **vy decreases by `$00.40` = 0.25 px/frame every frame** (jump apex after ~19 frames, apex ~52 px above start).
- Terminal fall speed: **vy clamps at `-$07.00` = -7 px/frame** (`$F9.00`, seen for 13 frames).
- Variable jump height: releasing A while rising sets vy to **`-$01.00`** (`$FF.00`) immediately (short hop).
- Standing on the ground: vy = `$FF.00` (-1) every frame (constant downward push / ground check).
- Ground Y in this stage's start area: pixel `$B4`.

## Other observed
- Mega Man spawn: X=`$80`, Y=`$64` at stage start (drops in from above), then the state byte `$0300` goes `$81` -> `$01`.
- `$B0` (life) drops 2 per contact hit here (`$90` -> `$8E` -> `$8C`).
- `$14` pad bits: right=`01`, left=`02`, A=`80`.

## Where to look in the asm (not done yet)
The constants `$04C0`, `$0040`, `$0700`, `$FF00` should appear as bytes `C0 04`, `40 00`, `00 07` in the player
code in `bank38_3F.asm`; searching for them is the quickest route to the real jump routine and a safe first
physics edit (e.g. change gravity or jump speed and re-run the trace).

## Code locations found (via memory-write callbacks in `harness/writes.lua`)
Mesen reports the PC *after* the writing instruction. CPU addresses below; file line numbers are in `bank38_3F.asm`.
- **Jump start:** line 12726, `LDA #$05` at `$3C814E` (CPU `$814E`), then `STA $03F0` = vy pixel part. Sub-pixel part is 0 (`STA $03D8` just above).
  So jump speed is `$05.00`; the first logged frame shows `$04.C0` because gravity is applied once before the log.
  This is state 0 (standing/walking) handling: A held, not Down-pressed. Down+A goes to the slide code instead (`$3C8156`, sets `$30 = 02`).
  The routine is picked through a jump table indexed by `$30` (table `$80FA`/`$810E`, called from `$3C80DA`).
- **Gravity:** `code_3FF2FB` area, lines ~24939-24954 (`$3FF302`..): `vy.sub -= $99`, borrow into `vy.pixel`,
  and a clamp: if `vy.pixel` goes below `$F9` it is set to `$F9.00` (terminal velocity -7). `$99` holds the gravity amount.
- **Gravity value `$99`:** set to `$40` at stage start (`$3EC544`) and by `$3BA8A2`; `$3BA872` sets it to `$15` (reduced gravity in some situation, probably underwater).
- **Grounded vertical speed:** `code_3FF324` (line ~24959) sets vy = `$FF.00` (-1).
- **Player state byte `$30`:** `0` = ground (stand/walk/jump start), `2` = slide (more values not mapped). Used as the index into the state-handler jump table.
- **X speed set** at `$C78D/$C792` (every frame, writes `$4C`/`$01`, i.e. 1.297 px/frame) and `$85EF/$85F4` (other bank, 54 writes, probably when the walk state starts/stops).

## Verified physics edit (round trip)
`$3C814E`: `LDA #$05` -> `LDA #$07` (one byte, file offset 491872). Rebuilt to a separate ROM and re-ran the harness:
jump apex rose from Y=`$7F` to about `$71` (about 14 px higher). The source file was put back to `$05` afterwards (not committed).
Easy knobs to try next: gravity `$40` at `$3EC544`, jump speed at `$3C814E`, run speed (find the constant behind `$C78D`).
