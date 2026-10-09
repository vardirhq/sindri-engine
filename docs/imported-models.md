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
The component has no invented default model and no editor authoring UI.

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
before it draws. Model authoring (thumbnails, dragging a model in, triangle
picking, an asset picker) is the next slice of [the 3D update](3d-update.md).
External Low Tide is the request that found this gap; it does not by itself
satisfy this repository's in-tree game proof rule.
