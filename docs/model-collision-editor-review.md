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
