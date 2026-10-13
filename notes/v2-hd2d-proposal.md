# Version 2 proposal: an HD-2D, mod-first engine (draft, 2026-10-12)

Decided with Luna in chat (2026-10-12): the long-term goal is a full mod that becomes its own game, NES look but with lighting and an Octopath-style HD-2D presentation, no NES limits.
This is **version 2**. PLAN.md's finish line (ship the classic frame-exact port first) is still the rule until Luna says otherwise.

## What exists today (relevant facts)
- `mm4-core` = game rules, no window, no audio. ~23k lines. Verified frame-exact against Mesen (`harness/regress.sh` 1009 ok, `regress_w.sh` 111, `regress_screens.sh` 47).
- Logic keeps NES semantics on purpose: 8-bit RAM-style state, original object slots, a 63-sprite OAM list (`oam.rs`), one-nametable `Screen` model with PPU scroll bands (`screen.rs`). Equality with the game is the whole point of the verifier.
- `mm4` = thin `minifb` software window (`main.rs`, 726 lines) that paints a `u32` framebuffer. No GPU path.
- Art comes from the owner's ROM at run time (tiles, 4-colour palettes). Nothing is shipped.

## Principle: classic stays the oracle, v2 is a second layer
Do not edit `mm4-core` into the new engine. A frame-exact core is only valuable while it stays frame-exact. Instead:
1. **`mm4-core` (classic)** stays as is. It is the reference that proves the behaviour.
2. **A render-facing "scene" description** is extracted from it each frame: list of drawable things (id, world position in pixels, flip, palette id, layer, light emission) + tile layers + camera. Today those are OAM bytes and a nametable; v2 needs them as plain data.
3. **`mm4-engine` (new crate)**: GPU renderer + data-driven content, consuming the scene description. Starts by *playing the classic core* with the new renderer, so the game is always runnable.
4. Logic is then migrated system by system from `mm4-core` into data-driven, wider-integer code in the new crate. Each migrated system is checked against the classic core by replaying the same inputs and comparing **game state** (positions, hp, object states), not pixels.

## Rendering plan (HD-2D)
- `wgpu` + `winit` (replaces `minifb`). Render at window resolution, never upscale a low-res image.
- Sprites and tiles = textured quads, nearest filtering, integer pixel scale, so pixels stay chunky while effects are smooth.
- Layers with depth (far background, near background, play layer, foreground); parallax; slight camera tilt/shift later.
- Post passes: bloom, tilt-shift depth of field, colour grading per stage, fog/particles.
- Lighting: light map pass (point lights from shots, fire, lava, pickups; dark stages with a light around Mega Man). Optional palette-stepped mode to keep the NES banding look.
- Sprites no longer limited by tiles, palettes (3 colours + transparent), 8-per-scanline or 64 total.
- Art stays NES resolution (Luna's Zero sprite is used as is). Only the rendering is high-res.

## Data-driven content (what makes modding easy)
Characters, enemies, weapons, stages, bosses, dialogue, palettes as data files (RON/JSON/TOML) loaded by the engine; vanilla Mega Man becomes one character definition, Zero another. Ripped ROM assets are imported at run time from the owner's ROM, mod assets are plain PNGs.

## Suggested order (each step leaves something runnable)
1. Scene-description extraction from `mm4-core` (stage tiles + sprites + camera as data), plus a wgpu window that draws it identically to today's output (parity check against the minifb frame).
2. Renderer features on top of that: lighting, bloom, depth of field. Demo on stage 2 with the existing game logic.
3. Sprite system as PNGs with per-sprite palettes. Drop in Luna's Zero as a player skin first (visual only).
4. Player as data (character definition): physics constants, hitbox, animations. Mega Man and Zero both defined; trace-compare Mega Man against the classic core.
5. Weapons, enemies, bosses as data, one family at a time, each compared against the classic core.
6. Stage format (unlimited sections/layers, no 8-bit coordinates), then the editor.
7. Audio (own mixer; the original music driver is out of scope for v2).

## Open decisions for Luna
- Is v2 started now, or only after the classic release (current PLAN.md rule)?
- Fidelity of v2: should vanilla Mega Man 4 still play identically (core remains the test), or may physics differ?
- Lighting look: smooth, palette-stepped, or both.
- Target platform for v2: Windows only, or also Linux/Mac/web (wgpu supports all).
- Playable-character scope: swap-in skin only, or full movesets (Zero's saber, dash, wall jump)?

## Risks
- Parallax needs layer-split backgrounds; original stages are flat tilemaps (start with flat + lighting + bloom + DoF, which gets most of the look).
- Migrating logic away from 8-bit semantics will break frame-exactness by design; the oracle must compare *behaviour*, with tolerances declared per system.
- Scope: this is a second project the size of the first. Keep the classic release as a stable milestone.
