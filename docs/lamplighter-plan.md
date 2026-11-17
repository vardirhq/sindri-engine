# Lamplighter

A plan for **Lamplighter**, a survival game about a wandering lamplighter keeping
a light alive in a world shut under the Lid: 2D pixel art standing in a 3D
world, a generated land of biomes, a dark broken only by what people light and
what falls through the cracks, and a lantern that burns what Wick finds. This document is the design and the order of work;
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

One day something settled over the sky: the **Lid**, a vast slow shell of
plates, and nobody knows whether it is alive or made. The sun is still up there,
and only thin cracks of it come through. There has been no day since. The land
under it is dark beyond what people light; settlements keep lanterns and beacons
burning so paths can be seen and people have somewhere safe, and the gloom live
in the dark between them. **Wick** is a wandering lamplighter, part adventurer
and part wizard-like drifter, who carries a sword and a lantern through the
wilds between the last lit places.

Three kinds of light meet under the Lid, and the game is how they meet:

- **Warm light** is people's: lanterns, lamps, hearths and fires. It is steady,
  it stays where it is lit, and it keeps the gloom off. Wick makes and keeps it.
- **Cracklight** is the sun's, falling through the Lid in shafts of gold, the
  **sunfalls**. A sunfall is beautiful and safe and never still: the plates shift,
  so it drifts across the land, comes and goes. It is the only daylight there is.
- **Red light** is the Lid's. **Seekers** come down from it and sweep the ground
  with a circle of cold red light, looking for warmth. To be caught in one is to
  be found: it calls them down, and the gloom with them.

The tone is mysterious and sometimes unsettling, but not without warmth: each
light Wick keeps is a small refuge, and each person helped makes the world a
little less empty. What the Lid is, and why it came, is for the player to
discover. The ending it points to is the Lid breaking: the sun let through, a
first day after so long.

### How it plays

- **Under the Lid.** There is no day and night: the land is always dark beyond
  what Wick's lantern, a fire, a lit lamp or a sunfall reaches, and the gloom come
  for anything outside a light. The pressure is the light running out and the
  Seekers sweeping, not a clock. A sunfall is a place to reach: safe while it
  lasts, and the only light that costs nothing.
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
  against the dark is the game's long-term goal, and the beacon at a village's
  heart is still its first.
- **Fight.** The sword is for when light is not enough. The gloom flee strong
  light and burn out in it; the brute does not care about light and must be
  fought.

### Open design choices

- How death works: back to the last lit lamp with something lost, or the run ends
  and a new world is generated. Decided by playing both at stage 4.
- Whether Wick can put the lantern down to leave light behind (a puzzle and
  survival tool, but it changes the dark's danger).
- What the Lid is, why it came, and how breaking it plays.

**Set aside.** The first premise was a fading sun: days shortening as it died, and
a bittersweet ending in which it erupts in a last display of light and colour
before it destroys the world. It is a good idea that does not fit a game under
the Lid, and is kept here rather than lost.
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
billboards, lit in the dark by his lantern. The voxel world's map view lies on the
same plane, is lit by the same 2D lights, and is generated from one seed.

### What the prototype showed is missing

- **Lamplighter's own ground.** *Done:* `art/blocks.py` cuts the plain tile of
  each of the game's terrain sheets (grass, dead grass, dirt, earth, soil,
  cobble, flagstone, stone) and draws water, sand, moss, mud and rock in the same
  palette, four variants each, into `blocks.tileset`, with a normal map of
  their relief (`blocks_n.png`).
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
- **Many lights in the dark:** fires, lamps, glowcaps and the lantern together within
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
   past the lantern's reach goes grey and soft (`night_vision`), so the dark is
   truly hard to see into. Things stand on the ground with their shadows under
   them, and the seams between biomes fray (`soft_edges`) rather than following
   the grid. What grows is scattered by seed and ground (`sindri.scatter`): dead
   trees, oaks, bushes, boulders and ground cover, made near Wick and unmade far
   from him, and what is taken stays taken. The land goes on as far as the
   generator does: water stops Wick (`sindri.map_walls`), and the world is moved
   back round him far out (`sindri.floating_origin`), so it stays sharp millions
   of squares away. Hamlets stand here and there on open ground, built whole by
   the seed (`sites` on the scatter): a well, houses round it, some still lit.
   Each hamlet has a lamp post by its well, dark until Wick lights it, and lit
   it is a safe place; out in the wilds lighting lamps ends nothing
   (`tests/a_hamlet_lamp.rs`). The title's Play opens the wilds, the Dim Fields
   a button below it, and pausing resumes where Wick was
   (`tests/the_title_opens_the_wilds.rs`). Still to come: hamlets with people, and other places (ruins, camps, the beacon) appearing at random. Wick walks it in the dark; a test walks him across biomes.
2. **Under the Lid.** *Started.* Sunfalls: shafts of cracklight with a gold pool
   and dust in the beam, scattered by seed, wandering round where they fell and
   coming and going as their crack opens and shuts (`scripts/sunfall.decay`).
   Seekers (`scripts/seeker.decay`): one hangs over the meadow north-east of the
   start and sweeps it with a circle of red light; caught in it Wick is found,
   the circle follows him (slower than he walks) and the gloom hear it from two
   and a half times as far. It calls them: while it holds him a gloom rises out
   of the ground near him every few seconds, and when it loses him a wave rises
   where he was last seen (`prefabs/wilds/gloom.prefab`, never more than six
   about). A lit lamp's pool or an open sunfall hides him
   (`tests/under_the_lid.rs`). The ground has relief that the lantern rakes
   across (`blocks_n.png`). The lantern burns: a full one lasts three minutes
   away from other light, gutters in its last fifth and is an ember dry, and
   standing by a lit lamp or a fire tops it up (`Game.oil`, `burn` on Wick,
   `tests/the_lantern_burns.rs`). More Seekers hang over the open heath and
   barrens, scattered far apart by the seed (`prefabs/wilds/seeker.prefab`),
   none near the start. Still to come: what else being found calls down; the
   gloom keep to the dark. A test outlasts a Seeker's sweep by a fire.
3. **Gather and craft.** *Started.* Deadwood, flint and resin lie about the
   wilds, scattered by seed (`scripts/pickup.decay`, `art/goods.py`); Wick takes
   them by walking over them, and what is taken stays taken. His pack is on
   screen with a button for each thing he can build (`scripts/pack.decay`, F and
   G from the keyboard): a campfire (4 wood, 2 stone) that burns for a minute
   and a half, and a lamp (6 wood, 4 stone, 2 resin) that burns for good, built a
   step ahead of him. A fire he has built is a safe pool while it burns: the
   gloom are driven out of it and a Seeker cannot find him by it
   (`scripts/fire.decay`, `tests/gather_and_build.rs`). Resin feeds the
   lantern: a card (and R) pours one in for half a lantern. Still to come:
   tools, torches, and oil and wax as better fuel than resin.
4. **Survive.** *Started.* Courage (`scripts/courage.decay`, on the HUD under
   the lantern): it ebbs slowly in the dark, fast with the lantern dry, for
   each gloom near and while a Seeker has him, and comes back by a lit lamp, a
   fire or an open sunfall. Afraid, below half, the screen's edges close in and
   the world greys, the gloom notice him from further off and his walk falters
   (`tests/courage.rs`). Hunger: being fed runs down from full to starving in
   five minutes; berries grow on the meadow and the moss, and Wick eats one
   from his pack once he is below half fed, a cooked one by a fire going twice
   as far. Starving, his lantern's scorch does not mend and his courage drains
   (`tests/hunger.rs`). Still to come: more to eat and cooking proper; death
   and what it costs. A test lives through three long dark spells.
5. **Places.** The village set into the world with its houses, villagers to help
   and trade with, ruined lamp posts and the beacon. Saved worlds.
6. **The story and polish.** *Started:* the wilds are heard, in Sound Lab
   effects the project ships: Wick's footsteps as his walk's `step` events land
   them, gathering, building, feeding the lantern, lighting a lamp, a gloom's
   pounce and a Seeker finding him, under a slow, uneasy song
   (`audio/under-the-lid.sindrisong`; `tests/the_wilds_are_heard.rs`). Still to
   come: the Lid, the ending, ambience, the
   screenshots and site entry, and `docs/parity.md` and the capability pages
   brought level with what is played.

## Checks that hold throughout

- Every stage has a scripted run that plays it; one test plays from the title
  through the first Seeker's sweep. A goal that cannot be reached is a failing test.
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
