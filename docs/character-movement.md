# Character movement

Status: geometric sweep/slide and ground-probe foundations implemented; full character movement
acceptance remains open in `physics-update.md`. Added for the platformer genre
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
query, using one f32 epsilon of prediction allowance at the skin boundary,
precedes the sweep. Blocking contacts at zero travel tie by entity then original
piece order. A surface tangent to the probe direction is not support. No hit
means unwalkable, not penetrating.

This API does not snap, choose jump behavior, carry platforms, limit uphill
movement or implement one-way support/drop-through. Grounded-state and motion
policy must be added in the following controller slices; one-way geometry still
has the ordinary two-sided geometric query semantics here.

## Remaining slices

The foundation sees one-way geometry on both sides, just like geometric queries.
The full controller must incorporate support-side policy and timed drop-through,
not interpret this primitive as a finished one-way movement API.

Remaining engine work includes uphill/downhill movement limits, grounded-state
and snap behavior, step height and clearance,
ceilings and moving-platform displacement. Platform riding must use synchronized
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
