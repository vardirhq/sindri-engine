# 3D Orbit Camera Lab

A feature example for perspective FOV and engine-owned third-person orbit,
from slice 6 of the 3D update. A blue walker heads to a raised gold goal.
The gray barrier blocks the camera's requested six-unit orbit; the camera
pulls in immediately, keeps facing the walker, then eases back out when
Decay removes the barrier after two seconds.

Open the project in the editor or run:

```sh
cargo run -p sindri-player -- examples/orbit
```

Read `assets/orbit.decay`. Gameplay chooses the target and orbit angles; the
shared runtime handles following, smoothing, aiming and obstruction queries.
`Camera.perspective_fov` changes projection without altering clipping planes.
The floor is on another collision layer, and the target's collider is excluded.
Hold the right mouse button and drag to look around the walker. The script
reads `Input.Pointer.delta`, applies sensitivity in radians per motion unit
(without `dt`), and clamps pitch to stay away from vertical. Without dragging,
the camera continues its slow automatic orbit. Click the view to capture the
cursor in the native/browser player: motion then continues beyond the window
edge without holding a button. Click again, press U or Escape, or leave the
window's focus to release. The console reports capture/release and nonzero
captured motion. Capture needs a player gesture in a browser and may be denied;
`Input.Pointer.locked` reports actual state. Native captured motion uses raw
device counts; browser motion uses physical pixels. Editor Play capture and
touch look remain pending.

The project regression opens the scene, compiles all scripts and plays to its
goal while checking camera pull-in, recovery, facing and the live FOV. Native
and exported WebGPU Chromium captures show the blocked and recovered camera;
the browser run reaches the scripted goal. A real Chromium right-button drag
also reached Decay as exactly 60 pixels right and 40 down once, with the camera
response captured. This is feature evidence.
Explorer's model-based game and editor Play proof remain pending.
`scripts/browser/pointer-lock.mjs` exercises real browser capture, unbounded
motion, U/Escape release, sandbox denial and the project goal. Collision uses one sight-line ray, not a camera-volume sweep;
collider poses come from the last physics phase.

Native X11/Vulkan verification also runs the actual window to the goal, drives
raw motion beyond the viewport, releases with U/Escape/focus loss and checks
that returning focus does not recapture. Its screenshot was inspected. Other
native window platforms have not been visually verified in this slice.
