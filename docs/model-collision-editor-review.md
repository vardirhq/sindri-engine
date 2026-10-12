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
scene container and restores the selected character guides. At this static-loading review, the scene bytes
stayed identical to that version of the shipped example through Play, Save refusal and Stop;
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
scenes and nested prefab overrides. The authoring and convex slices below extend this review. References added
during a running scene still require restarting Play; Explorer remains separate
work.

## Model asset authoring

The 2026-10-12 model-picker slice was reviewed on the same isolated
X11/software Vulkan setup with a new scratch project under
`target/tmp/model-picker-editor/`. It adds `models/alternate.glb`, with the wall
at x=0.5 and matching bounds, plus unsupported `.obj` and `.gltf` files.

Both Model and Physics3d Model Collider offer only the two GLBs. Select the
alternate render asset, then the alternate collider asset: each selection is
independent. Undoing the second restores only the collider reference; Redo
restores it. Saved scene JSON confirms the two fields independently. Generic
field widgets are scoped by component, and component edit history scopes keep
the two pickers separate while continuous drags within a component still merge.

Add Component → Rendering → Model on Goal starts with the actual
`models/alternate.glb` asset rather than a placeholder. Save persists it and one
Undo removes it. Restore Goal before playing the route.

Close and reopen the saved project. Both Level references still name the
alternate GLB. Play loads that asset, the walker stops at the shifted wall
(x≈0.19), then keyboard input takes it around the wall to the gold goal. All four
milestones appear in the console. Stop restores Walker to `[-2, 0.31, 0]` and
keeps the authored model choices; it does not change the saved scene bytes.

Inspected captures include `picker-fixed.png`, `both-picked.png`,
`model-added.png`, `reopened.png`, `alternate-blocked.png`,
`alternate-goal.png` and `alternate-stopped.png`. The shipped example remains
unchanged. Regression tests cover GLB-only choices in both asset layouts,
uppercase native loading with exact IDs, real project-backed defaults,
world/HUD separation, competing physics ownership, independent asset Undo/Redo
and continued drag merging. Explorer adoption remains pending.

The same authored scratch project was then opened in the actual native
`sindri-player <project>` window at 960 × 540. Inspected `native-player.png`,
`native-player-blocked.png` and `native-player-goal.png` show the imported wall,
keyboard obstruction and the walker on the gold pad. This closes the native
player-window visual check for imported models on X11/software Vulkan; it does
not broaden the GPU/platform coverage or replace Explorer game proof.


## Convex props and authoring

The convex-loading slice was reviewed on 2026-10-12 with a freshly built editor
and `target/tmp/convex-model-editor/` on isolated X11/software Vulkan.
Prop references `models/prop.glb` for rendering and convex collision, with a
separate dynamic body. Play loads both lab GLBs before gameplay. The purple prop
falls onto the imported floor, Decay observes the contact-start event and applies
one impulse, and the console reports `Model prop landed and pushed` followed by
`Model prop goal reached`. The inspector settles near x=-2.088, y=0.500, z=-2.

Replace the scratch prop GLB with a valid 75% size version, including its bounds.
The renderer shrinks and the solved body settles at y=0.375. Replace it with the
planar decoded-model fixture: the console names `models/prop.glb` and the
volumetric-hull error, while the prior small purple model and y=0.375 collision
remain. Restore the shipped prop. Stop restores its authored [-3, 3, -2] pose.
This checks a convex reload in the real editor; a separate Session regression
checks atomic retention when the same asset also supplies triangle collision.

Add Component → Physics → Convex Model Collider 3D on Goal supplies the real
`models/level.glb` asset and default material/layers/sensor fields. Save persists
it; Undo removes it and Save persists removal. Restore Goal before Play.
The convex asset picker offers the project's two GLBs. Choose level for convex
collision while Model continues to name prop; Undo/Redo changes the convex field
independently. Undo to the original prop reference and save. Reopening shows both
references and the authored prop pose intact, with both GLBs loaded. Native guard
regressions cover allowed body/primitive compounds and incompatible owners in
both orders, including 2D tilemap collision.

Inspected captures include `target/tmp/convex-editor-play-prop-goal.png`,
`convex-editor-reload-valid.png`, `convex-editor-reload-invalid.png`,
`convex-editor-stop-restored.png`, `convex-editor-component-added.png`,
`convex-editor-reopened.png`, `convex-editor-picked-final.png` and
`convex-editor-redo-final.png`. The rebuilt browser smoke separately fetches both
GLBs and reaches both the walker and prop goals; the editor/headless parity
regression compares every step and restores/replays the imported wall.
This is feature proof; Explorer and broader GPU/platform verification remain open.


## Convex prefab spawn-window motion

The spawn-window slice was reviewed with the freshly built native editor on
2026-10-12 using `target/tmp/convex-spawn-editor/` on isolated X11/software Vulkan.
The authored scene now has six entities; CameraRig's typed prefab export names
`prefabs/prop.prefab`. The prefab contains the convex-only dynamic GLB body and
its Prop script. Play preloads that prefab and its model, spawns the purple prop,
and reports `Model spawn controls queued` before it has a solver body. Native
Session regressions separately inspect this boundary and its first materialized
velocity/mass and checkpoint replay.

The native console then reports `Model prop landed and pushed` and
`Model prop goal reached`, and the purple GLB is visible on the imported floor.
Stop removes the spawned prop and the run's scene container, returning to six
authored entities. Restart queues the commands once for a fresh run and reaches
the goal again. Captures `target/tmp/convex-spawn-editor-play.png`,
`convex-spawn-editor-goal.png` and `convex-spawn-editor-stopped.png` were inspected.
The rebuilt Chromium smoke independently requires pending-read verification,
both GLB fetches, the prop goal and the keyboard walker goal with no runtime/GPU
errors. Explorer adoption and broader platform/GPU coverage remain open.
