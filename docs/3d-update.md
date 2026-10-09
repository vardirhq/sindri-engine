# 3D update: Milestone 8, imported content to a playable 3D game

One pull request, delivered in checked feature slices, the way
[the editor update](editor-update.md) was. Each slice is committed and pushed
before the next starts. Runtime, editor, scripting, proof and documentation
move together.

## Why

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
   with GGX. Models neither cast nor receive shadows, and they ignore fog.
   Cutout geometry casts solid shadows. The only light is one sun: there are
   no point or spot lights, no transparent 3D pass and no material type.
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
5. **Nothing proves it.** No in-tree project moves a character through a 3D
   level with a perspective camera. The low-tide crawler proof is external,
   and AGENTS.md does not count it.

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
- [ ] Prove that `sindri-player <project>` plays a model scene natively in a
  window. It is not yet looked at on screen.

### 2. Measure 3D
- [ ] A benchmark project: a few hundred props (cubes, surface meshes,
  repeated models), shadows on, perspective camera.
  `scripts/frame-benchmark.py` reports draw count, passes, uploads and
  encode/GPU time.
- [ ] Set the targets the next slice must meet (for example, draws and passes
  per frame independent of repeated props, and zero steady-state uploads).

### 3. One lit 3D path
- [ ] A `Material` shared by meshes, cubes, voxels and models:
  - base colour (factor and texture);
  - metallic/roughness;
  - emissive (replaces `glow`);
  - alpha mode (opaque, cutout or blend);
  - double-sided.
  Inline meshes gain optional normals, and a cube gets real normals.
- [ ] One render pass per stage. Draws are sorted by pipeline, material and
  mesh, and repeated mesh+material pairs are instanced.
- [ ] Inline meshes are cached by revision, like voxel sections, so there is
  no per-frame upload. Indices are u32.
- [ ] Frustum culling of meshes and models at extraction, from cached
  bounds.
- [ ] Models cast and receive shadows, and are fogged. The shadow pass
  respects cutout. PCF filtering, and an optional second cascade.
- [ ] Mipmaps generated on upload for smooth-filtered textures.
- [ ] A sorted, blended transparent 3D pass.

### 4. Lights
- [ ] `sindri.light` gains point and spot lights (range, cone), a bounded
  forward set per frame (for example 8, nearest/most intense first), lit on
  every geometry kind.
- [ ] The editor shows range spheres and cone gizmos. Create Point Light and
  Create Spot Light.
- [ ] Decay can read and set a light's colour, intensity and enabled state.

### 5. Author 3D scenes in the editor
- [ ] The editor loads models through the same asset queue as the player,
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
- [ ] Inspector for `sindri.model`: an asset picker, the node list, and the
  material overrides each node uses.
- [ ] 3D collider handles (box extents, sphere radius, capsule height), one
  undo step per drag. Fit to model bounds.

### 6. Play in 3D
- [ ] Decay 3D transforms:
  - `rotation` (quaternion);
  - `euler`/`yaw`/`pitch`/`roll`;
  - `forward`/`right`/`up`;
  - `look_at`, `rotate_around`.
- [ ] Camera: FOV, look-at, and an orbit/third-person follow mode with
  collision pull-in (a 3D ray).
- [ ] Input: pointer delta on `Pointer`, and pointer lock in native and
  browser (`requestPointerLock`).
- [ ] `sindri.physics3d.character`:
  - a Rapier kinematic character controller, with slopes, steps, ground snap
    and moving platforms;
  - `Physics3d.character_motion` in Decay;
  - editor gizmos reused from the 2D character.
- [ ] Collision from content:
  - static trimesh colliders built from a model's meshes;
  - convex hulls for dynamic props;
  - 3D collider dimensions follow transform scale.
- [ ] Physics material profiles in 3D, sharing the 2D asset format.

### 7. Prove it
- [ ] A new genre showcase, `games/explorer`: a third-person 3D game built
  from GLB models.
  - It needs a level with collision from its model, a character controller,
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

- Skeletal and morph animation; glTF animation clips. Node transforms are
  retained, so rigid part animation from Decay is possible.
- IBL and environment maps, SSAO, MSAA/TAA, LOD, occlusion culling.
- `.gltf` with external files, OBJ and FBX.
- 3D joints.
- Validation on representative integrated and discrete GPUs. This is not
  possible in this container; it is a manual check.

## Open questions

1. **Art for the showcase.** The choices are to generate GLBs procedurally
   with a script in-tree (like the #504 fixture), or to vendor a CC0 kit (for
   example Kenney or Quaternius) under `games/explorer/assets` with its
   licence.
2. **Animation.** A third-person character without skeletal animation will
   look stiff. Either skinning is pulled into scope, or the character is
   built from rigid parts.
3. **Low Tide.** Should its needs (the crawler, crew on a moving deck) steer
   slice 6, or stay external?
