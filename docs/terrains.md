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
| `blob` | 47 | All eight neighbors (north 1, north-east 2, east 4, south-east 8, south 16, south-west 32, west 64, north-west 128), a corner counting only when both sides beside it do; listed in ascending order of those masks |

`edges` is enough for blocky ground. `blob` tells an inside corner from a
filled one, for art whose corners show. A terrain that lists the wrong number of
sprites, or a sprite the sheet does not cut, is refused when the sheet loads.

A cell belongs to a terrain when its sprite is one of the terrain's. Past the
map's edge counts as more of the same, so ground painted to the edge of a map
carries on out of the picture rather than ending in a rim there.

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
and no random variants of the same pattern. The patterns are fixed: there is no
rule editor for arbitrary neighbor conditions.

## Proof

The platformer's level is painted with the `ground` terrain of
`textures/tiles.sheet`: grass grows on every open top and the earth shows a rim
at every gap. `games/platformer/tests/painted_ground.rs` repaints every ground
cell and requires that nothing changes.
