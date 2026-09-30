# Spatial Query Lab

Open this directory as a Sindri project. Pages publishes it at
`/examples/spatial/`; the directory and homepage link it.

Move the cyan query origin with the arrow keys or hold an on-screen direction
button. The dashed circle shows the radius. Yellow is the nearest active
entity with the selected tag, even when it lies outside the radius. Green
targets are inside the radius. The bottom readout lists the actual query
snapshot in nearest-first order.

| Control | Key | Action |
| --- | --- | --- |
| Radius - / + | Q / E | Adjust by 0.5, bounded to [0, 3.5] |
| Tag | T | Cycle enemy, ally and a missing tag |
| A | A | Toggle target A's active state |
| Parent | P | Toggle the parent of C and D |
| Rotate C/D | O | Rotate their parent by 45 degrees |
| Reset | R | Restore origin, radius, tag, active state and parent rotation |

A and B begin equally distant; A appears first in stable world order. E sits
exactly on the initial radius boundary and is included. C and D are children
of a translated, rotated, scaled parent: the queries compare their composed
world positions. Gray untagged markers and letter labels stay visible when
their target or parent is inactive, so exclusion can be observed.

A tagged entity with no transform is also authored in the scene and omitted
from both spatial results. Selecting the missing tag shows `nearest: null`
and `within_radius: []`. Reducing radius to zero leaves nearest meaningful
while the radius snapshot is empty.

The gameplay calls `World.nearest(tag, Vec3)` and
`World.within_radius(tag, Vec3, radius)` directly. It does not implement another
distance scan or sort. Results use active authored tags, inclusive radius
boundaries, nearest-first order and stable world-order ties. Radius snapshots
refuse more than 8192 results; negative and NaN radii are errors. Positive
infinity is an unbounded query, though this UI deliberately exposes finite
radii. See the Decay scripting reference for the full contract.

The native regression opens the authored scene, compiles the scripts and
exercises ordering, boundary inclusion, parent transforms, active filtering,
null/empty results, reset and phone touch controls. CI also exports and plays
desktop and phone WebGPU builds under project and custom-domain base paths.
Spatial indexing, cone/box queries and physics queries are outside this demo.
