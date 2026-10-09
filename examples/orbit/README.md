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

The project regression opens the scene, compiles all scripts and plays to its
goal while checking camera pull-in, recovery, facing and the live FOV. This is
feature evidence. Explorer's model-based game, mouse look and browser proof
remain pending. Collision uses one sight-line ray, not a camera-volume sweep;
collider poses come from the last physics phase.
