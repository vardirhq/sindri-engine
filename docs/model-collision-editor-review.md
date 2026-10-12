# Native model collision editor review

Reviewed on 2026-10-12 with the collision-loading slice, a freshly built editor
and a scratch copy of `examples/model-collision` on an isolated Linux X11
screen using Mesa software Vulkan. This is feature-example evidence; Explorer
adoption and broader native GPU/platform coverage remain open.

## Play and Stop

Open `assets/level.scene`, press Play, choose Game and focus the viewport.
Hold D into the imported wall, then S around its end, D across and W to the
pad. The console reports `Model wall blocked`, `Model wall bypass ready`,
`Model across wall` and `Model goal reached`. The blue walker renders on the
gold pad; one loaded GLB supplies both the green floor and orange wall.

Save during Play is refused as a running scene is not the document. Stop
returns Walker to its authored position `[-2, 0.31, 0]`, removes the run's
scene container and restores the selected character guides. The scene bytes
stay identical to the shipped example through Play, Save refusal and Stop;
SHA-256: `70bfab86921ddd3c059332a511e7c6bf160328cf0de6e7f7db620b9738c332d0`.

## Model reload

In the scratch GLB, move the wall from x=0 to x=1, preserving the floor and
updating its position bounds. The console reports the model reloaded; the
rendered wall moves and the walker stops at the moved collision surface.

Then write a GLB that still decodes, but whose indices form degenerate
triangles. The console identifies `models/level.glb`, node/mesh/primitive and
`degenerate triangle`. The previous rendered wall and collision remain:
holding a movement key still stops the walker at that surface. Restore the
original model after the review. Stop restores world edits, not changes to
external asset files.

## Evidence and limits

Inspected captures and the scratch project are retained locally under
`target/tmp/model-collision-editor/`, including `blocked.png`, `goal.png`,
`save-refused.png`, `stopped.png`, `reloaded-valid.png` and
`invalid-reload-valid.png`. The shipped example was unchanged.

Automated editor/headless parity checks also run the full route with matching
world state at every step, restore the initial world and replay obstruction.
Startup regressions cover unsaved inactive references, absent/unready queues
and named decode failures; headless tests cover both asset layouts, later
scenes and nested prefab overrides. Asset picker authoring, references added
during a running scene, dynamic model hull components and Explorer remain
separate work.
