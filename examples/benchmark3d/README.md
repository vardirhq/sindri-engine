# 3D Benchmark

A feature example for measuring the current 3D renderer before batching,
instancing and material unification. It is a fixed workload, not a game:
100 textured cubes on the left, 100 inline pyramid surfaces in the middle,
and 100 references to one purple GLB on the right. A cube floor adds one
more draw object. A perspective camera frames all three groups and the
world enables a 1024-pixel directional shadow map covering 100 world units.

The Decay script aims the camera once and reports `3D benchmark ready` after
30 fixed steps. Nothing moves after startup, so changes in steady uploads
or encoding cost can be measured without changing the content. The native
project regression opens it through the real asset queue, compiles/plays the
script to ready, checks stable poses and extracts 101 cubes, 100 surfaces
and 100 models sharing one decoded resource.

```sh
cargo run -p sindri-player -- examples/benchmark3d
python3 scripts/frame-benchmark.py run examples/benchmark3d --profile dev
python3 examples/benchmark3d/assets/generate.py --check
```

`assets/generate.py` reproduces the scene and copies existing repository-owned
fixture assets. The GLB is Model Collision Lab's generated prop; its source is
`examples/model-collision/assets/models/generate-prop.py`. Textures come from
Transform Lab. There is no new external asset or dependency.

The shared native/browser player and offscreen capture/benchmark honor authored
shadow settings. Cubes, surfaces and voxel meshes cast/receive shadows; imported
models still use their separate renderer and do neither. This difference is part
of the baseline, not evidence of a unified rendering path.

Current reports contain CPU phase timings and the standalone GPU completion
wait. That wait is not GPU timestamp timing. Whole-frame counters and
performance targets remain measurement work; extracted
command counts alone do not measure GPU work. `scripts/browser/benchmark3d.mjs`
requires all six asset fetches, the ready goal and a rendered capture without
runtime/GPU errors. CI retains that capture.

CI exports a second control project with only shadows disabled and compares its
RGB pixels with the normal capture. More than 100 pixels must visibly change and
darken. CI also runs the same comparison for native captures. This proves shadow
delivery separately from the authored setting and command counts.

A short debug/software-Vulkan sample and the exact capture comparisons are in
[the baseline record](../../docs/3d-benchmark-baseline.md).

The standalone report now includes `mesh_counters`: physical textured 3D/model
draws and passes, uniform writes/bytes, transient geometry pairs and separate
model/voxel cache upload events. `scripts/frame-benchmark.py summarize` prints
their means. Counts exclude 2D, post, the frame clear and initialization uploads;
editor reports record matching primary counters and separate `scene_mesh_counters`
for the extra Scene view beside Play. Native standalone reports optionally record `gpu_execution_us` from GPU
timestamps, with device/scope metadata and null for unsupported devices. The
summary prints these intervals separately from CPU work. Whole-frame counters
and editor/browser timestamps remain open.
