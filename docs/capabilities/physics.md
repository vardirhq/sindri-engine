<!-- Generated from docs/capabilities/physics.toml by `cargo run -p sindri-capabilities -- --write`. Edit the source, not this file. -->

# Physics

`sindri-physics` is the masked physics boundary. Rapier2D and Rapier3D are
private implementation dependencies; public code speaks only in Sindri body,
collider, shape, layer, pose, error, and event types. Runtime entities remain
the identity exposed by the subsystem, so no Rapier handle can leak into a
scene, script, editor, or game contract.

The exercised runtime is 2D. A parallel 3D world, scene driver and typed Decay
surface exist and are exercised by fixtures, but no shipped game yet makes a
gameplay decision with them. The contract is `docs/physics.md`.

Each entry links its tests and the projects that exercise it. Surface marks match the row in [parity](../parity.md) where one is named: ✅ works and is exercised · 🟡 a useful slice, with the gap named · ❌ absent · — not applicable.

| Capability | Engine | Editor | Decay | Proof |
| --- | :-: | :-: | :-: | :-: |
| **2D bodies and colliders** | | | | |
| [Bodies, fixed-step stepping, velocity, impulse](#bodies-2d) | ✅ | 🟡 | ✅ | ✅ |
| [Masks, sensors, collision events](#masks-sensors-events) | ✅ | 🟡 | ✅ | ✅ |
| [Named collision layers](#named-layers) | ✅ | ✅ | ✅ | ✅ |
| [Compound colliders](#compound-colliders) | ✅ | ✅ | — | ✅ |
| [Tilemap collision](#tilemap-collider) | ✅ | 🟡 | — | ✅ |
| [Scene gravity](#scene-gravity) | ✅ | ✅ | — | ✅ |
| [Moving a body by its transform](#scene-sync-2d) | ✅ | — | ✅ | ✅ |
| [Forces and rotation (2D)](#forces-rotation-2d) | 🟡 | 🟡 | 🟡 | 🟡 |
| [Solid contact snapshots (2D)](#contacts-2d) | ✅ | 🟡 | ✅ | ✅ |
| [One-way platforms](#one-way-platforms) | ✅ | 🟡 | ✅ | ✅ |
| [Physics materials as assets (2D)](#materials-2d) | ✅ | 🟡 | — | ✅ |
| [Collider gizmos in the Scene view](#collider-gizmos) | — | ✅ | — | — |
| [Continuous collision (CCD)](#ccd-2d) | ✅ | 🟡 | ✅ | ✅ |
| **2D character controller** | | | | |
| [Sweep, slide, ground, step and carry queries](#character-movement-2d) | ✅ | — | — | ✅ |
| [Character controller (2D)](#character-controller-2d) | ✅ | ✅ | ✅ | ✅ |
| **2D queries** | | | | |
| [2D closest-hit raycasts](#raycasts-2d) | ✅ | — | ✅ | ✅ |
| [Overlap and shape casts (2D)](#overlap-cast-2d) | ✅ | — | ✅ | ✅ |
| [Accelerated 2D physics queries](#accelerated-queries-2d) | ✅ | — | ✅ | ✅ |
| **2D joints** | | | | |
| [Joints and constraints](#joints-2d) | 🟡 | 🟡 | 🟡 | ✅ |
| [Typed joint controls from Decay](#joint-controls) | ✅ | — | ✅ | ✅ |
| [Hinge and slider position motors](#joint-position-motors) | ✅ | ✅ | ✅ | ✅ |
| [Authored joint references in prefabs](#joint-prefab-references) | 🟡 | — | 🟡 | ✅ |
| [Saved spawned-prefab reference remapping](#saved-reference-remapping) | ✅ | ✅ | — | ✅ |
| **3D** | | | | |
| [Standalone 3D physics world](#world-3d) | ✅ | — | — | ❌ |
| [3D physics in scenes](#scene-physics-3d) | ✅ | 🟡 | 🟡 | ❌ |
| [3D spawn-window controls](#spawn-window-3d) | 🟡 | ❌ | 🟡 | ❌ |
| [3D physics queries](#queries-3d) | ✅ | ✅ | ✅ | 🟡 |
| [3D replaceable static geometry groups](#static-groups-3d) | 🟡 | ❌ | ❌ | ❌ |
| **Not yet** | | | | |
| [Automatic 3D collider scaling](#collider-scaling-3d) | ❌ | ❌ | ❌ | ❌ |
| [Extreme 3D query numerical failure reporting](#query-numerics-3d) | ❌ | — | — | ❌ |
| [Script-triggered world snapshot save/load](#world-snapshots) | ❌ | — | ❌ | ❌ |
| [Character controller (3D)](#character-controller-3d) | ❌ | ❌ | ❌ | ❌ |
| [Compound solid character probes](#compound-character-probes) | ❌ | — | ❌ | ❌ |
| [Same-step solver response to controller motion](#same-step-controller-response) | ❌ | — | ❌ | ❌ |
| [Swept controller trigger events](#swept-controller-triggers) | ❌ | — | ❌ | ❌ |
| [Sweeping rotating platform carry along its arc](#rotating-carry-arc) | ❌ | — | ❌ | ❌ |
| [Sweeping a rotating character probe](#rotating-character-probe) | ❌ | — | ❌ | ❌ |

## 2D bodies and colliders

<a id="bodies-2d"></a>

### Bodies, fixed-step stepping, velocity, impulse

Fixed, dynamic and two kinds of kinematic body, stepped on the engine's fixed update with validated velocity and impulse controls.

Engine ✅ · Editor 🟡 · Decay ✅ · Proof ✅ — parity row *Bodies, fixed-step stepping, velocity, impulse*

**Tested by:** [`crates/sindri-decay/tests/a_script_drives_a_body.rs`](../../crates/sindri-decay/tests/a_script_drives_a_body.rs), [`crates/sindri-physics/tests/copies_go_on_as_the_original.rs`](../../crates/sindri-physics/tests/copies_go_on_as_the_original.rs)

**Exercised in:** [`games/orbital-baked`](../../games/orbital-baked), [`games/platformer`](../../games/platformer)

**See:** [`docs/physics.md`](../../docs/physics.md)

<details markdown="1">
<summary>Evidence</summary>

`PhysicsWorld2d` owns fixed, dynamic, position-kinematic, and
velocity-kinematic bodies; box, circle, and capsule colliders; friction and
restitution; and checked velocity/impulse/kinematic operations. Values are
validated before they reach the backend, and stepping takes an explicit engine
fixed-step duration rather than a render delta. Tests cover gravity, body
operations, and removal/reuse through generation-checked `EntityId`s, and the
crate passes the workspace's native and WASM checks.

`sindri.physics2d.rigid_body` and `sindri.physics2d.collider` are registered
scene components with defaults the engine accepts, so a scene authors bodies
and colliders and the editor's generic component inspector adds and edits them.

</details>

<a id="masks-sensors-events"></a>

### Masks, sensors, collision events

Independent membership and filter masks, sensor colliders, and normalized collision/sensor start and stop events.

Engine ✅ · Editor 🟡 · Decay ✅ · Proof ✅ — parity row *Masks, sensors, collision events*

**Tested by:** [`games/orbital-baked/tests/asteroid_physics.rs`](../../games/orbital-baked/tests/asteroid_physics.rs)

**Exercised in:** [`games/orbital-baked`](../../games/orbital-baked)

<details markdown="1">
<summary>Evidence</summary>

`games/orbital-baked` is the end-to-end proof: player, enemies, projectiles,
pickups, and effects use distinct masks and collision or sensor events
continuously.

</details>

<a id="named-layers"></a>

### Named collision layers

A scene names its collision layers bit by bit; scripts and the inspector use the names.

Engine ✅ · Editor ✅ · Decay ✅ · Proof ✅ — parity row *Named collision layers*

**Tested by:** [`crates/sindri-decay/tests/a_script_names_its_layers.rs`](../../crates/sindri-decay/tests/a_script_names_its_layers.rs)

**Exercised in:** [`games/platformer`](../../games/platformer), [`examples/physics`](../../examples/physics)

**Limits:**

- Named per scene rather than in project settings; colliders still store masks.

<details markdown="1">
<summary>Evidence</summary>

A scene's `sindri.physics2d.world` names its collision layers bit by bit.
`Physics.layer` and `Physics.mask` turn names into masks and refuse an unknown
name, and the editor's inspector draws every collision mask as a menu of the
named layers, keeping unnamed bits. The platformer names ground, hero and
pickups; its hero's ground probe asks for `ground`.

</details>

<a id="compound-colliders"></a>

### Compound colliders

One body made of several collider pieces, each fully editable in the inspector.

Engine ✅ · Editor ✅ · Decay — · Proof ✅ — parity row *Compound colliders (several pieces, one body)*

**Tested by:** [`editor/tests/inspector_field_meanings.rs`](../../editor/tests/inspector_field_meanings.rs)

**Exercised in:** [`games/platformer`](../../games/platformer)

<details markdown="1">
<summary>Evidence</summary>

The generic inspector exposes the body and collider payloads — a compound's
pieces are added, removed, reordered, and edited down to each piece's shape.
The platformer's hero is one body of two pieces, a capsule and a box.

</details>

<a id="tilemap-collider"></a>

### Tilemap collision

Painted tiles become solid, merged into as few rectangles as cover them so nothing catches on tile seams.

Engine ✅ · Editor 🟡 · Decay — · Proof ✅ — parity row *Tilemap collision*

**Tested by:** [`crates/sindri-scene/src/tilemap_collision.rs`](../../crates/sindri-scene/src/tilemap_collision.rs), [`games/platformer/tests/painted_ground.rs`](../../games/platformer/tests/painted_ground.rs)

**Exercised in:** [`games/platformer`](../../games/platformer)

**Limits:**

- An isometric map is refused by name.
- Passable sprites are typed as names rather than picked from the palette.

<details markdown="1">
<summary>Evidence</summary>

`sindri.physics2d.tilemap_collider` makes the orthogonal tilemap on the same
entity solid: every painted tile collides unless its palette sprite is listed
as `passable`. Solid cells are merged into as few rectangles as cover them, so
a floor is one box and a character cannot catch on the seams between tiles;
the rectangles follow the entity's scale, and join any collider pieces the
entity also carries.

</details>

<a id="scene-gravity"></a>

### Scene gravity

`sindri.physics2d.world` sets the scene's gravity, so editor Play runs a scene as its build will.

Engine ✅ · Editor ✅ · Decay — · Proof ✅ — parity row *Scene gravity*

**Tested by:** [`crates/sindri-physics/tests/gravity_changes.rs`](../../crates/sindri-physics/tests/gravity_changes.rs), [`crates/sindri-decay/tests/a_script_turns_gravity.rs`](../../crates/sindri-decay/tests/a_script_turns_gravity.rs)

**Exercised in:** [`games/platformer`](../../games/platformer), [`games/scorchball`](../../games/scorchball), [`examples/physics`](../../examples/physics)

**Limits:**

- One vector; no per-area gravity.

<details markdown="1">
<summary>Evidence</summary>

`sindri.physics2d.world` carries the scene's gravity (straight down at 9.81 by
default); a scene without one keeps its host's, which for the editor and
Orbital is none. Decay reads and turns it with `Physics.gravity` and
`Physics.set_gravity` (the Physics Playground's GRAVITY button), and a change
wakes resting bodies.

</details>

<a id="scene-sync-2d"></a>

### Moving a body by its transform

`ScenePhysics2d` keeps the simulation in step with the scene and writes physics results back into transforms.

Engine ✅ · Editor — · Decay ✅ · Proof ✅ — parity row *Moving a body by its transform*

**Tested by:** [`crates/sindri-scene/src/physics_sync/tests.rs`](../../crates/sindri-scene/src/physics_sync/tests.rs)

**Exercised in:** [`games/platformer`](../../games/platformer)

<details markdown="1">
<summary>Evidence</summary>

`ScenePhysics2d` builds the simulation from the scene components, keeps it in
step as entities are spawned, switched off, and despawned — a body outliving
its entity would collide on behalf of nothing — and writes what physics decided
back into the transforms the renderer reads, leaving the authored Z and the 3D
scale alone. A transform moved by something other than physics since then, a
script respawning the player, moves the body with it before the next step
rather than being put back. Friction combines as the smaller of two colliders'
values, so a frictionless hero slides down walls. A body whose authored values
have not changed is left as it is rather than rebuilt, because rebuilding one
discards the velocity and contacts the simulation owns. Editor Play and the
game session both step it once per fixed update, before scripts run, so a
script observes the events of the step that just happened.

</details>

<a id="forces-rotation-2d"></a>

### Forces and rotation (2D)

Additive world force and torque, immediate and off-centre impulses, angular velocity and rotation locks.

Engine 🟡 · Editor 🟡 · Decay 🟡 · Proof 🟡 — parity row *Forces and rotation (2D)*

**Tested by:** [`crates/sindri-physics/tests/forces_and_rotation.rs`](../../crates/sindri-physics/tests/forces_and_rotation.rs), [`crates/sindri-decay/tests/forces_and_rotation.rs`](../../crates/sindri-decay/tests/forces_and_rotation.rs), [`games/platformer/tests/forces_and_rotation.rs`](../../games/platformer/tests/forces_and_rotation.rs)

**Exercised in:** [`games/platformer`](../../games/platformer)

**See:** [`docs/physics.md`](../../docs/physics.md)

**Limits:**

- Browser input and visual inspector review remain in final integration.

<details markdown="1">
<summary>Evidence</summary>

Additive world force and torque last one fixed step, impulses act immediately
and off-centre kicks turn a body. Dynamic/velocity-kinematic rules, finite
values and rotation locks are validated; spawn-window requests replay after
collider mass is known and expire if never materialized. The platformer's wind
crate exercises force, torque, angular velocity and impulses through its real
kick input, with scene rotation writeback. Native, WASM and browser CI passed
on the forces/rotation head.

</details>

<a id="contacts-2d"></a>

### Solid contact snapshots (2D)

Copied solver contacts: other entity, world point, push normal, normal and friction impulses, and force over the last step.

Engine ✅ · Editor 🟡 · Decay ✅ · Proof ✅ — parity row *Solid contact snapshots (2D)*

**Tested by:** [`crates/sindri-physics/tests/contacts.rs`](../../crates/sindri-physics/tests/contacts.rs), [`crates/sindri-decay/tests/contact_snapshots.rs`](../../crates/sindri-decay/tests/contact_snapshots.rs), [`games/platformer/tests/contact_impacts.rs`](../../games/platformer/tests/contact_impacts.rs)

**Exercised in:** [`games/platformer`](../../games/platformer)

**Limits:**

- Sensors are excluded.
- Browser interaction and editor Play inspection remain in final integration.

<details markdown="1">
<summary>Evidence</summary>

`PhysicsWorld2d::contacts` and typed `Physics.contacts(entity)` expose copied
solid solver snapshots: other entity, world point, push normal towards the
queried body, normal and signed friction impulses, and world force over the
last fixed dt. Solver manifolds supply solved impulses. Ordering is
deterministic by entity, point, normal and impulses. Sleeping support remains
with zero new impulse/force. Runtime teleports and removals invalidate
contacts, and Decay filters inactive/despawned others. Spawn-window queries
are empty; missing bodies/physics fail.

The platformer's crate flashes on a hard landing; its hero grounds from
controller results while keeping its ray for clearance. This is a general
capability added for that genre showcase. Native regressions cover
momentum/force balance, compound pieces, snapshot copies, lifecycle, ordering
and sleeping support.

</details>

<a id="one-way-platforms"></a>

### One-way platforms

A local support normal and contact cone on solid pieces, with timed drop-through from Decay.

Engine ✅ · Editor 🟡 · Decay ✅ · Proof ✅ — parity row *One-way platforms*

**Tested by:** [`crates/sindri-physics/tests/one_way.rs`](../../crates/sindri-physics/tests/one_way.rs), [`crates/sindri-physics/tests/one_way_movement.rs`](../../crates/sindri-physics/tests/one_way_movement.rs), [`games/platformer/tests/one_way_platforms.rs`](../../games/platformer/tests/one_way_platforms.rs)

**Exercised in:** [`games/platformer`](../../games/platformer)

**Limits:**

- Geometric queries still see both sides, so Decay grounding uses solved support contacts and descent rather than foot-sensor overlap.
- Visual inspector and full browser interaction remain in final integration.

<details markdown="1">
<summary>Evidence</summary>

One-way support is authored with `sindri.physics2d.one_way`: a validated local
normal and contact cone, applied to solid collider and tilemap pieces. Pair
filtering also covers CCD ascent. `Physics.drop_through(entity, seconds)` uses
fixed simulation time, preserves ordinary floors/sensors and works in the spawn
window. Policy edits and undo keep live velocity/joints. The platformer's
visible planks prove ascent, landing, input-driven dropping to painted ground
and landing again; native regressions cover rotated normals, kinematic geometry
and timer cancellation. Chromium export/load smoke passed.

</details>

<a id="materials-2d"></a>

### Physics materials as assets (2D)

Reusable `physics_material` profile assets applied to every collider piece of an entity, with explicit overrides.

Engine ✅ · Editor 🟡 · Decay — · Proof ✅ — parity row *Physics materials as assets (2D)*

**Tested by:** [`crates/sindri-scene/src/physics_material/tests.rs`](../../crates/sindri-scene/src/physics_material/tests.rs), [`crates/sindri-export/tests/physics_materials.rs`](../../crates/sindri-export/tests/physics_materials.rs), [`games/platformer/tests/physics_materials.rs`](../../games/platformer/tests/physics_materials.rs)

**Exercised in:** [`games/platformer`](../../games/platformer)

**Limits:**

- Visual inspector review and browser gameplay interaction remain in final physics integration.
- 3D waits for its runtime.

<details markdown="1">
<summary>Evidence</summary>

Reusable physics coefficients are project `.profile` assets with type
`physics_material`, resolved at the scene boundary using shared validation.
`sindri.physics2d.material` applies them to all an entity's collider pieces,
including tilemap collision; explicit override flags win over profile values,
and entities without the component retain their literals. Editor creation,
profile selection, asynchronous delivery and hot reload use the existing
profile pipeline. Invalid reloads retain the previous valid edit; coefficient
changes preserve solver bodies, velocities and joints. Export discovers
material references and validates profiles, and the shared native/browser host
resolves them before stepping. The platformer reuses wood for its wind crate
and planks, explicitly disables plank bounce, and tests the effect of changed
restitution.

</details>

<a id="collider-gizmos"></a>

### Collider gizmos in the Scene view

Every collider is outlined from the pieces physics is given; the selected one's edges and radii drag.

Engine — · Editor ✅ · Decay — · Proof — — parity row *Collider gizmos in the Scene view*

**Tested by:** [`editor/src/native/collider_gizmo.rs`](../../editor/src/native/collider_gizmo.rs)

**Limits:**

- Offsets and rotations are still typed.
- 3D pieces are drawn as wireframes and have no handles.

<details markdown="1">
<summary>Evidence</summary>

The Scene view draws every collider's outline, from the very pieces the
physics world is given (a tilemap's merged rectangles included), brighter for
the selected entity and fainter for static geometry. The selected collider's
pieces carry handles: a box's four edges, a circle's radius, a capsule's radius
and height. Dragging an edge keeps the opposite edge where it was, along the
piece's own rotated axis, and a whole drag is one undo step.

</details>

<a id="ccd-2d"></a>

### Continuous collision (CCD)

An opt-in 2D body flag that sweeps fast bodies so they cannot tunnel through thin geometry.

Engine ✅ · Editor 🟡 · Decay ✅ · Proof ✅ — parity row *Continuous collision (CCD)*

**Tested by:** [`crates/sindri-physics/tests/continuous_collision.rs`](../../crates/sindri-physics/tests/continuous_collision.rs), [`crates/sindri-decay/tests/continuous_collision.rs`](../../crates/sindri-decay/tests/continuous_collision.rs)

**Exercised in:** [`games/platformer`](../../games/platformer)

**Limits:**

- Sensors stay discrete; bullet-versus-bullet sweeping is not guaranteed.
- Visual inspector and browser verification remain in final integration.

<details markdown="1">
<summary>Evidence</summary>

An opt-in 2D body flag, scene synchronization and typed Decay controls, with
thin kinematic-wall and spawn-window/undo lifecycle regressions. The
platformer's hero opts in; command-backed edits/undo preserve solver state.
Native and WASM checks passed. This entry was missing from the hand-written
inventory and is taken from its parity row.

</details>

## 2D character controller

<a id="character-movement-2d"></a>

### Sweep, slide, ground, step and carry queries

Read-only 2D movement queries: collision-limited slide, ground probes with slope limits, steps, platform carry and one-way rules.

Engine ✅ · Editor — · Decay — · Proof ✅

**Tested by:** [`crates/sindri-physics/tests/move_and_slide.rs`](../../crates/sindri-physics/tests/move_and_slide.rs), [`crates/sindri-physics/tests/ground_probe.rs`](../../crates/sindri-physics/tests/ground_probe.rs), [`crates/sindri-physics/tests/grounded_slide.rs`](../../crates/sindri-physics/tests/grounded_slide.rs), [`crates/sindri-physics/tests/slope_movement.rs`](../../crates/sindri-physics/tests/slope_movement.rs), [`crates/sindri-physics/tests/step_movement.rs`](../../crates/sindri-physics/tests/step_movement.rs), [`crates/sindri-physics/tests/platform_movement.rs`](../../crates/sindri-physics/tests/platform_movement.rs), [`crates/sindri-physics/tests/one_way_movement.rs`](../../crates/sindri-physics/tests/one_way_movement.rs)

**Exercised in:** [`games/platformer`](../../games/platformer)

**See:** [`docs/character-movement.md`](../../docs/character-movement.md)

**Limits:**

- Platform rotation follows a straight chord with fixed probe orientation; continuous arc and rotating-probe sweeps are absent.

<details markdown="1">
<summary>Evidence</summary>

A read-only 2D sweep/slide movement foundation proposes displacement against
current collider poses without mutating bodies. Positive skin, bounded
iterations, initial penetration, filters and deterministic ties have native
engine tests.

Read-only 2D ground probing adds configurable unit up, bounded slope angles,
travel and skin. Tests exercise zero-travel support, rotated shapes, slope
boundaries, steep obstruction, penetration, filters, current poses and invalid
inputs. The nearest hit remains visible when unwalkable. The composed grounded
movement query adds post-slide support state and optional downward snapping,
default off. Ascending requests and initial penetration never snap or report
grounded; steep support does not snap. Grounded motion enforces its support
slope limit while sliding: steep contacts cannot introduce unrequested rise,
explicit jumps retain their requested rise, and steep descent remains
ungrounded. Ordinary geometric projection remains unrestricted.

Optional steps require starting support, full lift clearance, better forward
progress and a walkable landing within the height cap. Rejected candidates
preserve the ordinary result. Native tests cover all three probe shapes,
ceilings/overhangs, height limits, absent/steep landings, rotated up and exact
fallback. Contact-normal refinement prevents flat box-face casts from producing
large artificial hops.

Opt-in platform snapshots verify old support and sweep synchronized
translation/rotation-point carry before the character move, retaining wall
clipping and crush/penetration outcomes. Native tests cover all shapes,
snapshot advancement, simulated kinematic ordering, rotation, filtering, jumps
and step accounting. Grounded movement respects per-piece one-way support
sides/cones across sliding, support, steps and carry. Request-scoped
drop-through ignores only one-way solids, preserving ordinary floors and sensor
filtering; hosts own duration and cancellation.

</details>

<a id="character-controller-2d"></a>

### Character controller (2D)

`sindri.physics2d.character` owns a kinematic probe moved by queued Decay requests, adopted by the platformer's hero.

Engine ✅ · Editor ✅ · Decay ✅ · Proof ✅ — parity row *Character controller (2D)*

**Tested by:** [`crates/sindri-scene/src/characters/tests.rs`](../../crates/sindri-scene/src/characters/tests.rs), [`crates/sindri-decay/tests/character_controllers.rs`](../../crates/sindri-decay/tests/character_controllers.rs), [`crates/sindri-decay/tests/character_controllers/motions.rs`](../../crates/sindri-decay/tests/character_controllers/motions.rs), [`crates/sindri-decay/tests/character_controllers/requests.rs`](../../crates/sindri-decay/tests/character_controllers/requests.rs), [`game/tests/the_character_controller_works.rs`](../../game/tests/the_character_controller_works.rs), [`games/platformer/tests/character_gameplay.rs`](../../games/platformer/tests/character_gameplay.rs), [`games/platformer/tests/controller_terrain.rs`](../../games/platformer/tests/controller_terrain.rs), [`games/platformer/tests/moving_platform.rs`](../../games/platformer/tests/moving_platform.rs), [`scripts/browser/platformer-goal.mjs`](../../scripts/browser/platformer-goal.mjs)

**Exercised in:** [`games/platformer`](../../games/platformer)

**See:** [`docs/character-movement.md`](../../docs/character-movement.md), [`docs/physics-character-editor-review.md`](../../docs/physics-character-editor-review.md)

**Limits:**

- Coyote time, jump buffering, gravity and velocity remain Decay gameplay policy.
- Solid response and discrete sensor events see controller motion at the following solve.

<details markdown="1">
<summary>Evidence</summary>

Scene controller ownership registers validated `sindri.physics2d.character`
settings beside the existing Collider 2D. It derives a stationary kinematic
body, requires one solid probe and rejects a simultaneous rigid body; extra
sensors, probe offsets/rotation and query masks are exercised. Runtime movement
requests replace per-step input and specify snap permission; timed controller
drop-through uses fixed simulation duration. Fresh support is seeded before the
solve and current platform motion is carried afterward, with motion applied
once and parent-relative writeback. Scene native tests cover spawning,
first-step vertical carry, rotation, parented riders,
teleports/rebuilds/inactivity/slot reuse, timers, request validation, cached
script borrows and saved authored data.

Typed Decay offers queued `Physics.move_character` requests and copied optional
`Physics.character_motion` results; `Physics.drop_through` selects controller
or dynamic-body timers by authored movement ownership. Motion/support/carry
fields and ordered hit lists filter inactive/despawned references without
rewriting historical displacement. Missing controller context and invalid
inputs fail explicitly; invalid requests preserve previous queued input. Shared
game runtime, editor Play and platformer harness offer the same scene context.
A rebuilt Chromium export of the shared-session fixture runs queued movement,
observes grounded/copied results and renders the controller above its floor.

The platformer adopts the controller through its real Hero script: native
regressions reach the flag with coins and no falls, traverse/drop through
planks, and exercise variable jump height, acceleration/braking and respawn.
Two authored stone risers and an inclined boardwalk exercise accepted steps,
walkable ascent and downhill snap without jumping; control tests disable
stepping or lower the slope limit to show the same terrain blocks walking.

The platformer's one-way plank ferry crosses the first gap using ordinary
kinematic velocity set by Decay. Hero adds no platform motion: scene support
snapshots supply each actual solved displacement once. Native game regressions
ride across and back with fixed relative position, board through a running
jump, leave/reacquire carry on jump/landing, and drop through into the
pit/normal respawn path. Chromium exports the same assets with the hero
initially aboard and renders riding in both directions and a keyboard jump.

Native Character 2D editor review exercises every setting, snap undo/redo,
save/reopen, add/remove through checked commands and restoring custom settings.
The reopened platformer runs/jumps in Play with a read-only inspector; Save
refuses during Play and file hashes confirm Stop restores the document.

The exported platformer has a CI browser goal regression. A scratch copy adds
one read-only Decay observer and retains the shipped starting pose and scripts.
Playwright drives ordinary movement/jump keyboard events, waiting for an
observed release between jumps. The gate asserts the flag, at least three coins
and zero falls, and preserves asset-fetch, WebGPU, pixel and script-error
checks.

</details>

## 2D queries

<a id="raycasts-2d"></a>

### 2D closest-hit raycasts

Finite world-space casts return entity, point, normal and distance, or null on a miss.

Engine ✅ · Editor — · Decay ✅ · Proof ✅ — parity row *2D closest-hit raycasts*

**Tested by:** [`crates/sindri-physics/tests/raycast.rs`](../../crates/sindri-physics/tests/raycast.rs), [`crates/sindri-decay/tests/a_script_casts_a_ray.rs`](../../crates/sindri-decay/tests/a_script_casts_a_ray.rs)

**Exercised in:** [`games/platformer`](../../games/platformer), [`examples/physics`](../../examples/physics)

**See:** [`docs/physics-playground.md`](../../docs/physics-playground.md)

<details markdown="1">
<summary>Evidence</summary>

Closest-hit 2D raycasts return Sindri `RayHit2d` snapshots (entity, world-space
point/normal, distance). Directions are normalized; maximum distance is
inclusive; invalid finite/range inputs fail. Membership masks, sensor opt-in
and whole-entity exclusion select pieces before choosing a hit. Inside hits
have zero distance/normal; exact ties prefer entity handle then piece order.
Decay returns `null` on a miss and skips inactive/despawned entities
immediately, while scene geometry changes are synchronized at the next step.

The platformer displays ground clearance through jumps against painted tilemap
colliders. `examples/physics`, the Physics Playground, has a probe that drags
rays, circle and box casts and area queries through a layer mask and reads out
what they hit, how far and the surface normal, with desktop/touch controls.
Pages exports it under `examples/physics/`; native project regressions and
desktop/phone browser smoke checks exercise its real Decay controls.

</details>

<a id="overlap-cast-2d"></a>

### Overlap and shape casts (2D)

Circle, box and capsule overlaps and sweeps, with the raycast's mask, sensor and exclusion filter.

Engine ✅ · Editor — · Decay ✅ · Proof ✅ — parity row *Overlap and shape casts (2D)*

**Tested by:** [`crates/sindri-physics/tests/overlap_and_shape_cast.rs`](../../crates/sindri-physics/tests/overlap_and_shape_cast.rs), [`games/orbital-baked/tests/combat_lab.rs`](../../games/orbital-baked/tests/combat_lab.rs)

**Exercised in:** [`games/orbital-baked`](../../games/orbital-baked), [`examples/physics`](../../examples/physics)

<details markdown="1">
<summary>Evidence</summary>

`PhysicsWorld2d::overlap` lists every entity with a piece overlapping a circle,
box or capsule placed in the world, and `shape_cast` sweeps one along a line to
the first piece it touches, both from current body poses with the raycast's
filter. Decay's `Physics.overlap_circle`, `overlap_box`, `cast_circle` and
`cast_box` reach them. Orbital's hostile mine damages what its blast circle
overlaps (`combat_lab.rs` proves a target caught by its edge), and the Physics
Playground's probe casts a ray, a circle or a box, or counts an area.

</details>

<a id="accelerated-queries-2d"></a>

### Accelerated 2D physics queries

A query-only per-piece BVH selects candidates for rays, overlaps, shape casts and controller probes.

Engine ✅ · Editor — · Decay ✅ · Proof ✅ — parity row *Accelerated 2D physics queries*

**Tested by:** [`crates/sindri-physics/src/world2d/spatial/tests.rs`](../../crates/sindri-physics/src/world2d/spatial/tests.rs), [`crates/sindri-physics/src/world2d/spatial/tests/controllers.rs`](../../crates/sindri-physics/src/world2d/spatial/tests/controllers.rs)

**Exercised in:** [`games/platformer`](../../games/platformer)

**Limits:**

- Dense or long queries can still visit every piece.
- Non-static pieces need a post-step refresh.

<details markdown="1">
<summary>Evidence</summary>

A query-only per-piece BVH works before the first step and after a body move.
Rays, overlaps and shape casts use the synchronized per-piece index, as do
controller penetration, slide and ground probes. Historical platform support
reads only the known support entity's pieces; current bounds cannot discard
its previous pose.

</details>

## 2D joints

<a id="joints-2d"></a>

### Joints and constraints

Scene-authored distance, hinge, slider and spring joints, each owned by its own entity, with typed Decay controls.

Engine 🟡 · Editor 🟡 · Decay 🟡 · Proof ✅ — parity row *Joints and constraints*

**Tested by:** [`crates/sindri-physics/tests/distance_joint.rs`](../../crates/sindri-physics/tests/distance_joint.rs), [`crates/sindri-physics/tests/hinges.rs`](../../crates/sindri-physics/tests/hinges.rs), [`crates/sindri-physics/tests/sliders.rs`](../../crates/sindri-physics/tests/sliders.rs), [`crates/sindri-physics/tests/springs.rs`](../../crates/sindri-physics/tests/springs.rs), [`crates/sindri-scene/src/physics_joints/tests.rs`](../../crates/sindri-scene/src/physics_joints/tests.rs), [`games/platformer/tests/distance_joints.rs`](../../games/platformer/tests/distance_joints.rs), [`games/platformer/tests/hinge_motor.rs`](../../games/platformer/tests/hinge_motor.rs), [`games/platformer/tests/slider_spring.rs`](../../games/platformer/tests/slider_spring.rs)

**Exercised in:** [`games/platformer`](../../games/platformer)

**See:** [`docs/physics-joint-editor-review.md`](../../docs/physics-joint-editor-review.md)

**Limits:**

- The 2D slice is complete; 3D joints are absent, which is why the family is 🟡.

<details markdown="1">
<summary>Evidence</summary>

`sindri.physics2d.distance_joint` adds a scene-authored maximum-distance
constraint owned by a separate entity. Endpoint strings resolve to stable IDs
inside the containing scene; unchanged frames retain one constraint, edits/undo
preserve body motion, and endpoint rebuilds reconnect after body
synchronization. Inactive/missing endpoints suspend the constraint; removing
its entity/component releases it independently of legacy script-created joints.
The platformer's lantern sways under Decay wind within its authored tether and
renders its cord in Decay.

`sindri.physics2d.hinge_joint` adds body-local anchors, bounded relative angles
and torque-capped velocity motors. Unchanged frames keep the constraint;
setting edits wake its endpoints without resetting body motion. The
platformer's Decay-driven windmill reverses its axle motor; native tests
exercise limits, caps, coasting, atomic validation, undo and rebuilds.

`sindri.physics2d.slider_joint` adds aligned local axes, signed travel limits
and force-capped velocity drive; `sindri.physics2d.spring_joint` adds radial
force-based stiffness/damping. The platformer's powered lantern trolley
reverses along its rail while Decay retunes the hanging light's rest length.
Native tests measure rotated rail motion/limits, force caps, coasting/braking,
spring damping and weight support; scene edits/undo, suspension, rebuilds and
game removal are exercised. Editor save integration and native visual review
are exercised.

All four authored constraints accept an enabled flag, defaulting true for old
payloads. Hinge and slider motor authoring uses registered velocity/position
choices from the engine enum rather than a free-text mode field.

The authored 2D joint slice has completed native visual review: all four
numeric payloads reopen, nested hinge overrides persist, editor Play switches
and rebuilds modes, and Stop restores the document. Play Save refuses to write,
confirmed by an unchanged file hash.

</details>

<a id="joint-controls"></a>

### Typed joint controls from Decay

Scripts tune, enable, retarget, remove and create all four 2D joint kinds, validated atomically before mutation.

Engine ✅ · Editor — · Decay ✅ · Proof ✅

**Tested by:** [`crates/sindri-decay/tests/hinge_motor.rs`](../../crates/sindri-decay/tests/hinge_motor.rs), [`crates/sindri-decay/tests/linear_joint_controls.rs`](../../crates/sindri-decay/tests/linear_joint_controls.rs), [`crates/sindri-decay/tests/joint_state_controls.rs`](../../crates/sindri-decay/tests/joint_state_controls.rs), [`crates/sindri-decay/tests/joint_state_controls/endpoints.rs`](../../crates/sindri-decay/tests/joint_state_controls/endpoints.rs), [`crates/sindri-decay/tests/joint_state_controls/removal.rs`](../../crates/sindri-decay/tests/joint_state_controls/removal.rs), [`crates/sindri-decay/tests/joint_state_controls/creation.rs`](../../crates/sindri-decay/tests/joint_state_controls/creation.rs), [`crates/sindri-decay/tests/joint_state_controls/hinge_creation.rs`](../../crates/sindri-decay/tests/joint_state_controls/hinge_creation.rs), [`crates/sindri-decay/tests/joint_state_controls/spring_creation.rs`](../../crates/sindri-decay/tests/joint_state_controls/spring_creation.rs), [`crates/sindri-decay/tests/joint_state_controls/slider_creation.rs`](../../crates/sindri-decay/tests/joint_state_controls/slider_creation.rs), [`games/platformer/tests/joint_controls.rs`](../../games/platformer/tests/joint_controls.rs)

**Exercised in:** [`games/platformer`](../../games/platformer)

<details markdown="1">
<summary>Evidence</summary>

Typed `Physics.set_hinge_motor`, `Physics.set_slider_motor` and
`Physics.set_spring` validate atomic runtime component edits for the next
synchronization, preserving unknown fields and rebuild behavior.
`Physics.joint_enabled` reads the authored flag and `Physics.set_joint_enabled`
suspends/reconnects the constraint at next fixed synchronization without
rebuilding endpoint bodies. `Physics.set_distance` tunes owned maximum-distance
constraints, including before initial sync and while suspended. Validation
rejects missing/conflicting/invalid owners, wrong kinds/types, missing physics
and non-positive/non-finite lengths before mutation. The platformer reels its
lantern tether with T and releases/reconnects it with L.

`Physics.set_joint_endpoints` retargets any of the four kinds. It accepts
scoped entity handles or null to clear an endpoint, storing stable scene
IDs/canonical prefab paths. Invalid scope, stale/unstable handles and identical
endpoints fail before mutation; inactive targets suspend until active.
Settings, enabled state, unknown payloads and body motion are retained. The
platformer switches its lantern between two hooks with R, including while
released.

`Physics.remove_joint` removes exactly one valid authored joint component,
releasing its owned constraint at the next fixed synchronization; the owner,
other components and bodies/motion survive. The platformer's Z cuts its lantern
cord.

`Physics.create_distance_joint`, `create_hinge_joint`, `create_spring_joint`
and `create_slider_joint` author an enabled component on an existing owner with
no joint of any kind. Scoped stable references and finite local
anchors/axes/lengths validate before mutation; null/inactive endpoints suspend.
Tests cover pre-sync creation, repeated removal/recreation, invalid arguments,
inactive/unbound targets and isolated runtime prefabs. The platformer repairs
its cut cord with C, rebuilds its windmill hinge with H, its lantern spring
with B and its trolley slider with J.

</details>

<a id="joint-position-motors"></a>

### Hinge and slider position motors

Damped force-based drives that hold a hinge angle or a slider separation, switchable with the velocity drive.

Engine ✅ · Editor ✅ · Decay ✅ · Proof ✅

**Tested by:** [`crates/sindri-decay/tests/joint_state_controls/hinge_position.rs`](../../crates/sindri-decay/tests/joint_state_controls/hinge_position.rs), [`crates/sindri-decay/tests/joint_state_controls/slider_position.rs`](../../crates/sindri-decay/tests/joint_state_controls/slider_position.rs), [`crates/sindri-scene/src/physics_joints/position_tests.rs`](../../crates/sindri-scene/src/physics_joints/position_tests.rs), [`crates/sindri-scene/src/physics_joints/slider_position_tests.rs`](../../crates/sindri-scene/src/physics_joints/slider_position_tests.rs)

**Exercised in:** [`games/platformer`](../../games/platformer)

**See:** [`docs/physics-joint-editor-review.md`](../../docs/physics-joint-editor-review.md)

<details markdown="1">
<summary>Evidence</summary>

Old payloads default to velocity mode. Typed `Physics.set_hinge_position_motor`
selects a relative angle within `[-pi, pi]`, finite non-negative
stiffness/damping and a torque cap; `Physics.set_slider_position_motor` sets a
finite signed anchor separation along the first local axis with a force cap,
and travel limits still apply. The velocity setters switch back explicitly.
Native tests exercise holding, retargeting, caps, limits, atomic rejection,
command undo/redo, scene round trips, suspension and collider rebuilds. The
platformer's P holds both windmills and resumes their reversal; O parks and
releases its trolley. Added for that showcase.

</details>

<a id="joint-prefab-references"></a>

### Authored joint references in prefabs

Joint endpoints inside prefabs resolve to their own instance, across spawning, placing, reloading and undo.

Engine 🟡 · Editor — · Decay 🟡 · Proof ✅ — parity row *Authored joint references in prefabs*

**Tested by:** [`crates/sindri-scene/src/physics_joints/prefab_tests.rs`](../../crates/sindri-scene/src/physics_joints/prefab_tests.rs), [`crates/sindri-scene/src/physics_joints/placed_tests.rs`](../../crates/sindri-scene/src/physics_joints/placed_tests.rs), [`games/platformer/tests/spawned_joints.rs`](../../games/platformer/tests/spawned_joints.rs), [`editor/src/prefab/joint_tests.rs`](../../editor/src/prefab/joint_tests.rs)

**Exercised in:** [`games/platformer`](../../games/platformer)

**Limits:**

- Flattened plain documents discard the alias metadata.

<details markdown="1">
<summary>Evidence</summary>

Runtime-spawned prefab references use a separate runtime identity, retaining
local sibling paths and the original top-level root ID without assigning saved
scene IDs. The Decay spawn host passes the original prefab library into
expansion; all four joint kinds reconnect only their own nested endpoints
across repeated spawns, inactivity and removal. Canonical paths shadow
aliases; ambiguous aliases fail before spawning. Placed instances retain
aliases through library-aware scene entry on native/browser hosts, scene
namespaces, editor reload/duplication and command undo. Qualified scene IDs
take precedence over relative namespace lookup when an exact match exists.

The platformer's reusable windmill nests its powered mechanism in
`windmill-kit.prefab`; Decay spawns and removes it, and the level places the
same nested assembly.

Registered entity fields have scoped native inspector choices, explicit
clearing and visible missing/inactive diagnostics through the runtime's shared
resolver. Real picker-click edits retarget/clear all four constraint kinds, and
command undo restores their constraints/unknown payloads.

</details>

<a id="saved-reference-remapping"></a>

### Saved spawned-prefab reference remapping

Saving a world remaps registered entity references to stable IDs, so spawned joint assemblies reopen intact.

Engine ✅ · Editor ✅ · Decay — · Proof ✅ — parity row *Saved spawned-prefab reference remapping*

**Tested by:** [`crates/sindri-core/tests/saved_prefab_references.rs`](../../crates/sindri-core/tests/saved_prefab_references.rs), [`crates/sindri-scene/src/physics_joints/saved_tests.rs`](../../crates/sindri-scene/src/physics_joints/saved_tests.rs), [`editor/tests/scene_joint_saves.rs`](../../editor/tests/scene_joint_saves.rs), [`games/platformer/tests/saved_joints.rs`](../../games/platformer/tests/saved_joints.rs)

**Exercised in:** [`games/platformer`](../../games/platformer)

**Limits:**

- Stable IDs remain a prerequisite.
- Save is unavailable during Play.

<details markdown="1">
<summary>Evidence</summary>

Opt-in `World::to_scene_with_references` remaps registered entity fields to
assigned stable IDs, including nested lists, without live edits. Unknown fields
remain unchanged, empty references stay unbound and
malformed/unresolved/unstable targets fail serialization.

Editor `SceneFile::save`/`save_as` and subtree prefab authoring use the active
registry automatically. Invalid references fail before file writes/path
adoption while unknown fields and the live world remain intact. Existing placed
instances still save as prefab references. Native editor tests reopen two
isolated instances of every joint kind, reuse saved subtrees and reject invalid
saves without changing disk or the agreed document. The platformer's real Decay
setup script spawns a nested windmill that editor Save As reopens with its
fixed axle and reversing motor.

</details>

## 3D

<a id="world-3d"></a>

### Standalone 3D physics world

`PhysicsWorld3d` runs the same Sindri-owned body and collider model in XYZ.

Engine ✅ · Editor — · Decay — · Proof ❌

**Tested by:** [`crates/sindri-physics/tests/world3d.rs`](../../crates/sindri-physics/tests/world3d.rs)

<details markdown="1">
<summary>Evidence</summary>

Native tests cover XYZ gravity, box/sphere/capsule solids, quaternion/local
offsets, compound mass, masks and collision/sensor events, kinematic targets,
velocity/impulse/teleport controls, rotation locking and atomic
validation/removal. The engine rejects quaternions outside the unit-norm
tolerance; all poses use `[x, y, z, w]`.

</details>

<a id="scene-physics-3d"></a>

### 3D physics in scenes

`ScenePhysics3d` reconciles 3D body, collider and world components with scene entities, and editor Play steps it.

Engine ✅ · Editor 🟡 · Decay 🟡 · Proof ❌

**Tested by:** [`crates/sindri-scene/src/physics_sync3d/tests.rs`](../../crates/sindri-scene/src/physics_sync3d/tests.rs), [`crates/sindri-scene/src/physics_sync3d/authoring_tests.rs`](../../crates/sindri-scene/src/physics_sync3d/authoring_tests.rs), [`crates/sindri-scene/src/physics_sync3d/hierarchy_tests.rs`](../../crates/sindri-scene/src/physics_sync3d/hierarchy_tests.rs), [`crates/sindri-decay/tests/physics3d_controls.rs`](../../crates/sindri-decay/tests/physics3d_controls.rs)

**See:** [`docs/physics-voxel-editor-review.md`](../../docs/physics-voxel-editor-review.md)

**Limits:**

- Visual scale is preserved but does not resize 3D colliders.
- Structural payload edits reset the rebuilt body's motion.
- Native inspector/Play interaction is unverified because local graphics adapter startup hangs or fails.

<details markdown="1">
<summary>Evidence</summary>

`ScenePhysics3d` reconciles registered body/collider/world components with
active scene entities, composed XYZ/quaternion transforms, gravity settings and
parent-space write-back. Twelve regressions cover lifecycle/edits, batch
rejection, solid/sensor events, targets, newer rotated/scaled parents and
checked command undo/redo/save/reopen. This is the prerequisite for Causeway
voxel collision proof.

Shared native/browser sessions and native editor Play step both dimensions
before scripts; fresh Play, Stop and scene replacement reset both editor
solvers. An exported scratch fixture exercises XYZ motion/landing in Chromium
with a read-only Decay observer and visible textured geometry. All three
components appear in the Physics menu.

Typed `Physics3d` adds copied Vec3 linear/angular velocities, setters and
dynamic impulses, plus all four sorted unique non-draining collision/sensor
event queries. Controls require an active body and leave authored motion
unchanged.

</details>

<a id="spawn-window-3d"></a>

### 3D spawn-window controls

Velocity and impulse requests on a body that has not materialized yet queue, replay after mass is known, and expire.

Engine 🟡 · Editor ❌ · Decay 🟡 · Proof ❌ — parity row *3D spawn-window controls*

**Tested by:** [`crates/sindri-physics/tests/pending3d.rs`](../../crates/sindri-physics/tests/pending3d.rs), [`crates/sindri-scene/src/physics_sync3d/pending_tests.rs`](../../crates/sindri-scene/src/physics_sync3d/pending_tests.rs), [`crates/sindri-decay/tests/physics3d_spawn.rs`](../../crates/sindri-decay/tests/physics3d_spawn.rs)

**Limits:**

- Game, voxel and editor proof remain open.

<details markdown="1">
<summary>Evidence</summary>

The standalone `BodyControl3d` queue validates setters/impulses before
insertion, replays them in order after compound mass is known and expires
unresolved requests at a successful synchronization. Complete-batch validation
includes queued kind conflicts and retains requests after failure.
`ScenePhysics3d` replays before solving. Typed bridge regressions exercise real
prefab spawning, copied pending reads, locks/kinds and invalid authoring. A
shared native-session regression and exported Chromium fixture verify actual
spawn replay and movement.

</details>

<a id="queries-3d"></a>

### 3D physics queries

Rays, overlaps and fixed-orientation shape casts over a per-piece BVH, reachable from typed `Physics3d`.

Engine ✅ · Editor ✅ · Decay ✅ · Proof 🟡 — parity row *3D physics queries*

**Tested by:** [`crates/sindri-physics/tests/queries3d.rs`](../../crates/sindri-physics/tests/queries3d.rs), [`crates/sindri-physics/src/world3d/spatial/tests.rs`](../../crates/sindri-physics/src/world3d/spatial/tests.rs), [`crates/sindri-decay/tests/physics3d_queries.rs`](../../crates/sindri-decay/tests/physics3d_queries.rs), [`crates/sindri-decay/tests/physics3d_probes.rs`](../../crates/sindri-decay/tests/physics3d_probes.rs), [`crates/sindri-decay/tests/physics3d_layers.rs`](../../crates/sindri-decay/tests/physics3d_layers.rs)

**Limits:**

- No shipped gameplay decision uses a 3D query yet.
- Backend shape casts at near-maximum finite coordinates/extents can return non-finite results.

<details markdown="1">
<summary>Evidence</summary>

Standalone 3D rays, overlaps and fixed-orientation shape casts use current
body/local poses even before the first step or immediately after teleports.
Native regressions cover all shapes, XYZ hits, rotated probes, filtering,
whole-entity predicates, sorted unique results, exact ties, initial overlap,
extreme finite directions and pending/solved poses. A query-only per-piece BVH
selects finite ray segments and probe/swept bounds, preserving entity/piece
order; differential regressions compare exhaustive candidates, and sparse XYZ
rays, overlaps and casts select one piece in 100/1,000/10,000-piece worlds.

Typed 3D rays, sphere overlaps and sphere casts use indexed synchronized
geometry, active-entity filtering, masks/sensors/whole-entity exclusion and
copied optional `RayHit3d`/sorted overlap lists. Typed `Physics3d.layer`/`mask`
read only the active authored 3D world's first 32 labels. Typed rotated
box/capsule overlaps and casts accept an arbitrary nonzero rotation axis and
radians, with f64 normalization before quaternion conversion.

</details>

<a id="static-groups-3d"></a>

### 3D replaceable static geometry groups

Keyed static collider groups under one real owner, replaced or removed without disturbing other handles.

Engine 🟡 · Editor ❌ · Decay ❌ · Proof ❌ — parity row *3D replaceable static geometry groups*

**Tested by:** [`crates/sindri-physics/tests/static_groups3d.rs`](../../crates/sindri-physics/tests/static_groups3d.rs), [`crates/sindri-voxel/src/collision_tests.rs`](../../crates/sindri-voxel/src/collision_tests.rs)

**Limits:**

- Authored Causeway, editor, Decay and browser group integration is absent; voxel collision reaches the solver through the scene adapter instead.

<details markdown="1">
<summary>Evidence</summary>

The standalone 3D solver owns keyed static collider groups under one real
entity. Replacement/removal retains other collider handles and the body handle;
queries update immediately with owner identity and canonical key/piece
ordering. A read-only validator supports complete scene-batch preflight.
Invalid geometry, nonstatic owners or incompatible pending controls leave the
world unchanged. Empty owners retain identity until full removal. Native
regressions include actual landing/events, deletion/falling, filters and
internal handle retention.

A renderer/physics-independent `compile_section_collision` derives exact
section-local collision boxes from occupied `VoxelSection` cells and a supplied
solidity/shape policy. Full cubes merge deterministically across material IDs;
partial slabs/posts retain exact bounds, air and policy-excluded blocks are
empty. Integer sixteenth-voxel bounds avoid world-coordinate precision loss;
invalid shapes return typed errors and output is bounded by 4,096 boxes per
section. Regressions prove exact cover without holes, overlaps or air, partial
shapes, worst-case bounds and edited negative sections across residency
unload/reload.

</details>

## Not yet

<a id="collider-scaling-3d"></a>

### Automatic 3D collider scaling

Scene 3D collider dimensions and offsets are world units; visual scale does not resize collision geometry.

Engine ❌ · Editor ❌ · Decay ❌ · Proof ❌ — parity row *Automatic 3D collider scaling*

<a id="query-numerics-3d"></a>

### Extreme 3D query numerical failure reporting

Near-maximum finite coordinates can produce non-finite shape-cast results, and the engine has no contract for reporting it.

Engine ❌ · Editor — · Decay — · Proof ❌ — parity row *Extreme 3D query numerical failure reporting*

<a id="world-snapshots"></a>

### Script-triggered world snapshot save/load

Decay's `Save` stores numbers and flags; there is no operation to persist and reopen a running world.

Engine ❌ · Editor — · Decay ❌ · Proof ❌ — parity row *Script-triggered world snapshot save/load*

<a id="character-controller-3d"></a>

### Character controller (3D)

The exercised controller and its scene/Decay integration are 2D only.

Engine ❌ · Editor ❌ · Decay ❌ · Proof ❌ — parity row *Character controller (3D)*

<a id="compound-character-probes"></a>

### Compound solid character probes

Scene controllers require one solid box, circle or capsule piece, because the movement primitive sweeps one convex shape.

Engine ❌ · Editor — · Decay ❌ · Proof ❌ — parity row *Compound solid character probes*

<a id="same-step-controller-response"></a>

### Same-step solver response to controller motion

Controller moves happen after the solve; dynamic response and sensors see them at the following solve, and no push impulse is applied.

Engine ❌ · Editor — · Decay ❌ · Proof ❌ — parity row *Same-step solver response to controller motion*

<a id="swept-controller-triggers"></a>

### Swept controller trigger events

Controller sweeps exclude sensors, so crossing a small sensor between solves can produce no event.

Engine ❌ · Editor — · Decay ❌ · Proof ❌ — parity row *Swept controller trigger events*

<a id="rotating-carry-arc"></a>

### Sweeping rotating platform carry along its arc

Platform carry sweeps the straight chord between support points; large rotations can miss obstacles on the arc.

Engine ❌ · Editor — · Decay ❌ · Proof ❌ — parity row *Sweeping rotating platform carry along its arc*

<a id="rotating-character-probe"></a>

### Sweeping a rotating character probe

Movement and carry keep the probe's rotation; there is no swept orientation change.

Engine ❌ · Editor — · Decay ❌ · Proof ❌ — parity row *Sweeping a rotating character probe*
