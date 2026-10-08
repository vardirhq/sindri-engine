# Imported models

Status: CPU decoding, reusable GPU model rendering and scene references are
implemented. Host loading, export packaging and crawler visual proof remain open.
Do not treat these building blocks as a completed external-project capability.

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
wrap and filter enums are retained for eventual GPU upload.

## Renderer boundary

`sindri-render::RenderModel::new(ModelData)` validates immutable geometry,
materials, images and hierarchy without depending on assets or scenes. Hosts
adapt decoder data once and share `Arc<RenderModel>` instances. The original
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
and one upload across repeated draws, followed by residency release. No unit
test depends on the external crawler.

Inspect an actual file without claiming a render proof:

```bash
cargo run -p sindri-assets --example model -- path/to/model.glb
```

## Remaining integration

- Exercise depth-tested material primitives through native and WebGPU hosts.
- Discover scene/prefab model references in the exporter and package the GLB
  unchanged in the existing content-hashed manifest layout.
- Continue external `low-tide-3d` PR #1 with the real cutaway file and verify
  recognizable Y-up rendering through an orthographic three-quarter camera.

## Editor boundary

One editor test exhaustively matching frame commands accepts the new model
variant. No editor implementation or UI changes are part of this slice.
Editor PR #503 introduces runtime and
player crates and moves browser/project host code; coordinate those host moves
at integration time rather than merging or editing its branch. Full model
authoring UI remains deferred. External Low Tide is the request that found this
gap; it does not by itself satisfy this repository's in-tree game proof rule.
