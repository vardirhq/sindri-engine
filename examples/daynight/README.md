# Day/Night Lab

A feature example for Explorer and external Low Tide 3D's planned lighting
track. This is feature proof, with in-tree game adoption still open. Geometry
reuses Model Collision Lab's two self-contained GLBs: a green ground/wall and a
purple prop. No engine time-of-day service or game-specific Rust is involved.

Open `examples/daynight` in the editor, or run `sindri-player examples/daynight`.
The Camera's Decay script runs a twelve-second day, starting at 06:00. It blends
sun brightness/color, ambient fill and background through sunrise, midday,
sunset and night. The sun rotates, and the purple GLB travels and turns with a
work spotlight and blue point light attached. At night the sun is disabled and
the mounted lights take over. A ready goal reports the first completed day; the
clock, carrier and lighting continue afterward.

The Camera's exported properties configure the project:

| Property | Meaning |
| --- | --- |
| `day_seconds` | Seconds per day, clamped to at least one |
| `start_hour` | Starting hour, wrapping over 24 hours |
| `preview_hour` | A nonnegative value freezes the clock at that hour; -1 plays |
| `move_carrier` | Move/turn the imported prop and its lights |
| `night_lights` | Enable night illumination independently of entity activity |

The project test opens the actual files, compiles the script, plays two clock
configurations through all four phases and verifies the goal, continuing sun and
carrier motion, composed night-light positions and both imported render commands.
Frozen-hour controls keep every model stationary: 10:00/14:00 shadow on/off pairs
isolate the changing sun direction on imported green ground, and midnight
lights on/off isolates mounted illumination with the sun off. Native live-cycle and WebGPU frozen-hour phase captures are retained by CI; native editor Play exercises the same clock.
Fog/post controls, skyboxes, local-light shadows and in-tree game proof are not
part of this example. See [the evidence](../../docs/daynight.md).
