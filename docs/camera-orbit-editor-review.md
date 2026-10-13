# Orbit camera editor review

Reviewed on 2026-10-13 using the freshly built native editor on isolated X11
with Mesa software Vulkan. `target/tmp/orbit-authoring-editor/` copies Orbit
Camera Lab, with the CameraRig script component removed from Camera. Walker
and Barrier retain their original Decay scripts; camera movement comes solely
from the authored orbit component and shared runtime.

Selected Camera and added Camera Orbit through Add Component. The enabled
menu action creates target `walker`, yaw 0, pitch 0.35 radians (20.05 degrees
in the angle control), distance 6, smoothing 8 and collision padding 0.2.
The inspector exposes target, offset, angles, distance, smoothing and mask/padding.
Saving writes the real target ID, with no placeholder reference.

Entering distance zero reports the settings refusal and leaves the saved value
at 6. A valid edit to 7 is accepted, Undo/Redo works, and Save/reopening the
native editor retains distance 7 and target `walker`. Unit tests separately
exercise missing Camera/target, self/child/inactive/UI/unidentified candidate
exclusion, invalid pitch/smoothing/padding/target and checked persistence.

Play pulls the camera toward the walker while the barrier obstructs its sight
line. After Barrier's original script disables the wall, the camera recovers to
the authored orbit, follows the walker to the gold goal and reports `Arrived`.
Stop restores the authored camera pose [0, 3, 6], six authored entities and the
saved orbit settings. No camera script sets or overrides those settings.

Inspected captures: `target/orbit-authoring-before.png`, `orbit-authoring-menu.png`,
`orbit-authoring-added.png`, `orbit-authoring-invalid.png`,
`orbit-authoring-play.png`, `orbit-authoring-goal.png`,
`orbit-authoring-stopped.png` and `orbit-authoring-reopened.png`.
This is feature proof for Explorer. The in-tree game and broader GPU/native
platform coverage remain open. Selected-guide evidence follows below.
Runtime/browser orbit behavior was already exercised by the shipped lab; this slice changes native
editor authoring and introduces no new browser or Decay API.

## Selected orbit guides

Rebuilt the native editor and reopened the same scratch project with distance
7. Selecting Camera in Scene view shows the amber world-space focus, requested
position, sight line and yaw circle. Selecting Floor hides those guides. Zooming
in until the actual camera marker is behind the Scene eye leaves the focus and
clipped guide line visible, without mirrored markers or a spray across the view.

Play/Pause shows the current camera pulled in near the walker while the
requested marker stays on the seven-unit orbit. Resuming disables the barrier,
recovers the camera to its requested position and reaches `Walker: Arrived`.
The blue current-pose marker disappears once the two positions are within
0.05 units. Stop restores position [0, 3, 6] and the authored guides. Editing yaw
moves the requested marker along its circle; Undo restores the earlier setting
and guide. No edited scene was saved in this guide review.

The first inspection exposed overlapping requested/current labels. The final
build puts the current label below its marker and backs labels with dark plates;
both are readable in the inspected selected and paused views.

Final inspected captures: `target/orbit-guides-framed.png`,
`orbit-guides-unselected.png`, `orbit-guides-pull-in.png`,
`orbit-guides-recovered.png`, `orbit-guides-stopped.png`,
`orbit-guides-yaw.png`, `orbit-guides-undone.png` and
`orbit-guides-near-plane.png`. Unit tests cover world-axis angles/offsets on
rotated/scaled parents, the yaw circle's requested distance/height, current and
target movement, invalid targets/settings, overflow and near-plane clipping.
The guides are read-only settings/pose evidence, without a second camera solver
or an obstruction-volume preview. Explorer proof remains open.
