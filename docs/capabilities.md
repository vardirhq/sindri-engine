# What Sindri can do

An inventory of what exists, what is drawn but does nothing, and what is
missing. `ROADMAP.md` says what is planned and in what order; this says what is
true today.

It exists because the two are easy to confuse. A roadmap full of ticked boxes
reads like a capable engine, and an editor full of buttons looks like a working
tool. Twice now something has been described as working because it was written
down rather than because it ran.

## Keeping this current

**This file is updated in the same commit as the change it describes**, not
afterwards and not in a follow-up. A list that lags is worse than no list,
because it is trusted and wrong — which is the exact failure it exists to
prevent.

Update it when a change:

- adds a capability, to either the engine or the editor
- **wires up a control that was drawn but inert** — move it out of "Drawn, but
  does nothing" rather than leaving it in both places
- removes a capability, or deletes dead chrome
- turns something in "Not yet" into something that is
- discovers that an entry here is wrong

That last one matters as much as the rest. Every entry should be something
someone ran, not something they read in a roadmap or inferred from a type
signature. If you cannot demonstrate an entry, correct it or delete it.

---

## Engine

### Lifecycle and time

Legal state transitions are enforced rather than assumed: an engine can be
created, started, paused, resumed, and stopped, and an illegal transition is a
`LifecycleError` rather than a silently ignored call. Frame time is capped so a
long stall cannot produce one enormous step, and fixed-step accumulation runs
simulation at a steady rate with a bound on catch-up steps that prevents a
spiral of death. Time scale is rational rather than floating-point, so slow
motion does not accumulate drift. `ManualClock` drives the whole loop from a
test with no window and no sleeping.

### World and entities

Entities are generation-checked slot handles, so a handle to a despawned entity
is detected rather than silently addressing whatever took its place. Spawning,
access, recursive destruction, and slot reuse are all safe. Hierarchies support
reparenting with cycle prevention, and `World::check_set_parent` answers whether
a move is legal without making it. Each entity carries a name, a parent,
children, an optional `Transform3D`, arbitrary JSON components, and an
editor-only section the runtime never interprets. There is one transform, with
2D-shaped accessors that read and write X and Y and cannot express a change to
Z, and a Z lock a transform can declare that the command layer refuses to write
past: a 2D entity is one that keeps to a plane, not one with a different
transform type —
see `docs/2d-model.md`.

**A child's transform is local to its parent.** Moving, turning or growing a
parent carries its children, and `World::world_transform` is where an entity
ends up with every parent folded in. Everything that places something in the
world reads that: sprites, shapes, meshes, tilemaps, tile volumes, voxel worlds,
cameras and their follow targets, lights, grid placement, particle bursts, and
2D physics, which starts a child body at its world pose and writes its answer
back as the local transform that puts it there. The editor draws and drags a
child's handles in the world and stores the local result, and moving an entity
to a new parent in the hierarchy keeps it where it was on screen. Scripts'
`transform.position` is the stored local value and `world_position` the world
one; `World.set_parent` keeps the local transform. Scale composes per axis,
which is exact unless a rotated parent is scaled unevenly. Shapes composed this
way already; the rest ignored the parent, so a child sprite drew at its offset
from the world origin, a child's particle burst fired there, and a script had
to place a follower by hand every frame. Screen UI keeps its own layout, which
already placed an element inside its parent's box.

Entity storage was measured at 1k, 10k, and 100k entities before considering an
archetype ECS; `docs/entity-scaling.md` records why one is not warranted.

A **prefab** is an authored reusable entity definition: a single-root scene
fragment in the same document shape a scene uses, sharing its entity rules,
component payloads, versioning, and canonical serialization. `World::spawn_prefab`
creates the whole subtree or none of it, and answers with the root, every entity
made, and which authored identity each became. Instances carry no `source_id`,
because a prefab's identities name entities inside the prefab and two instances
would collide on every one. `docs/prefabs.md` is the contract.

A scene places a prefab as an **instance**: an entity naming the prefab, with
its own ID, parent, name and transform and nothing else but its overrides — a
JSON merge patch per component, keyed by the prefab's entity IDs. Loading
expands it into the prefab's entities (`coin-3`, `coin-3/sparkle`), each
carrying a `PrefabLink`; saving writes it back as the reference, working the
overrides out as the difference from the prefab. Prefabs nest, and an
override reaches inside by path. The editor keeps instances linked: an edit to
the prefab reaches each one as a single undoable reconciliation that keeps its
overrides, and the inspector reverts, applies and unpacks them. Hosts that play
a scene expand it once with `SceneDocument::expanded`; the exporter ships every
prefab a scene places. The platformer's ten coins are instances of
`prefabs/coin.prefab`. An instance can do without some of its prefab's
entities (`removed`), patch a list by index, and keep its inner entities'
editor state.

### Scenes

Scenes are versioned JSON documents with stable authored IDs kept separate from
runtime handles, so saving a loaded scene reproduces the authored identities
rather than inventing new ones. Loading validates duplicate IDs, missing
parents, and hierarchy cycles. Unknown component payloads are preserved by
default rather than dropped, which is what makes a scene written by a newer
editor survive an older runtime.

Serialization is canonical and a fixed point: entities and keys are sorted,
empty sections omitted, short scalar arrays kept on one line. Saving an unedited
scene reproduces the file byte for byte, which is what makes saving safe to
offer at all. Golden fixtures enforce it. A migration API exists before a second
format version does.

### Commands and undo

Every world edit can go through a `WorldCommand` that produces its own inverse:
set name, set the transform, set parent, set or remove a component.
`Transaction` is all-or-nothing, and `CommandHistory` gives bounded undo and
redo with labelled steps and merge runs, so a continuous drag collapses into one
undoable step rather than several hundred. The history also numbers the state
the world is in, so a tool that remembers the number it last saved can tell
whether the world and the file still agree — including after undoing back to it.

### Input

Keyboard, mouse, and touch input arrive as platform-independent events and
accumulate into an `InputState` that answers held, pressed-this-frame, and
released-this-frame for keys and mouse buttons, plus pointer position, pointer
delta, scroll delta, and window focus. `axis(negative, positive)` returns -1, 0, or 1 and gives zero
when both are held, so opposed movement keys cannot resolve by event order.

Touch is held beside the mouse rather than folded into it, because they are
different facts: a mouse has one position and is always somewhere, while fingers
arrive and leave and there may be several. Fingers are bounded at ten, ordered
by the id their host gave them so one keeps its place while it stays down, and
let go of when a window loses focus — a finger cannot be reported as lifted once
the window has stopped hearing about it. What *unifies* the two is a separate
question and is answered where a game reads it: `pointer_position` is the mouse
if there is one and the first finger otherwise, and `pointer_down(Left)` is the
left button or any finger.

The editor routes all of it through the Game view during Play, in **that view's
own pixels**, so a script reads the same position there as in the real build.
A pointer outside the view is reported as gone rather than clamped to its edge,
because a game told the person is pointing at somewhere they are not is worse
than a game told they are not pointing at all.

Gamepads are read on desktop and in the browser (`GamepadReader`, over `gilrs`,
behind the platform's default `gamepad` feature, which links libudev on Linux
the way audio links ALSA). Every host that draws a window asks for them once a
frame, and so does the editor's Play. A game reads pads by **player slot**
rather than by device: a face button or Start on an unclaimed pad claims the
first free slot, a slot stays claimed while its pad is connected and is given
back when it goes, and joins and leaves are one-frame edges that never name
two slots at once, so a second pad pressing in the same frame joins the next.
The press that claims a slot is not reported through it, so joining does not
also jump. Slot 0 reads every pad, for a game with one player. Buttons are
named by position (`south`, `right_bumper`, `dpad_up`), sticks read in screen
axes with a round dead zone taken out, and focus loss lets go of every button
without dropping a player. Scripts read `Gamepad`; an action binding reads any
pad as `gamepad.south` or `gamepad.axis.left_x`.

Input actions are declared by a scene's `sindri.input.actions` component, in
the actions-document shape `ActionMap` reads. The script runner reads the
declaration when it changes and every action's value from the step's input
before any script runs; `Action.held`, `pressed`, `released`, `axis` and
`vector` answer by name, and `Action.bindings`, `Action.rebind` and
`Action.last_pressed` rebind one while the game runs, written back into the
component. The platformer's hero runs and jumps by actions
(`games/platformer/tests/a_run_reaches_the_flag.rs`), and `examples/input`
rebinds its boost (`game/tests/the_mixer_and_input_demos_work.rs`, and the browser
smoke). Actions are per scene, not per project or per player, and a rebinding
is not saved between sessions.

### Audio

Audio is a platform service rather than simulation state. Encoded WAV, Ogg, and
MP3 assets are identified and validated by the asset layer, then registered with
an `AudioBackend`. Native hosts use Rodio/CPAL, browser hosts use media elements
with an explicit user-interaction unlock, and headless tests use a silent backend
that records every request without needing a sound device. The shared boundary
supports one-shot and looping playback plus per-voice stop and global pause,
resume, and stop.

Scenes can author `sindri.audio.source` with a logical clip ID, autoplay, looping, and a
normalized volume. The generic component inspector can add and edit it, and the
project browser recognises WAV/Ogg/MP3 files as audio assets. Decay exposes typed
`Audio.play`, `loop`, `stop_all`, `pause_all`, and `resume_all` calls while only
emitting intent, so the language itself retains its no-I/O boundary. Gather
exercises the path end to end with background, pickup, and victory sounds. The
looping background music has been observed playing in a real browser through
`scripts/browser/smoke.mjs`, which fails if a clip a page asked for did not play.

Every voice plays through a named bus under `master`. `AudioMixer` in
`sindri-platform` sits in front of any backend: it starts a voice at its own
volume times its bus's times the master's, and `set_bus_volume` re-applies the
gain to every live voice the bus reaches, which each backend now supports
(`set_volume`, `is_active`). Decay's `Audio.play` routes to `effects`,
`Audio.loop` to `music`, `play_on`/`loop_on` to any bus, and `Audio.set_volume`
and `Audio.volume` move and read a bus; an authored source names its `bus` or
follows its looping. The game session routes scripts' requests and autoplay
through the mixer. Orbital's pause screen has Master, Music and Effects sliders,
saved between sessions (`games/orbital-baked/tests/the_mixer.rs`), and
`examples/audio` is the Sound Mixer feature example
(`game/tests/the_mixer_and_input_demos_work.rs`). There are no bus effects, ducking
or snapshots.

What audio does not do yet: nothing gathers the clips a scene names, the way
`referenced_textures` and `referenced_fonts` do, so dynamically named clips
must be listed in the project's `[assets].include`. The editor lists audio
files, edits `sindri.audio.source`, and can audition a selected clip through its
own audio backend, but it does not preview a scene source in place.

### Physics

`sindri-physics` is the masked physics boundary. Rapier2D and Rapier3D are
private implementation dependencies; public code speaks only in Sindri body,
collider, shape, layer, pose, error, and event types. Runtime entities remain the
identity exposed by the subsystem, so no Rapier handle can leak into a scene,
script, editor, or game contract.

The exercised runtime is currently 2D. `PhysicsWorld2d` owns fixed, dynamic,
position-kinematic, and velocity-kinematic bodies; box, circle, and capsule
colliders; independent membership/filter masks; sensors; friction and
restitution; checked velocity/impulse/kinematic operations; and normalized
collision/sensor start/stop events. Values are validated before they reach the
backend, and stepping takes an explicit engine fixed-step duration rather than a
render delta. Tests cover gravity, masks, sensor events, body operations, and
removal/reuse through generation-checked `EntityId`s, and the crate passes the
workspace's native and WASM checks.

Closest-hit 2D raycasts return Sindri `RayHit2d` snapshots (entity, world-space
point/normal, distance). Directions are normalized; maximum distance is
inclusive; invalid finite/range inputs fail. Membership masks, sensor opt-in and
whole-entity exclusion select pieces before choosing a hit. Inside hits have
zero distance/normal; exact ties prefer entity handle then piece order. Direct
scans work before the first step and after a body move. Decay returns `null` on
a miss and skips inactive/despawned entities immediately, while scene geometry
changes are synchronized at the next step. Engine and Decay regressions cover
geometry, compounds, filtering, validation, copies and removal/reuse. The
platformer displays ground clearance through jumps against painted tilemap
colliders, and `examples/physics` exposes live rays, hit dots, normals, falling
bodies, bounce and sensors with desktop/touch controls. Pages exports it under
`examples/physics/`; native project regressions and desktop/phone browser smoke
checks exercise its real Decay controls. 3D queries and an accelerated query index remain absent; implemented 2D
overlaps and shape casts are described below.

A parallel Sindri-owned 3D body/collider data model already fixes the public
shape of the later 3D slice, but no 3D runtime behavior is claimed yet.

Forces and rotation have runtime and typed Decay controls: additive world force
and torque last one fixed step, impulses act immediately and off-centre kicks
turn a body. Dynamic/velocity-kinematic rules, finite values and rotation locks
are validated; spawn-window requests replay after collider mass is known and
expire if never materialized. The platformer's wind crate exercises force,
torque, angular velocity and impulses through its real kick input, with scene
rotation writeback. Native, WASM and browser CI passed on the forces/rotation head; browser input and
visual inspector review remain in final integration. See `docs/physics.md`.

`PhysicsWorld2d::contacts` and typed `Physics.contacts(entity)` expose copied
solid solver snapshots: other entity, world point, push normal towards the
queried body, normal and signed friction impulses, and world force over the last
fixed dt. Solver manifolds supply solved impulses. Ordering is deterministic
by entity, point, normal and impulses. Sensors are excluded; sleeping support
remains with zero new impulse/force. Runtime teleports and removals invalidate
contacts, and Decay filters inactive/despawned others. Spawn-window queries are
empty; missing bodies/physics fail. The platformer's crate flashes on a hard
landing; its hero now grounds from controller results while keeping its ray
for clearance.
This is a general capability added for that genre showcase. Native regressions
cover momentum/force balance, compound pieces, snapshot copies, lifecycle,
ordering and sleeping support. Browser interaction/editor Play inspection remain
in final integration.

One-way support is authored with `sindri.physics2d.one_way`: a validated local
normal and contact cone, applied to solid collider and tilemap pieces. Pair
filtering also covers CCD ascent. `Physics.drop_through(entity, seconds)` uses
fixed simulation time, preserves ordinary floors/sensors and works in the spawn
window. Policy edits and undo keep live velocity/joints. Platformer's visible
planks prove ascent, landing, input-driven dropping to painted ground and landing
again; native regressions cover rotated normals, kinematic geometry and timer
cancellation. Queries remain geometric, so its Decay grounding uses solved support
contacts and descent rather than foot-sensor overlap. Chromium export/load
smoke passed; visual inspector and full browser interactions remain in final
physics integration.

Reusable physics coefficients are project `.profile` assets with type
`physics_material`, resolved at the scene boundary using shared validation.
`sindri.physics2d.material` applies them to all an entity's collider pieces,
including tilemap collision; explicit override flags win over profile values,
and entities without the component retain their literals. Editor creation,
profile selection, asynchronous delivery and hot reload use the existing profile
pipeline. Invalid reloads retain the previous valid edit; coefficient changes
preserve solver bodies, velocities and joints. Export discovers material
references and validates profiles, and the shared native/browser host resolves
them before stepping. Platformer reuses wood for its wind crate and planks,
explicitly disables plank bounce, and tests the effect of changed restitution.
Editor loader/reload regression coverage is present; visual inspector review and
browser gameplay interaction remain in final physics integration.

`sindri.physics2d.distance_joint` adds a scene-authored maximum-distance
constraint owned by a separate entity. Endpoint strings resolve to stable IDs
inside the containing scene; unchanged frames retain one constraint, edits/undo
preserve body motion, and endpoint rebuilds reconnect after body synchronization.
Inactive/missing endpoints suspend the constraint; removing its entity/component
releases it independently of legacy script-created joints. Platformer's lantern
sways under Decay wind within its authored tether and renders its cord in Decay.
`sindri.physics2d.hinge_joint` adds body-local anchors, bounded relative angles
and torque-capped velocity motors. Unchanged frames keep the constraint; setting
edits wake its endpoints without resetting body motion. Typed
`Physics.set_hinge_motor` updates the runtime component for the next synchronization,
including before endpoints are built. Platformer's Decay-driven windmill reverses
its axle motor; native tests exercise limits, caps, coasting, atomic validation,
undo and rebuilds.

`sindri.physics2d.slider_joint` adds aligned local axes, signed travel limits and
force-capped velocity drive; `sindri.physics2d.spring_joint` adds radial
force-based stiffness/damping.
Typed `Physics.set_slider_motor` and `Physics.set_spring` validate atomic runtime
component edits for next synchronization, preserving unknown fields and rebuild
behavior. Platformer's powered lantern trolley reverses along its rail while
Decay retunes the hanging light's rest length. Native tests measure rotated rail
motion/limits, force caps, coasting/braking, spring damping and weight support;
scene edits/undo, suspension, rebuilds and game removal are exercised. Editor save integration and native visual review are exercised.
Runtime-spawned nested mechanisms retain original root aliases independently of
serialized component payloads. The Decay spawn host passes the original prefab
library into expansion; all four joint kinds reconnect only their own nested
endpoints across repeated spawns, inactivity and removal. Platformer's reusable
windmill now nests its powered mechanism in `windmill-kit.prefab`. Canonical paths
shadow aliases; ambiguous aliases fail before spawning. Placed instances now retain
aliases through library-aware scene entry on native/browser hosts, scene namespaces,
editor reload/duplication and command undo. All four kinds exercise scene switching,
inactivity and removal; saving prefab references and reopening rebuilds aliases.
The platformer level windmill is placed from the same nested assembly. Flattened
plain documents discard metadata. Opt-in `World::to_scene_with_references` remaps
registered entity fields to assigned stable IDs, including nested lists, without
live edits. Unknown fields remain unchanged, empty references stay unbound and
malformed/unresolved/unstable targets fail serialization. All four joints reopen
with isolated endpoints; the platformer's Decay-spawned windmill reopens with motor
reversal and its fixed axle. Qualified scene IDs take precedence over relative
namespace lookup when an exact match exists.
All four authored constraints now accept an enabled flag, defaulting true for
old payloads. Typed `Physics.joint_enabled` reads the authored flag and
`Physics.set_joint_enabled` suspends/reconnects the constraint at next fixed
synchronization without rebuilding endpoint bodies. `Physics.set_distance`
tunes owned maximum-distance constraints, including before initial sync and
while suspended. Validation rejects missing/conflicting/invalid owners, wrong
kinds/types, missing physics and non-positive/non-finite lengths before mutation.
Unknown payload fields survive. Scene command undo/redo covers all four kinds;
platformer Decay reels its lantern tether with T and releases/reconnects it with L.
Script-triggered world snapshots remain absent.
Editor `SceneFile::save`/`save_as` and subtree prefab authoring automatically use
the active registry for reference-aware serialization. Stable identities remain
a prerequisite; invalid references fail before file writes/path adoption while
unknown fields and the live world remain intact. Existing placed instances still
save as prefab references. Native editor tests reopen two isolated instances of
every joint kind, reuse saved subtrees and reject invalid saves without changing
disk or the agreed document. The platformer's real Decay setup script spawns a
nested windmill that editor Save As reopens with its fixed axle and reversing
motor. Save remains unavailable during Play. Decay's number/flag store has no
world snapshot operation; that gap is separate from editor authoring saves.

Runtime-spawned prefab references now use a separate runtime identity, retaining
local sibling paths and the original top-level root ID without assigning saved
scene IDs. The platformer spawns/removes a reusable motor-driven windmill through
Decay. Repeated instances, inactivity, root removal, reparenting and command undo
are covered. Nested runtime aliases and placed roots survive library-based delivery;
registry-based save remapping is opt-in. Automatic save-path integration remains
open. Registered entity fields now have scoped native inspector choices, explicit
clearing and visible missing/inactive diagnostics through the runtime's shared
resolver. Real picker-click edits retarget/clear all four constraint kinds, and
command undo restores their constraints/unknown payloads. Visual inspector review
remains in final integration.

Typed joint endpoint retargeting uses `Physics.set_joint_endpoints` for all four
2D kinds. It accepts scoped entity handles or null to clear an endpoint, storing
stable scene IDs/canonical prefab paths. Invalid scope, stale/unstable handles and
identical endpoints fail before mutation; inactive targets suspend until active.
Settings, enabled state, unknown payloads and body motion are retained. Platformer
switches its lantern between two hooks with R, including while released. Generic
inverse references also drive the inspector choices through the same core rules.

Typed `Physics.remove_joint` removes exactly one valid authored 2D joint
component, releasing its owned constraint at the next fixed synchronization.
The owner, other components and bodies/motion survive, including before bodies
are built or while suspended; legacy distance connections remain separate.
All four kinds exercise removal and atomic rejection. Platformer Z cuts its
lantern cord, leaving the body in free fall and later tether controls inert until
repaired.
Typed `Physics.create_distance_joint` authors an enabled maximum-distance component
on an existing owner with no joint of any kind. Scoped stable references and length
validate before mutation; null/inactive endpoints suspend. Next synchronization
creates the owned constraint, retaining other components, bodies and legacy
connections. Tests cover pre-sync creation, repeated removal/recreation, invalid
owners/references/lengths, inactive targets and isolated runtime prefabs.
Platformer C repairs its cut cord at the selected hook and length.
Typed `Physics.create_hinge_joint` uses the same empty-owner/scoped-endpoint
contract with finite body-local `Vec2` anchors. Limits and motor start disabled;
existing motor controls configure the new component before synchronization.
Pre-sync creation, repeated recreation, local anchors, motor behavior, owner/
anchor rejection, inactive/unbound targets and runtime prefab isolation are tested.
Platformer H rebuilds its placed windmill hinge; the rotor remains at its axle
and reverses through Decay while the separate spawned windmill stays intact.
Typed `Physics.create_spring_joint` supplies finite body-local anchors, positive
rest length and non-negative stiffness/damping under the same empty-owner/scope
contract. Existing tuning works before synchronization. Native tests exercise
local-anchor force response, repeated recreation, retained body motion/legacy
ownership, invalid arguments, inactive/unbound targets and prefab isolation.
Platformer B rebuilds the light's spring without resetting its tuning phase or
replacing the trolley's independently owned slider.
Hinges also support a damped force-based position motor. Old payloads default
to velocity mode; typed `Physics.set_hinge_position_motor` selects a relative
angle within `[-pi, pi]`, finite non-negative stiffness/damping and a torque cap.
The velocity setter switches back explicitly. Native tests exercise holding,
retargeting, caps, enabled limits, atomic rejection, command undo/redo, scene
round trips, suspension and collider rebuilds. Platformer P holds both placed
and spawned windmills, resumes their reversal, and retains the selected drive
when H recreates the placed hinge. This capability was added for that showcase.
Script-triggered world snapshots remain absent.

Sliders also support damped force-based position drive through the shared motor
mode; omitted modes keep velocity behavior. Typed `Physics.set_slider_position_motor`
sets finite signed anchor separation along the first local axis, non-negative
stiffness/damping and a force cap. The velocity setter switches back; travel
limits still apply. Native tests prove rotated rails/offset anchors, signed
retargeting, caps, limits, invalid atomic calls, coasting, suspension/rebuild,
checked command undo/redo and save/reopen. Platformer O parks/releases its trolley
and J retains the selected drive and independent spring. Added for that showcase;
script-triggered world snapshots remain absent.

Typed `Physics.create_slider_joint` adds finite local anchors, unit local axes
and optional finite ordered travel bounds under the same owner/scope contract.
The new motor starts disabled; existing motor controls work before body sync.
Tests exercise pre-sync/repeated creation, local-axis bounded travel/reversal,
body-motion/legacy preservation, invalid geometry, inactive/unbound targets and
runtime prefab isolation for all four constructors. Platformer J recreates the
rail slider with its current drive direction, retaining the spring suspension.

`sindri.physics2d.rigid_body` and `sindri.physics2d.collider` are registered
scene components with defaults the engine accepts, so a scene authors bodies and
colliders and the editor's generic component inspector adds and edits them.

`sindri.physics2d.tilemap_collider` makes the orthogonal tilemap on the same
entity solid: every painted tile collides unless its palette sprite is listed as
`passable`. Solid cells are merged into as few rectangles as cover them, so a
floor is one box and a character cannot catch on the seams between tiles; the
rectangles follow the entity's scale, and join any collider pieces the entity
also carries. An isometric map is refused by name. `sindri.physics2d.world`
carries the scene's gravity (straight down at 9.81 by default); a scene without
one keeps its host's, which for the editor and Orbital is none.

`ScenePhysics2d` is what joins the two halves. It builds the simulation from
those components, keeps it in step as entities are spawned, switched off, and
despawned — a body outliving its entity would collide on behalf of nothing — and
writes what physics decided back into the transforms the renderer reads, leaving
the authored Z and the 3D scale alone. A transform moved by something other
than physics since then, a script respawning the player, moves the body with
it before the next step rather than being put back. Friction combines as the
smaller of two colliders' values, so a frictionless hero slides down walls. A body whose authored values have not
changed is left as it is rather than rebuilt, because rebuilding one discards the
velocity and contacts the simulation owns. Editor Play and the game session both
step it once per fixed update, before scripts run, so a script observes the
events of the step that just happened.

The Scene view draws every collider's outline, from the very pieces the physics
world is given (a tilemap's merged rectangles included), brighter for the
selected entity and fainter for static geometry. The selected collider's pieces
carry handles: a box's four edges, a circle's radius, a capsule's radius and
height. Dragging an edge keeps the opposite edge where it was, along the piece's
own rotated axis, and a whole drag is one undo step. The generic inspector
also exposes the body and collider payloads — a compound's pieces are added, removed,
reordered, and edited down to each piece's shape, with a two-piece compound used by the platformer — and editor Play steps them through the same
fixed-update path as a build. `games/orbital-baked` is the end-to-end proof:
player, enemies, projectiles, pickups, and effects use distinct masks and
collision or sensor events continuously.

Beside the raycast, `PhysicsWorld2d::overlap` lists every entity with a piece
overlapping a circle, box or capsule placed in the world, and `shape_cast`
sweeps one along a line to the first piece it touches, both from current body
poses with the raycast's filter (`crates/sindri-physics/tests/overlap_and_shape_cast.rs`).
Decay's `Physics.overlap_circle`, `overlap_box`, `cast_circle` and `cast_box`
reach them. Orbital's hostile mine damages what its blast circle overlaps
(`combat_lab.rs` proves a target caught by its edge), and Physics Playground
switches its line between a ray, a swept circle and an area.

A scene's `sindri.physics2d.world` names its collision layers bit by bit.
`Physics.layer` and `Physics.mask` turn names into masks and refuse an unknown
name (`crates/sindri-decay/tests/a_script_names_its_layers.rs`), and the
editor's inspector draws every collision mask as a menu of the named layers,
keeping unnamed bits. The platformer names ground, hero and pickups; its hero's
ground probe asks for `ground`.

### Editor Play

Play runs **the same fixed-step loop a shipped game runs**. The editor owns a
`FixedStepClock`, advances it with the real frame delta, and runs gameplay a
whole number of times per frame — effects, physics, screen UI, scripts and
animations, in that order, at the fixed rate. Before this it stepped once per
*rendered* frame, so a scene was simulated as fast as the machine happened to
draw and a play-test was evidence about the editor rather than about the game.

**An edge belongs to exactly one fixed step.** A key going down is one event and
gameplay runs in the fixed step, so the edge is spent once a step has seen it —
a 30 Hz display driving a 60 Hz simulation earns two steps a frame and would
otherwise fire a button twice. It also survives a frame that earned no step at
all, which at 144 Hz is most of them, so a click is never dropped before
gameplay sees it. Accumulated pointer motion follows the same rule: two frames
of dragging between steps sum rather than losing the first. This was wrong in
the shipped host too, and is now covered by tests in
`crates/sindri-platform/tests/game_loop.rs`.

**A held scene can be stepped once**, which is what a debugger's step button is
for: the bug that happens in one frame and is gone before anyone can look at it.
It runs the same body a played frame runs, so a scene stepped sixty times is a
scene that played for a second.

### Effects

`Effects2d` holds short-lived visual flecks as plain values in one array. A
fleck has no identity a script can hold, no components, no place in the
hierarchy, and nothing can collide with it — everything an entity is *for* is
given up, and `docs/effect-scaling.md` is why: 8,000 flecks-as-entities cost
5.25 ms a frame, a third of a 60 Hz budget, against 0.018 ms pooled. Over half
the entity cost was `serde_json` re-reading each one's payload, every entity,
every frame.

What a burst looks like is authored on the entity as `sindri.effect.burst` —
count, speed, spread, lifetime, size, tint, fade, drag — because those are a
designer's numbers and a call naming all of them would be unreadable. Flecks
draw their directions from a stream of their own, never the run's, so turning an
explosion up cannot change which enemies spawn. The pool is bounded, the oldest
fleck makes way for the newest, and overflow is counted rather than hidden.

They batch with ordinary sprites by layer and texture, so a burst is one draw
call rather than a second rendering path.

### Game saves

`sindri_core::SaveStore` is a flat, versioned key/value document. Flat because
Decay holds numbers, truths and text and nothing else: a structure a script could
not build is a structure nothing could write, and the file stays something a
person can read and repair. Ordered keys, so the same saved state is the same
bytes and a save can be diffed.

Three absences are distinguished, because they call for different things: a
first run starts cheerfully, a **damaged** save is worth telling someone about
before their progress is written over, and one written by a **newer** build is
reported without being read — a reader that guessed at a format it does not know
would corrupt it the moment it wrote back.

`sindri_platform::SaveBackend` is where the bytes go. `MemorySaves` is the
default so a headless run and a test have somewhere to write without choosing a
path; `DamagedSaves` exists so the unreadable path can be proven without
corrupting a real file; `FileSaves` writes beside the target and renames over it,
because a save half written is a save destroyed at the exact moment someone's
machine lost power mid-run; `BrowserSaves` uses `localStorage`, chosen over every
larger browser store because the alternatives are asynchronous and a game should
not have to ask whether its progress has landed yet.

Nothing in Decay writes to storage. The store is in memory, marked dirty on
change, and the host writes it out on a cadence and before it stops — how often
someone's disk is touched is a decision about their machine. Writing the same
value again is not a change, so a game storing its volume every frame does not
keep a disk busy.

Editor Play keeps a save in memory for the editor's lifetime and never writes it
to disk: `Save.*` calls work and round-trip inside a session so persistence can
be play-tested, but putting a file into someone's project because they pressed
Play would be a side effect they did not ask for. Editor preferences are a
separate thing and stay separate.

### Randomness

`sindri_core::Rng` is a **PCG-XSH-RR 64/32** generator written out rather than
depended on. Every general-purpose crate reaches the operating system for a
seed, which on `wasm32-unknown-unknown` means `getrandom` and a target that
refuses to compile without an opt-in — and more to the point, entropy is the
opposite of what this is for. A run that cannot be replayed from its seed is not
seeded at all.

Everything in it is integer arithmetic and the one division is by a power of
two, which is what makes a seed mean the same thing on every host. Fractions are
built from the top 24 bits, so `[0, 1)` is never `1.0`; bounded integers reject
the draws that would make the low values slightly more likely, because modulo
bias on a drop table over a long run is exactly the kind of wrongness that gets
blamed on the game design. Seeding takes two steps around the state, so
neighbouring seeds do not produce neighbouring first outputs.

The engine never asks the platform for entropy and does not pretend to: a host
that seeds nothing gets a fixed stream. The editor puts the stream back to its
seed on every fresh Play, so pressing Play twice gives the same run twice and a
bug found once can be found again; resuming from a pause deliberately does not,
since that would replay numbers the scene has already acted on. A game that
wants variety calls `Random.seed` with something it knows.

One stream is shared by every script, so the seed determines a run but a number
drawn early shifts every number after it. That is stated rather than hidden, and
it is why a run's seed is worth storing while a frame's numbers are not. It is
not a source of secrets: a handful of outputs reveals the state.

### Weave presentation styling

Weave resolves responsive presentation into a disposable clone of an authored
world. Its selectors are CSS's: element names (`text`, `button`, with the long
`sindri.ui.text` still accepted), IDs, classes, `*`, compounds such as
`button.primary:hover`, descendant and child combinators, and lists, with CSS
specificity and source order deciding conflicts. The cascade is CSS's too:
text properties inherit from the containing entity, `inherit`, `initial` and
`unset` do what they do in CSS, and custom properties (`--accent`) inherit and
substitute through `var()` with fallbacks, so a theme's tokens live in one
place. Media queries test orientation and minimum or maximum width and height,
combined with `and`, commas and nesting. `:disabled` and `:checked` follow the
entity's own component data everywhere. Every running host (the editor's
Play, native games, browser exports) styles through a
`sindri_weave::Presenter`: a game's world is settled with the stylesheet's
rules at start and on resize, and each frame only what the pointer's states
and running transitions change is laid over it, in place, for the draw, and
taken off again. So `:hover` (which, as in CSS, also matches the hovered
element's containers) and `:active` follow the pointer, a script's own
writes are not styled back, and hit-testing uses what was drawn.
CSS `transition` eases colours, lengths and numbers between states with named
or `cubic-bezier` easings, delays and `all`. The language workspace now has its own CI
workflow. `docs/ui-direction.md` is the plan beyond this. Percentage sizing,
min/max constraints, padding, gaps, wrapping, alignment, and text wrapping cover
the responsive composition used by the shipped examples.

Weave is kept in its own workspace, while `sindri-weave` is the one-way bridge
that knows the engine. Stylesheets compose through `@use`; the exporter follows
that graph, and the browser host applies independent roots in deterministic asset
order. `games/weave-poc` is the focused responsive showcase, while Orbital Last
Stand uses five composed Weave files — a shared theme, then its HUD, overlays
and screens — for its production-oriented screen UI.
`docs/weave.md` and `docs/weave-reference.md` record the supported surface.

The editor treats manifest-listed `.weave` roots as project assets, previews
their source, authors reusable classes in a dedicated inspector section, and
applies the composed presentation to both Scene and Game views. It watches the
manifest and complete `@use` graph for saved changes, keeps the last good
presentation after a broken save, and reports composition failures with their
source path, line, and column. The inspector's Styles section is Weave's
devtools: the box model as drawn, every matched rule with its `file:line`
and the overridden declarations struck through, and the computed values, at
the Game view's size. Clicking a value there edits it in place and writes
it into the stylesheet at the rule's line, which then reloads; the Game
view's Pick toggle selects an element by clicking it in the running game.

Weave's `margin` and `padding` (one to four values, per-side longhands, CSS
percentages) write that box, and `box-shadow` writes the shape's shadow.

Layouts are CSS flexbox, in the engine: `sindri.ui.layout` wraps and fits
its content, and each child's `sindri.ui.box` grows, shrinks, takes a
basis, orders and aligns itself within min/max limits. Layout decides sizes
as well as places, and drawing, hit-testing and the editor's picking use
them. Weave's `flex`, its longhands, `flex-wrap`, `order`, `align-self`,
`width: auto` and the rest write the same data.

Text sizes what holds it: a text element whose `sindri.ui.box` fits its
content (Weave's `width: auto`) is measured by its font each draw, so a label
sizes a button, and a label is the least a shrinking flex item keeps.
Hosts measure through `sindri_scene::measure_ui_text` and pass the sizes to
extraction and hit-testing.

`sindri.ui.grid` lays children out in CSS grid's columns and rows: fixed,
`auto` and `fr` tracks, explicit or automatic placement with spans, implicit
tracks, gaps and cell alignment. Weave's `display: grid` and the
`grid-template-*`, `grid-column`/`grid-row`, gap and alignment properties
write it.

Selectors include the structural pseudo-classes (`:first-child`,
`:nth-child(2n+1)`, `:root` and the rest), `:not()`, `:is()`, `:where()`,
and the `+` and `~` sibling combinators, matched in scene order; lengths
take `calc()`.

This is not a general CSS implementation yet. `@keyframes`, borders per side,
grid's `minmax()` and named areas, and accessibility mapping remain absent.

### Screen UI

`sindri.ui.image` and `sindri.ui.text` draw against the viewport. Text is a
**template**: the words carry `{}` slots and a script supplies the numbers, so a
HUD's words stay in the scene, and a script that does need to build text joins
it with `+`. An image carries a **fill** fraction and the edge it empties from, which is
what makes a bar a bar rather than a picture of one.

`sindri.ui.button` makes an element pressable, its rect being the entity's own
transform. `ScreenUi` is the runtime beside the world: it lays every element out,
hit-tests the pointer, and answers hover, click and hold. A click is a press and
a release on the same element, so sliding off before letting go changes a
person's mind. Overlapping elements are resolved by layer, so a modal is a modal
because it is on top. A disabled entity — or one under a disabled parent — is not
hit-tested, which is also what a *screen* is: a menu is an entity with children,
and showing it is switching it on. No screen stack was added, because the engine
already had the mechanism.

Nothing is silently withheld from gameplay while a menu is up: which scripts are
gameplay is not something a host can know. `Pointer.over_ui` is the one line a
gameplay script writes instead.

`sindri.ui.slider` is the first element a person drags rather than presses. It
carries `min`, `max`, `step`, a `value`, an orientation and a `disabled` flag,
and its point is that the two ways of *setting* it agree: a pointer drag and
`Ui.set_slider_value` from a script both pass through one `coerce_value`, which
clamps to the range and quantizes to the step. A value therefore cannot enter
the component off-step from either direction. An authored value is not coerced
on load — a scene may hold anything — so the read side stays defensive instead:
a degenerate range, or a value that is not finite, answers `min` rather than
letting a NaN spread into the layout.
The drag tracks the pointer along the element's own rect, so an element moved or
resized by Weave keeps its slider correct without the slider being told. Decay
reads `Ui.slider_value`, writes `Ui.set_slider_value`, and asks
`Ui.slider_changed` whether the person moved it during the step. The editor
offers it as **UI Slider** in Add Component with `orientation` drawn as a
declared choice; `value` is still a plain number box, because no field yet
carries a bounded-range meaning tying it to the `min` and `max` beside it.

The widgets beside it follow the same rule, that a person's change and a
script's write pass through one check. `sindri.ui.toggle` is a boolean that a
click, tap, Space or Enter flips; whether it looks like a switch or a checkbox
is styling, through Weave's `:checked`. `sindri.ui.text_input` is a single-line
field with a placeholder and a limit counted in characters: it takes committed
Unicode text, Backspace, Enter to submit and Escape to let go, and while it has
the keyboard its keys are held back from `Input`, so typing does not steer.
`sindri.ui.scroll` scrolls
its children vertically by wheel, drag or finger, clamped to how tall they are
as laid out, and clips their drawing and hit-testing to itself; a drag that
starts on a row never becomes a click on it. Tab and Shift+Tab walk every
focusable element in the order the scene is written. Decay reads and writes
them with `Ui.is_checked`, `Ui.input_text`, `Ui.scroll_offset` and their
setters, and `Ui.changed`, `Ui.submitted` and `Ui.is_focused` answer for the
step. Toggles that share a `group` are a radio group, and `sindri.ui.dropdown`
opens a popup of authored `sindri.ui.option` rows that the engine shows only
while it is open (Weave's `:open`), read with `Ui.selected`. A field edits at a
caret with a selection and the system clipboard, and a script draws the caret
from `Ui.caret` and the `Ui.selection_*` pair. In a browser a field being
edited gives the page's focus to a hidden textarea, which is how IME
composition, a paste and a phone's keyboard reach it. The arrows and any pad's
d-pad move focus toward the nearest control, preferring those in line, and a
row reached inside a scroll region is scrolled into view. `examples/ui` is the
feature example; Orbital Last Stand proves them in a game with a saved
compact-HUD switch, a pilot callsign carried onto the HUD and results, a
scrolling dropdown of its twelve bosses, and a scrolling field manual on its
pause screen.

No game in this repository uses the slider. `games/weave-poc` demonstrates it, and
Mujaffa Remaster — an external project — is what asked for it, so by the
capability rule the slider is implemented and unproven: `docs/parity.md` records
its proof column as ❌ until Gather or Orbital Last Stand adopts it.

`sindri.ui.layout` places a parent's active children in a row or column. Three
buttons could be authored as three offsets; what cannot be authored is a menu
closing up around an entry that was switched off. `sindri.ui.box` gives any
element CSS's padding and margin per side: a layout's children flow inside its
padding and keep their own margins free, without collapsing, as in flexbox.
A `sindri.ui.shape` can cast a soft `shadow` (colour, offset, blur, spread).

The overlay is authored in normalized units — two tall, centred, running out to
the aspect ratio — so one authored scene is responsive across a portrait phone
and a wide desktop window without a breakpoint. A **safe area** takes a notch or
a home indicator off the edges, moving anchored elements in while leaving centred
ones where they are; the editor reports none, because a desktop window has no
notch. Decay reads that same screen shape through `Viewport.aspect`, which lets
a game combine the viewport with its authored camera framing for responsive
world-space rules without exposing pixel dimensions or asking the host to pick
a gameplay camera.

What is not built: no toggle, dropdown or text input — the slider arrived
alone, so a settings screen still has no checkbox and no typed field — no scroll
region, and no accessibility surface. A button
carries a `label`, authored beside the thing it names, but the static web export
still renders through canvas/WebGPU and has no semantic DOM bridge to expose it.

### Grid geometry

`sindri-grid` provides renderer-independent signed cell coordinates, continuous
grid and plane points, finite rectangular bounds, deterministic cardinal and
eight-way neighbour queries, and validated orthogonal/isometric projection.
Cell centres and arbitrary fractional points round-trip through both projections
across negative and positive space. Upward/downward plane Y adapts world and
screen coordinate conventions without introducing a renderer dependency.

`sindri.tilemap` exposes the exact `GridSpace` and `GridBounds` its rendering and
editor picking use. A map's complete transform is composed around that local
grid, so moving, rotating, or scaling a map keeps drawn and picked cells in
agreement. `sindri.grid.navigation` adds authored internal wall edges without
duplicating those bounds or projection settings, while `sindri.grid.occupant`
names its grid by stable scene ID and carries a relative multi-cell footprint.
`WorldGridNavigation` resolves those references, derives anchors from current
world transforms, rejects conflicting or invalid placements as one incomplete
snapshot, and exposes validated placement and wall-aware path queries.

Decay can read an entity's continuous logical X/Y relative to a tilemap and
place it back through the same projection and map transform. It can also read
the viewport aspect ratio; general camera conversion and direct typed camera
access remain missing.

`docs/parity.md` tracks those counterparts explicitly rather
than allowing the runtime type to make the broader feature look complete.

### Voxel world foundation

`sindri-voxel` owns signed voxel and 16³ section coordinates, palette-backed
section storage, deterministic random-access generation, bounded 3D residency,
sparse edits, dirty tracking, and deduplicated generation/meshing work queues.
Mesh jobs carry a monotonic revision and meshing profile, and boundary changes
give the neighbour a new mesh revision even when its own stored voxels did not
change.
The block mesher samples a one-voxel halo through `VoxelSource`, so adjacent
solid sections omit their shared faces without requiring both to be resident.

Compiled block output is CPU-side indexed geometry in section-local coordinates.
It carries unit-square UV corners plus semantic voxel and face identity, splits
opaque, cutout, and transparent passes, and reports world-space section bounds.
Games provide `VoxelMaterialSource` policy for render class and whether a voxel
occludes every neighbour, only a matching voxel, or none. Atlas lookup, texture
bindings, and GPU upload deliberately remain outside this crate. A generic
`SectionMeshCache<T>` owns the renderer-independent lifetime contract: block,
smooth, hybrid, and custom representations are separate; current geometry
stays available while a replacement is pending; superseded worker results are
rejected; and one removal releases every profile of a leaving section.
`sindri-render` now has the matching GPU-side contract for textured meshes:
opaque cache identities and monotonic revisions reuse unchanged vertex/index
buffers, retain the last buffers while replacement geometry is unavailable,
accept 32-bit indices, release departed entries explicitly, and expose
cumulative cache counters. `sindri-scene` is the texture-aware seam between the
two: it maps semantic voxel/face identities to atlas regions, groups one section
into deterministic texture batches, keeps stable GPU identities across newer
section revisions, ignores superseded results, and releases every batch when a
section leaves residency. Scenes can author this as `sindri.voxel_world`, with
either a deterministic layered generator or a natural terrain generator
(continents, ranges, rivers, biomes, caves, overhangs, water, snow and trees
from one seed; see [`voxel-terrain.md`](voxel-terrain.md)), per-face voxel
material textures, bounded
horizontal/vertical residency, and a section focus. The same scene path renders
in games and in the editor Scene view. A world may instead name a block set in
`blocks` (the engine ships `builtin:blocks`, and new worlds use it), and then
its generator names blocks by name, drawn with each block's per-face art; the
generator's fields are `FieldMeaning::Block`, which the editor offers as a
menu of the set's blocks drawn as cubes. A block's faces may animate through
frames and a block may glow (drawn per batch through a `MeshSurface` UV offset
and glow, with no remeshing); blocks that do not hide their neighbours are cut
out, and `liquid` ones hide only their own inner faces. Without a block set, material IDs are
declared keys
(`FieldMeaning::Key`) and the generator's surface, subsurface, and deep layers
reference them (`FieldMeaning::Block` falling back to `KeyOf`, including the natural terrain's biome
and optional palette voxels, which also offer *None*), so the editor numbers an added material
past every ID in use, refuses to remove one a layer still names, and offers the
layers only the IDs that exist. The editor's extractor tolerates an invalid
voxel world or environment (`SceneExtractor::tolerate_invalid_components`):
the last valid terrain and lighting keep drawing, and the failure is reported
against its entity, naming the environment field and the values it accepts.
Games and capture tools stay strict and fail the frame. A resident voxel
world keeps its compiled sections until its definition or the textures its own
faces resolve to change; binding an unrelated texture no longer rebuilds it. Voxel Lab now uses that component rather
than a tile volume, while its browser route exposes the persistent-cache and
settled-remesh counters. Cached section bounds are tested conservatively against
the current camera after the voxel world's entity transform, so offscreen
resident sections retain their CPU cache without submitting draws or consuming
their pending GPU upload. This first bridge supports opaque block batches;
cutout/transparent pipelines and editor voxel picking/painting remain.

A world with `"view": "map"` is drawn instead as a flat map from straight
above (`extract/voxel_map.rs`): each column one unit square wearing the top
face of its highest supporting block, read through the same `VoxelGround`
scripts use, so edits show and agree with gameplay. Squares are lit by height
(a slope against a north-western sun, paler high ground, water darker by
depth) and go into the ordinary sprite batches, so a map sorts and shares
draws with sprites and tilemaps. Only the columns under the world camera's
four corners are worked out, sixteen square at a time, capped at 48 chunks a
side, and remembered until the entity, its textures or its block set change;
switching a world to a map releases its meshed sections. `Grid.surface` and
`Grid.height` read a column's top from Decay (the latter `f32?`, `null` where
nothing stands). Extraction tests cover the squares, row direction, relief,
edits and the absence of meshes; Low Tide's Basin is the proof in a game.
Low Tide also exercises existing sparse `Grid.set_block` edits for biome
harvesting, typed shared state and deck tiles for its six-resource hold
(four slots on the small starter crawler, growing to sixteen through three
resource-funded stern extensions),
and prefab spawning for deterministic wrecks across the Basin. Keyboard and
phone-touch runtime tests harvest, carry, stow and jettison actual blocks;
the workbench also consumes physical cargo for deck expansion, a bunk and an
engine upgrade, preserving the crew and deck origin while driving. Structure
adds permanent weight; duplicate and unaffordable builds spend nothing.
Its viewport-sized workbench exposes three directly selectable blueprints,
current cargo versus cost, installed/chassis-limit states and explicit actions.
Keyboard and phone menu tests exercise selection, rejection and closing;
phone/desktop captures exercise the compact HUD and responsive menu.
Its wrecks also spawn six resource-specific cargo prefabs as hull children;
Decay selects the nearby physical crate and retains a per-slot mask across
wreck streaming. Carried cargo can be set down as a world prefab and picked
up again before stowing. Keyboard/phone regression tests exercise choice,
recoverable drop and inventory accounting; flood tests exercise one-time loss
of loose ground cargo. This uses existing prefab, parenting, animation and
script APIs; the history remains session-only.
Its instrument HUD uses authored atlas icons, real hold segments and
speed/phase-time meters. A deck-parented outline identifies the crate named
in a contextual action panel; discard confirmation is tied to that cell and
cancels on movement, Escape or timeout. Keyboard and phone runtime tests
exercise these decisions. Frame lamps use painted sprite glows, not a new
engine lighting capability.
Low Tide also exercises a side-view high-tide dive: a second authored camera
frames a wreck tilemap, Decay swims against hull cells, manages air and a heavy
cache, then deposits ore and scrap in the original hold only after returning.
The map tide continues; recall, empty air and ebb recover crew without loot.
Per-site cache history survives wreck streaming during the current session.
The interior is an authored activity, not a generated terrain cross-section;
there is no Low Tide session persistence on desktop or web yet.
These are Decay gameplay rules, without a new engine host surface. Not
drawn: decoration that holds nothing up, and a face's animation or glow.

A block from a block set is meshed as the box its tile describes -- a slab's
top half way up, a post a thin column -- and a neighbour's face is dropped
only where a block covers all of it. Each cell wears one of its block's
variants, chosen by its position and the world's `variant_seed` exactly as a
tile volume chooses them, and a block with something standing on it wears its
`covered` look.

A world may keep its resident window under the world camera
(`follow_camera`): the window centres on where the middle of the picture meets
the height of the `focus` section, and moving it loads the sections it reaches
and releases those it leaves. A world's `edits` list what has been changed
from what was generated, cell by cell; only changes are stored, an edit back to
the generated block is dropped, and an edit remeshes only the sections it
touches. `VoxelGround` answers gameplay's questions from the same generator and
edits -- the block in a cell, a column's surface, the first block a ray meets
-- remembering generated sections between questions so edits never regenerate
terrain. A voxel world on a grid of boxes (`sindri.tile_grid` with `space:
"solid"`) is that grid's ground: its voxels are the grid's cells, centred on
them as a volume's blocks are, and the grid's calls answer from it. Scripts
read, build and remove blocks with `Grid.block`, `Grid.set_block` and
`Grid.tagged`; `sindri.grid.placement` stands things on its surface;
`Grid.can_reach` and `Grid.step_toward` path across it within the grid's
`max_step`, reading the ground around the walker and its goal; and
`voxel::aim_at_with` says which block and face the pointer is on.
`crates/sindri-decay/tests/a_script_walks_and_builds_on_voxels.rs` builds,
digs and walks round a wall on a generated world, and Causeway is played on
it.

### Hosts and platforms

`Game` receives `start`, `fixed_update`, `update`, and `stop`, each with a
`FrameContext` carrying the world, the input state, and frame time, and each
able to fail in a way that reaches the host rather than being swallowed. The
same loop runs windowed on a desktop, in a browser through the same `winit`
event loop and canvas attachment, and headless in a test.

`DesktopApp` supplies the window, event loop, and input translation once, so an
application does not rewrite them. Target-specific code is confined to the
crates that must have it. Application failures retain their complete
`std::error::Error` source chain when logged natively or announced through the
browser's `sindri:failed` event, rather than exposing only the outer host error.

### GPU

Adapter, device, and queue negotiation is shared, with conservative
cross-target limits, resource labels, and errors that name what failed.
Swapchain acquisition has seven outcomes and one policy applied to all of them,
so hosts do not re-derive when to skip, reconfigure, or recreate — see
`docs/rendering-surface.md`. Surfaces must negotiate an sRGB format or fail
loudly. Headless adapter initialisation is proven in CI on software Vulkan.

### Rendering

A frame goes through extraction, preparation, and rendering. Preparation
validates and orders passes deterministically by stage, then layer, then
insertion order; stage order is `Opaque3d`, `Transparent2d`, `Overlay`.

What can actually be drawn: a triangle, a coloured cube, a textured cube with
depth testing, textured sprites with tint, anchor, and layer, and procedural 2D
shapes. A sprite may also carry an advanced colour transform: independent RGBA multipliers and offsets evaluated as `sample * tint * multiply + offset`, travelling per instance so two sprites recoloured differently still share a draw call. The identity is `multiply = [1, 1, 1, 1]` and `offset = [0, 0, 0, 0]`, so a sprite authored before the transform existed draws exactly as it did. A transform that is not finite is refused at extraction rather than drawn — the reachable case is narrowing, since `1e39` is an ordinary `f64` and an infinity once it is an `f32` — while a finite value outside zero to one is left alone for the render target to clip. The editor draws the two quadruples as labelled RGBA rows inside an advanced section, collapsed until the scene has authored one and resettable to the identity. No companion game in this repository exercises the capability yet; Mujaffa Remaster is the external proof for this round. Rectangles, ellipses, grids, and regular polygons remain evaluated in the
shape shader. A polygon may also carry up to eight explicit 2D vertices; those
points travel in the same instanced payload and the shader measures against the
actual authored edges, so an irregular hull remains crisp without becoming a
texture. When no points are authored, regular polygons keep their existing path.
The eight-point cap keeps the per-instance vertex attributes inside Sindri's
conservative WebGPU limits. Orbital Last Stand exercises the capability with the
Strider's exact six-point hull.

**A sliced image says how it is cut, once.** A sheet document beside a texture —
`textures/tiles.png` is sliced by `textures/tiles.sheet`, at a derived ID
nothing has to declare — names the parts of it, either as a grid or as explicit
rects. A scene then writes `textures/tiles.png#floor`, and the `#` that had
always been rejected inside an asset ID is what carries the name, because it was
reserved so a fragment could not leak into a path. Three components used to carry
their own copy of a sheet's layout and could disagree; none carries one now.
Names rather than indices, so a re-slice that moves a cell does not silently
change what a scene draws.

Rects are checked once, where a sheet binds, and ride on the instance so every
part of one sheet stays in one draw call — a GPU test reads the pixels back to
prove the shader honours it. A name no loaded sheet places draws the whole image
and is reported by `unresolved_sprites`, the same rule an unbound texture
follows.

Animation reads those names: `sindri.animation.sprite` holds clips of sprite
names and which one plays, `SpriteAnimations` holds where each sprite has got to,
and the cursor holds a name rather than a rect so playback does not depend on
where anything sits in an image. Playback is runtime state, so watching an
animation run does not rewrite the scene it came from.
Low Tide also exercises speed-controlled clips on children of a moving deck:
its two tread belts roll with actual movement, pivot in opposite directions,
hold their frame while flooded and resume after the ebb. The project regression
observes playback through rest, driving, pivoting and flood recovery.

**A script chooses the clip.** `Animation.play`, `stop` and `restart` name one
of the clips the scene authored; `is_finished`, `frame` and `clip` read where
playback has got to; `set_speed` scales it. The split above is what the surface
is shaped around: choosing is authored state and writes the component, so it
lives in the world and undoes and saves like anything else, while the cursor is
derived and is only read — which is what keeps a script driving an animation
from dirtying the scene. `play` is idempotent so that the natural way to write
it (`if moving { play("walk") }`, named on every frame) runs the clip instead of
pinning it to frame one; `restart` is the separate thing to want. Gather's
player is the proof: its walk cycle had played since the sheet was sliced,
including while standing still, and now runs only while it is walking.

**A sheet is sliced in the editor, on the image.** Selecting a texture opens it
in the Sprite sheet tab, a top-row tab beside the Scene and Game views, where
the image fills the canvas pixel-sharp on a checkerboard, zooms about the
pointer with the wheel (5% to 6400%), pans with the middle or right button, and
fits back with Fit. Each cell is outlined on the picture, named cells are
labelled once there is room, and with nothing open the tab lists every image in
the project. The slice itself is the inspector's while an image is open:
columns, rows, margin and spacing are drags, so a packed sheet with gutters can be cut and not only one
that divides edge to edge. A cell is named by clicking it, and the panel lists
the cells that have names rather than a field per cell, so a 16x16 atlas is as
workable as a four-frame strip. Saving writes the sidecar, and the browser then
lists the sprites underneath the image, collapsed until asked for. A sprite's
animation inspector creates, renames, and removes clips from those named cells,
orders frames, edits timing and looping, chooses the runtime clip, and previews
it without changing scene state.

A tilemap is a grid of tiles drawn from one entity: `sindri.tilemap` carries the
map's grid, a palette of sprite names, and a flat array of cells indexing that
palette, with `null` where the map is empty. Its cells become instances in the
same batches loose sprites use, so a prop sorts among the floor rather than
behind it. Columns and rows lay out orthogonally or isometrically, and placing a
tile and finding the tile under a point are inverses on every cell of both. What
it buys is authoring rather than speed — the same floor already batched into one
draw — and the companion game measures it: 68 entities to 20, 45KB of scene to
12KB.

**World-space tilemaps can be authored in the editor.** Selecting one shows the
slices from its texture's sheet as a visual palette, lets its grid be resized
while preserving the overlap, and turns primary drag in the Scene view into an
undoable paint or erase stroke. The hovered cell is found by inverting the exact
camera matrix used to render that viewport, so an orthographic or isometric map
does not need separate picking approximations. The ordinary component fields
still edit projection, tile size, tint, render layer, and space. Screen-space
tilemaps render and remain editable as data, but direct painting is deliberately
limited to world space until the Game view has an authoring-input contract.

**A tile volume stacks cells instead of filling one plane.** `sindri.tile_grid`
carries the geometry — columns, rows, cell size, projection, and the screen step
one level costs — and `sindri.tile_volume` carries sparse cells at integer
`(column, row, level)` coordinates naming tiles in a reusable `.tileset`
asset. Empty is absence rather than a reserved ID, so an empty sky costs nothing
to store. A tile says what its faces look like, whether it hides a neighbour's
shared face, whether it is solid, and how much of its cell it fills; the
renderer resolves each occupied cell into only the faces nothing covers, so a
column two blocks taller than its neighbour exposes two side faces without a
cliff case and removing a lower block opens a real hole.

**Runtime tile volumes have one engine-owned chunk unit.** A chunk is 16×16
columns, addressed with Euclidean coordinates and held sparsely by
`TileChunkStore`; materializing it produces the existing scene component rather
than introducing a second cell format. Causeway used it to stream its terrain
until it moved onto `sindri.voxel_world`.

**Causeway's ground is the engine's voxel world.** Its floor is a grid of boxes
whose ground is a `sindri.voxel_world`: the engine's natural terrain from seed
1, drawn with Causeway's own blocks -- grass with its variants and buried
look, earth, sand, mud, moss, rock, snow, water, trees -- and following the
camera. Nothing in `game/` generates, streams or meshes terrain any more. The
walker starts on a bank with the beacon across a river; clicks on the water lay
planks as edits on the world, and the walker crosses them to the beacon
(`game/tests/the_game_is_played.rs`).

**A tile can be shorter than its cell.** `height` is a fraction of a cell
measured from its floor, defaulting to one, so every tile written before heights
existed means what it always did. A half is a slab. The value is logical: it
decides what the tile hides and, later, how high something standing on it
stands, and says nothing about the art — a slab carries its own baked faces the
way a slab in a voxel game is a separate block rather than a squashed one.
Anything taller than one cell is refused when the asset decodes, because
occlusion and occupancy both assume a cell's contents stay inside it, and
something two blocks tall is two cells.

Both projections are real. `sindri.tile_grid` takes `orthogonal` or
`isometric`, and the choice decides three things rather than one: where a cell
sits, which of its faces a view can see, and what "behind" means. An isometric
view is turned between the axes, so it shows a cell's top and two sides and
measures depth along the diagonal; an orthogonal view looks straight down the
rows, so it shows the top and the side facing the viewer — the others are
edge-on — and a column further east is neither nearer nor further. One tile set
serves both: faces a projection cannot see are left undrawn rather than painted
flat across the block.

Depth belongs to a cell's column rather than to each drawn face. Raising a block
moves it up the screen without moving it toward the camera, so a tower keeps the
depth of the column it stands in instead of walking out in front of everything
south of it, and a block's own faces are ordered by face role rather than sorted
against one another a fraction of a cell apart.

**Navigation reads the volume's shape.** A volume is three-dimensional and grid
navigation is not, so the bridge is a *surface*: for each column, the height of
the ground something walking across it stands on — the top of its highest solid
cell, which is a fraction rather than a level because a slab's top genuinely
sits half a cell lower than a block's. A column holding nothing solid has no
surface at all, which is not the same as ground at level zero: it is a hole.

Every edge between two columns is then either a step something can take or one
it cannot, and `sindri-grid` already refuses to cross a blocked edge — so the
volume becomes walls rather than the pathfinder learning about levels.
`sindri.grid.navigation` carries `max_step`, one cell by default: a walker
climbs or drops a block the way it does in the voxel games this reads like, a
two-block wall stops it, and half a cell stops it at anything but a slab. The
limit is symmetric because a wall is; `sindri-grid` blocks an edge rather than a
direction, so separate climb and drop limits wait on directional edges.

A volume only decides this once it *is* the floor. A grid still carrying
`sindri.tilemap` is a migration in progress and navigates by that flat map
alone, so dropping a volume beside an existing floor never closes ground the
game already walks on. And only `WorldGridNavigation::from_world_with_tile_sets`
can answer at all: which cells are solid and how tall they are is a question
only the tile set answers, and guessing would quietly disagree with a caller
holding the asset.

**Gather's floor is a tile volume.** The companion game carries no
`sindri.tilemap` anywhere: its 25×25 island is 625 blocks at level zero with an
outcrop stacked on top, and the shed's floor is 35 more. The flat map's palette
came across entry for entry — five kinds of grass included — so the floor kept
the variation the scene had already authored rather than flattening it. The
moat and the pond are water, which is opaque but not solid, so they are holes: a
walker is confined to the island by the shape of the ground rather than by an
authored wall. Three rocks standing off the shore turned out to be on nothing
once that was true, and now sit on a sandbar.

That last part is what a companion game is for. "Every occupant has ground under
it" was not expressible against a flat map, where every cell was ground by
definition; it became a thing that could be false, and was, in three places.

**A script can ask what the ground is.** `Grid.walkable(floor, x, y)` answers
whether a walker can stand where a point falls, from the walkable surface the
pathfinder walks. Continuous coordinates, because a script asks where it is
about to step and that is a fraction of the way into a cell; which cell that is
comes from `sindri_scene::nearest_cell`, the same rule that decides which column
a walker's own placement reads from, so a walker cannot stand in one cell and
ask about another. Off the grid, water, decoration and empty columns all answer
the same way, because none of them is ground.

This closes a gap the companion game had been living with. Occupancy was the
engine's own answer and `WorldGridNavigation` read it directly, but the only
question a script could ask was about a route between two *entities* — so
Gather's player tagged every solid prop and compared positions, which knows
nothing about terrain. Its pond and its outcrop are not entities, so the player
walked over water and into the hill while the Wisp, reading the same surface
from Rust, went round them. `game/tests/the_island_has_a_shape.rs` walks the
player at each edge of the island in turn and fails if it ever stands on a
column that is not walkable; with the check taken back out of `player.decay` it
reproduces the original bug at column (3, 2).

A walker's placement follows the same distinction. An authored occupant rests on
whatever *supports* it, which is what a pier or a lily on water is; a walker
stands only on what is *walkable*, so over water it sits on the grid's plane
rather than being lifted onto the surface. Making water support things is what
made that difference visible: before it, water had no surface at all and a
walker over it was already at plane height by accident.

What a script still cannot ask is whether a step is *legal from where it is*.
`Grid.walkable` is about a cell, not about an edge, so a script-driven walker
does not honour the `max_step` that decides for navigation whether a rise is a
step or a wall. Gather does not hit this — its outcrop is a stepped pyramid, so
every route up it is one level at a time — but a scene with a cliff beside low
ground would let a scripted walker climb it.

**A volume is resolved once, not every frame.** An island's faces do not move
while the player walks around it, and rebuilding them per frame was most of what
extracting Gather's scene cost: 6.7 ms a frame, of which the volume was 5.7 ms,
spread evenly across decoding six hundred cells of JSON, indexing them, parsing
every face's sprite reference, and building a matrix per face. None of that
depends on where the camera is. It is now resolved into faces once and replayed,
with only the depth each cell sorts at measured again — the one part a moving
camera does change — which takes the frame to 1.2 ms and the volume's share to
about 0.2 ms.

What makes that safe is knowing when the answer stops holding. `World` gives
each entity a revision, bumped whenever anything takes a mutable borrow of it,
and `TextureBindings` and `TileSetBindings` each carry a generation. A volume
keeps the ones it was built from, so editing its cells, moving it, changing its
grid, rebinding its tile set and re-cutting its atlas each show up on the next
frame. The revision is deliberately pessimistic: a borrow that writes nothing
still bumps it, because re-deriving something that did not need it costs time
while missing a change that did shows the player the wrong world. The
revisions and the generations are both handed out process-wide rather than
counted per world or per binding, because both are compared across one: a world
counting its own changes starts at zero, so a freshly opened scene's entity
carries the same revision as the different entity that held its handle in the
scene before -- which is a second scene drawn as the first, and was, until a
test opened two. A host that reloads its art by *replacing* its bindings hits
the same thing from the other side, handing over a fresh object whose count
happens to match the old one's. Whether an entity takes part in the scene is
asked every frame instead, because it is not a property of the entity: switching
off an ancestor is not a change to anything under it.

`crates/sindri-scene/tests/extraction/baked_volume.rs` is one test per input:
each edits exactly one of them and fails if the volume goes on drawing what it
drew before.

**A volume spreads across render layers.** `layer_step` says how many layers one
step of projected depth costs. Zero, the default, puts the whole volume on one
layer, which is right for a backdrop and wrong the moment anything walks between
the blocks. The reason it has to be said at all is that a 2D scene viewed
straight on has no other depth axis: every world draw sits at the same distance
from an orthographic camera, so the render layer *is* the distance. A step of
two leaves an odd layer between each pair of cells for whatever stands on them,
which is exactly the convention Gather's props already followed.

**Block art is baked per height, and per texel.** `tools/isometric-baker` grows
a `grain` option: a material says how big a texel is and how much of its surface
shifts a step along the ramp it already has, so a face reads as stone or dirt
rather than as a grey or brown quadrilateral. The shift is hashed from the
texel's position on the model rather than on the screen, so the same texel is
the same shade in every bake and from every direction — a baked sprite that
changed with the run would not be a baked sprite — and it is clamped to the
ramp, so a grainy material still emits only the four colours its palette
promised.

Height is baked rather than scaled, which is the other half of a slab being its
own tile. An isometric side face is a parallelogram whose vertical edges are the
block's height and whose top and bottom edges slant; scaling it vertically moves
the slant, and the correct shorter face removes a band from the middle, which
one textured quad cannot do. Baking the slab as its own model gives the right
shape for nothing.

**A script reaches one cell at a time.** `Grid.block(volume, column, row, level)`
answers with the tile a cell holds and `Grid.set_block` writes one, with the
empty string meaning absence in both directions — a volume stores a missing cell
as a missing cell, so unlike the flat map it needs no sentinel number standing
in for nothing, and writing `""` removes the cell rather than blanking it. The
column, row and level must be whole, because a cell between two levels is not a
cell.

A write naming a tile the volume's own tile set does not define is refused at
the call. Left alone it would fail the next extraction instead — a frame later
and nowhere near the script that caused it — so the host is handed the tile sets
precisely to put the refusal where the mistake is. A host that has bound none
refuses every write for the same reason, and its volumes say nothing about where
a walker may go.

Occlusion then has to answer *completely* rather than *at all*. A neighbour
hides a side face only when it is at least as tall as the face it abuts, so a
full block beside a slab hides the slab's side while a slab beside a block
leaves the block's upper half showing — which is the whole reason a slab reads
as a slab. A top face survives a block placed above it unless the tile fills its
cell, because a slab's top sits below that block's floor with a gap an isometric
camera looks into.

An animated sprite whose reference names no part of its sheet draws the first
frame of the clip it is playing rather than the whole sheet, so a scene shows a
pose before anything has run it — in a game's opening frame, in an offscreen
capture, and in the editor outside play mode. Once a clip is running it decides
alone: a frame that resolves to nothing draws the whole image rather than the
resting pose, because a plausible picture of the wrong moment is the failure that
hides.
A sprite is either screen-anchored, which is the default and cannot be occluded
by the world, or in the world, drawn through the world camera by its full
transform and hidden by opaque geometry in front of it. Sprites batch per space,
layer, and texture, with a measured five-to-one draw-call reduction. Each batch
draws with its own camera and its own instances, from buffers of its own; a GPU
test draws two batches at two scales into one frame and reads back the pixels
that only come out right when they stayed separate. The frame
clears once before anything draws, so a scene of only sprites has a depth buffer
to test against. Perspective and orthographic cameras exist with tested projection
maths, and the zero-to-one depth range and Y-up convention are chosen in one
place rather than at each call site. Colour space is enforced by a shared
constant, and the offscreen capture verifies authored colours actually survive
to the pixels.

Offscreen rendering produces a deterministic PNG, which CI uploads — two of
them: the cube proof, and the companion game photographed part-way through a
scripted run.

**The engine runs in a browser**, which until recently it had never been asked to
do. The cube example draws the same picture in Chromium as it does natively, in
the same colours, over WebGPU, and **the companion game is playable there**: the
keyboard drives the player, an orb is collected, and a lamp lights, which is
Decay executing on the browser target for the first time along with entity
references, the blackboard and input. `scripts/browser/smoke.mjs` checks that a
page starts the engine and fails when it does not.

What still has *not* run in a browser is asset loading. Both callers embed their
assets, so the decoders run and `AssetLoader` and `UrlRoot` are still only
exercised by tests. `docs/browser.md` records what the first run found, which was
two bugs that had been true since the browser target was added.

### Scene to frame

Gameplay writes to the world and nothing tells the renderer. `SceneExtractor`
derives an ordered frame from whatever the world currently holds, using the
built-in camera, mesh, sprite, tilemap, animation, and text schemas. No scene
needs hand-written extraction code.

Textures bind by reference: a scene names `textures/badge.png`, the renderer
knows only a `TextureId`, and one place knows both. An unbound reference draws a
magenta checker and is reported by name rather than failing the frame or
silently reusing the last texture.

### Assets

`AssetId` is a validated relative logical ID, never a path or URL. Sources
resolve it against a root: the filesystem source canonicalises to catch symlink
escapes, and `UrlRoot` percent-encodes and normalises bases with rules exercised
on every target rather than only in a browser. The load queue is bounded,
rejects duplicates, and carries a generation token so a late completion cannot
overwrite a replacement. Textures, fonts, sheets, scripts, and scene JSON decode
through typed decoders. Nothing pretends browser I/O is synchronous.

A `TextureId` is a generation-checked slot handle, so the renderer's texture
registry can release one texture and reuse its slot without a handle nobody
updated drawing whatever lands there next.

`AssetLoader` drives all of that in one place — request, enqueue, drain, decode,
apply — so a caller does not have to know the order. Requesting is idempotent, a
failure is reported once rather than retried forever, and `retain` releases what
is no longer wanted and says which IDs went, so a host can drop whatever it
built from them. GPU upload stays with the host, which is the only thing that
owns a device.

`FontAssetDecoder` validates OpenType bytes and records their declared family;
text binds that project-owned face under the scene's logical asset reference,
so native and browser builds never substitute different installed fonts.

`ProfileDocument` is the reusable-data counterpart to a prefab: a versioned
`.profile` asset with an author label, an optional game-defined type, and a
nested JSON-shaped value payload. `ProfileAssetDecoder` validates it through the
same native/browser loading path, and the manifest records it as `profile`.
Unlike a prefab it creates no entity; many script components can hold the same
logical profile reference and read one loaded document.

On native, `AssetWatch` notices when the file behind a loaded asset changes and
`AssetLoader::reload` loads it again, by polling modification time and length
rather than subscribing to filesystem events.

`AssetManifest` records what a project ships — each asset's length and the
SHA-256 of its stored bytes — as a versioned, ID-ordered file. A loader given one
holds arriving bytes to it, so a truncated response or a stale cache entry is an
error naming the asset rather than a picture from last week; an asset the
manifest does not list still loads. The editor picks one up from the directory a
scene lives in. The generic browser host sizes each kind's bounded request queue
from that manifest, so projects with more than the asset pipeline's default 64
textures or other same-kind assets still load without increasing fetch
concurrency.

A corpus of deliberately awkward images — every PNG colour type, sixteen bits per
channel, an interlaced encoding, and a JPEG — is decoded and checked pixel by
pixel on both native and `wasm32-unknown-unknown`, so a texture cannot decode one
way in the editor and another in the browser.

---

## The editor

Reflects `main`. The editor renders the real runtime frame through eframe's
shared WGPU device — it is not a mock, and it does not create a second device.


An action that fails says so: a sticky notice from the last thing you did is
kept apart from the per-frame render result, which used to overwrite it within a
frame.

### Works

- **Opens a project, not only a scene.** A project is a directory holding
  `sindri.toml`, which carries a format version, the project's name, and the
  scene opening it opens. The welcome window is its own window and the editor's
  is hidden behind it until a project is open: it lists the twelve most recently
  opened projects, marks one that has moved or been deleted as missing rather
  than dropping it, makes a project — a manifest, a scene, and the folders
  assets resolve from — and opens a folder that already is one. Gather ships
  with a manifest and is listed as a shipped sample when the editor is run from
  the repository. A scene opened from anywhere walks up to the nearest
  `sindri.toml`, so the browser is rooted at the whole project and headed with
  the project's own name — Gather rather than `assets`. What it *lists* is the
  directory asset references resolve against, which is the open scene's own:
  a project's Cargo manifest and `src/` are part of the project and are not
  files a component can name, and the rest of the project is a switch in the
  browser's toolbar away, remembered between launches and drawn only where the
  two listings differ. A launch honours the
  command line first, the last project when the user asked for that, and the
  welcome window otherwise. **Set as main scene** on a scene row nominates what
  the project opens on, and a scene made in a project that nominates nothing
  claims the empty place rather than leaving the project opening on nothing.
  `docs/project-format.md` is the contract
- **Choreographs with sequences, edited in a Timeline.** A `sindri.sequence`
  component holds named sequences, which one is `playing`, and a `speed`, as
  a sprite animation holds clips. A sequence has a duration, may loop, and
  carries tracks and cues. A track names a target (empty for the entity,
  a path of child names such as `Ship/Flame`, or, with a leading slash, a path
  from the scene's top level such as `/camera`, so one director can drive
  anything in the scene), a property
  (`position.x|y|z`, `rotation` in degrees about z, `scale`, `scale.x|y|z`,
  or a component and a path into it such as `sindri.shape/stroke.3`), and
  keys in time order, each with a CSS easing to the next. A cue names a moment
  and may play a sound on a bus. `Sequences` advances every playing sequence
  after scripts, holding each track's first key before it and last key after
  it, wrapping a loop and reaching every cue crossed, the start's on the first
  step and the end's as a one-shot finishes; a finished sequence lets go of
  what it moved. Problems are reported once per start. The shared game session
  (every desktop and browser export) and the editor's Play advance them and
  play cue sounds; Decay reads them through `Sequence.play`, `stop`,
  `restart`, `is_finished`, `time`, `cued`, `playing` and `set_speed`, a cue
  being answered on the step after it is reached. The editor's Timeline panel
  shows the selected entity's sequences as a ruler, a cue lane and one lane
  per track; clicking the ruler moves the playhead, and with preview on the
  Scene view shows the sequence posed there without touching the document.
  Keys and cues are picked, dragged on a 0.05 s grid and edited (time, value,
  easing; name, time, sound); a track is added for any target and property
  and keyed at the playhead from the value the scene holds. Sequences are
  added, lengthened, looped and set to autoplay from its toolbar; every edit
  is one undoable step. `examples/sequence` (Sequence Stage) is the feature
  example and `game/tests/the_sequence_demo_works.rs` plays it. Causeway's
  beacon lights by its own sequences: arriving plays `arrive`, a swell whose
  first cue sounds the chime, and the script moves it on to the looping `glow`
  when that finishes (`game/tests/the_beacon_lights.rs`). A cue's sound is
  found by the export's walk of the scene, so a project need not list it. The
  test harnesses of the platformer, Flappy, Scorchball and Orbital advance
  sequences too. No curve editor, blending, or tracks beyond numbers and cues
- **Plays a scene's audio in Play, and mixes it for the author.** Play starts
  a scene's autoplay `sindri.audio.source`s and performs every request its
  scripts make through `Audio`, through the same `AudioMixer` a build uses;
  clips load from the scene's folder on first use and a missing one is a
  console error. The Audio panel (the canvas arrangement's bottom-right
  corner, beside the inspector when docked, the bottom dock when wide) lists
  each bus the run names, master first, with a trim fader, mute and solo, and
  every sound playing now with its bus, volume and whether it loops, each
  stoppable. The monitor changes only what is heard: `Audio.volume` still
  answers what the game set, and a new run starts from the game's own mix
  while the monitor carries over. Pause holds every voice and Stop ends them.
  Without an audio device Play carries on silently, listing one-shots for a
  second. No level meters, and buses are not yet declared by the project
- **Profiles Play.** The Profiler panel (a top tab beside the Game view,
  under the Scene view beside the Game view when docked) keeps the last 300
  frames of Play, each timed on the CPU by phase: effects, physics, screen UI,
  scripts, sprite animation, cameras, and the Scene and Game views drawn.
  They are drawn as stacked bars against the 60 fps budget; pointing at a bar
  shows that frame and a click pins it, and otherwise the panel shows the
  average frame. Each script's time and runs per frame are listed slowest
  first, summed over every entity running it (`Scripts::set_measuring`
  reports them). Starting Play clears it. GPU time, a breakdown inside a
  script, and timing a shipped build are not measured
- **Shows a project's scenes as a board.** The Scenes panel (a tab beside the
  Scene view, or beside the Game view in the docked preset, and in the View
  menu) has a card for the main scene and each scene in `[project] scenes`,
  with the last frame either view drew of it, or its entity count until it has
  been opened. Arrows are the doors between scenes: literal `Scene.go("…")`
  calls in the scripts a scene's entities and placed prefabs run. A door to a scene the project does not
  carry is marked on its card, with a menu entry to add that scene when the
  file is in the project. Clicking a card opens its scene (asking first about
  unsaved work); **Add scene**, **Remove from project**, **Move
  earlier**/**later** and **Set as main scene** edit the manifest, which is
  written in place so `[web.splash]` and comments survive. Orbital Last Stand
  shows two cards with a door each way, and every shipped project's board is
  checked to have no missing scene and no door to nowhere
  (`editor/src/project/scene_board.rs`)
- Opens a scene from a command-line argument or **File → Open scene**, saves it
  back canonically, reloads from disk, and discards changes — including a scene
  carrying components it has never heard of, which it keeps through a save and
  lists in the inspector
- **Makes a scene and forks one**, so a project can be started rather than only
  continued. New scene (Ctrl+N) asks where the file goes, writes it, and opens
  it through the ordinary path; it holds one world camera, because a scene with
  none renders a black Game view, and it is named from its file. Save scene as…
  (Ctrl+Shift+S) is offered whether or not the scene has a file behind it —
  giving a detached scene one is the case that used to have no answer — and the
  project beside the scene, the remembered scene, the textures and the scripts
  all move with it. A save box takes a name rather than an extension, so
  `level` is written as `level.scene`, which is what the browser lists as a
  scene and what reopening it finds
- Shows the hierarchy from live runtime state as a Unity-style GameObject tree:
  every entity may own children, child-bearing rows fold with state remembered
  across launches, search retains and temporarily opens each match's ancestor
  path, selection works anywhere on a row, and empty space or Escape clears it
- **Lists world objects and UI objects apart**, under a World group and a UI
  group, with icons of their own. Which group a top-level entity is in is read
  from what it carries — a `sindri.ui.*` component means the viewport — so
  nothing has to be kept in step by hand and no entity can claim a space it is
  not drawn in. A group holding only UI elements is listed with the UI.
  Create GameObject makes an empty object or a UI Image directly, and the
  inspector says which space the selected entity is in
- **Snapping increments that can be set**, from a right-click on the snap
  button, and remembered across launches. They were constants the tooltip named
  and nothing could change. A step of zero means that one does not round
- **A console that can be read.** Filtered by level — everything, problems, or
  only what did not happen — and remembered across launches, because someone
  watching for a failure wants it filtered for as long as they are watching.
  Clear empties it, so a transient failure stops being counted once it has
  stopped being true. An entry about an entity ends in that entity's name and selecting it goes there:
  a script failure used to print the runtime's own handle, which is not
  something anyone can look for in a hierarchy
- Inspector edits of name and the complete transform: position, Euler-degree
  rotation backed by the stored quaternion, scale, and the Z lock, which takes
  away movement off the current layer
- **An entity's stable ID, shown and editable.** It is what the file keys an
  entity by, what a parent link names, what sibling order is derived from and
  what `sindri.grid.occupant` points at, and it used to be invisible — so a
  scene made here was `game-object-1`, `game-object-2`, and a shipped scene's
  `player` and `orb-1` were unreachable. Renaming one carries every occupant
  that names it along in the same undo step; one that is blank or already taken
  is refused at the field rather than written
- **The scene itself, where an entity's inspector would be.** With nothing
  selected the panel shows the scene's own name — a real field that round-trips
  through a save — along with its file and how many entities it holds. The
  rename is an ordinary undoable edit, so the editor still knows the document
  is unsaved
- **Editing any component's fields in the inspector**, driven by the stored
  payload rather than by hand-written rows: numbers get drags, booleans get
  checkboxes, text gets a field, and a short numeric array gets a labelled row.
  A component the engine has never heard of is editable too, which is what the
  preserve policy promises. Every edit goes through `SetComponent`, so it
  undoes; every edit is checked against the component's own schema first, so one
  that would stop it decoding is refused and said aloud rather than written into
  a scene that then will not open
- **Controls that know what a field means.** A component draws every field it
  has, filled out from the registry's own blank, so two of one component show
  the same rows and a field nobody wrote down is still visible at what it means;
  only a field actually changed is written back. A value that is one of a few
  names is a menu — a camera's projection, a UI anchor, a tilemap's projection,
  a rigid body's kind — taken from the engine's own list. Some of those names
  decide what else the thing holding them consists of, and the component says
  which: switching a camera to orthographic drops the vertical field of view,
  keeps the near and far planes, and gains a vertical size, and switching a
  collider piece from a box to a circle drops the half extents and gains a
  radius. Both are one edit through one path, at whatever depth the field sits,
  and the registry proved at startup that each variant decodes — so a spelling
  the editor offers is one the engine will accept, which typing the word into a
  text box could never promise. A field naming a project asset offers what the
  project holds while
  staying typeable — spelled the way the open scene resolves it, which is not
  the path from the project root whenever a project keeps its scene under
  `assets/`, and a reference the loader could never reach is offered nowhere
  rather than under a path that will not load, and the texture list includes
  every sprite cut from a loaded sheet (`blocks.png#stone-0`), so a sprite, mesh,
  or voxel face naming one is no longer marked missing — a tint opens a colour
  picker, with three channels or four, a declared range (`FieldMeaning::Range`)
  is a control that cannot leave it, a fixed set of whole numbers
  (`FieldMeaning::OneOf`, such as a shadow map size) is a menu, whole-number
  vectors drag in whole steps, and a row that is only a readout says on hover
  why it is one. `sindri.environment` and `sindri.voxel_world` declare every
  colour, range, choice, and texture they hold, with a test that each declared
  range is one validation accepts
- **When an inspector edit applies.** A component declares, per field or as a
  whole, how an edit to it reaches the scene (`ApplyMode`, registered with
  `apply_when` beside its meanings and checked against its fields the same
  way): `Instant`, as it is made, which is every field that says nothing;
  `Settled`, once the person stops, after a short pause in typing, on Enter
  or leaving the field, or on letting go of a drag; and `Manual`, only when
  Apply in the component's header is pressed, with Revert beside it. A held
  edit shows in the field as typed while the scene keeps what it had, and
  another change to the same component meanwhile, an undo or a script, is
  kept rather than overwritten. A voxel world is `Manual`: several edits can
  be made and the terrain regenerated once, behind a "Regenerating…" notice,
  instead of regenerating, with the editor frozen, at every keystroke
- **Switching an entity off without deleting it.** Off means it takes no part
  in the scene — not drawn, not stepped, not scripted, not picked — and neither
  does anything under it, while it stays in the world and in the file. An Active
  switch on the inspector, Disable and Enable on a hierarchy row's menu taking
  the whole selection, and a struck-through row for anything switched off. The
  flag is per entity and never written down through a subtree, so re-enabling a
  parent brings back exactly the children that were on
- **A History dock showing what undo will do, and everything past it.** The
  undo stack drawn: "Scene opened", every step in the order it happened, the
  step the world is at marked, and the undone steps still listed under it
  because they are still reachable. Clicking one travels there, by calling the
  same undo and redo the keys call, one step at a time
- **Everything the Scene view draws can be clicked in it**, including strings.
  Meshes, world sprites, filled tilemap cells, authored cameras and UI images
  are hit-tested from their own geometry; a string has none in the scene, so its
  box is measured by the text renderer that draws it — the same shaping, at the
  resolution the view renders at — rather than guessed from the font size, which
  would pick the wrong entity along its edges. A fully transparent element is
  skipped in both passes: a thing drawn as nothing is not a thing to click
- **Sibling order, moved rather than renamed.** Move up and Move down on a
  row's menu and on Alt+Up and Alt+Down, greyed out at the ends of a list. The
  order lives in the entity's editor-only section of the file rather than in
  the scene proper, because document order is canonical and meaningless by
  design and draw order is render layers and depths — so where a row sits in a
  panel is a fact about the panel. A scene nobody has reordered still lists
  alphabetically by stable ID
- **More than one entity at a time.** Ctrl-click adds and removes, Shift-click
  takes the range between two rows as the hierarchy is drawing them, and
  Ctrl-click does the same in the Scene view. Delete, Duplicate and a drag to a
  new parent then take the whole selection in one undo step, and dragging the
  handles moves, turns or scales every selected entity by what the one under
  the pointer was moved, turned or scaled by — from each one's own start, so a
  row stays a row. One panel and one set of handles can only be about one
  subject, so the inspector stays on the last entity pointed at and says how
  many the verbs outside it would take; the rest of the selection wears a ring
  in the Scene view where its own handles would have been
- **Every verb that acts on one entity, from that entity's own right-click
  menu.** Rename, Duplicate, Create child, Frame in the Scene view, and Delete,
  each also on a key — F2, Ctrl+D, F, and Delete or Backspace. Rename happens in
  the row itself, focused as it appears, committed with Enter and abandoned with
  Escape, so fixing one name among forty does not move your eyes to another
  panel; a double click starts it. Duplicate copies the whole subtree beside the
  original as a sibling, gives each copy a stable ID nothing else is using, and
  undoes in one step. A project row has a menu of its own for what the browser
  can already do — open a scene, look inside a folder, slice an image — plus the
  asset path a component field wants, the one the open scene resolves against,
  which until now had to be read off the row and typed back in
- **Creating empty root or child GameObjects and deleting entities**, from the
  hierarchy. Creation assigns a stable scene ID immediately, and creating a
  child opens its parent. Deleting takes the whole subtree, and **undo brings
  it back at the same handle** — so the
  selection and every earlier edit in the history keep pointing at what they
  named. That works because the history undoes in order: reaching a delete
  means everything after it is already undone, so the slot it freed is free
  again
- **Adding and removing components.** Add Component groups every type the
  entity's space accepts under Rendering, UI, Physics, Grid or Behaviour — by an
  authored table rather than by the type name's namespace, which is a naming
  scheme and not a taxonomy — and a family holding one offer is listed at the
  top level rather than hidden behind a heading. Only the components that
  *place* something are exclusive to a space, so a UI element can be given the
  script that drives it. It disables the ones that cannot be added yet, each
  saying why: no font in the project, no sliced sprite to build a clip from, no
  tilemap on this entity to navigate. Camera is among them on a scene that
  already has a world camera — a second authored one is a hard extract error, so
  offering it was a button that broke the scene in one click. Text and sprite animation
  are completed at the editor boundary: a project font gives UI Text a valid
  visible default, while a Sprite component whose texture has named sheet
  sprites gives Sprite Animation its first one-frame clip. Both are undoable.
  The menu offers one space or the other: an entity carrying `sindri.ui.*` is on
  the viewport and is not also offered a world sprite, and the reverse
- **A transport that says what it does.** One button enters and leaves play
  mode, labelled Play or Stop by which of the two pressing it will do; one icon
  pauses and resumes a scene already in play mode, and is disabled outside it;
  and a word beside them reads Editing, Playing, or Paused. Ctrl+P and
  Ctrl+Shift+P are the same two actions. Stop puts back everything playing
  changed, from the world as it was when Play was pressed rather than from the
  authored file, so pressing Play never costs an unsaved edit
- **Text authoring.** A `sindri.ui.text` component gets a multiline content editor
  and a picker listing the project's font assets, spelled as the scene resolves
  them. Existing missing font
  references remain visible and are called out instead of silently replaced
- **Sprite-sheet and animation authoring.** Selecting a texture opens its image
  slicer for grid dimensions and cell names. A `sindri.animation.sprite`
  component then creates, renames, and removes clips from those names, orders
  frames, edits frame time and looping, chooses what plays at runtime, and
  previews the selected clip against the real texture. Preview position is
  editor state and never dirties the scene
- **A script's `@export` properties in the inspector**, drawn from what the
  script declared: the field's name, its type, and its default, without running
  anything. A field the scene has not set shows its default and says so, and
  setting one is what puts it in the scene — so a scene records an author's
  choices rather than a copy of every default. This is the capability that
  justified a statically typed language
- Reparenting through a **Parent** menu or by dragging onto another GameObject
  or the World root. Both ask the world's cycle check before offering or
  accepting a move; drag targets show whether they are legal, successful drops
  select the moved entity and open its new parent, and the move is one undo step
- Undo and redo of every edit, with drag-merging so a slider drag is one step;
  Ctrl+Z, Ctrl+Shift+Z, and Ctrl+Y
- An unsaved marker that means the world and the file differ, so undoing back to
  what was saved reads as saved again
- A confirmation before anything that would throw unsaved work away — opening
  another scene, reloading from disk, discarding changes, and closing the window
  — each naming what it is about to do and offering to save instead
- Hot reload: saving a texture the open scene uses shows the edit within about a
  second, without restarting, and without blinking through the missing checker
- Loading the textures a scene names from the directory the scene lives in,
  through the real asset pipeline, so opening a project's scene shows that
  project's art rather than the two textures a demo crate supplied; each load or
  failure is named in the console, and the engine's procedural textures are
  generated rather than loaded
- A live viewport with orbit, pan, zoom, reset-to-authored-camera, and **Focus
  selection** (F), which centres the view on the selected entity; the zoom spans
  a factor of four hundred and moves proportionally, and the orbit cannot be
  driven onto the pole
- An axis indicator in the scene view's corner drawn from the same camera view
  the frame under it was drawn through, foreshortening and reordering its arms
  as the camera turns
- Perspective, orthographic and **2D** toggle. 2D looks straight at the XY
  plane without perspective, and any drag that would orbit pans instead. A
  scene whose camera is orthographic and faces straight down -Z opens in 2D,
  framed on what that camera frames; a scene that is not leaves 2D for
  perspective
- Scene-view click selection for world sprites, filled tilemap cells, and
  meshes. It inverts the exact camera used to draw the frame, uses the
  renderer's unit quad and cube dimensions, resolves transparent overlaps by
  layer and depth, honours opaque occlusion, and clears selection on empty
  space. Camera drags and tile painting retain the pointer when active
- Scene-view Select, Move, Rotate, and Scale tools on Q/W/E/R. Handles use the
  exact rendered camera, support local/world orientation and optional
  translation/angle/scale snapping, respect Z lock, and merge a whole drag into
  one undoable command-history step
- Scene and Game views, the latter rendering through the authored camera with no
  editor chrome painted over it — both live at once in the `Docked` arrangement
- **The local assistant sets itself up inside the editor.** The Assistant
  panel opens on one card: what the assistant does, that it runs only on this
  computer, the whole download (4.7 GB) stated before anything starts, and
  whether the model fits this machine's memory — GPU memory read through
  `nvidia-smi` or `rocm-smi`, unified memory on Apple silicon, otherwise system
  memory. One button, "Set up the assistant", does everything: no download
  page, no terminal, no installer and no password.

  It fetches the pinned llama.cpp server and the pinned Qwen2.5 Coder 7B file
  (`editor/assets/ai-runtime.json`, `ai-model-files.json`, commit-pinned URLs
  with SHA-256 and size) into the person's own data folder through the
  platform's curl, refuses any file whose hash does not match, unpacks the
  runner with `tar` beside its libraries, starts it on a free loopback port,
  waits for its `/health`, and tests that the model works: a question with a
  known answer (17 + 25), passed only if the reply is 42. That test is of the
  assistant, not of any feature, so "set up" means a model that runs here and
  answers correctly. The panel shows the four
  steps as they happen — a check for each one done, a spinner and a sentence
  for the current one, and for downloads a bar with size, speed and time left,
  then "Checking the download…" while it is hashed. Stop works at every step.
  An interrupted setup resumes the download it stopped in and skips what is
  already there; "Continue setting up" says so.

  Failures name their step and say what to do, not what a tool printed: a
  blocked or offline network, a full disk, a damaged download, a moved file, a
  busy server, a model that did not fit in memory, and a runner that stopped
  while loading, whose own log is one click away. Windows and Intel Macs are
  told the assistant is not available for them yet, because nothing is pinned
  for them.

  What it can do in the editor is tested afterwards, feature by feature, and
  shown in the ready card under "What it can do in Sindri" — never as a setup
  step. The test for Decay repair runs by itself as soon as setup finishes:
  the model is given two broken scripts — a misspelt host name, and a script's
  own function called as `this.glow(...)` — and repair switches on only if it
  fixes both, graded by the compiler. A feature that did not pass is shown as
  off, with "Test again", while the assistant itself stays set up; one that
  could not be tested says why. Each result is saved against the exact model
  hash and runner build, passed or not, so it survives restarts, is not re-run
  every time the panel opens, and is redone if either file changes. When
  ready, the panel names the model and the disk it uses, says whether it is
  running or resting,
  stops it on request to free memory, and removes everything, after a
  confirmation, with one button. The server starts by itself on the first fix
  of a session and stops when the editor closes.

  Tested without a network by an end-to-end run: real curl fetching local
  files, real `tar`, and a stand-in runner that answers `/health` and fixes the
  cases. The real pinned runner has been downloaded, verified, unpacked and
  started with these flags on Linux; no real model runs in CI.
- **Compile errors under a previewed script, and a fix the local model proposes
  for them.** Selecting a `.decay` file shows each error with its line and
  column, from `sindri_decay::check_source` — the analysis and host environment
  Play compiles with — or says it compiles. Once a model has passed the repair
  check, **Propose a fix** sends the file, its errors and the generated host API
  (`docs/generated/decay-api.md`) to the model on a worker thread, and the panel
  shows how long it has waited with a Stop button.

  The answer is untrusted. `assistant/repair.rs` takes the longest fenced block
  from it, compiles it, and refuses a candidate that does not compile, is the
  file unchanged, is larger than 32 KB, or no longer declares a script or
  component the original did — since a scene names those, and a fix that drops
  one compiles and breaks the scene anyway. A refused candidate goes back to the
  model with only its own errors, at most twice. What survives is shown as a
  diff with the attempt it took; Accept writes it in one step (a staged file
  renamed over the original), refusing a file that changed on disk since the
  fix was asked for, and "Put the previous version back" undoes it. Every
  outcome is logged to the console. The first fix of a session starts the
  model and says so while it waits. The loop is proved against scripted models
  and the transport against a stand-in runner on a real socket; no model runs
  in CI. It rewrites whole files, which is what the architecture allows for
  short scripts; semantic edits wait on the language tooling.
- **One field that finds anything.** Ctrl+K opens a palette over everything
  else, searching the panels, the scene's entities, the project's files and
  scenes, the arrangements, and the editor's verbs at once; each row says which
  kind it is, because "Console" the panel and `console.decay` the file are
  otherwise two rows that look the same and do very different things. Arrow keys
  move, Enter acts, Escape closes, and a pill in the title bar says the shortcut
  is there.

  Matching is a scored subsequence rather than a substring, so `oscn` finds
  `orbital.scene`, and the query is split into terms matched in any order,
  so a file is reachable by its folder as well as its name. The ranking is the
  part that decides whether a palette is worth having, so it is a pure function
  in `palette/score.rs` tested against the orderings a person would expect
  rather than against particular numbers.
- **A workspace the user arranges.** Every panel — Scene, Game, Hierarchy,
  Inspector, Project, Console, History — is a tab, and every tab is dragged into
  any of eleven places: seven docks — two columns down each side, one along the
  bottom, the centre split in two — and four overlays, one anchored to each
  corner of the scene view. **A dock takes room from the scene; an overlay
  covers it**, which is what makes the canvas-first arrangement expressible
  without a second editor.

  **Edges dock, corners float.** Dropping a tab on a window edge docks it
  there; dropping it in a corner floats it. Dropping onto an existing tab strip
  inserts between the tabs it lands between, and onto a group's body joins it at
  the end. A place with nothing in it is not drawn, so the arrangement has no
  empty furniture in it. Middle-click closes a tab and **View → Panels** brings
  it back; **View → Arrangement** offers `Canvas`, `Docked`, and `Wide` as
  starting points. Where every panel sits and how big it is survives a restart.

  Overlays anchor and stack rather than floating freely: two sharing a corner
  sit one above the other, so they cannot be piled on each other by accident.
  Clicking the tab already showing rolls an overlay up to its strip and back
  down, which is the answer to the honest objection to overlays — they cover the
  world — and costs no travel to a control somewhere else. They are drawn
  opaque: a translucent panel is legible over a mockup's calm sky and
  unreadable over a dense tileset, and which of those the scene holds is not
  something the editor gets to choose.

  Two rules the model enforces rather than asking callers to respect: a panel
  lives in exactly one slot, and the centre is never empty — the last tab in it
  refuses to be dragged out or closed, because an editor with a hole where the
  work goes is not a smaller editor. An arrangement read back from settings is
  repaired rather than trusted.
- Slots resize over a range wide enough for the handle to be a real control.
  They previously could not: an `egui::Panel` persists the size of its
  *contents* rather than its own, so any panel whose contents were narrower than
  their slot sprang back to its minimum on the next frame however far its edge
  was dragged. Every slot now claims its full width before drawing, and
  `editor/src/ui/widgets/panel.rs` carries a headless test that fails if the
  claim is removed
- Play, pause, and stop, driving the real engine lifecycle rather than a display
  flag, and a separate **Discard changes** that returns the world to the file.
  Play advances sprite animations and Decay scripts, and hands scripts the
  keyboard while it does; pause
  holds the frame, and stop puts every clip back to its start. A scene at rest
  shows its clips' first frames, so a broken clip is reported without anyone
  pressing anything
- A Project dock listing the real contents of the directory the open scene
  lives in, with a list/grid toggle, a search that filters it, a refresh, and a
  double click on a scene row to open it
- **A preview for every kind of asset the browser lists.** An image opens the
  slicer, a text file is read in a monospace column, a clip plays on demand, and
  a font draws a sample in the face itself. The last two are what a filename
  cannot answer: which of four `.wav` files is the pickup, and which of four
  typefaces suits a score. The clip plays through the editor's own audio device
  rather than the scene's, so auditioning one needs no running world and cannot
  leave a voice behind in it
- **Creates and edits reusable profiles.** A project-row menu makes a valid
  `.profile`; selecting it opens a structured inspector for its name,
  game-defined type, nested values, groups, and lists. List entries can be
  appended, duplicated, and removed, and Save writes canonical profile JSON.
  An exported Decay `Profile` field uses a project asset picker rather than a
  free-form string.
- A Console dock holding what the editor has actually said — every failure, what
  each scene turned out to be when it opened, and every texture it names that
  nothing has bound — bounded, with a repeated message collapsed into a count so
  a per-frame render failure cannot bury what explains it, and feeding the error
  and warning counts in the status bar
- Reopening the scene the editor was last left in, overridden by a path on the
  command line, with the file and its unsaved state named in the window title
- Preferences that survive a restart
- A deterministic full-window screenshot captured in CI

### Drawn, but does nothing

Listed because a control that looks like a feature is worse than an absent one.
`docs/editor-audit.md` is the full sweep — control by control, with what breaks
under use and what the editor cannot express at all.
`docs/editor-authoring-audit.md` is the second sweep, which asks the harder
question: whether the controls that do work add up to a tool the companion game
could be built in. They now do — every finding it made is fixed except the six
right-click surfaces it tabulates, which are places to put actions that already
exist rather than gaps in what the editor can express. This is the summary, and
it is deliberately short: everything the audits found is either working or gone,
and what is left here is waiting on a build rather than on a handler.

- **Play and Pause** — they run sprite animation and Decay scripts. No other gameplay
  is stepped, so the demo's own turning cube does not turn
Removed rather than left drawn, because each was a promise about a feature that
does not exist: "Scene", "Build", "Tools", and "Help",
which were labels shaped like menus; the top bar's project name; the hierarchy's
old inert add-entity button and the inspector's old inert Add Component button,
both since replaced by working command-backed controls; the inspector's Tag and
Layer, which are not things a Sindri entity has; the inert section chevrons and
overflow menus, superseded in the hierarchy by real child-row folding; and the
settings gear.

---

## Not yet

### Engine

- **Tile System 2 has blocks but not yet terrain.** Volumes store, render,
  cull, and author stacked cells at whole and partial heights, and the grid
  half is shared with the flat map. Terrain-name painting, automatic supporting
  strata, side-face picking, slice views, and collision and navigation derived
  from occupied cells remain; the accepted contract is in
  `docs/tile-system-2.md`.
- World-space text, rich spans, and font fallback are missing. Screen text has
  authored alignment and wrapping, including Weave-controlled wrapping
- **One mesh primitive: `Cube`.** No quad, sphere, or glTF import
- The exercised physics runtime is 2D. There is a Sindri-owned 3D data model but
  no 3D simulation, authoring workflow, or gameplay proof
- Effects are bounded, renderer-free runtime values driven from Decay; there is
  no general authored particle/emitter system or authored parallax system
- Grid walls, footprints, occupancy, and deterministic A* work through the
  engine, inspector, Decay, and Gather. Viewport wall painting, per-path
  policies/costs, and height authoring remain absent
- No optional TypeScript embedding SDK; browser games currently expose narrow
  application entry points and run their gameplay in Decay
- An entity spawned at runtime is not linked to its prefab, by design; only a
  scene's instances are
- Hot reload covers assets, not the scene file: editing a scene on disk while it
  is open is not noticed
- The fixed session pipeline has an explicit order, but there is no extensible
  scheduler or dependency graph for third-party systems
- No dedicated viewport authoring for irregular polygon vertices. The runtime
  and Decay path work, but point handles and a purpose-built inspector are still
  missing

### Editor

- Play mode is intentionally read-only. Stop restores the snapshot from Play;
  editing a running scene and keeping those changes is not supported
- The editor edits one scene at a time; the Scenes panel shows the rest but
  does not edit them side by side. There are no project settings beyond the
  scene list and main scene, and a door named at run time (`Scene.go(next)`)
  is not drawn on the board
- Context menus exist on hierarchy and project rows only. Empty panel space,
  component/property rows, the Scene view, and console lines still lack their
  natural context actions
- No copy/paste of entities or components
- No build/export controls; static web export is currently a CLI and CI workflow
- No versioned editor protocol; the editor and runtime are one process
- Decay source opens as a read-only text preview. The editor cannot modify it;
  editing, completion, and diagnostics live in the VS Code extension

---

## Decay

The gameplay language, in `decay/` — a **separate Cargo workspace**, not a
member of this one. Nothing under `decay/` depends on a `sindri-*` crate and no
engine crate depends on Decay, so everything below is true of the language in
isolation and none of it is true of the engine.

### Works

**A script drives a real entity.** `sindri-decay` binds the language to a world:
a `sindri.script` component names a source and a container, `WorldHost` gives
Decay's symbolic paths a meaning in terms of one entity's transform, and the
editor runs every script once a frame with whatever the transport says a frame
is worth. Authored `@export` properties reach the script before its first line.
Sources load through `sindri-assets` and hot-reload from the same `AssetWatch`
the textures use. Verified in the editor: the fixture's cube turns because
`editor/assets/scripts/spin.decay` says so, and Stop restores the world to the
pixel.

An opaque exported `Profile` is resolved and loaded from the project in the
same pass. `Profiles.number`/`text`/`flag` read scalar values;
`Profiles.count` and the typed `*_at` calls read object records from top-level
lists, always with explicit fallbacks. Orbital Last Stand exercises this by
driving its 160-entry module registry, weighted pools, requirements, display
copy, and generic effects from one profile rather than ID switchboards.

A script reaches its own transform's position, scale and Z rotation, its
sprite's tint and layer, the keyboard, the frame's delta and its own elapsed
time, logical grid position through an explicit tilemap entity, maths functions,
and `print`. The maths host now includes `exp()` alongside `abs`, `sqrt`, `sin`,
`cos`, `atan2`, `min`, and `max`; Orbital Last Stand uses it for the reference
Strider's frame-rate-independent heading interpolation. The whole table is in
`docs/scripting.md`. Verified in the editor: holding an arrow key moves the
fixture's cube, releasing stops it, Space recentres it and puts a line in the
console naming the entity that said it.
**Those paths are typed**, so `this.transfrom.position.x` is a compile error
with a line number rather than a first-frame failure, and reaching for a method
on a container says what to write instead. The analyzer's view and the host's
accessors are derived from one description, and a test walks every path the
analyzer accepts to assert the host answers it.

**Vectors are values.** `Vec2` and `Vec3` are the language's own types: built
with `Vec2(x, y)` and `Vec3(x, y, z)`, added, subtracted, scaled, negated and
compared, with `x`/`y`/`z` components that read and assign, and `length`,
`normalized` (zero stays zero rather than NaN), `dot`, `distance` and `lerp`.
The engine hands them out and takes them back whole —
`this.transform.{position,world_position,scale}`, the same through a reference,
`Pointer.position`, `Pointer.overlay` and `Stick.direction` — through the same
per-component accessors the `.x` paths use, so a whole write and a component
write cannot disagree, and every `this.transform.position.x` line written before
vectors still means what it did. A vector `@export` is stored as `[x, y]` or
`[x, y, z]`, which the inspector draws as its X/Y/Z number row. Mistakes are compile errors: a `Vec2` plus a `Vec3`, a vector times
a vector (pointing at `dot`), `length()` called, a component of a `let`.
Scorchball's ball velocity, kick, aftertouch and bot aim and Orbital Last
Stand's volley spread and bullet homing now use them, with both games' match
and run simulations passing unchanged. The maths host also gained `floor`,
`ceil`, `round`, `sign`, `clamp`, `lerp`, `PI` and `TAU`.

**Scripts name each other by type.** Every `script` in a project is a type
any other script can name: `Bolt.on(entity)` finds one (or `null`), its fields
read and write live — or set what a just-spawned one starts with, which
replaces `World.set_property` — and calling one of its functions sends a
message delivered after the pass, in order, in bounded rounds. A script type is
also an `Entity`. An unknown script, a misspelt field or message, or a wrong
argument is a compile error. The editor and the exporter load every project
script rather than only referenced ones, and the editor's preview and
`decay-lsp` check against the whole project. Orbital Last Stand's player sets
up each bullet it fires this way, and its run simulations pass unchanged.

A failing script reports itself and does not stop the others.

**Camera impacts are gameplay, not Rust glue.** Decay exposes
`Camera.add_trauma(amount)`, which adds impact trauma to the one authored world
camera carrying `sindri.camera.behavior`. Decay decides when the impact occurs;
the engine-owned camera behavior remains responsible for shake strength,
waveform, frequency, decay, and the resulting transform. The camera acceptance
demo uses this call directly, so its movement and Space-triggered impact are
Decay gameplay rather than bespoke Rust rules.

**A whole game's rules are written in it.** The companion game's moving,
gathering, counting and winning are four Decay scripts and no Rust — see "The
companion game" below.

**A script can name another entity.** `Value::Reference` is a value Decay can
hold, pass, compare and store but cannot construct or look inside; the engine
packs a runtime handle into it. `World.find` looks one up by the name a scene
gave it, `World.exists` asks whether it still names anything, `World.despawn`
removes it, and reaching through one gets the same transform and sprite paths a
script reaches on itself — checked at compile time, so `other.transfrom` is an
error with a line number. Reaching through a stale or null reference is reported
rather than silently ignored. Verified in the game: the orbs used to compare
against a position the player published to the shared board, and now ask the
player directly, with the picture unchanged.

**A script can author an irregular world-space polygon on itself.**
`World.set_shape_point(index, x, y)` writes one of at most eight authored points
on the current entity's `sindri.shape`. Non-integer and out-of-range indices are
refused, as is a call on an entity with no shape. The narrow call is deliberate:
it exposes the engine's bounded polygon capability without turning arbitrary
component arrays into a dynamic mutation API. Orbital Last Stand uses it for the
exact six-point Strider hull, and a dedicated Decay test covers both the write
and its bounds.

**And a script can throw flecks that are not entities.** `Effects.burst`,
`burst_at` and `live` put short-lived visual motes into a pool, with what a
burst looks like authored on the entity as `sindri.effect.burst`. Exercised in
`crates/sindri-decay/tests/a_script_throws_flecks.rs`.

**And a game can remember things between runs.** `Save.number`, `set_number`,
`flag`, `set_flag`, `has`, `clear`, and three questions that tell a first run
from a damaged save from one a newer build wrote. Exercised in
`crates/sindri-decay/tests/a_script_remembers.rs`.

**And a script can draw numbers a run can be replayed from.** `Random.value`,
`range`, `int`, `pick` and `seed` read the host's stream, which a seed
completely determines: the same seed and the same sequence of calls give the
same numbers in the editor, in a native build, and in a browser. Exercised in
`crates/sindri-decay/tests/a_script_draws_a_number.rs`.

**And a script can change what the screen says, and read what was clicked.**
`Ui.set_text`, `set_number`, `set_numbers` and `set_fill` write into the element
the entity already carries; `is_hovered`, `is_pressed` and `is_held` answer about
the pointer. The scene owns the words and the script owns the numbers, because
Decay has no way to build a string — a designer authors `"Score: {}"` and a
script fills the slot. Exercised in
`crates/sindri-decay/tests/a_script_drives_a_hud.rs` and
`a_script_reads_a_button.rs`.

**And a script can drive a body and be told what it touched.**
`Physics.set_velocity`, `apply_impulse` and `velocity_x`/`_y` act on an entity's
body; `collision_started`, `collision_stopped`, `sensor_entered` and
`sensor_exited` answer with the entities this one touched during the last step,
as an `List<Entity>`. An event is about a pair, so the answer names the other
half — whichever side of the event this entity was on. Despawning either half
from inside the answer is safe. A host running no physics refuses the call
rather than reporting a velocity of zero for a body that does not exist.
Exercised in `crates/sindri-decay/tests/a_script_drives_a_body.rs`.

**2D continuous collision is opt-in.** `RigidBody2d.continuous_collision` defaults
false, including old payloads. Live dynamic-body toggles preserve velocity and
joints; scene synchronization does not rebuild a body for CCD-only edits.
Typed Decay controls support newly spawned bodies before synchronization.
`crates/sindri-physics/tests/continuous_collision.rs` compares a fast bullet
against a thin kinematic wall with a discrete control; the bridge regression
exercises the spawn window and undo. The platformer wind crate opts in through
its authored body and Decay start control; its launch/rotation and the native
run-to-flag remain checked. Scoped native, Clippy, WASM and catalogue checks passed.
Visual inspector and real browser verification remain in final integration.
Sensors stay discrete; bullet-versus-bullet CCD is not guaranteed.

**And a script can tell where the person is pointing.** `Pointer.x`,
`Pointer.y` and `Pointer.inside` read the position and whether there is one;
`Pointer.is_down`, `just_pressed` and `just_released` take a button name.
`Touch.count`, `Touch.x` and `Touch.y` reach the individual fingers. A button
name nothing answers to is refused exactly as a key name is, and asking for a
finger that is not down is refused rather than answered with zero, which would
read as a finger in the corner of the screen. Exercised in
`crates/sindri-decay/tests/a_script_reads_the_pointer.rs`.

**And a script can ask about several.** `World.with_tag` answers with an
`List<Entity>` — every active entity carrying an authored `sindri.tags` tag,
in deterministic world order, bounded at 8192 and refused rather than truncated
past it. A tag says what an entity *is*, which is the question a game that makes
its enemies as it goes actually has: they have no authored names for `find` to
match, and asking by component type would put `sindri.sprite` in gameplay code.
The answer is a snapshot of handles, so an entity despawned mid-walk leaves one
that `World.exists` answers false for. Exercised in
`crates/sindri-decay/tests/a_script_asks_for_a_group.rs`.

`World.has_tag` answers the same question for one entity without walking the
world. Orbital Last Stand uses it on collision handles so dense projectile
traffic does not become one full-world query per impact.

**And a script can ask which is closest.** `World.nearest(tag, position)`
returns the closest active tagged entity to a world-space `Vec3`, or `null`.
`World.within_radius(tag, position, radius)` returns an inclusive-radius snapshot
sorted nearest first, with stable world order breaking ties. Both use
`World::world_transform`, skip missing/non-finite world positions, and share
`with_tag`'s authored-tag and inherited-active semantics. Radius results are
refused above 8192; `nearest` has no list-result bound. Negative and NaN radii
are errors, zero includes coincident entities, and positive infinity searches
all spatial matches. These are O(n) scans without an index (radius results also
need sorting). Orbital Last Stand's player uses `nearest` directly when the
closest enemy is visible, with a sorted unbounded-radius fallback otherwise.
Arc selects from sorted radius results with its impact-point exclusion. Exercised in
`crates/sindri-decay/src/host/query/tests.rs`,
`crates/sindri-decay/tests/a_script_queries_spatial_entities.rs`, and
`games/orbital-baked/tests/spatial_targeting.rs`. Cone/box queries and physics overlap/shape casts
remain deferred.

**And a script can make one.** `World.spawn` takes a typed `Prefab` — an asset
reference the scene authored into an `@export` field, not a string in the
source, which is what lets the editor resolve it and load the document before
the frame that needs it — and answers with a generation-checked reference to the
new root. Overrides are the ordinary writes through that reference;
`World.set_parent` moves it, and `World.set_property` authors a per-instance
starting value that reaches the spawned script before its first callback. A
second entity can read a numeric authored value through
`World.property_number`, with an explicit fallback and without reaching into
the running script's private mutable state. Orbital Last Stand uses that seam
for projectile-local damage: ordinary shots, criticals, arcs, novas, mines and
beams all use the same collision path without a global damage race. A
running entity receives addressed numeric events through `World.send_signal`
and consumes their accumulated value through `World.take_signal`; this keeps
live combat reactions out of both authored properties and the global board.
Typed access has since superseded both seams for new code: a script names
another by type (`Bolt.on(hit).damage`, `bolt.bounce(2.0)`), and a project
declares events (`event GoalScored(team: f32);`) that any script emits and any
script handles with `on GoalScored(team) { }`, delivered after the pass and
checked across files. Exercised in
`crates/sindri-decay/tests/scripts_reach_each_other_by_type.rs`,
`crates/sindri-decay/tests/scripts_hear_events.rs` and Scorchball. A
spawned script starts within the same pass, so a bullet fired during an update
moves during that update. Both the cascade that allows and the number of
entities one pass may create are bounded and reported rather than run.
Exercised in `crates/sindri-decay/tests/a_script_makes_an_entity.rs` and as
gameplay in `games/orbital-baked`.

The board is still there and still earns its place, for facts that belong to the
game rather than to an entity — the score, whether the game is won.

A project can also declare what goes on the board: `state Game { var score:
f32 = 0.0; }` in any file makes `Game.score` a checked name every script reads
and writes, with one type and one starting value, stored under the same board
name so `Game.get("score", 0.0)` still sees it. A misspelt name, a wrong type
and a write to a `let` are compile errors across files. Exercised in
`crates/sindri-decay/tests/scripts_share_state.rs`, and by Scorchball and the
platformer, whose shared values are declared this way.

A script keeps time with a `Timer` value: `var cooldown = Timer(0.0);`,
restarted with `Timer(2.4)` and read through `done`, `left`, `duration` and
`progress`. Every timer a script's fields hold runs down by each frame before
its `update`, so an early return cannot stop the clock. Exercised in
`crates/sindri-decay/tests/scripts_keep_time.rs`, and by Scorchball's banners,
power-ups, burning, aftertouch and bot reaction times.

A function written outside any script belongs to its file; declared `shared
fn`, it belongs to the project, and every script calls it by name, checked
across files, linking its own copy when it compiles. Exercised in `crates/sindri-decay/tests/scripts_share_functions.rs`
and by Orbital, whose view helpers live in one `view.decay`.

An `enum` in any file is a type every script holds, compares and passes, and
`match` must cover every variant or end with `_`. An enum can be a `state`
field and an `@export` authored by variant name, which the inspector offers as
a dropdown. Exercised in `crates/sindri-decay/tests/scripts_use_enums.rs` and
by Scorchball's match phase and power-up kinds.

Text joins with `+` — with numbers, flags, vectors and variants too — and has
`length`, `contains`, `starts_with`, `ends_with`, `find`, `slice`, `replace`
and case and trimming, counted in characters and capped at 64 KiB. A number is written as text with
`n.fixed(digits)` or `n.padded(width)`. Exercised in
`crates/sindri-decay/tests/scripts_use_text.rs`, by Scorchball's digit and
ring clips, and by Orbital's stat keys and run clock.

A list or struct `@export` is authored in the scene as JSON shaped like its
type and drawn by the inspector as rows per item and per field. Exercised in
`crates/sindri-decay/tests/scenes_author_lists_and_structs.rs` and by
Orbital's card names.

A `const` at the top of a file names a value worked out when the project
compiles — a number, flag, text or enum variant, from literals, operators and
other constants — and `shared const` makes it every file's. Each use is the
value written in place. Exercised in
`crates/sindri-decay/tests/scripts_share_constants.rs` and by Orbital
games' `VIEW_SIZE`.

A `struct` in any file is a value type every script builds with named fields
(`OfferCard(card: c, name: n, blurb: b)`), reads, writes through the `var`
holding it, and keeps in lists; it is copied where it is assigned. Functions
written after its fields are its methods, asked of one value with `this` a
read-only copy, and reach every file as the struct does. Exercised in
`crates/sindri-decay/tests/scripts_use_structs.rs`,
`crates/sindri-decay/tests/scripts_use_struct_methods.rs`, and by
Orbital's module chooser.

Maps are written (`["a": 1.0]`, `[:]`), read by key (`m[k]`, `get`,
`contains`, `keys()`, `values()`, `length`) and changed in place where a
`var` or field holds them (`m[k] = v`, `remove`, `clear`), keyed by text,
numbers, flags, variants or entities in the order keys were first set.
Exercised in `decay/crates/decay-runtime/src/tests/maps.rs`.

Lists are written (`[a, b]`), changed in place where a `var` or field holds
them (`push`, `pop`, `insert`, `remove_at`, `clear`, `xs[i] = v`), asked
(`contains`, `index_of`, `length`) and walked, and `for i in 0..n` walks a
range without building one. `List<T>` is the type host queries return. A
script's list is capped at 10,000 elements. Exercised by
`decay/crates/decay-runtime/src/tests/lists.rs` and by Orbital's
module chooser.

**A script can speak in the tilemap's coordinates.** `Grid.position_x` and
`Grid.position_y` invert a tilemap's projection and full world-XY transform;
`Grid.place` projects a continuous logical position back while preserving the
actor's Z. Both arguments are typed entity references, and the grid must
explicitly be the entity carrying `sindri.tilemap`, so a world with two maps is
not governed by an accidental first match. Gather uses this surface for player
movement, floor bounds, and orb distance checks.

A script is also text on disk that a test can run. `decay-syntax` lexes and parses
it, reporting diagnostics with a span, line, and column; the parser recovers
rather than stopping at the first error, and survives two hundred thousand
random token sequences without panicking or hanging. `decay-semantic` resolves
names through block scopes, checks a small type model (`f32`, `bool`, `String`,
`unit`, named host types), enforces `let` against `var`, and rejects duplicate
members and locals. Host globals such as `Input` enter through an
`Environment` rather than being builtins.

`decay-ir` lowers a checked program to a symbolic instruction list, with member
chains becoming paths such as `this.transform.position.x` rather than anything
the IR interprets. `decay-runtime` executes it: bindings, arithmetic,
comparisons, `if`/`else`, returns, calls between Decay functions, and script
instances whose fields persist across calls. Everything external crosses a
three-method `Host` trait — load a path, store a path, call a path. Each takes a
subject as well: `None` for a path a script rooted at something the host owns,
and the reference for one rooted at a value the script is holding. Three methods
and not six, because a subject is an argument rather than a mode.

The whole workspace compiles for `wasm32-unknown-unknown`, which its CI checks.
`decay/examples/player.decay` is executed by a test rather than only shown in
the README.

### World presentation

`sindri.environment` is authored scene state for background, ambient fill, directional shadows, voxel contact-depth ambient occlusion, and an ordered world post stack. The sun is a `sindri.light` entity (kind `directional`) aimed by its rotation along local -Z; `SceneExtractor::lighting` combines it with the environment's ambient into renderer-owned `WorldLighting`, tolerating an invalid light in the editor; textured cubes, authored meshes, and cached voxel meshes share that model in editor and browser rendering. Voxel Lab is the acceptance surface. Directional shadows use one bounded depth map with authored coverage, 256–2048 resolution, and bias controls; both transient world meshes and persistent voxel meshes cast and receive them. The post stack applies exposure, tone mapping, contrast, saturation, bloom, and vignette before crisp overlay/UI rendering. Screen-space AO for arbitrary geometry, fog, sky, local lights, materials, cascaded shadow quality, and advanced post effects remain follow-up capabilities.

### Not yet

- `while` and `for` are both bounded by the operation budget alongside the
  call-depth limit, and a script's list by 10,000 elements
- No maps, sets, closures, or first-class functions
- No query by more than one tag at a time, and no measured cost for a query at
  combat density
- The language has no built-in standard library. The Sindri host supplies
  `print`, `math.*`, time, random, input, world, profiles, UI, physics, audio,
  effects, and grid namespaces
- Despawning is not undoable — no script write is, and play mode restores from a
  snapshot, so routing it through `WorldCommand` stays open
- There is no general dynamic component API. Purpose-built typed paths cover
  sprites, shapes, UI, physics, audio, effects, profiles, and grid behavior
- No scroll wheel or gamepad surface. The platform tracks scroll; nothing has
  needed it from a script yet
- The official VS Code extension registers `.decay`, highlights the current
  language, and supplies comments, pairs and indentation. It starts the real
  `decay-lsp` at the nearest Sindri project root, so diagnostics, completion,
  hover and document symbols use the project's script types, events and shared
  state. The Sindri project browser opens scripts in VS Code with that root;
  saving uses the editor's existing watched compile/reload path. There is no
  formatter, debugger, definition/references/rename, signature help, or
  semantic highlighting yet
- No script state migration across a reload: a changed file recompiles, and the
  running instance keeps whatever fields it had
- Static exports fetch a manifest and content-hashed assets through the browser
  host. The lower-level `AssetLoader`/`UrlRoot` abstraction still lacks a
  separate application-level browser exercise outside that export path
- The only numeric type is spelled `f32` and every value it holds is an `f64`

---

## Scorchball

`games/scorchball` is the local-multiplayer genre showcase: a couch football
game for two to four pads, ported from an earlier Unity project with its
original art, sounds and tuning. Gamepads were the capability it needed and
were added for it as a general one (see Input). It has no
Rust of its own: a scene, four prefabs and four Decay scripts are the game, and
it exports to the site.

**Players are made when a pad joins.** The match reads `Gamepad.joined()`,
spawns a player prefab, and authors its slot with `World.set_property`; odd
slots play for Blue and even for Red. Unplugging a pad takes its player off
through `Gamepad.left()`. Each player readies with North, picks one of two
characters with the d-pad, walks with the left stick and kicks with the right
bumper, aimed with the right stick, all read through its own slot.

**Scripts talk by type and by event.** A player kicks the ball with a typed
message (`this.ball.kick(velocity, slot)`) and bends the shot with `nudge`;
the match counts players whose live `ready` field is set; a power-up, a
signpost that drops from the sky onto its growing shadow, sends `power_up` to
the player who took it, or emits `WindPicked` or `FirePicked` for whoever
handles them; the ball sends `catch_fire` to an opponent it touches while
alight, and emits `GoalScored` when it crosses a goal line. Every name and
value is checked when the project compiles. Possession, pickups
and reach are distance checks, because a collider does not scale with its
entity and the Enlarger makes a player bigger.

**It found two gaps, both closed.** A child sprite was drawn from its own
transform alone, so the marker over a player's head, a falling signpost and
the ring under the ball had to be placed by script every frame; a child's
transform is now local to its parent everywhere (see World and entities), and
they ride their parents. And the test harness did not lay out screen text, so
a misspelled text anchor passed every test and was first refused by the
editor; the harness now lays it out each step.

**It is checked, not just run.** `games/scorchball/tests/` presses pads: two
join on their own sides, ready up and kick off; Blue takes the ball, dribbles
past Red and scores; a sign cannot be taken until it lands; unplugging a pad
takes its player off; Blue collects signs until it has had all four powers and each does what it says; and a
Fireball sets an opponent running wild until it burns out. Select in the lobby
adds a bot, which attacks, supports, defends goal-side of the ball and goes for
loose balls it can reach first, looking a few times a second with a little aim
noise; the tests have one score on a player standing still, and two play each
other until one scores.

### Not yet

- No music: the original tracks' terms are unchecked, and MP3 is an exception
  the dependency policy would rather not grow.
- No match end, pause or restart without Stop.
- Bots do not pass, go for power-ups on purpose, or come in difficulties.
- Played with real pads by nobody yet: CI has none, and neither did the
  session that ported it.

## Spatial Query Lab

`examples/spatial` is an authored feature project for `World.nearest` and
`World.within_radius`. A movable origin and radius guide accompany the actual
nearest result and ordered radius snapshot. Tag selection, equal-distance
samples, an exact boundary target, active/parent toggles, rotating and scaled
parent transforms, a transformless tagged entity and persistent gray markers
make the query contract observable. Native regressions exercise these cases and
phone touch; browser smoke exercises visible controls and captures desktop/phone
project-subpath and custom-domain exports. This adds no query framework or
acceleration structure; Orbital remains the gameplay proof of the APIs.

## Camera Lab

`examples/camera` is an authored, exported feature project with Decay movement
and runtime camera controls. Camera-relative dead-zone and world-space
camera-center bounds guides, mode toggles, sampled coordinates, an automatic
tour, impact button and touch movement pad make follow, confinement and shake
observable. The native harness uses the same scene/scripts, binds the packaged
font and supplies UI hit-testing; Pages uses the existing shared browser host.
Runtime regressions exercise modes and phone touch; browser smoke covers desktop
and phone controls and project/custom-domain routes. Dedicated editor behavior
controls and gizmos remain absent. See `docs/cameras.md`.

## The platformer

`games/platformer` is the first genre showcase: a side-view level painted as a
tilemap and made solid by a Tilemap Collider 2D, a hero who runs and jumps, ten
coins and a flag, a HUD, and a camera that follows. It has no Rust of its own;
the scene and its Decay scripts are the game, and it exports to the site.

**Its hero uses the scene-owned Character 2D controller.** Decay integrates
velocity and gravity, queues displacement and reads cached walkable support.
One capsule probe checks ground obstacles; body and foot sensors preserve
pickups independently. Walls, ceilings and one-way undersides never grant
standing. It gives the jump the two forgivenesses players expect, a buffer
for a press just before
landing and coyote time for one just after running off a ledge. Letting go
early cuts the rise short.

**It found three gaps, all closed as general capabilities.** A script moving a
body's transform was undone by the next physics step, so a respawn did
nothing; a frictionless hero clung to walls because friction averaged; and
camera behaviors ran only in the camera example, so no other game's camera
followed anything. See Physics and `docs/cameras.md`.

**It is checked, not just run.** `games/platformer/tests/` stands the hero on
the painted ground, has a scripted player hold right and jump at every gap and
wall until it reaches the flag without falling (in about nine seconds), and
holds the camera to following it inside the level.

### Not yet

- No enemies or hazards; raised one-way planks support jumping and timed drop-through.
- No level after the first, no pause and no restart without Stop.
- No site card or captured screenshot yet; the showcase library adds them.

## The companion game

`game/`, crate `sindri-causeway` — "Gather". Five orbs on a diamond floor, a
thing you drive with a keyboard or touch stick, a row of lamps that fills as you
collect them, and a banner that fades in when you have them all. `ROADMAP.md` says why
it exists and why it is not an example; this says what of it is real.

### Works

**Its floor is a tilemap on a sliced sheet.** One entity holds a 9x9 grid of
cells indexing a two-name palette, where the original floor was 49 sprite
entities. Regional tile patterns now divide the expanded island into readable
areas without returning to one entity per cell.

**It is a game you can play.** `cargo run -p sindri-causeway` opens a window.
Arrow keys or WASD move the player on desktop, and the browser build adds a
touch stick without giving Decay a second movement path. Walking into an orb
takes it, taking all five wins, and Escape quits. Gameplay runs on the fixed
step, so gathering happens at the same rate whatever the frame rate is.

**None of its rules are in Rust.** Moving, gathering, counting and winning are
four Decay scripts in `game/assets/scripts/`. The Rust is a window, a device, a
loop, and the embedded bytes of the scene, the scripts, and the art. That split
is what the game exists to test, and it held. The game first earned one engine
feature, `Game.get`/`Game.set`, as a shared blackboard. Entity references later
let each orb find the player and read its transform directly; the board remains
for game-wide facts such as score and victory. Neither change added gameplay to
Rust.

**Its gameplay now uses the diamond's logical grid.** The player and orbs read
continuous coordinates through the floor tilemap, movement clamps to the 9x9
logical bounds, and placement projects back through the same isometric mapping
that draws and picks the floor. Arrow keys follow the two diagonal grid axes;
holding two walks along a screen axis. Orb bobbing remains a presentation offset
and is reset from the logical resting point every frame.

**It is checked, not just run.** `game/tests/the_game_holds_together.rs` asserts
every texture and script the scene names is shipped, that the scripts compile,
that every authored property names a field its script `@export`s, that the scene
holds no component the game cannot run — and it plays the game through the same
scripts and the same `InputState` the window feeds, steering to each orb in turn
and checking the banner comes up.

**It is a CI artifact.** `causeway-capture` plays a fixed run — a fixed key held
for a fixed number of fixed steps — and photographs where that leaves the game,
so the picture proves the scripts ran rather than that the scene loads.

**Its grid reads as an authored place.** The expanded island uses regional tile
patterns, a home shrine, east and west waystones, and standing-stone bars placed
over the same authored walls the Wisp routes around. Regular procedural shapes
draw those landmarks, while a small Decay script turns and pulses the shrine
heart and the Wisp's travelling halo.

**It opens in the editor, and Play runs it there.** All 30 entities load, both
viewports draw it, and the editor advances the same Decay sources the standalone
game does. `docs/editor-meets-the-game.md` records the first editor session
against the older 68-entity scene; the tilemap removed its 49 floor rows and
world renderables can now be selected in the Scene view. The general hierarchy
case is covered too: every GameObject may contain children, folded rows retain
their state, and search shows the path to each match.

**It found a bug the proofs could not.** It is the first thing in the workspace
that draws a world and a screen overlay in one frame, and doing so revealed that
every sprite batch after the first drew with the last batch's camera. See
`docs/rendering-frame-pipeline.md`.

### Not yet

- Gather deliberately presents progress as lamps and victory as a banner sprite;
  it does not exercise a dynamic numeric score even though script-to-text
  template binding now exists
- No restart without relaunching, no menu, no pause. Those fuller product flows
  are exercised by Orbital Last Stand instead

### Scripted pathfinding

Decay can query and advance authored grid occupants through deterministic A* with `Grid.can_reach` and `Grid.step_toward`. The host delegates to the same `WorldGridNavigation` adapter used by engine tests, so walls, occupancy, and whole footprints retain one meaning. Gather's Wisp exercises that path at runtime.

---

## Tooling beside the engine

Things that live in the repository, are run by hand, and change nothing about
what the engine can do. They are listed here so that finding one does not read
as evidence of a capability.

### The isometric baker

`tools/isometric-baker` bakes a 3D model into an ordinary sprite sheet and the
`.sheet` beside it, offline. Its contract is `docs/isometric-baker.md`.

It is emphatically **not** runtime 3D and does not imply any. The engine still
has one mesh primitive, no glTF import, no material authoring and no lighting
system, exactly as the sections above say. Nothing the engine, editor, native
game or browser export builds depends on the tool: the assets it produces are
ordinary PNGs and sheet documents, and by the time a game loads one there is
nothing left to say it was baked.

It bakes three views. Isometric is what it was built for and remains the
default; **top-down** and **side** were added because the isometric camera
cannot reach them by any choice of tile — its pitch is the tile's own ratio, so
a square tile would be a pitch of 90°, and its yaw is fixed at the 45° diagonal.
The flat views state `pixels_per_unit` rather than deriving a scale from a
diamond they do not draw, and each view refuses the other's fields.

Gather uses it for its whole world. Its floor tiles, shrine, waystones, ridge
wall segments, trees and stone outcrops are baked sprites drawn by the ordinary
sprite path; the recipes are `game/assets/textures/*.isobake`, and the PNG
and `.sheet` beside each are generated from them.

The floor is the part that mattered most. Gather's two tile tones differed by
six values out of 255, so the authored regions were invisible and the island
read as one flat dark diamond whatever was drawn on it. It now has four baked
tiles — shore, grass, path, flagstone — and a floor plan drawn in them.

Two things a scene could not previously say, both found by looking at the game
rather than at the code. A sheet now declares **where its sprites meet the
ground** — a quad is drawn centred on its entity, so the middle of the picture
was what landed on the tile, and Gather's player was drawn a third of a ball low
with nothing failing. And the baker now **refuses art that overhangs its own
footprint**, which is what had the shrine covering ground the game still handed
out, so a player standing legally beside it was drawn sliced by a plinth it was
not touching.

Orbital Last Stand uses it for one thing, and the one thing is the point:
`textures/detonation.isobake` bakes a five-frame blast, top-down, as
`variants` of one sheet. That game draws itself with `sindri.shape` and script
arithmetic, which is the right tool for a shield that breathes with your armour;
an explosion is the other kind of thing — not a parameter of anything, over as
soon as it happens, and drawn rather than derived. `scripts/detonation.decay` is
the whole script: it plays the clip and despawns when `Animation.is_finished`
says the clip ended.

Those tiles are **slabs**, not flat diamonds, which needed one thing from the
engine: `sindri.tilemap` gained `tile_overhang`, how far below its cell a tile's
art may reach. The cell is unchanged and still what the grid, picking, occupancy
and gameplay measure in; only the drawn quad grows, downward. Nothing has to be
authored per edge — a slab's sides are only visible where a neighbour is
missing, and a map extracts in reading order, so the three tiles that could
cover a skirt are all drawn after it. The thickness therefore shows exactly at
the island's rim.

What stayed a shape stayed for a reason: the Wisp halo and the shrine heart are
animated every frame by Decay, which no baked frame can do.

### Draw order and collision in Gather

Every world entity's sprite layer is derived from the isometric row it stands
on, and the two that move keep theirs current from Decay. This is not a
refinement: sprites batch by layer *and texture*, and a frame's passes are
ordered by layer alone, so two different textures on one layer are drawn in
whichever order their textures sort in — however far apart they stand. Gather
previously gave the orbs layer 10 and the player layer 20, so both drew over the
shrine from anywhere on the island.

Solid scenery claims its cell with `sindri.grid.occupant`, which the Wisp's A*
already reads, and carries a `solid` tag the player script tests with
`World.with_tag` before it steps. Two mechanisms because the two move
differently: the Wisp steps cell to cell and asks the engine to route it, while
the player walks in continuous coordinates and has to test the cell it is about
to enter. Nothing new was added to the engine for either.
## World presentation

Scenes can author `sindri.environment` as the scene-wide presentation contract. It owns background colour, ambient colour/intensity, contact depth, and world post-processing controls; the directional world light is a `sindri.light` entity drawn and aimed in the Scene view. Textured 3D geometry, including engine-owned voxel meshes, is shaded from the same ambient and directional settings in editor viewports and browser Voxel Lab; the default renderer lighting preserves the previous unlit appearance for scenes without an environment. Voxel Lab is the acceptance lab for the wider presentation track in `docs/world-presentation-plan.md`. Voxel meshes also carry deterministic corner AO controlled by the environment's contact-depth strength. Exposure, tone mapping, contrast, saturation, bloom, and vignette share one ordered world post path while overlay/UI remains crisp. Fog, sky, environment profiles/volumes, local lights, weather, water, screen-space AO for arbitrary geometry, and advanced cinematic effects remain future slices.



### Managed gameplay tweening

`Tween.number`, `vec2`, `vec3` and `color` create typed handles, advanced once
before their owner's subsequent script updates. Typed value reads, pause,
resume, cancel, restart, progress/completion and disposal are exercised by
`crates/sindri-decay/tests/scripts_tween.rs`; ownership, reclamation and the
8192 retained-handle bound are covered by the runtime store tests. Orbital's
powerups now appear with a short eased scale animation, pausing with the game's
menu state; the original collider, attraction, collection and lifetime logic
remain in Decay. `games/orbital-baked/tests/pickup_tween.rs` observes the real
script and authored prefab through completion and pause.

Named easing lives in `sindri-core` and is also used by Weave's existing CSS
transitions. Weave remains CSS-inspired; gameplay uses managed playback controls.
See [scripting](scripting.md#gameplay-tweens) for exact timing and lifetime
semantics. The Pages feature example at `examples/tween` compares the five curves on
movement, scale, rotation and colour, with mouse/touch buttons and keyboard
playback controls. Its real browser smoke observes movement and pause on desktop
and portrait viewports, and its project test compiles and plays the authored scene.
A tween composes on its handle: `Tween.set_delay`, `Tween.set_loops` (0 for
ever), `Tween.set_yoyo`, and `Tween.after` to hold one until another has
finished (the runtime store tests cover each). Orbital's pickup breathes with an
endless yoyo that starts after its appearance (`pickup_tween.rs`), and Tween
Lab's sixth row crosses, waits and returns as a sequence while it breathes
(`crates/sindri-decay/tests/tween_demo.rs`). No editor timeline, arbitrary
component property binding, completion callback or CSS `@keyframes` surface is
claimed complete.

### Reversible floods over voxel maps

Low Tide exercises a map-only `map_flood` overlay with `Grid.set_flood` and
`Grid.flooded`: one continuous water height, strict submersion against the
column's existing surface, a block palette top face and depth shading. Flood
changes reuse cached map columns; terrain edits still invalidate them. Terrain,
biomes, 3D block meshes and sparse edits are untouched, including builtin
natural overland terrain. This capability was found by the moving-home genre
showcase. Its season, cargo loss, waiting and escape rules are Decay gameplay.
Native headless tests exercise keys and phone touch; extraction tests exercise
the rendered faces and cache retention. Editor Scene view remains unverified;
no fluid simulation, 3D flood mesh, decoration or animated/glowing map face is
claimed. See `voxel-terrain.md` for the contract.

Hinge and slider motor authoring uses registered velocity/position choices
from the engine enum, rather than a free-text mode field. Old joint payloads
inherit the default choice. Native editor review of a scratch platformer scene
exercised the endpoint picker, clearing, Save and undo. Rebuilt editor clicks
selected Position for the nested windmill hinge and trolley slider; saved
payloads retain other settings, and slider undo/redo restores omitted defaults
and the selected mode. The completed native review is recorded below.

The authored 2D joint slice has completed native visual review: all four numeric
payloads reopen, nested hinge overrides persist, editor Play switches and
rebuilds modes, and Stop restores the document. Play Save refuses to write,
confirmed by an unchanged file hash. See [the repeatable editor review](physics-joint-editor-review.md).
Engine/editor/Decay/platformer regressions and prior browser checks complete
this 2D slice; 3D joints, gameplay world snapshots and final integration remain.

A read-only 2D sweep/slide movement foundation now proposes displacement against
current collider poses without mutating bodies. Positive skin, bounded iterations,
initial penetration, filters and deterministic ties have native engine tests.
The scene/typed host now integrate it and the platformer adopts it; see
[the contract](character-movement.md). Native editor proof is recorded in
[the character review](physics-character-editor-review.md).

Read-only 2D ground probing adds configurable unit up, bounded slope angles,
travel and skin. Tests exercise zero-travel support, rotated shapes, slope
boundaries, steep obstruction, penetration, filters, current poses and invalid
inputs. The nearest hit remains visible when unwalkable. The composed grounded
movement query adds post-slide support state and optional
downward snapping, default off. Ascending requests and initial penetration never
snap or report grounded; steep support does not snap. Native tests cover landing,
ledge departure, repeated support, descending slopes, blocked ascent, filters and
arbitrary up. Grounded motion now enforces its support slope limit while
sliding: steep contacts cannot introduce unrequested rise, explicit jumps retain
their requested rise, and steep descent remains ungrounded. Ordinary geometric
projection remains unrestricted. Native regressions exercise slope boundaries, descent,
mirrored/rotated probes and filtering. Optional steps now require starting support,
full lift clearance, better forward progress and a walkable landing within the
height cap. Rejected candidates preserve the ordinary result. Native tests cover
all three probe shapes, ceilings/overhangs, height limits, absent/steep landings,
rotated up and exact fallback. Contact-normal refinement prevents flat box-face
casts from producing large artificial hops. Opt-in platform snapshots now verify
old support and sweep synchronized translation/rotation-point carry before the
character move, retaining wall clipping and crush/penetration outcomes. Native
tests cover all shapes, snapshot advancement, simulated kinematic ordering,
rotation, filtering, jumps and step accounting. Hosts still need to capture and
advance snapshots without applying motion twice. Grounded movement now respects
per-piece one-way support sides/cones across sliding, support, steps and carry.
Request-scoped drop-through ignores only one-way solids, preserving ordinary
floors and sensor filtering; hosts own duration and cancellation. Native tests
exercise ascent/descent, rotated normals, deep/shallow overlap, snap, step landing
and carry/drop interactions. Scene/typed Decay integration and platformer
one-way/terrain gameplay now exercise these policies. Native editor proof is
recorded in [the character review](physics-character-editor-review.md); the
authored ferry exercises game translation carry. Rotation follows a chord with
fixed probe orientation; continuous arc and rotating-probe sweeps remain absent.


Scene controller ownership now registers validated `sindri.physics2d.character`
settings beside the existing Collider 2D. It derives a stationary kinematic body,
requires one solid probe and rejects a simultaneous rigid body; extra sensors,
probe offsets/rotation and query masks are exercised. Runtime movement requests
replace per-step input and specify snap permission; timed controller drop-through
uses fixed simulation duration. Fresh support is seeded before the solve and
current platform motion is carried afterward, with motion applied once and
parent-relative writeback. Scene native tests cover spawning, first-step vertical
carry, rotation, parented riders, teleports/rebuilds/inactivity/slot reuse, timers,
request validation, cached script borrows and saved authored data.
Controller queries/render transforms see the new pose immediately; solid response
and discrete sensors process it at the next solve. Same-step response, physical
push impulses, compound solid probes and swept controller triggers remain absent.
Typed Decay now offers queued `Physics.move_character` requests and copied optional
`Physics.character_motion` results; `Physics.drop_through` selects controller or
dynamic-body timers by authored movement ownership. Motion/support/carry fields
and ordered hit lists filter inactive/despawned references without rewriting
historical displacement. Missing controller context and invalid inputs fail
explicitly; invalid requests preserve previous queued input. Shared game runtime,
editor Play and platformer harness offer the same scene context. Native runtime
and shared-session regressions exercise previous-pass reads, replacement, copied
fields/lists, platform carry, filtering, drop/cancellation and physics-only hosts.
A rebuilt Chromium export of the shared-session fixture runs queued movement,
observes grounded/copied results and renders the controller above its floor.
The platformer now adopts the controller through its real Hero script: native
regressions reach the flag with coins and no falls, traverse/drop through planks,
and exercise variable jump height, acceleration/braking and respawn. The dynamic
crate retains CCD and contact-impulse proof. Native editor authoring/undo/Play
is reviewed; the browser goal gate now exercises the normal spawn and keyboard
controls through the flag with coins and no falls.
Two authored stone risers and an inclined boardwalk now exercise accepted steps,
walkable ascent and downhill snap through the Hero script without jumping.
Control tests disable stepping or lower the slope limit to show the same terrain
blocks walking. Browser keyboard captures show the hero traversing that terrain;
delivery/input evidence is recorded in the physics checkpoint.

The platformer's one-way plank ferry crosses the first gap using ordinary
kinematic velocity set by Decay. Hero adds no platform motion: scene support
snapshots supply each actual solved displacement once. Native game regressions
ride across and back with fixed relative position, board through a running jump
from the starting field, leave/reacquire carry on jump/landing, and drop through
into the pit/normal respawn path. Chromium exports the same assets with the hero
initially aboard and renders riding in both directions and a keyboard jump.
This is moving-platform translation proof for the genre showcase; rotation and
carry clipping retain engine/scene test evidence. Native editor proof is recorded
in [the character review](physics-character-editor-review.md);
browser goal proof is exercised by the normal-spawn keyboard gate.

Native Character 2D editor review exercises every setting, snap undo/redo,
save/reopen, add/remove through checked commands and restoring custom settings.
The reopened platformer runs/jumps in Play with a read-only inspector; Save
refuses during Play and file hashes confirm Stop restores the document.
See [the repeatable review](physics-character-editor-review.md). This is editor
proof rather than a new implementation. The browser goal gate completes the 2D
character acceptance; final-head physics integration remains open.

The exported platformer now has a CI browser goal regression. A scratch copy adds
one read-only Decay observer and retains the shipped starting pose and scripts.
Playwright drives ordinary movement/jump keyboard events using the native goal
player's terrain decisions, waiting for an observed release between jumps.
The gate asserts the flag, at least three coins and zero falls, and preserves
asset-fetch, WebGPU, pixel and script-error checks. Repeated local Chromium runs
collected five coins with eight jumps and no falls. This completes the 2D
controller proof for the genre showcase; it does not supply a 3D controller or
close the separately documented compound/solver/trigger/carry limitations.
