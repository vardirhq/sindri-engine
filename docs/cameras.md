# Camera semantics

Sindri has two different camera concepts and deliberately keeps them separate.

## Authored cameras

A `sindri.camera` component is a world/game camera. It is an ordinary scene entity and gets its pose from the entity's `Transform3D`:

- `Transform3D.position` is the camera position.
- `Transform3D.rotation` is the camera orientation.
- local `-Z` is forward.
- local `+Y` is up.
- scale is still part of the normal transform, but does not change projection.

Perspective and orthographic are projection choices for the same world-camera concept. Perspective projection uses vertical FOV, near and far planes. Orthographic projection uses vertical size, near and far planes. Orthographic cameras are not a special UI or overlay role.

The current renderer supports exactly one authored world camera for a game frame. A scene with world-rendered content and no authored camera reports `MissingWorldCamera`. A scene with more than one authored world camera reports `MultipleWorldCameras`. Sindri does not choose a winner from entity iteration order.

Supporting camera stacks, render targets, split-screen, or explicit camera priority later should be an authored feature with its own ordering/composition contract. It must not be introduced by making entity order significant.

## Gameplay camera behavior

Projection and gameplay behavior are separate. A camera may carry
`sindri.camera.behavior` beside `sindri.camera` to follow a target, stay inside
world bounds, and respond to impact shake without making those policies part of
the renderer.

Follow has an offset, a rectangular dead zone, exponential smoothing, and an
optional maximum speed. Confinement clamps the resulting XY camera position to
a world-space rectangle. Shake is transient trauma: amplitude is quadratic in
trauma, its phase is deterministic across native and web, and trauma decays each
update. Behavior order is follow, then confinement, then shake, so impact motion
may briefly cross a confine edge without changing the camera's underlying
follow position.

The behavior update is an engine operation rather than a render-extraction side
effect. A host advances it once per gameplay frame with
`update_camera_behaviors`; extracting the same world twice never advances or
changes camera state.

Every host that runs gameplay advances it after the scripts, so the camera
follows where that step left its target: the editor's Play, the browser host
that runs exported games, the camera example and the platformer's test harness.

### Camera Lab acceptance surface

The dedicated demo lives in `examples/camera`, now an authored project with
a `sindri.toml`, packaged font and responsive Weave presentation, exported at
`/examples/camera/` and listed in the Pages directory. Its native entry point
uses the same scene and Decay scripts.

The cyan camera-relative rectangle visualizes the dead zone; amber boundary
lines and corner markers visualize camera-center confinement. The outer gray
frame marks the target's movement area. Follow, confinement, shake, smoothing
and dead zone have independent on-screen and keyboard controls. A touch movement
pad, impact button, automatic tour and reset make the behavior explorable
without a keyboard. The interface shows settings and sampled coordinates.

Decay changes the real behavior through the existing runtime Camera API
([runtime control contract](camera-runtime-scripting.md)); it does not implement
a second follow or shake algorithm. Rust advances scripts and then
`update_camera_behaviors`. Turning follow back on restores the demo's selected
dead zone, smoothing and speed cap. Reset restores modes and target position,
then lets normal follow settle the view rather than overwriting transient shake.

Native regressions exercise follow, confinement, independent modes, restoring
settings, trauma decay, phone touch input and the tour. Browser smoke exercises
the visible controls and captures desktop and phone output, with both Pages
project-subpath and custom-domain exports.

Runtime mode changes are already available through `Camera.follow`,
`clear_follow`, `follow_offset`, `dead_zone`, `smoothing`, `max_speed`,
`bounds`, `clear_bounds` and `shake`. Orbital uses `Camera.impact` for
combat shake. Dedicated editor behavior controls and gizmos remain follow-up
work.

## Screen-space UI and overlays

The `sindri.ui.*` family does not require a camera entity. Their projection is owned by the viewport and their anchors resolve against the viewport's screen-space extent.

This is intentionally similar in principle to a screen-space-overlay UI model: UI can exist without a second scene camera. An orthographic projection matrix may still be used internally to turn screen coordinates into clip space; that projection math is not an authored camera.

## Editor Scene camera

The editor Scene view has its own editor camera. It is not stored as a `sindri.camera` component and does not modify authored camera transforms or projection settings.

The Scene camera is used consistently for Scene rendering, picking, gizmos, tile painting, focus, and the axis indicator. The Game view continues to render through the authored world camera.

**UI is the exception, and it is not an exception to the rule so much as an application of it.** A UI element is laid out against the viewport rather than in the world: an anchor picks a point and the element's transform is an offset from it, so no camera — Scene or authored — has anything to say about where it is. Picking one and drawing its gizmo therefore go through the overlay, handed out by `overlay_for_viewport`, which needs nothing but the viewport's aspect ratio. The rule is the same one as everywhere else here: whatever put the picture on the screen is what a pointer has to be inverted through, and a matrix rebuilt somewhere else is a second answer that only has to disagree once.

Authored cameras are visible and selectable objects in Scene view, including their direction and projection volume, but those editor-only visuals never appear in Game/runtime rendering.

Two rules keep that picture honest. **A frustum is drawn at the aspect the camera renders at**, which is the Game viewport's and not the Scene view's — otherwise dragging the divider beside the Scene view reshapes a frustum that has not changed, which is a picture claiming the camera frames more of the world because a panel got wider. And **every line is clipped against the near plane in clip space, before the perspective divide**: a point behind the eye has no projection, and dividing by its negative `w` puts it somewhere plausible on the opposite side of the screen, which is what made a frustum smear across the viewport while orbiting past it.

The projection volume is drawn for the **selected** camera only. An unselected one is its marker and a short forward stub, because a frustum reaches to the far plane and several of them crossing the viewport say nothing about the one being worked on. The marker and the stub are also the whole of what a click selects: a far-plane edge a hundred units away is not what someone means when they click on it.

## Scene format 7

Format 6 used orthographic `sindri.camera` components for the old screen-overlay implementation. Format 7 removes that role.

The built-in 6 -> 7 migration therefore removes format-6 orthographic camera components while preserving their entities and unrelated data. Perspective cameras survive unchanged. From format 7 onward, an authored orthographic camera is a normal transform-driven world camera.

The earlier 5 -> 6 migration remains part of the chain: old perspective `target`/`up` look-at data is converted to the equivalent `Transform3D.rotation`. `target` and `up` are not part of the current camera model and must not be reintroduced.
