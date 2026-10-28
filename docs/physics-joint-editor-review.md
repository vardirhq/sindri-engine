# Native 2D joint editor review

Reviewed on 2026-10-06 at `eb994996`, using a scratch copy of
`games/platformer`, the native editor and an isolated X display. The shipped
project was not changed. This completes the native visual review for the
2D joint slice; it does not complete the remaining physics update tracks.

## Authoring and reopening

Select these entities through the hierarchy filter. Enter numbers through the
inspector, Save, close the editor and reopen the same scene. Inspect both the
saved JSON and the reopened controls.

| Owner | Saved edits | Reopened result |
| --- | --- | --- |
| Lantern tether | Maximum distance 1.5 | 1.5 with original endpoints; undo saved 2.0, redo restored 1.5 |
| Lantern spring | Damping 2.5, rest length 0.8, stiffness 25 | All three values with the trolley and light endpoints |
| Trolley slider | Position mode, motor enabled, damping 12, force cap 60, stiffness 80, target distance 0.4 | Every setting, original axes/endpoints and travel bounds -1..1 |
| Windmill hinge | Position mode, motor enabled, damping 9, torque cap 30, stiffness 50, target angle 0.3 | Nested prefab component override and every setting; source prefab unchanged |

The endpoint picker displays scene choices and marks inactive entities. Clearing
and undo were inspected in the preceding review. Typing a missing endpoint
shows the inline resolver diagnostic; typing `banner` on the spring shows
“Banner is inactive”. Restore the original endpoint before Save and Play.
Automated picker tests cover all four kinds, scoped references and command undo;
this review supplements those tests with native window interaction.

## Play and Stop

Start Play after reopening. The windmill rotates, the trolley travels and the
spring-supported light moves. The inspector becomes read-only and reflects live
Decay changes. The game deliberately initializes velocity motors in `start`,
so authored position settings are not the game's initial drive policy.

Click the scene to release text-field focus, then press P and O. The placed
windmill holds its angle and the trolley parks. Press H, J and B to rebuild the
hinge, slider and spring, then V to spawn another windmill. The placed mechanism
keeps position mode; the new mechanism rotates independently. Inspecting the
live hinge shows target 0.6, stiffness 5, damping 1 and torque cap 1; the slider
shows target 0.5, stiffness 20, damping 4 and force cap 4. Those are the game's
Decay settings, distinct from the document values above.

Save during Play displays “Not saved. Stop the scene first: a running scene is
not the document”. A SHA-256 comparison confirms the scene file is unchanged.
Stop removes the spawned mechanism and restores the authored scene. Inspecting
the hinge restores target 0.3, stiffness 50, damping 9 and torque cap 30. Saving
after Stop produces the same file hash as before Play.

## Evidence and limits

The reviewed native windows rendered the level, reopened controls, live modes,
rebuild/spawn behavior, Save refusal and restored edit state. Captures were
inspected locally; this document records the repeatable procedure and outcomes.
Existing engine/editor/Decay/platformer regressions provide numerical constraint,
lifecycle, reference isolation and atomic-validation evidence. Browser joint
captures and interaction regressions were checked in the preceding feature
slices, and CI is green on `eb994996`.

This acceptance covers authored 2D distance, hinge, slider and spring joints,
velocity/position motor controls, references, lifecycle, undo and Decay game
proof. 3D physics/joints and script-triggered world snapshots remain separate
absent capabilities. Full workspace integration remains a later acceptance gate.
