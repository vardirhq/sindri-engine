# Imported models

Status: first implementation slice, CPU decoding only. Runtime rendering,
scene references, GPU ownership, export packaging and visual proof remain open.
Do not treat a decoded model as a rendered one.

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
and repeated asset requests. No unit test depends on the external crawler.

Inspect an actual file without claiming a render proof:

```bash
cargo run -p sindri-assets --example model -- path/to/model.glb
```

## Remaining integration

- Add a distinct model asset reference component instead of serializing
  imported geometry into `SurfaceMesh`.
- Bind decoded assets at the scene/render seam without making the renderer
  depend on core, scenes, assets or the editor.
- Retain GPU geometry and textures across instances/frames, with release and
  revision handling; apply normals with an inverse-transpose world matrix.
- Draw depth-tested material primitives through native and WebGPU hosts.
- Discover scene/prefab model references in the exporter and package the GLB
  unchanged in the existing content-hashed manifest layout.
- Continue external `low-tide-3d` PR #1 with the real cutaway file and verify
  recognizable Y-up rendering through an orthographic three-quarter camera.

## Editor boundary

No editor changes are part of this slice. Editor PR #503 introduces runtime and
player crates and moves browser/project host code; coordinate those host moves
at integration time rather than merging or editing its branch. Full model
authoring UI remains deferred. External Low Tide is the request that found this
gap; it does not by itself satisfy this repository's in-tree game proof rule.
