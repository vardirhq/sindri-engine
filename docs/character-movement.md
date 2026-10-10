# Character movement

2D status: geometric sweep/slide, ground probing, grounded snapping, slope
limits, optional steps, synchronized platform carry, one-way controller policy
and scene runtime ownership implemented; 2D acceptance is complete in
`physics-update.md`. Added for the platformer genre showcase, whose Hero script
now uses the controller. Authored low steps and an inclined boardwalk exercise
step selection, walkable ascent and downhill snap. A one-way plank ferry across
the first gap exercises solved translation carry, reversal, boarding, jump/landing
and drop-through. Native editor authoring/undo/save/reopen/Play is exercised in
[the editor review](physics-character-editor-review.md). The exported platformer
reaches its flag through real Chromium keyboard input with five coins and no falls.

## Ownership

`sindri-physics` owns geometric movement and collision results, independently of
scene JSON, Decay, the editor and gameplay. No new crate or dependency is needed.
The engine computes a proposed displacement; gameplay chooses speed, gravity,
jumping and whether to apply that proposal. Future scene/host integration must
use this same primitive rather than implement another collision algorithm.

## Implemented geometric primitive

`PhysicsWorld2d::move_and_slide(shape, pose, displacement, options, filter)` and
`move_and_slide_where(..., include)` are read-only queries. The probe can be a
box, circle or capsule; rotation stays fixed during a move. The caller supplies
world-space displacement, not velocity or a timestep. The result changes no
body pose, velocity, force, joint, contact or event state.

`SlideOptions2d` carries a positive finite `skin` distance in world units
(default 0.01) and an iteration budget in 1..=32 (default 8). Each sweep travels
until the probe reaches the skin around a surface, then removes the untravelled
motion into its outward normal and sweeps the remaining tangent. Rotated
surfaces use world normals; impact contact queries refine skin-cast geometry to
avoid false upward motion from tilted cast normals on flat box faces. Contact
prediction uses the same skin-relative allowance as support probing; when no
contact is available the cast geometry is retained. Small numerical normal error
can still require more than one hit on the same surface. Sub-epsilon remaining
movement is discarded.

`SlideMotion2d` returns actual translation, remaining sliding displacement,
ordered collisions and two outcome flags. Collision distances are relative to
each sweep start, not accumulated path length. Blocked normal movement is
removed, so `remaining` is not the original displacement minus translation.
`iteration_limit_reached` means unapplied tangent movement remains after the
budget. A head-on wall can block all movement without exhausting that budget.

Strict initial penetration blocks the whole move: translation is zero,
`remaining` is the requested displacement and `started_penetrating` is true.
Its collision has zero distance/normal and the probe origin as its point.
This includes zero-motion requests; the caller chooses recovery or respawn.
Exact touching and separation inside the skin permit tangent/escape movement,
but approach is blocked. This slice does not perform depenetration or push the
probe out to the skin when it starts closer than that.

Validation precedes queries: shape dimensions, pose, displacement, destination,
movement length and options must be valid/finite. Overflowing destinations or
lengths fail instead of returning non-finite results. Query filters match the
existing mask, sensor inclusion and whole-entity exclusion contract. The host
predicate can remove inactive entities and must be stable across the call.
Equal-distance contacts prefer entity handle, then original collider-piece
order. Inserts, teleports and removals update the query-only per-piece BVH before
a physics step. Penetration uses probe bounds; skin support inflates those bounds
by contact prediction; sweeps use start/end bounds expanded by target separation.
The exact geometry and one-way policy are unchanged. Previous-platform support
reads only that entity's pieces at their historical poses, so current bounds
cannot discard support that has moved away. Predicates visit spatial candidates
and must not depend on visits to remote entities.

Ordinary ray/overlap/shape-cast contracts are unchanged. In particular, ordinary
shape casts still return a zero normal at zero-distance overlap. The movement
sweep privately requests impact geometry at the skin and ignores tangent or
separating contacts so they cannot hide a blocking obstacle farther ahead.

## Implemented ground classification

`PhysicsWorld2d::probe_ground(shape, pose, options, filter)` and
`probe_ground_where(..., include)` share the movement narrow-phase helpers and
current-pose filtering. They are read-only and cast along negative world-space
up without changing probe rotation.

`GroundOptions2d` defaults to up `[0, 1]`, maximum slope angle pi/4 radians,
maximum travel 0.1 and skin 0.01 world units. Up must be finite and within 0.0001
of unit length; accepted up vectors are normalized before use. The angle must
be finite and within 0..=pi/2, travel finite/non-negative and skin finite/positive.
An overflowing probe destination fails before any query.

`GroundProbe2d` reports the nearest hit, `walkable` classification and initial
penetration. Distance is downward travel to the skin, not to the bare surface.
Steep hits are returned as unwalkable, never skipped in favor of a floor below.
A surface is walkable when its world normal is within the configured angle of
up, with a 0.0001 cosine tolerance for narrow-phase approximation. A nearby
walkable hit does not itself mean the caller is standing on it.

Initial penetration returns the usual zero-normal hit and is never walkable.
Exact touching/inside-skin support can be detected with zero travel: a contact
query, allowing one percent of skin plus one f32 epsilon for numerical error,
precedes the sweep. Rotated/curved sweeps can stop slightly outside the nominal
skin; this tolerance keeps snapped endpoints supported without moving them again.
Prediction is capped at the largest finite float. Blocking contacts at zero
travel tie by entity then original piece order. A surface tangent to the probe direction is not support. No hit
means unwalkable, not penetrating.

This API does not snap, choose jump behavior, carry platforms, limit uphill
movement or implement one-way support/drop-through. The composed grounded
movement API below adds support state and optional snapping; remaining movement
policy belongs to following controller slices. One-way geometry still has the
ordinary two-sided geometric query semantics here.

## Implemented grounded movement and optional snapping

`PhysicsWorld2d::move_and_slide_grounded(shape, pose, displacement, options, filter)`
and `move_and_slide_grounded_where(..., include)` compose the existing slide and
support queries. They remain read-only and do not retain state between calls.
The same filter and stable entity predicate apply to both phases.

`GroundedSlideOptions2d` supplies slide settings, up, maximum slope angle and
non-negative finite `snap_distance` and `step_height`. Both default to zero.
Slide skin is also the support skin, so movement and grounding cannot disagree about that separation. Up and angle use
the ground-probe validation contract. Snap defaults to zero: support is then
recognized only within skin plus its numerical tolerance after sliding. A floor near
the starting pose cannot keep the character grounded after it walks off a ledge.

A positive snap distance allows a downward probe at the slide endpoint. Only
walkable support accepts the downward translation to the skin. A steep surface
remains visible in the ground result but never snaps; it is not skipped for a
floor below. Requested motion with any positive component along up disables
snap and reports ungrounded, even if a ceiling blocks ascent or touching support
is still visible. Initial penetration likewise blocks snapping and grounding.

`GroundedSlideMotion2d.translation` is the full proposal to apply once.
`slide` retains the selected slope-limited sweep result, including its unsatisfied
movement and budget flags. It starts after any carry, otherwise at the supplied pose;
for an accepted step it is the forward sweep from the lifted pose.
`step_translation` is the accepted lift (zero for ordinary movement), and
`snap_translation` is the additional downward landing/snap motion. Their sum
with `slide.translation` and any `platform.motion.translation` is total translation. `ground` is the support query at
the selected slide endpoint before landing/snapping; its distance describes that
extra downward travel. `grounded` means the proposed endpoint
has walkable support and the request was not ascending or initially penetrating.
Snapping is separate from slide collisions and does not alter slide remaining.
An exhausted slide budget still permits support probing at its actual endpoint.

Gameplay keeps prior support state and chooses when to enable snap: typically
while following ground, with zero snap in air or while jumping. Speed, gravity,
jump timing and support transitions remain Decay decisions. The engine does not
supply coyote time, jump buffering or persistent state. Arbitrary up and fixed
probe rotation are supported. Ordinary shape casts and one-way geometry retain
their existing contracts; this is not yet the full controller.

## Implemented slope movement limits

Grounded movement uses `max_slope_angle` for both support and sliding. The
geometric `move_and_slide` query retains its unrestricted tangent projection;
both paths share one sweep implementation. Slope classification shares the same
0.0001 cosine tolerance with ground probing, including the configured boundary.

On an upward-facing, unwalkable contact, the remaining tangent cannot gain rise
beyond the positive upward component of the remaining request. A horizontal or
downward request cannot turn into an uphill climb. An explicit jump can still
slide along the incline, with its rise capped to what was requested. Scaling the
tangent preserves separation from the surface; removing just its upward component
would point back into the slope. Motion blocked by this policy is discarded, like
blocked normal motion, and does not count as exhausted iterations.

Walkable inclines use the geometric projection, which reduces displacement when
approaching a surface; this API does not maintain constant speed along slopes.
Downward sliding on a steep incline remains possible, but the surface cannot grant
grounding or snap. Downhill following of walkable support uses the caller's
optional snap distance; requests that leave its reach become airborne. Vertical
walls and ceilings retain ordinary sliding. No autonomous gravity or downhill
acceleration is applied. Up is world-space and independent of probe rotation.

This general policy is exercised by the platformer's inclined boardwalk through
its real Hero script. Native tests climb and snap down it without jumping, then
lower the slope limit and disable steps to show the same incline blocks walking.
Native editor authoring/undo/save/reopen/Play is exercised in
[the editor review](physics-character-editor-review.md). The exported platformer
reaches its flag through real Chromium keyboard input with five coins and no falls.

## Implemented steps and clearance

Positive `step_height` enables a conservative alternate lift/forward/landing
path. It requires walkable support at the starting pose, no initial penetration,
a non-ascending request and a non-walkable collision opposing its horizontal
component. Horizontal travel must exceed one skin. Unobstructed motion and jumps
retain ordinary sliding without adding a lift.

The full configured lift must be clear at the skin: ceilings and overhangs cannot
redirect it sideways. A second slope-limited sweep moves the horizontal component
from that lifted pose. The candidate must improve horizontal progress by more
than one skin over the ordinary result and finish its forward iteration budget.
A downward probe travels at most the step height, accepting only walkable support;
it never skips a steep surface for the floor below. Final rise cannot exceed the
configured height, and a zero-travel probe confirms non-penetrating support at
the proposed endpoint. Probe rotation stays fixed through every phase.

On acceptance, step lift, forward motion and downward landing replace the
ordinary path. On rejection, the ordinary result is retained exactly. Downward
requested motion is replaced by the landing sweep, so an accepted step follows
support rather than applying gravity displacement again. Landing is independent
of ordinary `snap_distance`, so stepping can work with snap disabled. It does not
retain an airborne step or bridge a gap without reachable support. Each slide
phase has its own bounded iteration budget; an attempted step adds at most a
lift, a forward sweep and support probes to the original query.

The full-height clearance requirement is deliberately conservative: a lower
obstacle beneath a ceiling that blocks the configured lift falls back to ordinary
sliding rather than searching smaller heights. Minimum progress is tied to skin;
very small movement requests do not step. These limitations remain visible for
future game integration. Stepping, like slope limits, is added generally for
platformer adoption. Its two authored 0.25-unit stone risers now exercise accepted
step lifts through the Hero script; disabling steps makes the first riser block.
Intended horizontal speed remains script-owned at obstacles, so clipping one
request does not prevent the next request from advancing over a low step.

## Implemented moving-platform carry

`GroundedSlideOptions2d.platform_support` defaults to `None`. A supplied
`PlatformSupport2d` identifies the previous ground body's runtime entity and its
previous synchronized pose. Before carrying, the engine verifies walkable contact
at that pose with the current local collider shapes/offsets/rotations and the same
filter/predicate. Removed, filtered, unsupported, previously penetrating or steep
support is ignored. Previous pose and character request validate before queries.
Support verification uses skin contact tolerance and authored compound order.

The previous inverse pose maps the probe origin into the support body's local
coordinates. Its current synchronized pose maps that point back into world space;
the difference is carry displacement, including translation and rotation about
the body origin. This displacement is swept first, against all filtered entities
except the carrying body. The character retains its own rotation. Grounded
movement, steps and snapping then run against the whole current world from the
carried pose; the support body is included again. A platform moving upward is not
mistaken for a character jump. Explicit jump requests still lose grounding after
carry. Carry does not add platform velocity or choose jump velocity inheritance.

`GroundedSlideMotion2d.platform` is a `PlatformCarry2d` when old support verifies.
It reports the entity, current pose, requested point displacement and
`motion` clipped by collisions. Carry collisions, penetration and exhausted iterations are
separate from the selected character slide. The actual carry translation is
included once in total translation. Wall clipping may leave support under the
character; a rising platform obstructed by a ceiling can leave initial penetration
in the following slide/ground result. Recovery, crush damage and respawn are
ordinary Decay gameplay decisions; the query never pushes through obstacles.

Hosts capture support from the final grounded hit, synchronize bodies, and supply
its prior pose on the next movement pass. Pending kinematic targets or velocities
produce no carry until the actual physics pose advances. Advance the snapshot on
every pass, even when carry is clipped, so old motion is not retried. Reuse the
returned current pose only if the final ground hit still identifies that entity;
otherwise capture the new support or clear the snapshot when airborne. Clear it
on teleport, parenting changes or collider/body rebuilds. Current local shapes
are used for verification, so the query cannot reconstruct previous geometry
across structural edits.

Do not also add support displacement via transform parenting, solver movement,
velocity or the character's requested displacement. This read-only API does not
store or consume snapshots; submitting the same old pose again can repeat motion.
The scene/host integration must enforce one application per movement pass.

Rotation carry sweeps a straight chord between the old and new point positions;
it does not sweep the circular arc or rotate the probe. Large rotations can miss
obstacles on that arc or leave a non-circular probe intersecting the support.
Use smaller synchronized steps and inspect the returned penetration/support
state. Continuous arc carry and rotating-probe sweeps remain absent and have
explicit parity gaps. The platformer's authored kinematic ferry now proves
translation carry through its real Ferry and Hero scripts: a stationary rider
follows each solved displacement once across the first gap and back, a running
jump boards it from the starting field, jump/landing clears and resumes support,
and drop-through leaves carry and falls into the normal respawn path. Native
regressions assert relative position, carry/total displacement, both directions
and airborne null support. Chromium runs the same game assets with the hero
initially aboard, showing stationary riding, reversal and a keyboard jump.
Rotation/collision-clipped carry still has engine/scene regressions rather than
an authored game encounter. Native editor proof is recorded in
[the editor review](physics-character-editor-review.md).

## Implemented one-way controller policy

Grounded movement respects `OneWay2d` on each enabled solid piece by default.
The policy's normalized local normal follows both body and piece rotation.
Approach must not move along that normal, the contact normal must lie within
its support cone (one f32 epsilon cosine tolerance), and the probe's rear
support point must start on the front support plane within 0.05 world units.
That last allowance matches solver support slop: shallow front penetration
still reports initial penetration; a probe inside/from below passes until it
clears the plane. Movement uses displacement, not the solver's velocity threshold.
Ordinary solids retain strict initial-penetration behavior.

`GroundedSlideOptions2d.drop_through` defaults false. True ignores only one-way
solid pieces during this entire request: slide, initial penetration, support,
snap, step clearance/landing and previous-platform verification/carry sweeps.
A dropped one-way support cannot carry or ground the probe; an ordinary support
can. Sensors retain their ordinary geometric behavior when explicitly included.
Masks, whole-entity exclusion, host predicates and deterministic ties still
apply. A rejected one-way hit cannot hide an ordinary floor farther below.
The public geometric slide/ground/ray/overlap/shape-cast queries remain two-sided.

The query has no body identity or timestep and stores no drop timer. The host
owns duration and cancellation, supplying the flag for each simulation request;
false restores policy immediately. This does not read or modify the separate
dynamic-body `drop_through(entity, seconds)` solver timer. Scene/Decay integration
must choose one movement owner and maintain its timed drop state explicitly.

Native regressions exercise all probe shapes, ascent/descent, underside/deep
penetration, shallow front penetration, cone rejection, piece/body rotation,
snap/drop to an ordinary floor, request cancellation, mixed solid/sensor pieces,
step landing and carry interactions. This remains engine evidence for platformer
adoption, with no editor/Decay/game proof yet.

## Implemented scene ownership

`sindri.physics2d.character` configures skin, iteration budget, up, slope angle,
snap/step distances and `carry_platforms` (default true). Omitted settings use
engine defaults and deserialize through the same validation. Add an ordinary
Collider 2D with exactly one solid box/circle/capsule piece; additional sensors
are allowed. The controller uses that piece's offset, rotation and filter mask,
excludes its own whole entity and ignores inactive entities. Shape dimensions
retain existing world-unit collider semantics; transform scale is not a second
shape-size authoring path. Compound solid probes remain absent.

The scene derives a stationary velocity-kinematic body for the controller.
An authored rigid body on the same entity is a configuration error, as are a
missing transform or an absent/multiple solid probe. Gameplay owns displacement,
velocity, gravity and jumps; solver linear/angular velocity is zeroed before
stepping so it cannot add a second movement. Input and runtime state are held
beside `World`, never in serialized component fields. Saved settings and unknown
payload fields survive reopening; pending motion, support handles, drop timers
and result caches do not.

`ScenePhysics2d::character_requests()` returns `CharacterRequests2d`:

- `move_character(entity, displacement, snap)` queues world displacement for the
  next fixed update; a later request replaces it. `snap` permits authored snap
  for this request, while upward displacement always suppresses it.
- `drop_through(entity, seconds)` replaces the controller's remaining simulation
  duration; zero cancels. Any positive remainder covers the entire movement
  pass and decrements afterward. Pause retains requests/timers; spawning starts
  them when synchronized. Nonfinite, overflowing or negative request values are rejected
  before replacing valid input. Missing, inactive and non-controller requests
  are discarded at synchronization. An invalid timestep consumes nothing.

No-request frames use zero displacement and enable snap only if the last result
was grounded. Carry and support queries still run each fixed step. Settings,
parenting, collider/body rebuilds and actor teleports invalidate cached state;
support rebuilds/teleports also clear riders' old snapshots. Fresh support is
probed at zero travel before the solve, so a newly spawned/rebuilt actor can ride
that step's moving platform without inheriting older motion. Drop requests are
consumed after synchronization, keeping same-step input valid across rebuilds.

The scene then advances the physics solve, runs controllers against those current
poses in entity order, applies total motion once to each controller's backend
body and writes world poses into local scene transforms. Z, scale and the probe's
fixed rotation use the existing physics writeback contract. Final walkable
support captures the current platform pose for the next pass; departures/jumps
clear it. Parenting to a simulated support does not add a second carry.

`character_motion(entity)` exposes the last applied `GroundedSlideMotion2d`.
`for_scripts_with_characters()` returns disjoint physics/event/request/result
borrows; `CharacterMotions2d::get(entity)` reads a shared cached result. Scripts
must check activity when reading during a pass, just as with other snapshots.
Existing `for_scripts()` remains available for hosts without controller APIs.
The shared game runtime, editor Play and platformer test host now supply these
borrows to Decay alongside the existing physics context. Physics-only hosts
retain their independent driver and existing APIs.

**Solver timing limit:** controller motion occurs after the solve so carry uses
actual current platform motion. Queries and render transforms see the applied
pose immediately; solid response and discrete sensor enter/exit processing see
it at the following solve. Moving a controller invalidates its prior solved
contact snapshot, so grounding must use controller results. Controllers do not
apply physical push impulses or swept trigger events. A fast controller can
cross a small sensor between solves without entering it. These limits are
explicit parity gaps; this is not a same-step solver-response implementation.

## Typed Decay requests and motion snapshots

`Physics.move_character(entity, displacement, snap)` queues world-space `Vec2`
displacement for the next fixed scene step; later input replaces earlier input.
The actor must be active, have valid Character 2D settings and a transform, and
have no competing authored rigid body. Probe assembly (including tilemap pieces)
is validated by scene synchronization before queued movement applies. Finite
components and finite displacement length are required; invalid arguments leave
pending input intact. Calls before a spawned actor's first synchronization are
valid. A host without scene controller context reports an error.

`Physics.character_motion(entity)` returns a copied optional `CharacterMotion2d`
from the last completed fixed pass. Check for null before reading: there is no
result before first movement, for an inactive actor or without its character
component. Stale entity handles are errors. All scripts in a pass read the same
cached results while queuing future input. Settings, parenting and teleports
invalidate caches at synchronization, not by changing an already returned copy.

The snapshot contains total `translation`, selected `slide_translation`,
`remaining`, `step_translation`, `snap_translation`, `grounded`, optional
`ground` (`RayHit2d`), `ground_walkable`, `ground_started_penetrating`, slide
`started_penetrating`/`iteration_limit_reached` and ordered slide `collisions`.
Carry has nullable `platform`, `carry_requested`, `carry_translation`, ordered
`carry_collisions` and separate `carry_started_penetrating`/
`carry_iteration_limit_reached`. Without carry its vectors, lists and flags are
zero/empty/false. Hits copy world point, normal and phase-relative distance.
Inactive/despawned hit/platform references are filtered during each call;
filtered ground also clears grounded/walkable in that copy. Applied displacement
and other flags remain historical. Editing fields, nested hits or lists cannot
change the engine cache or future movement.

`Physics.drop_through(entity, seconds)` dispatches authored characters to the
scene-controller timer; dynamic bodies retain the existing solver timer and
physics-only host compatibility. A finite non-negative duration replaces pending
time, zero cancels and invalid values preserve the queue. Timers begin at the
next valid fixed pass, cover its whole movement and decrement afterward; pause
retains them. Controller drop suppresses only one-way solids across slide,
support, snap, steps and carry. Neither request changes saved component settings.

Native regressions exercise previous-pass reads, input replacement, copies,
carry/support filtering, invalid input and controller timer dispatch. A shared
session regression exercises the same context the exported game uses. A rebuilt
Chromium export at `/examples/character-api/` runs that fixture script, observes
queued movement and grounded results, verifies snapshot edits leave the cache
intact and renders the controller above its solid floor. This API
is added generally for platformer adoption. Its Hero script now integrates
gravity and velocity, queues movement and grounds from cached support. One
ground-filtered solid capsule supplies the probe, while separate body/foot
sensors preserve pickups. Native runs reach the flag with coins and no falls,
traverse/drop through one-way planks, and check variable jump height, braking
and respawn. The dynamic crate retains CCD and contact-impulse proof.

## Browser goal proof

The CI browser job copies the shipped platformer with
`scripts/browser/prepare-platformer.py`, exports it through `sindri-export`, then
runs `scripts/browser/smoke.mjs` with `SINDRI_PLATFORMER_GOAL=1`,
`SINDRI_EXPECT_ASSETS=1` and `SINDRI_BASE_PATH=/examples/platformer/`.
The copy adds only `tests/browser_goal.decay`, a read-only observer. The normal
starting pose, terrain and gameplay scripts remain intact. The observer reports
completed movement, coins, falls, win state and held jump input; Playwright sends
ordinary ArrowRight/Space events using the native goal test's terrain decisions.
It waits for gameplay to observe a jump release before pressing again, so two
input edges cannot collapse into one browser frame.

The goal gate requires at least three coins, no falls and the flag within a
90-second wall-clock budget. It also retains the smoke checks for fetched assets,
WebGPU, rendered pixels and script/GPU errors, and captures the win screen.
Local repeated Chromium runs reached the flag with five coins and eight jumps;
no game-state writes or browser-only gameplay rules were added.

## Remaining limitations

The 2D controller slice is exercised in the engine, native editor, typed Decay
hosts and the platformer on native and browser targets. Gravity, coyote time,
jump buffering and player input remain Decay policy. Compound solid probes,
same-step solver response, swept trigger events and curved/rotating-probe carry
remain explicit gaps in parity. The 3D scene controller below is a foundation;
Decay access, platform carry and game proof remain pending.

## Foundation evidence

Native tests cover all probe shapes in empty space, zero motion, wall tangents,
skin, escape/approach at touching surfaces, initial overlap, bounded corner
iterations, rotated surfaces, filter/predicate behavior, immediate teleports and
removal, deterministic ties, one-way geometry's two-sided query behavior and
invalid input rejection. These geometric tests complement the typed host and
platformer gameplay and native editor evidence above.

Ground-probe tests cover separation/touching and zero travel, box/circle/rotated
capsule extents, slope boundaries and steep obstruction, arbitrary up, ceiling
orientation, initial penetration, masks/sensors/whole-entity exclusion, host
predicates, current poses/removal, deterministic ties and invalid options.

Grounded movement tests cover contact-only support, snapping on/off and distance
limits, leaving a ledge, landing and repeated grounded movement, jump and blocked
ceiling requests, descending walkable slopes, steep rejection, initial overlap,
arbitrary up with a rotated capsule, shared filters/predicates, immediate pose
changes/removal, budget exhaustion, read-only results and invalid/overflowing
inputs. Zero-travel support tests bound the numerical skin allowance.

Slope movement tests cover walkable and steep ascent, the configured angle
boundary and changed limits, explicit jump rise, negative vertical requests,
steep descent and walkable downhill snap, walls/ceilings, mirrored capsule and
rotated box/up, filtering, read-only state and unchanged geometric sliding.

Step tests cover opt-in low obstacles, the height boundary, tall walls, ceilings
and forward overhangs, airborne/jump/overlap rejection, no or steep landing,
box/circle/capsule extents, rotated up/probe, filters/predicates, exact fallback,
read-only state, motion accounting and invalid heights. A repeated flat-floor
regression covers contact-normal refinement for all three probe shapes.

Platform tests cover translation for all probe shapes, upward/downward support,
rotation-point displacement, snapshot advancement, wall clipping and ceiling
crush reporting, jumps/ledges, filters/sensors/host predicates/removal, stale or
steep support, compound offsets/rotation, carry plus step accounting, invalid
input and actual position/velocity-kinematic simulation ordering.

## 3D character movement foundation

Added generally for the planned Explorer genre showcase in PR #507. It is an
engine primitive with the scene ownership described below, not yet a Decay API
or completed game feature.
It uses Rapier's kinematic character controller behind Sindri-owned types;
2D movement and its platformer contract remain unchanged.

`PhysicsWorld3d::move_character(shape, pose, displacement, options, filter)` and
`move_character_where(..., include)` return a copied `CharacterMotion3d` without
changing any body pose, velocity, event state or scene. Shapes are the existing
box, sphere and Y-axis capsule probes (including a zero-height query capsule).
The world-space displacement is not velocity; probe orientation stays fixed.
The predicate is stable for the call and may run repeatedly across movement,
step and support probes. Membership masks, sensor policy and whole-entity
exclusion use the existing `RaycastFilter3d` contract, independently of the
obstacle's physical pair filter. Exact ties and traversal order remain
backend-owned, unlike the deterministic ordinary Sindri query tie policy.

The existing query-only per-piece BVH supplies Rapier's query pipeline. Its
collider world poses and bounds update at insertion/teleport, after solving and
when static groups change, so movement sees the same current geometry as rays
without waiting for a physics step or rebuilding a second spatial index.

`CharacterOptions3d` uses absolute world-unit distances: positive `skin`
(default 0.01), optional `snap_distance`/`step_height` (both default zero),
`step_min_width` (0.1) and opt-in `step_dynamic_bodies` (false). It also carries
`slide` (true), unit world-space `up` ([0, 1, 0]), `max_slope_angle` and
`min_slide_angle` (both pi/4 radians). Angles accept 0..=pi/2; distances must be
finite/non-negative and skin positive. Up must be within 0.0001 of unit norm
and normalizes before use. Scene integration must decide when to enable steps
and snap; gravity, jumps and acceleration belong in game scripts.

The result contains total `translation`, `grounded`, `sliding_down_slope` and
ordered movement-phase `CharacterCollision3d` values. A collision copies a
`ShapeHit3d` (entity, world point/normal, distance from that sweep's start), plus
`translation_applied` and `translation_remaining` at impact. Internal stair,
snap and support probes do not populate that collision list. Editing a copied
result cannot affect later movement.

Backend limits are explicit: `grounded` means Rapier found a nearby upward-facing
contact, not that it passed the configured walkable slope test. Prediction
includes skin plus 0.05 world units. `sliding_down_slope` preserves Rapier's flag,
which may also be set during uphill slope handling. Rapier's movement loop has
its own fixed 20-iteration budget, without a public exhaustion flag. Approximate
curved shape-cast normals can
introduce small normal nudges or lateral travel loss, including on flat faces;
native slope/capsule checks use explicit 0.01–0.02 world-unit tolerances.
Stationary near-zero requests can propose bounded depenetration (four passes,
total at most one quarter of the backend's local-bounds up extent); moving
penetration uses Rapier's shape-cast policy.
There is no Sindri penetration flag or recovery policy yet. Sensor-inclusive
queries can block movement, but Rapier's stationary correction ignores sensors.
Steps/snap and these flags need scene/game policy before they become gameplay
contracts. This foundation supplies no push impulses, swept trigger events or
rotating probe casts.

A displacement query passes zero timestep to the backend, disabling automatic
velocity-based platform friction/carry. Future scene ownership must add solved
translation/rotation-point carry exactly once and invalidate support when its
owner rebuilds, teleports or disappears. No moving-platform feature is claimed.

Shape, pose, settings, displacement, destination and expanded probe bounds
validate before queries. Non-finite copied backend results fail explicitly.
If any obstacle has overflowing spatial bounds, this controller query fails
instead of silently dropping the ordinary index's conservative fallback pieces.
Ordinary Sindri queries retain their existing overflow fallback behavior.

Native regressions cover free XYZ movement for every shape, wall slide/stop,
solid landing and stationary support, snap on/off and upward suppression,
low/tall steps and headroom, slope limits, rotated capsules/arbitrary up,
mask/sensor/exclusion/predicate filtering, immediate offset teleports and handle
reuse, copied/read-only state, cloned solve replay, no implicit platform carry,
bounded stationary correction and invalid/extreme input. All 206 physics tests,
26 existing 3D scene regressions, warning-denied checks/Clippy and Rust 1.95 WASM
checks/build pass. The rebuilt Orbit Lab passes its real WebGPU Chromium
capture/denial/goal regression with an inspected screenshot, checking existing
scene behavior after pose synchronization changed. The new movement query is
not yet exercised in a browser. Decay, editor interaction, moving-platform and
Explorer goal proof remain pending.

## Classified 3D ground queries

`PhysicsWorld3d::probe_ground(shape, pose, options, filter)` and
`probe_ground_where(..., include)` return copied `GroundProbe3d` values without
moving bodies, snapping, changing velocity or choosing gameplay support.
This is a general prerequisite for Explorer's moving platforms; scene carry and
Decay access remain pending.

`GroundOptions3d` defaults to world up [0, 1, 0], slope limit pi/4, travel 0.1
and skin 0.01. Up uses the movement query's squared-unit-norm tolerance of
0.0001 and normalizes before use. Slope accepts 0..=pi/2, travel must be finite
and non-negative, and skin finite and positive. The fixed-orientation box,
sphere or capsule travels downward to the skin, rather than physical contact.
Zero travel can classify touching or existing skin contacts, allowing numerical
slack of one percent of skin plus `f32::EPSILON`.

The hit copies obstacle entity, world point/normal and travel distance. The
nearest surface is retained even when too steep; the query does not look through
it for a walkable floor. Exact distance ties use entity-handle order, then piece
order within an entity. A normal must face up and its normalized alignment plus
1e-6 must meet the cosine of the slope limit. Horizontal walls and ceilings
remain unwalkable even at pi/2. Any initial contact deeper than `f32::EPSILON`
blocks support: `started_penetrating` is true and the first penetrating entity
in stable traversal supplies a zero-distance, zero-normal hit at the probe
origin. This flag is independent of the movement wrapper's raw Rapier flags.

The same current-pose per-piece index, masks, sensor policy and whole-entity
exclusion apply. The optional predicate runs once per spatial candidate entity;
all its pieces reuse that answer. Invalid shapes, poses, settings, overflowing
probe bounds/extents or nonfinite selected hit geometry fail explicitly. Any
unbounded obstacle fails the query even when excluded, matching the movement
wrapper's conservative validation rather than ordinary queries' fallback.

Native regressions cover every probe shape, skin and touching contacts, travel,
steep nearest support, penetration, arbitrary up, rotated compound offsets,
filters, predicate reuse, ties, immediate teleports/removal, copies/clone replay
and invalid/extreme input. They distinguish Rapier's broad grounded prediction
from classified support and verify support at a movement query's landed endpoint.
Browser, editor, Decay and Explorer proof of this new primitive remain pending.

## 3D scene ownership

`sindri.physics3d.character` authors the flat `CharacterOptions3d` fields above.
Defaults and edited payloads use the same validation. A controller requires a
transform and Collider 3D with exactly one solid box/sphere/capsule; extra sensors
are allowed. It uses the composed-scale solid piece, including its local offset,
rotation and filter mask, excludes its whole entity and ignores inactive obstacles.
Authored rigid bodies, mixed 2D/3D ownership and moving Z locks are rejected.
The scene derives a stationary velocity-kinematic body. Gameplay still supplies
all displacement, including gravity and jumps; solver velocities are zeroed before
solving so they cannot become a second movement owner.

`ScenePhysics3d::character_requests()` returns `CharacterRequests3d`.
`move_character(entity, displacement, snap)` queues world-space XYZ travel for the
next fixed pass. The last valid request wins; nonfinite vectors and overflowing
lengths are rejected before replacing input. Missing/inactive/non-controller
requests are discarded at synchronization. Pause retains input; an invalid
step or authored batch consumes none. A body rebuild or teleport preserves valid
same-step input. No-request frames query zero displacement with snap disabled:
gameplay must explicitly permit snap each step. Raw backend grounded status does
not automatically authorize snapping or become a walkable-support decision.

Controller settings and scaled geometry validate beside the ordinary body batch
before lifecycle changes. Voxel residency includes each character's geometry,
requested travel and skin/snap/step margins even without dynamic bodies. An
oversized residency window fails before any body changes. Characters are held
stationary during the solve, then queried against current solved poses in entity
order. Each successful result applies once; parent-space writeback retains
rotation and scale. Queries see the applied pose immediately. Physical response
and discrete sensor events observe it at the following solve, with no push
impulses or swept trigger events. Stationary bounded correction and all backend
flag limits from the query contract still apply.

`character_motion(entity)` returns the last applied `CharacterMotion3d`.
`for_scripts_with_characters()` supplies disjoint solver/event/request/result
borrows with `CharacterMotions3d::get(entity)`. Results are immutable snapshots;
activity must still be checked while reading during a script pass. Settings,
parenting, scaled geometry, body rebuilds and teleports invalidate caches during
synchronization; removal/inactivity discard them. Cloning the driver or a Session
checkpoint includes pending input and cached results, without changing saved
component payloads. `Session::character_requests3d()` exposes the same queue to
Rust hosts; Decay access is scheduled separately.

As with backend solver failures, a movement-query error after solving is not a
transactional rollback of the fixed pass. Requests already applied successfully
are consumed; a failed request and later requests remain queued. Hosts must report
the error rather than treating an incomplete pass as a successful gameplay frame.

Native scene regressions cover request replacement, invalid input/batch/step,
cloned replay, no solver gravity or velocity, masks/sensors, scaled offset probes
under rotated parents, lifecycle/rebuilds, current solved platform poses,
immediate query visibility and far-away voxel residency/budget rejection. A
shared Session regression verifies once-only movement and checkpoint replay.
This remains partial Explorer foundation evidence: platform carry, Decay,
controller browser verification, editor interaction and game adoption are pending.
