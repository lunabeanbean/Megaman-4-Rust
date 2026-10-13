# Tile collision (player), as ported to `port/src/player.rs`

Verified: 4 scripted traces on stage 2 (1,911 compared frames: running, tapping, reversing, jumps, hops, wall hits, climbing steps) are identical to the
real game in X, Y, vertical speed, state and camera (`mm4 --verify harness/ptrace_<a-d>.txt`).

## Terrain lookup
Probe point (world x, y inside the screen 0-239) -> screen = x>>8 -> layout `screenOrder[screen]` -> block (32x32) -> metatile (16x16)
-> **terrain type = high nibble of the metatile flags** (`$A400+m`). `ED2FC` ORs the type bytes of all probe points and tests bit `$10`:
**a probe is solid if the type value has bit 0 set (types 1, 3, 5, 7...)**. Types 0, 2, 4, 6 do not block here (4 is mapped to 2 for side probes).
Out-of-range Y (above 0 or at/below 240) counts as empty for vertical probes (so pits and the area above the screen are open).

## Probe shapes (hitbox tables at `$D69E/$D6DE` vertical, `$D808/$D848` horizontal; id 0 = standing)
| Use | Points (relative to Mega Man's X, Y) |
|---|---|
| Feet (falling) | y+12 at x-7, x, x+7 |
| Head (rising)  | y-9 at x-7, x+7 |
| Right side     | x+8 at y-8, y, y+11 |
| Left side      | x-8 at y-8, y, y+11 |
| Slide feet/head (ids 4/5) | y+10 at x-14, x, x+14 / y+0 at the same X |
(other ids exist for ladders, water, bosses: dump with the table addresses above)

## Order of operations per frame (code_3FF0F8 / code_3FF3ED)
Vertical: **move by vy, apply gravity, then probe**. Rising -> head probe, if solid: Y += 16 - (probeY & 15) and vy = -1.0.
Falling -> feet probe, if solid: Y -= (probeY & 15), vy = -1.0 and "landed" (carry set).
Horizontal: move by 1.297 px, then probe the side, if solid: right: X -= (probeX & 15); left: X += 16 - (probeX & 15). Sub-pixels are kept.
Standing on the ground = every frame vy = -1.0, moves down 1 px, feet probe hits, pushed back up (net 0).

## Not ported yet
Slide hitbox, ladders (type `$60`?), spikes, water/ice, conveyors (`$3C92C5`), moving platforms (`$3FF7D8`/`$3FF843`), special per-stage type rewrites
(`$3ED599`-`$3ED661`), screens above/below (vertical stages), sections/doors.
