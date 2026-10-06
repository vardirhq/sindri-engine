# Incremental physics update

One pull request, delivered in checked feature slices. Each implementation slice
is committed and pushed before work proceeds to the next; smaller checkpoints
are permitted. Gameplay and demonstrations remain in Decay. Runtime, scene,
editor, scripting, proof and documentation move together.

For a new implementation session, read [the recovery handoff](physics-update-handoff.md).
CCD, one-way platforms, forces/rotation, contacts and materials are implemented
and checked. Scene-authored distance constraints form a checked joint foundation.
Hinges, sliders, springs and velocity motors now form checked joint slices.
The remaining items below are the active scope.

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
- [x] Physics materials: reusable project assets, shared validation and explicit
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
  Play inspection remain in final integration. CI is green on `404b27ad`.
- Next slice after this checkpoint: reusable physics material assets.

### Reusable material checkpoint

- Reuse `.profile` assets with type `physics_material`; core/assets remain
  physics-independent. Scene resolution and all hosts share coefficient
  validation. Friction is finite/non-negative and restitution finite in `[0, 1]`;
  missing coefficients, unknown keys and wrong profile types fail explicitly.
- Entity material components apply to ordinary/compound and generated tilemap
  pieces. Explicit override flags win over the profile; an empty reference
  keeps literals. Coefficient edits preserve motion, forces and joints.
- Editor creation/selection uses existing profile tools and checked component
  commands. Async loading/hot reload retains the last valid edit on errors;
  profile save validates before overwriting. Play waits for initial delivery.
- Export collects scene/prefab references and validates before writing. Native
  and browser hosts use the same resolver. Native project capture now expands
  placed prefabs before scene entry, matching browser delivery.
- Platformer crate/planks share wood, with an explicit zero-bounce plank override.
  Regressions check coefficient use and a changed crate rebound, invalid/atomic
  updates, preserved motion/joints, editor reload/save and export rejection.
- Final scoped preflight passed 1,175 native tests and warning-denied all-target/
  all-feature checks for physics, scene, export, game host, editor and platformer.
  Warning-denied Clippy passed all six. Catalogue currentness/completeness passed
  11 tests; generated metadata was regenerated.
- All-feature WASM checks passed physics, scene, export, game host and platformer.
  A rebuilt Chromium export at `/examples/platformer/` fetched the material
  profile, configured WebGPU, ran scripts and drew the game without runtime
  errors. Native project delivery/play regression and Vulkan capture passed.
- Visual editor interaction review, full browser gameplay and workspace checks
  remain in final integration. CI is green on material commit `936d1a95`.
- Next slice: scene-authored joints and their typed controls/lifecycle.

### Scene-authored distance checkpoint

- Separate joint entities own maximum-distance constraints, using stable scene
  IDs resolved after all endpoint bodies. Unchanged frames keep the solver joint;
  edits replace only the constraint. Rebuilds reconnect in the same fixed step;
  inactive/missing endpoints suspend it and owner/component removal releases it.
- Existing `connect_distance` and deferred spawn-window behavior are preserved.
  Generic checked inspector edits and command undo preserve motion and ownership.
- Platformer's hanging lantern is driven by Decay wind and a scene tether;
  Decay aligns its visible cord. Its regression checks movement, bounded distance
  over 300 steps and removal, alongside the existing level-goal regressions.
- Final scoped preflight passed 1,115 native tests and warning-denied all-target/
  all-feature checks for physics, scene, editor and platformer. Typed preflight
  passed the lantern script with zero errors/reminders. Warning-denied Clippy
  passed those four crates; catalogue currentness/completeness passed 11 tests.
- All-feature WASM checks passed physics, scene and platformer. The rebuilt
  generic browser host loaded the export at `/examples/platformer/`, fetched
  assets, configured WebGPU and drew the swinging lantern without runtime errors.
  Native Vulkan capture passed and both captures were visually reviewed.
- This is a foundation, not joint acceptance completion. Hinges, sliders, springs,
  motors, typed owned-joint controls, dedicated reference diagnostics/selection
  and full prefab references remain. Runtime-spawned prefabs lack stable local
  IDs; prefab-root aliases depend on metadata absent from pre-expanded exports.
  The reference gap is recorded in parity. Visual editor interaction and full
  browser gameplay remain in final integration. Engine CI passed on `e4554125`;
  the site job was cancelled because GitHub could not allocate its hosted runner.

### Hinge and velocity-motor checkpoint

- Scene-authored hinges join body-local anchors, with optional bounded relative
  angles and force-based velocity motors capped by torque. Validate settings
  before mutation; unchanged frames keep the constraint and settings edits wake
  bodies without resetting their motion. Mixed joint kinds on one owner fail
  explicitly rather than silently replacing one another.
- Typed `Physics.set_hinge_motor(owner, velocity, max_torque)` updates the runtime
  component for the next fixed synchronization, preserving unknown fields and
  surviving endpoint rebuilds. Zero torque coasts; zero velocity with positive
  torque brakes. Invalid calls, wrong component/type and missing physics fail.
- Platformer's Decay-driven windmill reverses its axle every two seconds. Its
  regression observes both directions, a fixed axle and removal. Native runtime
  tests exercise offset anchors, bounded angles, torque caps, coasting, reversal
  and atomic rejection; command-backed scene edits/undo and rebuilds are covered.
- Final scoped preflight passed 1,421 native tests and warning-denied all-target/
  all-feature checks for physics, scene, Decay, editor and platformer. Typed
  windmill preflight passed with zero errors/reminders. Warning-denied Clippy
  passed all five crates; catalogue currentness/completeness passed 11 tests.
- All-feature WASM checks passed physics, scene, Decay and platformer. Rebuilt
  native Vulkan and Chromium WebGPU project delivery/captures passed, including
  exported scripts/assets; both captures were visually reviewed.
- Joint acceptance remains open for sliders, springs, additional motor modes,
  typed structural controls, dedicated reference authoring/diagnostics and full
  prefab references. Visual editor interaction, full browser gameplay and final
  workspace integration remain open. CI is green on hinge commit `fd9916e7`.

### Slider and spring checkpoint

- Scene-authored sliders align body-local unit axes, constrain perpendicular
  motion/relative orientation, bound signed travel and drive with a force cap.
  Springs apply radial, force-based stiffness/damping between freely rotating
  local anchors. Finite values, valid axes/limits, positive spring rest length
  and non-negative force coefficients are validated before backend mutation.
- Kind-specific backend builders and shared ownership preserve distance/hinge
  semantics. Settings edits on the same endpoints retain the constraint and
  wake bodies without resetting motion; all kinds share suspend/remove/rebuild
  handling. Scene components, reference resolution and synchronization are split
  by responsibility rather than growing a single joint module.
- Typed `Physics.set_slider_motor` and `Physics.set_spring` patch validated runtime
  component fields for next fixed synchronization, preserving unknown fields and
  supporting calls before endpoints are built. They survive endpoint rebuilds;
  missing physics, wrong component/type and invalid values fail atomically.
- Platformer's lantern trolley reverses along a bounded rail, while Decay changes
  its suspended light's rest length and draws the solved cord. Its run observes
  both directions, changed light height and independent constraint removal.
- Final scoped preflight passed 1,432 native tests and warning-denied all-target/
  all-feature checks for physics, scene, Decay, editor and platformer. Typed checks
  passed all three new scripts with zero errors/reminders. Five-crate Clippy and
  11 catalogue currentness/completeness tests passed; catalogue regenerated.
- Regressions measure rotated/distinct local axes, travel and force caps,
  coast/brake, spring extension/compression, damping and weight support. Scene
  command edits/undo, inactivity, endpoint rebuilds and existing game goals pass.
- All-feature WASM checks passed physics, scene, Decay and platformer. Rebuilt
  native Vulkan and Chromium WebGPU delivery/captures passed, fetching exported
  assets and running scripts without runtime errors. Both captures were reviewed.
- Joint acceptance remains open for complete prefab references, typed structural
  controls, dedicated reference authoring/diagnostics and remaining motor modes.
  Visual editor interaction, full browser gameplay and final workspace checks
  remain open. CI must verify the pushed slider/spring head.
