# Runtime camera control from Decay

Sindri owns the authored camera behavior and performs follow movement, dead-zone handling, smoothing, confinement, and shake. Decay controls that behavior at runtime rather than reimplementing camera math in gameplay scripts.

These calls operate on the single authored `sindri.camera` that carries `sindri.camera.behavior`. If there is no such camera, or more than one, the call is a runtime error rather than silently choosing one.

| Call | Returns |
| --- | --- |
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

`Camera.shake(strength, frequency, decay)` changes the authored shake behavior. `Camera.add_trauma(amount)`, documented in the main scripting contract, remains the event-style operation for triggering that shake.

The editor-authored values are the initial state. Runtime calls change the live camera behavior for the rest of play unless another script changes them again. Stopping play restores the authored scene in the normal editor lifecycle.
