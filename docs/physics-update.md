# Incremental physics update

One pull request, delivered in checked feature slices. Each implementation slice
is committed and pushed before work proceeds to the next; smaller checkpoints
are permitted. Gameplay and demonstrations remain in Decay. Runtime, scene,
editor, scripting, proof and documentation move together.

For a new implementation session, read [the recovery handoff](physics-update-handoff.md).
CCD, one-way platforms, forces/rotation and contact snapshots are implemented and checked. The remaining items below are the active scope.

## Acceptance checklist

- [x] Reconcile outdated overlap/cast, compound and parented-body documentation.
- [x] Continuous collision detection: opt-in authored body setting, runtime and
  Decay control; fast solid-body regression and game use. Sensor sweep limitations
  must be explicit rather than promising CCD trigger events.
- [x] One-way platforms: collision from the supporting side, configurable local
  normal, safe drop-through, platformer proof including ascent and descent.
- [x] Forces and rotation: force, torque, angular velocity, angular impulse and
  off-centre impulses, with explicit accumulation/reset semantics and game use.
- [x] Contacts: entity-based world contact points, normals and force/impulse
  information, deterministic snapshots and grounded/impact proof.
- [ ] Physics materials: reusable project assets, shared validation and explicit
  literal override rules, asset loading in editor and exported games.
- [ ] Joints: scene-authored distance, hinge, slider, spring and motor controls;
  stable entity references, lifecycle/removal, editor undo, Decay access and proof.
- [ ] Character movement: reusable sweep/slide collision primitive, slopes,
  steps, ground state and moving platforms; gameplay policy remains Decay.
- [ ] Accelerated queries: synchronized spatial index, unchanged filtering and
  deterministic tie rules, immediate body moves/removals and scaling evidence.
- [ ] 3D physics and queries: fixed-step simulation, bodies/colliders/events,
  scene synchronization, editor authoring, Decay access and voxel-world proof;
  native and browser semantics exercised end to end.
- [ ] Final integration: generated catalogue, native/WASM/browser checks,
  workspace lint/tests, final diff review and green CI on the final head.

## Verification

Follow `AGENTS.md`'s pre-push gate for every code push. Record actual results in
the PR; checkmarks reflect exercised behavior, not merely an exposed type.
`docs/physics.md` is the subsystem contract and `docs/parity.md` records the
surface-specific completeness. This checklist does not mark those surfaces done.

### CCD checkpoint

- Scoped preflight: 762 native tests passed across platformer, sindri-decay,
  sindri-physics and sindri-scene; all-target/all-feature checks passed.
- Typed platformer hero preflight: zero errors and runtime reminders.
- Warning-denied Clippy passed for all four affected crates.
- WASM all-feature check passed for all four affected crates.
- Generated catalogue write/currentness/completeness checks passed.
- Fast bullet comparison, old payload omission, live toggle/kind rejection,
  spawn-window control, undo and velocity/joint preservation are exercised.
- Platformer run-to-flag and existing ground/camera regressions passed.
- Full workspace and real browser verification remain in final integration.

### One-way platform checkpoint

- Scoped preflight: 772 native tests passed across platformer, sindri-decay,
  sindri-physics and sindri-scene; all-target/all-feature checks passed.
- Typed hero preflight: zero errors and runtime reminders.
- Warning-denied native Clippy and formatting passed on pinned Rust 1.95.
- All-feature WASM checks passed on Rust 1.95 for all four affected crates.
- Generated catalogue write/currentness/completeness checks passed.
- Native regressions exercise ascent, fast CCD ascent/descent, landing, timed
  dropping to an ordinary floor, expiration/cancellation, rotated normals,
  kinematic platforms, active sensors, spawn-window timers and removal.
- Command-backed policy edits/undo preserve live velocity and joints; changing
  a static platform's support side wakes sleeping riders.
- Platformer input-driven plank ascent/landing/drop/landing and underside-sensor
  jump rejection passed alongside its existing run-to-flag/camera/clearance tests.
- Real Chromium export/load smoke passed under `/examples/platformer/`: WebGPU
  configured, project assets fetched, Decay ran and visible planks rendered.
- Full browser interaction, visual inspector and workspace verification remain
  in final integration. The next slice after this checkpoint was forces and rotation.

### Forces and rotation checkpoint

- Runtime and typed Decay force/torque, angular velocity, angular impulse and
  world-point impulses are implemented, with one-step accumulation/reset and
  ordered spawn-window controls after mass calculation.
- Platformer's wind crate proves wind, torque and input-driven launch/rotation.
- CI passed on `2426d98e` (the automatic formatting/regeneration head following
  the runtime-host compatibility and crate-collision fix), including workspace
  Clippy/tests, WASM, project preflight and real browser smoke.
- Full browser gameplay interaction and visual inspector review remain in final
  integration. Contact snapshots are the next implementation slice.

### Contact snapshot checkpoint

- Entity-based copied solid solver contacts include world anchor midpoints,
  push normals relative to the queried body, normal/friction impulses and
  total force divided by the last fixed dt. Ordered deterministically; sensor
  contacts excluded, sleeping support retained with zero new impulses/force.
- Teleports/removal/rebuilds invalidate affected contacts; invalid steps retain
  the previous snapshot. Typed Decay returns empty in the spawn window and
  filters inactive/despawned others; live hosts need no authored components.
- Platformer grounds from contacts while keeping its clearance ray; the crate
  flashes an authored child shape on hard landings, with gameplay in Decay.
- Scoped native preflight passed 355 tests and all-target/all-feature checks.
  Typed preflight passed all three changed scripts with zero errors/reminders.
  Warning-denied Clippy passed physics, scene, Decay and platformer on Rust 1.95.
- Scene and catalogue tests/currentness/completeness passed; catalogue regenerated.
  All-feature WASM checks passed for physics, scene, Decay and platformer.
- The rebuilt generic browser host loaded the platformer export in Chromium at
  `/examples/platformer/`, configured WebGPU, fetched assets, ran Decay without
  runtime errors and drew the level. Full browser gameplay interaction and editor
  Play inspection remain in final integration. New-head CI is pending.
- Next slice: reusable physics material assets.
