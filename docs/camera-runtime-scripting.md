# Runtime camera control from Decay

Sindri owns the authored camera behavior and performs follow movement, dead-zone handling, smoothing, confinement, and shake. Decay controls that behavior at runtime rather than reimplementing camera math in gameplay scripts.

The behavior calls below operate on the single authored `sindri.camera` that carries `sindri.camera.behavior`. If there is no such camera, or more than one, the call is a runtime error rather than silently choosing one.

| Call | Returns |
| --- | --- |
| `Camera.impact(amount)` | unit |
| `Camera.follow(target)` | unit |
| `Camera.clear_follow()` | unit |
| `Camera.follow_offset(x, y, z)` | unit |
| `Camera.dead_zone(x, y)` | unit |
| `Camera.smoothing(value)` | unit |
| `Camera.max_speed(value)` | unit |
| `Camera.bounds(min_x, min_y, max_x, max_y)` | unit |
| `Camera.clear_bounds()` | unit |
| `Camera.shake(strength, frequency, decay)` | unit |

`Camera.follow(target)` changes the entity followed by the engine-owned follow behavior. The target is a typed `Entity` reference, not an authored-name string. `Camera.clear_follow()` disables follow without moving the camera back to an authored target.

`Camera.follow_offset(x, y, z)` changes the world-space offset from the target. `Camera.dead_zone(x, y)` changes the rectangular dead zone. `Camera.smoothing(value)` changes follow smoothing, and `Camera.max_speed(value)` changes the movement speed cap. These mutate the same behavior component the camera system reads each frame, so there is no separate script-side camera state.

`Camera.bounds(min_x, min_y, max_x, max_y)` changes the confinement rectangle and `Camera.clear_bounds()` removes confinement. Follow still runs before confinement, and shake still runs after confinement.

`Camera.shake(strength, frequency, decay)` changes the authored shake behavior. `Camera.add_trauma(amount)`, documented in the main scripting contract, remains the event-style operation for triggering that shake: each call adds, up to 1. `Camera.impact(amount)` raises trauma to at least `amount` instead, so a smaller hit while a bigger one is still shaking changes nothing. Use it where many hits can land in one frame — a spray of bullets, a chain of arcs — and adding would pin the camera at its hardest shake.

The editor-authored values are the initial state. Runtime calls change the live camera behavior for the rest of play unless another script changes them again. Stopping play restores the authored scene in the normal editor lifecycle.

## Projection size

| Call | Returns |
| --- | --- |
| `Camera.orthographic_size(camera, size)` | unit |

`Camera.orthographic_size(camera: Entity, size: f32)` changes the selected orthographic camera's live `vertical_size`. A positive finite size is required; a missing camera or perspective projection produces a runtime error without changing it. The existing `fit`, clipping planes and unknown payload fields are preserved. This projection call does not require a behavior component and selects an explicit entity, so a game can prepare an inactive camera without ambiguity.

Low Tide eases the size in Decay between driving, walking the deck and following the crew ashore. Portrait framing still leaves room for both crawler treads while aboard. The scene extraction path consumes the same changed camera payload on native and web.


## Perspective projection and 3D orbit

These calls select an explicit camera entity and do not require the singular
2D behavior camera:

| Call | Returns |
| --- | --- |
| `Camera.perspective_fov(camera, degrees)` | unit |
| `Camera.orbit(camera, target, yaw, pitch, distance)` | unit |
| `Camera.orbit_offset(camera, offset: Vec3)` | unit |
| `Camera.orbit_smoothing(camera, rate)` | unit |
| `Camera.orbit_collision(camera, mask, padding)` | unit |
| `Camera.clear_orbit(camera)` | unit |

FOV is vertical, in degrees, strictly between 0 and 180. The explicit camera
must use perspective projection. Clipping planes and unknown payload fields
are preserved. For direct aiming, use `camera.transform.look_at(world_point)`;
the transform is the camera's orientation, so there is no separate look-at state.

`orbit` creates or updates `sindri.camera.orbit`. The target must be an active
spatial entity with an authored scene ID, distinct from and outside the
camera's subtree. Settings are world-space: zero yaw puts the camera on the
focus's +Z side; positive yaw moves toward +X; positive pitch raises it.
Angles are radians and pitch is strictly between -pi/2 and pi/2. Distance must
be positive. Offset is added to the target's composed world position; neither
its rotation nor scale changes the offset or distance. Retargeting or changing
angles keeps the existing offset, smoothing, collision mask and padding.

The shared `Session` advances orbit after gameplay and animation. Position
smoothing is exponential per second; zero snaps. The sight line from the
focus to the smoothed camera is cast against synchronized 3D colliders. A hit
pulls the camera in immediately to `hit distance - padding`, clamped to zero;
clearing the obstruction lets smoothing ease it back out. The camera faces
the focus. When a solid encloses the focus, pull-in reaches zero and retains
the computed sight-line orientation. The ray excludes sensors,
the target, the camera's own geometry and inactive entities. Mask is a whole
u32 matching collider memberships; zero disables collision. Padding is
non-negative and strictly below the requested orbit distance.

Camera pose conversion respects parent transforms, preserves local scale and
refuses a singular parent or a move off a Z-locked layer. Invalid runtime calls
leave settings unchanged; an invalid authored orbit reports a step problem and
leaves the camera pose unchanged. An orbit component owns the pose and suspends
any 2D follow, confinement or shake on the same camera. `clear_orbit` removes
that component while keeping the last pose, and the 2D behavior resumes.

The ray protects the center sight line, not the near-plane volume. Target
transforms include this step's gameplay writes; obstacle collider poses are
from the physics phase before scripts, so a scripted collider teleport is
synchronized on the next fixed step. Mouse delta/lock and a camera-volume sweep
remain separate capabilities.

[Orbit Camera Lab](../examples/orbit/README.md) opens and runs to its goal
through the shared runtime while exercising FOV, pull-in, recovery and aiming.
It is feature evidence; Explorer's gameplay, editor and browser proof remain
pending.

Orbit Camera Lab also demonstrates mouse look: `Input.Pointer.delta` is a
read-only `Vec2` displacement in viewport pixels for this fixed step. Its Decay
script applies sensitivity without `dt`, ignores UI captures and clamps pitch.
Pointer lock remains a separate dependency; dragging cannot cross the window
edge yet. Touch games should use the existing `Gesture` drag surface.
