# Native Character 2D editor review

Reviewed on 2026-10-07 at `da9a5328`, with a freshly built native editor,
a scratch copy of `games/platformer` and an isolated X display. The shipped
scene and scripts were not changed. This establishes the editor surface;
browser goal proof and final physics integration remain open.

## Authoring, undo and reopening

Filter the hierarchy for Hero and scroll its inspector to Physics2d Character.
Edit numeric fields by double-clicking their values, typing and pressing Enter.
Click the Scene tab to release text-field focus before keyboard undo/redo.
Save after each completed command and inspect the scratch scene JSON.

| Field | Observed edit and result |
| --- | --- |
| Snap distance | 0.2 → 0.35; undo saved 0.2, redo restored 0.35 |
| Max iterations | 8 → 6 |
| Max slope angle | 0.7853982 → 0.6 radians |
| Skin | 0.01 → 0.02; attempting -0.01 retained 0.02 |
| Step height | 0.3 → 0.4 |
| Carry platforms | Checked → unchecked, saved false; re-enabled before Play |
| Up | [0, 1] → [0, -1], saved; undo restored [0, 1] |

Close and reopen the scratch project. The controls and saved JSON retain the
edited numeric settings, carry enabled and upward support. The negative skin
attempt did not enter the document; no inline diagnostic was observed, so this
review does not claim one.

Remove the component with its inspector trash button and Save: the payload is
absent. Undo restores it. Remove it again, then choose Add Component → Physics →
Character 2D. The saved payload contains registered defaults: skin 0.01,
eight iterations, slope pi/4, up [0, 1], zero snap/step distances and carry enabled.
Undo the addition and then the removal to restore all custom settings. The saved
file's SHA-256 matches its value before this remove/add sequence.

## Play and Stop

Play the reopened scene. The hero settles on the floor and the inspector becomes
read-only with the authored controller values. Select Game, focus the viewport,
hold Right and press Space: the hero moves and jumps through its actual Decay
script. The level and animated mechanisms render in both Scene and Game views.

Save during Play reports “Not saved. Stop the scene first: a running scene is
not the document”. The scene file's SHA-256 is unchanged. Stop restores the hero's
authored pose and editable settings; Save produces the same pre-Play file hash.

## Evidence and limits

Native captures were inspected for reopened fields, live read-only controls,
running/jumping, Save refusal and restored edit state. The repeatable procedure
above records the outcome without depending on local screenshot coordinates.
Artifacts are `/tmp/sindri-joint-review-character-*.png`, editor build/runtime
logs `/tmp/sindri-character-editor*.log` and scratch project
`/tmp/sindri-character-editor-project`. Existing checked command/schema tests
and engine/Decay/platformer controller regressions supply numerical validation,
movement and lifecycle evidence; this review supplies actual window interaction.

No engine, editor implementation, host API or game asset changed in this review.
It does not claim a browser run to the flag, rotation/arc carry game encounters
or final workspace/head CI acceptance.
