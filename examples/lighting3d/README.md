# 3D Lighting Lab

A feature example for Explorer's planned lighting track. It uses the existing
Benchmark's cube, pyramid and GLB assets. No in-tree game uses these controls yet.
Open this project in the editor or `sindri-player examples/lighting3d`.
Pages publishes it at `https://sindri.vardir.no/examples/lighting3d/`.

The Camera's Decay script assigns warm/blue colors, intensity, range and spot
half-angles to lights authored black, dark and disabled. During the first three
seconds it changes ambient intensity/color and the background from cool to warm,
alternates the point and spot, turns/tilts their scaled carrier and rotates
the directional sun. At the ready goal it restores the carrier and original
ambient/background, keeping the lights on. Setting its exported `lit` to false
reaches the same goal with every light disabled; the entity and script continue
running. Scene ambient fill is
restored at the goal, so final native/browser control images isolate the
script's illumination.

The goal test opens the actual project and steps its shared Session, observing
both alternating phases, extracted ambient/clear colors, composed positions and
directions, updated sun state, retained entity activity and the final enabled/
disabled controls.
Typed checking and LSP metadata use the same host surface. Native/editor and
real WebGPU captures are feature evidence, with game adoption still open.
