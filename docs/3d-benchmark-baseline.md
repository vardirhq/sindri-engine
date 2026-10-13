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
GPU timestamp measurement. Actual draw/pass/upload counters, timestamp timing,
longer optimized samples and batching/instancing targets remain open.
