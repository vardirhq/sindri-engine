# Model Collision Lab

A feature example of static and dynamic convex collision from imported GLBs. One model contains
both the green ground and the orange wall; the same asset is referenced by
`sindri.model` and `sindri.physics3d.model_collider`. The shared player prepares
collision before starting gameplay after asset loading completes (asynchronously in the browser).

Run `cargo run -p sindri-player -- examples/model-collision`. Hold D until the
blue walker hits the wall, S to go around its end, D to cross, then W to reach
the gold pad. Decay owns the input, downward movement and goal; the controller
clips motion against the imported triangles. The console reports obstruction,
bypass, crossing and goal completion.

`models/generate.py` reproduces the self-contained static GLB using Python's
standard library. Its floor and wall are two triangle primitives with normals,
indices and materials, with no inline scene collision vertices. It is authored
fixture geometry rather than an art showcase.

The player's regression opens this project, compiles its script and plays the
keyboard route to its goal through Session using the same installation helper
as the native/browser host. `scripts/browser/model-collision.mjs` plays the
exported project with keyboard events, verifies the GLB fetch and all milestones,
and captures the result without changing world state.

Editor Play and native headless `ProjectRun` also install collision before
stepping. The editor/headless parity test follows the same route with matching
world state, restores the initial world and replays wall obstruction. A [native editor review](../../docs/model-collision-editor-review.md) verifies
Play goal, valid/invalid model reloads, Save refusal and Stop restoration.
A purple volumetric prop is rendered and collides from `models/prop.glb`.
It falls onto the imported floor; Decay observes landing, applies one impulse,
and reports its travel goal. `models/generate-prop.py` reproduces that GLB.
The native player and editor/headless parity regressions check this goal beside
the walker goal, and the browser smoke requires both GLB fetches and both goals.
Editor Add Convex Model Collider uses project models, permits body/primitive
compounds, and withholds incompatible owners. Reload preparation is atomic when
one GLB supplies both triangle and convex collision. Explorer game adoption
remains separate work; this example is feature proof.
