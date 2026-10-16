# 3D foundation review

PR #507 ends at the shared 3D foundation and Day/Night Lab. The remaining
Milestone 8 implementation is split into ordered follow-up PRs in
[`3d-update.md`](3d-update.md#follow-up-order). Milestone 8 and in-tree game
adoption remain incomplete; feature examples do not replace that acceptance.

## Delivered boundary

- Shared asynchronous GLB loading, rendering, warnings, export and safe reload
  across the player, native editor and headless project paths.
- Static triangle and dynamic convex model collision, composed positive scale,
  shared physics material profiles and checked lifecycle/replay behavior.
- Typed 3D transforms, perspective/orbit cameras, pointer motion/capture and
  character movement with classified grounding and solved moving-platform carry.
- Scoped standalone/editor draw, pass, upload and optional native GPU timing
  measurements, including independent primary and extra Scene view work.
- Shared numeric standard material factors and normals, imported-model
  directional shadow casting/reception, bounded point/spot lights and typed
  runtime light, ambient and background controls.
- Transform, Orbit Camera, Character, Model Collision, Lighting, Benchmark and
  Day/Night feature projects, with real project/script runs and surface evidence.

The lighting work adds general capabilities requested by external Low Tide 3D
and proves them in feature examples for Explorer. It does not claim a shipped
in-tree game uses them. Earlier capabilities retain the game/evidence status
recorded in [`parity.md`](parity.md) and [`capabilities.md`](capabilities.md).

## Review focus

The review uses the complete changed-file list against `main`, subsystem
contracts, implementation seams and regression evidence. It covers dependency
and target boundaries, runtime ordering, checked authoring, asset preparation,
cache ownership, shader/collision validation and honest capability status.

`Session` remains the fixed-step authority. Model collision is prepared from
asset bytes and explicitly bound before gameplay; assets do not depend on
physics. Convex construction validates whole candidates before mutation.
Composed scale and resource replacement invalidate cached geometry safely.

The renderer remains independent of core, and voxel storage remains independent
of renderers and games. Editor native cursor handling reuses workspace `winit`;
no crate or external dependency boundary is added. Native-only imports remain
behind target guards. Decay receives copied typed values and checked atomic
writes; script clocks and gameplay decisions stay in Decay.

GPU model caches retain immutable geometry and separate shadow/color uniforms.
Each editor viewport owns its textured 3D renderer, preserving queued uniforms
and independent measurements. Light and orbit inspector drafts validate before
emitting checked commands; Undo/Redo and save/reopen are exercised. Pointer
capture follows actual host state, releases on focus/Escape, and rejects stale
asynchronous browser grants after release.

The final integration includes main's browser startup-failure page and manifest
cache revalidation. It preserves Low Tide's updated Keeper assets/scripts and
moves this branch's capability evidence into the per-area TOML inventory,
regenerating its pages rather than retaining the old hand-written index.
Validation checks the combined template and rebuilt browser player.

## Validation

The combined foundation/main tree passes warning-denied workspace Clippy,
workspace tests with all features (3,247 passing), the full WASM graph, and a separate
required-GPU/timestamp run (163 passing). The separate Decay workspace passes
formatting, Clippy and tests. Typed preflight checks all ten changed scripts
with zero errors and zero runtime-contract reminders. Formatting, file-size,
generated catalogue freshness and final diff checks pass. The subsequent
main integration passes 165 affected-crate tests, including Low Tide's scripted
runs and the migrated inventory checks. Full workspace Clippy and WASM checks
pass again; all 25 documentation pages build and inventory evidence links
resolve.

Fresh exports pass missing-canvas, missing-adapter and refused-device startup
checks against the integrated browser template. The same template runs the
continuous Day/Night Lab through all phases and its goal, fetching its four
assets without runtime/GPU errors. The real pointer-capture smoke verifies
unbounded motion, explicit/Escape release, synchronized collider-scale queries,
sandbox denial and both project goals. It closes the completed primary game
before opening the sandbox case, avoiding competing GPU runtimes after the
first case has already passed; all behavior assertions remain intact.

Native authored-color readback and settled shadowed/unshadowed benchmark
controls pass, including required nonzero GPU intervals. The fixed workload
records 602/301 draws and 603/301 passes, with zero settled model uploads.
The native editor authored-color capture is checked against the same scene
expectations.

Day/Night Lab's detailed native/browser/editor controls and measurements live in
[`daynight.md`](daynight.md). Native and browser frozen-hour controls isolate
moving sun shadows on imported ground and mounted illumination with the sun
disabled. Both clocks keep advancing after their goal. Native editor Play visits
all phases, saves a frozen sunset preview and restores authored state on Stop.
Other interaction reviews include
[`model-collision-editor-review.md`](model-collision-editor-review.md),
[`3d-lights.md`](3d-lights.md), [`runtime-environment.md`](runtime-environment.md)
and [`rendering-materials.md`](rendering-materials.md).

## Limits and follow-ups

This remains a pre-alpha foundation. Repeated props still require per-draw
passes; inline mesh retention, instancing and general mesh/model culling are
unfinished. Counters measure named render scopes rather than all frame work;
browser GPU timestamps and whole-frame upload/UI coverage remain open.

The complete shared material asset/texture/alpha contract, mipmaps, transparent
sorting and improved shadow filtering/cascades remain open. Local lights do not
cast shadows. Imported models receive renderer fog, but shipped project hosts
still omit authored fog/AO/post; runtime fog controls remain absent.

Advanced model placement/picking/navigation and collider authoring, smooth
terrain/foliage, skeletal animation and Explorer/Causeway game adoption are
follow-ups. Representative integrated/discrete GPU validation remains broader
manual work. See the ordered checklist rather than treating foundation readiness
as completion of the full milestone.
