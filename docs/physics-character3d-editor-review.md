# Native Character 3D editor review

Reviewed on 2026-10-11 from `f63a8b0a` plus the 3D footing-gizmo slice,
with a freshly built editor and a scratch copy of `examples/character3d`
on an isolated X11 display using Vulkan. This is feature-example proof for
the planned Explorer capability. Explorer adoption remains open.

## Authoring and guides

Open the project's `assets/character.scene`, select Walker in the hierarchy,
then choose Scene. Green lines show its scaled collider; orange lines show
four slope rays and their ring, a foot marker and the snap ring. The default
zero step height has no step ring. Unselected characters use the existing
selection filter rather than filling the scene with footing guides.

Double-click a numeric inspector value, type it and press Enter. Click the
Scene tab to release text-field focus before undo/redo or Save. Allow the
command to finish before inspecting saved JSON.

| Interaction | Observed result |
| --- | --- |
| Snap distance 0.15 → 0.30 | Saved 0.3; orange snap ring moves downward |
| Undo and Save | Saved 0.15 |
| Redo and Save | Saved 0.3 |
| Step height 0 → 0.25 | Saved 0.25; an orange step ring appears above the foot |
| Skin -0.01 attempt | Rejected with an invalid-component console diagnostic; default 0.01 remains |
| Close and reopen | Snap 0.30 and step 0.25 remain in the inspector and guides |

The footprint comes from the scaled solid probe, with world-space up and
step/snap distances. These guides do not predict collision results. Automated
scene tests cover parent rotation, offsets, alternate up, round/box extents,
sensors, unsupported geometry, zero reaches and disabled controllers.

## Play and Stop

Play the reopened scene, select Game and focus its viewport. Ride, hold D and
press Space. The actual Decay script reports carry, jump, floor landing and
goal completion; the blue controller renders on the floor and the reversing
gold ferry continues moving. Its live inspector is read-only.

Save during Play reports that a running scene is not the document. The saved
scene's SHA-256 remains unchanged. Stop restores Walker's authored position
[0, 1.31, 0], editable settings and selected footing guides. Save after Stop
preserves the same pre-Play file hash:
`ba92e6d776c324c902ecc9dbf0490a3d60c3f90f968b74065aba3951d239d290`.

## Evidence and limits

Captures for selected/edited guides, reopened settings, Play goal, Save refusal
and restored edit state were inspected. Local artifacts and logs are in
`target/tmp/character3d-editor-*`; the scratch project is
`target/tmp/character3d-editor-project`. The shipped project was unchanged.

This review covers Linux X11/Vulkan editor interaction and a box controller.
It does not prove other native window platforms, imported-model collision,
Explorer gameplay or the full Milestone 8 acceptance gate.
