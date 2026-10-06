# Character movement

Status: geometric sweep/slide, ground probing, grounded snapping, slope
limits, optional steps, synchronized platform carry, one-way controller policy
and scene runtime ownership implemented; acceptance remains open in
`physics-update.md`. Added for the platformer genre showcase, which has not yet
adopted it. No game proof is claimed for this slice.

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
order. Inserts, teleports and removals are visible before a physics step because
this foundation scans pieces at their current body-relative poses.

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

This is a general geometric policy added for the platformer, which has not yet
adopted the controller. It does not make the engine/editor/Decay/game acceptance
complete. One-way controller support and vertical integration remain open.

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
platformer adoption and has engine evidence only.

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
explicit parity gaps. This is engine evidence for platformer adoption, not yet
scene/editor/Decay or game proof.

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
is added generally for platformer adoption; its dynamic-body hero has not yet
been migrated and full game acceptance remains open.

## Remaining slices

Checked editor authoring/undo and a real platformer scripted run must prove
the capability vertically. Typed requests/results and shared host context are
implemented; editor Play interaction and game/browser adoption remain open.
The platformer's
current dynamic-body hero is retained until that integration is ready; coyote
time, jump buffering and player input remain gameplay policy in Decay.
Native and real browser proof are required before checking character acceptance.

## Foundation evidence

Native tests cover all probe shapes in empty space, zero motion, wall tangents,
skin, escape/approach at touching surfaces, initial overlap, bounded corner
iterations, rotated surfaces, filter/predicate behavior, immediate teleports and
removal, deterministic ties, one-way geometry's two-sided query behavior and
invalid input rejection. This is engine evidence only; Editor, Decay and game
proof remain absent.

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
