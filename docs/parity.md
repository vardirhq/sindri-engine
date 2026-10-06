# Parity

Last audited against `main`: **2026-09-27**.

What a game engine is expected to do, what Sindri actually does, and the
distance between them. This file replaces `function-matrix.md` and
`feature-integration-matrix.md`, which are deleted: both graded Sindri against
Sindri, so a feature nobody had thought of had no row and could not show as
missing.

That failure was not hypothetical. The integration matrix recorded 2D physics as
**Ready** — masked runtime, body kinds, sensors, validation, velocity, impulse,
all true — while a character could not be given a capsule with a circle at each
side, because a collider was singular from the scene file down to
`BodyRecord2d`. "Ready" meant *complete against what we built*, not *sufficient
for a game*. A table with an outside edge would have carried a `Compound
colliders` row reading ❌ from the first day it existed.

So this table is written from the outside in. The baseline column is what a
person arriving from Unity or Godot expects to find. Rows exist for things we
have never built and have no plan to build, because a gap with no row is a gap
nobody schedules.

## How to read it

Four surface columns, then the outward judgement:

| Column | Question |
| --- | --- |
| **Engine** | Does the runtime do it? |
| **Editor** | Can somebody author it without editing JSON by hand? |
| **Decay** | Can a script reach it? |
| **Proof** | Does a project in this repository use it, and which one? |

**Surface legend:** ✅ works and is exercised · 🟡 a useful slice, with the gap
named · ❌ absent · — genuinely not applicable to that surface.

**Parity legend:**

- **Ahead** — we do this better than the baseline, on purpose. Protect it.
- **Par** — a competent equivalent exists.
- **Behind** — the same idea, less of it.
- **Absent** — the baseline has it and we have nothing.
- **Won't** — a deliberate decision not to have it. See the anti-goals.

A ✅ still means what it always meant: implemented *and* exercised, not
represented by a type, a schema, or an editor control.

**What counts as proof** follows the three kinds of project in `AGENTS.md`.
For a gameplay capability, exercised means a game uses it: Causeway, Orbital
Last Stand, or a genre showcase (`games/<genre>` — the platformer and Scorchball
today). A feature example (`examples/<feature>`) or a lab such as Voxel Lab
shows a capability on its own; that is a partial proof (🟡) for a rendering or
tooling row, where isolation is the honest test, and not a proof for a gameplay
one. Either way the gap cell names the project, so removing a project shows
which rows it was holding up. Gather's removal is why this rule exists: it left
eight rows citing a game that no longer existed.

The layout still lags the three kinds. `examples/` holds `camera`, `cube` and
`triangle`, three feature demos — `voxel-lab`, `weave-poc` and `shapes-lab` —
live under `games/`, and most of the feature examples `AGENTS.md` names (audio,
Weave, Decay, tilemaps, isometric, voxels) do not exist as examples yet. Moving
and filling them is follow-up work, not a change to any cell here.

**Ranking is by "does this stop somebody shipping a game", not by feature
count.** Unity has thousands of features and most of them do not matter. The
ranked queue at the end of this file is the actionable output; the tables are
the evidence behind it.

## Where Sindri is already ahead

Written down first, and deliberately, because parity work is the most likely
way to lose these. Every row below is somewhere copying the baseline would be a
regression.

| We do | They do | Why ours is better |
| --- | --- | --- |
| Components self-describe through `ComponentSchemaRegistry`; templates are checked against serde at startup | Unity's inspector is hand-written per component; drift is found by a user | A template that has drifted from its struct is a startup error, not a row quietly missing from a panel a release later |
| Capability rule: docs and a real game use, in the same change | Features ship, then rot unproven | `capabilities.md` describes what a game actually did, not what an API allows |
| Scenes are readable single files; no sidecars, no GUIDs | `.meta` files, GUID churn, unmergeable scenes | Two people can edit a scene and resolve it in a normal diff |
| Determinism by default: a written-out PCG stream, replayable from a seed on every host, no platform entropy | `UnityEngine.Random` is not contractually stable across versions or platforms | A seed reproduces a run, which is what a roguelike and a bug report both need |
| Decay and Weave designed alongside the engine | C# bolted to a C++ core; UI Toolkit retrofitted after two prior UI systems | One scripting surface and one presentation model, both generated from the same registry |
| Weave: responsive stylesheets for game UI, composed and hot-reloaded | No native equivalent; USS is the nearest and arrived late | Game UI that adapts to a phone and a desktop from one authored source |
| Isometric and grid gameplay in the engine: footprints, occupancy, walls, A\* over whole footprints | Plugins (A\* Pathfinding Project, tile extensions) | The thing most 2D games need is not a third-party dependency |
| Validation refuses at the boundary and names what failed, including the index of the offending collider piece | Silent clamping, silent nulls | A bad value is a named error, not a mystery at frame 4000 |
| Fixed-step loop with input edges consumed exactly once | Update/FixedUpdate confusion, input polled in the wrong phase | Input cannot be missed or double-counted |

## What we will not copy

The advantage of a foundation this new is that its mistakes are still
avoidable. These are anti-goals; a PR that drifts toward one should be stopped
by this list rather than discovered a year later.

- **No sidecar metadata files or asset GUIDs.** Identity comes from the path
  inside the project. Renames are the price; unmergeable scenes are not.
- **No serialization by field-name reflection.** Renaming a Rust field must not
  silently drop authored data. Schema versions and explicit decoding instead.
- **No second render pipeline.** Unity has three that split the ecosystem in
  half. One path, extended.
- **No second input system.** The action layer already in `sindri-platform`
  becomes *the* input system; it does not grow up beside the old one.
- **No prefab variant hierarchy.** Prefabs plus overrides through the reference.
  Nesting a prefab inside another is not a variant and is supported; a prefab
  that is "another prefab, but different" is not. If variants ever seem
  necessary, prove it with a game first.
- **No assembly-definition graph.** Crate boundaries in `dependency-policy.md`
  already do this job.
- **No inspector that needs a plugin to be usable.** Odin exists because
  Unity's is not sufficient. Ours has to be sufficient — see the ranked queue.
- **No editor-only code path.** What the editor plays is what the build runs.
- **No silent defaults for things the engine cannot invent.** A component that
  names a font, a sheet, or a clip has no blank; the registry already
  distinguishes a field template from a fresh default, and that distinction stays.

---

## Stranded capability

A class the old matrices could not express: **built, working, and unreachable.**
Neither of these appeared in either matrix, in any column, because nobody had
written the row.

| Capability | State | Why it is stranded |
| --- | --- | --- |
| **Scroll input** | `Source::ScrollX/ScrollY` are bound and parsed; the wheel now scrolls `sindri.ui.scroll` regions | Still not a Decay reading of its own, so a game cannot zoom or cycle with the wheel. |

Stranded capability is the cheapest work in this file: the engine cost is
already paid and only the reach is missing. It is also the most invisible, which
is why it earns a section rather than a footnote.

---

## Scenes, entities, and prefabs

| Feature | Engine | Editor | Decay | Proof | vs. baseline | Gap that matters |
| --- | :-: | :-: | :-: | :-: | --- | --- |
| Entities and hierarchy | ✅ | ✅ | ✅ | ✅ | **Par** | Undo entries for script writes |
| **Child transforms follow their parent** | ✅ | ✅ | ✅ | ✅ | **Par** | A child's transform is local to its parent for sprites, meshes, tiles, voxels, cameras, lights, physics, effects and the editor's handles, not only shapes. Scripts read `world_position`. Scorchball's markers, signposts and ball ring ride their parents. Low Tide's crew walks a tilemap deck that drives and turns under them, in the deck's own coordinates, and re-parents between the deck and the world at its ramp. Scale composes per axis, so a rotated parent scaled unevenly does not shear its children |
| Tags and queries | ✅ | ✅ | ✅ | ✅ | **Par** | Query by more than one tag |
| Deterministic gameplay nearest / radius queries | ✅ | — | ✅ | ✅ | **Par** | Authored-tag queries use composed world positions; radius results are nearest-first with world-order ties. Orbital player and Arc retain their gameplay filters. Spatial Query Lab exposes ordered results, tag/active controls and parent transforms on Pages. Cone/box queries and indexing remain |
| Prefabs | ✅ | ✅ | ✅ | ✅ | **Par** | A scene places a prefab as an instance: the reference plus its overrides, each a JSON merge patch keyed by the prefab's own entity IDs, worked out on save as the difference from the prefab rather than tracked. An edit to the prefab reaches every instance, at once in an open editor as one undo step that keeps overrides and handles. The editor places an instance on a grid cell or in the middle of the Scene view, makes a prefab from a subtree, opens a prefab to edit it, and shows each instance's overrides with revert, apply and unpack; script `Prefab` fields have a picker. The platformer's ten coins are instances of one prefab. An instance can also do without its prefab's entities, patch a list by index, and keep its inner entities' editor state; a prefab is dragged from the browser into the view; Apply and Make prefab are undone with the scene, files included; and a scene whose prefab is missing opens with placeholders that save back unchanged. The editor's panels have no automated coverage; the operations under them do |
| **Nested prefabs** | ✅ | ✅ | ✅ | ✅ | **Par** | A prefab places instances of others; a scene reaches inside one by path (`loot/sparkle`), a script's spawn makes the whole nest, and a loop is refused naming it. Apply writes an override into the outer prefab rather than changing the inner one. No variants, deliberately — see the anti-goals |
| Reusable data profiles | ✅ | ✅ | ✅ | ✅ | **Ahead** | Unity has no native equivalent; ScriptableObject is close but needs code per asset. Optional schemas when a second catalog proves the shape |
| Transform | ✅ | ✅ | 🟡 | ✅ | **Par** | No structured vector or rotation value in Decay |
| Scene save / load | ✅ | ✅ | — | ✅ | **Par** | Readable single file, which is **Ahead**; see the advantages table |
| **Multiple scenes / additive loading** | ✅ | 🟡 | ✅ | ✅ | **Behind** | `World::add_scene` loads a scene beside the ones a world already holds; `LoadedScenes` keeps which one is played and switches between them, leaving everything a scene holds as the player left it. `Scene.go`/`Scene.current` let a script ask, `[project] scenes` declares them and the exporter walks each one for its own assets. `Scene.go` now switches scenes in the browser as well as natively, so a title screen reaches its game on both targets. The editor's Scenes panel shows the project's scenes as a board — a card each, with the last frame drawn of it, arrows for each literal `Scene.go` door, and a warning on a door to a scene the project does not carry — and adds, removes, orders and nominates them. A world holding several scenes does not round-trip through `to_scene` |
| Scene streaming / Addressables | ❌ | ❌ | ❌ | ❌ | **Absent** | Terrain chunks now stream inside one scene, but scene/asset streaming remains absent |
| Undo / redo | ✅ | ✅ | — | — | **Par** | Command-backed; script writes are outside it |

## Assets and the import pipeline

| Feature | Engine | Editor | Decay | Proof | vs. baseline | Gap that matters |
| --- | :-: | :-: | :-: | :-: | --- | --- |
| Asset IDs, manifests, async load | ✅ | ✅ | 🟡 | ✅ | **Par** | No general typed asset value in Decay |
| Project browser: import, rename, copy, delete, preview | — | ✅ | — | ✅ | **Par** | — |
| Content-hashed static export | ✅ | ❌ | — | ✅ | **Par** | No editor export workflow |
| Hot reload | ✅ | 🟡 | 🟡 | ✅ | **Ahead** | Native reload works; Unity's domain reload is a byword for slow |
| **Per-asset import settings** | ❌ | ❌ | — | — | **Absent** | No filter mode, mip, wrap, or compression choice. Every texture is imported one way |
| **Texture compression** | ❌ | ❌ | — | — | **Absent** | Ships raw. Fine at current scale, a problem for a real download |
| **Build-time atlas packing** | ❌ | 🟡 | — | — | **Behind** | The slicer authors sheets by hand; nothing packs loose sprites automatically |
| `referenced_audio` gather | ❌ | — | — | — | **Behind** | Hosts cannot infer every clip a scene needs, unlike `referenced_textures` |

## Voxel worlds and terrain

The ordered work required to close these gaps is tracked in
[`voxel-system-completion-checklist.md`](voxel-system-completion-checklist.md).

| Feature | Engine | Editor | Decay | Proof | vs. baseline | Gap that matters |
| --- | :-: | :-: | :-: | :-: | --- | --- |
| Engine-owned voxel coordinates and 16³ sections | 🟡 | 🟡 | 🟡 | 🟡 | **Behind** | `sindri-voxel` owns signed world/section/local coordinates, palette-backed sections, air/material IDs, revisions, and deterministic random-access generation. `sindri.voxel_world` makes the engine world visible in authored scenes and Voxel Lab now opens that path in the editor. On a grid of boxes it is the grid's ground: scripts read, build and remove its blocks through `Grid.block`/`set_block`/`tagged`, placement stands things on it, `Grid.step_toward` paths across it and the pointer aims at its blocks. Causeway is played on it. |
| Bounded voxel residency | 🟡 | 🟡 | ❌ | 🟡 | **Behind** | Engine world tracks entering/staying/leaving 3D sections, keeps render and simulation radii distinct, preserves sparse edits across unload/reload, and exposes deterministic deduplicated generation/mesh work queues. The scene component authors a bounded 3D window and Voxel Lab exercises it; `follow_camera` keeps the window under the world camera. Work is drained synchronously. |
| Voxel dirty-region tracking | 🟡 | ❌ | 🟡 | ❌ | **Behind** | A voxel edit dirties its section plus a neighbouring section when the edit touches that boundary, and queues resident affected sections with new monotonic mesh revisions. A scene's edits are the component's `edits` list, so a script's `Grid.set_block` remeshes only the sections it touches. Causeway's planks are such edits. |
| Neighbour-aware block meshing | 🟡 | 🟡 | ❌ | 🟡 | **Behind** | Engine block mesher samples through `VoxelSource` across section boundaries, applies game-provided material/occlusion policy, and emits indexed section-local geometry split into opaque, cutout, and transparent passes. The scene/editor bridge renders the opaque path with atlas-neutral per-face materials in Voxel Lab; cutout/transparent rendering and Causeway migration remain. |
| Persistent voxel compiled/GPU mesh cache | 🟡 | 🟡 | ❌ | 🟡 | **Behind** | `SectionMeshCache<T>` keys compiled representations by section and block/smooth/hybrid/custom profile, retains old geometry during replacement, rejects stale results, and can release all profiles named by a leaving residency delta. `sindri-render` caches textured GPU buffers by opaque identity and revision, while the scene bridge maps semantic voxel faces to stable texture batches, releases departed sections, and conservatively culls transformed section bounds before draw submission. Voxel Lab exercises this through both the editor scene and browser instrumentation; non-opaque pipelines remain. |
| Voxel world authoring | 🟡 | 🟡 | ❌ | 🟡 | **Behind** | Voxel Lab can inspect the engine-owned world and exercises engine picking. The inspector keeps the material list valid -- new materials get unused IDs, a material the generator uses cannot be removed, and generator layers choose from defined materials -- and an invalid definition keeps the last valid terrain drawing instead of freezing the view. But a project author cannot yet use the Scene view as a complete terrain tool. Camera-driven residency, material/block palette painting, place/remove strokes, selection/fill, slice/cutaway views, undoable world edits, and saved edit deltas remain. This is editing a world, not creating a reusable voxel object. |
| Reusable voxel asset authoring | ❌ | ❌ | ❌ | ❌ | **Absent** | No project asset format or dedicated editor exists for bounded reusable voxel models such as trees, rocks, buildings, furniture, machines, or item-like props. The intended editor is a small orbitable voxel workspace with block/material palette, place/remove/paint tools, undo/redo, preview, save/load, and a placement path into scenes/worlds. These assets must be ordinary project assets rather than game-specific Rust data. |
| Voxel persistence and digging | ❌ | ❌ | ❌ | ❌ | **Absent** | Sparse overrides now survive engine residency changes in memory, and the engine's natural terrain generates a real vertical volume with caves, tunnels and overhangs, but there is no save format, Causeway migration, or persisted tunnel proof. |
| **Voxel world viewed as a map** | ✅ | 🟡 | ✅ | ✅ | **Ahead** | `"view": "map"` draws a voxel world flat from straight above: each column the top face of its highest supporting block, lit by height (slope against a north-western sun, paler high ground, water darker with depth), with only what the camera sees worked out, sixteen columns square at a time. The same world, blocks, edits and `VoxelGround` as the blocks view, so one generator serves a 3D game and a top-down one. `Grid.surface` and `Grid.height` read a column's top from a script. Low Tide separates permanent liquid columns from the reversible flood overlay for swimming and marked shallow dives; these are Decay gameplay rules. Its Basin is a generated drained sea floor drawn this way, and its crawler and crew read the ground through those calls. Gathering removes biome resource blocks with existing `Grid.set_block` edits; the six-resource inventory, carrying and cargo weight are Decay rules, exercised with keys and phone touch. The inspector offers `view` as a choice, but the editor's Scene view has not been looked at with a map world. No decoration layer: a flower or tuft on top of a block is not drawn, and animated or glowing faces draw as their first frame |
| Reversible voxel map flooding | ✅ | 🟡 | ✅ | ✅ | **Behind** | Low Tide found and exercises a continuous `map_flood` overlay: `Grid.set_flood` and `Grid.flooded` share the drawing threshold without changing terrain, edits or cached columns. Decay drives rising water, cargo loss, waiting and a nonlethal escape; headless keyboard and phone-touch tests exercise recovery after the ebb. Builtin overland blocks retain their original terrain. Map face/cache tests cover strict height boundaries. Editor Scene view is unverified; no dedicated authoring tool, 3D flood mesh or fluid simulation. |
| Smooth / hybrid voxel meshing | 🟡 | ❌ | ❌ | ❌ | **Behind** | Causeway once overlaid one smoothed chunk on its terrain as a prototype; that went with its own terrain code when it moved onto the voxel world. A seam-safe density mesher and Block/Smooth/Hybrid engine profiles remain unbuilt. |

## 2D rendering

| Feature | Engine | Editor | Decay | Proof | vs. baseline | Gap that matters |
| --- | :-: | :-: | :-: | :-: | --- | --- |
| Sprites, sheets, UVs, layers, blending | ✅ | ✅ | 🟡 | ✅ | **Par** | Decay has sprite asset and visibility paths only |
| **Advanced sprite colour transform** | ✅ | ✅ | ✅ | ❌ | **Par** | Per-channel multiply and offset beyond tint, as `sample * tint * multiply + offset`, collapsed behind an advanced section with a reset. A transform that is not finite is refused at extraction; one merely outside zero to one is left to clip. No game in this repository proves it; see the proof note below |
| Sheet-declared ground anchor | ✅ | ✅ | — | ✅ | **Ahead** | Says where a sprite meets the ground, so sorting is authored rather than guessed |
| Procedural shapes | ✅ | 🟡 | ✅ | ✅ | **Ahead** | Instanced with sprites; Unity needs a plugin or a mesh. No point-handle authoring |
| Tilemaps (ortho + iso, overhang) | ✅ | ✅ | ✅ | ✅ | **Par** | `Grid.tile`/`set_tile` read and write cells; `Grid.columns`/`rows` give the size. The platformer's level is a painted tilemap and is the proof. Low Tide's scripts read its deck for walls and its Basin for soft sand, and write typed resource bundles into and out of its starter's four-slot hold with `Grid.set_tile`; its workbench spends physical cargo on up to three stern extensions (sixteen slots), a bunk and an engine upgrade, preserving the deck origin and crew while driving. Keyboard and phone-touch runtime tests exercise construction and refusal; this is fixed-blueprint Decay gameplay using existing engine APIs. Low Tide's high-tide dive also reads an authored side-view hull for swimming collision and returns recovered cargo to the original deck; keys and phone touch play the trip, air loss and ebb recovery. Its site history is session-only, and the authored interior is not a voxel cross-section |
| 2D depth and grid placement | ✅ | 🟡 | ❌ | ✅ | **Ahead** | `sindri.grid.placement` names a cell and the runtime derives the transform from it — plane position, the height of the ground in that column, and the Z that orders it. Depth is a consequence of position rather than an authored number, so a placed sprite carries no render layer and no script computes one. Causeway places its entities this way. `depth_step` on the grid turns projected depth into Z; zero keeps a scene ordering exactly as it did. A moving entity omits the cell and takes depth alone. A placement names its grid with the same stable-ID type an occupant uses and carries the cells it covers, and every one of them has to hold it up: standing over a hole is an error naming that cell rather than a prop resting at height zero. Flat ground no longer draws over what stands on it: a placed entity sorts one cell forward, past ground no higher than its feet, because no face of such a cell — top or wall — can cover it, while a block raised a step ahead keeps its own depth and still covers it. Terrain ordering is untouched by that rule. Placing a prefab by clicking a cell in the editor, and one depth per sprite ordering a tall sprite against a tall block, both remain; `docs/2d-depth-and-placement.md` carries the contract |
| Ordering a walker against several occluders at once | ❌ | ❌ | ❌ | ❌ | **Behind** | One depth per entity answers for one relationship at a time. Standing in the corner between open ground a step ahead and a raised block a step ahead, the ground wants the walker sorted forward and the wall wants it left alone; the wall wins, and the ground seam returns for that one case. The same single point is why a multi-cell wall or house sorts from one anchor rather than from the cells it occupies. Sort footprints — ordering derived per overlapping pair from occupied cells rather than from one point — are the way out and are not built; `docs/2d-depth-and-placement.md` records why no single depth can do it. It was measured rather than estimated: `sweep_occlusion` stood on all 387 standable columns of Gather's island and reached the corner in 22 of them. That test left with Gather, so no project holds the number as a budget any more. The editor draws the report on the grid under Build → Ordering, filling the ground stood on and outlining what covers it. The sweep needs a volume to stand on, so a scene whose props are sprites on no grid gets an empty report rather than a guarantee |
| Stackable tile volumes | ✅ | 🟡 | 🟡 | ❌ | **Behind** | Sparse XYZ storage, tile sets, face culling/rendering, undoable block painting, and one engine-owned 16×16 runtime chunk coordinate/store work. No game uses a tile volume any more: Causeway moved its ground onto the voxel world, which reads the same tile sets. Eviction, persisted edits, asynchronous budgets, physics collision, asymmetric climb/drop, second walkable levels, and slice views remain |
| **Autotiling / rule tiles** | ❌ | ❌ | ❌ | ❌ | **Absent** | Unity and Godot both ship it. Painting a wall run by hand is the daily cost |
| Tilemap collision | ✅ | 🟡 | — | ✅ | **Par** | `sindri.physics2d.tilemap_collider` makes painted tiles solid, merged into as few boxes as cover them so nothing catches on tile seams, with passable sprites for decoration. The Scene view outlines the generated boxes. Passable sprites are typed as names rather than picked from the palette. The platformer's level is solid through it |
| **2D lights and shadows** | ❌ | ❌ | ❌ | ❌ | **Absent** | URP 2D lights, Godot's CanvasModulate + Light2D. Nothing here |
| **Custom shaders / materials** | ❌ | ❌ | ❌ | ❌ | **Absent** | No material asset, no shader authoring. The single biggest ceiling on visual identity |
| **3D directional + ambient lighting** | 🟡 | 🟡 | ❌ | 🟡 | **Behind** | `sindri.environment` authors ambient fill and one bounded shadow map, and a `sindri.light` entity is the directional sun, aimed by rotation and drawn in the Scene view, shared by textured world and voxel geometry in editor and browser Voxel Lab. Local lights, materials, cascaded shadow quality, and shipped-game proof remain. |
| **Voxel ambient occlusion / contact depth** | 🟡 | 🟡 | ❌ | 🟡 | **Behind** | Engine block meshing samples neighbouring cells around exposed corners, flips face diagonals to preserve the AO gradient, and carries the result into the shared textured renderer. `sindri.environment` authors the effect strength and Voxel Lab exercises it in editor and browser rendering. This is deterministic mesh-time voxel AO, not general SSAO for arbitrary geometry. |
| Bloom | ✅ | ✅ | ❌ | 🟡 | **Behind** | `sindri.environment` authors bloom and the shared renderer applies it in editor viewports and browser Voxel Lab. Voxel Lab is acceptance proof, not yet a shipped-game proof; Decay control is intentionally absent until gameplay needs it. |
| **Post-processing stack** | 🟡 | 🟡 | ❌ | 🟡 | **Behind** | `sindri.environment` authors exposure, tone mapping, contrast, saturation, bloom, and vignette through one ordered world path in editor and browser Voxel Lab, with overlay/UI rendered crisply afterward. LUT grading and advanced cinematic effects remain absent, and Voxel Lab is acceptance proof rather than shipped-game proof. |
| **Nine-slice sprites** | ❌ | ❌ | — | — | **Absent** | Every UI panel that resizes needs it |
| **Sprite masking / stencil** | ❌ | ❌ | ❌ | ❌ | **Absent** | — |
| Sorting and draw order | ✅ | ✅ | 🟡 | ✅ | **Par** | Layers plus the ground anchor |
| **Camera follow / confine / shake** | ✅ | 🟡 | 🟡 | ✅ | **Behind** | `sindri.camera.behavior` provides stable-target follow with offset/dead zone/smoothing/max speed, XY confinement, and deterministic trauma shake, and every gameplay host (editor Play, the browser host, the camera example) advances it after the scripts. The platformer follows its hero inside the level's bounds and a test holds it. Decay exposes runtime follow/offset/dead-zone/smoothing/speed/bounds/shake controls and trauma/impact. Camera Lab exercises mode toggles, guides, tour and phone touch through the real behavior and is exported to Pages; Orbital uses impact shake. Low Tide exercises explicit `Camera.orthographic_size(camera, size)` for eased walking/driving zoom, with target isolation, payload preservation and invalid-size/projection tests. Dedicated authoring UX/gizmos remain. |

## 3D rendering

Honest summary: 3D is a foundation, not a feature. It is listed so the size of
the gap is legible, not because it is scheduled.

| Feature | Engine | Editor | Decay | Proof | vs. baseline | Gap that matters |
| --- | :-: | :-: | :-: | :-: | --- | --- |
| Cameras, depth, cube primitive, inline textured surface mesh | 🟡 | 🟡 | ❌ | 🟡 | **Behind** | `examples/cube` draws an authored inline mesh; meshes are inline, rebuilt into GPU buffers per draw, and have no asset/import pipeline yet |
| **glTF / model import** | ❌ | ❌ | ❌ | ❌ | **Absent** | `tools/isometric-baker` renders models to 2D sprites offline, in three views; that is not runtime 3D |
| **Materials** | ❌ | ❌ | ❌ | ❌ | **Absent** | — |
| **Lighting** | 🟡 | 🟡 | ❌ | 🟡 | **Behind** | One authored ambient contribution plus one directional world light shade textured 3D geometry and voxel terrain in editor viewports and Voxel Lab, with one bounded directional shadow map. No local lights, PBR materials, normal maps, cascaded shadows, or Decay control yet. |
| **Skeletal animation** | ❌ | ❌ | ❌ | ❌ | **Absent** | — |
| **3D physics** | 🟡 | ❌ | ❌ | ❌ | **Absent** | A Sindri-owned data model exists; no runtime |

## Animation

Still the weakest major system relative to the baseline, but no longer the one
where nothing connects. Sprite clips exist, play, and are now *chosen by
gameplay*: a script plays, stops, restarts and times a clip, and is told when a
one-shot has ended. Sequences now animate any numeric field and tweens move
values from scripts; what remains absent is frame events, blending and
skeletal animation.

| Feature | Engine | Editor | Decay | Proof | vs. baseline | Gap that matters |
| --- | :-: | :-: | :-: | :-: | --- | --- |
| Sprite frame clips, timing, loop state | ✅ | ✅ | ✅ | ✅ | **Par** | `Animation.play`/`stop`/`restart`/`is_finished`/`frame`/`clip`/`set_speed`. The platformer's hero switches between idle, run, jump and fall; Scorchball's players choose a clip and its speed from how they move; Orbital's mine blast plays once and despawns itself when it ends; Low Tide's deck-child tread belts change clip and speed while driving/pivoting, hold their frame in a flood and resume after ebb |
| Clip authoring and preview | — | ✅ | — | ✅ | **Par** | — |
| **Animation events (a frame fires a callback)** | ❌ | ❌ | ❌ | ❌ | **Absent** | Footsteps, hit frames, spawn-on-frame all need it |
| Property animation (animate any component field) | ✅ | ✅ | ✅ | ✅ | **Behind** | A sequence track keys any numeric component field, by component and a path into it (`sindri.sprite/tint.3`), or a transform channel, edited in the Timeline; Causeway's beacon swells and breathes this way. Unity's Animation window also keys colours, vectors, booleans and references as one value; a track here holds a single number, so a colour is four tracks and nothing non-numeric animates |
| **Tweening / easing** | ✅ | 🟡 | ✅ | ✅ | **Behind** | Managed number/vector/colour tweens, pause/resume/cancel/restart, typed values, progress/completion and disposal, composed with a delay, a loop count (or for ever), yoyo and `Tween.after` sequences. Orbital's pickup appears and then breathes with an endless yoyo; Tween Lab chains a crossing, a wait and a return. Weave shares named easing through CSS transitions. Authored choreography is a sequence edited in the Timeline (see Timeline / cutscenes); scripted tweens have no editor view, and property binding and completion callbacks remain |
| **State machine / blending** | ❌ | ❌ | ❌ | ❌ | **Absent** | Animator controller, Godot AnimationTree. Transitions are hand-written today |
| **Skeletal / cutout 2D animation** | ❌ | ❌ | ❌ | ❌ | **Absent** | Spine, Unity 2D Animation. Frame sheets only |
| Timeline / cutscenes | ✅ | ✅ | ✅ | ✅ | **Behind** | A `sindri.sequence` component holds named sequences: keyframed tracks that move a transform channel or any numeric component field on the entity, a named child, or, with a path starting `/`, anything in the scene, each key with a CSS easing, and cues that a script waits on with `Sequence.cued` and that may play a sound. The runtime advances them after scripts in the shared game session and the editor's Play. The editor's Timeline panel draws keys and cues against time, scrubs a preview into the Scene view without touching the document, and adds, keys, moves, retimes and removes tracks, keys and cues as undoable edits. Sequence Stage is the feature example, and Causeway's beacon lights by an authored swell whose cue sounds the chime, then breathes on a loop; an export carries a cue's sound without the project listing it. No curve editor, no sprite-clip, enable or event tracks beyond cues, no blending between sequences, and keys hold a single number |

## Physics and collision

| Feature | Engine | Editor | Decay | Proof | vs. baseline | Gap that matters |
| --- | :-: | :-: | :-: | :-: | --- | --- |
| Bodies, fixed-step stepping, velocity, impulse | ✅ | 🟡 | ✅ | ✅ | **Par** | — |
| **Forces and rotation (2D)** | 🟡 | 🟡 | 🟡 | 🟡 | **Partial** | Runtime and typed Decay force/torque, angular velocity/impulse and world-point impulses; one-step accumulation/reset, validated body kinds and spawn requests. Platformer wind crate and input-driven launch/rotation regressions are added. Native, WASM and browser CI passed on the forces/rotation head; browser interaction and visual inspector remain in final integration |
| **Solid contact snapshots (2D)** | ✅ | 🟡 | ✅ | ✅ | **Partial** | Copied entity-based world points, support normals, normal/friction impulses and force from the last solve. Deterministic ordering, sleeping support, sensor exclusion and teleport/removal invalidation. Platformer grounds from contacts and flashes its crate on hard landings. Editor Play inspection and browser interaction remain in final integration |
| Compound colliders (several pieces, one body) | ✅ | ✅ | — | ✅ | **Par** | Pieces are added, removed, reordered and fully edited, a piece's shape included. The platformer's hero is one body of two pieces, a capsule and a box |
| Masks, sensors, collision events | ✅ | 🟡 | ✅ | ✅ | **Par** | — |
| **Named collision layers** | ✅ | ✅ | ✅ | ✅ | **Par** | A scene's `sindri.physics2d.world` names its layers bit by bit; `Physics.layer`/`Physics.mask` turn names into masks and refuse a name the world does not give; the inspector shows every mask field as a menu of named layers, keeping unnamed bits. The platformer names ground, hero and pickups and probes for ground by name; Physics Playground's mask control asks for its layers by name. Named per scene rather than in project settings, and colliders still store masks |
| Per-piece validation naming the failing index | ✅ | — | — | ✅ | **Ahead** | Neither baseline tells you *which* collider was wrong |
| **2D closest-hit raycasts** | ✅ | — | ✅ | ✅ | **Par** | Finite world-space casts return entity/point/normal/distance or null; query membership masks, sensor opt-in and entity exclusion; deterministic ties. Platformer HUD measures ground clearance; Physics Playground exposes hits, normals, masks and triggers on Pages. Direct scans; no query index |
| **Overlap and shape casts (2D)** | ✅ | — | ✅ | ✅ | **Par** | `PhysicsWorld2d::overlap` and `shape_cast` answer for circles, boxes and capsules from synchronized pieces with the raycast's mask/sensor/exclude filter; Decay has `Physics.overlap_circle`, `overlap_box`, `cast_circle` and `cast_box`. Orbital's hostile mine damages what its blast circle overlaps, by collider; Physics Playground shows a swept circle and an area. Solid contacts are exposed separately |
| **3D physics queries** | ❌ | ❌ | ❌ | ❌ | **Absent** | No 3D runtime, so no 3D casts |
| **Joints and constraints** | 🟡 | 🟡 | 🟡 | ✅ | **Partial** | Scene-authored maximum-distance constraints own separate joint entities and stable scene endpoint IDs. Edit/undo, endpoint rebuild, inactivity and removal are exercised; platformer's Decay-driven hanging lantern proves the constraint. Legacy `Physics.connect_distance` is preserved. Hinges add local anchors, limits and torque-capped velocity motors; typed `Physics.set_hinge_motor` and the platformer windmill exercise motor reversal. Sliders add aligned local axes, travel limits and force-capped drive; springs add force-based stiffness/damping. Typed controls and the platformer lantern trolley exercise reversal and retuning. Typed enabled-state controls suspend/reconnect all four constraint kinds, and distance tuning changes owned maximum lengths; platformer Decay reels/releases/reconnects its lantern. Command undo/redo and atomic rejection are covered. Other motor modes, endpoint retargeting and structural creation/removal remain absent; reference picker/diagnostics and visual editor review remain incomplete |
| **Authored joint references in runtime-spawned prefabs** | 🟡 | — | 🟡 | ✅ | **Partial** | Runtime-only instance identity resolves local sibling paths and the original top-level root ID without assigning saved scene IDs. All four joint kinds are exercised across repeated spawns, inactivity and removal; core reparenting/undo preserve scope. Platformer Decay places/removes a reusable powered windmill. Nested renamed-root aliases, pre-expanded placed-root aliases and saved-spawn reference remapping remain incomplete |
| **Saved spawned-prefab reference remapping** | ❌ | — | — | ❌ | **Absent** | Assigning saved scene IDs retains local identity during the run, but serializing a spawned mechanism does not rewrite its component endpoint paths. Runtime prefab identity is deliberately not serialized; reopening that saved scene cannot restore its local references without a general remapping step |
| **Physics materials as assets (2D)** | ✅ | 🟡 | — | ✅ | **Partial** | Typed reusable `.profile` assets, shared coefficient validation and explicit entity overrides. Editor async loading/hot reload and export reference collection; shared native/browser resolver. Platformer crate and planks share wood, with a plank bounce override and rebound regression. Visual editor/browser gameplay review remains in final integration; 3D waits for its runtime |
| **Character controller** | ❌ | ❌ | ❌ | ❌ | **Absent** | Every platformer and top-down game writes one. The platformer's hero is a dynamic body driven by velocity, with solved support contacts and coyote time and a jump buffer in Decay: it works, and it is exactly the code a controller would own |
| Moving a body by its transform | ✅ | — | ✅ | ✅ | **Par** | A script writing a body's position teleports it, keeping its velocity, as in Unity; a position-kinematic body takes it as its next target. The platformer's respawn is the proof |
| **One-way platforms** | ✅ | 🟡 | ✅ | ✅ | **Partial** | Local support normal/cone on solid pieces, including tilemaps; CCD pair filtering and timed Decay drop-through. Platformer planks prove ascent, descent/landing and dropping to an ordinary floor. Command-backed policy/undo regressions preserve velocity/joints. Native, WASM and Chromium export/load smoke passed; visual inspector and full browser interaction verification remain in final integration. Geometric queries still see both sides |
| **Continuous collision (CCD)** | ✅ | 🟡 | ✅ | ✅ | **Partial** | Opt-in 2D body flag, scene synchronization and typed Decay controls; thin kinematic-wall and spawn-window/undo lifecycle regressions. Platformer hero opts in; command-backed edits/undo preserve solver state. Native and WASM checks passed; visual inspector/browser verification remains in final integration. Sensors stay discrete; bullet-versus-bullet sweeping is not guaranteed |
| Collider gizmos in the Scene view | — | ✅ | — | — | **Par** | Every 2D collider is outlined from the pieces physics is given, tilemap boxes included; the selected one's box edges, circle radius and capsule height drag, one undo step a drag. Offsets and rotations are still typed, and 3D colliders have no gizmo |
| Scene gravity | ✅ | ✅ | — | ✅ | **Par** | `sindri.physics2d.world` sets the scene's gravity, so editor Play runs a platformer as its build will; the platformer and Scorchball each set theirs. One vector; no per-area gravity |

## Navigation and grids

Sindri's strongest domain relative to the baseline.

| Feature | Engine | Editor | Decay | Proof | vs. baseline | Gap that matters |
| --- | :-: | :-: | :-: | :-: | --- | --- |
| Orthogonal and isometric grids | ✅ | ✅ | ✅ | ✅ | **Ahead** | First-class, not a plugin |
| Footprints, occupancy, walls | ✅ | ✅ | ✅ | ✅ | **Ahead** | — |
| A\* over whole footprints | ✅ | ✅ | ✅ | ✅ | **Ahead** | Unity needs A\* Pathfinding Project for the equivalent |
| **Navmesh / off-grid navigation** | ❌ | ❌ | ❌ | ❌ | **Absent** | Grid only. Fine for the games we build; a wall for a free-movement game |
| **Steering / avoidance** | ❌ | ❌ | ❌ | ❌ | **Absent** | Agents walk through each other |
| Per-path costs and policies | ❌ | ❌ | ❌ | — | **Behind** | One uniform cost |
| **Viewport wall painting, height tools** | — | 🟡 | — | ✅ | **Behind** | Inspector authoring only |

## Audio

| Feature | Engine | Editor | Decay | Proof | vs. baseline | Gap that matters |
| --- | :-: | :-: | :-: | :-: | --- | --- |
| WAV / Ogg / MP3, native + browser + silent backends | ✅ | ✅ | ✅ | ✅ | **Par** | The editor's Play now plays what a build plays: autoplay sources and every scripted `Audio` request, through the same mixer. With no device it plays silently and still lists what is playing |
| Play, loop, pause, resume, stop | ✅ | 🟡 | ✅ | ✅ | **Par** | — |
| Per-play volume | ✅ | — | 🟡 | ✅ | **Behind** | — |
| **Buses / mixer / master volume** | ✅ | 🟡 | ✅ | ✅ | **Behind** | Every sound plays through a named bus under `master`; `AudioMixer` applies bus gain in front of every backend, changing live voices. Decay: `Audio.set_volume`, `Audio.volume`, `play_on`/`loop_on`; an authored source names its `bus`. Orbital's pause screen moves master, music and effects and saves them; Sound Mixer is the feature example. The editor's inspector edits a source's bus as text. The editor's Audio panel lists every bus Play uses and every sound playing, and monitors them with a trim, mute and solo per bus that change only what the author hears, never `Audio.volume`. No level meters, and a project cannot yet declare its buses and their starting volumes. No effects (reverb, ducking) or snapshot blending |
| **Spatial audio / panning** | ❌ | ❌ | ❌ | ❌ | **Absent** | — |
| **Per-voice handles** | ❌ | — | ❌ | — | **Behind** | A script cannot stop the specific sound it started |
| **Music transitions / crossfade** | ❌ | ❌ | ❌ | ❌ | **Absent** | — |

## Input

| Feature | Engine | Editor | Decay | Proof | vs. baseline | Gap that matters |
| --- | :-: | :-: | :-: | :-: | --- | --- |
| Keyboard, mouse, touch behind the platform boundary | ✅ | ✅ | ✅ | ✅ | **Par** | — |
| Unified pointer, bounded fingers | ✅ | ✅ | ✅ | ✅ | **Par** | — |
| Touch stick built from a finger | ✅ | — | ✅ | ✅ | **Ahead** | A considered solution to a problem most engines leave to the game. Low Tide steers, walks and drives with it on a phone, draws it where the thumb landed, and its tests play a whole salvage run by touch |
| **Stick anchor in screen units** | ❌ | — | ❌ | — | **Behind** | `Stick.anchor_x`/`anchor_y` are physical pixels, and a script cannot learn the view's size in pixels, so a game drawing its stick cannot place the ring from the numbers the input came from. Low Tide reads `Pointer.overlay` when the stick engages instead, which is right only while the steering thumb is the pointer's finger |
| **Action mapping (named actions, rebindable)** | ✅ | 🟡 | ✅ | ✅ | **Behind** | A scene declares `sindri.input.actions`; scripts read `Action.held/pressed/released/axis/vector` and rebind with `Action.rebind`/`Action.last_pressed`, written back into the component. The platformer's hero runs and jumps by actions; the Input example rebinds boost. Declared per scene rather than per project, edited in the inspector as raw JSON, no per-player maps, and a rebinding is not saved between sessions |
| **Gamepad, by player slot** | ✅ | 🟡 | ✅ | ✅ | **Behind** | Pads on desktop and in the browser, read by player slot: a face button or Start joins, unplugging leaves, one join per frame. Scorchball is played by up to four pads and its tests press them. Play in the editor reads pads, not yet exercised with a real one in CI. No rumble or per-player action map |
| **Scroll wheel** | 🟡 | — | 🟡 | ✅ | **Behind** | Scrolls UI scroll regions in every host (Orbital's pause manual, the Control Room archive); a script sees the region's offset, not the wheel itself |
| **Rebinding UI** | ❌ | ❌ | ✅ | ✅ | **Behind** | No engine widget; a game builds one from `Action.bindings`, `Action.last_pressed` and `Action.rebind`, as the Input example does. Rewired sells on this |

## UI and presentation

| Feature | Engine | Editor | Decay | Proof | vs. baseline | Gap that matters |
| --- | :-: | :-: | :-: | :-: | --- | --- |
| Anchored image and text, fill bars, buttons, row/column, safe area | ✅ | ✅ | ✅ | ✅ | **Par** | Low Tide exercises viewport-sized HUD/menu panels, direct blueprint buttons, live material costs, icon-led hold/speed/tide meters and contextual actions; keyboard/phone runtime tests cover selection, refusal and returning to play. A crate outline and cell-bound discard confirmation are Decay gameplay on existing APIs. Wreck-child cargo prefabs can be chosen by proximity, set down and recovered ashore; runtime tests exercise inventory and per-slot streaming history. Its workbench is authored from existing UI primitives, not a new dialog widget. |
| Pointer hit-testing, hover/press/held | ✅ | ✅ | ✅ | ✅ | **Par** | — |
| Weave responsive stylesheets | 🟡 | 🟡 | — | ✅ | **Ahead** | CSS selectors (element names, compounds, descendant and child combinators, lists) with CSS specificity, inheritance, custom properties with `var()`, and combined media queries. Neither baseline has stylesheets this close to CSS; `docs/ui-direction.md` is the plan to make it complete. `:hover` and `:active` follow the pointer in every host, and hit-testing uses the presented geometry. The editor's Styles section shows each element's box model, matched rules with `file:line` and overridden declarations, and computed values; a value edited there is written into the stylesheet, and an element can be picked by clicking it in the running game |
| Project fonts | ✅ | ✅ | — | ✅ | **Par** | — |
| **Slider** | ✅ | ✅ | ✅ | ✅ | **Par** | Horizontal or vertical, with `min`, `max`, `step`, `disabled` and a label; a drag and a scripted write pass through the same clamp-and-quantize contract, so a value cannot enter the component off-step. Weave styles it like any other node. `Ui.slider_value`, `Ui.set_slider_value` and `Ui.slider_changed` read, write and detect the change. The editor names it in Add Component with `orientation` as a declared choice — but no field carries a bounded-range meaning, so `value` is a free number box beside the range it is supposed to obey. A focused slider moves by its step under the arrows, the d-pad or the stick, and takes `autofocus`. Orbital's pause screen mixes its audio with three, saved between sessions, and Sound Mixer is the feature example |
| **Toggle and checkbox** | ✅ | 🟡 | ✅ | ✅ | **Par** | `sindri.ui.toggle`, one boolean component whose art is authored: a switch or a checkbox is a matter of styling, with Weave's `:checked`, `:disabled`, `:hover` and `:focus`. Click, tap, Space or Enter flips it; `Ui.is_checked`, `Ui.set_checked` and `Ui.changed` read, write and detect it. Orbital proves it with a compact-HUD switch that is saved and changes the HUD. The editor lists it in Add Component with plain fields; there is no dedicated authoring gizmo |
| **Text input (single line)** | ✅ | 🟡 | ✅ | ✅ | **Par** | `sindri.ui.text_input` with a placeholder and a limit counted in characters. Editing happens at a caret with a selection: Left, Right, Home, End, Shift to select, Backspace, Delete, and Ctrl or Cmd with A, C, X and V through the system clipboard; `Ui.caret`, `Ui.selection_start` and `Ui.selection_end` let a script draw the caret and highlight. IME composition is committed once on desktop; in a browser the field gives the page's focus to a hidden textarea, so composition, paste and a phone's on-screen keyboard reach it. Keys are held back from gameplay while it edits. Orbital's pilot callsign proves it. No multi-line field, no word-wise movement, and no caret placed by clicking inside the text (a click puts it at the end); iOS raises its keyboard only for a tap that lands while the game is already editing |
| **Scroll region** | 🟡 | 🟡 | ✅ | ✅ | **Behind** | `sindri.ui.scroll` scrolls vertically by wheel, drag or finger, clamps to its laid-out content, clips drawing and hit-testing to itself (nested clips intersect), and a drag never becomes a click on a row. `Ui.scroll_offset` and `Ui.set_scroll_offset`. Orbital's pause screen scrolls a field manual. Vertical only, no scrollbar or momentum, and a rotated region clips to its axis-aligned bounds |
| **Radio group and dropdown** | ✅ | 🟡 | ✅ | ✅ | **Par** | Toggles sharing a `group` are a radio group. `sindri.ui.dropdown` opens a popup of authored `sindri.ui.option` rows that the engine shows only while open; the selected option is `:checked` and the open header `:open`. Click, tap, keyboard or pad open it, choose, and close it; a press elsewhere or Escape puts it away, and the arrows stay inside an open list. `Ui.selected`, `Ui.set_selected`, `Ui.is_open`. Orbital's boss picker is a scrolling dropdown of the twelve bosses; the Control Room has both |
| **Tabs, dialog, tooltip** | ❌ | ❌ | ❌ | ❌ | **Absent** | Still built by hand from buttons and switched-off screens |
| **Rich text** | ❌ | ❌ | ❌ | ❌ | **Absent** | No colour or emphasis inside a string. TextMeshPro was absorbed for exactly this |
| **World-space text** | ❌ | ❌ | ❌ | ❌ | **Absent** | Damage numbers, name plates |
| UI transitions | ✅ | ✅ | ✅ | ✅ | **Par** | CSS `transition` with named and `cubic-bezier` easings, delays and `all`, on colours, lengths and numbers; USS has the same. `@keyframes` is still absent |
| UI devtools (computed style, which rule won) | — | ✅ | — | — | **Par** | Unity's UI Toolkit debugger was the bar. The editor's Styles section now meets it: box model, matched rules with `file:line`, overridden declarations and computed values, and an element picked by clicking the running game — see the Weave row |
| **Keyboard focus navigation** | ✅ | — | ✅ | ✅ | **Par** | Tab and Shift+Tab walk controls in the order the scene is written; the arrow keys move focus to the nearest control that way, preferring those in line, as browser spatial navigation does; a row reached inside a scroll region is scrolled into view; Space or Enter presses, Escape backs out. Weave's `:focus` shows it and `Ui.is_focused` reads it. No focus scopes beyond an open dropdown |
| **Gamepad focus navigation** | 🟡 | — | ✅ | ✅ | **Behind** | Any pad's d-pad moves focus as the arrows do, South presses or submits and East backs out, in every host. A control marked `autofocus` takes focus when its screen appears, so a menu is ready for a pad; with nothing focused and nothing asking, the d-pad and arrows focus nothing, so gameplay that uses them never presses a button. The Control Room's regression drives its settings with a pad, and Orbital's title, chooser, pause and results all start focused. A pad's left stick moves focus as the d-pad does, once per push, and moves a focused slider. There is no per-player focus |
| **Accessibility labels** | ❌ | ❌ | ❌ | ❌ | **Absent** | — |
| **Drag and drop** | ❌ | ❌ | ❌ | ❌ | **Absent** | — |

## Effects

| Feature | Engine | Editor | Decay | Proof | vs. baseline | Gap that matters |
| --- | :-: | :-: | :-: | :-: | --- | --- |
| Pooled fleck bursts, batched with sprites | ✅ | 🟡 | ✅ | ✅ | **Behind** | Measured and fast, but it is one effect shape |
| **Particle system (emitters, curves, shapes)** | ❌ | ❌ | ❌ | ❌ | **Absent** | No emission shape, lifetime curve, colour ramp, sub-emitter, or trail |
| **Effect preview in the editor** | — | ❌ | — | — | **Absent** | Authored blind through the generic inspector |
| **Trails / line renderer** | ❌ | ❌ | ❌ | ❌ | **Absent** | — |
| **Screen-space feedback (shake, flash, hitstop)** | ❌ | ❌ | ❌ | ❌ | **Absent** | The `Feel` package sells on precisely this |

## Scripting (Decay)

| Feature | Engine | Editor | Decay | Proof | vs. baseline | Gap that matters |
| --- | :-: | :-: | :-: | :-: | --- | --- |
| Typed host, compile / run / reload | ✅ | 🟡 | ✅ | ✅ | **Par** | Source editing stays external. A previewed script shows its compile errors, from the check Play compiles with |
| Safe entity, prefab, profile references | ✅ | ✅ | ✅ | ✅ | **Ahead** | A stale handle is refused, not a null dereference |
| `@export` properties discovered and authored | ✅ | ✅ | ✅ | ✅ | **Par** | — |
| Language server: highlight, complete, hover, definitions, diagnostics | — | 🟡 | ✅ | ✅ | **Par** | VS Code only |
| Vectors as values (`Vec2`, `Vec3`) | ✅ | 🟡 | ✅ | ✅ | **Par** | Arithmetic, `length`, `normalized`, `dot`, `distance`, `lerp`; transform, pointer and stick hand them out whole. Proven in Scorchball's ball and Orbital Last Stand's bullets. A vector `@export` is stored as `[x, y]` and drawn by the inspector's X/Y/Z number row, as a position is; that has not been looked at in the running editor |
| **Rotations as values** | ❌ | — | ❌ | — | **Behind** | Only `rotation_z` as a number; no quaternion or angle type |
| Scripts reach each other by type | ✅ | ✅ | ✅ | ✅ | **Par** | `Bolt.on(e).damage`, live fields, typed messages delivered after the pass; checked at compile time across files. Orbital Last Stand's bullets are set up this way, and Scorchball's kicks and power-ups; Orbital's enemy damage reads and Orbital Baked's signals still use the untyped calls |
| Events (emit, handle anywhere) | ✅ | ✅ | ✅ | ✅ | **Par** | `event GoalScored(team: f32);`, `GoalScored.emit(1.0)`, `on GoalScored(team) { }`; delivered after the pass to every listener, checked at compile time across files. Scorchball's goals, wind and fireball |
| Shared game state, declared and typed | 🟡 | ✅ | 🟡 | 🟡 | **Par** | `state Game { var score: f32 = 0.0; }` gives `Game.score`, checked across files, with one type and one starting value; kept on the board so `Game.get("score", 0.0)` still reads it. Numbers, flags and enums. Scorchball, the platformer and Orbital use it: Orbital Last Stand declares 108 values and Orbital Baked 121. Each keeps under a hundred and fifty `Game.get`/`Game.set` calls for keys built at runtime and for eight values whose callers disagree about the fallback (`hp`, `max_hp`, `boss_hp`, `boss_max`, `next_level`, `sector`, `target_x`, `target_y`) |
| Timers as values | ✅ | 🟡 | ✅ | ✅ | **Par** | `Timer(2.4)` with `done`, `left`, `duration`, `progress`; a script's timer fields run down before each `update`. Scorchball's banners, power-ups, burning and bot reactions use them. No pause, and no timer that calls something when it ends |
| Shared helper functions across files | ✅ | ✅ | ✅ | ✅ | **Par** | A top-level `fn` is its file's; `shared fn` is callable by name from every script in the project, checked across files, with no `this`. No namespaces: shared names are one flat space per project, clashes refused. Orbital's view helpers use it |
| Constants | ✅ | ✅ | ✅ | ✅ | **Par** | `const LIMIT: f32 = 3.0;` for a file, `shared const` for the project; numbers, flags, text and enum variants, worked out at compile time from literals, operators and other constants, and written in place. None inside a script or function, and no vector, list or struct constants. Orbital's `VIEW_SIZE` |
| Structs | ✅ | ✅ | ✅ | ✅ | **Par** | `struct Card { name: String, weight: f32 }` in any file; built with every field named, read and written by field, copied where assigned, kept in lists. Methods after the fields, asked of a value with `this` a read-only copy; none that change their struct. A field may have a default, worked out like a constant, and then may be left out when one is built. A scene authors a struct or list `@export` as JSON, and the inspector edits it field by field and item by item. Orbital's module chooser uses one, with `show` and `hide` methods, and reads its card names from a scene-authored list |
| Lists and ranges | ✅ | ✅ | ✅ | ✅ | **Par** | `[a, b]`, `push`, `pop`, `insert`, `remove_at`, `clear`, `contains`, `index_of`, `xs[i] = v`, `for i in 0..n`; the same `List<T>` host queries return. Value semantics, capped at 10,000 elements. No sort, filter or slicing. Orbital's module chooser uses them |
| Maps | ✅ | ✅ | ✅ | ✅ | **Par** | `Map<K, V>`, written `["a": 1.0]` or `[:]`; `m[k]`, `m[k] = v`, `get(k, fallback)`, `contains`, `remove`, `clear`, `keys()`, `values()`, `length`. Keyed by text, numbers, flags, variants or entities, in the order keys were first set; value semantics, capped at 10,000 keys. Not authored by a scene yet, and no game uses one yet |
| Colours | ✅ | ✅ | ✅ | ✅ | **Par** | `Color(r, g, b[, a])` or `Color("#rrggbb")`; `r`/`g`/`b`/`a` read and written by channel; `lerp`, `with_alpha`; constants and struct defaults may be colours. The engine's `tint`, `fill`, `stroke`, `color_multiply` and `color_offset` are `Color`, assigned whole or by channel. Orbital's charger elite colours use it |
| Optional values | ✅ | ✅ | ✅ | ✅ | **Par** | `f32?`, `String?`, `List<T>?`: a value or `null`, never used as its type until `?? fallback` or a check (`!= null`, or an early `return` on `== null`) narrows a `let` or parameter. Replaces `-1` sentinels |
| Text: joining and methods | ✅ | ✅ | ✅ | ✅ | **Par** | `"Score " + score`, joining numbers, flags, vectors and variants; `length`, `contains`, `starts_with`, `ends_with`, `find`, `slice`, `replace`, case, trimming. Capped at 64 KiB. Numbers written with `n.fixed(digits)` and `n.padded(width)`; no format strings or interpolation, and UI templates still own a HUD's words. Scorchball's clips, Orbital's stat keys and run clock |
| Enums and exhaustive `match` | ✅ | ✅ | ✅ | ✅ | **Par** | `enum Phase { Lobby, Play }`, written `Phase.Lobby`; `match` must cover every variant or end with `_`. Usable in `state` and as an `@export` authored by name, with an inspector dropdown. `match` runs blocks as a statement, or gives a value where one goes (`let clip = match kind { Power.Grow => "enlarger", ... };`), every arm the same type. No variants carrying data, and a value arm is an expression, not a block. Scorchball's phase and power-ups use them |
| **Coroutines / sequencing** | ❌ | — | ❌ | — | **Absent** | "Wait a second, then do this" is a `Timer` field and a check, not a sequence |
| **Debugger (breakpoints, stepping)** | ❌ | ❌ | ❌ | — | **Absent** | `print` debugging only |
| **Formatter** | ❌ | ❌ | ❌ | — | **Absent** | — |
| **In-editor script editing** | — | ❌ | — | — | **Behind** | Scripts are created and previewed read-only; editing is external |
| **Signals / event bus** | ❌ | ❌ | ❌ | ❌ | **Absent** | Godot's signals are a headline feature. Scripts poll |
| **Script unit tests** | ❌ | ❌ | ❌ | ❌ | **Absent** | No way to test a `.decay` file without running a game |

## Editor experience

Where the most day-to-day friction is, and where one architectural change
removes most of it.

### The root cause of hand-typed fields (fixed)

This records the defect ranked queue item 1 removed; the tables it describes
are gone. It stays because its conclusion — a component declares what its
fields mean, and no consumer guesses it from a name — is the rule every new
component follows.

The editor used to infer meaning for some fields, and it is worth being precise
about how, because the mechanism was the defect rather than its absence.

Three lookup tables in the editor guessed a field's meaning from its **name**:

```rust
// editor/src/native/inspector_panel/field.rs
match (type_name, key) {
    (_, "texture") => Some(assets.textures),
    (_, "font")    => Some(assets.fonts),
    (_, "clip")    => Some(assets.audio),
    ("sindri.script", "source") => Some(assets.scripts),
    _ => None,
}
```

plus `is_colour`, which requires the key to be literally `tint`, `color` or
`colour`, and `editor/src/inspector/choices.rs`, which maps `(type_name, key)`
to an enum's spellings. The `choices` module is careful — it takes each list
from the engine's own constants rather than repeating them — but it is still a
table in the editor keyed by field name.

Four consequences followed, and they were why more table entries was not the fix:

1. **The bare-key rules are global.** `(_, "texture")`, `(_, "font")` and
   `(_, "clip")` match *any* component, including one a game brings of its own.
   A game component with a `clip` field is offered the audio list whether or not
   it holds audio.
2. **Anything absent from the table is a free-text box, silently.** A field
   named `sheet`, `icon` or `portrait` gets nothing. This is what happens to
   every component added since the table was written, which is why new work
   keeps landing as raw fields.
3. **A colour must be spelled `tint`.** Named anything else it is four number
   boxes; and the check is `Numbers(4)`, which a UV rect and a quaternion also
   satisfy.
4. **It lives in the editor.** `sindri-capabilities` cannot document it, and no
   other tool can use it.

Underneath all four: meaning is *guessed by the consumer* rather than *declared
by the component*. That is a second copy of knowledge about a component living
away from the component — exactly the drift `check_template` was written to stop
for field lists. `ComponentSchemaRegistry` stores a field template, a
`serde_json::Value` exemplar checked against what serde asks the type for, so it
captures each field's **shape** and nothing about its **meaning**. A texture id
and a display label are both `String` to the registry.

A second consequence used to follow, and is now fixed. A value that is an array
of objects fell to `ValueKind::Opaque` and was displayed as stored — the
`pieces` array of a compound collider is exactly that shape, so compounds were
authorable in a scene file and not in the inspector. The template's exemplar
item is what closed it: it says what a piece consists of, what each field means,
and what a fresh one is, so the panel can draw a list and add to it without
inventing anything.

The fix was to move meaning to the registration — asset(texture), asset(script),
asset(clip), entity reference, choice, colour, angle, bounded range, list-of —
and have the editor read it instead of guessing. It extended the registry that
already existed, made the knowledge checkable against the template the way field
lists already were, travels to every tool rather than only the editor, and
covers components not yet written. It was the highest-leverage item in this
file.

| Feature | Editor | vs. baseline | Gap that matters |
| --- | :-: | --- | --- |
| Scene view, hierarchy, generic inspector, project browser | ✅ | **Par** | — |
| Gizmos: transform, snapping, Z-lock-safe movement | ✅ | **Par** | No camera or effect gizmos. 2D colliders have theirs; 3D colliders do not |
| Play / pause / stop / single-step, snapshot restore | ✅ | **Ahead** | Single-step and snapshot restore are better than Unity's play mode |
| Tilemap painting, sheet slicer, texture picker | ✅ | **Par** | — |
| **Asset pickers for schema fields generally** | ✅ | **Ahead** | Declared per component in the schema registry, checked against the field template, and carried in `docs/generated/`. Unity needs a plugin (Odin) for the equivalent |
| **Array-of-object editing** | ✅ | **Par** | A list of objects is added to, removed from and reordered, with each item's fields drawn through its meanings. Decided by the template, so a tilemap's thousand tiles stay a readout |
| **Tagged-enum (variant) fields** | ✅ | **Par** | A field that decides what else its object holds is switched as one edit, at any depth — Godot's equivalent is swapping a Resource subtype. What is ours is that every variant is proved to decode at startup, so an unpickable one fails the build rather than the scene |
| Console / log panel | ✅ | **Par** | Every failure the editor reports, plus script `print` named by the entity that printed it, filtered by level, repeats collapsed to a count, with a jump to the entity a line is about and an error count in the status bar. It can now be put wherever it is wanted — it opens beside the Scene view — so watching the log no longer costs the project browser, and a failure recurring every frame is one counted line wherever it sits in the log rather than a new line per frame |
| Profiler view | 🟡 | **Behind** | The Profiler panel times each frame of Play by phase — effects, physics, screen UI, scripts, animation, cameras and each view drawn — as a stacked chart of the last 300 frames against the 60 fps budget, with each script's share, slowest first. CPU only: no GPU timings, no per-call breakdown inside a script, and shipped builds are not profiled |
| **Search / filter in hierarchy or project** | ❌ | **Absent** | Painful past a few dozen entities |
| **Project settings surface** | ❌ | **Absent** | `sindri.toml` is edited by hand |
| **Build / export UI** | ❌ | **Absent** | Export is CLI-only |
| **Prefab creation from a selection** | ✅ | **Par** | *Make prefab* writes a subtree as a prefab beside the scene and puts an instance in its place, keeping instances inside it nested; on a selection, a prefab of each as one undo step |
| **Save inspector** | ❌ | **Absent** | Persistence is play-testable but not viewable |
| **Multi-select and bulk edit** | ❌ | **Absent** | — |
| **Customisable layout** | ✅ | **Par** | Every panel is a tab, draggable into any of seven docks or four scene-anchored overlays — edges dock, corners float — with the arrangement and every size persisted. Three presets to start from. Behind Unity and Unreal only in that a panel cannot yet be torn off into a window of its own |
| **Canvas-first workspace** | ✅ | **Ahead** | The scene view is the document and panels overlay its corners, rather than the viewport being the rectangle left over when the docks have taken theirs. Unity and Godot have no equivalent posture; the nearest comparison is a design tool. Still to come: command palette, chrome collapse, and the Game view as an anchored thumbnail — see `docs/editor-direction.md` |
| **Guided local-AI setup** | ✅ | **Ahead** | One button inside the editor downloads a pinned llama.cpp runner and model into the person's own folder, verifies both by SHA-256, starts the runner and checks the model against Sindri's repair cases, showing each step with progress, speed and time left, and resuming where it stopped. No installer, terminal, download page or password; removal is one button. Linux x86-64 and Apple silicon only — Windows and Intel Macs are told it is not available yet. No comparable engine ships a local-model setup: Unity and Unreal have no local story, and third-party assistants start from "install this yourself and paste a key" |
| **Local AI Decay repair** | 🟡 | **Par** | A previewed script that does not compile offers **Propose a fix** once a model has repaired both verification cases. The answer is compiled by the same check Play uses and sent back with its own errors at most twice; only a candidate that compiles and keeps every declared script is shown, as a diff, and it is written only on Accept, with the previous text kept to put back. Unity's assistant proposes code through a cloud service and does not grade it with the compiler first. No model runs in CI — the loop is proved against scripted models — and it rewrites whole files rather than making semantic edits. Writing a new script from a description is the next use of the same loop |
| **Command palette** | ✅ | **Ahead** | Ctrl+K over panels, entities, project files, scenes, arrangements and verbs, ranked by a scored subsequence match with multi-term search. Unity has no equivalent; Unreal's is command-only and Godot's is files-only |
| **Customisable shortcuts** | ❌ | **Behind** | Keys are fixed in `native/shortcuts.rs` |
| **Live edit while playing** | ❌ | **Behind** | Stop restores the world wholesale, so a value tuned during a run is lost. The largest single cost in the change-and-feel loop; see `docs/editor-direction.md` |
| **Record and scrub a run** | ❌ | **Absent** | The step is fixed and deterministic, so this is available rather than aspirational, and nothing offers it |
| **In-editor script editing** | ❌ | **Behind** | — |

## Build, export, and platform

| Feature | Engine | Editor | Decay | Proof | vs. baseline | Gap that matters |
| --- | :-: | :-: | :-: | :-: | --- | --- |
| Static web export, base-path aware, content-hashed | ✅ | ❌ | — | ✅ | **Ahead** | Genuinely simple next to Unity's WebGL output |
| Native desktop | ✅ | ✅ | — | ✅ | **Par** | — |
| **Game-independent browser project host** | ❌ | — | — | ❌ | **Behind** | Exports load project manifests and scripts, but their shared browser runner is still built from Causeway, calls its terrain setup, and shares `sindri.causeway.save`. Extract a generic host and project-scoped persistence; a bundle rename does not close this gap. |
| Browser / WASM | ✅ | — | — | ✅ | **Par** | — |
| Versioned `sindri.toml`, validation | ✅ | 🟡 | — | ✅ | **Par** | — |
| **Native packaging (installer, icon, splash)** | ❌ | ❌ | — | ❌ | **Absent** | A desktop build cannot be shipped to a player as-is |
| **Mobile targets** | ❌ | ❌ | — | ❌ | **Absent** | Touch input exists; a build does not |
| **Console targets** | ❌ | ❌ | — | ❌ | **Won't** | Not a realistic target and should not pretend to be |

## Diagnostics

| Feature | State | vs. baseline | Gap that matters |
| --- | :-: | --- | --- |
| Named validation errors at the boundary | ✅ | **Ahead** | — |
| Deterministic replay from a seed | ✅ | **Ahead** | Reproducing a bug is a seed, not a video |
| In-editor console | ✅ | **Par** | See the editor section. It can be placed anywhere and opens beside the Scene view |
| Profiler / frame timing | 🟡 | **Behind** | The editor's Profiler panel times Play frame by frame and script by script; a browser or desktop build has no timing of its own |
| **Debug draw from scripts** | ❌ | **Absent** | A script cannot draw a line to show what it thinks it is doing |
| **Frame / draw-call debugger** | ❌ | **Absent** | — |
| **Crash and error reporting in a shipped build** | ❌ | **Absent** | — |

## Absent whole systems

Named so they are decisions rather than oversights.

| System | Status | Position |
| --- | --- | --- |
| **Localization** | **Absent** | No string table, no locale switch. Needed before any non-English release |
| **Networking / multiplayer** | **Won't**, for now | A large system that no planned game needs. Revisit only with a game that requires it |
| **Video playback** | **Absent** | Rarely load-bearing for 2D games |
| **Visual scripting** | **Won't** | Decay is the answer. A second authoring path would split the ecosystem |
| **In-game console / cheats** | **Absent** | Cheap, and useful for testing |
| **Object pooling (general)** | **Behind** | Effects pool internally; nothing general exists |
| **Analytics / telemetry** | **Won't** | Not our business to add |

---

## What the Unity Asset Store proves

A plugin that sells for a decade is a feature the engine should have had. This
is a market-validated gap list, and it is more honest than our own judgement
because somebody paid for every row.

### Tier 1 — Unity had to absorb it

The strongest possible evidence: Unity bought or cloned these, conceding they
were native gaps. Treat these rows as close to automatic.

| Package | What it fixed | Sindri |
| --- | --- | --- |
| **TextMeshPro** | Text quality, rich text | **Absent** — no rich text |
| **Cinemachine** | Camera follow, framing, confining, shake | **Behind** — `sindri.camera.behavior` follows, confines and shakes, and the platformer uses it; Decay camera-mode controls and authoring gizmos remain |
| **Post Processing Stack** | Bloom, colour grading, vignette | **Behind** — an authored world stack now covers exposure, tone mapping, contrast, saturation, bloom, and vignette; LUT grading and advanced cinematic effects remain |
| **Shader Graph** | Shader authoring without code | **Absent** — no materials at all |
| **Input System** | Action mapping, rebinding, gamepad | **Behind** — actions reach scripts and rebind at runtime; gamepads read by player slot, without rumble or per-player actions |
| **Addressables** | Asset streaming and release | **Absent** — not urgent at our scale |
| **ProBuilder** | In-editor geometry | **Won't** — 3D is not the product |

### Tier 2 — still not native, still selling

| Package | What it fixes | Sindri |
| --- | --- | --- |
| **Odin Inspector** | Unity's inspector is not sufficient for real data | **Par** — components declare what their fields mean, so asset pickers, colours, choices, lists and variants need no plugin, which is the anti-goal met. Multi-select and bulk edit remain absent |
| **DOTween** | Tweening and easing | **Behind** — managed Decay playback and Orbital pickup proof; property binding, sequences and loop/yoyo remain |
| **A\* Pathfinding Project** | Real pathfinding | **Ahead** — ours is native |
| **Behavior Designer** | Behaviour trees, AI authoring | **Absent** — AI is hand-written Decay |
| **Rewired** | Input mapping and rebinding | **Behind** — as above |
| **Feel / NiceVibrations** | Game juice: shake, hitstop, flash | **Absent** |
| **Master Audio** | Buses, mixing, music transitions | **Absent** — no mixer |
| **Easy Save** | Save/load that works | **Ahead** — versioned, atomic, damaged-state aware, three backends |
| **Spine / 2D Animation** | Skeletal 2D | **Absent** |
| **Dialogue System** | Branching conversation | **Absent** — a profile catalog could carry the data today |
| **Localization packages** | String tables | **Absent** |
| **Peek / editor productivity** | Search, navigation, bulk edit | **Absent** |

Read together, the two tiers say the same thing three times: **camera, tweening,
and input mapping** are the features people reliably pay to add. Camera is now
built and used; input mapping is built and still stranded; managed tweening is now used in Orbital, with composition and property binding still absent.

---

## The ranked queue

Ordered by whether it stops somebody shipping a game, not by size. This is the
output of the file; everything above is evidence.

1. ~~**Field meaning in the schema registry, and array-of-object editing.**~~
   **Done.** A component says what its fields are for — asset kind, choice,
   colour, angle, bounded range, collision mask, entity reference — and the
   registry checks every path against the field template, so a renamed field is
   a startup error rather than a control that quietly stopped appearing. The
   editor's three name-keyed tables are gone, meanings are read at every depth
   rather than only at the top level, and a list of objects is editable: a
   compound collider's pieces are added to, removed from and reordered, each
   piece drawn through the meanings that describe it. A **variant tag** — a
   field that decides what else its object holds — is now switched as one edit
   at any depth: a piece's `shape` moved from `box` to `circle` takes the half
   extents away and puts a radius there, and a camera's `projection` does the
   same thing at the top level through the same code rather than through a rule
   written for cameras. Each variant is proved at startup by building the
   component it would produce and decoding it, so a variant that cannot be
   chosen safely stops the build.
2. ~~**Decay control of animation clips.**~~ **Done.** A script names one of the
   clips the scene authored, stops it, restarts a finished one, reads the frame
   it is on, and is told when a one-shot has ended. The two halves stay where
   they belong — which clip plays is written to the world, where it has got to
   is read from the cursor beside it — so a script driving an animation still
   does not rewrite the scene it came from. Gather's player was the first
   proof — its walk cycle had run even while standing still, because nothing
   could tell it otherwise — and since Gather was removed the platformer's hero
   and Scorchball's players carry it.
3. ~~**UI widget set: toggle, text input, scroll region.**~~ **Done.** Toggles,
   radio groups, text fields with a caret and the clipboard, dropdowns and
   scroll regions are authored, styled by Weave, scripted, and driven by
   pointer, keyboard and pad; the Control Room and Orbital use them, and the
   slider now has users here: Orbital's pause mixer and Sound Mixer.
4. ~~**Physics queries: overlap and shape cast.**~~ **Done.** Circles, boxes
   and capsules are overlapped and swept from the same pieces as the raycast,
   in Rust and Decay. Orbital's mine blast is an area check; Physics Playground
   shows a swept circle and an area. 3D queries wait on 3D physics.
5. ~~**Audio buses and a master volume.**~~ **Done.** Named buses under
   master, applied to live voices on every backend and set from Decay.
   Orbital's pause screen moves master, music and effects and saves them.
   Effects on a bus and snapshot blending remain.
6. ~~**Un-strand the input action layer.**~~ **Done.** A scene declares its
   actions; scripts read and rebind them. The platformer runs and jumps by
   actions and the Input example rebinds at runtime. Project-level actions,
   per-player maps and saving a rebinding remain.
7. ~~**Named collision layers instead of raw `u32` masks.**~~ **Done.** The
   physics world names its layers; Decay and the inspector use the names.
8. ~~**Tween composition.**~~ **Done** for composition: delay, loops, yoyo and
   sequences, proved by Orbital's pickup and Tween Lab. Property binding and
   editor timelines remain, and Weave keyframes are still absent.
9. ~~**Collider gizmos in the Scene view.**~~ **Done.** Every 2D collider is
   outlined from the pieces physics is given, and the selected one's edges,
   radius and height drag as one undo step. Offsets, rotations and 3D colliders
   remain.
10. **Multiple scenes and additive loading.** **Mostly done.** Scenes load
    beside each other and `Scene.go` switches between them natively and in the
    browser, and the editor's Scenes panel edits the project's scene list as a
    board of cards with the doors between them. What remains is round-tripping
    a world that holds several scenes through `to_scene`.
11. ~~**Un-strand bloom.**~~ **Done.** Bloom now lives inside the authored world post stack used by editor and browser rendering.
12. ~~**Camera follow, confine, and shake.**~~ **Done.** `sindri.camera.behavior`
    follows, confines and shakes in every gameplay host, and the platformer
    follows its hero with it. Decay camera-mode controls and authoring gizmos
    remain as the row's gap.
13. **Autotiling.** The daily cost of painting tilemaps by hand.
14. **Profiler view.** **Started.** The Profiler panel times each frame of
    Play by phase and each script within it. GPU time, a breakdown inside a
    script, and timing a shipped build remain.

Of the open items, what is left of 10 is cheap relative to its daily cost and
13–14 are real but survivable. The next queue is the gaps the done items left
behind — per-player input, a saved rebinding, contact detail, bus effects — and
the absent systems below.

### Proof that lives outside this repository

The advanced sprite colour transform still sits at ✅ ✅ ✅ ❌ — built on every
surface, used by nobody here. It and the UI slider were both found by Mujaffa
Remaster, an external project; the slider has since been picked up here, by
Orbital's pause mixer and Sound Mixer.

That is worth naming rather than tolerating quietly, for two reasons. The
capability rule does not count an external user, so the Proof column is honest
at ❌ and will stay ❌ until a game here — Orbital Last Stand, Causeway or a
genre showcase — picks each one up —
which means neither capability is complete, however finished it looks. And an
external forcing function finds real gaps: the slider is queue item 3 and the
colour transform is a genuine limit in `sindri.sprite`, so this is not drift
into features nobody needed. It is the proof step being skipped, and it is
cheaper to close now than after a third row joins them.

The two obvious closings are both small. A settings or pause screen in the
platformer or Scorchball wants a volume slider, which also gives queue item 5
somewhere to land. And
`games/orbital-baked/assets/scripts/charger.decay` recolours a baked sprite
through `sprite.tint` every frame — multiply-only, which is the exact limit the
colour transform was built to lift — so it is the natural first user.

---

## Beyond parity — where Sindri could lead

Everything above answers "what is an engine expected to do", and every row can
be checked against Unity or Godot. This section answers a different question:
**what could an engine provide that none of them do?** It is kept separate
deliberately. Mixed into the tables above, a reader could no longer tell "the
baseline has this and we do not" from "nobody has this and we might", and that
distinction is what makes the rest of this file worth reading.

Nothing here is scheduled. These are candidates, and they compete with each
other rather than with the ranked queue.

### The thesis

Sindri already refuses to hand a game raw `dt` and raw key states and wish it
luck. It has a fixed step, input edges consumed exactly once, and a seeded
stream that replays a run on every host. Read together those are one idea:

> The engine gives you primitives for translating imperfect human input and
> time into deterministic gameplay.

That is a stronger position than any single feature below, and it is a
description of what Sindri *is* rather than a direction bolted on. The
candidates worth taking are the ones that follow from it.

### Judged against the games, not against plausibility

A candidate earns a row by replacing something a game in this repository is
doing by hand today. Where a game is *not* asking for it, that is recorded too —
an idea that sounds good and nothing needs is the most expensive kind.

| Candidate | What it would replace | Position |
| --- | --- | --- |
| **Gameplay spatial queries** — nearest, within radius, within cone, within box, over tagged entities and backed by an index | Orbital player uses `World.nearest` with a `World.within_radius` fallback for off-screen targets; Arc uses sorted radius results with its impact-distance filter | **Partly taken.** `World.nearest` and inclusive radius scans use canonical world transforms and world-order ties. Cone/box queries and indexing remain deferred |
| **Entity lifecycle policies** — despawn after a duration, off-camera, or on animation end | `World.despawn(this.entity)` and a hand-decremented countdown in `bullet`, `beam`, `arc`, `core`, `charger`, `drifter`, `challenger` | **Take.** Small, and seven scripts want it |
| **Cooldowns and charges** — start, ready, remaining, normalised, recharge | `player.decay` hand-rolls `cooldown` and `mine_cooldown`; `director.decay` hand-rolls `spawn_timer` | **Partly taken.** Decay's `Timer` covers start, ready (`done`), remaining (`left`) and normalised (`progress`), and runs down on its own; charges and recharge are not built. Orbital's countdowns have not moved yet |
| **Buffered actions** — a press remembered for a window and consumed exactly once | Nothing yet; the games are not platformers | **Take, but narrowed.** See below |
| **Named time domains** — gameplay, UI, physics and real clocks, scalable and freezable | **Nothing.** No game here scales time; only pause exists | **Row, not queue.** Best fit with the thesis, no current demand, and cross-cutting: every consumer must declare a clock |
| **Gameplay sensors** — vision cone, aggro radius, interaction range, with enter/stay/leave | Enemy scripts compute their own geometry | **Downstream.** Mostly spatial queries plus a component; thin once those exist |
| **Feedback orchestration** — one asset firing sound, effect, shake, haptics, hit-stop and flash | Real glue, but Sindri has no shake, no haptics, no hit-stop, no flash, and one effect shape | **Capstone.** It would orchestrate five systems that do not exist |
| **Deterministic state history** — a short rolling window of selected properties | **Nothing.** No game here rewinds, trails, or replays | **Low.** Unusually cheap given determinism, and nothing needs it |
| **Spawn regions and patterns** — a random point in a circle, edge, or area | A few lines of Decay per use | **Utility, not a system.** Does not earn a subsystem |

### The one that changes something above

Spatial queries expose a framing error in this file's own physics section, which
lists raycast and overlap as **physics** queries. Target selection in
`player.decay` is not a physics problem — it is a gameplay query over tagged
entities. The first slice now lives in `World.nearest` and `World.within_radius`,
with Orbital proving the filtered targeting path. Physics overlap/shape casts, cone/box
queries and spatial indexing remain separate follow-up work.

Determinism also matters here in a way it does not for the baseline: a query
that answers in world order answers the same on every host, and a game built on
"the nearest enemy" then replays from a seed. Neither Unity nor Godot promises
that.

### Buffering, narrowed

The tempting version of this is a general *forgiveness* system — jump buffering,
coyote time, grace periods — authored in one block. Half of it does not belong
in an engine.

"Remember a press for 120ms and let gameplay consume it exactly once" knows
nothing about a game and is genuinely engine-level. "A grace period after being
grounded" requires the engine to know what *grounded* means, which is
game-specific: a top-down shooter has no such state, and an engine that assumed
one would be a system games fight rather than use.

So the half worth taking is the input half, and it already has a home: the
**input action layer** in `sindri-platform`, which is built and stranded.
Buffered press and consume-once are features of a named action, not a new
subsystem — which folds this candidate into an existing queue item instead of
adding an eleventh.

## Decay tooling modernization

Decay is the primary gameplay language, so language tooling is part of the
authoring surface rather than optional editor polish. The September 2026 deep
audit found concrete drift and protocol/test gaps in `decay-lsp` and the VS Code
integration. The complete prioritized checklist and definition of done live in
[`docs/decay-lsp-modernization.md`](decay-lsp-modernization.md). Update that
checklist in the same change that closes or discovers a Decay tooling gap.

## Maintenance rule

Update this file in the same change that moves any cell, as
`AGENTS.md` requires. Four specific rules keep it from rotting into a
checklist nobody reads:

1. **A surface cell claims only what is implemented and exercised.** For a
   gameplay capability, exercised means a game uses it, and the gap cell names
   which — see "What counts as proof". `capabilities.md` remains the detailed
   evidence.
2. **A parity cell is a judgement and must survive an argument.** "Ahead" is a
   claim about the baseline, not enthusiasm.
3. **Add the row before the feature.** The value of this file is that absent
   things have rows. A gap discovered during work belongs here in the same
   change, marked ❌, even when nothing is planned.
4. **Keep the two questions apart.** Everything above "Beyond parity" is
   answerable against Unity or Godot. Everything below it is not, and moving a
   candidate up requires the baseline to have grown it, not for us to have
   liked the idea. A candidate earns its place by naming what a game in this
   repository does by hand — and when no game wants it, the row says so.
