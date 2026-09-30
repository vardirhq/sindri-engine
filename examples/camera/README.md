# Camera Lab

An interactive feature example for the engine-owned camera behavior. Open this
directory as a Sindri project, or run `cargo run -p sindri-camera-demo --bin camera`.
The published Pages route is `/examples/camera/`.

Move with the arrow keys or hold the four on-screen direction buttons. Start
the automatic tour to take the target beyond the camera's confinement rectangle.
Manual movement stops the tour.

| Control | Key | What changes |
| --- | --- | --- |
| Follow | F | Follow the target, or hold the current view |
| Bounds | B | Confine the camera center to X [-8, 8], Y [-4, 4] |
| Shake | K | Enable or disable impact motion |
| Smooth | S | Switch smoothing between 5 and 1000; the speed cap stays 8 |
| Zone | Z | Switch the full-width/full-height dead zone between 2.5 × 1.5 and zero |
| Tour | T | Start or stop the rectangular movement route |
| Impact | Space | Add 1 trauma, with strength 0.3, frequency 30 and decay 1.6 |
| Reset | R | Restore default modes and return the target to the origin |

Reset lets the camera settle through its normal follow behavior; it does not
teleport or overwrite the engine's shake state. With a dead zone, settling need
not put the camera exactly at the target.

The cyan rectangle is parented to the rendered camera and shows the dead zone
in world units. Amber dashes and corner markers show **camera-center limits**,
not a guarantee that the viewport stays inside an arena. The outer gray frame
marks the movement area's edge. The coordinate readout samples the camera
before that fixed step's engine camera update.

Gameplay, mode selection and screen controls live in `assets/camera-demo.decay`.
Follow, dead-zone handling, smoothing, confinement and shake are performed by
`update_camera_behaviors`. Rust supplies hosting, UI hit-testing and rendering.
The exported project uses the existing shared browser runner. The packaged
font and Weave media rules keep the browser interface readable on phones.

The native tests run the authored scene and scripts, including touch input.
CI exports both project-subpath and custom-domain routes, exercises controls
in desktop and phone browsers, and captures their rendered output.
