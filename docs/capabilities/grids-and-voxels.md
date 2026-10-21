<!-- Generated from docs/capabilities/grids-and-voxels.toml by `cargo run -p sindri-capabilities -- --write`. Edit the source, not this file. -->

# Grids and voxels

Renderer-independent grid geometry and navigation, and the engine-owned voxel
world: storage, generation, residency, meshing, collision, and the map view.
`docs/grid.md`, `docs/voxel-terrain.md` and `docs/voxel-world-plan.md` are the
contracts.

Each entry links its tests and the projects that exercise it. Surface marks match the row in [parity](../parity.md) where one is named: ✅ works and is exercised · 🟡 a useful slice, with the gap named · ❌ absent · — not applicable.

| Capability | Engine | Editor | Decay | Proof |
| --- | :-: | :-: | :-: | :-: |
| **Grid geometry** | | | | |
| [Orthogonal and isometric grids](#grid-geometry) | ✅ | ✅ | ✅ | ✅ |
| [Footprints, occupancy, walls](#grid-navigation) | ✅ | ✅ | ✅ | ✅ |
| **Voxel world** | | | | |
| [Engine-owned voxel coordinates and 16³ sections](#voxel-storage) | 🟡 | 🟡 | 🟡 | 🟡 |
| [Bounded voxel residency](#voxel-residency) | 🟡 | 🟡 | ❌ | 🟡 |
| [Neighbour-aware block meshing](#voxel-meshing) | 🟡 | 🟡 | ❌ | 🟡 |
| [Persistent voxel compiled/GPU mesh cache](#voxel-mesh-cache) | 🟡 | 🟡 | ❌ | 🟡 |
| [Voxel collision](#voxel-collision) | ✅ | ✅ | ✅ | 🟡 |
| [Voxel ground for gameplay](#voxel-gameplay-ground) | ✅ | — | ✅ | ✅ |
| [Voxel world viewed as a map](#voxel-map-view) | ✅ | 🟡 | ✅ | ✅ |
| [Reversible voxel map flooding](#voxel-map-flooding) | ✅ | 🟡 | ✅ | ✅ |
| **Not yet** | | | | |
| [Voxel persistence and digging](#voxel-persistence) | ❌ | ❌ | ❌ | ❌ |
| [Reusable voxel asset authoring](#reusable-voxel-assets) | ❌ | ❌ | ❌ | ❌ |
| [Per-path costs and policies](#path-costs) | ❌ | ❌ | ❌ | — |

## Grid geometry

<a id="grid-geometry"></a>

### Orthogonal and isometric grids

Signed cell coordinates, bounds, neighbour queries and validated orthogonal/isometric projection, shared by drawing and picking.

Engine ✅ · Editor ✅ · Decay ✅ · Proof ✅ — parity row *Orthogonal and isometric grids*

**Tested by:** [`crates/sindri-grid/src/geometry/tests.rs`](../../crates/sindri-grid/src/geometry/tests.rs), [`crates/sindri-decay/tests/grid_position.rs`](../../crates/sindri-decay/tests/grid_position.rs)

**Exercised in:** [`games/platformer`](../../games/platformer), [`games/low-tide`](../../games/low-tide)

**See:** [`docs/grid.md`](../../docs/grid.md)

<details markdown="1">
<summary>Evidence</summary>

`sindri-grid` provides renderer-independent signed cell coordinates,
continuous grid and plane points, finite rectangular bounds, deterministic
cardinal and eight-way neighbour queries, and validated orthogonal/isometric
projection. Cell centres and arbitrary fractional points round-trip through
both projections across negative and positive space. Upward/downward plane Y
adapts world and screen coordinate conventions without introducing a renderer
dependency.

`sindri.tilemap` exposes the exact `GridSpace` and `GridBounds` its rendering
and editor picking use. A map's complete transform is composed around that
local grid, so moving, rotating, or scaling a map keeps drawn and picked cells
in agreement. Decay can read an entity's continuous logical X/Y relative to a
tilemap and place it back through the same projection and map transform.

</details>

<a id="grid-navigation"></a>

### Footprints, occupancy, walls

Authored wall edges and multi-cell occupants resolved into one validated navigation snapshot.

Engine ✅ · Editor ✅ · Decay ✅ · Proof ✅ — parity row *Footprints, occupancy, walls*

**Tested by:** [`crates/sindri-scene/tests/navigation/authored.rs`](../../crates/sindri-scene/tests/navigation/authored.rs), [`crates/sindri-scene/tests/navigation/volume.rs`](../../crates/sindri-scene/tests/navigation/volume.rs), [`crates/sindri-grid/src/pathfinding/tests.rs`](../../crates/sindri-grid/src/pathfinding/tests.rs), [`crates/sindri-decay/tests/a_script_asks_what_is_walkable.rs`](../../crates/sindri-decay/tests/a_script_asks_what_is_walkable.rs)

**Exercised in:** [`games/low-tide`](../../games/low-tide)

<details markdown="1">
<summary>Evidence</summary>

`sindri.grid.navigation` adds authored internal wall edges without duplicating
the map's bounds or projection settings, while `sindri.grid.occupant` names its
grid by stable scene ID and carries a relative multi-cell footprint.
`WorldGridNavigation` resolves those references, derives anchors from current
world transforms, rejects conflicting or invalid placements as one incomplete
snapshot, and exposes validated placement and wall-aware path queries.

</details>

## Voxel world

<a id="voxel-storage"></a>

### Engine-owned voxel coordinates and 16³ sections

Signed voxel and section coordinates, palette-backed sections, deterministic generation, sparse edits and work queues.

Engine 🟡 · Editor 🟡 · Decay 🟡 · Proof 🟡 — parity row *Engine-owned voxel coordinates and 16³ sections*

**Tested by:** [`crates/sindri-scene/src/voxel/tests.rs`](../../crates/sindri-scene/src/voxel/tests.rs), [`crates/sindri-scene/tests/extraction/voxel_world.rs`](../../crates/sindri-scene/tests/extraction/voxel_world.rs), [`crates/sindri-voxel/src/terrain/tests.rs`](../../crates/sindri-voxel/src/terrain/tests.rs)

**Exercised in:** [`games/voxel-lab`](../../games/voxel-lab)

**See:** [`docs/voxel-terrain.md`](../../docs/voxel-terrain.md), [`docs/voxel-world-plan.md`](../../docs/voxel-world-plan.md)

<details markdown="1">
<summary>Evidence</summary>

`sindri-voxel` owns signed voxel and 16³ section coordinates, palette-backed
section storage, deterministic random-access generation, bounded 3D residency,
sparse edits, dirty tracking, and deduplicated generation/meshing work queues.
Mesh jobs carry a monotonic revision and meshing profile, and boundary changes
give the neighbour a new mesh revision even when its own stored voxels did not
change.

Scenes author this as `sindri.voxel_world`, with either a deterministic layered
generator or a natural terrain generator (continents, ranges, rivers, biomes,
caves, overhangs, water, snow and trees from one seed), per-face voxel material
textures, bounded horizontal/vertical residency, and a section focus. The same
scene path renders in games and in the editor Scene view. A world may instead
name a block set in `blocks` (the engine ships `builtin:blocks`, and new worlds
use it), and then its generator names blocks by name, drawn with each block's
per-face art; the generator's fields are `FieldMeaning::Block`, which the
editor offers as a menu of the set's blocks drawn as cubes. Without a block
set, material IDs are declared keys (`FieldMeaning::Key`) and the generator's
surface, subsurface, and deep layers reference them (`FieldMeaning::Block`
falling back to `KeyOf`, including the natural terrain's biome and optional
palette voxels, which also offer *None*), so the editor numbers an added
material past every ID in use, refuses to remove one a layer still names, and
offers the layers only the IDs that exist.

The editor's extractor tolerates an invalid voxel world or environment
(`SceneExtractor::tolerate_invalid_components`): the last valid terrain and
lighting keep drawing, and the failure is reported against its entity, naming
the environment field and the values it accepts. Games and capture tools stay
strict and fail the frame.

A world's `edits` list what has been changed from what was generated, cell by
cell; only changes are stored, an edit back to the generated block is dropped,
and an edit remeshes only the sections it touches.

</details>

<a id="voxel-residency"></a>

### Bounded voxel residency

A bounded resident window, optionally following the world camera, loads the sections it reaches and releases those it leaves.

Engine 🟡 · Editor 🟡 · Decay ❌ · Proof 🟡 — parity row *Bounded voxel residency*

**Tested by:** [`crates/sindri-scene/src/voxel/tests.rs`](../../crates/sindri-scene/src/voxel/tests.rs)

**Exercised in:** [`games/voxel-lab`](../../games/voxel-lab)

<details markdown="1">
<summary>Evidence</summary>

A world may keep its resident window under the world camera (`follow_camera`):
the window centres on where the middle of the picture meets the height of the
`focus` section, and moving it loads the sections it reaches and releases those
it leaves. Cached section bounds are tested conservatively against the current
camera after the voxel world's entity transform, so offscreen resident sections
retain their CPU cache without submitting draws or consuming their pending GPU
upload.

</details>

<a id="voxel-meshing"></a>

### Neighbour-aware block meshing

Blocks mesh as the boxes their tiles describe, dropping only fully covered faces, with a one-voxel halo across sections.

Engine 🟡 · Editor 🟡 · Decay ❌ · Proof 🟡 — parity row *Neighbour-aware block meshing*

**Tested by:** [`crates/sindri-voxel/src/mesh_tests.rs`](../../crates/sindri-voxel/src/mesh_tests.rs), [`crates/sindri-scene/tests/extraction/voxel_blocks.rs`](../../crates/sindri-scene/tests/extraction/voxel_blocks.rs), [`crates/sindri-scene/tests/extraction/voxel_surfaces.rs`](../../crates/sindri-scene/tests/extraction/voxel_surfaces.rs)

**Exercised in:** [`games/voxel-lab`](../../games/voxel-lab)

**Limits:**

- Only opaque block batches are bridged to the GPU; cutout/transparent pipelines and editor voxel picking/painting remain.

<details markdown="1">
<summary>Evidence</summary>

The block mesher samples a one-voxel halo through `VoxelSource`, so adjacent
solid sections omit their shared faces without requiring both to be resident.

Compiled block output is CPU-side indexed geometry in section-local
coordinates. It carries unit-square UV corners plus semantic voxel and face
identity, splits opaque, cutout, and transparent passes, and reports
world-space section bounds. Games provide `VoxelMaterialSource` policy for
render class and whether a voxel occludes every neighbour, only a matching
voxel, or none. Atlas lookup, texture bindings, and GPU upload deliberately
remain outside this crate.

A block from a block set is meshed as the box its tile describes — a slab's top
half way up, a post a thin column — and a neighbour's face is dropped only
where a block covers all of it. Each cell wears one of its block's variants,
chosen by its position and the world's `variant_seed` exactly as a tile volume
chooses them, and a block with something standing on it wears its `covered`
look. A block's faces may animate through frames and a block may glow (drawn
per batch through a `MeshSurface` UV offset and glow, with no remeshing);
blocks that do not hide their neighbours are cut out, and `liquid` ones hide
only their own inner faces.

</details>

<a id="voxel-mesh-cache"></a>

### Persistent voxel compiled/GPU mesh cache

CPU and GPU caches keyed by section and revision keep current geometry while replacements are pending.

Engine 🟡 · Editor 🟡 · Decay ❌ · Proof 🟡 — parity row *Persistent voxel compiled/GPU mesh cache*

**Tested by:** [`crates/sindri-scene/src/voxel_render/tests.rs`](../../crates/sindri-scene/src/voxel_render/tests.rs)

**Exercised in:** [`games/voxel-lab`](../../games/voxel-lab)

<details markdown="1">
<summary>Evidence</summary>

A generic `SectionMeshCache<T>` owns the renderer-independent lifetime
contract: block, smooth, hybrid, and custom representations are separate;
current geometry stays available while a replacement is pending; superseded
worker results are rejected; and one removal releases every profile of a
leaving section. `sindri-render` has the matching GPU-side contract for
textured meshes: opaque cache identities and monotonic revisions reuse
unchanged vertex/index buffers, retain the last buffers while replacement
geometry is unavailable, accept 32-bit indices, release departed entries
explicitly, and expose cumulative cache counters. `sindri-scene` is the
texture-aware seam between the two: it maps semantic voxel/face identities to
atlas regions, groups one section into deterministic texture batches, keeps
stable GPU identities across newer section revisions, ignores superseded
results, and releases every batch when a section leaves residency.

A resident voxel world keeps its compiled sections until its definition or the
textures its own faces resolve to change; binding an unrelated texture does not
rebuild it. Voxel Lab uses that component, while its browser route exposes the
persistent-cache and settled-remesh counters.

</details>

<a id="voxel-collision"></a>

### Voxel collision

`sindri.physics3d.voxel_collider` makes an authored voxel world solid near the bodies that can reach it.

Engine ✅ · Editor ✅ · Decay ✅ · Proof 🟡 — parity row *Voxel collision*

**Tested by:** [`crates/sindri-scene/src/physics_sync3d/voxel_tests.rs`](../../crates/sindri-scene/src/physics_sync3d/voxel_tests.rs), [`crates/sindri-scene/src/voxel_collision3d/tests.rs`](../../crates/sindri-scene/src/voxel_collision3d/tests.rs), [`crates/sindri-scene/src/voxel_collision3d/lifecycle_tests.rs`](../../crates/sindri-scene/src/voxel_collision3d/lifecycle_tests.rs), [`game/tests/a_block_lands_on_voxel_lab.rs`](../../game/tests/a_block_lands_on_voxel_lab.rs), [`scripts/browser/voxel-landing/observer.decay`](../../scripts/browser/voxel-landing/observer.decay)

**Exercised in:** [`games/voxel-lab`](../../games/voxel-lab)

**See:** [`docs/physics.md`](../../docs/physics.md), [`docs/physics-voxel-editor-review.md`](../../docs/physics-voxel-editor-review.md)

**Limits:**

- No game uses voxel collision since Causeway became a tactics game on model islands; the proof is a fixture over Voxel Lab.

<details markdown="1">
<summary>Evidence</summary>

`sindri.physics3d.voxel_collider` on a `sindri.voxel_world` entity resolves the
world's generator, palette and edits through `VoxelGround`, takes each block's
collision box from its block set (`collides`, default true, with the tile's own
height/extent bounds; water is usually `false`), and keeps resident only
the sections dynamic bodies can reach this step plus a margin.
`ScenePhysics3d::step_with_tile_sets` validates the voxel snapshot and the body
batch before committing either, and the shared game session and editor Play
pass their block sets. Causeway used it until it became a tactics game played
on model islands; now a fixture over Voxel Lab gives the world collision and
drops a block onto it, natively in `a_block_lands_on_voxel_lab.rs` and in an
exported build in a CI Chromium run. Scene regressions exercise landing, owner
events/queries, edited shafts under a sleeping body, distant residency,
non-colliding and slab blocks, release and atomic rejection, and the native
editor review authors the voxel collider, block-set
Collides and 3D bodies and lands them in Play.

`SceneVoxelCollision3d` reconciles explicit complete resident voxel snapshots
into static owner groups. Occupancy/policy revisions retain compiled geometry;
scale/settings changes reuse boxes, pose-only changes move the body, and
section exit or world inactivity/removal releases owned geometry. Complete
prevalidation and world/section/rebuild/piece limits reject invalid or
over-budget snapshots without publishing partial collision.

</details>

<a id="voxel-gameplay-ground"></a>

### Voxel ground for gameplay

`VoxelGround` answers gameplay's block, surface and ray questions; scripts build, dig, place and path on a voxel grid.

Engine ✅ · Editor — · Decay ✅ · Proof ✅

**Tested by:** [`crates/sindri-scene/src/extract/voxel_ground_tests.rs`](../../crates/sindri-scene/src/extract/voxel_ground_tests.rs), [`crates/sindri-scene/tests/placement/on_voxel_ground.rs`](../../crates/sindri-scene/tests/placement/on_voxel_ground.rs), [`crates/sindri-decay/tests/a_script_walks_and_builds_on_voxels.rs`](../../crates/sindri-decay/tests/a_script_walks_and_builds_on_voxels.rs), [`games/voxel-lab/tests/a_click_lands_on_what_it_points_at.rs`](../../games/voxel-lab/tests/a_click_lands_on_what_it_points_at.rs)

**Exercised in:** [`games/voxel-lab`](../../games/voxel-lab)

<details markdown="1">
<summary>Evidence</summary>

`VoxelGround` answers gameplay's questions from the same generator and edits —
the block in a cell, a column's surface, the first block a ray meets —
remembering generated sections between questions so edits never regenerate
terrain. A voxel world on a grid of boxes (`sindri.tile_grid` with `space:
"solid"`) is that grid's ground: its voxels are the grid's cells, centred on
them as a volume's blocks are, and the grid's calls answer from it. Scripts
read, build and remove blocks with `Grid.block`, `Grid.set_block` and
`Grid.tagged`; `sindri.grid.placement` stands things on its surface;
`Grid.can_reach` and `Grid.step_toward` path across it within the grid's
`max_step`, reading the ground around the walker and its goal; and
`voxel::aim_at_with` says which block and face the pointer is on.
`a_script_walks_and_builds_on_voxels.rs` builds, digs and walks round a wall on
a generated world, and Voxel Lab picks its blocks with it.

</details>

<a id="voxel-map-view"></a>

### Voxel world viewed as a map

`"view": "map"` draws a voxel world from straight above as height-lit squares in the ordinary sprite batches.

Engine ✅ · Editor 🟡 · Decay ✅ · Proof ✅ — parity row *Voxel world viewed as a map*

**Tested by:** [`crates/sindri-scene/src/extract/voxel_map_tests.rs`](../../crates/sindri-scene/src/extract/voxel_map_tests.rs), [`crates/sindri-scene/tests/extraction/voxel_map.rs`](../../crates/sindri-scene/tests/extraction/voxel_map.rs), [`games/low-tide/tests/the_basin_is_lit.rs`](../../games/low-tide/tests/the_basin_is_lit.rs)

**Exercised in:** [`games/low-tide`](../../games/low-tide)

**Limits:**

- Not drawn: decoration that holds nothing up, and a face's animation or glow.

<details markdown="1">
<summary>Evidence</summary>

A world with `"view": "map"` is drawn instead as a flat map from straight
above (`extract/voxel_map.rs`): each column one unit square wearing the top
face of its highest supporting block, read through the same `VoxelGround`
scripts use, so edits show and agree with gameplay. Squares are lit by height
(a slope against a north-western sun, paler high ground, water darker by
depth) and go into the ordinary sprite batches, so a map sorts and shares draws
with sprites and tilemaps. Only the columns under the world camera's four
corners are worked out, sixteen square at a time, capped at 48 chunks a side,
and remembered until the entity, its textures or its block set change;
switching a world to a map releases its meshed sections. `Grid.surface` and
`Grid.height` read a column's top from Decay (the latter `f32?`, `null` where
nothing stands). Extraction tests cover the squares, row direction, relief,
edits and the absence of meshes; Low Tide's Basin is the proof in a game.

</details>

<a id="voxel-map-flooding"></a>

### Reversible voxel map flooding

A map-only flood overlay with one continuous water height, set and read from Decay, that never edits terrain.

Engine ✅ · Editor 🟡 · Decay ✅ · Proof ✅ — parity row *Reversible voxel map flooding*

**Tested by:** [`crates/sindri-decay/tests/a_script_floods_a_map.rs`](../../crates/sindri-decay/tests/a_script_floods_a_map.rs), [`games/low-tide/tests/a_flood_season.rs`](../../games/low-tide/tests/a_flood_season.rs)

**Exercised in:** [`games/low-tide`](../../games/low-tide)

**See:** [`docs/voxel-terrain.md`](../../docs/voxel-terrain.md)

**Limits:**

- The editor Scene view is unverified.
- No fluid simulation, 3D flood mesh, decoration or animated/glowing map face.

<details markdown="1">
<summary>Evidence</summary>

A map-only `map_flood` overlay with `Grid.set_flood` and `Grid.flooded`: one
continuous water height, strict submersion against the column's existing
surface, a block palette top face and depth shading. Flood changes reuse cached
map columns; terrain edits still invalidate them. Terrain, biomes, 3D block
meshes and sparse edits are untouched, including builtin natural overland
terrain. This capability was found by the moving-home genre showcase, Low Tide,
whose season, cargo loss, waiting and escape rules are Decay gameplay. Native
headless tests exercise keys and phone touch; extraction tests exercise the
rendered faces and cache retention.

</details>

## Not yet

<a id="voxel-persistence"></a>

### Voxel persistence and digging

Sparse overrides survive residency changes in memory, but there is no save format or persisted tunnel proof.

Engine ❌ · Editor ❌ · Decay ❌ · Proof ❌ — parity row *Voxel persistence and digging*

<a id="reusable-voxel-assets"></a>

### Reusable voxel asset authoring

No project asset format or dedicated editor exists for bounded reusable voxel models such as trees, rocks or props.

Engine ❌ · Editor ❌ · Decay ❌ · Proof ❌ — parity row *Reusable voxel asset authoring*

<a id="path-costs"></a>

### Per-path costs and policies

A* treats every passable edge alike; there are no per-path costs or policies.

Engine ❌ · Editor ❌ · Decay ❌ · Proof — — parity row *Per-path costs and policies*
