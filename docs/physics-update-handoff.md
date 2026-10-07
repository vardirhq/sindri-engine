# Physics update: recovery handoff

## Start here

Continue **PR #497**, https://github.com/vardirhq/sindri-engine/pull/497,
on branch `codex/physics-update` in `vardirhq/sindri-engine`.
Keep all work in this one draft PR. Do not merge it or replace it with multiple
PRs. Read `AGENTS.md` and `CLAUDE.md` freshly before editing, then the relevant
subsystem and Decay guidance.

The user authorized every item in `docs/physics-update.md`, including the
documentation issues. Deliver checked incremental pushes: each feature is
one push or less, with smaller checkpoints preferred when useful. The user
explicitly stopped the previous session because it was freezing and asked for
a pushed handoff. Do not ask them to authorize the scope again.

## Current continuation checkpoint

Typed `Physics3d` setters/impulses now use the ordered engine queue for active
unmaterialized bodies after validating authored body/nonempty colliders, composed
pose, dimension ownership and moving depth locks. Live controls retain runtime
semantics without reparsing edited authoring. Pending reads copy the last setter
or authored start; impulses resolve only once mass is known, and locked angular
reads stay zero. Public signatures and dependencies are unchanged; reference
prose and generated catalogue describe the new timing contract.

Four bridge regressions exercise real typed prefab spawning, call order, copied
pending reads, preservation of authoring, starting velocities, locks/kinds and
malformed/missing/conflicting authoring. A shared native-session regression proves
spawn-before-solve timing and first-step movement. A rebuilt exported Chromium
fixture loads the real prefab and reports `3D typed prefab spawn replay verified`
only after observing copied reads and live movement/velocity. WebGPU, eight
fetched assets and 20 visible colors pass the existing smoke checks.

Scoped preflight (Decay/game all-target/all-feature checks and full tests),
warning-denied Clippy, Decay all-target/all-feature WASM check, game WASM build,
format/file-size gates and all 11 catalogue tests pass (403 native tests plus
11 catalogue tests). The game library and changed 3D integration test also pass
WASM checks. A broader game `--all-targets` WASM experiment fails in existing
`the_beacon_lights` / `the_game_is_played` tests, which import native-only asset
loaders and `Session::new`; it is not the CI WASM contract, which checks workspace
libraries/binaries. Do not claim every game test target builds for WASM. Logs:
`/tmp/sindri-host3d-game-wasm{,-scoped}.log`. Artifacts:
`/tmp/sindri-host3d-{preflight,clippy,wasm,wasm-build,generate,catalogue,export,browser}.log`,
`/tmp/sindri-host3d-browser.png`, export `/tmp/sindri-host3d-export`.
Prior head `16bed816` is green in CI (run `37664260537`). Check the current pushed head's CI before claiming readiness.

Next close the actual game/voxel proof: Causeway occupied/resident/edited/bounded
voxel collision and usable 3D authoring/Play/replay. Primitive prefab/session/
browser fixtures prove the host contract, not game integration. The 3D acceptance
and final integration items stay unchecked; keep PR #497 draft. The earlier
foundation notes below describe the preceding checkpoint, before typed wiring.


The standalone 3D spawn-control foundation is now implemented. `BodyControl3d`
has linear/angular velocity setters and impulses; `remember_control` validates
finite values and caller-supplied authored kind for unregistered bodies, while
`apply_control` acts on live bodies. Insertion revalidates all requests and
replays per-body call order only after actual compound mass is known. Explicit
`pending_linear_velocity`/`pending_angular_velocity` getters return the last
matching setter, without resolving impulses; ordinary velocity reads still
require live bodies. Rotation locking applies during replay. Removal cancels
requests even without a live body, and `finish_synchronize` expires unresolved
requests after a successful complete insertion batch.

`ScenePhysics3d` includes pending requests in complete-batch prevalidation,
retains them after failure for a corrected retry, preserves initial spawn queues
while reconciling and expires unresolved requests before solving. Four engine
and three scene regressions cover order/mass, reads, locks, invalid requests,
atomic kind-conflict rejection/retry, pre-solve movement, authored-state
preservation and removal/inactive/colliderless expiry. Scoped preflight passes
671 native physics/scene tests; the final focused seven tests also pass.
Warning-denied all-target/all-feature Clippy, formatting/size gates, both changed
crates' WASM checks and rebuilt game WASM host pass. All 11 catalogue tests pass;
regeneration leaves generated artifacts unchanged (no host/schema change).
The rebuilt Chromium landing/query smoke passes with WebGPU, four assets and
21 visible colors; this is a regression, not browser queue-replay proof.
Artifacts: `/tmp/sindri-pending3d-{preflight,focused,scene-focused,clippy,wasm-check,wasm-build,generate,catalogue,export,browser}.log`,
`/tmp/sindri-pending3d-browser.png`; export `/tmp/sindri-pending3d-export`.

Next wire existing typed `Physics3d` setters/impulses to this engine queue for
active, validated authored 3D bodies that have not synchronized. Decide and test
pre-materialization reads using explicit pending setters/authored starts, with
rotation locking, without assuming impulse effects before mass exists. Prove
real prefab spawning through typed scripts and shared native/browser sessions.
Decay writes still fail before synchronization in this checkpoint; the parity
row is Engine partial and Decay/editor/proof absent. This is an engine foundation
toward Causeway; native inspector/Play/replay and occupied/resident/edited voxel
collision/game proof remain open. Prior head `888ce871` is green in CI. Verify
the new head, keep 3D acceptance/final integration unchecked and keep PR #497 draft.

Typed `Physics3d.overlap_box`, `cast_box`, `overlap_capsule` and `cast_capsule`
now use indexed active geometry and existing copied hit/list contracts. Arbitrary
orientation takes a nonzero Vec3 rotation axis and finite radians using the
right-hand rule. The host normalizes axes in f64 before quaternion conversion;
identity uses angle zero, and sweep orientation remains fixed. Boxes have positive
Vec3 half-extents; Y capsules have non-negative straight-segment half-height and
positive radius (zero half-height is a sphere). All inputs fit engine f32 range.
No dependency, engine query API, component schema, 2D surface or game rules changed.

Five bridge regressions exercise rotation-dependent hits/misses around X/Z,
right-hand angle direction, sensor/exclusion filtering, normalized sweep travel,
initial overlaps, invalid shapes/axes/angles/directions/arity/context and valid
zero-height/tiny/large-axis probes. Typed query and shared-session drivers exercise
all four calls on actual geometry. Scoped preflight passes 398 native game/Decay
tests and both scripts have zero errors/reminders. All 11 catalogue tests,
warning-denied Clippy, formatting/size gates, native editor check, all-target/
all-feature Decay WASM and rebuilt game WASM host pass. API JSON/Markdown regenerate.
Rebuilt exported-host Chromium verifies all four probes and solid landing with
WebGPU, four delivered assets and 21 visible colors.
Artifacts: `/tmp/sindri-probes3d-{preflight,clippy,catalogue,generate,wasm-check,wasm-build,editor-check,export,browser}.log`,
`/tmp/sindri-probes3d-browser.png`; export `/tmp/sindri-probes3d-export`.

This is query scripting integration toward Causeway, whose occupied/resident/
edited voxel collision and game proof remain open. Continue with 3D spawn-window
controls, native inspector/Play/replay evidence and voxel collision proof.
The named-mask head `25e0d134` has passed browser smoke, Decay preflight,
Clippy, WASM, formatting and baker CI; test/render captures are still running.
Verify final-head CI and keep 3D acceptance/final integration unchecked and PR draft.

Typed `Physics3d.layer(name)` and `mask(names)` now read the active authored
3D world's first 32 labels independently of 2D. Unknown/empty names, wrong
arguments, malformed or multiple active settings and missing 3D context fail;
duplicate labels select the first bit, repeated requests OR together and an
empty list returns zero. Inactive settings are ignored and current authored
edits are visible without stepping. Three bridge regressions exercise real query
filtering, dimension isolation, authoring/inactivity, bit 31 and invalid inputs.
Both the native typed query script and shared-session driver now query with
named masks. Rebuilt exported-host Chromium confirms queried solid landing
with WebGPU, four delivered assets and 21 visible colors.

Scoped preflight passes 880 native game/Decay/scene tests, both typed scripts
have zero errors/reminders, and warning-denied all-target/all-feature checks and
Clippy pass. All 11 catalogue tests pass and API JSON/Markdown regenerate.
Decay and scene WASM checks, the game WASM build and native editor check pass.
Artifacts: `/tmp/sindri-layer3d-{preflight,clippy,catalogue,generate,wasm-check,scene-wasm,wasm-build,editor-check,export,browser}.log`,
`/tmp/sindri-layer3d-browser.png`; export `/tmp/sindri-layer3d-export`.
No dependency, body/component schema, 2D surface or gameplay behavior changed.
This is scripting integration toward Causeway; voxel/game proof remains open.
Continue with typed rotated box/capsule probes, spawn-window controls and the
remaining native editor and occupied/resident/edited voxel proof below.

The browser-repair head had no automatic CI checks attached, so CI was started
through its existing manual trigger (run `37656772383`). It exposed a checkout
expression bug: `condition && 0 || 2` selects 2 even when the condition is true,
leaving `origin/main` unavailable to manual Decay preflight. The expression now
uses `condition != workflow_dispatch && 2 || 0`, which keeps full history for
manual runs. This is a permanent repair to the existing CI contract, not a
self-modifying workflow. Final-head CI must verify both changes; keep the PR draft.

The latest typed-query head `aac1a6a4` passed every CI job except the browser
smoke. Full failure triage found the keyboard player jumped over the platformer
flag sensor and ran into the tall wall beyond it (four coins, no falls, stuck
at x=62.691). The browser player now reads the goal position from the exported
scene and releases horizontal input near the flag to land there. It still
requires the observer's real win, at least three coins and no falls; no game
scripts, physics behavior or smoke thresholds changed. A Node input-sequence
regression verifies airborne braking, waiting for observed victory and input
cleanup, and runs before browser captures in CI. Local scoped preflight,
Node regression and real Chromium runs pass. The final normal run braked airborne at x=60.026/y=6.735 and reached the flag
with five coins and no falls in 11.367 simulated seconds; a run with 40 ms input polling also collected five
coins without falling (11.55 simulated seconds). Artifacts:
`/tmp/sindri-browser-goal-{preflight,brake,delayed,final}.log` and matching
browser captures. CI must verify this repair before treating the head as green.
Continue the remaining 3D feature work listed below on this same draft PR.

Typed `Physics3d.raycast`, `overlap_sphere` and `cast_sphere` now use the indexed
3D world without advancing physics. Closest hits are copied optional `RayHit3d`
values with entity, world Vec3 point/normal and distance; overlaps are sorted
unique copied entity lists. Exclusion is typed `Entity?`. Finite f32-range inputs,
positive radii, non-negative travel, nonzero normalized direction and finite
endpoints validate. Membership masks, sensor opt-in and whole-entity exclusion
retain engine semantics. Active-entity filtering hides inactive/despawned
geometry even before synchronization. Exact ties and zero-distance inside/initial
hits retain engine semantics. Extreme geometry numerical limits remain open.

Six new bridge regressions exercise snapshots, boundary/miss/inside hits,
normalization, filtering, compound uniqueness/exclusion, sphere travel/initial
overlaps, invalid values/arity/context and copied optional/list values in typed
scripts. The shared-session Decay driver queries its actual landing floor with
all three calls, checking entity, point, normal and sweep travel.
Scoped preflight plus native game/editor checks and tests pass 1,012 unique
bridge/game/editor tests. Warning-denied Clippy, typed preflight, formatting,
file-size, Decay all-target/all-feature WASM and game WASM build pass. Generated
API JSON/Markdown and all 11 catalogue tests pass. No dependency, scene component
or 2D API changed.

Rebuilt exported-host Chromium confirms the queried solid landing with standard
WebGPU/asset/pixel checks intact. The sequential normal platformer run reaches
the flag with five coins, no falls, seven jumps and 11.433 simulated seconds
(27 assets). Browser input runs were isolated from native compilation.
Artifacts: `/tmp/sindri-query3d-{preflight,clippy,focused,catalogue,wasm-build,wasm-check,host-check,host-tests,browser,platformer-browser}.log`,
`/tmp/sindri-query3d-browser.png`, `/tmp/sindri-query3d-platformer.png`.
Scratch project/export: `/tmp/sindri-query3d-{project,export}`. Observer harness:
`/tmp/sindri-physics-tools/browser/query3d-smoke.mjs` (standard checks plus the
completion marker and bounded simulation wait). Permanent driver:
`game/tests/physics3d_driver.decay`; bridge query regression:
`crates/sindri-decay/tests/physics3d_queries.rs` and `physics3d_query.decay`.

Typed velocity/angular-velocity reads/setters, dynamic impulses and four copied
non-draining collision/sensor event queries already have separate 3D context in
shared native/browser sessions and editor Play. Controls require active
synchronized bodies and leave authored motion unchanged; structural rebuilds
restore it. Spawn-window queuing remains absent and is explicitly tracked.

Continue with typed rotated box/capsule probes and layer-name lookup, 3D
spawn-window controls, native 3D inspector/Play/replay evidence and Causeway
occupied/resident/edited voxel collision on native/browser. Native editor
interaction remains unverified: the previous headless adapter review hung or
failed before a usable window. Owned processes are stopped; retry on a usable
graphics session. Automatic collider scaling is absent (world-unit dimensions/
offsets). 3D acceptance and final integration stay unchecked. Prior head
`385b1421` had no reported failures, with test/render captures still running;
CI must verify the latest checkpoint.

The 3D query-only BVH now selects rays, overlaps and shape casts at current
poses. Insert/remove/teleport update immediately; non-static pieces refresh after
solving, and position targets remain pending. Four regressions compare exhaustive
candidates, lifecycle/rotation/solved motion and conservative overflow retention.
Sparse worlds up to 10,000 pieces select one local piece/entity; sampled XYZ rays
visit 26–34 nodes. All 177 physics tests, scoped preflight, warning-denied Clippy
and all-target/all-feature WASM pass. No dependency/API/host/component/2D change.
The overflow regression exposed backend NaN cast results at near-maximum finite
coordinates/extents; numerical failure reporting is tracked as absent in parity.
Scene synchronization is now covered by the latest checkpoint. Continue with
game/editor host integration, typed Decay access and Causeway resident/edited
voxel collision proof on native/browser. 3D acceptance and final integration remain unchecked.

Standalone 3D rays, overlaps and fixed-orientation shape casts now use current
body/local quaternion poses. Eight regressions cover XYZ/all shapes, rotations,
filtering, whole-entity predicates, sorted unique overlaps, exact entity/piece
ties, immediate/solved/pending poses, initial overlap and extreme/invalid inputs.
All 173 physics tests, scoped preflight, warning-denied Clippy and all-target/
all-feature WASM compilation pass. Query acceleration is now covered by the
latest checkpoint; scene/editor/Decay and the voxel game proof remain open.
No host, component, dependency or 2D behavior changed. CI must verify this head.

The standalone 3D foundation now owns fixed-step bodies, compound box/sphere/Y
capsule colliders, XYZ velocities/impulses, position-kinematic targets and
entity collision/sensor transitions. Quaternions use `[x, y, z, w]`; complete
requests validate before insertion or mutation, and accepted near-unit rotations
normalize before reaching the backend. Rotation locking clears initial and live
angular velocity. Backend handles remain private; the shared event-kind reexport
preserves the 2D API. No dependency, component registration or host call changed.
All 165 native physics tests and 856 scene/Decay/platformer tests passed, alongside
warning-denied Clippy, all-target/all-feature WASM and scoped preflight.
Prior head `02b99c8e` is green in CI; this checkpoint needs its own CI verification.
Continue with scene lifecycle/transform/gravity synchronization,
typed Decay Vec3/events and checked editor authoring/Play. Complete the Causeway
voxel-terrain proof with residency, edited collision and bounded work on native
and browser before checking 3D acceptance. Final integration remains open.

The 2D joint checklist is complete. Character movement now has checked
engine-only sweep/slide, ground-probe and optional grounded-snap foundations: see
`docs/character-movement.md` for the contract and remaining slices. The read-only
result has skin, bounded iterations,
ordered hits and initial-penetration/budget flags. Ground probes classify nearest
support against up/slope limits and detect zero-travel support with a contact
query before sweeping. Composed movement reports support after sliding and can
snap downward onto walkable support; snap defaults off and ascending requests
never snap or report grounded. The same slope limit now prevents steep contacts
from generating unrequested rise; explicit jumps and steep descent remain possible,
and downhill snapping accepts only walkable support. Gameplay retains prior state.
Optional steps now require starting support, clear lift/forward sweeps, better
progress and a walkable landing, with exact fallback. Movement normals are refined
from impact contacts to avoid flat-box hops; ordinary shape casts are unchanged.
Opt-in platform snapshots now verify old support and add swept synchronized
translation/rotation-point carry before character movement. Wall clipping and
ceiling crush are explicit; hosts must advance snapshots once and avoid extra
parent/solver carry. Rotation follows a chord with fixed probe orientation.
One-way controller policy now applies support-side/cone filtering across all
phases; request-scoped drop-through cancels one-way support/carry while preserving
ordinary solids and sensor filtering. Hosts own duration; geometric queries and
dynamic-body drop timers retain their separate contracts.
One-way checkpoint validation passed 146 native physics tests, warning-denied
Clippy, all-target/all-feature WASM, eleven catalogue tests and size/format gates.
Prior carry head `c4309a77` is green in CI; the next pushed head needs verification.
Scene ownership now derives stationary kinematic controller bodies from a
validated Character 2D component and one solid collider, plus optional sensors.
Runtime requests queue displacement/snap and timed drop-through. Fresh support
is seeded before solving; controller movement uses current solved platform poses
and writes back once, with cache invalidation on lifecycle/parent/structural edits.
`for_scripts_with_characters` supplies disjoint queue/result borrows for the next
host slice. Controller motion reaches solver response/sensors at the next solve;
compound probes and swept triggers remain explicit gaps.
Scene ownership validation passed 475 native scene tests (24 controller
regressions), warning-denied Clippy, all-target/all-feature WASM, eleven catalogue
tests and regenerated component metadata. Prior one-way head `5597ce44` is green
in CI; the new pushed checkpoint requires CI verification.
The character editor metadata checkpoint `ccda1971` passed all CI checks.
Before adding the typed controller hosts, the Decay host boundary was split by
responsibility: `host/mod.rs` owns context/services and `host/access.rs` owns
runtime loading, storing, calling and entity/vector addressing. Both implementation
blocks moved unchanged; the parent is now 168 lines, leaving room for controller
context. Scoped preflight, warning-denied Clippy and all-target/all-feature WASM
checks passed. This is a refactoring checkpoint, not a new controller API or proof.
Typed controller access now offers `Physics.move_character(entity, Vec2, snap)`
and optional copied `Physics.character_motion(entity)` results. Character 2D
uses the scene timer through `Physics.drop_through`; dynamic bodies retain the
solver timer and physics-only host compatibility. `Characters2d` is a separate
optional `ScriptFrame`/host context, reborrowed per script; existing `Physics2d`
and service literals remain compatible. Shared game runtime, editor Play and
platformer harness supply the scene queue/result borrows.
Snapshots expose slide/ground/step/snap/carry vectors, flags and ordered copied
hits, filtering inactive/despawned references. Requests validate active actors,
settings, transform and body ownership; probe assembly validates at synchronization.
Null guards are required before accessing optional motion/ground. Invalid input
preserves queued requests; no snapshot edits change the cache.
Validation passed 1,023 native tests across Decay, game host, editor and platformer,
including ten controller host regressions and a shared-session run. All four
changed scripts passed typed preflight with zero errors/reminders. Warning-denied
Clippy, native checks, eleven catalogue tests/currentness, formatting/size and
WASM checks passed (Decay/platformer all targets; game excludes native-only tests).
The rebuilt browser host loaded a small exported fixture under
`/examples/character-api/`, fetched scene/script assets, configured WebGPU,
rendered its actor/floor and printed the verified movement/ground/copied-cache
observation from `game/tests/character_controller.decay`. Browser log/capture:
`/tmp/sindri-character-api-browser.log` and `-browser.png`; fixture/export:
`/tmp/sindri-character-api-project` and `-export`. The prior refactoring head
`3842bac5` passed all CI; the newly pushed API head requires verification.
The platformer adoption checkpoint now replaces its dynamic hero with authored
Character 2D and a ground-filtered capsule probe. Separate capsule/foot sensors
preserve pickups; the dynamic crate retains authored and typed CCD plus contact
impulse proof. Hero owns velocity/gravity in Decay and uses queued displacement
and cached support; respawn cancels movement/drop state. Three new gameplay tests
cover variable jump height, acceleration/braking and respawn, while the existing
flag/coin/no-fall, one-way and joint/material/crate tests remain passing.
Scoped preflight passed all 29 platformer tests and both changed scripts with
zero errors/reminders. Native warning-denied check/Clippy, all-target/all-feature
WASM, formatting and file-size gates passed. Chromium fetched 25 assets and drew
the exported game; keyboard right/jump standing/airborne captures were reviewed.
Artifacts: `/tmp/sindri-platformer-controller-{preflight,clippy,wasm,browser}.log`,
`/tmp/sindri-platformer-controller-{standing,jump,browser}.png`, export directory
`/tmp/sindri-platformer-controller-export`. The browser host binaries are from
`ce493275` (this checkpoint changes only game assets/tests/docs).
The terrain checkpoint adds two 0.25-unit stone risers and a roughly 20-degree
plank boardwalk using existing tile art/general rotated tilemap collision. Hero
keeps intended horizontal speed at obstacles so a clipped request does not starve
the next low-step attempt; ceiling hits still reset rising velocity. Four native
control tests walk both risers and climb/snap down the boardwalk without jumping,
then disable steps/lower the slope limit and block on the same terrain. The
camera test allows two seconds through the added terrain while preserving its
follow and bounds assertions. All 33 platformer tests pass, including the
no-fall goal/coin run, and typed Hero preflight reports zero errors/reminders.
Native warning-denied checks/Clippy, all-target/all-feature WASM, formatting/size,
native capture and Chromium export/input checks passed. Browser snapshots show
the hero stepping and on the boardwalk; delivery fetched 25 assets without
script/GPU errors. Logs/captures/export are under
`/tmp/sindri-platformer-terrain-*`; helper:
`/tmp/sindri-physics-tools/browser/platformer-terrain-smoke.mjs`.
The moving-platform checkpoint adds an authored one-way plank ferry across the
first gap, driven by ordinary Decay kinematic velocity. Hero adds no platform
motion. Four real-script gameplay tests ride across and back with fixed relative
pose and once-only solved carry/total translation, board with a running jump
from the normal spawn, leave/reacquire carry on jump/landing, and drop through
into the pit/normal respawn. All 37 platformer tests pass; Ferry typed preflight
has zero errors/reminders. Native warning-denied check/Clippy, all-target/all-feature
WASM, formatting/size and native capture passed.
Chromium exports the same assets with only the hero's initial pose moved aboard.
It fetched 26 assets and drew both riding directions and a keyboard jump without
script/GPU errors. These captures prove carry/input, not a browser run to the
goal. Logs/captures are `/tmp/sindri-platformer-ferry-*`; scratch project/export:
`/tmp/sindri-platformer-ferry-project` and `-export`; helper:
`/tmp/sindri-physics-tools/browser/platformer-ferry-smoke.mjs`. Browser host
binaries remain from `ce493275` because these slices change game assets only.
Native character-editor review is now complete at `da9a5328`, with a rebuilt
binary and scratch platformer on display :98. All settings were exercised,
including carry and up reversal; snap undo/redo and every saved field survived
reopen. Negative skin input retained its valid value. Removing/adding Character
2D through the Physics menu saved absent/default payloads, and undo restored
custom settings. Reopened Play runs/jumps the hero with read-only fields; Play
Save refuses and Stop/Save restores the pre-Play file hash. The shipped scene
was unchanged. Procedure/results: `docs/physics-character-editor-review.md`.
Captures: `/tmp/sindri-joint-review-character-*.png`; build/runtime logs and
observations: `/tmp/sindri-character-editor*`; scratch project:
`/tmp/sindri-character-editor-project`. Owned editor/Xvfb processes were stopped.
The browser goal checkpoint completes 2D character acceptance. A permanent CI
fixture copies the normal shipped platformer and adds a read-only Decay observer;
Playwright uses real keyboard events to reach the flag, requiring at least three
coins and no falls. Repeated local runs collected five coins and eight jumps.
The player waits for an observed jump release before the next press; releasing
and pressing in one browser frame previously lost the action edge. Source:
`scripts/browser/prepare-platformer.py`, `platformer-goal.mjs`, the smoke hook and
`games/platformer/tests/browser_goal.decay`. See the browser proof section in
`docs/character-movement.md` for the CI reproduction.
2D accelerated queries now cover rays, overlaps, shape casts and controller
penetration/sweep/ground phases. Probe bounds expand for skin/contact prediction;
step/snap/carry collisions share those paths. Historical support reconstructs
only the known platform entity's pieces instead of selecting its current bounds.
The query-only per-piece BVH updates immediately on insert/remove/teleport and
refreshes non-static pieces after solving. Position targets retain their pending
semantics; overflowing bounds retain the scan fallback. No dependency/API changed.
Four new controller regressions compare exhaustive candidates across shapes,
rotation, skin, arbitrary up and one-way/drop policy, retain zero-travel separated
skin support, reconstruct support moved 100 units away, and select one local
entity at 100/1,000/10,000 pieces. Native physics/scene/Decay/platformer and real
browser results are recorded in the latest physics-update checkpoint.
Continue with 3D physics/queries and voxel proof, followed by full integration.
Run browser smokes sequentially as CI does: the first goal attempt overlapped
another Chromium and a native build, fell and showed late jump positions. An
isolated previous-host baseline and indexed-host goal runs preserve the strict
coins/no-falls assertions. This observation is consistent with input scheduling
under load; do not weaken the gate or claim arbitrary-load reliability.
Gameplay snapshots and a 3D character controller remain absent. Inspect every
failed job on the current PR head before changing code if CI fails.

CCD, one-way platforms, **forces and rotation**, **contact snapshots**, and
**reusable physics materials** are implemented in this PR. The checklist in `physics-update.md` is current.

Forces/rotation add world force and torque, angular velocity/impulse and impulses
at world points. Forces accumulate for one step; spawn-window requests replay in
order after mass calculation. Existing hosts with their own physics driver are
supported. Platformer's wind crate proves force/torque and input-driven launch
and rotation. CI was green on `2426d98e`, the previous branch head.

Contacts expose copied solid solver points, push normals towards the queried
body, normal/friction impulses and world force over the last fixed dt, ordered
deterministically. Sensors are excluded; sleeping support has zero new impulse.
Teleports/removal/rebuilds invalidate affected snapshots. Typed Decay filters
inactive/despawned others and returns empty before spawn synchronization.
Platformer grounds from solved support and flashes a child shape on crate
landings. The crate is a **tilemap**, so do not use `this.sprite.tint` on it.
Rapier disables contact clustering in 2D; read `solver_manifolds()` for impulses,
not a fixture that assumes clusters must be generated.

Contact validation: scoped native preflight passed 355 tests; all-target/all-
feature checks, typed checks for all three changed scripts, warning-denied
Clippy, scene and catalogue tests/currentness/completeness, and all-feature WASM
checks passed. Chromium export/load smoke passed on the rebuilt browser runtime,
fetching assets, running Decay without runtime errors and drawing the level.
CI is green on contact head `404b27ad`. Full browser gameplay interaction/editor Play inspection
and workspace integration remain at the end of the checklist.

Materials reuse `.profile` assets with type `physics_material`. Scene-side
resolution keeps strings out of backend colliders and validates coefficients
across hosts. `sindri.physics2d.material` applies to all an entity's pieces;
explicit override flags win, and empty profiles keep literals. Coefficient
updates preserve velocity/forces/joints. Editor async reload retains last-valid
profiles, save rejects invalid values, and export gathers/validates references.
Platformer's crate/planks share wood, with a plank restitution override and
rebound proof. Native project capture now expands prefabs before scene entry.

Material validation: final scoped preflight passed 1,175 native tests and checks;
warning-denied Clippy passed all six affected crates. Catalogue checks passed
11 tests and metadata was regenerated. All-feature WASM checks passed physics,
scene, export, game host and platformer. Chromium fetched the exported material
and drew the game; native delivery/play and Vulkan capture passed. Visual editor
interaction/full browser gameplay/workspace checks remain in final integration.
CI is green on material commit `936d1a95`.

A checked **scene-authored distance-joint foundation** now adds separate owner
entities, stable scene references, idempotent synchronization, constraint-only
edits, same-step endpoint rebuilds and removal/inactivity handling. Existing
`connect_distance` is preserved. Platformer's wind-driven hanging lantern proves
bounded movement and removal; its visible cord is authored by Decay.
Scoped preflight passed 1,115 native tests, all-target/all-feature checks and
zero-error/reminder typed script preflight. Four-crate Clippy, 11 catalogue tests,
WASM checks and rebuilt native/Chromium captures passed. See the checkpoint in
`physics-update.md` for limits and evidence. Engine CI passed on `e4554125`;
the site job was cancelled because GitHub could not acquire a hosted runner.

A checked **hinge and velocity-motor slice** now joins body-local anchors,
bounds relative angles and drives with capped torque. Settings edits preserve
body motion; unchanged frames keep solver ownership. Typed
`Physics.set_hinge_motor` writes the runtime component for next synchronization,
including before endpoints are built, preserving unknown fields. Platformer's
windmill uses Decay to reverse its physical axle every two seconds.
Scoped preflight passed 1,421 native tests, all-target/all-feature checks and
zero-error/reminder typed preflight. Five-crate Clippy, 11 catalogue tests,
WASM checks and rebuilt native/Chromium captures passed. CI still must verify
the pushed hinge head; CI is now green on `fd9916e7`.

A checked **slider and spring slice** now adds aligned local axes, signed travel
limits and force-capped linear drive, plus radial force-based springs/damping.
`Physics.set_slider_motor` and `Physics.set_spring` tune validated runtime payloads
for next synchronization, preserving unknown fields and rebuild behavior. The
platformer's bounded lantern trolley reverses through Decay and carries a light
whose spring rest length Decay retunes. Joint builders and scene synchronization
are split by responsibility; legacy distance and hinge behavior remains covered.
Final scoped preflight passed 1,432 native tests, all-target/all-feature checks
and typed preflight for all three scripts with zero errors/reminders. Five-crate
Clippy, 11 catalogue tests, WASM checks and rebuilt native/Chromium delivery and
captures passed. CI is green on slider/spring commit `338db9fc`.

A checked **spawned-prefab reference slice** now retains runtime-only local
identity in core, separately from saved scene IDs and editor links. Authored
joints resolve local siblings and their original top-level root inside one spawn;
missing/inactive endpoints never bind another instance or a scene entity. Scope
survives reparenting, cloning, command undo and assigning saved IDs, and dies with
the generation-checked root. Editor duplication clears runtime scope.
Platformer Decay places/removes a reusable powered windmill with V; two cycles
observe reversal, fixed anchors and cleanup. All four joint kinds are exercised
across repeated spawns, inactivity and removal; expanded nested paths are tested.
Final scoped preflight passed 1,385 native tests and all-target/all-feature checks
for core, scene, editor and platformer. Four-crate warning-denied Clippy, typed
setup-script preflight (zero errors/reminders), 11 catalogue tests and WASM checks
passed. Rebuilt Chromium export/load plus V placement/removal/replacement fetched
24 assets without runtime errors; native Vulkan capture passed. Both captures
were visually reviewed. CI is green on spawned-prefab commit `acbed858`.

A checked **joint suspension and distance tuning slice** now adds an enabled
flag to all four authored constraint kinds (omission means true). Typed
`Physics.joint_enabled`, `Physics.set_joint_enabled` and `Physics.set_distance`
read/patch validated runtime state for next fixed synchronization, retaining
endpoint bodies, settings and unknown fields. Suspended distance settings now
validate even before endpoints are available. Scene command undo/redo and
rebuilds cover all kinds. Platformer Decay reels with T and releases/reconnects
with L, hiding the cord while suspended or after owner removal. Its new run checks
free fall, reconnection and short/long lengths alongside existing goal regressions.
Physics signatures/reference prose are split for repository function/file limits.
Final scoped preflight passed 768 native tests and all-target/all-feature checks
for scene, Decay and platformer; typed lantern preflight had zero errors/reminders.
Three-crate warning-denied Clippy, 11 regenerated-catalogue checks and WASM checks
passed. Rebuilt Chromium delivery fetched 24 assets and exercised T/L plus the
spawned windmill without runtime errors; native Vulkan capture passed. Captures
of short/released/reconnected states and the native run were visually reviewed.
CI is green on joint-control head `195106ff`.

A **nested runtime root reference slice** preserves namespaced original root
aliases through library expansion, separately from component strings and saved
IDs. Canonical paths shadow aliases; ambiguous aliases fail before spawn mutation.
Decay passes the original prefab library into spawning. All four joint kinds
exercise renamed roots; core tests cover repeated expansion, scope isolation,
cloning, undo, canonical precedence and atomic ambiguity rejection. Platformer's
`windmill-kit.prefab` wraps the reusable powered mechanism, proven by its V
placement/removal regression. Final scoped preflight passed 1,082 native tests
and warning-denied all-target/all-feature checks for core, scene, Decay and
platformer. Four-crate Clippy, 11 catalogue tests, the 1,086-file size gate and
all-target/all-feature WASM checks passed. Typed windmill/setup checks reported
zero errors/reminders. Rebuilt Chromium delivery fetched 25 assets and exercised
nested placement/removal/replacement without runtime errors. Native Vulkan
capture passed; both captures were visually reviewed. CI must verify this slice.

A **placed-prefab root reference slice** retains original root aliases in
runtime-only `PrefabLink` metadata. Library-aware `LoadedScenes` variants load
original scenes through `add_scene_with`, prefixing aliases with scene namespaces.
Native/browser entry and later scene switches retain the prefab library. Editor
reload/undo carry aliases through link commands; duplication rebases them to the
copy's namespace. Saving prefab references and reopening regenerates aliases.
All four kinds cover nested roots, repeated instances, scene switching, inactivity
and removal. Editor reload/undo/redo and independent duplicate cleanup are tested.
Platformer's level windmill is placed from the same nested kit Decay spawns.
Final scoped preflight passed 1,424 native tests and all-target/all-feature checks
for core, scene, editor, the game host and platformer. Five-crate warning-denied
Clippy, 11 catalogue tests and the 1,089-file size gate passed. Typed windmill/setup
checks reported zero errors/reminders. WASM all-target/all-feature checks passed
core, scene and platformer; host-library WASM checks and the rebuilt browser host
passed. Chromium fetched 25 assets and exercised placed/spawned windmills plus V
removal/replacement without runtime errors. Native Vulkan capture passed; native
and browser captures were visually reviewed. CI passed on placed-reference head
`41073354`. Plain flattened documents discard aliases; opt-in saved-spawn
reference remapping is supplied by the next slice.

A **saved-spawn reference slice** adds opt-in registry-based
`World::to_scene_with_references(prefabs, components)` after stable ID assignment.
Registered entity fields, including dotted/list paths, become their local target's
saved ID. Empty references remain unbound; malformed/unresolved/unstable targets
fail. The live world and unknown payloads remain unchanged; no runtime identity
serializes. All four joint kinds reopen with isolated endpoints and exercise
qualified-ID precedence, inactivity/reactivation and removal. Platformer's
Decay-spawned nested windmill reopens, reverses its motor and keeps its world-space
axle fixed. Ordinary serializers remain verbatim and automatic editor/script save
integration is not supplied. Final scoped preflight passed 1,398 native tests and
warning-denied all-target/all-feature checks for core, scene, editor and platformer.
Four-crate Clippy, 11 catalogue tests and the 1,093-file size gate passed.
All-target/all-feature WASM checks passed core, scene and platformer, and the
generic browser host was rebuilt. Export passed; Chromium WebGPU fetched 25
assets from the saved/reopened fixture without runtime errors. Native Vulkan
capture passed; both final captures were visually reviewed. CI passed on saved-
reference head `9a1f1a07`.

An **entity-reference authoring slice** shares the engine resolver with native
inspector entity fields. Typeable scoped choices include names/IDs, explicit None
and visible missing/inactive diagnostics. Runtime prefabs keep local paths;
loaded-scene roots and other spawn instances remain isolated. Real picker-click
regressions retarget/clear all four joint kinds through checked commands and undo,
preserving unknown fields and constraints. Final scoped preflight passed 1,383
native tests and warning-denied all-target/all-feature checks for core, scene and
editor. All 18 platformer tests, three-crate Clippy, 11 catalogue tests and the
1,098-file size gate passed. All-target/all-feature WASM checks passed core and
scene; the generic browser host was rebuilt. Chromium WebGPU fetched 25 assets
without runtime errors and native Vulkan capture passed; both game captures were
visually reviewed. The native editor built and opened the platformer, but
interactive desktop picker inspection remains unverified. Actual widget pointer
interactions are covered in egui tests. CI passed on authoring head `3f129f34`.

A **typed endpoint retargeting slice** adds `Physics.set_joint_endpoints` for all
four authored joint kinds. Scoped entity handles become stable scene IDs or local
prefab paths; null clears either endpoint. Both references validate atomically,
including stale/unstable/scope/same-target rejection. Inactive targets suspend
until active. Settings, enabled state, unknown fields and body motion remain.
The shared inverse reference method also supplies inspector choices. Platformer
switches its lantern between two hooks with R, including while released.
Final scoped preflight passed 1,264 native tests and warning-denied all-target/
all-feature checks for core, Decay, editor and platformer. Typed lantern preflight
had zero errors/reminders. Four-crate Clippy, 11 regenerated catalogue tests and
the 1,100-file size gate passed. All-target/all-feature WASM checks passed core,
Decay and platformer; the generic browser host was rebuilt. Export and native
Vulkan capture passed. Chromium fetched 25 assets and exercised R switching,
suspended retargeting, reconnection and spawning without runtime errors. Native
and browser captures were visually reviewed. CI passed on typed endpoint head
`a3713835`.

A **typed joint removal slice** adds `Physics.remove_joint` for all four kinds.
Only the authored joint component is removed; next fixed synchronization releases
its constraint. Owner/other components and bodies/motion remain, with legacy
connections unaffected. Valid before body sync or while suspended; invalid
owners/physics fail atomically. Later controls fail until authored again.
Platformer Z cuts its lantern cord and ignores subsequent tether controls.
Final scoped preflight passed 332 native tests and warning-denied all-target/
all-feature checks for Decay and platformer. Typed lantern preflight had zero
errors/reminders. Two-crate Clippy, 11 regenerated catalogue tests and the
1,101-file size gate passed. All-target/all-feature WASM checks passed both
crates; the generic browser host was rebuilt. Export and native Vulkan capture
passed. Chromium WebGPU fetched 25 assets and exercised retarget/release/
reconnect, cutting, subsequent tether controls and prefab spawning without
runtime errors. Native/browser captures were visually reviewed. CI passed on
typed removal head `5973c8f5`.

A **typed distance creation slice** adds `Physics.create_distance_joint` on an
existing owner with no authored 2D joint of any kind. It shares scoped endpoint
validation with retargeting; null/inactive targets suspend. Finite positive
lengths, valid owners and stable same-scope references validate before mutation.
Next fixed synchronization creates the owned constraint while retaining other
components, bodies/motion and legacy connections. Creation before body sync,
repeated removal/recreation, owner conflicts, invalid lengths/references,
inactive targets and isolated runtime prefabs are exercised. Platformer C repairs
its cut cord at the selected hook and length; repeated repairs keep controls usable.
Final scoped preflight passed 338 native tests and warning-denied all-target/
all-feature checks for Decay and platformer. Typed lantern preflight had zero
errors/reminders. Two-crate Clippy, 11 regenerated catalogue tests and the
1,103-file size gate passed. All-target/all-feature WASM checks passed both crates; the generic browser host
was rebuilt. Export and native Vulkan capture passed. Chromium WebGPU fetched
25 assets and exercised repeated cutting/repair, retarget/release/reconnect and
prefab spawning without runtime errors. Native/browser captures were visually
reviewed, including the repaired lantern after recovery from free fall. CI passed
on typed distance creation head `a2c54306`.

A **typed hinge creation slice** adds `Physics.create_hinge_joint` with finite
body-local `Vec2` anchors, angular limits/motor initially disabled and existing
motor controls available before body synchronization. Shared owner/scope checks
validate before mutation. Tests exercise pre-sync and repeated creation, local
anchors, motor drive, owner/anchor rejection, missing physics, unbound/inactive
endpoints and runtime prefab isolation. Platformer H rebuilds its placed hinge;
a repeated-rebuild run observes fixed axle and reversal while a second windmill
remains independently owned. Final scoped preflight passed 342 native tests and
warning-denied all-target/all-feature checks for Decay and platformer. Typed
setup-script preflight had zero errors/reminders. Two-crate Clippy, 11 regenerated
catalogue tests and the 1,104-file size gate passed. All-target/all-feature WASM
checks passed both crates; the generic browser host was rebuilt. Export and
native Vulkan capture passed. Chromium WebGPU fetched 25 assets and exercised
two H rebuilds with a spawned windmill, plus tether cut/repair/retarget/release,
without runtime errors. Native/browser captures were visually reviewed. CI must
verify this slice; all checks passed on prior head `a2c54306`.

A **typed spring creation slice** adds `Physics.create_spring_joint` with finite
body-local `Vec2` anchors, positive rest length and non-negative stiffness/damping.
It shares owner/scope validation and fixed synchronization with other creation
calls, retaining other components, bodies/motion and legacy connections. Tests
exercise pre-sync/repeated creation, local-anchor force response, retuning,
owner/settings rejection, missing physics, unbound/inactive targets and runtime
prefab isolation. Platformer B rebuilds at its current rest length and continues
short/long tuning while its independent trolley remains on the rail. Final
scoped preflight passed 346 native tests and warning-denied all-target/all-feature
checks for Decay and platformer. Typed spring-script preflight had zero errors/
reminders. Two-crate Clippy, 11 regenerated catalogue tests and the 1,105-file
size gate passed. Joint-call classification is extracted to keep the main
physics dispatcher within its function limit. All-target/all-feature WASM checks
passed both crates; the generic browser host was rebuilt. Export and native
Vulkan capture passed. Chromium WebGPU fetched 25 assets and exercised two B
rebuilds alongside hinge recreation, prefab spawning and tether controls without
runtime errors. Native/browser captures were visually reviewed. CI passed on
typed spring creation head `d7b64c78`.

A **typed slider creation slice** adds `Physics.create_slider_joint` with finite
body-local anchors, unit axes and optional finite travel bounds. Enabled lower
bounds cannot exceed upper bounds. Motor starts disabled and accepts existing
typed drive controls before synchronization. Shared owner/scope validation
precedes mutation; solver allocation retains other components, body motion and
legacy ownership. Native tests cover pre-sync/repeated creation, bounded travel
and reversal along local axes, invalid geometry, inactive/unbound targets and prefab
isolation for all four constructors. Platformer J rebuilds the rail slider with
its current drive direction, preserving bounded motion and its independently
owned spring. Final scoped preflight passed 350 native tests and warning-denied
all-target/all-feature checks for Decay and platformer. Typed trolley-script
preflight had zero errors/reminders. Two-crate Clippy, 11 regenerated catalogue
tests and the 1,106-file size gate passed. All-target/all-feature WASM checks
passed both crates; the generic browser host was rebuilt. Export and native
Vulkan capture passed. Chromium WebGPU fetched 25 assets and exercised two J
rebuilds alongside spring/hinge recreation, prefab spawning and tether controls
without runtime errors. Native/browser captures were visually reviewed. CI passed
on typed slider creation head `04ef2587`.

A **hinge position motor slice** adds Sindri-owned `MotorMode2d` with velocity
as the backward-compatible default. Position drive uses a relative angle within
`[-pi, pi]`, non-negative finite stiffness/damping and the existing torque cap;
enabled limits still apply. Typed `Physics.set_hinge_position_motor` validates
atomically, retains unknown fields/body motion and works before bodies exist.
The velocity setter switches back explicitly; zero torque coasts. Suspension
and collider rebuilds retain settings. Platformer P switches both windmills
between holding 0.6 radians and reversing. H sends a typed `Windmill` message to
restore the selected drive after recreation, retaining direction.

Validation: scoped preflight passed 865 native tests and warning-denied
all-target/all-feature checks for physics, scene, Decay and platformer. Two
scripts passed typed checking with zero errors/reminders. Four-crate Clippy,
11 catalogue tests/regeneration, the 1,111-file size gate and all-target/all-feature
WASM checks passed. Native tests cover targets, caps/limits, atomic rejection,
coasting, missing physics, suspension/rebuild, undo/redo and canonical save/reopen.
The rebuilt generic browser host/export and native Vulkan capture passed.
Chromium WebGPU fetched 25 assets and exercised P hold/H recreation/P resume,
prefab spawning and tether controls without runtime errors. Visually reviewed
captures showed both windmills hold the same angle through recreation and resume
motion. CI passed on hinge position head `adb632e5`.

A **slider position motor slice** reuses `MotorMode2d` for signed local-anchor
separation along the first body's axis, in unscaled world units. Position drive
uses finite non-negative stiffness/damping and the existing force cap; enabled
travel limits still apply to targets outside them. Old scenes keep velocity mode.
Typed `Physics.set_slider_position_motor` validates atomically, preserves unknown
fields/body motion and works before bodies exist. The velocity setter switches
back; zero force coasts. Suspension and collider rebuilds retain the drive.
Platformer O parks/releases its trolley at signed distance 0.5, and J restores
the selected drive and direction after recreation while its spring keeps tuning.
Native tests cover rotated rails/offset anchors, signed retargeting, caps/limits,
invalid calls, coasting, missing physics, lifecycle, undo/redo and save/reopen.

Scoped preflight passed 874 native tests and warning-denied all-target/all-feature
checks for physics, scene, Decay and platformer. Typed trolley preflight had zero
errors/reminders. Four-crate Clippy, 11 catalogue tests/regeneration, the 1,115-file
size gate and all-target/all-feature WASM checks passed. The generic browser host
was rebuilt; export and native Vulkan capture passed. Chromium WebGPU fetched
25 assets and exercised O parking/J recreation/O release, prefab spawning and
tether controls without runtime errors. Visually reviewed native/browser captures
show retained parking through recreation and resumed travel with the spring
attached. CI passed on slider position head `e92487ec`.

An **editor reference-aware save slice** connects scene Save/Save As and subtree
prefab authoring to `World::to_scene_with_references` with the active registry.
Stable IDs remain a prerequisite. Unknown fields, live component strings and
runtime identity remain unchanged; existing placed instances still save as
references. Invalid local targets/types and unstable endpoints fail before
writing or adopting a path. Tests retain disk, path and the agreed document on
Save/Save As failures; two instances of every joint kind reopen independently
and saved subtrees spawn twice with isolated constraints. The platformer's real
Decay setup script spawns its nested windmill and editor Save As reopens it with
motor reversal and a fixed axle, connecting an existing engine capability to
editor authoring through the genre showcase.

Scoped preflight passed 622 native editor tests and warning-denied all-target/
all-feature checks. Editor Clippy, 11 catalogue tests, the 1,116-file size gate
and the WASM editor-stub check passed. Runtime/browser/render/dependencies and
Decay host/scripts are untouched. Prior slider head `e92487ec` is green; CI must
verify the new save slice.

Decay `Save` is a number/flag progress store; it has no existing world snapshot
save/load operation to connect. That absent capability now has its own parity
row. Save is still refused during Play. Joint acceptance remains open for native
visual editor review/final integration; gameplay world snapshot persistence is
a separate capability gap.

Continue with **the remaining joints and typed controls/lifecycle**, then
character movement, accelerated queries, 3D/voxel physics and final integration.
Historical recovery notes below describe the original loss, not the current
implementation.

Use pinned Rust 1.95. This local checkout has the toolchain at
`/tmp/sindri-physics-rustup`, with rustup shims under
`/tmp/sindri-physics-cargo/bin`; use `RUSTUP_HOME` and prepend the shims to PATH.
The local system Rust is 1.97 and adds a lint to an unchanged scene test. Keep
build debug = 0, incremental = false and warning flags consistent to reuse
artifacts. Native artifacts are in `target/`; browser artifacts are in
`/tmp/sindri-physics-wasm`. Never clean a directory while a build uses it.
Linux all-features checks need pkg-config, ALSA and udev build dependencies.
Invoke `python3 scripts/preflight.py` (the script has no executable bit here).

## Historical saved state

At handoff preparation, the branch head was
`23cac712d0f9d6f17206a53de110780e8bf5442d`, based on main
`dae1b14df254413d24679556e88156a907e5909c`.
This handoff is a subsequent documentation commit.

**Only the documentation reconciliation and acceptance checklist were saved
to GitHub. No new physics implementation was pushed.** The workspace reset
before the local CCD implementation was committed/pushed. That checkout,
toolchain bootstrap, build cache, staged one-way/force files, and running
preflight disappeared. A fresh clone confirmed the remote state. Do not claim
to have recovered code that is absent from the branch.

The saved docs checkpoint corrected outdated overlap/shape-cast claims,
compound-game-proof claims, and parented-body limitations in
`docs/physics.md` and `docs/capabilities.md`. It added
`docs/physics-update.md`. Its GitHub CI was observed green before the reset.

At the original handoff all nine implementation checkboxes were unchecked.
The current checklist in `physics-update.md` supersedes this historical state.

## First task: rebuild CCD in small slices

The following is a reconstruction guide for the lost implementation, not
committed code or a guarantee about the current checkout.

1. Add `continuous_collision: bool` to public `RigidBody2d` in
   `crates/sindri-physics/src/types2d.rs`, with `#[serde(default)]` and
   default false. Older scenes must still deserialize.
2. Pass the value to Rapier's `.ccd_enabled(...)` in the body builder.
   Add backend-private controls in `world2d/controls.rs`:
   getter `continuous_collision(entity)`, and dynamic-body-only setter
   `set_continuous_collision(entity, enabled)`; verify Rapier's actual
   current `enable_ccd` signature before writing.
3. Add the false default to the registered rigid-body schema in
   `crates/sindri-scene/src/extract/physics_registry.rs`.
   Exercise authoring through the generic command-backed inspector.
4. In `ScenePhysics2d` synchronization, a CCD-only payload change must toggle
   the live body without rebuilding it. Compare an authored snapshot with
   the previous CCD value replaced; preserve live velocity, joints and
   contacts. Normal collider/kind edits keep their existing lifecycle.
5. Add typed Decay getter `Physics.continuous_collision(entity) -> bool`
   and setter `Physics.set_continuous_collision(entity, bool) -> unit`.
   Update operation catalogue, environment, host dispatcher and reference.
   The host setter validates an authored dynamic body and updates both the
   live backend (if built) and the runtime authored payload.
   Before a freshly spawned body is synchronized, the payload keeps the
   requested setting. Getter uses the live backend when present, payload
   otherwise. Fail clearly for a host without physics or a missing body.
6. `surface/call.rs` approached 580 lines. Split the PhysicsCall enum,
   PHYSICS_CALLS table and is_event implementation to
   `surface/call/physics.rs`, re-exporting crate-private names. Place the
   module declaration after the parent's inner module documentation.
7. Enable CCD in the platformer hero's rigid-body payload and its Decay
   start function. Its movement is a real dynamic body. Scorchball's ball
   uses manual Decay movement and is not a valid rigid-body CCD proof.
8. Update changelog, physics contract, scripting docs, capabilities and
   parity; regenerate the catalogue and run its completeness tests.
   Do not hand-edit generated files.

### CCD regression design and limitations

The lost regression used zero gravity; a thin **velocity-kinematic** wall at
x = 1 with half extents [0.025, 2]; and a radius-0.05 dynamic bullet moving
240 units/s from x = 0 over a 1/60 second step. Discrete control crossed x = 2;
the CCD body stopped before x = 1. Test old payload omission and live toggles.

Rapier 0.36 automatically sweeps dynamic bodies against fixed colliders.
A fixed-wall-only test therefore cannot distinguish the opt-in setting.
The researched backend also allowed two enabled CCD bullets to tunnel through
each other. Sensors remain discrete. Document these limits; do not promise
swept trigger events or reliable bullet-versus-bullet CCD.

A lost Decay integration test started a script before its authored body
existed, enabled CCD, set velocity to [7, 0], checked the getter, then
synchronized and checked the live setting/velocity. It next changed the runtime
payload's CCD setting and stepped again: velocity had to remain 7, proving
the toggle did not rebuild the body.

### Results observed before reset

These apply only to the lost local version and must be rerun on rebuilt code:

- Focused native CCD tests passed.
- Decay spawn-window/velocity-preservation integration test passed.
- Typed hero script preflight passed with zero errors/reminders.
- Platformer native check and all six end-to-end tests passed.
- Full sindri-decay tests passed during the final preflight.
- WASM check for sindri-decay, sindri-scene and platformer passed.
- Generated catalogue validation/completeness tests passed.
- Clippy found one `match_same_arms` error: the setter unit return arm
  needed merging with other unit-returning PhysicsCall variants. That was
  fixed locally, but a clean complete subsequent Clippy result was not observed.
- The complete preflight result was never recovered. It was last observed
  compiling/testing the physics crate. Do not infer success.

## Remaining feature guidance

The acceptance checklist in `docs/physics-update.md` is the scope authority.
These are original implementation notes. The current checkpoint and acceptance
checklist above supersede their historical status.

### One-way platforms

- Public serializable per-piece optional OneWay2d policy, omitted = ordinary
  solid. Default local normal [0, 1], configurable support cone/angle.
  Validate finite nonzero normals and a bounded angle.
- An entity-level `sindri.physics2d.one_way` component with a useful default
  can expose ordinary authoring in the generic inspector and apply to all
  solid pieces, including tilemap pieces. Keep sensors unchanged.
- Use backend-private PhysicsHooks maps from collider handles to policies
  and entity handles to timed drop-through requests. Clean these on removal.
  Transform support normals through collider rotation.
- **Filter contact pairs as well as modifying solver contacts.** The
  researched Rapier CCD sweep calls FILTER_CONTACT_PAIRS; solver-only
  one-way contacts can block a fast ascending CCD character.
- Verify Rapier 0.36 APIs locally: ContactModificationContext used
  `rigid_mut()` and `update_as_oneway_platform`, unlike older tutorials.
- Timed `Physics.drop_through(entity, seconds)` should ignore only one-way
  solid contacts, not ordinary floors or sensors. Define cancellation,
  expiration, missing-body and spawn-window behavior explicitly.
- Native proof: rise through from below, descend/land, drop to an ordinary
  floor, expire and land again, rotated normals, kinematic platforms, removal.
- Platformer proof needs actual visible one-way planks and a drop input.
  The existing foot sensor can overlap a plank from below: revise grounding
  so that overlap alone does not grant a jump. Keep gameplay policy in Decay.
  Preserve the existing run-to-flag and ground-clearance regressions.

### Forces and rotation

- General API: additive world force, torque, angular velocity setter/getter,
  angular impulse and impulse at world point.
- Proposed semantics: forces/torques accumulate for the next fixed step and
  are reset immediately after it; impulses act immediately, independent of dt.
  State these rules explicitly and handle valid newly spawned entities.
- Verify current Rapier methods: add_force, add_torque, reset_forces,
  reset_torques, apply_impulse_at_point, apply_torque_impulse, set_angvel, angvel.
- Dynamic-only forces/impulses; setter supports dynamic/velocity-kinematic.
  Reject nonfinite values and wrong kind before mutation.
- Tests should measure force/mass * dt, accumulation and expiration,
  off-centre translation/rotation, torque and rotation-lock behavior.
  Add typed Vec2 Decay controls and real gameplay proof.

### Contact snapshots

Expose copied, deterministic entity-based contacts with world points,
normals, normal/tangent impulses and force over the last fixed dt.
Orient normals consistently relative to the queried entity; keep Rapier
handles private. Exercise grounding and impacts through Decay.

Rapier 0.36 contact pairs use rigid/manifold accessors. Actual solved impulses
may be in solver_clusters when clustering is enabled. Inspect current source
instead of assuming old contact fields are the solved values. Specify empty,
sensor-only, multi-piece, removal and sleeping-contact behavior.

### Reusable physics materials

Do not stop at a runtime material struct. Require reusable project assets,
validation, explicit literal override rules, editor loading/hot reload,
exported native/browser loading, reference collection and real game proof.

Existing `.profile` assets and ProfileDocument/ProfileSources were explored
as reuse options, not accepted architecture. Scene cannot depend on Decay.
Runtime Collider2d is Copy; adding asset strings there affects callers.
Keep asset resolution at the scene/host seam and backend material values
engine-owned. Audit export and async browser loading.

### Complete joints

Preserve existing connect_distance behavior. Add scene-authored distance,
hinge, slider and spring joints plus motors, typed Decay control, stable
serialized scene references resolved to runtime entities, removal/rebuild
lifecycle, undoable editor authoring and gameplay proof. Split world2d
responsibilities before growing a file past the repository limit.

### Character movement

A reusable engine sweep/slide primitive with slopes, steps, ground state
and moving-platform support. Gameplay policy remains Decay. Define and test
initial overlap, skin, iteration budget, downhill/uphill, ceilings, step height,
platform displacement, one-way collision and drop-through interactions.

### Accelerated queries

Current queries are in world2d/query.rs and world2d/sweep.rs.
Preserve mask, sensor inclusion, whole-entity exclusion, inactive filtering,
inside-hit zero normal and deterministic ties (entity then piece order).
Index must see inserts before first step, immediate teleports/removals, collider
edits and current synchronized poses. Supply meaningful scaling evidence.

### 3D runtime and voxel proof

The standalone 3D body world now runs fixed-step simulation, entity events and
indexed rays/overlaps/shape casts. The scene driver is implemented; integrate
game/editor hosts, exercise native authoring and add typed Vec3 Decay access. Inspect both editor and exported
game host plumbing so they share semantics. Transform3D uses quaternion [x,y,z,w].

Voxel-world proof must collide with actual voxel terrain. Do not use an
invisible plane and claim dynamic voxel collision. Address section residency,
collision updates on edits, budgeting and removal at the engine seam.
Prove native/browser behavior in a real project.

## Execution and checkpoint discipline

- Use an isolated checkout; do not edit unrelated dirty checkouts.
- Avoid recreating the old absolute paths or assuming old sessions are alive.
  The previous toolchain and cache were lost. Check what is installed first.
- Rust is 1.95. Avoid overlapping Cargo builds in one target directory.
  Prior default debug/incremental builds filled the disk and caused avoidable
  cache corruption. Use debug = 0, incremental = 0, modest build jobs when
  appropriate; never clean a target directory while a build is running.
- Use terminal output/logs that remain inspectable; distinguish a running
  process from stale redirected output. Keep user updates frequent.
- Follow the mandatory per-push gate. Use
  `scripts/preflight.py --base origin/codex/physics-update` for narrow slices.
  Keep warning flags consistent to avoid rebuilding the same graph repeatedly.
- Regenerate with `cargo run -p sindri-capabilities -- --write` and
  `cargo test -p sindri-capabilities` for host/schema changes.
- Run typed Decay preflight for every changed script, plus runtime regressions.
- Use `scripts/browser/README.md` and actual browser smoke tests; WASM
  compilation alone does not prove browser behavior.
- GitHub connector tree/commit/ref updates worked when shell push had no
  credentials. Never force-push over a moved branch; reconcile fresh remote
  state and any autofix commits before editing.
- Push completed smaller slices immediately. Keep remaining acceptance items
  visible in the PR body. Do not let an implementation wait unpushed while
  unrelated work accumulates.
- Finish with full applicable workspace/native/WASM/browser checks, final diff
  and documentation review, and green CI on the final head before marking ready.

## Copyable new-session request

Continue the physics update in vardirhq/sindri-engine, draft PR #497,
branch codex/physics-update. Read AGENTS.md, CLAUDE.md,
docs/physics-update-handoff.md and docs/physics-update.md first.
Implement every remaining acceptance item, including docs, in this one PR.
CCD, one-way platforms, forces/rotation, contacts, materials and the joint/motor
slices are checked and saved on the branch. Editor saves now remap registered
references automatically. Continue with character movement and the remaining
checklist. Script-triggered world snapshots are a separate absent capability
tracked in parity.
Push checked small slices regularly, each feature in one push or less.
Keep gameplay in Decay, prove editor/runtime/script/game behavior, and do
not mark the PR ready until final applicable checks and CI are green.
