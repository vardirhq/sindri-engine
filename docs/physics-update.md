# Incremental physics update

One pull request, delivered in checked feature slices. Each implementation slice
is committed and pushed before work proceeds to the next; smaller checkpoints
are permitted. Gameplay and demonstrations remain in Decay. Runtime, scene,
editor, scripting, proof and documentation move together.

For a new implementation session, read [the recovery handoff](physics-update-handoff.md).
CCD, one-way platforms, forces/rotation, contacts and materials are implemented
and checked. Scene-authored distance constraints form a checked joint foundation.
Hinges, sliders, springs and velocity motors now form checked joint slices.
Hinges and sliders also support checked damped position motors.
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
- [x] Joints: scene-authored distance, hinge, slider, spring and motor controls;
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


### Typed spring creation checkpoint

- Added `Physics.create_spring_joint` with finite body-local `Vec2` anchors,
  positive rest length and non-negative stiffness/damping, including zero.
  Shared owner/scope validation precedes mutation; synchronization creates the
  enabled constraint while retaining other components, bodies/motion and legacy
  connections. Existing tuning can change the new component before body sync.
- Tests cover pre-sync and repeated recreation, local-anchor force response,
  retuning, body-motion/legacy preservation, invalid owners/settings, missing
  physics, null/inactive endpoints and isolated runtime prefab creation.
- Platformer B rebuilds its light's spring at the current rest length. A scripted
  run rebuilds in both tuning phases, keeps the independent trolley on its rail
  and observes continued physical short/long suspension.
- Final scoped preflight passed 346 native tests and warning-denied all-target/
  all-feature checks for Decay and platformer. Typed spring-script preflight had
  zero errors/reminders. Two-crate Clippy, 11 regenerated catalogue tests and
  the 1,105-file size gate passed. Joint-call classification is extracted from
  the main dispatcher to keep functions within the repository size limit.
- All-target/all-feature WASM checks passed both crates; the generic browser host
  was rebuilt. Export and native Vulkan capture passed. Chromium WebGPU fetched
  25 assets and exercised two B rebuilds alongside hinge recreation, prefab
  spawning and existing tether controls without runtime errors. Native/browser
  captures were visually reviewed.
- CI passed on typed spring creation head `d7b64c78`. Typed slider creation, remaining motor
  modes, automatic save integration and final joint/workspace acceptance remain open.


### Typed slider creation checkpoint

- Added `Physics.create_slider_joint` with finite body-local anchors, unit local
  axes and optional finite travel bounds. Enabled lower distance cannot exceed
  upper distance. The enabled slider starts with its motor disabled; existing
  motor controls can tune it before synchronization.
- Shared empty-owner/scope validation precedes mutation. Next synchronization
  creates the owned constraint, retaining other components, bodies/motion and
  legacy connections. Native tests cover pre-sync/repeated creation, local-axis
  bounded travel and reversal, invalid anchors/axes/limits, inactive/unbound
  targets and runtime prefab isolation for all four constructors.
- Platformer J rebuilds its trolley slider and restores its current motor
  direction. Repeated rebuilds retain bounded reversal and the independently
  owned spring suspension.
- Final scoped preflight passed 350 native tests and warning-denied all-target/
  all-feature checks for Decay and platformer. Typed trolley-script preflight had
  zero errors/reminders. Two-crate Clippy, 11 regenerated catalogue tests and
  the 1,106-file size gate passed.
- All-target/all-feature WASM checks passed both crates; the generic browser host
  was rebuilt. Export and native Vulkan capture passed. Chromium WebGPU fetched
  25 assets and exercised two J rebuilds alongside spring/hinge recreation,
  prefab spawning and tether controls without runtime errors. Native/browser
  captures were visually reviewed.
- CI passed on typed slider creation head `04ef2587`.
  Remaining motor modes, automatic save integration and final joint/workspace
  acceptance remain open.


### Hinge position motor checkpoint

- Added Sindri-owned `MotorMode2d` and a force-based hinge position motor with a
  relative angular target in `[-pi, pi]`, finite non-negative stiffness/damping
  and the existing torque cap. Enabled angular limits still bound motion.
  Old payloads default to velocity mode; no backend types enter scene data.
- Typed `Physics.set_hinge_position_motor` validates all settings before changing
  the component, preserves unknown fields/body motion and works before bodies
  exist. `set_hinge_motor` explicitly restores velocity mode. Zero torque coasts;
  settings survive suspension and collider rebuilds.
- Added for the platformer: P switches both placed and spawned windmills between
  holding 0.6 radians and reversing. H recreates the placed hinge and a typed
  `Windmill` message restores its selected drive and current direction.
- Native tests prove holding/retargeting, offset anchors, torque caps, enabled
  limits, invalid-call atomicity, zero-torque coasting, missing physics/wrong kind,
  suspension/rebuild and mode switching. Checked scene commands exercise undo,
  redo and canonical save/reopen with the position drive retained.
- Final scoped preflight passed 865 native tests and warning-denied all-target/
  all-feature checks for physics, scene, Decay and platformer. Both changed scripts
  passed typed preflight with zero errors/reminders. Four-crate Clippy, 11 generated
  catalogue tests and the 1,111-file size gate passed.
- All-target/all-feature WASM checks passed all four crates; the generic browser
  host was rebuilt. Export and native Vulkan capture passed. Chromium WebGPU
  fetched 25 assets, exercised P hold/H recreation/P resume alongside prefab
  spawning and tether controls, and reported no runtime errors. Native/browser
  captures were visually reviewed; both browser windmills held the same angle
  through recreation and resumed motion.
- CI passed on hinge position head `adb632e5`. Slider position motors, automatic save integration, visual editor review and final
  joint/workspace acceptance remain open.


### Slider position motor checkpoint

- Reused Sindri-owned `MotorMode2d` for damped force-based slider position drive.
  Old payloads default to velocity mode. The finite signed target is anchor
  separation along the first body's local axis, in unscaled world units;
  stiffness/damping are finite and non-negative. The force cap and enabled
  travel limits still apply, including targets outside the limits.
- Typed `Physics.set_slider_position_motor` validates before mutation, retains
  unknown fields/body motion and works before bodies exist. The velocity setter
  explicitly switches back. Zero force coasts; suspension and collider rebuilds
  retain settings.
- Added for the platformer: O parks/releases the trolley at signed distance 0.5;
  J recreates its slider and restores the selected drive and current direction.
  The independent spring remains attached and continues its tuning.
- Native tests prove signed retargeting on rotated rails with offset anchors,
  force caps, travel limits, invalid settings/calls, coasting, missing physics,
  pre-sync drive and suspension/rebuild. Checked scene commands exercise
  undo/redo and canonical save/reopen; gameplay parks, rebuilds and resumes
  bounded reversal while retaining its spring.
- Scoped preflight passed 874 native tests and warning-denied all-target/
  all-feature checks for physics, scene, Decay and platformer. Typed trolley
  preflight had zero errors/reminders. Four-crate Clippy, 11 regenerated catalogue
  tests and the 1,115-file size gate passed. All-target/all-feature WASM checks
  passed all four crates; the generic browser host was rebuilt.
- Export and native Vulkan capture passed. Chromium WebGPU fetched 25 assets
  and exercised O parking/J recreation/O release alongside prefab spawning and
  tether controls without runtime errors. Visually reviewed native/browser
  captures show the trolley retain its parked position through recreation and
  resume travel while its spring stays attached.
- CI passed on slider position head `e92487ec`. Automatic save-path integration,
  native visual editor review and final joint/workspace acceptance remain open.


### Editor reference-aware save checkpoint

- Connected editor scene Save/Save As and subtree prefab authoring to
  `World::to_scene_with_references` with the active component registry. Existing
  placed instances still collapse to references; unknown fields and the live
  world remain unchanged. Stable IDs remain required before saving.
- Missing local targets, wrong-type reference fields and unstable endpoints fail
  before writing or adopting a new path. Tests preserve disk, the prior path and
  the agreed document on both Save and Save As failures. Existing canonical,
  placed-instance, missing-prefab and prefab-root save regressions pass.
- Two runtime instances of each joint kind reopen with isolated endpoints through
  the actual editor save API. Saved subtrees spawn twice with independent solver
  ownership. The platformer's real Decay setup script spawns its nested windmill;
  editor Save As/reopen retains its fixed axle and reversing motor. This connects
  an existing engine capability to editor authoring, proven by that showcase.
- Decay `Save` has only a number/flag progress store, so there is no existing
  world-save operation to wire. Added a separate absent script-triggered world
  snapshot save/load row to parity. Editor Save remains refused during Play;
  this slice does not add a gameplay persistence lifecycle.
- Scoped preflight passed 622 native editor tests and warning-denied all-target/
  all-feature checks. Editor Clippy, 11 catalogue currentness/completeness tests,
  the 1,116-file size gate and the WASM editor-stub check passed. No runtime,
  browser, renderer, dependency or Decay host/script surface changed.
- CI must verify this slice; all checks passed on prior slider head `e92487ec`.
  Native visual editor review, the separate gameplay snapshot gap and final
  joint/workspace acceptance remain open.

### Joint motor authoring checkpoint

- Native editor review on an isolated X display opened a scratch copy of the
  platformer, rendered the level and inspected the trolley joint. Its endpoint
  picker displayed scoped choices and inactive labels; clearing, Save and undo
  were confirmed by inspecting the saved payload. Typing a missing endpoint
  displayed the inline resolver diagnostic. No shipped scene was edited.
- The review found free-text motor modes. Hinges and sliders now register
  velocity/position choices from `MotorMode2d`, using the existing generic
  inspector and checked-command path. Older payloads inherit velocity mode;
  changing mode retains the other settings. A schema test checks the offered
  spellings against enum serialization. Added for platformer authoring.
- Full review of all joint kinds, numeric edits, save/reopen and editor Play
  remains open, as do character movement, accelerated queries, 3D/voxel physics
  and final integration. The joint checklist is deliberately still unchecked.
- Rebuilt native editor review selected Position for the nested windmill hinge
  and trolley slider, then saved and inspected both payloads. Trolley undo
  restored the omitted default mode; redo restored Position, with endpoints and
  travel limits retained. Scoped preflight passed 515 tests and warning-denied
  checks; two-crate Clippy, 11 catalogue tests, formatting/file-size gates and
  all-target/all-feature WASM checks passed. CI must verify the new head.

### Native 2D joint acceptance checkpoint

- Completed [the native editor review](physics-joint-editor-review.md) on
  `eb994996`, whose CI is green. All four joint kinds retain numeric edits on
  Save/reopen, including the nested hinge override. Distance undo/redo and
  inactive reference diagnostics were inspected.
- Editor Play runs the mechanisms, switches modes through P/O, rebuilds through
  H/J/B and spawns an independent windmill through V. Live inspector settings
  reflect Decay. Play Save is refused without changing disk; Stop restores
  authored settings and saving again preserves the pre-Play hash.
- The 2D joint checklist is complete across engine, authoring, scripting and
  platformer proof. 3D joints and gameplay world snapshots remain separate gaps;
  character movement, accelerated queries, 3D/voxel and final integration remain.
- Documentation preflight passed formatting and the 1,116-file size gate.
  Re-ran all three editor joint-save tests and 26 platformer regressions;
  all 29 passed. This checkpoint changes documentation only; CI must verify it.

### Character movement foundation checkpoint

- Added engine-owned read-only 2D sweep/slide queries with shape/pose/displacement,
  positive skin, bounded iterations and ordered collision/outcome results.
  Ordinary shape-cast initial-overlap behavior stays unchanged.
- Native regressions exercise wall tangents, touching/escape/approach, initial
  penetration, corners/budgets, rotated surfaces, filtering, immediate teleports
  and removal, deterministic ties, validation and two-sided one-way geometry.
- Established [the character movement contract](character-movement.md) and
  existing crate boundary. Added for platformer adoption, not yet game-proven.
  Ground/slope/step/platform and one-way movement policies, scene/editor/Decay
  integration and native/browser gameplay proof remain open. Acceptance remains
  unchecked; no new dependency, host API, script or component registration.
- Scoped preflight passed 75 native physics tests and warning-denied all-target/
  all-feature checks. Clippy, all-target/all-feature WASM compilation, 11
  catalogue currentness/completeness tests and the 1,119-file size gate passed.
  Runtime/editor/browser behavior is unchanged until this API is integrated.
  CI must verify the pushed foundation head.

### Ground classification foundation checkpoint

- Added read-only `probe_ground`/`probe_ground_where` with configurable unit up,
  slope angle, travel and skin. Results retain the nearest steep hit rather than
  probing through it, and mark initial penetration unwalkable.
- Contact probing before the sweep closes a zero-time shape-cast edge case at
  the skin boundary. Tests exercise stationary support, shape extents, rotated
  up/surfaces, slope boundaries, steep obstruction, filtering, lifecycle and
  validation. Shared movement test fixtures now live in one support module.
- Added for the platformer, not yet adopted. Grounded state/snap and uphill/
  downhill/step/platform/one-way movement policy, authoring, Decay and native/
  browser game proof remain open. Character acceptance is still unchecked.
- Scoped preflight passed 83 native physics tests and warning-denied all-target/
  all-feature checks; Clippy, all-target/all-feature WASM compilation, 11
  catalogue tests and the 1,123-file size gate passed. No component registration,
  script/host API, dependency or existing runtime/render/browser behavior changes.
  CI must verify this slice.

### Grounded movement and optional snap checkpoint

- Added read-only `move_and_slide_grounded` and its predicate variant, composing
  slide and ground probing with a shared skin. Results separate total translation,
  unchanged slide outcome, pre-snap support and snap translation.
- Snap defaults off; optional downward travel accepts only walkable support.
  Upward requests suppress snap and grounded state even when a ceiling blocks
  ascent. Initial penetration never snaps. Support is measured after motion, so
  walking off a ledge loses support. Gameplay owns prior state and snap decisions.
- Native regressions exercise landing/repeated support, ledge departure, snap
  limits, downhill support/steep rejection, blocked ascent, initial penetration,
  arbitrary up/rotated capsule, filters/current poses, budget exhaustion, read-only
  and validation. A rotated-slope re-probe exposed early narrow-phase stopping;
  zero-travel support now allows one percent of skin plus one epsilon. A boundary
  regression rejects gaps beyond that relative tolerance.
- Added for platformer adoption, still engine-only. Uphill/downhill motion limits,
  steps/platforms, one-way/drop-through policy, scene/editor/Decay and native/
  browser game proof remain open; character acceptance remains unchecked.
- Scoped preflight passed 94 native physics tests and warning-denied all-target/
  all-feature checks. Physics Clippy, all-target/all-feature WASM compilation,
  11 catalogue tests and the 1,126-file size gate passed. No dependencies, scripts,
  host calls or component registrations change. Existing runtime/editor/render/
  browser behavior is unchanged until integration; CI must verify this slice.

### Grounded slope movement checkpoint

- Grounded movement now enforces the same slope angle/tolerance as support.
  Upward-facing steep contacts cannot generate rise beyond the positive remaining
  request. Horizontal/downward approaches cannot become uphill climbs; explicit
  jumps can still slide with bounded rise and steep descent stays ungrounded.
- Geometric `move_and_slide` retains unrestricted projection. Both paths share
  one sweep implementation with an internal slope policy; no new dependency,
  component, host call or script. Snap still accepts only walkable support.
- Native regressions exercise configured/boundary angles, ascent/jump/descent,
  downhill following, walls/ceilings, mirrored capsule, rotated box/up, filters,
  read-only state and unchanged geometric movement.
- Added for platformer adoption, still engine-only. Steps/clearance, moving
  platforms, one-way/drop-through and scene/editor/Decay/native/browser game proof
  remain open; character acceptance remains unchecked.
- Scoped preflight passed 105 native physics tests and warning-denied all-target/
  all-feature checks. Physics Clippy, all-target/all-feature WASM compilation,
  11 catalogue tests and the 1,128-file size gate passed. No existing game,
  editor, render or browser behavior changes until this primitive is integrated.
  CI must verify the pushed slope head; full acceptance remains open.

### Step and clearance checkpoint

- Added optional `step_height` (default zero) to grounded movement. Supported,
  non-ascending requests can try a full clear lift, slope-limited horizontal sweep
  and walkable landing within the height cap, followed by endpoint support check.
  Progress must exceed the baseline by one skin; rejected candidates retain it
  exactly. Full-height headroom and minimum progress are conservative limits.
- Results separate selected forward slide, step lift and downward landing/snap;
  total translation is applied once. Ordinary snapping can remain disabled.
- A flat-floor test exposed tilted skin-cast normals that produced a large box
  hop. Movement now refines impact geometry with a contact query using the shared
  skin tolerance; ray/overlap/ordinary shape-cast contracts remain unchanged.
- Native regressions exercise all probe shapes, height boundaries, tall walls,
  ceilings/overhangs, unsupported/jump/overlap rejection, absent/steep landings,
  arbitrary up/rotation, filtering, fallback, read-only/accounting and validation,
  plus repeated flat-floor movement.
- Added generally for platformer adoption, still engine-only. Moving platforms,
  one-way/drop-through controller policy, scene/editor/Decay integration and
  native/browser game proof remain open; character acceptance stays unchecked.
- Scoped preflight passed 118 native physics tests and warning-denied all-target/
  all-feature checks. Physics Clippy, all-target/all-feature WASM compilation,
  11 catalogue tests and the 1,130-file size gate passed. No dependencies, host
  calls, scripts or component registrations change. Existing gameplay/browser/
  editor rendering paths do not use these primitives yet; CI must verify the
  pushed step head. Prior slope head `6de2dcc8` has completed CI successfully.

### Synchronized platform carry checkpoint

- Added opt-in `platform_support` snapshots of runtime entity/previous body pose.
  Current local collider pieces verify old walkable support with the same filters.
  Removed, inactive/filtered, unsupported, steep or penetrating old support skips
  carry. Inputs and derived points/destinations validate before movement.
- Previous/current synchronized poses produce probe-origin displacement, including
  translation and rotation about the support origin. A separate geometric carry
  sweep excludes only the support; character slide/step/ground then includes it
  again. Total translation includes actual carry once; no velocity is added.
- Carry results retain entity/current pose, requested translation and collision/
  budget outcome. Hosts own/advance support snapshots even when clipped, switch
  them to the final grounded hit and clear them on teleport or structural edits.
- Native regressions exercise all shapes, vertical motion, rotation, repeated
  snapshots, wall clipping/ceiling crush, jumps/ledges, sensor/mask/predicate/removal,
  stale/steep support, compound local geometry, carry-step accounting, validation
  and actual position/velocity-kinematic ordering.
- Rotation follows a chord with fixed probe orientation. Continuous arc and
  rotating-probe sweeps are absent, now explicit parity gaps. Added generally for
  platformer adoption; one-way controller policy and scene/editor/Decay/native/
  browser game proof remain open. Character acceptance stays unchecked.
- Scoped preflight passed 132 native physics tests and warning-denied all-target/
  all-feature checks. Physics Clippy, all-target/all-feature WASM compilation,
  11 catalogue tests and the 1,133-file size gate passed. No dependencies, host
  calls, scripts or component registrations change. Existing game/editor/browser
  paths have not adopted the controller yet. Prior step head `75ab9215` is green
  in CI; CI must verify the pushed carry head.

### One-way character movement checkpoint

- Grounded movement now respects each solid piece's `OneWay2d` support side and
  cone in initial-overlap checks, sweeps, support/snap, step phases and previous
  platform verification/carry. Local normals follow body and piece rotation;
  support-plane slop matches the existing solver policy. Ordinary geometric
  queries and dynamic-body solver/drop timer behavior remain unchanged.
- `GroundedSlideOptions2d.drop_through` ignores only one-way solids throughout a
  request, removing their grounding/carry while preserving ordinary floors and
  sensor filtering. Hosts own drop duration/cancellation; the read-only query
  has neither entity identity nor simulation time.
- Fourteen native regressions cover all probe shapes, ascent/descent, underside
  and deep overlap, shallow front penetration, normal cones/rotations/extreme
  lengths, snapping/drop/cancellation, mixed solid/sensor pieces, masks/entity
  exclusions/predicates/ties, step landing, carry and budget/overlap outcomes.
- Scoped preflight passed 146 native physics tests, warning-denied all-target/
  all-feature checks and formatting/file-size gates (1,135 Rust files). Physics
  Clippy, all-target/all-feature WASM checks and eleven catalogue tests passed.
  No dependencies, host calls, component registrations or scripts change.
- Added for the platformer showcase, which still uses its dynamic-body hero.
  Character acceptance stays unchecked: scene/editor/Decay/platformer integration
  with native and real-browser game proof remains open. Prior carry head
  `c4309a77` is green in CI; CI must verify this checkpoint after pushing.
