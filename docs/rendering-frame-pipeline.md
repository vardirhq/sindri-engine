# Rendering frame pipeline

Imported models use `FrameCommand::Model`: a shared immutable renderer resource
and an entity matrix. `RenderModel` retains authored local node matrices and
accumulated mesh instances separately. `TexturedCubeRenderer` owns an independent
model GPU cache/pipeline, so the existing `FrameRenderers` host contract and
inline mesh/voxel vertex layouts stay compatible. Geometry and embedded textures
upload once per shared resource; draw uniforms are independent per primitive
instance. See [imported models](imported-models.md) for the supported material
subset, diagnostics, residency behavior and deferred model shadows.

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
inline and cached voxel shadow geometry; imported model shadows remain deferred.
The fixed `examples/benchmark3d` workload and shadow-on/off native/browser captures
exercise delivery, rather than inferring it from authored component presence.

## Current boundary

The frame command enum intentionally contains only the render capabilities that exist today. It is
not a general render graph. New commands should be added when a working renderer needs them, while
asset resolution and GPU resource ownership remain outside serialized scene data.

## Physical 3D counters

`TexturedCubeRenderer::draw_stats()` returns cumulative `MeshDrawStats` for
textured cubes, inline/cached meshes, their shadow draws and imported model
primitives. Snapshot before encoding and call `since` afterwards. Submission
slot resets do not reset counters, including the reset between shadows and
color. The counters record executed encoder paths, rather than counting
`FramePass` records: a model command can draw several primitives, and empty
inline geometry or a missing cached mesh draws nothing.

`draw_calls` counts indexed draws; `render_passes` includes the shadow-map clear
as well as each draw's physical render pass. Uniform writes and their payload
bytes exclude initialization and driver padding. `transient_geometry_uploads`
counts uploaded vertex/index buffer pairs; a shadowed inline mesh currently
uploads once for each of its shadow and color draws. Persistent model asset
uploads and cached voxel geometry pairs remain separate cache counters.

These counters exclude the frame clear, sprites, shapes, glyphs, post-processing,
texture initialization and UI. They are not whole-frame GPU totals.
`project-benchmark` records per-frame deltas under `mesh_counters`, and
`scripts/frame-benchmark.py` prints their means when present. Reports without these counters retain their CPU summary. Editor reports now
record the same counters per primary view, with `scene_mesh_counters` for the
extra Scene view beside Play. Missing views are omitted, while an encoded view
with no 3D work records zero. Repeated draws in one editor frame accumulate;
frame close/capture and Play reset follow the existing profiler lifecycle.
Standalone native benchmarks now optionally bracket the prepared frame with
GPU timestamps; editor timestamp delivery remains open. GPU completion wait
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
unavailability. Editor/browser timing and whole-frame draw/upload totals remain
open. This measurement foundation is proven by the fixed feature example,
without claiming gameplay capability completion.
