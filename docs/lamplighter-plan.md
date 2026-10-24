# Lamplighter

A plan for turning `games/platformer` from a showcase level into a short real
game with a name. This document is the design and the order of work; it moves
with the game, and a stage is checked off here when its acceptance criteria are
met and its tests pass.

## What it is, and what kind of project it is

**Lamplighter** is a side-view platformer about a small courier, **Wick**, who
walks a valley at dusk lighting its lanterns so the way home stays open, until
the last beacon turns the night to dawn. Five to eight levels, a title and level
select, saved progress, music and an ending.

It is still **the platformer genre showcase**: the place the plain 2D path is
proven (tilemap collision, scene gravity, the character controller, one-way
planks, joints, springs and hinges, 2D lights, normal maps, particles, camera
follow and bounds, scene changes, saves). What changes is the bar. The showcase
rule in `AGENTS.md` was "a level and a few scripts, not a campaign", and that
produced a level that proved features and was not a game anyone would choose to
play. Causeway is held to the standard of a game somebody would choose; the 2D
path should be too, because a showcase nobody wants to play does not showcase
anything. `AGENTS.md` records this as the platformer's exception, and says why.

Say which of these a change is, in its documentation: a capability the engine
already had that Lamplighter now uses is proven here; a capability found by
trying to make Lamplighter good is added generally, never shaped around it.

## Why a remake, and what it keeps

The current project has become a physics proving ground. It carries a crate, a
hanging lantern, a trolley on a rail, a plank ferry, two windmills and springs,
each with its own tests: 21 test files, and about 70 references from
`docs/capabilities/*.toml`, of which about half are physics. That evidence is
worth keeping, and it is held up by toys placed in one level for the sake of the
test. Lamplighter keeps every mechanism and makes each one a **level piece**: a
thing a level is built around and a player uses, with its tests moving to that
level. The capability pages then point at something that is played.

| Mechanism now | Becomes | Level | Tests that move with it |
| --- | --- | --- | --- |
| Hero, run, jump, drop-through | Wick | all | `character_gameplay`, `controller_terrain`, `one_way_platforms` |
| Plank ferry | the harbor ferry | 2 | `moving_platform` |
| Crate and wind | a crate to push onto a switch | 2, 3 | `forces_and_rotation`, `contact_impacts` |
| Windmills | sails that turn into platforms | 3 | `hinge_motor`, `spawned_joints` |
| Trolley on a rail | a cable car across a gorge | 4 | `slider_spring`, `joint_controls` |
| Hanging lantern on a tether | swinging lamps to ride and cut | 5 | `distance_joints`, `saved_joints` |
| Cave lights and occluders | the lantern caves | 6 | `cave_lights` |
| Springs | bounce pads | 4, 7 | `physics_materials` |
| Coins | embers (optional, one prefab) | all | `coins_are_one_prefab` |
| Particles | sparks on lighting, dust on landing | all | `sparkle_and_dust` |
| Tile normal maps | lit tiles | all | `studio_normal_map`, `painted_ground` |
| Sliced panels | the HUD and menus | all | `sliced_panels` |
| The run to the flag | each level's run to its beacon | 1 to 8 | `a_run_reaches_the_flag` |

## The game

**The loop.** Run to the lantern ahead and light it by touching it: a spark, a
flare of 2D light, a note, and it is the checkpoint. The level's last lantern is
a beacon; lighting it ends the level. Embers are optional and ask for a detour.
Falling, water and the dark ("gloom" that creeps in at the edge of the light)
send Wick back to the last lit lantern, with no lives and no punishment beyond
the walk back. The valley darkens as the game goes on, so the lanterns matter
more and the light you carry matters more.

**Levels** (the order teaches one idea at a time, and the last combines them):

1. **Dusk Road.** Run, jump, one-way planks, the first lantern. No hazards but a
   gap. Teaches the hero.
2. **The Ferry Landing.** Water gaps, the plank ferry, a crate to push onto a
   switch that lowers the gate.
3. **Windmill Ridge.** Sails turn on hinge motors and carry you up; wind pushes
   the crate; time the jump from sail to sail.
4. **The Cable Car.** A trolley on a rail across a gorge, springs to reach the
   upper path, a spring-loaded lamp to light.
5. **The Hanging Lamps.** Swing on tethered lamps; cut a cord to drop a bridge.
6. **The Lantern Caves.** The dark is the hazard: lit areas are safe, 2D lights
   and occluders carve the cave, gloom wisps keep to the shadows.
7. **Storm Stair.** A climb that uses everything, under rain and a dimming sky.
8. **The Dawn Beacon.** The last climb and the last lantern; lighting it plays a
   sunrise across the whole valley and rolls into the ending.

**The shell.** A title screen, a level select that shows which beacons are lit
and the embers found, a pause menu, settings (volume buses, reduced motion),
saved progress, drawn touch controls (the current README says they are not
drawn), and credits. Scenes change with `Scene.go`; progress and settings are
`Save` values.

## How it looks

A style bible comes first and every asset answers to it: a **dusk palette** of
deep violet and teal shadows against one warm amber, so light is the only
warmth on screen and a lit lantern reads from across a level. 16 pixel tiles,
3 pixels of outline-free shading, three parallax layers per biome (sky, far
hills, near trees or rock), and 2D lights doing the glow instead of painted
halos. Biomes: road and meadow, harbor, ridge, gorge, caves, storm.

- **Characters and props** are baked from models with the Sprite Baker in side
  view, each animation frame a variant of one sheet so the palette cannot
  shimmer between frames: Wick (idle, run, jump, fall, land, light), a gloom
  wisp, lanterns, the ferry, windmill, trolley, crate, signs and houses. Recipes
  are generated by a script so a limb's angle in frame 3 is one number, and the
  baked sheets are checked in and re-baked by CI, as Orbital's blast is.
- **Tiles, backgrounds, effects and UI art** are drawn by deterministic Python,
  as `art/draw.py` does today but with autotiled edges and corners, per-biome
  sets, normal maps, and layered backgrounds.
- **Type** is a font already in the repository under an open licence, so the
  title and HUD are Weave text and a project font, never an operating-system
  lookup.
- **Sound.** Effects and music are made in the Sound Lab and sent to the
  project: a looping theme per biome as Ogg, effects as WAV, each `.sindrisfx`
  or `.sindrisong` beside its output so it reopens.

## What the engine already has, and what to verify

Present, named in `docs/generated/decay-api.md`: `Scene`, `Save`, `Audio` with
buses, `Camera` follow, bounds, shake and impact, `Sequence` timelines, `Effects`,
`Animation`, `Tween`, `Ui`, `Action` rebinding, 2D lights, normal maps, particles
and joints. Each stage verifies before it relies on one, and where a capability
is missing it is added as a general one with a row in `docs/parity.md`.

Looked at, and what was found:

- **Autotiling exists** (`docs/terrains.md`): a sheet declares a terrain of kind
  `edges` (16 sprites) or `blob` (47, with true inside corners), and the brush and
  `Grid.set_terrain` pick the sprite from the neighbors. Its stated limits are one
  terrain per cell, no transition between two terrains, and no random variants of
  a pattern. Ground uses `blob`; the repetition the missing variants would hide is
  broken up with decoration sprites placed by the level generator.
- **Parallax has no authored system** (`docs/capabilities/runtime.toml`, and
  `docs/2d-model.md` argues it is a consequence of depth under a perspective
  camera, which a side-view pixel game does not want). Backgrounds are layers
  that a small Decay script moves by a fraction of the camera's position. If that
  proves clumsy across levels, a general layer component is added, with its row.
- **The pixel grid is a decision.** The level was drawn at 16 pixels to a tile
  under a camera showing 9 tiles over 540 pixels, which is 3.75 screen pixels to
  each art pixel and soft. Lamplighter draws at **16 pixels to a tile on a 320 by
  180 grid**: the camera shows 11.25 tiles by 20, which is exactly 3 screen
  pixels to an art pixel at 960 by 540 and 6 at 1920 by 1080. The camera has a
  `pixel_perfect` flag (`docs/2d-inventory.md`); whether it keeps the scale whole
  at window sizes that are not multiples is verified in stage 2 and, if it does
  not, closed in the engine. *Found in stage 2:* it does not. A 1366 by 768
  window gives uneven art pixels, so an integer-scale camera mode is a gap to
  close in the engine, in its own pull request, with a `docs/parity.md` row.
- **A pause is a scene.** The engine has no global pause or time scale, and none
  is needed: a scene that is left is switched off rather than unloaded, so the
  pause screen is a scene of its own and `Scene.go` back finds the level as it
  was. What the engine did lack was the other half, starting a level over;
  `Scene.restart(name)` was added for it, generally, with its own tests.
- Browser `Save` across reloads is already proven by Orbital's browser test, and
  respawn without a scene reload is `Hero.respawn()`, played by the scripted run.
- **Still to verify**, none assumed: scene fades and drawn touch controls. Baked
  side-view sheets as animation clips are verified: Wick's run plays in the
  renderer.

## Stages

Each stage is one or more pull requests that leave `main` working: the project
opens, its scripts compile, its scripted run reaches its goal, and the
documentation says what is now true.

0. **Plan.** This document, and the `AGENTS.md` amendment. *Done.*
1. **Move.** `games/platformer` becomes `games/lamplighter`, with the crate, the
   Pages route, directory entry, capability references, CI and scripts renamed,
   and nothing else changed. Reviewed on its own because it touches the most
   files. *Done:* the one thing left under the old name is the scene file,
   `assets/platformer.scene`, which stage 3 replaces with a scene per screen and
   level, and prose that says "the platformer" meaning the genre.
2. **Art foundation.** The style bible and palette, the autotiled tile sets and
   backgrounds for the first two biomes, Wick and the lantern baked and
   animated, the font, and a screenshot test that opens the art. *Done:*
   `art/dusk/build.py` draws the meadow and harbor tile sets (47-tile blob
   terrains with normal maps), the water, sky and hill layers, decoration and
   the lantern with its stepped glow, and bakes Wick from a recipe the script
   writes; CI runs it again and fails on any drift. `tests/art_foundation.rs`
   opens both art scenes and checks every picture, sprite and normal map they
   name and that their ground is what its terrain paints. Wick's run clip was
   seen playing in the real renderer. The font is the Inter already in
   `assets/fonts`; a pixel face is a stage 3 decision, made with the title screen.
3. **The shell.** Title, level select, pause, settings, saved progress and drawn
   touch controls, with level one behind it. *Done:* the title, levels,
   settings and pause screens are scenes (`art/dusk/shell.py` writes them), the
   old level is `level-1.scene` behind them, progress and settings are `Save`
   values, and `tests/the_shell.rs` plays the route between them. Drawn left,
   right and jump controls appear after touch input; the scripted touch test
   holds a button and verifies Wick moves.
4. **Level 1, end to end.** Lanterns, checkpoints, respawn, embers, the first
   music and effects, and a scripted run. After this stage the game is playable
   from the title to the first beacon and to the ending screen.
5. **Levels 2 to 7**, one or two per pull request, each with its mechanism
   rebuilt as a level piece and its tests moved. The capability pages are
   updated as the tests move, not afterwards.
6. **Level 8 and the ending.** The sunrise, credits, final music.
7. **Polish and proof.** A scripted run of every level in one test, the browser
   smoke, the screenshots and site entry, `docs/parity.md` and the capability
   pages brought level with what is played.

## Checks that hold throughout

- Every level has a scripted run that plays it to its beacon, and one test
  plays the whole game. A level that cannot be completed is a failing test.
- `scripts/preflight.py` and the typed Decay preflight on every changed script;
  no game rules in Rust; the project has no `src` beyond its test harness.
- The sprite sheets and tile art are reproducible: re-running the art scripts
  and the baker changes nothing, and CI says so.
- A capability gap is closed in the engine, generally, in the stage that finds
  it, with its parity row and tests.

## Open questions

- Whether Wick is a person, an animal or a spark. The first art pass decides,
  and the answer only changes the models.
- How hard the game should be. The plan assumes a gentle one: the point is the
  light, not the death count.
- Whether the browser host persists `Save` across reloads. If it does not, that
  is the first gap, and it is closed in the engine.
