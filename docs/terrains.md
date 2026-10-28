# Terrains

A terrain paints ground rather than tiles. The sheet names, once, which sprite
each pattern of neighbors draws: grass on an open top, a rim on an open side,
plain earth in the middle. A brush or a script then paints `ground`, and each
cell, and every neighbor whose picture depends on it, gets the right sprite.

## In the sheet

```json
"terrains": {
  "ground": {
    "kind": "edges",
    "sprites": ["ground-0", "ground-1", "...", "ground-15"]
  }
}
```

| Kind | Sprites | Indexed by |
| --- | --- | --- |
| `edges` | 16 | Which of the four sides are the same terrain: north 1, east 2, south 4, west 8 |
| `corners` | 16 | Which of the tile's four corners are the terrain: north-west 8, north-east 4, south-west 2, south-east 1 (below) |
| `blob` | 47 | All eight neighbors (north 1, north-east 2, east 4, south-east 8, south 16, south-west 32, west 64, north-west 128), a corner counting only when both sides beside it do; listed in ascending order of those masks |

`edges` is enough for blocky ground. `blob` tells an inside corner from a
filled one, for art whose corners show. A terrain that lists the wrong number of
sprites, or a sprite the sheet does not cut, is refused when the sheet loads.

A cell belongs to a terrain when its sprite is one of the terrain's. Past the
map's edge counts as more of the same, so ground painted to the edge of a map
carries on out of the picture rather than ending in a rim there.

## Corners

A `corners` terrain is the layout of a Wang or Tiled corner set: two terrains,
such as grass and dirt, and one tile for each of the sixteen ways the four
corners of a tile can be one or the other. Sprite 0, every corner the ground,
is the plain ground tile, and belongs to the terrain like the others, so a map
filled with it is a map of ground.

What a corners tile draws is decided at its corners, not by its neighbors, so
there is no cell to paint. A corner is a grid line's crossing: a map of 3 by 2
tiles has corners 0 to 3 across and 0 to 2 down. Setting one rewrites the up to
four tiles that meet there. Each reads the corners it already has from the
sprite it shows, adds or removes this one, and draws the sprite for the result,
so the map holds nothing but its tiles. An empty tile reads as having no
corners and a tile showing any other sprite is left alone.

| Call | Returns |
| --- | --- |
| `Grid.set_terrain_corner(map, column, row, terrain, set)` | nothing |

`set` is `true` to make the corner the terrain and `false` to give it back to
the ground. `Grid.set_terrain` on a corners terrain, and this call on any other
kind, are errors that name the terrain.

## Painting

The tilemap inspector lists the sheet's terrains above the palette, each shown
by its fully surrounded sprite. With one picked, the brush paints that terrain:
the cell is set and it and its eight neighbors are redrawn from the patterns
they now make. Erasing on a map whose sheet has terrains redraws the neighbors
too, so a hole dug in a hill leaves its edges showing.

Decay paints the same way:

| Call | Returns |
| --- | --- |
| `Grid.set_terrain(map, column, row, terrain)` | nothing |

`""` empties the cell. A terrain the map's sheet does not declare is an error
naming it.

## Limits

One terrain per cell, with no transitions between two terrains (grass into sand)
except through a `corners` terrain, whose two terrains are the ground and the
terrain, and no random variants of the same pattern. The editor brush does not
offer a corners terrain; paint it from Decay. The patterns are fixed: there is no
rule editor for arbitrary neighbor conditions.

## Proof

Lamplighter's ground is a `corners` terrain, the first grass-to-dirt tileset Pixel
Lab generated: `assets/scripts/ground.decay` paints its dirt path corner by corner
with `Grid.set_terrain_corner`, and `games/lamplighter/tests/the_ground_is_painted.rs`
reads the tiles back.

The platformer's level is painted with the `ground` terrain of
`textures/tiles.sheet`: grass grows on every open top and the earth shows a rim
at every gap. `games/platformer/tests/painted_ground.rs` repaints every ground
cell and requires that nothing changes.
