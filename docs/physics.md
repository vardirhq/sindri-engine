# Physics architecture

Status: accepted implementation direction for the physics feature track.

Sindri exposes Sindri physics. Rapier is a backend implementation detail and is
never part of the scene format, editor contract, Decay language surface, or game
API.

## Goals

- Support both 2D and 3D without designing a 2D API that later has to be broken.
- Keep Rapier out of `sindri-core`, serialized scene data, the editor, Decay, and
  Gather.
- Run physics on the engine's fixed simulation step, never on render delta.
- Keep runtime `EntityId` ownership authoritative; physics handles are private
  implementation details.
- Give 2D and 3D parallel concepts without pretending dimension-specific values
  are interchangeable.
- Keep native and `wasm32-unknown-unknown` on the same gameplay semantics.
- Evolve Gather with the 2D implementation so the subsystem is proven by a real
  game rather than only fixtures.

## Crate boundary

Create one `sindri-physics` crate at a real dependency boundary:

```text
sindri-core                 sindri-grid
     ^                           ^
     |                           |
sindri-physics -----------------+
     |
     +-- rapier2d
     +-- rapier3d

sindri-scene -> sindri-core + sindri-grid + sindri-render + sindri-physics
sindri-decay -> sindri-core + sindri-grid + sindri-platform + sindri-physics
editor       -> ... + sindri-physics
Gather       -> ... + sindri-physics
```

`sindri-physics` owns the public physics model and the Rapier adapters. A second
`*-rapier` crate is not justified while there is one backend. Rapier types may
appear inside private implementation modules only. If another backend ever
becomes real, the adapter can be split without changing the public contract.

The crate depends on `sindri-core` because bodies are synchronized to runtime
entities, and on `sindri-grid` only if a concrete physics/navigation integration
requires it. Do not add the grid dependency speculatively. It has no renderer,
window, browser, editor, or Decay-language dependency.

Both `rapier2d` and `rapier3d` must satisfy the repository MSRV, licence policy,
`cargo deny`, native builds, and `wasm32-unknown-unknown` before either is merged.

The adapters use Rapier 0.36. The 2D overlap adapter reads the backend
intersection flag while keeping sub-shape IDs private. The event collector
also owns a soft-body tear channel; Sindri currently authors only rigid bodies
and continues to publish collision and sensor events only.

## Public model

The concepts are intentionally parallel rather than generic over dimensionality.
Dimension-neutral configuration can be shared internally, but public APIs use
ordinary explicit types.

### Bodies

`RigidBodyKind` is shared:

- `Static`
- `Dynamic`
- `KinematicPosition`
- `KinematicVelocity`

The scene components are distinct:

- `sindri.physics2d.rigid_body`
- `sindri.physics3d.rigid_body`

They carry Sindri-owned configuration such as body kind, gravity scale, damping,
and whether rotation is locked. They never serialize Rapier handles, activation
state, solver state, or backend-specific flags.

### Continuous collision

`RigidBody2d.continuous_collision` defaults to false, including old payloads
that omit it. It opts a dynamic body into swept solid collision. The live
`continuous_collision` getter and `set_continuous_collision` setter preserve
velocity, attached joints and body identity. The setter rejects non-dynamic or
missing bodies before mutation.

The backend automatically sweeps dynamic bodies against fixed geometry; the
opt-in regression therefore uses a thin velocity-kinematic wall. Sensors remain
discrete, and bullet-versus-bullet collision is not guaranteed. The generic command-backed inspector authors the flag on the body. CCD-only
payload edits toggle the live backend without rebuilding its body, contacts or
joints; ordinary collider/kind edits retain their existing rebuild lifecycle.
`Physics.continuous_collision(entity)` and
`Physics.set_continuous_collision(entity, enabled)` expose the same control in
Decay. The setter requires an authored dynamic body, updates both the live
backend and runtime payload, and works before a spawned body is synchronized.
The getter uses live state when present and authored state during that window.
Missing authored bodies and hosts without physics report errors. The platformer
hero opts in, with its existing run-to-flag regression exercising the setting.

### One-way platforms

`OneWay2d` is a validated support-side policy: local `normal` defaults to
`[0, 1]`, `angle` defaults to pi/4 radians and is bounded to 0..=pi/2. Normals
must have finite, nonzero length. `sindri.physics2d.one_way` applies it to every
solid piece on the entity, including generated tilemap boxes. The normal follows
each piece's rotation and the body's rotation. Sensors retain ordinary overlap
and enter/exit behavior. Omitting the component means ordinary solid collision.

The backend filters contact pairs, including CCD sweeps, to reject relative
ascent above 0.1 units/s and bodies not yet above the support plane. Supporting contacts must
face the configured cone. Projection uses each convex piece's actual geometry;
a 0.05 world-unit penetration allowance stabilizes resting contacts. A body
starting deeply inside must clear the supporting face before it can land.
Fast descent still requires CCD to avoid discrete tunneling. Policies are live
controls: editing or undoing only the policy preserves velocity and joints.

`PhysicsWorld2d::drop_through(entity, seconds)` and typed
`Physics.drop_through(entity, seconds)` ignore only one-way solid contacts for
a dynamic body. Durations are finite and nonnegative: zero cancels, a new
request replaces the remaining time. Timers advance after each fixed step,
so any positive remainder covers the whole next step. Pause does not expire
them. Removal and structural body/collider rebuilds clear policy/timer state. The host validates the authored body;
requests before synchronization start when the spawned body materializes.
Abandoned requests are discarded at the end of synchronization.

Geometric rays/overlaps/casts still see the pieces from either side and during
drop-through. They do not themselves grant grounding. The platformer uses a
downward support normal, clearance and descending velocity in Decay, suppresses
jump permission while dropping, and keeps sensors for coins/the flag. Its
visible plank tilemap exercises ascent, landing, Down/S/d-pad drop to an ordinary
floor, and another landing after expiration. Engine regressions also exercise
rotated support normals, kinematic platforms, fast CCD ascent and cancellation.

### Forces and rotation

`PhysicsWorld2d` adds world-space force and torque, angular velocity in radians
per second, angular impulses and impulses at world points. Positive torque and
angular velocity turn counterclockwise. Forces and torques accumulate for the
next valid fixed step and are cleared immediately after it, even when a host
runs several fixed steps per frame. Invalid timesteps do not consume them.
Pause keeps them pending. Impulses change velocity immediately, without a dt
factor; off-centre impulses also turn the body according to its mass/inertia.

Forces and impulses require dynamic bodies. Velocity setters also support
velocity-kinematic bodies. Every request validates finite values and body kind
before mutation; rotation locks prevent angular motion while retaining linear
impulses. Collider mass properties are available immediately after insertion,
so impulses work before the first step. Existing velocity/impulse APIs remain.

Typed Decay adds `Physics.apply_force(entity, Vec2)`, `apply_torque(entity, f32)`,
`angular_velocity(entity)`, `set_angular_velocity(entity, f32)`,
`apply_angular_impulse(entity, f32)` and
`apply_impulse_at_point(entity, Vec2, Vec2)`. The point is in world space.
An authored body can receive these requests between spawning and synchronization:
requests replay in call order when the body materializes, after its collider
mass is known. The angular getter in that window returns the latest queued
setter or the authored initial value; queued impulses affect it at materialization.
Removal, abandoned synchronization and structural rebuilds discard pending work.
A host with its own physics driver can control live bodies without scene body
components. An authored body is required only to validate requests before
materialization. Missing bodies and hosts without physics fail explicitly.

The platformer's wooden crate is pushed by wind and tossed by K or the west
controller button. Decay reapplies force each update, applies torque while it is
airborne, bounds its angular velocity and gives kicks an off-centre and angular
impulse. The native input regression checks its launch and scene rotation
writeback. Browser interaction and visual inspector review remain in final
integration. Native, WASM and browser CI passed on the forces/rotation head.

### Contact snapshots

`PhysicsWorld2d::contacts(entity)` returns copied `Contact2d` values from the
last valid fixed step. Each value names the other entity and a world-space
surface-anchor midpoint, with a unit normal pointing towards the queried body
(the direction it is pushed). `normal_impulse` is nonnegative; signed
`tangent_impulse` acts along `[-normal.y, normal.x]`. `force` is their total
world impulse divided by that step's seconds. The adapter reads the manifolds
actually solved rather than cached geometric-point impulses. Rapier currently
disables clustering in 2D; the adapter uses its solver-manifold accessor, which
also selects clusters if the backend enables them in the future. Predictive
contacts within the solver's contact reach may be present.

Contacts are ordered by other entity, point, normal, normal impulse and tangent
impulse using total float ordering. Multiple points/pieces can name the same
entity; they are not deduplicated. Sensors are absent. Sleeping support remains
present with zero new impulses/force, so resting geometry can grant grounding
without replaying an old impact. Snapshots are empty before the first step;
missing bodies fail. Invalid steps retain the previous snapshot. Teleports,
removal and structural rebuilds invalidate affected contacts immediately.
Setting a kinematic target takes effect at the next step. Pausing retains the
last snapshot; copies held by a caller survive later changes.

Typed `Physics.contacts(entity) -> List<Contact2d>` exposes the same values in
Decay and omits inactive/despawned other entities immediately. Inactive queried
entities and authored colliders awaiting spawn synchronization return an empty
list. Active entities with no collider/body and hosts without physics fail.
Hosts driving their own physics need no authored component for live bodies.
Script updates observe the preceding fixed solve, as they do collision events.
Scene transforms synchronize at the next step, so a script teleport is reflected
in snapshots after synchronization, not midway through a script pass.

This capability is added for the platformer genre showcase: the wind crate
flashes amber on a hard landing based on normal impulse. The hero now uses
controller support for jump permission and a geometric ray for HUD clearance.
Gameplay policy and thresholds remain in Decay. Browser interaction and editor Play
inspection remain in final integration.

### Colliders

The scene components are distinct:

- `sindri.physics2d.collider`
- `sindri.physics3d.collider`

The first 2D shapes are box, circle, and capsule. The first 3D shapes are box,
sphere, and capsule. Shape enums are Sindri types. Dimensions are expressed in
scene/world units and validated as finite and positive.

A collider carries:

- shape
- local offset
- sensor/trigger flag
- collision membership mask
- collision filter mask
- friction
- restitution

Collision masks are Sindri bit masks from the first implementation. They are not
Rapier `Group` values in public APIs or serialized JSON.

A collider may exist without a rigid-body component; it is treated as a static
collider owned by the entity.

A 2D collider may be authored in **several pieces**, because one shape is often a
poor description of a thing: a character is a capsule with a circle at each side,
a ship a box and two pods. The pieces belong to the one entity and move as one
object — a compound is one collider made of parts, not several colliders — and
they need no child entities to say so, because a piece already carries its own
offset and rotation.

```jsonc
"sindri.physics2d.collider": {
  "pieces": [
    { "shape": { "shape": "capsule", "half_height": 0.4, "radius": 0.22 },
      "offset": [0.0, 0.05], "rotation": 0.0, /* … */ },
    { "shape": { "shape": "circle", "radius": 0.18 },
      "offset": [-0.38, -0.1], "rotation": 0.0, /* … */ }
  ]
}
```

The single form — a collider written directly, with no `pieces` — still means
exactly what it always did, and is kept rather than migrated: a scene is a file
someone wrote, and a format that can only be read after a rewrite is one that
breaks their project on upgrade.

Two consequences worth stating, because both are derived rather than authored.
Mass properties come from **every** piece and sum, so a body's centre of mass is
decided by the compound and not by its first piece; `PhysicsWorld2d::mass`
exposes the total so that claim can be checked. And validation is **per piece and
names the index** — a compound is a list, so "restitution must be between 0 and
1" without one is a needle in it. Nothing is inserted unless every piece passes,
because a half-built body is worse than none.

The standalone 3D engine world accepts a slice of collider pieces, validates all
of them before insertion and sums their mass. The 3D scene/editor surface remains
unimplemented. This general piece boundary is a prerequisite for voxel collision
geometry; it does not claim authored compounds or voxel/game proof.

An entity may not participate in both the 2D and 3D physics worlds at once.
Validation reports that as an authored configuration error instead of choosing a
world silently.

### Tilemap collision

`sindri.physics2d.tilemap_collider` makes the tilemap on the same entity solid,
so a painted level collides where it was painted and a wall is repainted rather
than re-placed:

```jsonc
"sindri.physics2d.tilemap_collider": {
  "passable": ["grass-tuft", "vine"],   // palette sprites that do not collide
  "layers": { "memberships": 1, "filter": 4294967295 },
  "friction": 0.5,
  "restitution": 0.0
}
```

Every painted tile is solid unless its sprite is passable. The solid cells are
merged, greedily in reading order, into rectangles that cover each once: a floor
becomes one box, not one per tile, because a character sliding along separate
boxes catches on the seams between them. The rectangles are ordinary collider
pieces, scaled with the entity, and join any pieces an authored collider on the
same entity has, so a tilemap on a kinematic body is a moving platform. Only an
orthogonal map can be solid; an isometric map is a floor seen at an angle and is
refused by name.

### Gravity

`sindri.physics2d.world` is the scene's say over the world as a whole:

```jsonc
"sindri.physics2d.world": { "gravity": [0.0, -9.81] }
```

A scene carries it rather than the host deciding, because a platformer falls and
a game seen from above does not, and the editor's Play must run each the way its
build will. One per scene, like the Environment. A scene without one keeps the
gravity its host constructed `ScenePhysics2d` with.

## Transform ownership

`Transform3D` remains the authored and visible transform for both dimensions.
Physics does not introduce a second scene transform.

For 2D, physics reads/writes X, Y, and rotation about Z. Existing Z and 3D scale
are preserved. For 3D it reads/writes XYZ and quaternion-equivalent scene
rotation through the existing transform representation.

Synchronization rules are explicit:

- static body: world transform seeds physics; checked authored changes update the
  backend body/collider before the next step;
- dynamic body: physics owns position/rotation while simulation runs and writes
  the resulting transform back after each fixed step;
- any body whose transform was moved by something other than physics since the
  last write-back (a script respawning the player, clamping it to the screen)
  is moved to the new transform before the next step: a teleport, keeping its
  velocity, as setting a Rigidbody2D's position does in Unity. A
  position-kinematic body takes the move as its next target instead, so a
  platform moved this way carries what stands on it;
- kinematic-position body: gameplay/editor supplies the target transform before
  the fixed step; physics resolves contacts from that motion;
- kinematic-velocity body: gameplay supplies velocity; physics owns the resulting
  transform for the step.

Scale is not simulated. Collider dimensions are authored explicitly. Changing an
entity's visual scale must not secretly mutate collision geometry. The one
exception is said out loud: a tilemap collider's rectangles are derived from
where its tiles are drawn, so they follow the map's scale.

Friction combines as the smaller of the two colliders' values, so a collider
authored frictionless is frictionless against everything: a platformer's hero
pressed into a wall slides down it rather than clinging with half the wall's
friction. Two equal frictions combine to that value, as an average would.

Parented 2D bodies are supported. Synchronization reads the composed world pose
and converts the simulated answer back into the parent's local space. Moving a
parent carries its bodies through the authored-pose change rule. This is exercised
by scene hierarchy regressions and Low Tide's moving deck; it does not create a
physical joint between parent and child.

## Reusable collision materials

Physics coefficients can be reused as project `.profile` assets with type
`physics_material`. They use the existing asynchronous profile pipeline, not a
new asset kind or backend resource. For example:

```json
{
  "format_version": 1,
  "name": "Wood",
  "type": "physics_material",
  "values": { "friction": 0.3, "restitution": 0.1 }
}
```

Both coefficients are required numbers. Friction must be finite and non-negative;
restitution must be finite in `[0, 1]`. Unknown coefficient keys are rejected.
Other profile types remain unrestricted game data. `physics_material_profile`
at the scene boundary uses the engine-owned `PhysicsMaterial::validate` rules;
asset decoders and core profile documents stay independent of physics.

An entity's `sindri.physics2d.material` names `profile` and applies it to every
ordinary and generated tilemap collider piece, sensors included. Without a
material component, existing per-piece literals are unchanged. An empty profile
keeps those literals. A non-empty reference must resolve to a loaded physics
profile; a missing or differently typed profile fails the fixed step. The profile
replaces both piece coefficients, then explicit `override_friction` and
`override_restitution` flags select the component's local `friction` and
`restitution` values. Disabled override values are ignored. A missing reference
is still an error even when both overrides are enabled. Friction combines by
minimum; restitution retains the existing average rule.

Hosts build `PhysicsMaterialSources` from their loaded profile documents before
stepping. Reloaded coefficients update registered colliders in place, preserving
velocity, forces, mass and joints, and waking affected contacts. Changes to shapes
or body kinds still use the normal rebuild path. Invalid source sets cannot
partly replace a valid source set; backend piece updates validate all values
before changing any collider.

The editor offers “New physics material here”, a structured profile editor,
profile selection and explicit override controls through checked component
commands and undo/redo. Its asynchronous loader watches valid and invalid files;
a failed edit reports the asset and retains the previous valid profile. Invalid
coefficient edits cannot overwrite the file through the profile editor. Play
waits for initial delivery before physics runs. Export walks scene and prefab
material references, including inactive entities, and validates physics profiles
before producing output; native and browser hosts use the same scene resolver.

Added for the platformer: its wind crate and one-way planks share
`materials/wood.profile`, with an explicit zero-bounce override on the planks.
Native and browser delivery expand placed prefabs during scene entry from the
original library, retaining root aliases for authored references. Native regressions verify sharing, precedence and a changed crate rebound when
the shared restitution changes. Editor loader/reload tests exercise delivery and
last-valid retention. Visual inspector review and full browser gameplay remain
part of final physics integration.

## Scene-authored distance constraints

`sindri.physics2d.distance_joint` lives on a separate joint entity:

```json
{ "first": "lantern-anchor", "second": "wind-lantern", "max_distance": 2.0 }
```

The endpoint strings are stable scene entity IDs, not runtime handles. Resolution
uses an exact qualified scene ID (a string containing `/`) before relative
namespace lookup when that ID exists inside the loaded scene (its namespace is
applied first). Otherwise it tries the owner's namespace
before the containing scene. It never binds
across loaded scene roots. An inactive local match does not fall through to an
external entity with the same name. Placed-prefab sibling IDs resolve within their
instance namespace. Original top-level/nested root aliases survive library-based
scene loading, namespacing, editor reload, duplication and undo. Canonical paths
precede aliases; inactive matches suspend without rebinding.
Runtime-spawned prefabs resolve through `World::prefab_entity` using runtime-only
instance identity. Local sibling paths and original top-level and nested root IDs
resolve within that spawn; missing or inactive targets never bind another instance or a
scene entity. Reparenting does not change ownership. All four authored joint
kinds share this path. After stable IDs are assigned,
`World::to_scene_with_references` remaps their registered endpoint fields for
save/reopen, preserving settings and unknown fields without editing the live world.
All four kinds exercise reopened endpoint isolation, inactivity and removal;
platformer proves motor reversal and a fixed axle after reopening its Decay spawn.
Flattening into a plain expanded document discards aliases; see `prefabs.md`.

Constraints synchronize after all bodies, so endpoint collider/body rebuilds
reconnect the owned joint in the same fixed step. Missing, empty, inactive or
collider-less endpoints suspend it without failing a step; they reconnect when
available. This also means a misspelled but valid ID waits rather than producing
a runtime diagnostic. Removing/disabling the joint entity or its component drops
its constraint. Removing an endpoint clears backend ownership; reappearance can
reconnect the authored constraint. Several joint entities may connect one body.

`PhysicsWorld2d::set_distance_joint(owner, DistanceJoint2d)` validates before
replacing an owned constraint; unchanged requests retain the solver joint.
Changing endpoints/distance replaces only that constraint, preserving body
motion. `remove_owned_joint(owner)` releases it independently of legacy
`connect_distance` connections. Both distance paths use the existing positive,
finite maximum centre distance, free rotation/closer motion and disabled mutual
endpoint contacts. Runtime owner IDs are never serialized.

The checked component inspector authors endpoint IDs with a typeable entity
picker, including names and an explicit None choice. Missing references and
inactive targets are marked; unresolved text remains editable. Choices obey the
same scene/prefab scope as runtime resolution. Real picker-click regressions
retarget/clear all four joint kinds through commands, then undo restores each
constraint and its unknown fields. Visual editor review remains in final integration.
This foundation is added for the platformer: a physical lantern hangs from an
anchor while Decay wind drives it and Decay stretches/turns its visible cord.
The game regression observes movement, bounded distance and tether removal.
Additional typed owned-joint controls and complete prefab references remain incomplete;
the existing typed `Physics.connect_distance` keeps its previous semantics.

## Reversible authored joint suspension and distance tuning

All four joint components accept `enabled`, defaulting to true when omitted.
False releases the solver constraint at the next fixed synchronization while
retaining its owner, endpoint references and settings. Re-enabling reconnects
available active endpoints; missing endpoints continue to suspend the constraint.
Bodies are not rebuilt, and their motion is not reset. Reconnection and restoring
mutual endpoint collision can change motion through the ordinary physics solver.
Multiple joint kinds on one active owner remain an error even when suspended.
All active owners' settings are validated before ownership changes, including
suspended constraints and distance settings whose endpoints are unavailable.

The generic checked inspector exposes the flag. Command suspension, undo/redo,
endpoint rebuilds and all four constraint kinds are exercised.
`Physics.joint_enabled(owner)` reads the authored flag, including before the next
synchronization; it does not report whether a live constraint currently exists.
`Physics.set_joint_enabled(owner, enabled)` patches that flag.
`Physics.set_distance(owner, max_distance)` tunes an owned maximum-distance
constraint, including while suspended or before endpoints materialize.
These typed calls require physics and exactly one valid authored joint;
distance tuning requires the distance kind and a finite positive maximum.
Invalid, missing, conflicting or wrong-kind calls leave payloads unchanged.
Unknown fields are preserved. Legacy `Physics.connect_distance` is unchanged.

Platformer adds these capabilities to let the player reel the hanging lantern
in/out with T and release/reconnect it with L. Decay hides the cord while
suspended or after its owner is removed. Runtime regressions observe length,
free fall, reconnection and return to the original length.
Typed `Physics.set_joint_endpoints(owner, first, second)` retargets every joint
kind through scoped authored references at the next fixed synchronization. Null
clears either endpoint. Both handles validate before mutation; stale, unstable,
out-of-scope or identical endpoints fail. Inactive references remain authored,
with the constraint suspended until active. Settings, enabled flags, unknown
fields and body motion are preserved. No runtime handles are serialized.
Platformer switches its lantern between two hooks with R, including while released.
`Physics.remove_joint(owner)` removes the authored joint component and releases
its solver constraint at the next fixed synchronization. The owner, other
components and endpoint bodies/motion remain; legacy `connect_distance`
constraints are unaffected. Exactly one valid authored joint and a physics host
are required, including while suspended or before initial body synchronization.
Subsequent joint controls fail until a joint is authored again. Platformer Z cuts
its lantern cord; Decay hides the cord and ignores tether controls until repaired.

`Physics.create_distance_joint(owner, first, second, max_distance)` authors an
enabled maximum-distance component on an existing owner carrying no authored
2D joint of any kind. Validation and scoped endpoint rules match retargeting;
null leaves an endpoint unbound, and inactive targets suspend until active.
The maximum length must be finite and positive. Physics is required, but endpoint
bodies need not be synchronized yet. Next fixed synchronization creates the
constraint, preserving other owner components, bodies/motion and legacy connections.
Existing joints, invalid handles/references and invalid lengths fail before mutation.
Platformer C repairs the cut cord using its selected hook and length.
Script-triggered world snapshots remain absent.

Editor scene Save/Save As and subtree prefab authoring now use the active
component registry for stable-reference remapping automatically. All entities
must already have stable IDs. Invalid local targets/types or unstable endpoints
fail before writing/adopting a path, without changing the live world. Tests reopen
two isolated instances of all four joint kinds and reuse saved subtrees. The
platformer's real Decay-spawned nested windmill reopens through editor Save As,
retaining its fixed axle and motor reversal. Save is refused during Play; Decay's
number/flag progress store does not save world snapshots. Script-triggered world
save/load remains a separate absent capability.

## Scene-authored hinges and motors

`sindri.physics2d.hinge_joint` uses the same separate owner entity, stable endpoint
reference resolution and suspend/remove/rebuild lifecycle as distance joints.
One entity owns one kind of constraint; active distance and hinge components on
one owner fail synchronization rather than silently replacing each other.

```json
{
  "first": "windmill-anchor", "second": "windmill-rotor",
  "first_anchor": [0.0, 0.0], "second_anchor": [0.0, 0.0],
  "limits_enabled": false, "lower_angle": 0.0, "upper_angle": 0.0,
  "motor_enabled": true, "motor_velocity": 2.0, "motor_max_torque": 1.0
}
```

Each anchor is measured in world units in its body's translated/rotated local
frame, without transform scaling. Anchors coincide while the bodies may rotate
relative to one another. The default anchors are the centres; default limits
and motor are disabled. Connected bodies do not collide with each other.
Enabled limits bound the second body's angle relative to the first, in radians;
finite bounds must satisfy `-pi <= lower_angle <= upper_angle <= pi`.
They describe relative orientation, not accumulated revolutions.

A velocity motor targets relative angular speed in radians/second; positive
turns the second body counterclockwise relative to the first. Its force-based
velocity drive has a finite non-negative torque cap, so target speed is not a
promise under load. A disabled motor coasts; an enabled zero-speed motor brakes
within its torque cap. All numeric fields, including disabled settings, must be
finite. Negative torque and invalid enabled limits fail before backend mutation.
Settings edits between the same endpoints update the existing solver constraint
and wake both bodies without resetting their velocity. Unchanged synchronization
retains it; ordinary low-motion sleeping behavior remains applicable.

`motor_mode` defaults to `"velocity"`, including old payloads that omit it.
`"position"` selects a force-based damped angular target: `motor_target_angle`
is the second body's relative orientation in radians within `[-pi, pi]`,
`motor_stiffness` is torque per radian of error and `motor_damping` is torque
per radian/second. Gains default to zero and must be finite and non-negative,
even when disabled. The existing torque cap and enabled angular limits apply;
a target beyond enabled limits cannot override them. Targets represent principal
relative angles, not accumulated revolutions. Edits retain body motion and
solver ownership; suspension and endpoint rebuilds retain the authored mode.

`Physics.set_hinge_position_motor(joint, target_angle, stiffness, damping, max_torque)`
selects position mode, validates the complete settings before mutation and
preserves unknown fields. Zero torque disables drive. The next synchronization
applies it, including calls before endpoint bodies exist. `set_hinge_motor`
explicitly selects velocity mode again. Platformer P switches both placed and
spawned windmills between holding 0.6 radians and their reversing drive; H
recreates the placed hinge and asks its script to restore its current mode.
Checked command edits, undo/redo and scene serialization exercise the new fields;
native backend tests prove holding, retargeting, caps, limits and atomic rejection.
Editor save-path integration and native visual review are exercised.

Typed `Physics.set_hinge_motor(joint, velocity, max_torque)` controls the hinge
**owner**, not a body. It requires a hinge component and a physics host, validates
before changing its runtime component, preserves unknown fields and applies at
the next fixed synchronization. Its values persist through endpoint rebuilds and
the valid spawn-to-synchronization window. Zero torque disables the motor rather
than braking; to brake, set zero velocity with positive torque. Typed
`Physics.remove_joint` releases the authored hinge at next synchronization.

`Physics.create_hinge_joint(owner, first, second, first_anchor, second_anchor)`
authors an enabled hinge on an existing owner with no authored 2D joint of any
kind. The scoped endpoint/physics contract matches distance creation, including
null/unbound or inactive targets and pre-body synchronization. Finite `Vec2`
anchors use body-local world units without transform scale; angular limits and
motor start disabled. All validation precedes mutation. The next fixed step
creates the owned constraint while retaining other owner components and bodies.
`set_hinge_motor` can configure drive immediately after creation. Platformer H
rebuilds its placed windmill hinge and restores its selected drive while the spawned
windmill remains independently owned.

The generic checked inspector authors hinge settings and command undo reverses
motor edits. Native visual review is exercised; script-triggered world snapshots
remain absent. This general capability is added for the platformer: Decay reverses
its powered windmill axle every two seconds while physics keeps the rotor on its
anchor. Native tests exercise offset anchors, limits, torque caps, coast/reverse,
invalid atomic edits, command undo, endpoint rebuilds and gameplay removal.
Script-triggered world snapshots remain a separate absent capability.

## Scene-authored sliders and springs

`sindri.physics2d.slider_joint` constrains perpendicular translation and aligns
two body-local axes; it permits signed translation along the first axis. Anchors
use the same unscaled local world units as hinges. `first_axis` and `second_axis`
are finite unit vectors (length tolerance `1e-4`), defaulting to `[1, 0]`. They
set the relative orientation the slider maintains, so a rotated first body makes
a rotated rail. The defaults have zero anchors, disabled limits and disabled drive.
Enabled travel limits require finite `lower_distance <= upper_distance`; negative
travel is allowed. `motor_velocity` is relative speed in world units/second along
the first axis; `motor_max_force` is a finite non-negative force cap. The drive is
force-based: it approaches its target subject to the cap and load. Disabling it
coasts; enabled zero speed brakes within the cap.

```json
{
  "first": "trolley-rail", "second": "trolley-body",
  "first_axis": [1.0, 0.0], "second_axis": [1.0, 0.0],
  "limits_enabled": true, "lower_distance": -1.0, "upper_distance": 1.0,
  "motor_enabled": true, "motor_velocity": 0.8, "motor_max_force": 4.0
}
```

Slider `motor_mode` defaults to `"velocity"`, including old payloads. `"position"`
uses `motor_target_distance`: finite signed separation of the two local anchors
along the first body's axis, in world units without transform scale. Stiffness
is force per unit of position error; damping is force per unit/second. Both
fields default to zero and must be finite and non-negative, even when disabled.
The existing force cap and enabled travel limits apply. A target outside enabled
limits remains valid but cannot override them. Settings preserve body motion and
solver ownership, including suspension/rebuild.

`Physics.set_slider_position_motor(owner, target_distance, stiffness, damping, max_force)`
selects position mode and validates before patching the runtime component,
retaining unknown fields. Zero force disables drive. It applies at next fixed
synchronization, including before bodies exist. `set_slider_motor` explicitly
returns to velocity mode. Platformer O parks its trolley at signed distance 0.5
or resumes reversal; J recreation restores the selected drive and direction.
This general capability is added for the platformer. Tests exercise rotated rails
with offset anchors, signed retargeting, caps, enabled limits, coasting, atomic
rejection, lifecycle, checked command undo/redo and canonical save/reopen.

`sindri.physics2d.spring_joint` applies radial, force-based spring and damping
between two freely rotating local anchors. It pulls extended anchors together
and pushes compressed anchors apart; it does not enforce a hard maximum length.
`rest_length` is finite and strictly positive; `stiffness` and `damping` are finite
and non-negative. Defaults are length `1`, stiffness `10`, damping `1` and zero
anchors. Stiffness is force per unit of extension; damping is force per unit
of relative radial speed. Zero stiffness permits damping alone; zero damping
permits oscillation.
Under gravity the supported weight stretches the spring beyond its rest length;
a regression measures this against `mass * gravity / stiffness`.

```json
{
  "first": "trolley-body", "second": "spring-lantern",
  "rest_length": 1.0, "stiffness": 20.0, "damping": 1.2
}
```

Both kinds share the joint owner, stable reference resolver and post-body
synchronization lifecycle. Connected endpoints do not collide with each other.
Invalid settings fail before backend edits; unchanged synchronization retains
one constraint. Settings edits on unchanged endpoints update it in place and
wake the bodies without resetting their motion. Owner/component removal drops
it; inactive or missing endpoints suspend it; endpoint rebuilds reconnect in the
same fixed step. The generic checked inspector and command undo author these
settings. Native visual review is exercised; script-triggered world snapshots
remain absent. All numeric fields are validated even when their toggle is disabled.

Typed `Physics.set_slider_motor(owner, velocity, max_force)` and
`Physics.set_spring(owner, rest_length, stiffness, damping)` validate and patch
only their fields in the runtime component, preserving unknown payload fields.
They require the appropriate component and a physics host. Values apply at the
next fixed synchronization, including before endpoints are built, and survive
rebuilds. Zero force in the slider call disables its motor rather than braking.

`Physics.create_spring_joint(owner, first, second, first_anchor, second_anchor,
rest_length, stiffness, damping)` authors an enabled spring on an empty
owner under the same scoped endpoint and physics contract as distance/hinge creation. Finite
`Vec2` anchors use body-local world units without transform scale; rest length
must be finite and positive, stiffness/damping finite and non-negative (zero is
valid). All validation precedes mutation. Next fixed synchronization creates the
owned constraint, preserving other components, body motion and legacy connections.
`set_spring` can retune the new component before synchronization. Platformer B
rebuilds its light's spring at its current rest length and continues tuning while
the trolley remains on its independent rail.

`Physics.create_slider_joint(owner, first, second, first_anchor, second_anchor,
first_axis, second_axis, limits_enabled, lower_distance, upper_distance)` authors
an enabled slider on an existing empty joint owner, sharing the scoped endpoint
and physics contract. Finite anchors are body-local world units without transform
scale, and finite axes must be unit vectors. Bounds validate even when disabled;
enabled lower distance cannot exceed upper distance. Its motor starts disabled;
`set_slider_motor` can configure the new component before synchronization.
Invalid settings fail before mutation. Next fixed synchronization creates the
owned constraint while retaining other components, body motion and legacy
connections. Platformer J rebuilds its trolley slider, restores its selected
drive and keeps the separately owned spring attached. Native visual review is
exercised; script-triggered world snapshots remain absent.

These general capabilities are added for the platformer: Decay reverses a
powered lantern trolley near its rail ends, retunes its suspended light's spring
and draws the cord from solved positions. Its regression observes both travel
directions, bounded rail motion, changed light height and independent removal.
Runtime regressions exercise rotated rails, travel limits, force caps, coast/brake,
spring extension/compression, damping, weight support and atomic invalid edits.
Prefab endpoint references, velocity/position motors, editor save paths and
native editor interaction are now exercised. The [native joint review](physics-joint-editor-review.md)
records numeric Save/reopen, Play controls and Stop restoration. The 2D joint
acceptance slice is complete; 3D joints, gameplay world snapshots and final
workspace integration remain separate work.

## Runtime ownership and stepping

`PhysicsWorld2d` and `PhysicsWorld3d` are separate runtime state beside `World`,
not serialized state inside it. Each owns its private backend and maps `EntityId`
to private handles. Current scene/game/editor hosts synchronize only the 2D world.
Despawning an entity removes its body/collider before the next
step; generation-checked IDs prevent a reused slot from inheriting old physics
state.

The host advances physics exactly once for each engine fixed update using that
fixed step duration. Render frames never step Rapier directly. Pause therefore
pauses physics automatically, and time-scale/fixed-step semantics remain engine
semantics rather than backend semantics.

The fixed-update order for the first slice is:

1. apply deferred/checkable gameplay writes;
2. synchronize static/kinematic authored state into physics;
3. step physics with the engine fixed `dt`;
4. apply queued controller sweep/slide against current solved poses;
5. synchronize dynamic/velocity-driven/controller results back to `World`;
6. publish normalized collision/sensor events;
7. run consumers that are defined to observe post-physics events.

Controller velocity is held at zero and fresh support is seeded after
synchronization, before the solve. Controller movement updates its collider
pose after the solve, so solid response and discrete sensor events observe that
move at the following solve; same-step response and swept controller triggers
remain absent. Scenes without controllers retain their existing behavior.

The exact system-ordering API is still an open roadmap item, so physics must not
quietly invent a general scheduler. Its ordering is documented as part of the
engine fixed-step contract until that scheduler exists.

## Events and queries

Backend events are normalized to Sindri events containing `EntityId`s and Sindri
collider semantics only:

- collision started / stopped
- sensor entered / exited

Contact manifolds and raw solver data are deliberately not first-slice public
API. Add them only for a demonstrated gameplay requirement.

The implemented query is `PhysicsWorld2d::raycast(origin, direction,
max_distance, RaycastFilter2d) -> Result<Option<RayHit2d>, PhysicsError>`.
`raycast_where` additionally accepts an entity predicate for a scene host to
exclude inactive/despawned entities immediately. A query-only per-piece BVH
selects candidates at current body poses, including offsets and rotations. It
updates immediately on insertion, removal and teleports, and after each completed
solver step; it does not depend on the solver broad phase being current.

Direction is normalized, distance is non-negative and inclusive, and geometry
inputs must be finite. Zero direction is rejected. Normalization accepts even
extreme finite directions; an overflowing segment endpoint is rejected. The
filter mask matches collider memberships independently of collision pair
filters, sensors are excluded by default, and excluding an entity excludes its
whole compound. Mask 0 misses. Exact ties prefer the smaller entity handle,
then authored piece order. Inside/on hits report distance 0, the origin point,
and normal (0, 0). Other hits report the world-space surface normal.

`RayHit2d` is a snapshot. Scene transform writes and new colliders take effect
at the next synchronization. Decay skips inactive/despawned entities even during
a script pass. The platformer proves ground clearance against tilemap geometry;
`examples/physics` visualizes filtering, hits, normals and trigger events.

2D overlaps and shape casts are implemented for circles, boxes and capsules.
Decay exposes `Physics.overlap_circle`, `overlap_box`, `cast_circle` and
`cast_box`; Orbital's mine blast and Physics Playground exercise them.
2D rays, overlaps, shape casts and controller penetration/sweep/ground phases
use the query index. Previous platform
support is reconstructed directly from that known entity's pieces.
The incremental physics update is tracked in `docs/physics-update.md`.

The 2D index uses one leaf per registered collider piece. Ray traversal tests bounds
against the finite segment; overlaps use probe bounds; casts use the union of
start/end bounds at fixed orientation. Candidates are sorted by entity handle and
piece order before the unchanged exact geometry/filtering phase. Predicates are
called once per candidate entity, rather than for every registered entity; callers
must not depend on visits to remote geometry. Masks, sensors, exclusions, inside
hits and exact ties retain their previous results. Controller skin support
expands probe bounds by contact prediction; controller casts expand start/end
bounds by the target separation plus contact allowance. Initial penetration
uses the probe bounds. Step, snap and carry collision phases share these paths.
Previous-platform verification reads only the known support entity, reconstructing
its historical piece poses regardless of where its current index bounds lie.

Only non-static bodies need post-step refresh. Position-kinematic targets remain
pending until the solver step, while other body teleports update immediately.
Scene collider edits retain their existing remove/reinsert synchronization path.
Outward bounds padding protects touching geometry; overflowing bounds take a
conservative exact-scan fallback instead of introducing new validation failures.
Work is output-dependent: a dense/long query may still select every piece, and
sorting costs O(k log k) for k candidates. Refresh is proportional to registered
non-static collider pieces, including sleeping bodies. No backend types escape
and no new dependency is introduced.

Results contain Sindri entity IDs, hit position/normal in the appropriate vector
dimension, and distance. They never expose Rapier collider handles.

The standalone 3D world exposes `raycast`/`raycast_where`, `overlap`/
`overlap_where` and `shape_cast`/`shape_cast_where` using `RaycastFilter3d`,
`RayHit3d`, `ShapeHit3d` and quaternion `PhysicsPose3d` values. Queries reconstruct
current body/local poses, independently of the solver cache: inserts and ordinary
teleports are visible before stepping; position-kinematic targets remain pending
until the solve. They scan entities sorted by handle and pieces in authored order;
3D acceleration is still absent. Stable predicates run once per non-excluded
entity, and hosts must not rely on exhaustive predicate visits when indexing lands.

Membership masks, sensor opt-in and whole-entity exclusion match the 2D contract.
Overlaps return sorted unique entity handles. Rays/sweeps normalize finite XYZ
directions in f64, accept extreme finite magnitudes, reject zero directions,
negative/non-finite distance and overflowing endpoints. Rays include the segment
endpoint. Ties prefer entity handle then authored piece order. Inside/on rays
return the origin with zero distance/normal; penetrating shape casts return the
probe origin with zero distance/normal. Non-penetrating casts report the touched
piece's world-space witness and outward normal. Sweeps keep probe rotation fixed;
box/sphere dimensions must be positive, and query capsules allow zero half-height
with positive radius. Pose validation uses the body's quaternion tolerance.

Native tests exercise these engine APIs. Scene/Decay/editor access and game/voxel/
browser proof are still absent; a unit-tested query is not a completed surface.

## Editor

The inspector authors the Sindri body/collider components through the existing
checked command and undo/redo path. It must provide:

- body kind and shared physical properties;
- dimension-appropriate shape and dimensions;
- sensor toggle;
- collision membership/filter masks;
- friction and restitution;
- validation errors in the inspector rather than backend panics.

The Scene view draws every 2D collider's outline from the pieces the physics
world is given, tilemap rectangles included, and the selected collider's pieces
carry drag handles (a box's edges, a circle's radius, a capsule's radius and
height), each drag one undo step. The 3D shapes have no gizmos yet.

The editor never links against or imports Rapier types directly; it consumes the
public `sindri-physics` model/schema.

## Decay

Decay exposes typed Sindri physics operations, not Rapier terminology. The 2D
surface comes with the first vertical slice; 3D names are reserved by the design
but are not claimed implemented until the 3D slice exists.

Initial 2D gameplay operations should cover:

- get/set linear velocity;
- apply impulse;
- sensor/collision event observation;
- 2D overlap/shape casts when a real game needs them; closest-hit raycasts are implemented and proven by platformer clearance.

Dimension is explicit in names/types where ambiguity would otherwise exist. The
language workspace remains independent: `decay/` gains no Sindri dependency;
`sindri-decay` performs all conversion and host calls.

## Gather proof

The first 2D physics track is incomplete until Gather visibly depends on it.
Gather should evolve so that:

- authored walls/obstacles have static colliders;
- the player has a collision-constrained body/collider rather than walking
  through those obstacles;
- pickups use sensor enter events instead of a hand-written distance check;
- at least one hazard interaction uses collision or sensor events;
- native and browser Gather exercise the same physics semantics.

Pathfinding remains navigation, not physics. The Wisp may plan through the grid,
but collision remains authoritative for physical overlap. Do not couple Rapier
into `sindri-grid` to make those systems agree implicitly.

## 3D engine foundation and remaining integration

`PhysicsWorld3d` now implements standalone fixed-step simulation over Sindri-owned
`RigidBody3d`, `Collider3d`, `PhysicsPose3d` and `PhysicsEvent3d` values. Vectors are
XYZ; body and local collider rotations are unit quaternions in `[x, y, z, w]`
order. Quaternions must be finite with squared norm within 0.0001 of one; zero or
non-unit rotations outside that tolerance fail explicitly; accepted near-unit
values normalize at the backend boundary. Shapes are boxes, spheres and Y-axis
capsules with finite positive dimensions. Friction
is non-negative, restitution is in [0, 1], and damping is non-negative.

Bodies support static, dynamic, position-kinematic and velocity-kinematic kinds.
Insertion validates the whole body and every piece before mutating the backend;
errors identify the failing piece. Compounds use local offsets/rotations and sum
mass. Gravity and scale, damping and all-axis rotation locking are engine policy.
Locked rotation clears authored initial angular velocity and angular setters.
Velocity/angular-velocity setters accept dynamic or velocity-kinematic bodies;
impulses require dynamic bodies. All controls reject invalid values before
mutation. Teleports preserve velocity; position-kinematic moves/targets take effect
at the next fixed step. Missing handles fail instead of queuing future spawns.

One positive finite engine `Duration` advances exactly one step. The engine
returns solid/sensor start/stop events with sorted entity pairs and dimension-
specific event values; event stream order remains backend-owned. Membership/filter
masks use the same mutual-pair rules as 2D. Removing an entity removes all of its
pieces and drops events whose handles no longer map to live entities, preventing
reused backend slots from inheriting old identity. No backend types escape.

Native tests exercise XYZ integration, gravity scale, all shapes landing on solid
geometry, rotated/offset collision, quaternion round-trip, masks/sensor enter/exit,
kinematic target timing, compound mass/impulse, teleport velocity, rotation lock,
atomic rejection and removal/reuse. WASM compilation is a separate gate; no 3D
browser simulation is claimed until the shared host and game exercise it.

This is the engine prerequisite for the physics-update voxel-world proof in
Causeway. 3D acceptance remains unchecked: query indexing, scene transform and
lifecycle synchronization, gravity authoring, editor checked commands/Play,
typed Decay Vec3 controls/events and actual resident/edited voxel collision remain
separate slices. Voxel collision must derive from the occupied world, account for
residency/dirty revisions and removal, and have a bounded update policy. An
invisible plane or this standalone API is not voxel/game proof. CCD controls,
contacts, force/torque, materials, joints and character controllers also have no
3D authoring/Decay surface yet. The 2D API remains unchanged.

## Feature-track slices

Implement this in reviewable PRs rather than one giant patch:

1. **Physics foundation.** Add `sindri-physics`, dependency-policy changes,
   Sindri-owned 2D/3D public model, backend masking, fixed-step 2D runtime,
   collision layers/events, native + WASM tests. Do not mark Editor or Script
   complete.
2. **2D authoring.** Register/serialize the 2D scene components, inspector
   controls, checked edits, undo/redo, and collider gizmos.
3. **Decay 2D.** Add typed velocity/impulse/event access through `sindri-decay`
   with language/host contract tests.
4. **Gather physics.** Convert obstacles, pickups, player movement, and one hazard
   interaction to the real physics path; prove native/browser behavior and update
   capability docs/matrices.
5. **3D physics.** Establish the standalone runtime first, then integrate
   scene/editor/Decay surfaces with a real 3D proof that exercises them end to end.
   The engine foundation alone does not complete this track.

Each PR runs all checks relevant to its dependency and target surface. A slice
that introduces Rapier must run `cargo deny`; a slice touching browser-reachable
physics must compile WASM; the Gather slice must run the real browser smoke test.

## Non-goals for the first 2D slice

- richer joints (a runtime maximum-distance joint now exists)
- continuous-character-controller abstraction
- further collider shapes (authored 2D compound colliders now exist)
- persistent contact IDs and mutable contact-manifold scripting
- physics-driven visual scale
- platformer navigation
- backend selection/plugin API
- deterministic cross-platform floating-point lockstep

Those are not forbidden forever. They are deliberately absent until a real game
needs them.

### Joint motor authoring choices

Hinge and slider `motor_mode` fields are registered choices derived from
`MotorMode2d`: `velocity` and `position`. The generic inspector offers these
choices through its existing checked-command path, including the default
velocity mode for old payloads that omit the field. Selecting a mode preserves
other settings; it does not enable the motor or reset its target or gains.
This closes an authoring gap found while reviewing the platformer mechanisms.

## Geometric movement foundation

`PhysicsWorld2d::move_and_slide` computes read-only 2D swept displacement with
positive skin, bounded iterations and explicit penetration/budget outcomes.
It uses current poses and existing query filtering; ordinary shape casts retain
their zero-normal initial-overlap contract. See [character movement](character-movement.md)
for the result semantics, validation, ownership and remaining vertical slices.
This is not yet the full character controller; one-way geometry is two-sided
in this primitive. No editor, Decay or platformer integration is claimed.

Ground probes now report nearest support and classify world-space normals
against configurable up and slope limits, including zero-travel contacts at the
skin. Steep surfaces are reported, not skipped; initial penetration is unwalkable.
The [character movement contract](character-movement.md) defines tolerances
and remaining controller movement policy. Ordinary geometric
queries are unchanged; this is not yet controller or game proof.

`move_and_slide_grounded` composes sliding with post-move support and optional
snapping using one shared skin. Snap defaults off; positive downward travel is
accepted only for walkable support. Upward requests and initial penetration
suppress snapping and grounded state. The result separates slide motion, support,
snap and total translation; gameplay applies total translation once and owns
prior support state. Native tests exercise landing, ledges, blocked ascent,
repeated support and downhill snapping.
One-way controller policy and scene ownership are now implemented; checked
editor/Decay/platformer proof remain open.

Grounded sliding also enforces `max_slope_angle`: upward-facing steep contacts
cannot create rise beyond the positive remaining request. Horizontal approaches
stop at steep slopes; explicit jumps can slide with bounded rise and steep descent
remains possible without support or snap. Walkable slopes, vertical walls and
ceilings keep geometric projection. Motion and support share the slope tolerance;
ordinary `move_and_slide` remains unrestricted. No gameplay gravity or constant
slope speed is supplied. See the character contract and native slope regressions.

Optional grounded steps use `step_height` (default zero) and require initial
walkable support, no ascent, clear full-height lift, improved forward progress,
a walkable landing within the height cap and final non-penetrating support.
Ceilings, overhangs, tall obstacles and absent/steep support retain the ordinary
path. The result separates step lift, selected forward slide and downward landing;
apply their total translation once. Full-height clearance and skin-sized minimum
progress are conservative limits. Impact contact geometry now refines movement
cast normals, preventing large artificial hops on flat box faces. Ordinary
ray/overlap/shape-cast contracts are unchanged; see the character contract/tests.

Opt-in `platform_support` captures the previous synchronized support entity/pose.
Grounded movement verifies old walkable contact using current local pieces, derives
the origin's displacement through previous/current body poses and sweeps carry
before character motion. The support alone is excluded during carry, then restored
for ordinary slide/step/ground queries. `platform` reports requested and actual
carry plus collisions/current pose; total translation includes it once. No velocity
inheritance or persistent snapshot state is supplied. Hosts advance/clear snapshots
for grounding changes, teleports or structural edits and avoid additional parent/
solver motion. Wall clipping and ceiling crush remain explicit. Rotation follows
a chord with fixed probe orientation, not an arc/orientation sweep. See the
character contract for ordering, limitations and native kinematic/geometry tests.


Grounded controller phases now respect per-piece one-way support sides/cones;
request-scoped `drop_through` ignores only one-way solids while ordinary geometric
queries remain two-sided. Scene-authored `sindri.physics2d.character` owns one
solid collider probe (additional sensors allowed) and derives a stationary
kinematic body, rejecting simultaneous rigid-body ownership. Runtime
`CharacterRequests2d` queues displacement/snap permission and timed drop-through;
`ScenePhysics2d` seeds fresh support, captures synchronized poses, applies movement
once and exposes cached results through `character_motion`/`CharacterMotions2d`.
Settings, parenting, teleports, removal and structural edits clear old state;
positive drop remainders cover a whole pass and decrement afterward. Saved
component data contains settings, never input queues or runtime support handles.
No game rules are supplied. See the character contract for frame ordering,
query masks, transform semantics, sensor timing and remaining editor/Decay/game
integration. Compound character probes, same-step solver response and swept
controller triggers remain absent.
