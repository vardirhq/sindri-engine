# Lamplighter

A plan for **Lamplighter**, a survival game about a wandering lamplighter keeping
a light alive in a world whose sun is fading: 2D pixel art standing in a 3D
world, a generated land of biomes, days that end in a deadly dark, and a lantern
that burns what Wick finds. This document is the design and the order of work;
it moves with the game, and a stage is checked off here when its acceptance
criteria are met and its tests pass.

## What changed

Lamplighter was a top-down action-adventure in the spirit of the classic 2D
Zelda games: a hand-made field and village, lamp posts to light and a beacon to
restore. It becomes a **survival game**, in the broad family of Don't Starve but
its own game: what that genre and Lamplighter share is that the dark is the
danger and light is what keeps you alive. Lamplighter never copies another game's
characters, creatures, items, look or rules; where an idea comes from the genre
it is reworked into Wick's world.

What carries over: Wick, his lantern and his sword, the gloom and the brute,
the lamp posts, the villagers and their houses, the pixel art and its pipeline,
2D lighting as the core mechanic, and every system the field and the village
proved. The field (`world.scene`), the village and the houses stay playable while
the new world is built; the village becomes a landmark placed in the generated
land, and the field is retired once the wilds can be played from the title.

## The game

The sun no longer gives enough light to reach much of the world. Each day is a
little shorter than the last. Settlements depend on lanterns and beacons to keep
paths visible and give people somewhere safe; beyond those pools of light the
land is nearly pitch black, and the gloom live in it. **Wick** is a wandering
lamplighter, part adventurer and part wizard-like drifter, who carries a sword
and a lantern through the wilds between the last lit places.

The tone is mysterious and sometimes unsettling, but not without warmth: each
light Wick keeps is a small refuge, and each person helped makes the world a
little less empty. Why the sun is fading is for the player to discover. One
possible ending is bittersweet: the dying sun erupts in a last display of light
and colour before it destroys the world, which gives the journey its meaning
even if the world cannot be saved.

### How it plays

- **Day, dusk and night.** By day Wick explores and gathers. Dusk warns. At night
  the land is dark beyond what Wick's lantern, a fire or a lit lamp reaches, and
  the gloom come for anything outside a light. The days shorten as the sun fades,
  so the nights grow: the pressure is the light running out, not a calendar.
- **Fuel for light.** The lantern burns. Oil, wax and resin keep it lit, and a
  fire burns wood. A light left unfed gutters and goes out. Light is a resource
  Wick manages, not a switch, and a gloom's pounce still scorches the lantern
  (`Game.scorch`, as now).
- **Gather and craft.** Wick picks up what the land offers (deadwood, stone,
  resin, wax from wild hives, glowcaps, reeds) and makes things from it: torches,
  a campfire, a lamp post that lights a place for good, tools, traps and better
  lanterns. An inventory and a crafting menu, both on screen UI.
- **Survival meters.** Health, hunger, and **courage**, which drains in the dark
  and near the gloom and returns by a fire, in company and with food. Low courage
  makes the dark worse: the gloom bolder, the edges of the screen closing in.
  Food is foraged, hunted and cooked over a fire.
- **Places to keep.** Lamp posts Wick builds and lights make lasting safe ground;
  villages are lit places with people to trade with and help. Keeping a place lit
  through the night is the game's long-term goal, and the beacon at a village's
  heart is still its first.
- **Fight.** The sword is for when light is not enough. The gloom flee strong
  light and burn out in it; the brute does not care about light and must be
  fought.

### Open design choices

- How death works: back to the last lit lamp with something lost, or the run ends
  and a new world is generated. Decided by playing both at stage 4.
- Whether Wick can put the lantern down to leave light behind (a puzzle and
  survival tool, but it changes the night's danger).
- The cause of the fading sun, and whether the ending is the bittersweet one.
- How hard it is. The working assumption is fair rather than punishing: the dark
  is dangerous, never arbitrary.

## How it looks: 2D in 3D space

The world is a 3D space seen from a tilted perspective camera behind and above
Wick. The ground lies flat; everything that stands (Wick, the gloom, trees, rocks,
lamp posts, houses) is a **pixel sprite standing upright as a billboard**, wearing
the camera's rotation so every texel stays square and crisp however the camera
leans. Near things are larger than far ones and the horizon recedes, which is the
depth a flat top-down view lacks; the art stays pixel art at 16 pixels to a unit.

It is built on what Lamplighter already uses, which is why it works: the world
stays on the plane the game plays on, so 2D physics, Decay, the 2D lights,
`Lighting.level` and `Lighting.visible` are unchanged; only the camera tilts and
the standing sprites set `billboard`. The darkness and the warm pools of light are
still the 2D lighting; the lit lantern is still the one warm thing on screen.

**The prototype.** A scene built from the field's own pieces showed it working
with no engine change: a `natural_terrain` voxel world drawn in map view (meadow,
bog and heath, a river, sand and rock) under a perspective camera tilted 38° from
straight down, with Wick, two lamp posts, dead trees, boulders and gloom as
billboards, lit at night by his lantern. The voxel world's map view lies on the
same plane, is lit by the same 2D lights, and is generated from one seed.

### What the prototype showed is missing

- **Lamplighter's own ground.** *Done:* `art/blocks.py` cuts the plain tile of
  each of the game's terrain sheets (grass, dead grass, dirt, earth, soil,
  cobble, flagstone, stone) and draws water, sand, moss, mud and rock in the same
  palette, four variants each, into `blocks.tileset`. Normal maps are still to
  come.
- **Soft biome edges.** A map's biome edges are hard squares (`docs/voxel-terrain.md`,
  "Not yet"); the field's ground uses `corners` terrains for smooth transitions.
  The map view needs edge blending or autotiling between blocks.
- **What stands on the ground.** The generator places voxel trees, which a map
  draws as a top face. Lamplighter needs the land's standing things (trees, rocks,
  bushes, hives, reeds) as billboard sprites placed by the same seed and biome,
  and only near the camera. Engine work, general: a scatter of prefabs or sprites
  over a voxel world, deterministic per section, spawned and despawned with the
  view.
- **Culling under perspective.** *Done:* extraction leaves out sprites, tilemap
  cells and cast shadows off screen under a perspective camera as under an
  orthographic one (`docs/scene-extraction.md`), and the map view already works
  out only the sections the camera sees.
- **A world that does not end.** Residency (generating and dropping the land as
  Wick walks), and saving a generated world as its seed plus its edits.

## The world

One generated land per game, from a seed: biomes chosen for the dark (meadow,
deadwood, bog, heath, stone barrens, old orchards), rivers and lakes, and lit
places set into it by hand-made rules: villages (the existing village and houses
as the first), ruined lamp posts and a beacon to restore. What Wick builds and
what he changes is saved as edits over the seed.

The houses keep their interior scenes, entered through their doors as now.

## What the first Pixel Lab runs found

What came back from the real service, which the client's mock did not show:

- A tileset is sixteen corner-pattern tiles, saved in an order of its own; the
  corner labels `lower` and `upper` were the opposite of the prompts (the `lower`
  tiles were the dirt). A render caught it; `art/tiles.py` now asserts the colors.
  This was the reason for the engine's `corners` terrain kind (`docs/terrains.md`).
- A character asked for at 32 pixels came back at 48; its walk came back at 60, and
  east frames at 56. The art script puts every frame on one cell by the feet.
- A single `animate` call returns one direction. `POST /characters/animations`
  takes all four, one job each; the character's ZIP export names them.
- An object call costs ten generations and returns sixteen candidates as a review
  object; promoting one is free. A character costs one, a walk four, a tileset one.
- The sword's south row was regenerated and is clearly better; the same prompt made
  the north row's blade nearly invisible, so that row keeps its first take.

- A creature is better asked for as an object than a character: the character call
  used its human body whatever the description said, while the object call gave
  eight good gloom to choose from. Candidates of an object call are free to promote,
  so a wood of five trees cost what one did.
- `target_palette` on a pro tileset made noise; the first and second tileset's greens
  differ, so a second terrain needs its greens recolored onto the first's ramp.

- **Resolution.** The world is drawn at 16 pixels to the unit. A 32 pixel tileset was
  blander than the 16 pixel one at the same size on screen, and a Wick generated at 64
  pixels is a better character but would stand four tiles tall against the 16 pixel
  ground; mixing pixel sizes is worse than either, so the density stays.
- **Paving.** The lamps stand on pads of cobblestone, a second `corners` terrain on a
  layer above the ground with the plain ground made transparent, painted from Decay.
- **Shadows from props.** A box occluder at a prop's foot threw a rectangle under it
  that did not match its shape, and the part of the sprite behind its own base was
  darkened by it. Occluders are now ellipses on the footprint, and the contact and cast
  shadows (`sindri.shadow2d`) now sit on every prop, lamp, Wick and gloom. Lighting
  a standing object from its base, so its lit side is not darkened by its own
  occluder, is still to build.

## What the engine has, and what to verify

Present and proven by the field and the village: 2D lighting with shadows,
normal maps, cast and contact shadows, `Lighting.level` and `Lighting.visible`,
named parts, the 2D character controller, physics with named collider pieces,
camera follow and shake, sprite animation, particles, screen UI, `Scene.go`,
`Save`, `Audio`. Present and proven by Low Tide and Voxel Lab: `natural_terrain`
voxel worlds, map view, `Grid.surface` and `Grid.height`, voxel edits. Present but
not yet used by a game in this way: billboard sprites under a perspective camera.

To verify before relying on it:

- **Billboards lit and shadowed correctly** under the tilt: a standing sprite lit
  from where it stands, its contact shadow on the ground, its cast shadow laid
  away from the light, at every camera lean the game uses.
- **Draw order of billboards** against each other and against the ground, from
  the camera's distance.
- **Screen UI for inventory and crafting** on a phone and a desktop: tapping and
  dragging items, and the gamepad.
- **Many lights at night:** fires, lamps, glowcaps and the lantern together within
  the 16-light frame, chosen by what the view shows.

## Stages

Each stage is one or more pull requests that leave `main` working: the project
opens, its scripts compile, its scripted run reaches its goal, and this document
says what is now true.

0. **Direction.** This plan and the `AGENTS.md` description. *Done with this change.*
1. **The wilds.** *Started:* `wilds.scene` (`art/wilds_scene.py`) is a generated
   land of meadow, deadwood, heath, bog and barrens in the game's own blocks,
   with Wick, lamps, trees, boulders and gloom standing as billboards under a
   camera tilted 38° and following him (`tests/the_wilds.rs`), and culling works
   under perspective. It is seen through a lens: the band where Wick walks is in
   focus and the land above and below it softens (`depth_of_field`), and the dark
   past the lantern's reach goes grey and soft (`night_vision`), so the night is
   truly hard to see into. Things stand on the ground with their shadows under
   them, and the seams between biomes fray (`soft_edges`) rather than following
   the grid. What grows is scattered by seed and ground (`sindri.scatter`): dead
   trees, oaks, bushes, boulders and ground cover, made near Wick and unmade far
   from him, and what is taken stays taken. The land goes on as far as the
   generator does: water stops Wick (`sindri.map_walls`), and the world is moved
   back round him far out (`sindri.floating_origin`), so it stays sharp millions
   of squares away. Hamlets stand here and there on open ground, built whole by
   the seed (`sites` on the scatter): a well, houses round it, some still lit.
   Still to come: the wilds opened from the title, hamlets with people and a lamp
   to light, and other places (ruins, camps, the beacon) appearing at random. Wick walks it day-lit; a test walks him across biomes.
2. **The day and the dark.** A clock with dawn, day, dusk and night driving the
   ambient light and the sky; shortening days; the lantern burns fuel; the gloom
   come out at night and keep to the dark. A test survives one night by a fire.
3. **Gather and craft.** Things to pick up, an inventory, a crafting menu, and
   the first recipes: torch, campfire, lamp post, lantern oil. A test gathers and
   builds a campfire before dusk.
4. **Survive.** Health, hunger and courage; food, cooking and the courage effects;
   death and what it costs. A test lives through three days.
5. **Places.** The village set into the world with its houses, villagers to help
   and trade with, ruined lamp posts and the beacon. Saved worlds.
6. **The story and polish.** The fading sun, the ending, music and sound, the
   screenshots and site entry, and `docs/parity.md` and the capability pages
   brought level with what is played.

## Checks that hold throughout

- Every stage has a scripted run that plays it; one test plays from the title
  through the first night. A goal that cannot be reached is a failing test.
- `scripts/preflight.py` and the typed Decay preflight on every changed script;
  no game rules in Rust; the project has no `src` beyond its test harness.
- The sprite sheets and tile art are reproducible: re-running the art scripts and
  the baker changes nothing, and CI says so.
- A world is the same for the same seed on every machine, and a test says so.
- A capability gap is closed in the engine, generally, in the stage that finds it,
  with its parity row and tests.
- Nothing is copied from another game: names, characters, creatures, items, art
  and rules are Lamplighter's own.

## Wick and the gloom, as built

The lantern is its own sprite (`wick-lantern`, drawn by `art/lantern.py`), not part of
Wick's: his art has empty hands in all four directions, and `wick.decay` floats the
lantern on a circle around his hands. It rides at his right hand a little ahead of where
he faces, swings round him as he turns, drifts a little, and is drawn in front of him on
the near half of its circle and behind him on the far half. It carries two lights as parts:
its own, a 140 degree beam that `wick.decay` turns toward where he faces, and `#halo`, a
small round halo that keeps his heels from being black. Each is flickered by a `Flame`
script on the lantern, the halo's attached as `#halo`, and they are scorched together.

The gloom keep to the dark and are undone by light: a gloom caught in a strong light
stops and burns out, and uses cover (`Lighting.visible`) to creep on and ambush. The
brute is the second kind, which light does not trouble: it is bigger, takes five blows,
and only the sword puts it out (`fears_light` and `size` on `gloom.decay`).
