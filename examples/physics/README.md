# Physics Playground

A feature example, authored entirely in Decay and scene components. Open
`sindri.toml` in the editor or export it through `sindri-export`.

The cyan ray shows the closest collider hit, the yellow dot its point and the
green line its normal. A blue trigger flashes when falling bodies enter it.
Orange bodies fall and bounce using the scene's gravity and restitution.

The query button (C) switches what the cyan line asks:

- Ray: `Physics.raycast`, the closest piece the line meets.
- Circle cast: `Physics.cast_circle`, a circle of radius 0.35 swept along the
  same line. The ring shows where it stops, which is sooner than the ray, and
  a gap the ray slips through can stop it.
- Area: `Physics.overlap_circle`, every object inside a ring of radius 1 at
  the line's far end, each named once.

The mask, sensor and length controls apply to all three.

Use the screen buttons on desktop or touch:

- Sensors (S): include trigger pieces in the ray.
- Mask (T): all / solid layer 1 / falling bodies layer 2 / none.
- Origin (I): move inside the solid circle to inspect distance 0, normal (0, 0).
- Length (Q/E): change the ray's inclusive maximum distance.
- Left/right: turn the ray by 15 degrees; arrow keys turn continuously.
- Drop (B): reset both falling bodies. Reset (R) also resets the query and counts.

Queries read synchronized physics geometry. A teleport becomes visible at the
next fixed step.

`crates/sindri-decay/tests/physics_demo.rs` opens the project, checks its scripts,
plays the controls and observes rays, bounce and sensor events. Browser smoke
tests use the exported project on desktop and phone under both Pages base paths.
