# 3D Lighting Lab

A feature example for Explorer's planned lighting track. It uses the existing
Benchmark's cube, pyramid and GLB assets. No in-tree game uses these controls yet.
Open this project in the editor or `sindri-player examples/lighting3d`.
Pages publishes it at `https://sindri.vardir.no/examples/lighting3d/`.

Two orange panels face away from the camera. `Double-sided panel` shows its back
with the same lighting as other geometry; `Single-sided panel` is hidden. Select
either panel's Material and change `sidedness` between `inherit`, `single` and
`double` to compare visibility, then Undo/Redo. The single-sided capture control
changes only the visible panel's policy. This is also feature proof for the
Explorer material track; shared material assets and alpha modes remain open.

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

Three `materials/*.profile` assets now provide shared factors and sidedness for
six objects. `polished.profile` drives a cube and a GLB; changing its emission
updates both. The two back-facing panels share `orange.profile`, with an explicit
local Single override on the hidden panel. Use **New render material** in the
project browser to create another; select it in an entity's Material profile
picker. The structured profile inspector validates Save, and the scene picker
uses normal Undo/Redo. Texture ownership remains with each mesh/model.

The native/WebGPU controls include `prepare-lighting3d.py --inline-materials`
(equal pixels with inline values) and `--green-materials` (one shared profile
edit changes two visible surfaces). Neither changes gameplay scripts or the goal.
