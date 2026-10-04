# Incremental physics update

One pull request, delivered in checked feature slices. Each implementation slice
is committed and pushed before work proceeds to the next; smaller checkpoints
are permitted. Gameplay and demonstrations remain in Decay. Runtime, scene,
editor, scripting, proof and documentation move together.

For a new implementation session, read [the recovery handoff](physics-update-handoff.md).
Only the documentation reconciliation has been pushed; all physics implementation
items below still require work.

## Acceptance checklist

- [x] Reconcile outdated overlap/cast, compound and parented-body documentation.
- [ ] Continuous collision detection: opt-in authored body setting, runtime and
  Decay control; fast solid-body regression and game use. Sensor sweep limitations
  must be explicit rather than promising CCD trigger events.
- [ ] One-way platforms: collision from the supporting side, configurable local
  normal, safe drop-through, platformer proof including ascent and descent.
- [ ] Forces and rotation: force, torque, angular velocity, angular impulse and
  off-centre impulses, with explicit accumulation/reset semantics and game use.
- [ ] Contacts: entity-based world contact points, normals and force/impulse
  information, deterministic snapshots and grounded/impact proof.
- [ ] Physics materials: reusable project assets, shared validation and explicit
  literal override rules, asset loading in editor and exported games.
- [ ] Joints: scene-authored distance, hinge, slider, spring and motor controls;
  stable entity references, lifecycle/removal, editor undo, Decay access and proof.
- [ ] Character movement: reusable sweep/slide collision primitive, slopes,
  steps, ground state and moving platforms; gameplay policy remains Decay.
- [ ] Accelerated queries: synchronized spatial index, unchanged filtering and
  deterministic tie rules, immediate body moves/removals and scaling evidence.
- [ ] 3D physics and queries: fixed-step simulation, bodies/colliders/events,
  scene synchronization, editor authoring, Decay access and voxel-world proof;
  native and browser semantics exercised end to end.
- [ ] Final integration: generated catalogue, native/WASM/browser checks,
  workspace lint/tests, final diff review and green CI on the final head.

## Verification

Follow `AGENTS.md`'s pre-push gate for every code push. Record actual results in
the PR; checkmarks reflect exercised behavior, not merely an exposed type.
`docs/physics.md` is the subsystem contract and `docs/parity.md` records the
surface-specific completeness. This checklist does not mark those surfaces done.
