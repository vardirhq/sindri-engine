# 3D Transform Lab

A feature example of world-space aiming, direction reads and orbiting, from
slice 6 of the 3D update. A cube faces a raised goal and walks to it while a
perspective camera orbits the scene. The cube starts under a scaled parent:
its script uses world coordinates so the parent does not change its speed.

Open this project in the editor, or run:

```sh
cargo run -p sindri-player -- examples/transform
```

Read `assets/transform.decay`: `look_at` faces the goal, `forward` supplies the
unit movement direction, and `rotate_around` turns the camera's position and
orientation together. Directions include parent rotation and ignore scale;
they are read-only, but a vector copied from one can be changed normally.

The shared-runtime test opens the project, validates its scene, compiles both
scripts and steps until the walker reaches the goal. This demonstrates the
transform API; the Explorer genre showcase will supply the gameplay proof
with models, collision and a character controller.
