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
- [x] 2D character movement: reusable sweep/slide collision primitive, slopes,
  steps, ground state and moving platforms; gameplay policy remains Decay.
- [x] 2D accelerated queries: synchronized spatial index, unchanged filtering and
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

### Scene character ownership checkpoint

- Added validated `sindri.physics2d.character` settings using one solid authored
  collider probe, plus optional sensors. Scene synchronization derives a
  stationary kinematic body and rejects competing rigid-body ownership, missing
  transforms and compound solids. No game input/gravity/jump rules are added.
- Runtime requests replace displacement/snap input per fixed step and maintain
  simulation-time drop-through. Missing/inactive requests are discarded;
  spawning, pause, invalid timesteps, cancellation and lifecycle resets have
  explicit contracts. Saved settings retain unknown fields without runtime
  requests, timers, support handles or cached results.
- Fresh support is seeded before the solve; controllers use actual current
  platform poses afterward, apply motion once and write back in parent space.
  Teleports, settings/parent changes, removal and structural edits clear old
  snapshots. Fresh support after rebuilds carries only the new step's motion.
  `character_motion` and disjoint script borrows expose cached results and the
  queue for the forthcoming typed host surface.
- Twenty-four native regressions cover first-step vertical carry, current
  translation/rotation, parented riders, body/probe offsets, request replacement,
  snapping/ascent, timed drop to ordinary floors, lifecycle/slot reuse, direct
  teleports, saved data and next-solve sensor processing.
- Scoped preflight passed 475 native scene tests, warning-denied all-target/
  all-feature checks and formatting/file-size gates (1,142 Rust files). Scene
  Clippy, all-target/all-feature WASM and eleven catalogue tests passed; generated
  component metadata was regenerated. No dependencies or Decay host calls change.
- Controller motion reaches solver response and discrete sensors at the next
  solve. Same-step response, compound solid probes and swept controller triggers
  remain absent and have explicit parity rows. Added for platformer adoption;
  checked editor/Decay/game/native/browser proof and character acceptance remain
  open. Prior one-way head `5597ce44` is green in CI; CI must verify the new head.


### Typed character-host checkpoint

- `Physics.move_character` queues replacement world displacement/snap input;
  optional `Physics.character_motion` copies the previous completed pass's
  translation, support, step/snap, carry, hits and failure flags. Null guards
  and inactive/despawned reference filtering are exercised; edits cannot alter
  cached results. Invalid request values preserve pending input.
- Authored controllers use the scene drop timer through `Physics.drop_through`;
  dynamic bodies retain their solver timer and physics-only host compatibility.
  A separate optional context preserves the existing physics/service API.
  Shared runtime/editor Play and platformer harness provide the scene borrows.
- Scoped preflight passed 1,023 native tests and four zero-error/reminder typed
  fixtures. Ten controller-host regressions and a shared-session regression
  exercise request ordering, copied values/lists, platform carry, filtering,
  invalid inputs, drop-through/cancellation and old-host compatibility.
- Warning-denied Clippy and native checks passed all four changed crates.
  Eleven catalogue/currentness tests passed and API documents were regenerated.
  WASM passed Decay/platformer with all targets and the shared game host with
  native-only integration tests excluded. Formatting/file-size gates passed.
- Real Chromium ran the rebuilt host and exported fixture under
  `/examples/character-api/`: WebGPU configured, scene/script assets fetched,
  actor/floor rendered and the script verified queued motion, grounding and
  independent snapshot copies. This is host evidence for platformer adoption;
  editor authoring/undo/Play and real platformer/native/browser proof remain.
  Character acceptance is still unchecked; the platformer retains its dynamic
  hero. Prior refactoring head `3842bac5` is green in CI; the new head is pending.


### Platformer controller-adoption checkpoint

- The real Hero script now integrates velocity/gravity, queues character
  displacement and reads walkable support instead of solver velocity/contacts.
  Input, coyote time, jump buffering, variable jump height and drop policy remain
  in Decay. Respawn clears script velocity and pending movement/drop state.
- The authored hero has Character 2D and one ground-filtered capsule probe.
  Separate body/foot sensors preserve the previous coin/flag pickup volume;
  the ground mask preserves passage past the dynamic crate. The crate retains
  authored/typed CCD and contact-impulse gameplay proof.
- Scoped preflight passed 29 native platformer tests, including the existing
  no-fall run to the flag with coins, one-way jump/drop/underside and every joint,
  material and crate regression. Three new tests cover short/held jumps,
  acceleration/braking and respawn. Both changed scripts passed typed preflight
  with zero errors/runtime-contract reminders. Warning-denied native checks,
  Clippy, all-target/all-feature WASM, formatting and file-size gates passed.
- Real Chromium loaded the exported game under `/examples/platformer/`, fetched
  25 assets, configured a 960×540 WebGPU canvas and drew the level. Keyboard
  right/jump captures show the hero moving and rising from the starting floor;
  no browser/script/GPU errors occurred. Native project capture also passed
  and its rendered level was reviewed. This is delivery/input evidence, not
  a browser run to the goal.
- Added for the platformer genre showcase. Checked editor authoring/undo/Play,
  authored game slope/step/platform-carry proof and browser goal proof remain
  open; character acceptance stays unchecked. No dependencies, host API or
  generated catalogue changes. Final-head CI must verify this checkpoint.


### Platformer slope/step gameplay checkpoint

- Add two 0.25-unit stone risers and a roughly 20-degree plank boardwalk to the
  starting field, using the existing painted tile assets and general tilemap
  collision/rotation. No bespoke Rust gameplay or engine API is introduced.
- Hero keeps intended horizontal speed at obstacles while the controller clips
  displacement. Resetting that speed on every hit starved conservative step
  selection at a riser; subsequent input requests can now clear it. Ceiling
  collisions still cut rising velocity. The camera test allows two seconds of
  travel through the new terrain while retaining its bounds/follow assertions.
- Four native tests run the real Hero script without jump input: both risers
  produce accepted lifts onto their own support entities, the boardwalk grants
  walkable uphill support and downhill snap, disabling steps blocks the first
  riser, and a lower slope limit (with steps disabled) blocks the same incline.
  All 33 platformer tests pass, including the original no-fall flag/coin run.
- Typed Hero preflight has zero errors/runtime-contract reminders. Native
  warning-denied checks/Clippy, all-target/all-feature WASM, formatting and
  file-size checks pass. Native project capture and Chromium export/input
  checks draw the new terrain; browser snapshots show the hero stepping and
  walking on the boardwalk. Chromium fetches 25 assets with no script/GPU errors.
- This is general character movement proven by the platformer genre showcase.
  Checked editor authoring/undo/Play, game moving-platform carry and browser goal
  proof remain open, so character acceptance stays unchecked. No generated
  catalogue, host or dependency changes. The prior adoption head `6bd4f0ef`
  passed all CI; CI must verify this checkpoint's head.


### Platformer moving-platform gameplay checkpoint

- Add a two-unit one-way plank ferry crossing the first gap, using ordinary
  kinematic velocity, generated tilemap collision and the shared wood material.
  Its Ferry script reverses between authored bounds. Hero adds no platform
  velocity or displacement: scene support snapshots supply solved translation.
- Four native gameplay regressions run the real Hero/Ferry scripts: a stationary
  rider follows each solved displacement once across and back, a running jump
  boards from the normal starting field, jumping clears carry until landing,
  and drop-through leaves carry and falls into the ordinary respawn path.
  Assertions cover relative pose, total/carry displacement, both directions,
  grounded support and airborne null carry. All 37 platformer tests pass,
  including the original no-fall flag/coin, slope/step, one-way and joint tests.
- Ferry typed preflight reports zero errors/runtime-contract reminders. Native
  warning-denied checks/Clippy, all-target/all-feature WASM, formatting and
  file-size gates pass. Native capture renders the authored ferry. Chromium
  exports the same assets with only the hero's initial pose moved aboard:
  26 assets load, WebGPU draws the game, snapshots show both riding directions
  and a keyboard jump, with no browser/script/GPU errors. This is carry/input
  evidence; the browser run to the goal remains open.
- This is general character carry proven by the platformer genre showcase.
  Rotation and collision-clipped carry retain engine/scene regression evidence;
  continuous arcs and rotating probes remain explicit gaps. Checked editor
  authoring/undo/Play and browser goal proof remain open, so character acceptance
  stays unchecked. No new host APIs, dependencies or generated catalogue changes.
  Prior terrain CI has no failures; its test/render job is still running at
  the last inspection. CI must verify the next head.


### Native character-editor proof checkpoint

- Rebuilt the native editor at `da9a5328` and reviewed a scratch platformer on an
  isolated display. Every Character 2D setting was exercised: numeric edits,
  carry toggle and up-vector reversal. Snap undo/redo and saved JSON were checked;
  a negative skin attempt retained the valid value. Close/reopen retained edits.
- Inspector removal saved an absent component; Add Component → Physics →
  Character 2D saved registered defaults. Undoing add/removal restored all custom
  settings and the pre-sequence file hash. The shipped scene was not edited.
- Reopened Play settles, runs and jumps the hero through its real Decay script;
  the inspector is read-only. Play Save refuses and file hashes remain unchanged.
  Stop restores the authored pose/settings and saving matches the pre-Play hash.
- [The native review](physics-character-editor-review.md) records repeatable
  procedure, values, artifacts and limits. This is exercised editor evidence,
  not a code/API/gameplay change. Editor parity earns a check; overall character
  acceptance remains unchecked until browser goal proof/final verification.
  Prior ferry CI has no failures, with its test/render job still running at the
  last inspection. The next head requires CI verification.


## Browser character goal checkpoint

- The CI browser job now exports a scratch copy of the shipped platformer with
  one read-only Decay observer. Its normal spawn, Hero script and terrain are
  unchanged. Playwright drives ArrowRight/Space using the native goal player's
  terrain decisions and waits for an observed release between jumps.
- Three fresh Chromium/WebGPU runs reached the flag with five coins, eight jumps
  and zero falls (11.533, 11.467 and 11.367 simulated seconds). Each fetched 27
  assets under `/examples/platformer/`, configured a 960×540 canvas and passed
  pixel/script/GPU checks. The final capture shows the win message and flag.
- The permanent gate requires at least three coins and no falls within 90 wall
  seconds. It never writes game state or supplies browser-specific game rules.
- Scoped preflight passed formatting, file sizes, warning-denied platformer
  check, all 37 native tests and typed observer preflight with zero errors or
  runtime-contract reminders. Node helper/smoke syntax, Python fixture syntax
  and workflow YAML parsing passed. Host binaries are unchanged from the typed
  controller checkpoint; the new observer ran in the real WASM host.
- This completes the 2D character acceptance together with the existing engine,
  scene, Decay, game and native editor evidence. A 3D controller and the explicit
  compound/solver/trigger/carry gaps remain absent. Prior head `1ea225dd` passed
  every CI check; CI must verify this new checkpoint. Accelerated queries,
  3D/voxel physics and full final integration remain unchecked.


## Indexed ray/area query checkpoint

- A query-only per-piece BVH now selects candidates for 2D rays, overlaps and
  shape casts. Insert/remove/teleport update immediately; completed solves refresh
  non-static pieces. Position-kinematic targets remain pending until solving.
  Candidates retain entity/piece ordering, masks, sensor policy, whole-entity
  predicates and the unchanged exact geometry phase. No new dependency or public
  API was introduced. Overflowing bounds use an exhaustive fallback.
- Four new regressions compare indexed results with exhaustive candidates before
  stepping and after rotation/teleport/removal/reinsertion, verify solved velocity
  and pending position targets, cover overflow fallback and count sparse query
  work. At 100, 1,000 and 10,000 pieces, first/middle/last samples select one piece;
  the 10,000-piece rays visit 28–34 BVH nodes rather than every piece.
- Scoped preflight passed all 150 physics tests, warning-denied all-target check,
  formatting and size checks. Physics all-target/all-feature Clippy and WASM
  checks passed. A further 856 native tests across scene, Decay and platformer
  passed, including all 37 platformer tests.
- A rebuilt warning-denied WASM host ran the normal platformer to the flag with
  five coins/eight jumps/no falls and fetched 27 assets. Physics Playground's real
  Chromium controls exercised masks, sensor opt-in, inside/miss rays, swept
  circles, overlaps and events, fetched five assets and passed WebGPU/pixel/error
  checks using CI's `scene,script,font` asset-kind setting. Both captures were
  visually reviewed. The first Playground invocation omitted that setting and
  incorrectly required a runtime request for the licence text; the corrected
  invocation matches CI and passes without source changes.
- This is an internal query optimization used by the platformer genre showcase;
  it adds no gameplay policy. Controller penetration/sweep/ground/support paths
  still scan and are the next slice, so accelerated-query acceptance remains
  unchecked. Prior head `7dc8ad3a` is green in CI; this checkpoint needs its own CI.


## Indexed controller query checkpoint

- Controller penetration, skin support and slide casts now select BVH candidates
  with bounds expanded for contact prediction/target separation. Steps, ground
  probing, snap and carry collisions share these paths. Filtering, exact geometry,
  one-way policy and entity/piece ordering are unchanged. Known previous support
  reconstructs only that entity's pieces, independent of current index bounds.
- Four new regressions compare exhaustive candidates across circles, boxes,
  capsules, rotation, arbitrary up, skin and drop policy; retain zero-travel skin
  support between separated shapes; reconstruct a platform moved 100 units away;
  and visit one local entity at 100, 1,000 and 10,000 sparse pieces. They complement
  the earlier ray/overlap/cast differential and 28–34-node sparse ray evidence.
- Scoped preflight passed all 154 physics tests, warning-denied all-target check,
  formatting/size gates. All-target/all-feature Clippy and WASM passed. A further
  856 native scene/Decay/platformer tests passed, including the 37 platformer
  regressions for goal/coins/no-falls, steps/slopes, carry, jump/landing and drop.
- The rebuilt warning-denied WASM host fetched 27 assets and reached the flag
  with five coins/no falls in two isolated Chromium runs (11.433 and 11.417
  simulated seconds). The previous host's isolated baseline also collected five
  coins with no falls. A separate aboard-start ferry export fetched 26 assets,
  rendered both riding directions and a keyboard jump; captures were reviewed.
- The first goal attempt overlapped a second Chromium instance and a native build
  and fell after late jump positions. The isolated comparison is consistent with
  delayed input under contention; it is not proof of arbitrary-load reliability.
  Browser checks remain sequential as in CI; assertions were not weakened and no
  gameplay or host hook was added to make the run pass.
- This completes 2D query acceleration, exercised by the platformer genre showcase
  through its existing Decay controller/clearance calls. Dense/long queries remain
  output-dependent, non-static pieces need post-step refresh, and overflowing
  bounds conservatively scan. No dependencies, public APIs or catalogue entries
  changed. 3D physics/queries, voxel proof and final integration remain unchecked;
  CI must still verify the new checkpoint.


## Standalone 3D foundation checkpoint

- `PhysicsWorld3d` now owns fixed-step static/dynamic/position-kinematic/velocity-
  kinematic bodies, compound box/sphere/Y-capsule collision, XYZ velocity and
  impulses, quaternion poses, pending kinematic targets and normalized entity
  collision/sensor transitions. Backend handles remain private. Complete inputs
  validate before mutation; near-unit quaternions normalize after validation,
  and rotation locking clears initial/live angular velocity.
- Eleven new native regressions exercise all shapes landing on solid geometry,
  body/local quaternion rotation and offsets, compound mass/impulses, masks and
  sensors, fixed steps/gravity scale, targets/teleports, rotation locking, atomic
  rejection and removal/arena reuse. All 165 physics tests and a further 856
  scene/Decay/platformer tests passed (1,021 native tests total). Scoped preflight,
  warning-denied Clippy and all-target/all-feature WASM compilation passed.
- This engine foundation is a prerequisite for Causeway's actual voxel-terrain
  proof. It has no scene synchronization, checked editor authoring/Play, typed
  Decay Vec3 access, 3D queries or resident/edited voxel collision yet. WASM
  compilation does not prove 3D browser simulation. 3D acceptance and final
  integration remain unchecked. No dependency, host call or component
  registration changed; the existing 2D event-kind reexport remains compatible.
- Prior head `02b99c8e` is green in CI; this new checkpoint needs verification.
  Continue with 3D queries, then the scene/editor/Decay and voxel proof slices.


## Exact 3D query checkpoint

- Standalone 3D rays, overlaps and fixed-orientation box/sphere/Y-capsule casts
  now reconstruct current body/local quaternion poses independently of solver
  caches. Insertions and teleports are visible before stepping; kinematic targets
  remain pending until solving. Masks match memberships, sensors are opt-in,
  exclusion/predicates cover whole entities, and overlaps return sorted unique
  handles. Exact hit ties prefer entity handle then authored piece order.
- Eight native regressions cover every shape/XYZ axis, world points/normals,
  body/local/probe rotations, filtering/predicates/compound deduplication,
  equal-distance piece normals, insert/teleport/solved/pending/removal poses,
  initial overlap, finite endpoints, extreme finite directions and invalid inputs.
  All 173 physics tests pass, alongside scoped preflight, warning-denied Clippy
  and all-target/all-feature WASM compilation.
- Queries currently scan sorted entities and pieces; spatial acceleration is the
  next engine slice. These are prerequisites for Causeway voxel-terrain proof,
  with scene/editor/Decay access and native/browser game proof still absent.
  No host call, component registration, dependency or 2D behavior changed.
  3D acceptance and final integration remain unchecked; CI must verify this head.


## Indexed 3D query checkpoint

- A query-only per-piece BVH now selects 3D finite ray segments, overlap probe
  bounds and fixed-orientation sweep start/end bounds. Candidates retain entity/
  piece order, masks, sensor policy, whole-entity predicates and exact geometry.
  Insertion/removal/teleport update immediately; non-static pieces refresh after
  completed steps. Position-kinematic targets retain their pending semantics.
- Four regressions compare exhaustive candidates across all shapes, XYZ/local/
  probe rotations, masks/sensors/predicates, insertion/teleport/removal/reuse and
  solved dynamic/kinematic motion. Sparse queries at 100/1,000/10,000 pieces select
  one local piece/entity; sampled 10,000-piece XYZ rays visit 26–34 BVH nodes.
  Piece/probe bound overflow conservatively retains candidates and removal clears
  the fallback set. All 177 physics tests, warning-denied Clippy, scoped preflight
  and all-target/all-feature WASM compilation pass.
- The first overflow differential test exposed non-finite backend shape-cast
  output at near-maximum finite coordinates/extents. That numerical reporting
  gap is now explicit in parity and the contract. Overflow tests assert candidate
  retention, not numerical stability of exact geometry at those extremes.
- This is an internal engine prerequisite for Causeway voxel-terrain proof.
  No dependency, public API, host call, component registration or 2D behavior
  changed. Dense/long queries remain output-dependent, and non-static pieces
  require refresh. Scene/editor/Decay/voxel and native/browser game proof remain
  open; 3D acceptance and final integration stay unchecked. CI must verify this
  checkpoint; continue with 3D scene synchronization and authoring.


## 3D scene ownership checkpoint

- Registered 3D rigid-body, single/compound collider and gravity-world components
  now feed `ScenePhysics3d`. Composed transforms supply world XYZ/quaternion poses;
  absent transforms use the body pose. Collider-only entities are static. Complete
  active batches validate before gravity/lifecycle/body mutation, and both drivers
  reject mixed 2D/3D ownership. Multiple active 3D world settings and moving
  Z-locked 3D bodies fail explicitly.
- Unchanged frames and transform/ancestor teleports retain velocity. Structural
  body/collider payload edits rebuild from authored motion. Removed/inactive/
  despawned bodies, including inherited inactivity, release ownership. All non-
  static kinds write back through parent space after solving, preserving scale
  and component payloads. Parents precede children even with newer parent IDs;
  equivalent quaternion signs do not cause external-motion detection.
- Twelve new native regressions exercise XYZ/rotation, scale retention, solids/
  compound sensors/events, gravity override/restoration, invalid-batch atomicity,
  lifecycle/edits/targets, rotated/scaled/newer parents and checked default
  add/edit/undo/redo/save/reopen with unknown fields preserved. Scoped preflight,
  warning-denied Clippy and all-target/all-feature WASM compile pass. Component
  catalogue regeneration and all eleven catalogue tests pass. All 177 physics,
  487 scene, 381 Decay/platformer and 11 catalogue tests pass (1,056 total).
  Warning-denied native game/editor all-target/all-feature checks also pass.
- Collider dimensions/offsets remain world units; visual transform scale does
  not resize collision geometry, now tracked as an absent capability in parity.
  This is another prerequisite for Causeway's resident/edited voxel collision
  proof. Game/editor hosts still run the 2D driver; native inspector/Play,
  typed Decay Vec3/events and native/browser voxel/game proof remain open.
  3D acceptance and final integration remain unchecked. Prior head `3fd9869a`
  is green in CI; this checkpoint needs verification. Continue with shared
  game/editor 3D stepping and authoring, then typed scripting and voxel proof.


## Shared 3D host checkpoint

- Shared native/browser game sessions and native editor Play step 2D, then 3D,
  with the same fixed duration before scripts. Each driver validates its own
  batch; there is no cross-dimension transaction. Fresh Play, Stop and scene
  replacement reset both editor solvers; pause/resume retains runtime state.
- Added the three 3D components to the editor Physics menu. The prior head's only
  failed CI job reported missing family/glyph entries for precisely those types;
  the complete affected editor suite now passes locally.
- Two native shared-session regressions exercise XYZ motion, landing on an actual
  solid cuboid and disabled/reactivated bodies. A read-only Decay observer watches
  solved XYZ transforms; typed preflight has no errors or runtime reminders.
- Scoped preflight passes all 655 game/editor tests, warning-denied all-target/
  all-feature checks, formatting and file-size gates. Warning-denied Clippy and
  game WASM library check/build pass. No dependency, schema or Decay host call
  changed; no generated catalogue update is required.
- An exported scratch fixture with the rebuilt WASM host passes the standard real
  Chromium smoke checks: scene/script/texture fetched, WebGPU active, visible
  textured geometry (21 colours), and the observer confirms XYZ motion/landing.
  The same host reaches the normal platformer goal with five coins, no falls,
  eight jumps and 11.533 simulated seconds, fetching 27 assets.
- Native editor visual review remains unverified. In this headless environment,
  the default/Vulkan adapter spins before opening a window, GL has no compatible
  surface and Chrome's SwiftShader ICD crashes on native startup. Owned processes
  were stopped. This is a review limitation, not evidence that Play is correct.
- This is general host plumbing toward Causeway's voxel proof, not a completed
  game capability. Native authoring/Play/replay, typed Decay Vec3 controls/events,
  resident/edited voxel collision and final integration remain open. 3D acceptance
  remains unchecked; CI must verify this new head.


## Typed 3D controls and events checkpoint

- Added the independent typed `Physics3d` namespace without changing the 2D
  `Physics` API. Copied Vec3 linear/angular velocities, velocity setters and
  dynamic impulses use active synchronized bodies. Finite f32-range input and
  body kinds validate before mutation; rotation locking retains zero angular
  velocity. Live controls leave authored motion unchanged and survive ordinary
  scene synchronization; structural rebuilds restore authored settings.
- Collision started/stopped and sensor entered/exited return copied sorted unique
  other-entity lists from the last successful 3D step. Every script sees the same
  non-draining snapshot; inactive/despawned references are filtered. Shared
  native/browser sessions and editor Play supply separate 2D and 3D context.
- Six new bridge regressions exercise vectors, kinds, invalid input/arity/context,
  stale/inactive/unsynchronized bodies, locking, subsequent scene steps and all
  four event queries. Two scripts exercise independent copied sensor lists. A new
  shared-session Decay driver exercises actual Vec3 motion and solid landing.
- Scoped preflight passes all 1,006 game/Decay/editor tests, warning-denied
  all-target/all-feature checks, formatting, size and typed-script gates. Clippy,
  Decay all-target/all-feature WASM and game WASM library check/build pass.
  Regenerated API JSON/Markdown and all 11 catalogue tests pass. The full native
  suite found a missing scripting-contract call table; all nine entries were
  added before the passing rerun. No dependency or scene schema changed.
- Rebuilt exported-host Chromium smoke confirms velocity/impulse controls,
  rotation locking, copied values and an actual solid collision landing event,
  with WebGPU, scene/script/texture delivery and visible geometry checks intact.
  The isolated normal platformer goal run passes with five coins, no falls,
  seven jumps and 11.417 simulated seconds (27 assets). An earlier run overlapping
  native compilation fell; isolation passed without changing gameplay or checks.
- Spawn-to-synchronization queuing remains absent, explicitly tracked in parity;
  3D controls called before body materialization fail. This is general scripting
  plumbing toward Causeway, not completed voxel/game proof. Native inspector/
  Play/replay, typed 3D queries, occupied/resident/edited voxel collision and final
  integration remain open. 3D acceptance stays unchecked; CI must verify this head.


## Typed 3D ray and sphere-query checkpoint

- `Physics3d.raycast`, `overlap_sphere` and `cast_sphere` query indexed synchronized
  geometry without advancing physics. Rays/casts return copied optional
  `RayHit3d` values with entity, world Vec3 point/normal and world-unit distance;
  overlaps return copied sorted unique entity lists. The typed exclusion is
  `Entity?`, rather than requiring a fake handle for no exclusion.
- Queries validate finite f32-range vectors/scalars, positive radii, non-negative
  travel, nonzero normalized direction and finite endpoints. Membership masks,
  sensor opt-in and whole-entity exclusion retain engine semantics. Inactive/
  despawned geometry is filtered even before the next synchronization. Inside/on
  and initial-overlap hits have zero distance/normal; exact ties retain entity/
  piece ordering. Extreme finite geometry numerical limitations remain open.
- Six bridge regressions exercise XYZ snapshots, inclusive endpoints/misses,
  direction normalization, sensors/masks/exclusion, inactive/despawned geometry,
  compound uniqueness/exclusion, sphere travel/initial overlaps, invalid input,
  missing context and copied optional/list values through a real typed script.
  The shared-session Decay driver queries its actual landing floor with all
  three calls and checks matching entity, point, normal and sweep travel.
- Scoped preflight plus native host checks/tests pass 1,012 unique game/Decay/
  editor tests. Warning-denied all-target/all-feature Clippy passes for all
  three hosts/bridge crates; typed script preflight, formatting and size gates
  pass. Decay all-target/all-feature WASM and rebuilt game WASM host pass.
  API JSON/Markdown regenerate and all 11 catalogue tests pass. No dependency,
  scene component or 2D API changed.
- Real Chromium exported-host smoke confirms the queried solid landing with
  WebGPU, scene/script/texture delivery and visible geometry checks intact.
  The sequential normal platformer goal run passes with five coins, no falls,
  seven jumps and 11.433 simulated seconds, fetching 27 assets.
- This is general query scripting toward Causeway, not occupied/resident/edited
  voxel/game proof. Typed rotated box/capsule probes, layer-name lookup,
  spawn-window controls, native 3D inspector/Play/replay and final integration
  remain open. 3D acceptance remains unchecked. Prior head `385b1421` has no
  reported failures, with test/render captures still running; CI must verify
  this new checkpoint.


## Browser goal-player repair checkpoint

- Full CI triage on `aac1a6a4` found one failure: the browser keyboard player
  jumped over the flag, then repeatedly jumped against the wall beyond it.
  The remaining engine, typed preflight, native tests/render, Clippy, WASM,
  packaging and site checks passed.
- The terrain-reading browser player now reads the goal position and brakes
  near the flag, allowing airborne approaches to land on its actual sensor.
  Gameplay, physics, the read-only observer and the win/coin/no-fall assertions
  are unchanged. A Node regression checks the airborne input sequence and
  waits for an observed win; the browser CI job runs it before captures.
- Scoped preflight and the Node regression pass. Real exported-host Chromium
  runs pass with five coins and no falls, including 40 ms input polling.
  WebGPU, asset delivery and visible-render checks remain intact.
  This repairs platformer proof automation; 3D acceptance and final integration
  remain open, and the new head still requires CI verification.


## Named 3D query-mask checkpoint

- Typed `Physics3d.layer(name)` and `mask(names)` read current active authored
  3D world labels without stepping, independently of 2D. The first 32 labels
  map to mask bits; duplicate labels select the first nonempty matching name,
  repeated requests combine with OR and an empty list returns zero. Unknown
  names, invalid arguments/settings, multiple active 3D worlds and absent
  host context fail explicitly.
- Three bridge regressions exercise full-u32 bits, duplicate/empty labels,
  authoring/inactivity, dimension isolation and query selection. Typed query
  and shared-session scripts now use named masks. Rebuilt Chromium confirms
  actual queried landing with WebGPU, delivery and visible geometry intact.
- Scoped preflight passes 880 native tests; all 11 catalogue tests, generated
  API artifacts, warning-denied Clippy, typed scripts, native editor check,
  WASM checks and game WASM build pass. No dependency, component schema,
  2D surface or gameplay behavior changed. This is scripting integration toward
  Causeway; typed rotated probes, spawn-window controls, native editor and
  occupied/resident/edited voxel proof remain open.
- Manual CI on the browser-repair head exposed an existing checkout expression
  that selected depth 2 when depth 0 was intended, leaving `origin/main` absent.
  The conditional now keeps full history for manual Decay preflight and depth
  2 for other runs. Final-head CI must verify this permanent workflow repair
  and the named-mask slice. 3D acceptance and final integration stay unchecked.


## Typed rotated 3D probe checkpoint

- `Physics3d.overlap_box`/`cast_box` and `overlap_capsule`/`cast_capsule` use
  indexed active geometry and the ray/sphere filters and copied result contracts.
  Orientation takes a finite nonzero Vec3 axis and finite radians with the
  right-hand rule; f64 normalization precedes quaternion conversion. Identity
  uses angle zero and sweeps keep orientation fixed. Box half-extents are positive;
  local-Y capsule straight-segment half-height is non-negative and radius positive,
  with zero half-height defining a sphere. All inputs fit engine f32 range.
- Five bridge regressions exercise changed geometry around X/Z, angle direction,
  filters, normalized travel, initial overlaps, invalid values/arity/context and
  zero-height/tiny/large-axis validity. Typed and shared-session scripts call
  all four probes against actual geometry; exported Chromium verifies landing
  with WebGPU, asset delivery and visible geometry checks intact.
- Scoped preflight passes 398 native tests, both typed scripts have zero errors/
  reminders, and all 11 catalogue tests, regenerated APIs, warning-denied Clippy,
  native editor check, Decay WASM checks and game WASM build pass.
  No dependency, engine query API, component schema, 2D surface or game rules changed.
- This is scripting integration toward Causeway. Spawn-window controls, native
  inspector/Play/replay and occupied/resident/edited voxel/game proof remain open.
  3D acceptance and final integration stay unchecked. The named-mask head has
  passed browser/Decay preflight and other completed CI gates, with test/render
  captures still running; the new probe head requires its own verification.


## Ordered 3D spawn-control foundation checkpoint

- Standalone `BodyControl3d` velocity/angular-velocity setters and impulses queue
  for unregistered bodies after finite-value and authored-kind validation.
  Insertion revalidates the complete request and replays per-body call order
  after compound mass is known. Explicit pending getters return the last setter;
  ordinary live reads still require materialization. Rotation locks apply during
  replay, removal cancels pending input and successful synchronization expires
  unresolved requests.
- Scene prevalidation includes pending kind conflicts before any gravity,
  removal or rebuild. Failed validation retains valid requests for a corrected
  retry. Reconciliation preserves initial spawn queues and replays before solving.
  Four engine and three scene regressions exercise order/mass, snapshots, locks,
  rejection/retry, authored-state preservation and lifecycle expiry.
- Scoped preflight passes 671 native tests; final focused regressions, Clippy,
  all 11 catalogue tests, formatting/size gates, changed-crate WASM checks and
  game WASM build pass. Generated artifacts remain unchanged. Rebuilt Chromium
  landing/query smoke passes standard WebGPU/asset/pixel checks; queued replay
  is proven natively, not in the browser yet.
- This is an engine foundation toward Causeway. Typed Decay spawn-window wiring
  and real prefab/session proof follow; native inspector/Play/replay and voxel
  collision/game proof remain open. No dependency, host surface, component schema
  or 2D behavior changed. Prior head `888ce871` is green in CI; verify the new
  checkpoint before treating it as green. 3D acceptance/final integration stay open.

### Typed 3D spawn controls

Existing Physics3d velocity/angular setters and impulses now queue for active
valid authored bodies with nonempty colliders before synchronization. Reads copy
last setters or authored starting motion; impulses resolve only after mass is
known, and rotation locking returns zero. Four bridge regressions cover real
typed prefab spawning, call order, authored-state preservation, locks/kinds and
invalid authoring. This is a general engine/Decay foundation toward Causeway;
a shared native-session regression and exported Chromium fixture verify actual
spawn replay and movement. Game/voxel/editor proof remains open. The 3D acceptance item remains unchecked.

### Voxel collision geometry prerequisite

The voxel crate now compiles occupied section cells into exact disjoint boxes
through a caller-supplied collision policy. Full cubes merge deterministically;
slabs/posts stay exact, air/noncolliding blocks are omitted and invalid shapes
return typed errors. Five regressions prove exact coverage, worst-case output
bounds and edited negative residency round trips. All 50 voxel tests, scoped
preflight, warning-denied Clippy and all-target/all-feature WASM checks pass.
No physics/renderer dependency, component or host surface was introduced. This
is a general foundation toward Causeway; actual scene-resident collision,
revision caches, bounded frame updates and game/editor/Decay/browser proof
remain absent. Both major acceptance items stay unchecked.

### Replaceable 3D static geometry prerequisite

The solver can now validate, replace and remove keyed static collider groups
under one real entity without rebuilding its body or unaffected collider handles.
Queries refresh immediately with canonical piece ordering and owner identity;
invalid inputs/kinds/pending conflicts reject before mutation. Six regressions
prove handle preservation, filters, lifetime and actual landing/contact followed
by falling after removal. Scoped preflight passes 187 native tests; warning-denied
Clippy and all-target/all-feature WASM checks pass. This general primitive is
needed for Causeway's resident/edited voxel collision without artificial section
entities. Scene policy/transform/revision caches, residency and budgets, and
actual game/editor/Decay/browser integration remain absent. Acceptance stays
unchecked. No dependencies, components or host APIs changed.

### Resident voxel collision snapshot seam

`SceneVoxelCollision3d` connects supplied complete resident sections and explicit
shape policy to static owner groups. It caches occupancy/policy revisions, reuses
boxes for scale/settings changes, moves pose-only updates, releases section/world
lifecycle and prevalidates the whole bounded snapshot before committing. Seven
native regressions prove cache/transform/lifetime behavior, all four budget gates,
late errors/retry and actual landing/owner contact followed by falling through an
edited voxel hole. Scoped preflight passes 497 scene tests, warning-denied Clippy
and all-target/all-feature WASM checks pass. No dependencies, component or host
surface changed. This is the native scene seam toward Causeway: authored terrain/
block policy, residency production and shared-session/editor/game/Decay/browser
wiring remain open. Both major acceptance items stay unchecked.

### Authored voxel collision checkpoint

`sindri.physics3d.voxel_collider` turns an authored `sindri.voxel_world` into
static 3D collision. Block sets gained an explicit `collides` flag (default
true; Causeway's water is `false`) and each colliding block contributes its own
bounds; `supports`/`walkable` are not consulted. `VoxelGround` produces whole
sections with edits applied, a content revision per section and a policy
revision. Residency follows planned dynamic bodies (farthest piece plus twice
the step's velocity/gravity travel, plus a margin in voxels) in the world's
composed voxel space, independent of the camera. `ScenePhysics3d` plans the
voxel snapshot beside the body batch and validates both before committing
either; a voxel owner may not also carry a 3D body or collider. The shared
game session and editor Play pass their bound block sets; the cache resets with
the solver.

Causeway uses it: taking a laid block back spawns a loose-block prefab that pops
out of the emptied cell, lands on the world's voxels and returns to the stock
only when `Physics3d.collision_started` reports the Floor. The native play test
drives the real gestures through the camera and observes the pop, the landing
before the give-up time and the stock returning. Six scene regressions cover
landing/owner events, raycasts against the owner, an edited shaft under a body
that has slept, distant residency, non-colliding water and slab bounds, release
on component removal, and atomic rejection of missing worlds, conflicting owners,
unbound block sets and over-budget reaches. A browser fixture
(`prepare-causeway-voxels.py`, `SINDRI_VOXEL_LANDING=1`) drops the same prefab
over the wanderer in exported Causeway and requires a ray hit on the Floor and a
landing. Editor-run proof of 3D authoring/Play remains; both acceptance items
stay unchecked.

