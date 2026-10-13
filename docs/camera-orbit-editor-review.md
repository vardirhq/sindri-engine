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
This is feature proof for Explorer. Dedicated orbit guides, the in-tree game
and broader GPU/native-platform coverage remain open. Runtime/browser orbit
behavior was already exercised by the shipped lab; this slice changes native
editor authoring and introduces no new browser or Decay API.
