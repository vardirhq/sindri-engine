# 3D Character Lab

A feature example for scene-owned 3D character movement, added for the planned
Explorer showcase. Gravity, jumping, movement speed, ferry reversal and the goal
are Decay gameplay. The shared engine clips displacement and supplies classified
support and solved platform carry; it contains no lab-specific gameplay.
Pages publishes it at `https://sindri.vardir.no/examples/character3d/`.

Run `cargo run -p sindri-player -- examples/character3d`, or open `sindri.toml`
in the editor and enter Play. The blue cube starts aboard a gold ferry. Ride it,
press Space to jump off, then hold D to reach the thin gold goal pad. A/D move
along X and W/S along Z. Landing on the gray floor after riding and jumping is
required before reaching the goal. Console messages report each milestone.
Tune the Walker's exported `speed`, `gravity` and `jump_speed` in the scene.

The request made during a script update is applied after the next physics solve.
Jump permission comes from classified `motion.grounded`, rather than the raw
Rapier contact flag. The jump step includes the last pre-solve support carry;
the following airborne step has no carry. Snap is enabled only while grounded
and descending. Collision normals cancel upward intent at a ceiling.

The native Session tests open this project, compile its scripts, verify idle
carry through reversal, and use keyboard input to jump, land and reach the goal:

```sh
cargo test -p sindri-runtime --all-features --test a_3d_character_project_plays
```

Export with `sindri-export` and install the generic player's WebAssembly package
in the export's `pkg/`, as in CI. The real Chromium check uses ordinary keys,
checks gameplay milestones and GPU/runtime errors, and captures the rendered goal:

```sh
SINDRI_BASE_PATH=/examples/character3d/ node scripts/browser/character3d.mjs \
  target/dist/character3d target/character3d.png
```

This is native Session, browser and editor feature proof. The
[native editor review](../../docs/physics-character3d-editor-review.md) exercises
selected footing guides, settings/undo/save/reopen and keyboard Play to the goal.
The Explorer game's goal remains a separate acceptance item. Imported models,
slopes, steps and terrain residency are not demonstrated by this small project.
