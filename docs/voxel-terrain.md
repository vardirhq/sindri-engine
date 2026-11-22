# Natural voxel terrain

`sindri.voxel_world` can generate its world with `natural_terrain`: continents
and sea, mountain ranges, rivers, biomes, caves, overhangs, beaches, snow, ice
and trees, all decided by one seed. The generator is
`sindri_voxel::NaturalTerrain`; the scene document is
`sindri_scene::NaturalTerrainDocument`. Voxel Lab's scene is the worked example.

Every section is a pure function of the settings and its coordinate, so a
world is the same on every machine, sections can be generated in any order,
and generating one never depends on another.

## How a column is decided

1. **Shape.** A warped, low-frequency field says where land is. A folded ridge
   field, allowed only where a second "uplift" field is high, raises ranges
   along part of the world rather than everywhere. Small detail roughens both.
2. **Climate.** Warmth and moisture are slow fields of their own. Warmth falls
   with height, so ranges are colder than the plains around them.
3. **Biome.** A column takes the biome whose `temperature`/`moisture` point is
   nearest its climate, read through a little noise so edges fray into
   tongues and bays rather than following a line. Heights blend across biome
   edges by climate distance, so a biome's `relief` and `terraces` change the
   land gradually rather than at a cliff.
4. **Rivers.** Where a slow field crosses its middle, the lowlands are cut
   just below the sea, which fills them. Rivers fade out as the ground rises.
5. **Surface.** In order: the sea bed below sea level; a beach a course or two
   above it; snow above the snow line; bare rock above the tree line or on
   slopes of three voxels or more; otherwise the biome's own ground. Every
   height threshold is frayed, as Causeway's are.
6. **Volume.** In the heart of a range, a band of three-dimensional noise
   around the surface undercuts faces into ledges and overhangs. It only
   takes rock from a few voxels into the side of a wall thick enough, both
   ways across, to keep a core, so a thin ridge or spur is never cut through
   into a window. Tunnels run where two fields both cross their middle, and
   caverns open deep below the sea. Neither breaks the surface except in the
   ranges, where a tunnel coming out is a cave mouth, and a tunnel never runs
   through a wall too thin to hold it.
7. **Water and trees.** Open voxels at or below sea level are water, and the
   surface freezes to ice where it is cold. Trees stand at most one to a
   6×6 square, on a biome's own gentle ground, as often as its `trees`
   says; cold biomes grow conifers.

## Authoring

| Field | Meaning |
| --- | --- |
| `seed` | The world. Another seed is another world. |
| `sea_level` | The level water fills to. |
| `relief` | How far above the sea the highest ranges reach (1–1024). |
| `feature_size` | Width of the largest landforms in voxels (8–4096). Continents are a few times this. |
| `tree_line`, `snow_line` | Heights above the sea where ground turns to bare rock, then snow. |
| `caves`, `rivers` | Whether to cut them. |
| `stone_voxel` | Everything below a biome's subsurface. Required. |
| `water_voxel`, `beach_voxel`, `sea_bed_voxel`, `cliff_voxel`, `snow_voxel`, `ice_voxel`, `trunk_voxel`, `leaves_voxel` | Optional. `null` leaves that feature out: no water leaves the sea empty; trees need both trunk and leaves. |
| `biomes[]` | `name`, climate point (`temperature`, `moisture`, 0–1), `surface_voxel`, `subsurface_voxel`, `subsurface_depth`, `trees` (0–1), `relief` (0–4, a multiple of the world's hills), and `terraces` (step height; 0 for none). |

Every voxel field names a block. A world that sets `blocks` to a block set
(`builtin:blocks`, or a project's own `.tileset`) names blocks from it by
name — `"surface_voxel": "grass"` — and the engine numbers the set's blocks
itself; each face draws with the block's art, a tile's south face being the
voxel's front. A world with no block set names its own `materials` by number,
as every world did before block sets, and still loads unchanged. In the
inspector each field is a menu of the set's blocks drawn as cubes (or of the
world's materials), and the optional ones also offer *None*. Switching a
world's generator between `layered_terrain` and `natural_terrain` writes the
arriving generator's fields and drops the other's. A new Voxel World component
starts as a natural terrain built from `builtin:blocks`: seven biomes, water,
beaches, cliffs, snow, ice and trees.

## Viewed as a map

A voxel world with `"view": "map"` is drawn flat, from straight above, as a
map rather than as blocks: one unit square per column, wearing the top face of
the column's highest block that holds anything up (so water and leaves show,
a flower does not). Column X runs across the map and row Z runs down it, the
way a tilemap's rows run and the way `Grid.block` names cells.

Height is drawn as light, from a sun low in the north-west: a column above its
north-western neighbour is lighter, one below it darker, and a cliff a hard
line; higher ground is a little paler, and water darkens with its depth. Only
the columns the world camera can see are worked out, sixteen square at a time,
and remembered until the world, its textures or its block set change.

Everything else is the blocks view's: the generator, the block set and its
variants, the edits, and the ground scripts read. `Grid.surface` and
`Grid.height` answer what is on top of a column and how high it is, which is
what a top-down game needs. Low Tide's Basin is the worked example: a drained
sea floor from these same settings, with its own blocks and biomes.

With `"soft_edges": true`, the seams between different kinds of ground fray
rather than running along the grid. Along each seam, short strips (an eighth of
a square long) of one ground are drawn a few pixels (up to six sixteenths of a
square) into the square beside it, following a wave from the world's
`variant_seed` that is pinned to the grid at both ends, so the lines of
neighbouring seams join. A strip wears its ground's own picture as it would
continue across the seam and the light of the square it lies on. Ground frays
into water, so banks are ragged too, but water never creeps onto ground, so
what shows as water is somewhere a walker cannot stand. Only the drawing
changes: blocks, `Grid.surface` and walking are as before. Lamplighter's wilds
use it.

### Water that moves: `map_water`

With `map_water`, a map's water is drawn through the engine's own water shader
rather than as a still picture. A square is water when its top is one of
`blocks` (by name, `["water"]` by default). Its colour runs from `shallow` to
`deep` (linear RGB) by how many blocks of water lie under it, up to `depth`, and
is paler near the land whatever is under it, so a river's banks and a sea's
shelf read as shallows. The block's own picture shows through as the detail.

The rest is movement, drawn in whole pixels of the block's picture so the water
stays as crisp as the ground beside it: rows of the surface sway a pixel or two
to and fro and light ripples drift across it (`waves`), a pixel here and there
on a crest flashes and is gone (`glints`), and foam laps up and back along the
land in each square that touches it (`foam`), each 0 for none and 1 for
plenty, all at `speed` (1 is a calm lake's pace). Water is lit by 2D lights like
the ground, so at night glints flash only where a lantern reaches them. Only the
drawing changes: what a walker can cross is as before. Lamplighter's wilds use
it.

The land at the water's edge is drawn darker, wet (`wet`, 0 to 1): fully beside
the water, half at a corner of it. What stands by the water (any billboarded
picture) is mirrored in it (`reflections`, 0 to 1): upside down from its foot,
a little shorter than it is tall, darker and bluer, fading along its length and
wavering a pixel by rows. The mirror image is cut along the map's squares and
only the pieces over water are drawn, after the water and before the frayed
banks, so it shows in the river and not on the grass.

### Drops you can see: `map_cliffs`

A map is drawn flat, so a terrace or a cliff is otherwise only a darker line.
With `map_cliffs`, where a square stands two levels or more above the one south
of it (nearer a camera that looks north), the face of that drop is drawn as a
band across the top of the lower square: the higher block's side, in shade, a
little taller for each level, up to most of a square. A gentle slope that steps
down a level at a time is left alone, so it is not striped.

```json
"map_water": {"blocks": ["water"], "shallow": [0.1, 0.34, 0.38],
              "deep": [0.015, 0.05, 0.14], "depth": 6,
              "waves": 0.7, "glints": 0.6, "foam": 0.7, "speed": 1.0,
              "wet": 0.3, "reflections": 0.9}
```

## What stands on it: scatter

A voxel world may carry `sindri.scatter` to be dressed with what stands on it:
trees in one biome, boulders in another, ground cover everywhere. Each rule
names the blocks it grows on (`on`, by name), the prefabs it makes (one chosen
by the seed for each place), how far apart things are (`spacing`, in squares:
the land is cut into patches of that side and each may hold one) and how likely
each patch is to hold one (`chance`), and whether a thing may be mirrored
(`flip`). Where two rules choose one square, the first rule's thing stands.
`clearings` (a column, a row and a radius, in squares) are left bare: where
play starts, round what is placed by hand.

Where each thing stands is the seed's answer, so the same land is dressed the
same way every time and nothing about it is stored. The runtime makes the
things within `reach` squares of play (the entity named by `around`, or the
middle of the world camera's view) from their prefabs, and unmakes them a few
squares further out, so a land as wide as the generator costs what is near and
walking back finds the same trees where they stood. They are ordinary entities:
they stop a walker, cast shadows and are lit like anything placed by hand. One
a script despawns was taken, and the scatter remembers it in `taken` so it does
not grow back. Lamplighter's wilds are dressed this way.

### Sites: places stood whole

A scatter's `sites` stand a layout rather than one thing: a hamlet, a ruin, a
camp. Each names its kind (`name`), the blocks its middle stands on (`on`), how
far apart sites are (`spacing`, in squares: one to a patch of that side at most),
how likely a patch is to hold one (`chance`), how far it reaches from its middle
(`radius`, in squares) and its `pieces`: each the prefabs it may be, where it
stands from the middle (`at`, squares east and north) and how likely it is to
stand (`chance`).

The seed chooses where in a patch a site's middle is, and a middle is only taken
where the site's own ground is under it and every piece's square and the land
round it out to its radius can be walked, so a hamlet is not built across a
river; a few places are tried before the patch is given up. The seed also
chooses whether the layout is mirrored, which pieces stand and which prefab each
is, so no two sites of one kind are quite alike. Nothing a rule grows stands
within a site, and a piece is made, unmade and remembered as `taken` like
anything else the scatter stands. `sindri_scene::scatter_sites` answers where
sites are without making them, for a map or a script that wants the nearest one.
Lamplighter's wilds build hamlets this way: a well, houses round it, some still
lit, a woodpile, a cart.

## Walls where the map cannot be walked

A voxel world drawn as a map is only a picture to 2D physics, so a walker would
cross its rivers and its sea. `sindri.map_walls` puts that right near play: the
runtime raises a static 2D box over every run of squares in a row that a walker
cannot stand on (water, lava, nothing at all) within `reach` squares of play
(the entity named by `around`, or the middle of the world camera's view), on the
collision layers `memberships` and `filter` name, sixteen squares by sixteen,
and takes a chunk down once play is a chunk past its reach. They are ordinary
entities named "Map wall"; Lamplighter's wilds stop Wick at the water this way.

## A land as wide as the generator: the floating origin

The generator answers for any column, and the map, the scatter and the walls
only work out what is near play, so a land goes on as far as anyone walks,
costing what is near. What runs out first is precision: positions are 32-bit
floats, good to about a hundredth of a unit a hundred thousand units out, and a
walk's physics stutters long before that. `sindri.floating_origin` keeps play
near the origin instead: once the entity it follows (`around`) is further than
`distance` from it on either axis, the runtime moves the whole world back by
whole multiples of `step`, camera and particles in flight included, and keeps
the total in `moved`. A land's squares are counted from the land's own placing,
which moved too, so the generator, the scatter, the walls and what was taken
all agree, and a land as wide as the generator's integer squares (millions
across) is as sharp at its far edge as at its start. Screen elements stay where
they are. A script that keeps a position of its own across steps should keep it
relative to something in the world, or add `moved` back.

## Built worlds

`"generator": { "kind": "built" }` generates nothing: the world is only the
blocks placed in it, its `edits`, so a ship, a station or a wreck is written
block by block. It draws, collides and is edited like any other voxel world,
and its entity's transform places, turns and scales it: at a scale of 0.5 its
blocks are half a metre. With `sindri.physics3d.voxel_body` beside it, the
whole structure is one rigid body (`docs/physics.md`).

## Blocks

`builtin:blocks` is the block set the engine ships, with its art compiled into
`sindri-assets`: grass, dirt, earth, stone, rock, sand, snow, ice, mud, moss,
gravel, clay, planks, log, leaves, water and lava. `builtin:` references are
not asset IDs (a colon cannot appear in one), so no loader looks for them on
disk: a host binds them from `sindri_assets::builtin_textures()` and
`builtin_tile_sets()`, as it binds `procedural:` textures, and the exporter
leaves them out of a build. The editor binds them for every scene.

A block set is an ordinary tile set, the same asset a tile volume uses, so a
block defined once serves both. Selecting one in the Project panel opens the
block set editor: the set's blocks as cubes, and the chosen block's name, a
texture picker for each face (an empty face borrows the opposite one), whether
it hides its neighbours, supports, is walkable, its height and its tags.
*New block set here* starts a project's own set as a copy of the built-in one.
Tags are words a game gives a block (`hot`, `liquid`) that the engine never
reads and scripts ask about with `Grid.tagged`.

A face can move and a block can glow. A face's `animation` lists further frames
(sprites the same size on the same texture as its first) and a speed; the
faces are meshed once, with the first frame, and the renderer shows the others
by shifting where they read, so a lake ripples without being rebuilt. A
block's `glow` lights it itself: shadow takes none of it away, and the bloom
pass picks up what exceeds full brightness. The built-in water and lava are
8-frame loops, and lava glows. Hosts pass the scene's time as
`SceneRuntime::with_seconds`; at zero every animation shows its first frame.
The editor runs its own clock and redraws while a scene has animated blocks.

Whether a block hides its neighbours decides how it is meshed. One that does
(most blocks) is opaque. One that does not is cut out, holes and all, like
leaves; one that does not and is tagged `liquid` hides only the faces between
its own blocks, so a lake has no walls inside it.

An invalid value is reported against the field, for example
``voxel generator `biomes.2.trees` must be from 0 to 1``, and the editor keeps
drawing the last valid world meanwhile.

### Modeled blocks

A tile's optional `"model": "models/vent-block.glb"` replaces its face
geometry in the 3D voxel blocks view. Coordinates are in cell units with
the lower corner at `[0, 0, 0]` and the upper corner at `[1, 1, 1]`.
The transformed GLB geometry must be nonempty and fit within those bounds.
Use a static, opaque GLB with embedded images. Face sprites remain required
for map and tile views, and the block must have full-cell bounds. Collision
continues to use that cell and respects `collides`.

The exporter follows models referenced by the block set, and the desktop
editor requests them when the set loads. Missing or invalid models produce
asset diagnostics. Tolerant hosts draw the face-textured box until the model
is available. The Android editor does not yet bind models. Each resident
section combines repeated uses of a GLB into indexed batches by material.
Batches share the source GLB's images and GPU textures. Model geometry follows the
world owner's transform and uses the section's residency and culling.
Modeled blocks draw all their geometry and leave their neighbors' faces
visible, including across section boundaries. DRIFT's shuttle exercises
this with recessed vents that are generated alongside its block set.

## Not yet

Water is drawn opaque: a liquid hides only its own inner faces, but light does
not pass through it, because blended rendering needs a sorted pass the voxel
renderer does not have yet. A block's variants are not yet read by the voxel
mesher, and tile volumes do not yet animate or glow. There are no strata, ores
or structures, and water does not flow. Causeway's world is this generator with
its own blocks.

## Map flood overlay

A voxel world with `view: map` may carry `map_flood: {"level": -4.0,
"block": "brine"}` (or `null`). Below that continuous voxel height, the map
draws the chosen block's top face shaded by water depth; equal height remains
dry. `Grid.set_flood` changes it and `Grid.flooded` reads the same threshold.
This is a map presentation capability found by Low Tide, not hydrodynamics or
3D water: blocks views and all terrain/block queries stay unchanged. It works
with custom Basin blocks and builtin overland blocks.

Changing the generator's sea level would change generation and biome shorelines
and regenerate visible columns. The overlay instead retains generated sections,
sparse edits and map column chunks, changing the face chosen during submission.
Terrain edits, texture reloads and tile-set reloads still invalidate the map.
Flooded empty columns draw nothing. Decorations and animated/glowing water retain
the map view's existing limitations. The editor's Scene/Play surface remains
unverified for this capability; there is no dedicated flood tool.
