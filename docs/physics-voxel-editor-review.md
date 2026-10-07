# Native 3D and voxel collision editor review

Reviewed on 2026-10-07 at `b66601e4`, with a freshly built native editor on an
isolated X display (Xvfb, Mesa lavapipe Vulkan) and a scratch copy of Causeway
made by `scripts/browser/prepare-causeway-voxels.py`, which adds the read-only
`BrowserVoxelLanding` observer. The shipped scene and scripts were not changed.

## Voxel Collider 3D authoring

Select Floor. Its inspector shows the Physics3d Voxel Collider section with the
authored friction 0.80 (marked as differing from the default), layer filter and
memberships, margin 2.00 and restitution 0.00.

| Step | Observed result in the saved scene |
| --- | --- |
| Friction 0.80 → 0.60, Save | `friction: 0.6` |
| Undo, Save | `0.8`; redo, Save: `0.6` |
| Remove the component (trash), Save | component absent |
| Add Component → Physics → Voxel Collider 3D, Save | registered defaults: friction 0.5, restitution 0, all layers, margin 2 |
| Undo the addition and the removal, Save | friction 0.6 restored; one more undo restores 0.8 |

Numeric fields need the window focused before typing (`xdotool windowfocus`);
without focus, keystrokes never reach the text field and nothing changes.

## Block-set Collides

Open `causeway.tileset` from the Project panel. The block-set editor lists
Collides beside Supports and Walkable. Clay shows it checked; Water shows it
unchecked with Supports checked and Walkable unchecked. Checking it and saving
drops the key (default true); unchecking and saving writes `"collides": false`.

## Play, Stop and replay

With Console open, Play. The observer drops Causeway's own loose-block prefab
above the wanderer; the console prints `Voxel landing dropped`, then
`Voxel collision landing verified`. The observer prints that line only after a
downward `Physics3d.raycast` hits the Floor *and* the block's script records a
landing reported by `Physics3d.collision_started`, not by the give-up timer. The
Game view renders the generated voxel world. Save during Play leaves the file's
SHA-256 unchanged; Stop and Save match the pre-Play hash
(`bcb7d27f…2e360`). Clear the console and Play again: both lines appear again,
so the solver and voxel cache reset with each run.

## 3D body authoring and Play

Hierarchy + → Create Empty. Set the transform to (32656, 12, 32552), above the
wanderer's start. Add Component → Physics → Rigid Body 3D (Dynamic by default)
and → Collider 3D (a 0.5 half-extent box by default); the saved scene contains
both registered payloads. Play: the inspector turns read-only and the live
position Y reads 1.500. That is the top of the voxel at level 0 (height 1) plus
the box's 0.5 half-extent, so the box has fallen eleven units and landed on the
generated terrain. Stop restores the authored Y of 12, and saving writes
(32656, 12, 32552) again.

## Evidence and limits

Captures were inspected for the voxel collider inspector, the Add Component menu
listing Voxel Collider 3D, the block-set Collides control, both console lines,
the Game view and the live landed height. Artifacts are `/tmp/vox-*.png`, the
editor log `/tmp/vox-editor.log` and the scratch project `/tmp/vox-review`.
Scene/session regressions, the Causeway play test and the CI browser fixture
provide numerical, gameplay and browser evidence; this review provides real
window interaction. It does not claim per-block physical coefficients, 3D joints
or 3D character controllers, which remain 2D-only or absent.
