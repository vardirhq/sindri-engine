# Physics Playground

A feature example, authored entirely in Decay and scene components. Open
`sindri.toml` in the editor or export it through `sindri-export`.

The cyan ray shows the closest collider hit, the yellow dot its point and the
green line its normal. A blue trigger flashes when falling bodies enter it.
Orange bodies fall and bounce using the scene's gravity and restitution.

Use the screen buttons on desktop or touch:

- Sensors (S): include trigger pieces in the ray.
- Mask (T): all / solid layer 1 / falling bodies layer 2 / none.
- Origin (I): move inside the solid circle to inspect distance 0, normal (0, 0).
- Length (Q/E): change the ray's inclusive maximum distance.
- Left/right: turn the ray by 15 degrees; arrow keys turn continuously.
- Drop (B): reset both falling bodies. Reset (R) also resets the query and counts.

Queries read synchronized physics geometry. A teleport becomes visible at the
next fixed step. There is no overlap or shape-cast API in this slice.

`crates/sindri-decay/tests/physics_demo.rs` opens the project, checks its scripts,
plays the controls and observes rays, bounce and sensor events. Browser smoke
tests use the exported project on desktop and phone under both Pages base paths.
