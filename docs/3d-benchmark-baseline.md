# Fixed 3D workload baseline

Recorded October 13, 2026, for `examples/benchmark3d`: 100 cubes, 100 inline
pyramids, 100 references to one GLB and a cube floor. The perspective camera
uses a 35-degree vertical FOV. Directional shadows are enabled at 1024 pixels
with 100-unit coverage. Imported models still neither cast nor receive shadows.

## Delivery evidence

The native project test loads real assets, compiles the Decay observer and
reaches ready after 30 fixed steps. After another 60 steps, every pose remains
unchanged. Extraction produces 101 cube commands, 100 inline meshes and 100
models sharing one decoded resource. These are command counts, not GPU draw
or render-pass counters.

Native offscreen and real Chromium captures show all three groups. Turning only
shadows off changes 472 native pixels and 460 browser pixels by more than three
channel levels. Both comparisons darken 449 pixels with shadows enabled. CI
retains native and browser on/off captures and requires at least 100 visibly changed and
darkened pixels. Both browser projects fetch all six assets, reach ready and
report no runtime/GPU errors.

## Short CPU sample

```sh
python3 scripts/frame-benchmark.py run examples/benchmark3d \
  --profile dev --frames 12 --settle 60
```

This used debug binaries, X11/Xvfb and Mesa software Vulkan. The editor window
was 1440×1024; the standalone target was 1280×720. Both reports have 12 playing
frames and no errors. Raw reports remain under `target/bench/`.

| Playing metric | Standalone | Editor |
| --- | ---: | ---: |
| CPU work mean | 29.04 ms | 63.48 ms |
| CPU work p95 | 30.30 ms | 65.79 ms |
| CPU encoding mean | 23.08 ms | 30.66 ms |
| Fixed steps per frame | 1 | 8 |
| GPU completion wait mean | 72.47 ms | 73.35 ms |

The editor catches up with eight steps per frame in this slow software-rendered
run. The existing comparison estimates 36.59 ms of editor CPU work for one
step, versus 29.04 ms standalone, and 12.49 ms for the editor's own UI work.
The editor draws one frame during its three-second untouched rest check.

This sample proves the workload and report path are usable. It does not set
cross-machine performance targets. GPU completion wait is a CPU stall, not a
GPU timestamp measurement. This original report predates physical counters.
Timestamp timing, longer optimized samples and batching/instancing targets remain open.


## Physical 3D counters

Recorded October 14, 2026, with the same native debug/software Vulkan setup.
Standalone runs settle for 30 frames, then record three consecutive frames for
the original project and the control with only shadows disabled.
`scripts/check-3d-benchmark.py` verifies each frame, and CI retains both reports.
The existing imported-model GPU readback also verifies two model commands draw
four primitives, upload the shared asset once and keep counters across slot resets.

| Work per frame | Shadows on | Shadows off |
| --- | ---: | ---: |
| Indexed 3D draws | 502 | 301 |
| Physical 3D render passes | 503 | 301 |
| Uniform queue writes | 502 | 301 |
| Uniform payload bytes | 105,984 | 93,120 |
| Transient geometry buffer pairs | 200 | 100 |
| New imported-model asset uploads | 0 | 0 |
| New cached mesh geometry pairs | 0 | 0 |

The shadow map has one extra clear pass. The 100 inline pyramids upload once
for color and once for shadows; shared model geometry stays resident. The
counters exclude the frame clear, 2D, post-processing, initialization and driver
padding. They count recorded GPU commands and queue payloads, not GPU execution
time. The standalone three-frame runs verify counts, rather than setting
performance targets.

## Editor view counters

The editor now records the same scoped counters. Native Xvfb/software Vulkan
runs settle for 30 frames and record three editing and three playing frames,
with shadows on and off. All Game frames match the standalone table above
exactly, with no console errors. The native CI gate repeats both runs and retains
the reports. CPU timings in this short debug run are not performance targets.

A third native run moves the Scene tab into the Inspector group so that Scene
and Game both draw during Play. Its six playing frames each record 502 draws,
503 passes, 105,984 uniform bytes and 200 transient geometry pairs independently
under `mesh_counters` and `scene_mesh_counters`. No persistent uploads or console
errors occur. The Game count stays 502 rather than becoming 1,004. The summary
prints both scopes, while views that were not drawn have no counter object.
Profiler regressions cover capture, repeated-view accumulation, separation,
frame reset and Play reset; serialization distinguishes missing views from
measured zeroes. Whole-frame totals and browser GPU timestamps remain open.


## Native standalone timestamps

The standalone tool now optionally brackets each prepared frame with native GPU
queries. Both shadow controls retain the exact counters above and produce
nonzero `gpu_execution_us` values for three consecutive measured frames after
30 settling frames. Software-Vulkan CI requires those results; a baseline GPU
device test proves timing stays unavailable without the optional features.

This run uses `llvmpipe (LLVM 23.1.1, 256 bits)` with Vulkan and a 1 ns timestamp period. Timestamp interval means are 411.51 ms with shadows and 106.61 ms without. These short debug/software samples ran alongside local validation, so they
prove measurement delivery and do not set performance targets. The interval
covers frame clear, shadows and prepared draws; query resolution/readback and
presentation are outside it. Timestamp values do not enter CPU phase or
frame-interval sums. Backend ordering and precision affect their interpretation.
Browser timestamps and whole-frame counters remain open.


## Native editor timestamps

Editor benchmark launches now optionally enable the same query features while
preserving eframe's device descriptor policy. Each viewport owns its own timer.
Shadowed/unshadowed native runs settle for 30 frames and record three editing
and three playing frames. Every playing frame retains the exact physical
counters above, records a nonzero primary interval and reports no console
errors. The unused extra Scene interval stays null. Native CI requires both
shadow controls to provide timing and retains their reports.

On the same debug/llvmpipe Vulkan setup (1 ns period), primary Play interval means are 106.67 ms with shadows and 53.62 ms without.
A third run drags Scene into the Inspector group, leaving Game as the main
view. All six playing frames retain 502 draws per view and record independent,
nonzero primary/extra Scene intervals. The inspected capture shows both views.
Their mean intervals are 68.27 ms for Game and 41.21 ms for Scene.
These short software/debug samples prove delivery, without setting targets.
CPU work/interval totals exclude both execution values; CPU completion/readback
wait remains separate. GPU intervals exclude editor UI, scene-board copies,
queued uploads before the encoder bracket, readback and presentation. The
interactive Profiler panel still shows CPU phases. Browser timestamps and
whole-frame counters remain open.


## Physical 2D batch counters

`BatchDrawStats` extends the measurement path to sprite, shape and glyph
batches. The fixed 3D workload reports measured zeroes for these scopes with
shadows on and off, while keeping the existing exact 3D counters/timestamps.

Platformer's standalone and native editor Game reports both settle for 30
frames and record three consecutive frames at 1280×720. Every frame has:

| Physical batch work per frame | Standalone / editor Game |
| --- | ---: |
| Draws / render passes | 9 / 9 |
| Instances | 295 |
| Queue buffer writes | 18 |
| Uniform payload bytes | 576 |
| Instance payload bytes | 41,968 |

Native CI repeats the reports and checks those exact values. A two-view editor
run records the same values independently for Game and Scene in each of six
Play frames; the primary count remains nine. Profiler/report regressions cover
scope separation, accumulation, capture/reset and undrawn versus measured zero.
A required-GPU regression encodes empty and nonempty batches through all three
renderers over two submissions and verifies exact writes/bytes and slot reuse.

The rebuilt WebGPU player renders the 3D fixture and completes Platformer's
keyboard-to-flag goal with five coins, no falls and eight jumps. Native and
normal-editor authored-color checks pass. This is measurement foundation work
exercised by a feature example and an existing genre showcase; it does not close
the whole-frame item. Clears, post, atlas/resource uploads, scene-board copies
and egui UI remain outside these counters. Browser timestamps remain open.

## Imported-model shadow reception

The first shared world-effects slice adds directional-map reception and renderer
fog to imported models. Model color uniforms grow from 288 to 400 bytes; the
fixed workload still issues 502/301 draws and 503/301 passes with shadows on/off,
but its current uniform payload is 117,184/104,320 bytes per frame. The earlier
tables describe their measured heads, not a revised historical baseline. Model
geometry remains resident and settled model uploads remain zero.

A dedicated small control generated by `prepare-model-receiver.py` replaces the
benchmark floor with Model Collision Lab's unchanged green-ground/orange-wall
GLB and uses one blue textured cube as its caster. Same-host on/off captures
check green receiver pixels specifically, rather than accepting the cube's own
shading as evidence. Native software Vulkan at 960×540 darkens 4,238 green model
pixels (4,319 total changed/darkened). Fresh Chromium WebGPU at the same size
darkens 4,234 green model pixels (4,313 total), fetches the four required assets
and reaches the reused ready goal with no runtime/GPU errors. Both original
300-prop browser controls still pass (460 changed, 449 darkened pixels).

Local preflight passes all 117 affected render/GPU tests, including the two new
behavioral regressions. All 33 GPU-package tests execute with GPU/timestamps
required. Warning-denied Clippy/WASM, formatting/file-size gates, exact native
benchmark counters and native/editor authored-color verification pass. These
counts describe unique preflight tests; the required-GPU rerun is not added to
that total. No dependencies, scripts, schemas or generated APIs change.
This is correctness evidence, not a frame
performance target. Shared materials, imported casters and game proof remain
open; authored model fog is still forwarded only by editor viewports.


## Imported-model shadow casting

Material-aware imported casters now reuse the color geometry cache and write
independent 96-byte uniforms. The fixed workload's current shadowed frame is
602 draws, 603 passes, 602 uniform writes and 126,784 payload bytes; its
unshadowed frame remains 301/301/301 and 104,320 bytes. Transient geometry pairs
remain 200/100, with zero settled model/cached-geometry uploads. Three-frame
native controls verify every field with required GPU timestamps.

The imported-only control generated with `--model-caster` contains the green
GLB ground/orange wall and a purple GLB box, with no textured world meshes.
At 960×540, native software Vulkan darkens 10,271 green receiver pixels (10,895
changed, 10,890 darkened total). Rebuilt Chromium WebGPU darkens the same 10,271
green pixels (10,892 changed, 10,882 darkened total), fetches both original GLBs
and reaches the reused ready goal without runtime/GPU errors. Captures were
visually inspected. CI retains both native/browser on/off pairs.

Three new GPU regressions cover shared uploads/hierarchy/index widths, shadow
and color slot independence, material/texture alpha and masks, mirrored and
single/double-sided facing, and typed invalid-transform errors. Game adoption,
world-mesh cutout casting, shared materials and improved shadow quality remain
open; this is correctness evidence, not a frame performance target.

Local preflight passes 120 affected render/GPU tests; the required-GPU rerun
executes all 36 GPU-package tests, including timestamp checks. Warning-denied
Clippy/WASM, formatting/file-size gates and native/editor authored-color checks
pass. Fresh Chromium also passes both original 300-prop controls (2,110 changed,
2,088 darkened pixels) and both mixed cube/model receiver controls without
runtime/GPU errors. No dependency, script, schema or generated API changes.
