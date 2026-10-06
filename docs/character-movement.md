# Character movement

Status: geometric sweep/slide, ground probing, grounded snapping and slope
limits implemented; full character movement acceptance remains open in
`physics-update.md`. Added for the platformer genre
showcase, which has not yet adopted it. No game proof is claimed for this slice.

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
surfaces use world normals; narrow-phase normal refinement can require more
than one hit on the same surface. Sub-epsilon remaining movement is discarded.

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
non-negative finite `snap_distance`. Slide skin is also the support skin, so
movement and grounding cannot disagree about that separation. Up and angle use
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
`slide` retains the slope-limited sweep result, including its unsatisfied
movement and budget flags. `snap_translation` is the additional downward motion;
`ground` is the support query at the slide endpoint before snapping, so its hit
distance describes that extra travel. `grounded` means the proposed endpoint
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
complete. One-way support, steps and moving-platform behavior remain open.

## Remaining slices

The foundation sees one-way geometry on both sides, just like geometric queries.
The full controller must incorporate support-side policy and timed drop-through,
not interpret this primitive as a finished one-way movement API.

Remaining engine work includes step height and clearance, ceiling/step
interactions and moving-platform displacement. Platform riding must use synchronized
poses and avoid counting platform movement twice. Tests must exercise the
one-way/drop-through interactions and initial-overlap/budget outcomes together.

Scene authoring, checked editor commands/undo, typed Decay access and a real
platformer scripted run then prove the capability vertically. The platformer's
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
