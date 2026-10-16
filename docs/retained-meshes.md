# Retained inline meshes

Inline `sindri.mesh` surfaces use immutable `RenderMesh` resources with u32
indices. This is a general renderer capability exercised by 3D Benchmark and
Lighting Lab for the planned Explorer/Low Tide 3D rendering track. In-tree game
adoption remains open; this does not complete M8 or the material/batching track.

## Ownership and invalidation

`SceneExtractor` keeps one resource per active surface entity. Its decoded
component cache avoids deserializing unchanged mesh payloads. Inline extraction
opts into `Decoded::retain_active`, immediately pruning removed, disabled and
no-longer-mesh decoded payloads instead of keeping the generic amortized cache
allowance. Entity revision and texture-binding generation provide a fast path;
when either changes, source
geometry and resolved atlas rectangle determine whether a replacement is needed.
Transforms, cameras, layers, material factors and texture-ID changes with the
same UV rectangle reuse geometry. Position, UV, normal, index or atlas-rectangle
edits create a new resource. Different entities currently have separate resources,
even when their authored geometry is identical.

Prepared frames own strong references. They can still draw an older revision
after edits, undo, removal or a later extraction. Successful extraction drops
cache ownership for inactive, removed or no-longer-surface entities. Each
`TexturedCubeRenderer` lazily uploads a resource once and shares that buffer pair
between color and directional-shadow draws. Its weak GPU cache drops dead owners
at the next submission; resource identity cannot be reused while an entry lives.
Each renderer/device has its own cache, including the editor's separate views.
Per-draw uniforms remain independent of shared geometry.

## Validation and compatibility

`RenderMesh::new` rejects nonfinite position/UV/normal/AO attributes, nontriangle
index counts and out-of-range indices. Empty geometry draws nothing. The budget
is 1,048,576 vertices and 3,145,728 indices per resource, below baseline WebGPU
buffer limits. Scene extraction returns a typed geometry error for invalid
triangles; mismatched position/UV counts retain the existing skip behavior.
Authored normals retain their existing component validation.

Existing scene JSON numbers remain compatible; surface indices may now exceed
65,535. The explicit low-level `FrameCommand::TexturedMesh` remains a transient
u16 API. Voxel `CachedMeshId` commands retain their existing revision/retirement
protocol in a separate cache namespace.

## Measurement and proof

The fixed 300-prop benchmark targets zero steady-state inline geometry uploads,
with unchanged draw/pass counts and appearance. Its 100 inline surfaces require
100 buffer pairs on first use, shared between shadows and color. Settled frames
report zero `transient_geometry_uploads` and zero `cached_geometry_uploads`.
The latter now includes retained inline resources alongside voxel cache uploads;
model uploads remain separate. This slice does not promise fewer draws or a
portable frame-time improvement.

Scene regressions cover reuse, atlas changes, checked edit/undo/redo, old-frame
ownership, disabling/despawn and u32 indices. GPU standard-material tests exercise
the retained path alongside transient, voxel and imported geometry for factors,
emission, local lights and transformed normals. A GPU lifetime test draws indices
above 65,535, checks one upload across repeated shadow/color draws and retires the
resource after all owners disappear.

Native/editor settled counters meet the zero-upload target. Release browser
exports run the benchmark, no-shadow and large-index controls, smooth/flat
normals, and Lighting Lab through their ready goals with real asset fetches.
The large-index capture matches the original byte for byte. Detailed measurements
and host controls are in [the baseline record](3d-benchmark-baseline.md).

Instancing, shared identical geometry between entities, stage pass consolidation,
frustum culling, material assets and game integration remain separate follow-ups.
