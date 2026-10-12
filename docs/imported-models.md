# Imported models

Status: bounded static GLB decoding, reusable GPU rendering, scene references,
external-project export and native/browser project loading are implemented.
The actual Low Tide cutaway crawler has been visually inspected in native
offscreen Sindri and exported WebGPU Sindri. This is static model support, not
the whole glTF specification or completed imported-model editor UX.

## Asset boundary

`sindri-assets::ModelAssetDecoder` consumes bytes from the normal asynchronous
asset source/queue/store pipeline. It accepts self-contained binary GLB 2.0,
using `gltf` with `utils` and `names`, without the library's filesystem importer.
Neither GPU ownership nor synchronous browser I/O belongs to this decoder.
`AssetLoader<ModelAssetDecoder>` coalesces repeated requests by logical asset ID.

`ModelAsset` retains nodes in their original index order, names, children,
local column-major matrices, scene roots, and mesh references. Geometry is
stored once per mesh primitive even when multiple nodes reference it. Matrices
are glTF Y-up already; do not convert source construction coordinates again.
The renderer bridge must multiply the entity world matrix by each accumulated
node matrix, retaining local node data for later independent manipulation.

Primitives retain positions, normals, UV0, material indices, and 16-bit or 32-bit
indices. Non-indexed triangles acquire sequential 32-bit indices. Missing
normals are generated from triangle geometry; missing UVs are zero only when
the primitive does not need a texture. Base-color factors are linear RGBA.
Materials retain metallic/roughness factors, double-sidedness and alpha cutoff.
Embedded PNG/JPEG base-color images use the existing texture decoder; sampler
wrap and filter enums are retained for GPU upload.

## CPU collision preparation

`ModelAsset::collision_mesh(ModelCollisionLimits)` produces owned positions and
triangle indices in model space, with one source range per instantiated node,
mesh and primitive. It walks selected roots depth-first in authored order and
accumulates column-major local matrices. Repeated meshes expand at each selected
node; unreachable nodes stay out. Entity world transforms remain the host's
responsibility. No coordinate conversion, material filtering, welding, hull
construction, physics body or runtime collider is implied.

This foundation is added for the planned Explorer showcase. Assets own CPU
geometry preparation; physics will own backend geometry and queries; scene and
runtime will bind logical model references through shared geometry resources.
The asset crate acquires no physics, renderer, scene or new math dependency.

Reflection reverses each triangle's second and third index, preserving authored
orientation under negative node determinants. Both decoded index widths and
generated non-indexed triangles produce checked u32 indices. Per-part vertex and
triangle ranges retain original node, mesh and primitive indices. Normals, UVs
and materials do not enter collision preparation; masked holes therefore have
no implied collision policy.

Default limits allow 65,536 selected hierarchy nodes, 1,000,000 expanded
vertices and 1,000,000 triangles, counting repeated instances. Traversal is
iterative, including deep chains. Limits are checked before expanding pending
children, appending geometry or copying indices. Vertex indices also stay within
u32 range. A caller can supply other limits; zero rejects nonempty data.

Preparation validates the selected traversal and geometry even when public
asset fields were edited after decoding. Missing/repeated nodes, missing meshes,
empty/incomplete/out-of-range triangles, nonfinite vertices, singular/projective
transforms, overflowing accumulated matrices/positions and degenerate transformed
triangles return `ModelCollisionError`. Unselected data is not revalidated here;
the decoder remains responsible for the complete GLB. Failure returns no partial
mesh and never changes the original model. Affine matrix products and triangle
area checks use f64 intermediates with checked narrowing to model-space f32.

Six native regressions use the existing static GLB fixture and cover hierarchy,
reused meshes, source ranges, both index widths, non-indexed geometry, reflection,
exact bounds, 8,192-node traversal and malformed mutable data. The inspector
prints 15 vertices, five triangles and five primitive instances for that fixture:

```bash
cargo run -p sindri-assets --example model -- \
  crates/sindri-assets/tests/fixtures/models/static-model.glb --collision
```

This is CPU preparation proof. Physics now provides immutable static triangle
resources and standalone keyed groups with native query/landing/replay tests;
see [the physics contract](physics.md#static-triangle-geometry). Standalone convex
hull resources and mixed rigid bodies now have native landing/control/replay
tests as well. Static model scene bindings and native/browser/editor-session
feature proof are described below. Explicit convex model scene bindings now have native regression proof;
automatic convex loading/authoring and Explorer game proof remain pending.

## Renderer boundary

`sindri-render::RenderModel::new(ModelData)` validates immutable geometry,
materials, images and hierarchy without depending on assets or scenes. The
public facade adapts decoder data once; hosts share `Arc<RenderModel>`
instances. The original
node names, indices, local matrices and mesh references remain available; a
separate list accumulates selected-scene node matrices for drawing.

`FrameCommand::Model` carries that shared resource and the entity world matrix.
The GPU cache uploads each mesh primitive and base-color image once, reuses
them across nodes, instances and frames, and releases entries when the resource
loses its last owner. Cache statistics expose residency, uploads and draws.
Each draw has an independent reusable uniform slot.

The model vertex path retains normals and UVs instead of converting through
inline `SurfaceMesh` or the existing position/UV/AO vertex format. Both index
widths are used directly by the GPU. Drawing composes entity and hierarchy
matrices, transforms normals by inverse transpose, writes/tests depth, and
handles mirrored instances and double-sided materials. Embedded base-color
textures are sampled as sRGB, factors are linear, and alpha masks discard.
Ambient plus the existing directional world light uses metallic/roughness
factors in a direct GGX lighting approximation. Environment-map lighting,
texture mipmaps, model shadows and atmosphere are currently deferred. Existing
mesh lighting, shadows and vertex layouts are unchanged.

Invalid render resources and singular/nonfinite model matrices return typed
errors. Geometry buffers and image dimensions are checked against device
limits before upload. The pipeline uses target-independent wgpu APIs.

## Scene and facade boundary

A scene entity references an external asset without embedding its vertices:

```json
{ "components": { "sindri.model": { "asset": "models/crawler.glb", "layer": 0 } } }
```

`ModelComponent` validates the logical `AssetId`. `referenced_models(world)`
deduplicates all references, including inactive entities needed by future scene
switches. Hosts decode once, call `sindri::model::prepare` once per asset and
bind the shared resource with `SceneExtractor::bind_model`. Missing bindings
name the asset in a typed extraction error. Active instances extract into the
opaque 3D stage with the authored layer, world camera and entity world matrix.
The facade performs the conversion so neither scene nor render depends on the
asset crate. Local node matrices remain Y-up and are never converted twice.
The component has no invented default model. The editor completes Add Component
with the first supported GLB in the project, and its asset field uses the model
picker described below.

## Supported subset and diagnostics

Only triangle primitives and one embedded binary buffer are supported. External
buffer/image URIs, sparse accessors, nontriangle geometry, blending, skinning,
morph targets, additional vertex attributes and non-base-color textures fail
with an asset-aware `AssetDecodeError`. Required extensions fail explicitly.
Optional extensions, animations and cameras produce retained warnings and do
not drive Sindri lighting, motion or cameras. Hosts must report these warnings.

Buffer views and accessor ranges are checked before the library's geometry
readers run. Attributes must have matching lengths and finite values, indices
must form complete in-range triangles, and node cycles/multiple parents fail.
This is a deliberately bounded static model import, not the whole glTF spec.

## Verification

`crates/sindri-assets/tests/fixtures/models/static-model.glb` is a deterministic
small fixture with two mesh primitives/materials, both index widths, a reused
mesh, parent/local transforms, non-indexed geometry, and one embedded texture.
Its sibling `generate.py` reproduces it using the Python standard library.
Decoder tests cover these plus malformed ranges, cycles, unsupported features
and repeated asset requests. Renderer tests cover hierarchy and invalid resource
validation. The GPU imported-model test reads back material/texture pixels,
checks distinct instance uniforms, mirrored-instance depth, both index widths,
and one upload across repeated draws, followed by residency release. Scene and
facade tests cover logical references, world transforms, disabled discovery and
conversion without an axis change. Native queue tests bind repeated scene
references to one resource. Export tests cover project-root/asset-directory
layouts, repeated references, other scenes, placed prefabs, original bytes/hash,
and missing/malformed model diagnostics. No unit
test depends on the external crawler.

Inspect an actual file without claiming a render proof:

```bash
cargo run -p sindri-assets --example model -- path/to/model.glb
```

## Project loading and export

`AssetKind::Model` identifies binary GLB assets in the manifest. Export walks
all listed scenes and expanded prefab worlds, including inactive references,
validates models through the same decoder and packages original bytes under
the existing content hash. Repeated references produce one manifest entry.
Projects with assets beside `sindri.toml` and those using `assets/` both work.
Missing files and unsupported/malformed GLBs stop export with named diagnostics.

Both of `sindri-player`'s sources load models. The browser source sizes a
model queue from the manifest, verifies fetched bytes and decodes them; the
native source (`sindri-player <project>`) decodes every `.glb` under the
project's assets. `Player::install` prepares and binds each logical model once,
and reports each decode warning against its asset ID. `project-capture` and
`project-benchmark` bind every `.glb` the project holds through the filesystem
source and the same asynchronous queue and decoder, so a scene reached later,
or a prefab spawned later, draws without a load. None reparses model instances
during extraction. Every host applies authored ambient and directional lighting
before drawing. A project's assets are `assets/` when it has one and the
project root otherwise (`sindri_runtime::project::assets_root`), as export
reads them.

## External crawler proof

[Low Tide PR #1](https://github.com/vardirhq/low-tide-3d/pull/1) references the
unchanged `Low_Tide_Kit/starter_crawler_cutaway.glb`. Native and Chrome headless
WebGPU captures at 1200 × 1000 were visually inspected on 2026-10-09: the tracks,
ramp, wood deck, furniture and cabin are visible with correct Y-up orientation,
material colors and orthographic three-quarter framing. The model has 177
nodes, 19 meshes, 85 primitives, 16 materials and no base-color textures.
No construction-coordinate conversion, inline vertices or replacement art is
used. A simple seabed, ambient light and directional sun are authored in the
external scene; no Low Tide behavior is added to engine crates.

The export carried the scene, a solid ground texture and the original
888,192-byte GLB, SHA-256
`de9a1ec5d3d99cd8e2ca47ebe7e48d9fb202ccd4fddd09be85b6ea4818730ec2`.
Browser smoke confirmed WebGPU, loading completion and HTTP delivery of all
three kinds. See the external project's
[proof and reproduction commands](https://github.com/vardirhq/low-tide-3d/blob/poc/isometric-crawler/docs/POC.md).
Imported-model shadows remain deferred and disabled in the proof.

## Editor boundary

The editor loads the models a scene and its prefabs name through the same
queue and decoder as its textures (`editor/src/textures/models.rs`), prepares
each once and binds it to both the Scene and Game views' extractors. A changed
file is loaded again and rebound. Decode warnings and failures are console
lines. Until a model arrives, or when its file will not decode, the editor's
tolerant extraction draws the rest of the scene and names the waiting entity
as a problem; a game still fails on an unbound model, because every host binds
before it draws. Both render-model and static-model-collider `asset` fields now
declare `FieldMeaning::Asset(AssetKind::Model)`. The inspector offers project GLBs
(case-insensitive extension matching), keeping glTF/OBJ/FBX files visible in the
project browser without offering them as supported model choices. References
remain typeable and schema checked. Add Model and Static Model Collider complete
their field templates with a real project GLB; without one they are withheld.
Rendered models belong to world space, and static collision cannot share its
owner with other physics bodies, colliders or characters. Render and collision
fields have separate widget scopes and separate component undo entries; a drag
within a component still merges. Native directory suffix discovery also ignores
ASCII case without changing asset IDs, so a chosen `.GLB` runs natively.

Native [authoring review](model-collision-editor-review.md#model-asset-authoring)
verifies independent selections, Undo/Redo, Add Model, save/reopen and Play using
a different GLB wall, then opens that authored project in the real native player
window and visually verifies wall obstruction and the walker on the goal pad. This is general authoring for the imported-content track,
requested by external Low Tide; Explorer's in-tree game adoption remains open.
Thumbnails, model drops, triangle picking, node inspection and material overrides
remain in [the 3D update](3d-update.md).
External Low Tide is the request that found this gap; it does not by itself
satisfy this repository's in-tree game proof rule.

## Explicit runtime collision binding

A decoded `ModelAsset` can now be delivered to
`Session::bind_model_collision(id, &asset)` for an authored
`sindri.physics3d.model_collider` reference. CPU preparation expands the selected
GLB hierarchy once; the scene driver owns static geometry and applies composed
positive scale, pose changes and lifecycle on its shared fixed step. Rendering
and collision references are independent. Native Session tests use the binary
GLB fixture for character obstruction, checkpoint replay and asset replacement.
See [scene ownership and limitations](physics.md#static-model-scene-ownership).

This is an explicit host API. Shared asset discovery and export include
collision-only references, including inactive entities and prefab contents, and deduplicate render/collision uses.
Export verifies and ships the original GLB bytes. The shared native/browser
player now prepares collision only for authored model-collider references after
loading completes, before starting Session. Discovery expands every project scene
and prefab, including instance overrides and inactive entities, so later scenes
and spawned content already have bindings. Missing assets or failed collision
preparation report the logical asset ID. Render-only models need not contain
valid physics geometry. Native headless `ProjectRun` loads only authored collision
references through the filesystem worker queue before stepping, including later
scenes and spawnable prefabs, in both project asset layouts. Render-only model
bytes remain a drawing host's responsibility.

Editor Play binds decoded collision models from its asynchronous asset queue
before creating the recording or starting scripts. Startup discovers the live
world's unsaved edits, inactive entities, declared later scenes and expanded
spawnable prefabs. Later-scene collision assets remain retained for the run.
If an asset is still loading, Play reports its ID and stays stopped; retry when
loading completes. Missing, malformed and invalid collision geometry also keep
Play stopped. A successful GLB reload replaces render and collision resources
before the next step; failed collision preparation retains the previous binding.
Stop releases Play-only retention and restores the authored world. New collision
asset references introduced during a run require stopping and starting Play again.

[Model Collision Lab](../examples/model-collision/README.md) is the player feature
proof: one GLB supplies the rendered ground/wall and static collision, while
Decay drives the walker into the wall and around it to a goal. The native player
regression opens/compiles the project and steps the keyboard route through
Session. Editor Play and `ProjectRun` now run the same obstruction/bypass/goal
route with matching world state at every step; checkpoint restoration and replay
retain wall collision. Headless regressions cover collision-only layouts,
inactive missing/malformed assets, later scenes and nested prefab overrides.
The exported browser smoke plays the same route with ordinary keyboard
input, verifies the GLB fetch and checks runtime/GPU errors. A native editor
[review](model-collision-editor-review.md) verifies the goal, moved-wall reload,
invalid-reload retention, Save refusal and Stop restoration. It remains feature
proof; Explorer game adoption is open.
Convex model scene authoring and Explorer proof also
remain scheduled in slice 6; rendered GLBs do not automatically acquire collision.


## Explicit convex model binding

`Session::bind_model_convex_collision` prepares at most 4,096 expanded vertices
from a decoded selected GLB hierarchy, then builds shared scene-owned volumetric
geometry. The `sindri.physics3d.model_convex_collider` component can share a rigid
body and primitive pieces; a single hull fills all model gaps and concavities.
Planar/oversized/invalid geometry fails atomically. Positive composed nonuniform
scale, dynamic writeback, lifecycle and checkpoint-aware replacements have native
scene and Session regression evidence. See [ownership and limits](physics.md#convex-model-scene-ownership).
Convex-only references now participate in discovery/export and asynchronous
player loading, native headless runs and editor Play. Discovery retains all
representations needed per logical asset, and atomic preparation prevents a
failed hull reload from partially replacing shared triangle collision. Editor
Add Component uses a project model and enforces compatible ownership. Model
Collision Lab exercises a falling imported prop and a Decay impulse/travel goal;
Explorer adoption remains open.
