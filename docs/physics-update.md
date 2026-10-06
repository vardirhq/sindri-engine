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
  remain open. CI is green on slider/spring commit `338db9fc`.

### Spawned-prefab reference checkpoint

- Core retains runtime-only `PrefabIdentity` separately from saved scene IDs and
  editor instance links. `World::prefab_entity` resolves enclosing local paths and
  the original top-level root within one spawn, without crossing instances or
  falling through to a scene entity. Root removal invalidates the scope even
  after slot reuse; reparenting, cloning, undo and assigning saved IDs retain it.
- All four authored joint kinds use this resolver. Repeated spawns, inactive
  endpoints, reactivation and removal are exercised. Expanded nested sibling
  paths and command undo are covered; editor duplication clears runtime scope.
- Platformer Decay places/removes a reusable windmill with V. Its regression
  runs two placement cycles, observes motor reversal and a fixed axle, checks
  constraint cleanup and retains the original level-goal regressions.
- Final scoped preflight passed 1,385 native tests and warning-denied all-target/
  all-feature checks for core, scene, editor and platformer. Typed setup-script
  preflight passed with zero errors/reminders; four-crate Clippy passed. Catalogue
  currentness/completeness passed 11 tests; all 1,082 Rust files pass the size gate.
- All-target/all-feature WASM checks passed core, scene and platformer. The generic
  browser host was rebuilt; Chromium WebGPU loaded the export at
  `/examples/platformer/`, fetched 24 assets and exercised placement, removal and
  replacement without runtime errors. Native Vulkan capture passed; both native
  and spawned-browser captures were visually reviewed.
- Nested renamed-root aliases, exported placed-root aliases and saved-spawn
  reference remapping remain incomplete. Typed structural controls, dedicated
  reference authoring/diagnostics, other motor modes and final editor/browser/
  workspace integration remain open. Joint acceptance stays unchecked. CI is
  green on spawned-prefab commit `acbed858`.

### Joint suspension and distance tuning checkpoint

- All four authored joint components accept `enabled`, defaulting true for old
  payloads. False releases only the constraint at next fixed synchronization;
  true reconnects available active endpoints with retained settings. Endpoint
  bodies and their motion are retained. Ordinary collision/constraint solving
  may change motion after release/reconnection.
- Typed `Physics.joint_enabled` reads the authored flag;
  `Physics.set_joint_enabled` patches it, and `Physics.set_distance` tunes owned
  maximum-distance constraints. Calls support initial synchronization and
  suspended owners, preserve unknown fields and reject missing/conflicting/
  invalid owners, wrong types/kinds, missing physics and invalid lengths before
  mutation. Active owners' settings, including suspended distances with missing
  endpoints, are validated before ownership changes.
- Scene command suspension, undo/redo and endpoint rebuilds cover all four kinds.
  Platformer Decay reels the lantern tether with T and releases/reconnects it
  with L. Its regression observes short/long lengths, free fall, hidden cord and
  reconnection; owner-removal and existing level-goal regressions remain covered.
- Physics signature construction and reference prose are split by responsibility
  to keep growth within the function/file limits. Catalogue regenerated.
- Final scoped preflight passed 768 native tests and warning-denied all-target/
  all-feature checks for scene, Decay and platformer. Typed lantern preflight
  passed with zero errors/reminders. Three-crate Clippy and 11 catalogue
  currentness/completeness tests passed; all 1,086 Rust files pass the size gate.
- All-target/all-feature WASM checks passed those three crates, and the generic
  browser host was rebuilt. Chromium WebGPU fetched 24 assets and exercised T/L
  shortening, release, reconnection and restored length, plus the spawned windmill,
  without runtime errors. Native Vulkan capture passed. Native and browser
  short/released/reconnected captures were visually reviewed.
- Joint acceptance stays open for endpoint retargeting, structural creation/
  removal, additional motor modes, dedicated reference authoring/diagnostics and
  remaining prefab reference integration. Final editor/browser/workspace checks
  remain open. CI is green on joint-control commit `195106ff`.

### Nested runtime root reference checkpoint

- Expansion retains namespaced original root aliases as runtime metadata. Core
  spawn validates aliases before mutation, resolves canonical paths before aliases
  and rejects competing aliases. Component strings and saved scene IDs are unchanged.
- Decay spawning retains the original prefab library instead of discarding metadata
  through a pre-expanded document. Spawn limits still count the complete expansion.
- All four joint kinds exercise renamed nested roots across repeated spawns,
  inactivity, reactivation and removal. Core tests cover local resolution, cloning,
  canonical precedence and atomic ambiguity rejection.
- Platformer nests its reusable powered windmill in `windmill-kit.prefab`; its
  scripted placement/removal regression proves the general reference capability
  through an existing genre showcase.
- Final scoped preflight passed 1,082 native tests and warning-denied all-target/
  all-feature checks for core, scene, Decay and platformer. Four-crate Clippy,
  11 catalogue currentness/completeness tests and the 1,086-file size gate passed.
  Typed windmill/setup checks passed with zero errors/reminders.
- All-target/all-feature WASM checks passed those four crates; the browser host
  was rebuilt. Chromium WebGPU fetched 25 assets and exercised nested placement,
  removal and replacement without runtime errors. Native Vulkan capture passed;
  native and spawned-browser captures were visually reviewed.
- CI was running on nested-runtime commit `4049494a`. Pre-expanded placed-root references,
  saved-spawn remapping and remaining joint/editor/workspace acceptance stay open.


### Placed-prefab root reference checkpoint

- Original top-level and nested root aliases are retained as runtime link metadata.
  Library-aware `LoadedScenes` entry keeps original scenes and prefabs through
  native/browser delivery and scene switches. Scene namespaces prefix aliases;
  canonical paths win and conflicting aliases fail during expansion.
- Editor link commands carry aliases through reload/undo; duplication rebases
  them to the copy's namespace. Serialized links/scenes omit aliases, and saving
  prefab references/reopening regenerates them from the original library.
- All four joint kinds exercise placed nested roots, repeated instances, scene
  namespaces/switching, inactivity and removal. Canonical inactive matches block
  alias fallback. Editor reload/undo/redo and independent duplicate cleanup are
  covered. Platformer places its level windmill from the same nested kit Decay spawns.
- Final scoped preflight passed 1,424 native tests and warning-denied all-target/
  all-feature checks for core, scene, editor, the game host and platformer.
  Five-crate Clippy, 11 catalogue tests and the 1,089-file size gate passed.
  Typed windmill/setup checks had zero errors/reminders.
- WASM all-target/all-feature checks passed core, scene and platformer; the game
  host library passed all-feature WASM checks and its browser runtime was rebuilt.
  Chromium WebGPU fetched 25 assets and exercised placed/spawned windmills plus
  removal/replacement without runtime errors. Native Vulkan capture passed; both
  native and browser captures were visually reviewed.
- CI must verify this pushed slice; the prior nested-runtime head `4049494a` still
  had its test/capture job running when this slice was checked. Saved-spawn
  remapping, typed controls/motor modes, reference authoring and final integration
  remain open. Plain flattened scene documents discard expansion metadata.


### Saved-spawn reference checkpoint

- Added opt-in `World::to_scene_with_references`: registered `FieldMeaning::Entity`
  fields, including dotted/list paths, become their local target's assigned scene
  ID. Empty references stay unbound; malformed/unresolved/unstable targets fail.
  Unknown payloads stay untouched, live state is unchanged and no handles serialize.
- All four joint kinds reopen with isolated endpoints and retain settings/unknown
  fields, inactivity/reactivation and removal. Qualified scene IDs win over relative
  namespace matches within their containing scene, including namespaced reopening.
  Core tests cover generic list fields and atomic rejection.
- Platformer saves/reopens a Decay-spawned nested windmill and observes motor
  reversal, its fixed world-space axle and five independently owned constraints.
- Final scoped preflight passed 1,398 native tests and warning-denied all-target/
  all-feature checks for core, scene, editor and platformer. Four-crate Clippy,
  11 catalogue tests and the 1,093-file size gate passed.
- WASM all-target/all-feature checks passed core, scene and platformer; the generic
  browser host was rebuilt. Export and native Vulkan capture passed. Chromium
  WebGPU fetched 25 assets from the saved/reopened fixture without runtime errors;
  native and browser captures were visually reviewed.
- CI must verify this slice; all checks passed on prior head `41073354`. Existing
  serializers remain verbatim; automatic editor/script save-path integration,
  other joint controls and final acceptance remain open.


### Entity-reference authoring checkpoint

- Shared `World::resolve_entity_reference` with physics and native authoring;
  resolution retains inactive targets for diagnostics while physics requires
  active endpoints. Canonical IDs, prefab aliases and loaded-scene isolation keep
  their existing precedence.
- Registered entity fields get typeable scoped choices, explicit None, and
  visible missing/inactive diagnostics at object/list depth. Runtime prefabs show
  their local paths even after assigning saved IDs; unrelated instances/scenes
  never enter the choices.
- Real picker-click tests retarget and clear all four joint kinds, applying
  checked commands and undo while preserving unknown fields and solver ownership.
- Final scoped preflight passed 1,383 native tests and warning-denied all-target/
  all-feature checks for core, scene and editor. All 18 platformer gameplay tests,
  three-crate Clippy, 11 catalogue tests and the 1,098-file size gate passed.
- All-target/all-feature WASM checks passed core and scene; the generic browser
  host was rebuilt. Chromium WebGPU fetched 25 assets without runtime errors;
  native Vulkan capture passed. Both game captures were visually reviewed.
- The native editor built and opened the platformer. Interactive desktop picker
  inspection remains unverified; pointer-driven egui tests cover the actual widget.
- CI must verify this slice; all checks passed on prior head `9a1f1a07`. Typed
  endpoint/structural controls, additional motor modes, save-path integration and
  final joint/workspace acceptance remain open.


### Typed endpoint retargeting checkpoint

- Added `Physics.set_joint_endpoints` for distance, hinge, slider and spring.
  Both scoped handles validate before either stored reference changes; null
  clears an endpoint. Inactive targets suspend until active. Settings, enabled
  state, unknown fields and body motion are preserved through synchronization.
- Shared `World::entity_reference` produces stable scene IDs or canonical local
  prefab paths and also supplies native inspector choices. Runtime handles never
  serialize; unstable, stale, out-of-scope and identical targets fail atomically.
- Platformer R switches the lantern between two hooks, including while released.
  All four kinds exercise retarget/clear/reconnect before/after synchronization,
  inactive targets, unknown-field preservation and runtime prefab isolation.
- Final scoped preflight passed 1,264 native tests and warning-denied all-target/
  all-feature checks for core, Decay, editor and platformer. Typed lantern
  preflight had zero errors/reminders. Four-crate Clippy, 11 regenerated catalogue
  tests and the 1,100-file size gate passed.
- All-target/all-feature WASM checks passed core, Decay and platformer; the generic
  browser host was rebuilt. Export and native Vulkan capture passed. Chromium
  WebGPU fetched 25 assets and exercised R switching, suspended retargeting,
  reconnection and prefab spawning without runtime errors. Native/browser game
  captures were visually reviewed.
- CI passed on typed endpoint head `a3713835`.
  Structural controls, additional motor modes, save-path integration and final
  joint/workspace acceptance remain open.


### Typed joint removal checkpoint

- Added `Physics.remove_joint` for all four authored 2D kinds. It removes the
  component and releases the solver constraint at next fixed synchronization,
  preserving the owner, other components and endpoint bodies/motion. Legacy
  distance connections remain separately owned. Valid before initial body sync
  or while suspended; invalid/missing/conflicting joints and missing physics
  fail before mutation. Later joint controls fail until authored again.
- Platformer Z cuts the lantern cord for the rest of the run. The body falls,
  the visible cord disappears and later tether controls stay inert.
- Final scoped preflight passed 332 native tests and warning-denied all-target/
  all-feature checks for Decay and platformer. Typed lantern preflight had zero
  errors/reminders. Two-crate Clippy, 11 regenerated catalogue tests and the
  1,101-file size gate passed.
- All-target/all-feature WASM checks passed both crates; the generic browser host
  was rebuilt. Export and native Vulkan capture passed. Chromium WebGPU fetched
  25 assets and exercised retarget/release/reconnect, cord cutting, subsequent
  tether controls and prefab spawning without runtime errors. Native/browser
  captures were visually reviewed.
- CI passed on typed removal head `5973c8f5`. Creation
  controls, additional motor modes, save-path integration and final joint/
  workspace acceptance remain open.


### Typed distance creation checkpoint

- Added `Physics.create_distance_joint` for an existing owner carrying no joint
  of any authored 2D kind. Scoped endpoint references and finite positive lengths
  validate before mutation, sharing retargeting's reference contract. Null leaves
  an endpoint unbound; inactive targets suspend until active.
- Next fixed synchronization creates the owned constraint, retaining other owner
  components, bodies/motion and legacy connections. Tests cover pre-sync creation,
  repeated removal/recreation, conflicting owners, invalid lengths/references,
  missing physics, inactive targets and runtime prefab isolation.
- Platformer C repairs the cut lantern cord at the selected hook and length;
  repeated cut/repair cycles retain the body and restore tether controls.
- Final scoped preflight passed 338 native tests and warning-denied all-target/
  all-feature checks for Decay and platformer. Typed lantern preflight had zero
  errors/reminders. Two-crate Clippy, 11 regenerated catalogue tests and the
  1,103-file size gate passed.
- All-target/all-feature WASM checks passed both crates; the generic browser host
  was rebuilt. Export and native Vulkan capture passed. Chromium WebGPU fetched
  25 assets and exercised repeated cutting/repair, retarget/release/reconnect and
  prefab spawning without runtime errors. Native/browser captures were visually
  reviewed, including the repaired lantern after recovery from free fall.
- CI passed on typed distance creation head `a2c54306`.
- Typed hinge/slider/spring creation, additional motor modes, automatic save-path
  integration and final joint/workspace acceptance remain open.


### Typed hinge creation checkpoint

- Added `Physics.create_hinge_joint` with finite body-local `Vec2` anchors,
  without transform scale. The enabled hinge starts with angular limits and
  motor disabled; existing motor controls can tune it before synchronization.
- Creation shares empty-owner validation with distance creation and scoped
  endpoint validation with retargeting. Next fixed synchronization owns solver
  allocation, preserving other components, bodies/motion and legacy constraints.
- Tests cover pre-sync creation, repeated recreation, local anchor geometry,
  subsequent motor drive, conflicting owners, invalid anchors, missing physics,
  null/inactive targets and runtime prefab isolation for both constructors.
- Platformer H rebuilds its placed windmill hinge and restarts its motor. A
  repeated-rebuild run keeps the rotor at its axle and observes reversal while
  the separately spawned windmill remains independently owned.
- Final scoped preflight passed 342 native tests and warning-denied all-target/
  all-feature checks for Decay and platformer. Typed setup-script preflight had
  zero errors/reminders. Two-crate Clippy, 11 regenerated catalogue tests and
  the 1,104-file size gate passed.
- All-target/all-feature WASM checks passed both crates; the generic browser host
  was rebuilt. Export and native Vulkan capture passed. Chromium WebGPU fetched
  25 assets and exercised two H rebuilds with a spawned windmill, plus existing
  tether cut/repair/retarget/release controls, without runtime errors.
  Native/browser captures were visually reviewed.
- CI must verify this slice; all checks passed on prior head `a2c54306`. Typed
  slider/spring creation, remaining motor modes, automatic save integration and
  final joint/workspace acceptance remain open.
