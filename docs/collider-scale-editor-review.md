# 3D collider scale editor review

Reviewed on 2026-10-13 in the native editor on isolated X11 with Mesa software
Vulkan. `target/tmp/orbit-scale-editor/` copies Orbit Camera Lab, including its
new keyboard controls; the repository project remains unchanged by the review.
This adopts the existing general scene scale capability in a feature example.
Explorer game proof remains open.

Selected Barrier in Scene view and changed Transform X scale from 10 to 2.
The mesh and green collider wireframe narrow together. The collider's local
half extents remain [0.5, 0.5, 0.5], and the saved transform is [2, 4, 0.4];
the inspector does not rewrite shape dimensions to world units. Undo restores
the ten-unit wall and its wireframe. Redo restores two units; Save and a fresh
editor reopening retain that scale and all six authored entities.

Played the reopened project in Game view. Key 1 narrows the active barrier to
0.02 units; the view shows the thin wall and camera recovery, and the console
reports `Barrier narrow` and `Barrier scale query narrow`. The latter requires
the synchronized center ray to hit the barrier and the ray at X=3 to miss it.
The walker reaches `Arrived`. Key 2 restores ten units and reports
`Barrier scale query wide`, requiring both rays to hit the active barrier.
Game view shows the wide wall; pausing and selecting Scene shows the matching
wide collider guide. Stop restores the saved two-unit wall, authored camera
and walker poses, and six entities. No Play scale change is saved.

Inspected captures: `target/orbit-scale-editor-before.png`,
`orbit-scale-editor-narrow.png`, `orbit-scale-editor-undone.png`,
`orbit-scale-editor-redone.png`, `orbit-scale-editor-reopened.png`,
`orbit-scale-editor-play-narrow.png`, `orbit-scale-editor-play-wide.png`,
`orbit-scale-editor-play-guides.png` and `orbit-scale-editor-stopped.png`.
Saved scene data separately confirms the persisted scale. The reopened capture
shows the narrow wall before Play; the stopped capture shows the retained scale
in the inspector.

Session regression drives only normal keyboard input, observes reactivation
after automatic removal, repeated narrow/wide queries, orbit recovery/pull-in,
R returning to automatic removal and the walker goal. The exported Chromium
smoke uses the far-side inspection camera and exercises the same scale controls
after the goal, retaining narrow/wide
captures alongside its pointer-lock proof. Native/browser feature evidence
does not close Explorer adoption or broader GPU/native-platform coverage.

Final browser captures inspected: `target/browser-orbit-scale-scale-narrow.png`
and `target/browser-orbit-scale-scale-wide.png`. The inspection camera frames
the thin wall and restored wide wall behind the blue walker on its gold goal;
center/edge query milestones passed for narrow, wide and a second narrow edit.
The rebuilt player also retains the exact 1420/60 captured-motion assertion,
release/Escape behavior, sandbox denial and both game-goal checks. CI retains
both scale captures with the existing orbit capture.
