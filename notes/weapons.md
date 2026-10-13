# Special weapons (session 20: all of weapons 0-13 are ported)

All of this is ported from bank `$3C` (the spawn code, jump table `$91EB`/`$91F9` indexed by `$A0`) and bank `$3A` (the shot AIs), and **verified frame-exact against Mesen** (`harness/wtest.sh`, see below).

## Weapon slots (`$A0`, energy in `$B0+$A0`, bit 7 = owned, low 5 bits = units 0..28)
Slot -> stage that gives it (`$93B1`): 12 Flash Stopper = stage 0, 4 Rain Flush = stage 1, 9 Drill Bomb = stage 2, 11 Pharaoh Shot = stage 3, 8 Ring Boomerang = stage 4,
10 Dust Crusher = stage 5, 7 Dive Missile = stage 6, 13 Skull Barrier = stage 7. Slots 1-3 are the Rush items (1 Coil, 2 Marine, 3 Jet; stage 2 also gives slot 2, stage 1 slot 3, slot 1 is owned from the start),
5 and 6 the two adaptors (5 = wire, 6 = balloon/spring platform; names are my reading, the code does not say). Energy cost per shot (`$FD8E`): `00 02 01 01 04 02 02 01 01 01 01 01 04 02`.
`$A0` is cleared at every stage (re)start (`$3EC660`) and when the pause menu closes the weapon is whatever was picked.

## What is ported (all verified, 3 seeds x 900 frames each, stage 2 with enemies, random inputs, B held half the time)
| slot | spawn (bank `$3C`) | object AIs (bank `$3A`) | port |
|---|---|---|---|
| 4 Rain Flush | `$8E3B` cloud (type 5, anim `$36`) in slot 1 | `$8BA3` cloud rises and starts the rain (`$013A=$3C`) | `weapons.rs` `ai_8ba3`, `weapon_phase` (every enemy hit once, `$05B8 = $013A`) |
| 6 spring platform | `$8F17` type 6 (anim `$3E`, solid) in a free slot 3..1 | `$8BFB` rises, compresses when Mega Man lands (`$3F`), `PlayerOp::SetY(+3)` | `ai_8bfb` |
| 7 Dive Missile | `$8F98` (shared 7..10), type = weapon | `$8CA4` homing (nearest vulnerable enemy by X) | `ai_8ca4` |
| 8 Ring Boomerang | same | `$8D9E/$8DBC/$8DD7` flies out, aims back, vanishes on touching Mega Man | `ai_8d9e..` ; no reflect, no damage-kill |
| 9 Drill Bomb | same | `$8E31` flies, explodes on a wall or on B; explosion type `$3B` (`$9F85/$9F94`) lays 9 sparks (`$80`, anim `$62`) | `ai_8e31`, `ai_9f94` |
| 10 Dust Crusher | same | `$8E6F` ball (anim `$4A`) bursts at animation frame 4 into four pieces (type `$0A`, anims `$4B-$4E`) | `ai_8e6f` |
| 11 Pharaoh Shot | `$9045` B pressed = small shot `$47`; B held = fireball above him (type `$0D`, `$8F40`); release = medium `$48` / big `$49` (type `$0C`, `$8F25`) | | `Player::weapon_pharaoh`, `ai_8f25`, `ai_8f40` |
| 12 Flash Stopper | `$914E` B starts the time stop (`$013C:$013D = $012C`), while it runs B fires plain buster shots (`3C8CCC`, `Player::plain_buster`) and the routine counts the timer down | none | `World::w13c`; `weapon_phase` freezes every enemy with a damage entry (`$05B8 = $82`) |
| 13 Skull Barrier | `$91C2` type `$0F` (anim `$50`) in slot 1 | `$8FE7` copies Mega Man's position | `weapon_phase`: a harmful enemy shot (damage byte bit 7) is removed together with the barrier |

Shared engine changes: `World::weapon` (`$A0`) picks the damage-table bank (`$20 | weapon`, weapon 12 uses `$20`) and the number of shot slots scanned (`$FABE[weapon]`: 3,3,3,3,0,4,0,1,1,1,4,3,3,1); `shot_hits` uses it;
`damage_enemy` has the per-weapon branches of `$3A82A4` (dust ball `$4A` bursts on the first hit, weapon 8 never dies on a hit, weapon 9 turns into its explosion on the enemy) and `apply_damage` is the shared `$3A830F`;
`reflect_shot` skips weapon 8 and 13. Weapon-energy items (anim `$1C/$1D`) refill the selected weapon with the same 4-frame-per-unit timing as life items (`Pickup::Weapon`). The HUD weapon meter is sprite column x=`$10` (`oam::hud_meter(1, ..)`), shown while `$0131` bit 7 is set (`Player::w131`).

Quirks worth knowing: buster shots scan slots 3,2,1 and the **last free one wins** (lowest), weapon 6 and 11 take the **first free** one (highest) (`ObjEnv::shot_hi`); the Flash Stopper's timer only counts down while the weapon routine runs (walking, jumping, sliding); `$04C8` (slot 0's scratch byte 4) holds the Pharaoh fireball's slot and is cleared only when a shot is fired.

## Rush items, wire (added later in session 20, all verified)
| slot | what | port |
|---|---|---|
| 1 Rush Coil | `$8DD5` (shared 1..3): B calls Rush (type 4, anim `$13`) into slot 4 at the top of the screen; `$88DE` picks the routine by `$A0` (`$8B25/$8B28`): `$88F3` falls (gravity), `$8996` waits for animation frame 4, turns into the item (anim `$8B1C[w]`, hitbox `$8B1F[w]`, solid flag `$8B22[w]`), `$89E3` waits 44 frames (16-bit counter `$0468/$0480`) and bounces Mega Man (he must stand on it: `aframe | $30 == 0`): vy = 6.93 (`$06EE`), `$0498` set (the air handler then ignores "A released"), energy paid; `$8A5D` Rush leaves (anim `$32`) | `rush_*` in `weapons.rs`, `PlayerOp::Bounce` |
| 2 Rush Jet | `$891A` falls in the air and **leaves when the tile is water**; `$8A86` after Mega Man touches it: steered by the pad (tables `$8B32..`), carries him (`carry_player`), energy every 60 frames (`$04B0`), leaves on a wall / empty energy. Standing on it (animation `$24`) sets `$04B0` in Mega Man (`Player::m4b0`): he cannot walk or slide, only jump (`code_3FF188`) | `rush_fall2`, `rush_jet` |
| 3 Rush Marine | `$894B` falls and rests **only over water** (`$41 == $60`); `$8B42`: touching it with state < 2 mounts him (`PlayerOp::Mount`: state 4, `$0510 = $3C`, anim `$27`, he is put at Rush's X/Y, camera `$27/$28` too); Mega Man state 4 (`Player::marine_step`, `$3C84AD`): anim `$27` -> `$29`, pad speed tables `$854D..`, vertical/horizontal probes `$1A/$1B/$26`, up out of the water is stopped (`$10 != $60`), clamp Y `$10..$E0`, plain buster shots **8 px lower** (`code_3C8DA0`), energy every 60 frames. When the energy is gone the graphics set `$4A` is streamed back: `3FFD05` loop = **11 extra full frames inside the logic frame** (`Game::gfx_chunk_frames`; the loader copies 3 tiles per frame, records `[bank, tiles, src, dst]` after the palette bytes that end at the first byte with bit 7). State 0 with animation 7 stays state 0 (`airborne` sets the state only when it changes the animation) | `rush_fall3`, `rush_mount`, `marine_step`, `Player::marine_exit` |
| 5 wire | `$8E9D`: Up + B while standing (animation 1; weapon 5 turns animation 1 into `$38` while Up is held, `code_3C82AF`) shoots a wire (type `$0B`, anim `$3C`, slot 4) up and puts Mega Man into state 5 (anim `$39`); `$8ED4` the wire rises 4 px/frame until it hits the ceiling (anim `$3D`) or the top of the screen (then it is pulled back to him); state 5 (`$858D`, `Player::wire_step`): once the wire is `$3D` he climbs at 4 px/frame, A or Down lets go. The draw pass clips the wire's sprite parts at `$0468` (`oam::object_parts_sid` `maxy`) | `ai_8ed4`, `wire_step` |

Other engine facts found on the way: the hurt handler writes Mega Man's X into slot 4 (`PlayerEvent::Slot4X`), the section reset clears `$04C8` (`Player::m4c8`), weapon-energy items refill the *selected* weapon with the same 4 frames per unit as life items, `PlayerOp::Pay` lets an object pay energy, `World::loop_steps` is reset at the start of the logic frame (extra frames in the player code count too).

Recordings (all in `harness/regress.sh`, group "special weapons", `regress_w.sh` = 111 ok): `wtest.sh` (random), `wseq.sh` (directed), `wbatch.sh` (stage 2 with enemies), `wbatch2.sh` (ladder, teleport start, no enemies), Marine on stage 6 (water at the start: Rush must be called from inside the pool, walk left into it).

## Pause menu (`pause.rs`, verified with `--verify-pause`)
Cursor `$0138` over 2 columns of 8 cells (cell 7 = E-tank, 15 unused), cell -> weapon table `$9B71` = `00 0C 04 09 0B 08 0A FF | 07 0D 01 03 02 05 06 FF`; Left/Right switch column (+/-8) and search upward for an owned weapon, Up/Down step inside the column skipping empty cells;
Start/A on a weapon: `$A0 = n`, `$0131 = n|$80` (0 for the buster), Mega Man's `$30 = 0`, slots 1..4 are removed (`3EDC00`), `$04C8 = 0`; Start/A on the E-tank cell clears `$0131` and refills life if an E-tank is left (cursor back to 0).
Start opens it only in states 0, 1, 3, 4 and not during a time stop / rain (`$3EC707`). Empty cells read `$AF` (zero page wrap of `LDA $B0,X` with X = `$FF`), always 0.

## Recording methods
- `sh harness/wtest.sh <weapon> <energy hex> <frames> <seeds...>`: pokes `$A0`, `$B0+n`, `$0131` at stage frame 699 (`POKET`) and records random inputs (env `B=0.5`, `UP`, `DOWN`, `ROMN=mm4_noenemy.nes`, `KEEP`); the replay gets `MM4_WEAPON/MM4_WEN/MM4_HUD`.
  **Use seeds 91000 and up** (seed numbers are shared with `regress.sh`; I clobbered 2011/2012/2101/2102 once).
- `sh harness/wseq.sh <weapon> <energy> <frames> <name> "<lua input sequence>"`: the same with hand-written inputs (e.g. landing on the spring platform).
- `node harness/pause_trace.js <seed> [frames]` + `mm4 --verify-pause harness/pauselog_<seed>.txt` for the menu (env `OWN=0..1` = chance that each weapon is owned).
