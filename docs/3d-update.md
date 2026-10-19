# 3D foundation and Milestone 8 follow-ups

PR #507 merged the shared 3D foundation, including the Day/Night Lab proof.
The checked items below record that foundation and completed follow-ups.
Unchecked work is split into separate PRs in dependency order. Milestone 8 itself
remains incomplete until the game acceptance is met. Runtime, editor, scripting,
proof and documentation continue together in each follow-up.

The foundation boundary is shared model loading/export/rendering and model
collision, 3D transforms/cameras/input/characters, scoped native measurements,
shared material factors/normals/imported shadows, point/spot lights, typed runtime
light/environment controls and the day/night/mounted-GLB feature example. See
[`3d-foundation-review.md`](3d-foundation-review.md) for review and validation.
Follow-ups cover the remaining material/alpha contract, batching/culling/cache
work, advanced editor authoring, fog/post delivery, terrain/animation and the
in-tree playable game. The checkboxes below retain the full Milestone 8 plan.

## Original Milestone 8 gap assessment

The following assessment records the starting point of the full milestone;
the checked slices below describe what the foundation now delivers.

The exit gate of Milestone 8 is: *imported glTF content, camera, lighting and
collision work in native, browser and editor preview.* PR #504 brought static
GLB decoding, a model GPU path and project export; slice 1 merges it. Read against main
(October 2026, at `7d4a844`) plus #504, these gaps remain:

1. **The 3D renderer is one draw per pass.** In `TexturedCubeRenderer`, every
   3D draw opens its own render pass and its own uniform slot, and nothing is
   instanced. Inline `sindri.mesh` surfaces are uploaded again every frame,
   and uploaded twice when shadows are on. Only voxel sections and tile
   volumes are frustum-culled. Textures have no mipmaps. A scene of a few
   hundred props will be draw-bound long before it is GPU-bound.
2. **There are two lighting models.** Cubes, meshes and voxels use flat
   derivative normals with Lambert lighting. #504's models use real normals
   with GGX. Models now cast/receive shadows and accept renderer fog.
   Cutout geometry casts solid shadows. The only light is one sun: there are
   no point or spot lights or transparent 3D pass. Numeric standard factors and
   common GGX are opt-in; shared material assets and legacy migration remain.
3. **The editor cannot author a 3D scene from models.**
   - `.glb` files are only an icon in the project browser: no loader, no
     thumbnail, no drag-in (only prefabs drop, and onto Z=0).
   - Picking hits only `Cube`. Surface meshes and models cannot be clicked.
   - There is no ground grid. F pans but does not frame by bounds, and the
     unused `SceneCamera` has no fly mode.
   - Create offers no primitive, camera or point/spot light. 3D colliders
     have no handles.
4. **Gameplay cannot move anything through 3D.**
   - Decay's transform surface is `position`/`scale`/`rotation_z`: there is
     no 3D rotation, forward, or look-at.
   - Camera controls have no FOV, look-at or orbit.
   - There is no pointer delta or pointer lock.
   - There is no 3D character controller, 3D joints, 3D material profiles or
     CCD.
   - Collision cannot come from a model: there are no trimesh or convex
     colliders, and collider size ignores scale.
5. **There is no smooth ground.** The only generated world is
   `sindri-voxel`'s block terrain. A 3D game built from smooth models has
   no heightmap terrain to stand them on, and no way to scatter trees or
   rocks across one.
6. **Nothing proves it.** No in-tree project moves a character through a 3D
   level with a perspective camera. The low-tide crawler proof is external,
   and AGENTS.md does not count it.

## Follow-up order

The foundation stops after Day/Night Lab. No unchecked feature below is a
merge gate for PR #507. Continue with separate, reviewable PRs in this order:

1. Retained inline geometry/u32 feature proof meets the zero-settled-upload
   target (slices 2 and 3; [contract](retained-meshes.md)). Explicit material face
   visibility has native/editor/WebGPU feature proof. Reusable numeric material
   profiles have native/editor/WebGPU feature proof; texture bindings and unified
   alpha remain separate work (slice 3), alongside remaining measurement coverage.
2. Batch and instance repeated geometry, and cull meshes and models against
   cached bounds (slice 3).
3. Finish shadow quality, mipmaps, transparency and authored fog/post delivery,
   then expose their runtime controls (slices 3 and 4b).
4. Finish model placement/picking, Scene-view navigation and collider authoring
   (slice 5); extend native-window feature proof where still unchecked.
5. Add smooth terrain and foliage after rendering and placement dependencies
   (slice 5b), and skeletal animation (slice 6b).
6. Build Explorer, adopt shared capabilities in real gameplay, add the Causeway
   prop, and meet measured browser/editor performance targets (slices 6 and 7).
   Only then reassess the Milestone 8 exit gate and final integration (slice 8).

For each PR, read `AGENTS.md`, keep runtime/editor/Decay/proof documentation
together, and check acceptance only after its tests and relevant surface proof
pass. Existing checked subitems remain evidence even when their parent is
unchecked because it still requires game adoption or broader implementation.
The yaw/pitch/roll convention remains radians, Y-up, -Z forward and YXZ order.

## Slices

### 1. Land #504 on the shared runtime
- [x] Merge `feat/imported-glb-models` onto main and resolve it against
  `sindri-runtime`/`sindri-player`:
  - `ProjectAssets.models`;
  - the `.glb` scan in `directory.rs`;
  - the model queue in `browser.rs`;
  - preparation and binding in `Player::install`.
  `project-capture` binds models in `game/src/project` (where main moved it)
  and draws with the authored lighting. `assets_root` serves projects that
  keep their assets at the root, as Low Tide does. The Low Tide crawler
  renders through the merged native capture.
- [x] Report model decode warnings: host logs, and editor console lines.
- [x] The editor loads, binds and hot-reloads models for both views, and
  draws around one that is loading or broken.
- [x] Prove that `sindri-player <project>` plays a model scene natively in a
  window. Model Collision Lab was reviewed on X11/software Vulkan: imported
  wall, keyboard obstruction and walker reaching its goal pad.

### 2. Measure 3D
- [x] Fixed benchmark project: 100 cubes, 100 inline pyramid surfaces and 100
  references to one GLB, a perspective camera and authored directional shadows.
  Shared player/capture paths now apply shadow settings; native extraction checks
  counts/resource sharing and Session ready, and real browser smoke draws it.
  See [3D Benchmark](../examples/benchmark3d/README.md).
- [x] Standalone benchmark records physical textured 3D/model draw/pass counts,
  uniform writes/bytes, transient geometry pairs and persistent cache events.
  Consecutive shadowed/unshadowed frames verify deltas; the summary prints means.
  These are scoped 3D counters, excluding 2D, post and initialization uploads.
- [x] Record these counters in editor benchmark frames, separating the primary
  view (Game in Play, Scene while editing) from the extra Scene view beside Play.
  Native shadowed/unshadowed editor runs verify matching Game/standalone counts.
- [x] Native standalone reports optionally measure a bracketed GPU frame
  interval, with explicit unsupported/null results, adapter/period metadata and
  separate summary means/p95. Software-Vulkan CI requires real timing and query
  reuse; baseline devices still work. CPU completion wait remains separate.
- [x] Editor benchmarks use optional GPU timestamps for each viewport, record
  primary and extra Scene intervals independently, and keep execution outside
  CPU phases. Native shadow controls and a two-view layout prove delivery.
- [x] Record physical sprite/shape/glyph batch draws, passes, instances and
  buffer-write bytes in standalone/editor reports, with a separate extra Scene
  scope. GPU empty/reuse tests and native Platformer reports exercise the path;
  the fixed 3D fixture records measured zeroes.
- [ ] Finish whole-frame coverage (clears, post, atlas/resource uploads and editor
  UI) and GPU timestamps in browser reports.
- [x] Set and meet the retained-inline target: zero settled geometry uploads,
  with unchanged draws/passes and native/browser appearance. Native and editor
  reports, real u32 browser controls and GPU lifetime tests exercise the boundary.
- [ ] Set batching targets for draws/passes independent of repeated props.

### 3. One lit 3D path
- [ ] A `Material` shared by meshes, cubes, voxels and models:
  - [x] Shared linear numeric factors and GGX shader, authored `sindri.material`
    overrides on cubes, inline meshes, models and voxel-world owners. Checked
    ranges, emission, mask-alpha delivery and absent-override restoration have
    GPU/scene tests. Native/browser swatches and editor edit/Undo/Redo/save/reopen
    review prove the slice; game adoption remains open. See [materials](rendering-materials.md).
  - [x] Preserve and render core GLB emissive factors, including override/removal
    and masked-shadow invariance, with native/browser imported controls. Emissive
    maps and game adoption remain open.
  - [x] Reusable typed render material profiles, shared numeric factors and face
    policy, explicit local overrides, checked editor authoring, asynchronous
    loading and export discovery. Native/WebGPU shared/inline and shared-edit
    controls, plus editor creation, Save/correction, selection/history, reload,
    reopen and Play prove the slice; game adoption remains open. See [profiles](rendering-materials.md#reusable-material-profiles).
  - [ ] Shared material texture bindings, unified alpha and legacy-look migration.
  - [x] Shared inherit/single/double sidedness across cubes, inline/cached surfaces,
    voxel owners and imported primitives. GPU color/shadow, reflection, history
    and residency regressions pass; native/WebGPU panel controls and editor
    edit/Undo/Redo/save/reopen/Play prove the slice.
    See [face visibility](rendering-materials.md#face-visibility). Game adoption remains open.
  - base color (factor and texture);
  - metallic/roughness;
  - emissive (replaces `glow`);
  - alpha mode (opaque, cutout or blend);
  - double-sided.
  - [x] Cube outward face normals and optional authored inline/cached render normals,
    with inverse-transpose shading under nonuniform/reflected transforms and
    orientation-aware world color/shadow culling. Scene validation/extraction/history
    and GPU readback exercise the path; native/browser ellipsoid controls prove
    smooth versus flat shading. Editor authoring review and game adoption remain open.
- [ ] One render pass per stage. Draws are sorted by pipeline, material and
  mesh, and repeated mesh+material pairs are instanced.
- [x] Inline meshes are cached by geometry revision, with u32 indices and no
  settled uploads. Shadow/color draws share buffers; independent uniforms,
  edit/undo/redo, atlas changes, old-frame lifetime and retirement have regressions.
  Native/editor counters and browser controls pass. See [retained meshes](retained-meshes.md);
  deduplication, instancing and in-tree game adoption remain open.
- [ ] Frustum culling of meshes and models at extraction, from cached
  bounds.
- [ ] Models cast and receive shadows, and are fogged.
  - [x] Shared directional shadow reception and renderer fog shading; GPU tests
    exercise resize/disable/cache reuse and all three fog terms. Native/browser
    green-receiver controls prove model shadow reception. This is feature proof;
    Explorer adoption and shipped-host authored fog delivery remain pending.
  - [x] Imported-model shadow casting with cached hierarchy/geometry, independent
    uniforms, glTF alpha masks and mirrored/single/double-sided facing. GPU
    regressions and native/browser imported-only controls exercise it.
  - [ ] World-mesh cutout shadow silhouettes, improved PCF/cascades, shipped-host
    authored fog delivery and integrated game proof.
- [ ] Mipmaps generated on upload for smooth-filtered textures.
- [ ] A sorted, blended transparent 3D pass.

### 4. Lights
- [x] Point-light foundation: strongest-eight shared forward set, world-space
  range falloff across world/imported geometry, composed parent positions,
  editor creation/range guides, strict/tolerant validation and native/browser
  controls. See `3d-lights.md`; game proof and local shadows remain open.
- [x] `sindri.light` gains point and spot lights (range, cone), a bounded
  forward set per frame (for example 8, nearest/most intense first), lit on
  every geometry kind.
- [x] The editor shows range spheres and cone gizmos. Create Point Light and
  Create Spot Light.
- [x] Typed `this.light` / `entity.light` color, intensity, range, cone angles
  and independent enabled state, with atomic validation. Lighting Lab opens,
  compiles and plays to both goals; live native/browser phases and editor
  Play/Stop exercise the controls. This is feature proof, not game adoption.
- [x] Directional lights rotate through the existing 3D transform API.
  Runtime property and transform changes update lighting and shadow direction
  in the next rendered frame in native, browser and editor Play runtimes.
- [x] Point and spot lights can ride moving 3D parents: composed world
  translation and rotation determine position and spotlight direction.
  Decay can change their properties, and imported GLB geometry is lit through
  the unified renderer in native, browser and editor Play. Day/Night Lab
  exercises a moving GLB carrier; frozen-hour controls isolate dynamic sun
  shadows on imported ground. See `daynight.md`.

### 4b. Runtime environment controls and day/night proof

The day/night example follows its runtime dependencies below.
Implementation order within the lighting track is unified 3D materials and
lighting (3), imported-model shadow casting/receiving (3), runtime Decay light
access (4), point/spot lights (4), runtime environment access, then the example
and integration tests below. Runtime light access may land before local lights
without changing the current slice order.

Low Tide 3D (`vardirhq/low-tide-3d`) is the initial external integration case
that requests these general capabilities. It stays a consumer, never an engine
dependency. Time progression, sunrise/sunset transitions and gameplay
consequences belong to game Decay scripts. There is no engine day/night API,
second lighting path or Low Tide gameplay in Rust.

- [x] Typed scene environment ambient intensity/color and background/clear
  color, with default-aware reads and atomic checked whole/channel writes.
  Lighting Lab exercises the live cool/warm phases through Session and native,
  browser and editor Play; see `runtime-environment.md`.
- [ ] Expose fog color, density and distance once shipped-host fog delivery is
  unified; imported-model fog shading has GPU proof, but project hosts still
  omit authored fog.
- [x] Apply ambient/background edits on the next extraction without restarting
  or rebuilding render resources, using the authored environment and existing
  shared lighting/clear path. Fog/post controls remain separate.
- [x] Add a small, self-contained feature example with a configurable Decay
  game clock, sunrise, midday, sunset and night; a rotating directional sun;
  smooth sun brightness/color, ambient and background transitions; and night
  illumination while the sun is disabled. No visible-sun renderer is required.
- [x] The example moves an imported GLB object carrying a functional point
  or spot light. Imported models cast and receive dynamic shadows that follow
  the sun. This proves generic mounted lights rather than crawler-specific
  headlights, cabin/work lights or floodlight logic.
- [x] Test typed property reads/writes, invalid values and atomic rejection
  (in the light/environment slices), runtime frame updates, parenting and
  shadow direction.
  Open the example, compile its scripts and run a scripted clock to its goal
  through Session; visually verify native and real WebGPU browser output and
  exercise the same scene in editor Play before claiming completion.
- [x] Update scripting and subsystem contracts, generated Decay API docs and
  machine-readable metadata (via the generator), LSP/semantic validation where
  relevant, capability/parity evidence and the changelog. Name the external
  integration and feature example honestly; retain in-tree game proof as a
  separate completion requirement under AGENTS.md. Day/Night Lab reuses the
  generated typed APIs; this example adds no host surface.

Acceptance: a Decay script continuously advances its own clock and changes
the sun, ambient illumination and background; the same scene renders correctly
in native, browser and editor Play; a moving imported GLB carries functional
lights; and imported models cast and receive shadows. Do not tick this item
based only on schemas or compilable APIs. This feature-level acceptance is delivered in the foundation; future extensions
land in separate PRs.

Skyboxes, procedural atmosphere, stars, moon rendering and weather are optional
future enhancements, not prerequisites; do not build a complete skybox solely
for this example.

### 5. Author 3D scenes in the editor
- [x] The editor loads models through the same asset queue as the player,
  with hot reload and decode warnings in the console.
- [ ] Offscreen-rendered thumbnails for `.glb` files.
- [ ] Drag a model into the Scene view. It lands on the surface under the
  pointer (ray hit), else on the ground grid. Each drop is one undo step.
- [ ] Triangle picking for surface meshes and models (per-mesh BVH, built
  once per resource).
- [ ] The Scene view:
  - adopts `SceneCamera`;
  - has fly mode (right-drag look plus WASD), F to frame by bounds, and a
    ground grid with snapping.
- [ ] Create menu: Cube, Sphere, Plane, Camera, Point Light, Spot Light,
  Model (asset picker).
- [x] Render/static-collision model asset metadata and GLB-only project pickers,
  real project-backed Add Component defaults and independent widget/undo scopes;
  native save/reopen and Play proof in Model Collision Lab.
- [ ] Inspector model node list and material overrides each node uses.
- [ ] 3D collider handles (box extents, sphere radius, capsule height), one
  undo step per drag. Fit to model bounds.

### 5b. Terrain
Depends on slice 3's instancing for foliage and slice 5's ray hits for
placement. The roadmap's "height/mesh terrain next" is this slice.
- [ ] `sindri.terrain`: a chunked heightfield, either generated from a seed
  (reusing `sindri-voxel`'s height and biome fields) or loaded from a 16-bit
  heightmap. Normals come from the heights. Distant chunks drop detail
  without cracks.
- [ ] Up to four ground layers (for example grass, rock and sand) blended by
  slope, height or painted weights, through slice 3's lit path and shadows.
- [ ] A Rapier heightfield collider. Decay can ask `Terrain.height_at` and
  `Terrain.normal_at`.
- [ ] Foliage: rules (a model, density, slope and height range, biome) that
  scatter instances deterministically from the seed. Each sits on the
  surface and can tilt to its normal; it may carry a collider.
- [ ] Editor:
  - sculpt brushes (raise, lower, smooth, flatten);
  - brushes that paint layers and paint or erase foliage;
  - one undo step per stroke;
  - anything dropped or moved can snap to the ground.
- [ ] Terrain exports and plays in the browser, and its edits save as a
  heightmap asset beside the scene.

### 6. Play in 3D
- [x] Decay `yaw`/`pitch`/`roll` on the transform (radians, Y-up, -Z
  forward; each keeps the others when written).
- [x] Decay 3D transforms, the rest:
  - [x] `rotation` (copied `Quat` quaternion; atomic normalized whole writes);
  - [x] `forward`/`right`/`up`;
  - [x] `look_at`, `rotate_around`.
  The [Transform Lab](../examples/transform/README.md) opens and runs to its
  goal through the shared runtime; game proof remains with Explorer.
- [ ] Camera:
  - [x] Explicit perspective FOV and transform look-at.
  - [x] Engine-owned orbit/third-person follow with smoothing and collision
    pull-in (a 3D ray), checked in the [Orbit Camera Lab](../examples/orbit/README.md).
  - [x] Editor Camera Orbit creation from a real authored target, checked
    settings/target edits, Undo/Redo and save/reopen; native authored Play
    without the camera script observes pull-in/recovery and reaches the goal.
  - [x] Selected orbit focus, requested position/yaw circle and current-pose
    guides, clipped in Scene view and verified in native editor Play.
  - [ ] Explorer gameplay/browser proof.
- [ ] Input:
  - [x] Read-only mouse `Pointer.delta` / `Input.Pointer.delta` in viewport
    pixels per fixed step, with typed access and drag-to-look in Orbit Camera Lab.
  - [x] Pointer lock in native and browser (`requestPointerLock`).
    - [x] Shared input capture feedback and relative-motion accumulation,
      including transition rebasing, focus loss and fixed-step regressions.
    - [x] Host capture/release and actual-state feedback, Decay requests and
      read-only lock state, with native/browser runtime verification.
  - [x] Native editor Play input: checked window capture feedback, raw mouse
    counts, U/Escape/focus-loss/pause/stop/hidden-view release, no automatic
    recapture, unit regressions and actual X11 window verification.
  - [ ] Explorer game input proof.
- [ ] `sindri.physics3d.character`:
  - [x] Engine-owned read-only Rapier movement query with tested slope, step
    and ground-snap options; current poses, copied results and strict validation.
  - [x] Scene-owned requests/results and fixed-step application, scaled probes,
    character voxel residency, lifecycle invalidation and Session replay.
  - [x] Read-only classified ground probes with support entities, skin contacts,
    slope limits, penetration rejection, deterministic ties and native tests.
  - [x] Read-only grounded movement with verified previous support, solved
    translation/rotation-point carry, separate hits and classified final support.
  - [x] Scene-owned solved moving-platform support/carry with translation/rotation,
    lifecycle invalidation, opt-out, actual-carry terrain residency and native
    scene/Session replay regressions. Browser/editor/game proof remains below.
  - [x] `Physics3d.character_motion` and movement requests in Decay: typed
    optional support/carry, separate raw movement and detailed collisions,
    next-step/last-valid-input validation, nested copies, native scripts and
    shared Session replay, compiler/LSP and generated API metadata.
  - [x] Character Lab feature example: Decay gravity/jumps, ferry carry/reversal,
    native Session keyboard goal and real WebGPU Chromium goal/capture, with CI.
  - [x] Editor footing gizmos reuse the selected 2D character overlay, using
    validated scaled probes for slope/step/snap guides. Native Character Lab
    review exercises settings, undo/redo, save/reopen, Play goal and Stop.
  - [ ] Explorer controller adoption and game goal proof.
- [ ] Collision from content:
  - [x] CPU model-space triangle preparation in `sindri-assets`: bounded selected
    hierarchy/instance expansion, source ranges, reflected winding, invalid-data
    diagnostics, native GLB regressions and the `model --collision` inspector.
    No runtime model collider is implied.
  - [ ] Physics triangle/hull geometry, backend validation and query support:
    - [x] Immutable static triangle geometry and keyed groups, native immediate
      queries/character movement, landing/replay and atomic validation regressions.
      Standalone foundation only; no scene model collider is implied.
    - [x] Immutable volumetric hull geometry and mixed primitive/hull bodies,
      bounded construction/mass validation and native dynamic landing, pending
      impulses, sensor events, kinematic controls and replay. Standalone proof;
      scene model bindings and browser/editor/Explorer project proof remain open.
  - [ ] Scene/runtime model-collider asset bindings, asynchronous loading and
    native/browser/editor/Explorer proof, without an assets → physics dependency:
    - [x] Static model component, explicit Session CPU binding, composed positive
      scale and native scene/GLB movement, lifecycle, replacement and replay tests.
    - [x] Shared collision-only asset discovery and export, with inactive/prefab
      references, deduplication, original bytes and missing-asset regressions.
    - [x] Shared player collision installation after native/browser loading,
      before Session starts; scene/prefab/override/inactive discovery, render-only
      opt-out and named missing/invalid asset regressions.
    - [x] Model Collision Lab feature proof: actual GLB wall obstruction and
      keyboard bypass/goal through player Session tests and exported browser smoke.
    - [x] Native headless collision loading and editor Play binding from the
      asynchronous model queue, including unsaved references and retained later
      scene/prefab assets; named startup failures and safe model hot reload.
    - [x] Model Collision Lab editor/headless goal parity and restore/replay proof;
      headless layout/inactive/missing/malformed/nested override regressions.
    - [x] Native editor Play goal, moved-wall reload, invalid collision replacement
      retention, Save refusal and Stop restoration; see
      [the interaction review](model-collision-editor-review.md).
    - [x] Model asset field metadata/pickers, project-backed Add Component defaults,
      ownership/world-space guards, uppercase native discovery and independent
      Undo/Redo; generated catalogue and native save/reopen/Play review.
    - [ ] Explorer game proof.
    - [ ] Model-derived convex colliders across authored projects:
      - [x] Convex scene ownership and explicit Session GLB binding, composed
        positive scale, mixed primitive/hull bodies, dynamic voxel reach and native
        landing/lifecycle/atomic failure/replacement/checkpoint regressions.
      - [x] Convex reference discovery/export and automatic player/headless/editor
        loading, atomic shared reload and editor defaults/ownership guards.
      - [x] Model Collision Lab falling GLB prop, Decay contact-start impulse and
        travel goal through native player tests, rebuilt Chromium and native
        editor Play; safe reload and authoring review.
      - [ ] Explorer adoption of dynamic model props and game goal proof.
      - [x] Pre-synchronization Decay controls for convex-only/mixed bodies,
        real-body/schema/material/ownership checks, deferred geometry retry,
        scaled/combined mass and checkpoint replay; typed spawned-prop lab proof.
  - static trimesh colliders built from a model's meshes;
  - convex hulls for dynamic props;
  - [x] Scene collider dimensions and offsets follow composed positive scale,
    with uniform round shapes, orthogonal nonuniform boxes and atomic rejection.
  - [x] Orbit Lab keyboard shrink/restore with synchronized center/edge query
    evidence through Session and real Chromium; native editor scale edits,
    Undo/Redo, save/reopen, Play query/goal and Stop restoration.
  - [ ] Explorer scale adoption and game proof.
- [ ] Physics material profiles in 3D, sharing the 2D asset format.
  - [x] Standalone live coefficient reads/updates for primitive, convex, triangle
    and keyed static colliders, with atomic validation, retained motion/geometry,
    contact wakeup, friction/restitution response and native replay regressions.
  - [x] Resolve shared profiles and overrides on primitive/model/convex/voxel
    colliders, retaining live motion and groups on coefficient edits/reloads.
    Share Session source validation and preserve latest host assets on restore.
    Export references from inactive entities and prefabs; use a shared profile
    in Model Collision Lab and exercise native scene/runtime/export regressions.
  - [x] Exercise project loading/export and native/browser feature goals;
    review native editor profile selection, overrides, Undo/Redo, save/reopen,
    valid/invalid reload and Stop restoration.
  - [ ] Adopt shared physics profiles in Explorer and prove its game goal.

### 6b. Skeletal animation
- [ ] Decode skins (joints, inverse bind matrices, `JOINTS_0`/`WEIGHTS_0`)
  and animation clips (translation, rotation, scale; step and linear
  sampling) from GLB.
- [ ] GPU skinning in the model path, including its shadow pass.
- [ ] `sindri.animator`: the playing clip, speed and looping, with
  cross-fades between clips. Decay can play, blend and query clips.
- [ ] The editor previews a model's clips in the inspector and plays them
  in the Scene view.

### 7. Prove it
- [ ] A new genre showcase, `games/explorer`: a third-person 3D game built
  from GLB models.
  - Its level stands on `sindri.terrain` with scattered foliage.
  - It needs collision from its models and the terrain, a character controller,
    an orbit camera with mouse look and touch, lights and shadows, and a
    goal.
  - It is exported to Pages, smoke-tested in Chromium, and has a scripted run
    to its goal in its test.
  - It is authored in the editor, and screenshots of that are part of the
    review.
- [ ] Causeway gets a model prop in its world (the Milestone 8 item "a 3D
  prop in the companion game").
- [ ] Benchmark targets from slice 2 are met in editor Play and in the
  browser.

### 8. Final integration
- [ ] Update `parity.md`, `capabilities.md` and `ROADMAP.md` (Milestone 8
  ticked or honestly partial), `docs/imported-models.md`, the CHANGELOG, and
  the regenerated `docs/generated`.
- [ ] Review pass and editor screenshots.

## Out of scope (stated, not silently dropped)

- Morph-target animation.
- IBL and environment maps, SSAO, MSAA/TAA, LOD, occlusion culling.
- `.gltf` with external files, OBJ and FBX.
- 3D joints.
- Caves, overhangs and holes in terrain. Those stay with voxel terrain.
- Validation across representative integrated and discrete GPUs remains a
  broader manual check. This slice exercises native X11/Vulkan on an RTX 3060
  and browser WebGPU on Chromium's software Vulkan backend.

## Decisions

1. **Art for the showcase.** A CC0 kit (Quaternius) vendored under
   `games/explorer/assets` with its licence, because a rigged character
   exported by a real tool tests skinning far better than a generated one.
   Small generated GLBs stay as test fixtures.
2. **Animation.** Skeletal animation is in scope (slice 6b).
3. **Terrain.** Smooth heightmap terrain with foliage is in scope (slice 5b).
   Voxel terrain stays for worlds that are dug into or destroyed, as
   Causeway's is. Low Tide's seabed is the external case it serves: tides
   need a waterline that slides over a slope, and the crawler's tracks need
   continuous ground.

## Open questions

1. **Low Tide.** Should its needs (the crawler, crew on a moving deck) steer
   slice 6, or stay external?
