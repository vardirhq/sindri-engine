# Model Collision Lab

A feature example of static collision from an imported GLB. One model contains
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

This proves the shared player binding path. Editor Play collision delivery,
headless `ProjectRun` automatic model loading, dynamic convex model components
and Explorer game adoption remain separate work; this example is feature proof.
