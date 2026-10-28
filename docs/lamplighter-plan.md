# Lamplighter

A plan for **Lamplighter**, a top-down action-adventure in the spirit of the
classic 2D Zelda games, about a wandering lamplighter bringing light to a world
whose sun is fading. This document is the design and the order of work; it
moves with the game, and a stage is checked off here when its acceptance
criteria are met and its tests pass.

## What changed

Lamplighter began as a plan to turn the platformer showcase into a short side-view
game. That direction is retired. The platformer stays as it was, in
`games/platformer` (its levels, its physics level pieces, its tests, and the title,
level select, settings, pause and saved-progress shell built around it), as the
proof of Sindri's plain 2D platformer path. **Lamplighter is a new project in
`games/lamplighter`**, a top-down action-adventure. What the platformer work found
carries over: the dusk palette, the autotiled-ground and baked-sprite art
pipeline, `Scene.restart`, and the way a shell of scenes and `Save` values holds
a game together.

## The game

The sun no longer gives enough light to reach much of the world. Settlements
depend on lanterns and beacons to keep paths visible and give people somewhere
safe. Beyond those pools of light the land is nearly pitch black. **Wick** is a
wandering lamplighter, part adventurer and part wizard-like drifter, who carries
a sword and a lantern between isolated villages, forests, caves and forgotten
ruins, bringing light to places left in darkness.

The tone is mysterious and sometimes unsettling, but not without warmth: each
lantern Wick lights is a small refuge, and each person helped makes the world a
little less empty. Why the sun is fading is for the player to discover. One
possible ending is bittersweet: the dying sun erupts in a last display of light
and colour before it destroys the world, which gives the journey its meaning
even if the world cannot be saved. The light Wick brings matters to the people
living in it now.

### How it plays

Exploration, sword combat, puzzles and light-based abilities.

- **Explore** an interconnected world of villages, woods, caves and ruins.
  Darkness hides paths, enemies and secrets until they are lit.
- **Fight** creatures with Wick's sword. Light may reveal enemies or change how
  they behave.
- **Bring light.** Carry a lantern, light fixed lanterns and restore larger
  beacons. These make lasting pools of light and make an area easier to navigate.
- **Solve puzzles** by using light to reveal clues, trigger mechanisms and reach
  places that first seem inaccessible.
- **Grow** by finding abilities that widen what the lantern can do and open
  routes through places already explored.

The sword and the lantern are complementary: Wick can defend himself, but light
is the defining tool of the adventure.

### Lantern abilities (open)

Possibilities: a focused beam forward; a temporary flame left behind; light
reflected around corners; revealing hidden markings, paths or objects. Which of
these, and in what order they are found, is decided as the world is designed.

### Open design choices

- Whether Wick can **put down the carried lantern and pick it up again**. It
  would make puzzles that need a light source left behind, but it changes
  exploration and combat. Decided at stage 3, by playing both.
- The cause of the fading sun, and whether the ending is the bittersweet one.
- How hard it is. The working assumption is gentle: the point is the light.

### The first area

A dark village and the woods around it. Wick arrives to find the village beacon
out and its remaining people sheltering indoors. With only the carried lantern,
the player learns to move, explore, use the sword and light a fixed lantern.
Restoring the beacon is the first goal. Lighting it reveals more of the village
and the paths into the wider world: the first accomplishment is making a place
safer, not defeating an enemy.

## How it looks

Top-down pixel art with a slightly elevated view, as in the classic 2D Zelda
games. The world is drawn in **readable, relatively neutral colours**; the 2D
lighting system makes the darkness and the warm pools of lantern light, so a lit
lantern is the one warm thing on screen. The pixel grid is 16 pixels to a tile on
a 320 by 180 logical screen (camera `vertical_size` 11.25), which is exactly three
screen pixels to an art pixel at 960 by 540.

- **Wick** keeps his existing look: a blue hood, a red scarf and a warm lantern.
  The top-down version needs directional sprites (idle, walk, sword swing, hurt)
  and the sword. Baked from a model with the Sprite Baker in top-down view with
  each animation frame a variant of one sheet, or drawn, whichever reads better;
  decided at stage 2.
- **Environment art** begins with a grass-to-dirt tileset for the village
  clearing and woodland paths (a `blob` terrain, as the platformer's ground is),
  then trees, walls, buildings and lanterns. Drawn by deterministic scripts, with
  normal maps read back from the tiles so light rakes across them.
- **Type** is a font already in the repository under an open licence.
- **Sound.** Effects and music are made in the Sound Lab and sent to the project,
  each source beside its output so it reopens.

## What the engine has, and what to verify

Present: 2D lighting (ambient, `sindri.light2d`, occluders with shadows, additive
halos, normal maps), `blob` terrains and the brush that paints them, baked
top-down sprite sheets, sprite animation clips, the 2D character controller and
physics queries, camera follow and bounds, `Scene.go` and `Scene.restart` (a
scene left is switched off, so going back finds it as it was), `Save`, `Audio`
buses, the Weave and screen-UI components. Each stage verifies before it relies
on one, and a missing capability is added generally, with a row in
`docs/parity.md`.

To verify first, none assumed:

- **A top-down controller:** the 2D character controller and gravity set to zero,
  with four- or eight-way movement, against tilemap and collider walls.
- **Darkness as the mechanic:** a near-black ambient with a carried light that
  follows Wick, and fixed lights that persist. How many lights a screen can hold.
- **Light as a game rule:** asking whether an enemy or an object is currently lit
  by a 2D light, so light can reveal enemies, change their behaviour and trigger
  mechanisms. If the engine cannot answer that, it is the first gap, and it is
  closed generally.
- **Sword hits:** swing hitboxes through physics overlap queries, with knockback
  and hurt states.
- **A world of connected areas:** how areas are split into scenes, how Wick moves
  between them and how what he has done in each persists.
- **Integer-scale pixel camera:** a known gap (`docs/parity.md`): the engine does
  not keep a pixel game's scale to whole multiples of the window.

## Stages

Each stage is one or more pull requests that leave `main` working: the project
opens, its scripts compile, its scripted run reaches its goal, and the
documentation says what is now true.

0. **Direction.** This plan, the `AGENTS.md` amendment, and the platformer back in
   `games/platformer`. *Done with this change.*
1. **Seed.** `games/lamplighter` as a new project with its crate, Pages route,
   directory entry and CI scope: a dark scene, Wick walking top-down on a placeholder
   floor with the camera following and a carried lantern light, and the title
   and pause shell carried over from the platformer.
2. **Art foundation.** The grass-to-dirt tileset and the village-and-woods tile
   sets, Wick's directional sprites and sword, trees, walls, buildings and
   lanterns, with normal maps, and a test that opens the art.
3. **Movement, lantern and light.** The top-down controller, the carried lantern,
   fixed lanterns Wick lights by interacting (a lasting pool of light and a place
   to return to), beacons, and the drop-and-pick-up-the-lantern decision.
4. **Sword combat.** The swing, one or two creature kinds that react to light,
   health, hurt, knockback, and defeat sending Wick back to the last lit lantern.
5. **The first area, end to end.** The dark village and woods, villagers to speak
   to, the beacon to restore, saved progress, the first music and effects, and a
   scripted run that plays it through.
6. **Abilities and the wider world.** The lantern abilities, the puzzles that use
   them, and the villages, woods, caves and ruins joined into one world with routes
   opened by what Wick has found.
7. **The story and polish.** The reason for the fading sun, the ending, the
   screenshots and site entry, and `docs/parity.md` and the capability pages
   brought level with what is played.

## Checks that hold throughout

- Every area has a scripted run that plays it, and one test plays the first area
  from the title to its beacon. An area that cannot be completed is a failing test.
- `scripts/preflight.py` and the typed Decay preflight on every changed script;
  no game rules in Rust; the project has no `src` beyond its test harness.
- The sprite sheets and tile art are reproducible: re-running the art scripts and
  the baker changes nothing, and CI says so.
- A capability gap is closed in the engine, generally, in the stage that finds it,
  with its parity row and tests.
