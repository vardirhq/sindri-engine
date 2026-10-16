# Rendering frame pipeline

Imported models use `FrameCommand::Model`: a shared immutable renderer resource
and an entity matrix. `RenderModel` retains authored local node matrices and
accumulated mesh instances separately. `TexturedCubeRenderer` owns an independent
model GPU cache/pipeline, so the existing `FrameRenderers` host contract and
inline mesh/voxel vertex layouts stay compatible. Geometry and embedded textures
upload once per shared resource; draw uniforms are independent per primitive
instance. See [imported models](imported-models.md) for the supported material
subset, diagnostics, residency behavior and model shadow casting/reception.

Sindri separates a frame into three explicit stages:

1. **Extraction** converts world and scene state into render commands without issuing GPU work.
2. **Preparation** validates frame-wide state and deterministically orders passes by stage, layer,
   and stable insertion order.
3. **Rendering** consumes only the prepared frame and dispatches commands to GPU renderers.

`ExtractedFrame` owns the viewport, clear operations, and unordered passes. Calling `prepare`
rejects empty viewports and non-finite clear values before returning `PreparedFrame`.

The current stage order is:

1. `Opaque3d`
2. `Transparent2d`
3. `Overlay`

Layers order passes within a stage. Equal stage/layer pairs preserve extraction order. Transparent
sprites inside a batch continue to use `TransparentOrder` before they enter the frame packet.

`Transparent2d` holds world sprites, which are drawn through the world camera and tested against the
depth the opaque stage wrote; `Overlay` holds the `sindri.ui.*` family, which nothing in the world
may hide, ordered by the same layers. See [scene extraction](scene-extraction.md) for which
component draws in which space and [transparency](rendering-transparency.md) for what each does
about depth.

## Clearing belongs to the frame

`ClearOperations` is frame-wide state, and the frame is cleared once by
`encode_clear` before any pass draws. Every renderer afterwards loads both
attachments.

It used to belong to the opaque mesh pass, which held only while a scene had
exactly one mesh and at least one: a second mesh cleared away the first, and a
scene with no mesh cleared nothing at all, leaving its sprites drawing over the
previous frame and testing against a depth buffer nobody had filled. A scene of
only sprites is what a 2D game is, so that case is the ordinary one.

## Drawing a prepared frame

`encode_prepared_frame` is the third stage: given the renderers, a target, and a prepared frame, it
clears once and encodes each pass in the order preparation put them in. It knows nothing about
worlds, components, or scenes — a prepared frame is a list of commands and the matrices to draw them
with — which is why it belongs in `sindri-render` rather than in whichever host needed it first. It
lived in the cube example for a release, which made the editor depend on an example in order to draw
anything at all.

The cube example is the reference integration. A versioned `SceneDocument` is deserialized from
JSON, validated, loaded into `World`, and extracted into an opaque textured-cube pass followed by a
five-instance overlay pass. Native, WebGPU, the software-Vulkan capture, and the editor's two
viewports all consume that same prepared frame through that same encoder.

Text is a frame command rather than host UI. `TextInstance` carries a logical
font reference, pixel position, metrics, colour, and content; `TextRenderer`
resolves only project fonts a host explicitly bound, shapes them with Glyphon,
and loads the existing colour attachment so text composes at its ordered overlay
layer. An absent font skips the string and is diagnosed by asset loading rather
than silently selecting a system fallback.

## Every sprite batch owns its buffers

A frame can hold several sprite batches, and each of them draws with its own
camera and its own instances. That is now true because each batch takes a slot
of its own — its own uniform buffer, its own instance buffer, its own bind
groups — rather than sharing one set with the rest of the frame.

It was not true before, and the way it failed is worth keeping written down.
The renderer held one uniform buffer and one instance buffer, and wrote both
once per batch on the way to encoding that batch's pass. `queue.write_buffer`
does not write when it is called: it stages the write, and every staged write
lands *before* the command buffer that follows executes. So all of a frame's
batches wrote in turn, the last one won, and every pass drew with the last
batch's camera and the last batch's instances. A frame with a single sprite
batch is unaffected, which was every proof, every example, and every test in
the workspace — while any scene that mixed world sprites with a screen overlay
drew the world through the overlay's camera. Building the companion game is
what found it, because the game is the first thing here with both.

`SpriteBatchRenderer::begin_submission` hands the first slot back, and slots are
reused rather than freed, so a frame costs nothing after the first one of its
shape. It is a *submission* boundary and not a frame boundary, for the same
reason the bug existed: a slot may only be reused once the commands that read
it have been submitted. `encode_prepared_frame` calls it, so one call is one
submission — a host drawing two frames at once, as the editor does with its
scene view and its game view, submits between them.

`crates/sindri-gpu/tests/sprite_batches_are_independent.rs` is the check. It
draws two batches into one frame, at two scales and with two instance counts,
and reads back the pixels that only come out right when each batch kept its
own.

## Authored shadow settings in every project host

The shared native/browser player and offscreen `ProjectRenderer` apply the
validated environment's `shadow_settings` before encoding each frame, as editor
viewports do. An absent environment restores defaults, so a previous scene's
shadow state cannot leak into the next. The same encoder handles primitive,
inline and cached voxel shadow geometry; imported models cast into and receive that map using cached geometry.
The fixed `examples/benchmark3d` workload and shadow-on/off native/browser captures
exercise delivery, rather than inferring it from authored component presence.

## Current boundary

The frame command enum intentionally contains only the render capabilities that exist today. It is
not a general render graph. New commands should be added when a working renderer needs them, while
asset resolution and GPU resource ownership remain outside serialized scene data.

## Physical 3D counters

`TexturedCubeRenderer::draw_stats()` returns cumulative `MeshDrawStats` for
textured cubes, inline/cached meshes, their shadow draws and imported model
color/shadow primitives. Snapshot before encoding and call `since` afterwards. Submission
slot resets do not reset counters, including the reset between shadows and
color. The counters record executed encoder paths, rather than counting
`FramePass` records: a model command can draw several primitives, and empty
inline geometry or a missing cached mesh draws nothing.

`draw_calls` counts indexed draws; `render_passes` includes the shadow-map clear
as well as each draw's physical render pass. Uniform writes and their payload
bytes exclude initialization and driver padding. `transient_geometry_uploads`
counts uploaded vertex/index buffer pairs for explicit transient commands. Scene
inline surfaces now use immutable retained resources: one upload shared by shadow
and color draws, included with voxel uploads in `cached_geometry_uploads`.
Persistent model uploads remain separate. See [retained meshes](retained-meshes.md)
for ownership, revision and retirement rules.

These counters exclude the frame clear, sprites, shapes, glyphs, post-processing,
texture initialization and UI. They are not whole-frame GPU totals.
`project-benchmark` records per-frame deltas under `mesh_counters`, and
`scripts/frame-benchmark.py` prints their means when present. Reports without these counters retain their CPU summary. Editor reports now
record the same counters per primary view, with `scene_mesh_counters` for the
extra Scene view beside Play. Missing views are omitted, while an encoded view
with no 3D work records zero. Repeated draws in one editor frame accumulate;
frame close/capture and Play reset follow the existing profiler lifecycle.
Standalone native benchmarks now optionally bracket the prepared frame with
GPU timestamps. Native editor benchmarks reuse the same timer per viewport. GPU completion wait
is still a CPU stall, not GPU execution time. This is feature-example instrumentation, proven
with the fixed 3D Benchmark and the imported-model GPU readback regression.


## Native benchmark timestamps

`project-benchmark` requests optional encoder timestamps and enables timing on
its offscreen renderer. Each frame writes `gpu_execution_us` separately from
`phases`; unsupported devices write `null` and metadata says timing is
unavailable. Metadata records adapter/backend, timestamp period and scope.
`scripts/frame-benchmark.py` prints GPU execution mean/p95 without adding it to
CPU work or frame-interval sums. Old reports retain their existing summaries.

The interval covers the frame clear, shadow work and all prepared-frame draws.
Query resolution/copy, mapping, readback and presentation lie outside it. This
is the GPU's bracketed command interval, not a sum of shader times; backend
reordering and timestamp precision can affect it. CPU encoding includes the
extra query commands, and benchmark readback is deliberately synchronous.
Software-Vulkan CI requires real nonzero timing for both shadow controls and
exercises query-buffer reuse. A baseline-device GPU test proves graceful
unavailability. Browser timing and whole-frame draw/upload totals remain open. This measurement foundation is proven by the fixed feature example,
without claiming gameplay capability completion.


Editor benchmark launches wrap eframe's existing device descriptor to request
only supported timestamp features while preserving its limits and required
features. Interactive launches keep the default descriptor. Each viewport owns
its own query pair through the existing engine facade, without a new dependency.
Reports add primary `gpu_execution_us` and extra `scene_gpu_execution_us`, with
null for unavailable or undrawn intervals. Repeated draws accumulate per scope;
capture, frame and Play resets follow the existing profiler lifecycle. Execution
never enters CPU phases; `gpu` continues to book CPU completion/readback wait.

Editor metadata records adapter/backend, period and interval scope. Intervals
cover viewport clear/shadows/draws and optional world post-processing, excluding
egui painting, scene-board copies, query readback and presentation. Queue
uploads scheduled before encoder execution are outside the timestamp bracket. Native CI
requires nonzero primary timestamps for both shadow controls. A real two-view
layout exercises independent Scene/Game intervals; profiler/report regressions
protect accumulation, reset, microsecond serialization, null and measured zero.
These reports do not make the interactive Profiler panel a GPU profiler.


## Physical 2D batch counters

`SpriteBatchRenderer`, `ShapeRenderer` and `GlyphRenderer` expose cumulative
`BatchDrawStats`. Their actual indexed draw sites record physical draws/passes,
instance counts, the two per-batch queue buffer writes and uniform/instance
payload bytes. Sprite camera/time/mask uniforms now occupy 160 bytes; shape and
glyph camera uniforms remain 64. Shared 2D lighting uniforms/shadow textures are
resource updates outside this per-batch scope. Empty batches return before any
write or pass. Submission slot resets preserve these counters, including the
resets between editor views.
`combined` merges independent snapshots; `since` gives one view's delta.

Standalone reports add `batch_counters`. Editor reports add the same primary
scope and `scene_batch_counters` for the extra Scene view beside Play. Missing
views remain absent, encoded empty views record zero, and repeated draws
accumulate with the profiler frame lifecycle. Summaries print these scopes
separately from 3D counters and CPU/GPU timings. Old reports stay readable.

A real GPU regression encodes empty and nonempty sprite, shape and glyph batches
through two submissions, reads the target and verifies exact draw/write/byte
counts and slot reuse. Platformer's native editor/standalone reports exercise
batch counters in a game; their check allows transient sprite variation between
different simulation times while verifying the exact per-batch payload
accounting against the stable non-sprite workload. Rebuilt browser smoke checks
the shared render path.
The fixed 3D workload records measured zeroes for these batches in native CI.
Resource initialization, atlas/texture uploads, frame clears, post-processing,
scene-board copies and egui's editor UI remain outside these counters. The
whole-frame counter item remains open.

## Shared world effects on imported geometry

`world_effects.wgsl` supplies directional-map visibility and atmosphere for both
textured world and imported-model shaders. The map uses one light projection,
bias, bounds check and comparison sampler. Only direct model lighting is shadowed;
ambient remains available. Fog is applied after surface lighting, with the same
camera-distance, density and height terms as world meshes. Cube shader extraction
preserves its existing calculations. Model uniforms grow from 288 to 400 bytes;
physical draw/pass counts and cached geometry residency are unchanged.

Resizing the map invalidates model draw bindings as well as textured-mesh
bindings, so slot reuse never samples a previous map. The renderer accepts fog
from every caller; authored fog delivery remains editor-only until shipped hosts
forward the environment settings. Imported geometry casts shadows through its own material-aware depth pipeline.

## Imported casters in the shadow stage

Both frame encoders now include imported-model primitives in the directional
shadow stage, with typed errors propagated through the existing frame error.
The shadow renderer reuses the color cache, retained hierarchy and index widths;
its independent uniform slots prevent queued color writes from corrupting depth
transforms. Each primitive writes 96 bytes (light/world matrix, base color and
mask/facing settings). Shadow bindings exclude the receiving depth texture to
avoid sampling an attachment being written. Alpha masks and single/double-sided
facing, including mirrored nodes/entities, follow imported material semantics.
World-mesh cutout casting remains pending. The fixed workload now issues 602
shadowed draws and 603 passes, with 126,784 uniform bytes; its unshadowed counts
remain 301/301 and 104,320 bytes. Geometry residency is unchanged.

## Standard factors on frame passes

`FramePass.sidedness` independently selects inherited, single- or double-sided
visibility in both color and shadow draws. Imported settings are inherited per
primitive; other geometry defaults to single-sided. Each pass resets the policy
without replacing its geometry. See [face visibility](rendering-materials.md#face-visibility).

`FramePass.material` carries optional renderer-owned numeric factors, validated
at preparation. Both shadow and color dispatch select factors for each pass,
including resetting absent overrides; renderer state cannot leak a previous
entity's material into the next. Scene extraction attaches the owning entity's
`sindri.material` to primitive, imported and voxel passes. Shared GGX and emission
are an opt-in mesh path; absent model overrides use original primitive factors.
Texture/alpha/sidedness ownership stays separate. Textured/model color uniforms
are 368/416 bytes; the fixed workload now writes 138,032/115,568 bytes with shadows
on/off, without changing draw/pass counts or geometry residency. See
[standard materials](rendering-materials.md).


## Textured vertex normals and orientation

Inline vertices and cached replacements carry optional object-space normals;
zero requests geometric flat shading. Frame preparation rejects nonfinite values
with `FramePlanError::NonFiniteNormal` before GPU encoding. Textured vertices
are now 36 bytes, while color/shadow uniforms are unchanged. World color and
shadow dispatch select the clockwise pipeline for a negative model determinant,
including cached draws without replacements. See [normal semantics and proof](rendering-materials.md#textured-geometry-normals).
