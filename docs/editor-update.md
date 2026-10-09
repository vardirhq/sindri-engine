# Editor update: one runtime, live editing

One pull request, delivered in checked feature slices, the way
[the physics update](physics-update.md) was. Each implementation slice is
committed and pushed before work proceeds to the next; smaller checkpoints are
permitted. Runtime, editor, scripting, proof and documentation move together.

For a new implementation session, read [the recovery handoff](editor-update-handoff.md).

## Why

A game played in the editor is laggy while the same game in a browser keeps
getting faster. Reading the code (October 2026, at `b2872f0`) gives four
reasons, in the order they are likely to matter. None of them has been
measured yet, which is why measuring is the first slice.

1. **The editor is an unoptimised build and the browser is not.** The README
   and `scripts/capture-editor.sh` start the editor with `cargo run`, the dev
   profile, while `.github/workflows/pages.yml` builds every browser host with
   `wasm-pack --release`. The workspace already optimises dependencies for
   tests, because unoptimised physics and Decay made the suite three times
   slower (`Cargo.toml`, `[profile.test.package."*"]`); nothing does the same
   for the editor. Every physics feature added since widens the gap.
2. **Play is a second copy of the game loop.** The editor steps gameplay in its
   own `fixed_step` (`editor/src/native/runtime.rs`) through its own
   `EditorFrame`, beside the shipped host's `Session` (`game/src/session.rs`)
   and the hand-assembled `Run` in each genre showcase's `src/lib.rs`. Work
   that makes the shipped host faster does not reach the editor, and the copies
   can drift in behaviour, which defeats the reason Play exists.
3. **The editor does work every frame that the game does not.**
   - Weave presentation copies the world: `ProjectStyles::resolve`
     (`editor/src/weave_styles.rs`) clones it once plus once per stylesheet,
     per view, and `present_live` clones it twice during Play. The shipped
     host settles styles once and lays per-draw states over the world in
     place, then undoes them (`game/src/styling.rs`), precisely because "a
     copy of the world is a copy of the level". That is also a behaviour
     difference: a script in the build runs on a styled world, and in the
     editor on an unstyled one.
   - With the Scene and Game views both showing, the world is measured,
     extracted and drawn twice per frame.
   - `advance_play` clones the component registry and `Scripts::compile`
     deserialises every script component every frame, playing or not;
     `animated::moves` scans every entity once per view per frame.
   - `render_view` requests a repaint unconditionally, so an editor at rest
     redraws at the display's refresh rate for ever.
   - No list is virtualised (no `show_rows` anywhere), so the hierarchy,
     console and project browser lay out every row every frame, and a run that
     spawns entities grows the hierarchy while it plays.
4. **The Profiler cannot see the editor.** It times gameplay phases and each
   view drawn, but not egui layout and tessellation, extraction apart from
   GPU submission, or waiting on present. Nobody can currently say where an
   editor frame goes.

Fixing 2 is also the foundation for the two largest open items in
`docs/editor-direction.md`: **edit while playing, with a decision at Stop**,
and **record and scrub a run**. Both need a run to be a deterministic thing
held apart from the document. So the performance work is the spine of this
update rather than a preliminary to it.

## Decisions taken

- **A new crate, `sindri-runtime`, owns a run of a scene.** The session moves
  out of `sindri-causeway` (`game/`), which is the one WebAssembly host every
  export ships (`sindri_export::page::HOST_MODULE`), and the editor, the
  shipped host and every project's test harness drive it. This is the proven
  dependency boundary `AGENTS.md` asks a new crate for: three consumers
  already carry copies of it. Intended direction:

  ```text
  sindri-runtime -> sindri-core + sindri-decay + sindri-platform
                    + sindri-scene + sindri-weave
  editor         -> ... + sindri-runtime
  sindri-causeway, games/*, examples/* -> sindri-runtime
  ```

  No window, GPU, renderer or editor dependency: rendering stays in each host.
  `AGENTS.md`'s crate graph is updated in the commit that creates the crate.
- **Every slice below is in scope**, including record-and-scrub and the
  remaining audit gaps. The scope does not need authorising again.
- **The frame-time target is set from measurement**, in slice 1, rather than
  guessed now. It is a ratio against the standalone host in the same build
  profile, recorded in this document when the baseline exists.

## What kind of change this is

In `AGENTS.md`'s terms: the shared runtime and the frame-cost work make an
existing capability (Play) trustworthy, and are proven by the existing games
and showcases. Edit-while-playing and record-and-scrub are new editor
capabilities; each is proven on a genre showcase or flagship named in its slice.

## Acceptance checklist

- [x] **1. Measure the whole frame.** The Profiler times upkeep,
  presentation, extraction, encoding, panels, egui's painting and waiting
  beside the existing gameplay phases, and can record while editing.
  `sindri-editor <project> --benchmark <report.json>` and
  `project-benchmark` record comparable reports; `scripts/frame-benchmark.py`
  runs and compares them. Baselines for debug and optimised builds of
  Causeway, the platformer, Orbital Last Stand and Voxel Lab are below, with
  the target chosen from them. The run is played without input: what a
  scripted input sequence would add is proven by slice 3's parity tests
  rather than timed here.
- [x] **2. Build profiles.** The dev profile optimises every dependency;
  a new `editor` profile (release, incremental, 256 codegen units) is what
  `cargo editor <project>` runs. README, `scripts/capture-editor.sh` and
  `docs/editor-architecture.md` say which is for what; the measurements are
  below.
- [x] **3. One runtime session.** `sindri-runtime` exists with the session,
  styling settlement, physics and audio wiring the shipped host uses today.
  `sindri-causeway` runs on it natively and in the browser; editor Play runs
  on it and `fixed_step` / `EditorFrame` are gone; the genre showcases' and
  examples' test harnesses use it instead of their own `Run`. Proof: the same
  scripted input over the same scene ends in an identical world in the editor
  session and the standalone session, for the platformer, Scorchball, Low
  Tide and Orbital. Scripts in editor Play run on a styled world, as in the
  build. WASM and real-browser checks pass for every exported project.
  The generic host every export ships moves out of Causeway's crate into its
  own, so a published game loads `sindri_player.js` rather than
  `sindri_causeway.js`, and Causeway becomes a project like any other.
  Done: `sindri-runtime` (session, `ProjectRun`, `StepReport`) and
  `sindri-player`; editor Play starts its session through
  `play_session::start`; every game's harness is a `ProjectRun`;
  `editor/tests/play_matches_the_build.rs` ends the platformer, Scorchball,
  Low Tide and Orbital in the same world both ways after 600 scripted steps;
  WASM and CI's real-browser run of every export pass. Voxel Lab keeps its
  own browser app: it is an engine lab whose Rust terrain and camera are the
  subject, as Shapes Lab's are.
- [ ] **4. Cut per-frame waste.**
  - Weave presentation in the editor settles and lays states in place with
    undo, as the shipped host does; no view clones the world to present it.
  - The registry, script queries, the animated-world scan and edit-mode
    extraction are cached by world and asset revision.
  - One extraction serves both views when they show the same world and
    camera-independent data.
  - The editor redraws only when input, an asset, a run or an animation asks
    it to; at rest it is idle.
  - Hierarchy, console and project lists lay out only visible rows.
  - Acceptance: editor Play meets the slice-1 target against the standalone
    host on all four benchmark projects, and an untouched editor does not
    redraw.
- [x] **5. Edit while playing, with a decision at Stop.** Authoring stays
  enabled during a run. An edit made while playing is a command applied to the
  running world and recorded against the run; Stop lists those edits and
  applies the ones kept to the restored snapshot as ordinary history
  entries. Save during a run still refuses, with the reason. Edits whose
  target the run destroyed or spawned are explained rather than silently
  dropped. Proof: tune the platformer hero's jump during a run, keep it, and
  the saved scene carries it; discard and it does not.
  `docs/editor-direction.md` and the authoring-guard contract are updated.
  Done: every editor edit goes through `apply_edit`, which records it against
  the run while playing (`editor/src/run_edits.rs`); Stop offers each back
  (`native/run_review.rs`) as the component fields and transform parts it
  changed. `run_edits/tests.rs` tunes the platformer hero's jump during a run,
  keeps it and finds it in the saved scene, discards it and does not, and
  keeps a scale without the position the run moved the hero to. Checked in
  the editor itself: a coin moved while playing is offered at Stop, kept as
  an undoable entry, and discarded leaves the scene as it was. Prefab
  placement and prefab files stay stopped-only, as Save does.
- [x] **6. Record and scrub a run.** A run records periodic session snapshots
  and its input log; the Timeline scrubs to any recorded step by restoring the
  nearest snapshot and replaying, deterministically. Scrubbing pauses the run;
  resuming continues from the scrubbed step and discards the recorded future.
  Memory is bounded and the bound is stated. Proof: scrub an Orbital Last
  Stand run back to an earlier wave and resume; a replayed range is identical
  to the recorded one.
  Done: the whole session can be checkpointed (`Session::checkpoint`, with
  both physics worlds copying Rapier's state and fresh pipelines, and the
  script runtime, screen UI, effects and grid surfaces cloned), and a copy goes
  on exactly as the original (`sindri-physics/tests/copies_go_on_as_the_original.rs`,
  `sindri-runtime/tests/a_run_goes_back.rs`). Play records each step's input,
  which steps the Game view was drawn after, and a copy every second
  (`editor/src/recording.rs`); the Timeline's Run strip scrubs it
  (`native/run_scrub.rs`). The bound is two minutes: 120 copies, one a second.
  `editor/tests/scrub_a_run.rs` scrubs Orbital back from its third wave to its
  first, resumes, and every replayed step matches the recording.
- [x] **7. Close the remaining audit gaps.**
  - Gizmos for the shapes that still have none. Colliders, cameras and lights
    have them; audit joints, 3D colliders, character controllers, audio
    falloff and effect reach first, then build each one that is missing.
  - Console problems grouped by cause, naming the entities under each.
  - The six right-click surfaces `docs/editor-authoring-audit.md` §6 lists.
  - Angle, Mask and Entity inspector controls, sprite thumbnails, and Camera
    Behaviour's declared meanings (`docs/editor-usability-audit.md` §6).

  Done. The audit found joints, 3D colliders, character footing and effect
  reach undrawn; audio has nothing to draw, since a source has no falloff.
  `sindri_scene::shape_gizmos` works each out from what the simulation is
  given and `native/shape_gizmo.rs` draws them, joints and 3D colliders always
  and footing and reach for the selection; a click on a joint selects it.
  `Console::causes` groups current problems by message, and the status bar and
  console tab count causes. Every surface in the authoring audit's table has
  its menu (`native/scene_menu.rs`, `inspector_panel/section/heading.rs`, the
  hierarchy's and project browser's empty space, console lines), and entities
  copy and paste through an editor clipboard (`editing/clipboard.rs`, Ctrl+C
  and Ctrl+V read as egui's Copy and Paste events). Mask and Entity controls
  and sprite thumbnails already existed; the Angle row is new, and edits in
  degrees. Camera Behaviour declares its meanings, which needed a new registry
  idea: `describe_optional` says what a `null` field holds once added, so its
  fields can be described and the inspector offers Add and Remove. Fields
  list in the order the type declares them (`field_order`). Checked in the
  editor under Xvfb: each menu opens and acts, the joints, footing and
  inspector rows draw.
- [x] **8. Final integration.** Causeway plays in the editor at the target;
  `parity.md`, `capabilities.md`, `editor-architecture.md`,
  `editor-direction.md`, the audits' status lines, `CHANGELOG.md` and
  `ROADMAP.md` are current; workspace, WASM, browser and editor capture checks
  pass; the final diff from `main` is reviewed; CI is green on the final head.

  Done. The docs listed are current; workspace Clippy, the changed crates'
  and every game's tests and the WASM check pass, and CI is green on the
  final head. A review of the whole diff from `main` found ten problems, all
  fixed: among them, scrubbing now replays the edits made during a run, a
  restore keeps hot-reloaded scripts, a scene opened mid-run abandons the run
  whole, and `ProjectRun` reads `sindri.toml` as TOML, names scenes as the
  export does and validates component data. Slice 4 stays open on the
  platformer.

## Baselines

Measured on this update's branch before any of slices 2–8, on a 4-core cloud
VM with no GPU: Mesa's lavapipe (software Vulkan, Mesa 25.2.8) under Xvfb. The
editor runs at its default 1440×1024 in the Canvas arrangement with the Game
view showing; the standalone host draws offscreen at 1280×720. 300 frames per
section after 60 to settle, vsync off. Times are means of the CPU **work** per
frame in milliseconds: waiting for the GPU or the display is reported apart
and left out, because on a software GPU it measures the GPU, not the editor.

Reproduce with `scripts/frame-benchmark.py run <project> [--release]`.

| Project | Build | Editor at rest | Editor in Play | Steps / frame | One-step game frame | Standalone | Ratio | Editor's own UI |
|---|---|---|---|---|---|---|---|---|
| Platformer | debug | 46.7 | 67.5 | 5.14 | 18.6 | 9.8 | 1.91× | 29.6 |
| Platformer | optimised | 15.4 | 16.7 | 2.10 | 6.4 | 1.3 | 4.97× | 9.4 |
| Causeway | debug | 48.0 | 88.8 | 6.44 | 54.9 | 28.7 | 1.92× | 20.8 |
| Causeway | optimised | 16.5 | 47.2 | 4.07 | 35.6 | 3.0 | 11.81× | 10.3 |
| Orbital | debug | 127.8 | 184.5 | 8.00 | 76.2 | 38.7 | 1.97× | 58.6 |
| Orbital | optimised | 41.7 | 49.6 | 4.37 | 32.1 | 6.8 | 4.73× | 13.6 |
| Voxel Lab | debug | 203.8 | 403.5 | 8.00 | 378.1 | 159.3 | 2.37× | 21.9 |
| Voxel Lab | optimised | 109.6 | 172.0 | 8.00 | 161.1 | 9.4 | 17.08× | 10.7 |

The **one-step game frame** is what the editor spends drawing the game plus
one fixed step of gameplay, which is what a standalone frame is. The editor
in Play runs more than one step a frame because it is slow, and each extra
step makes the next frame slower: Orbital and Voxel Lab sit at the clock's
eight-step ceiling. **Editor's own UI** is panels plus egui's painting.

What the numbers say:

1. **The unoptimised build is most of the lag a person sees.** Debug costs
   3–4× an optimised build almost everywhere, and the panels alone take
   22–48 ms a frame in debug against 1–6 ms optimised.
2. **Per step, gameplay costs the same in both hosts**: 4.6 against 4.8 ms
   on the debug platformer, 0.78 against 0.71 optimised. The simulation is
   not where the editor loses.
3. **Where it loses is drawing.** Optimised, the editor's encoding is 4.9 ms
   on the platformer against 0.4, 34 ms on Causeway against 1.8, 23 ms on
   Orbital against 0.8 and 98–160 ms on Voxel Lab against 8 — even editing,
   with nothing moving, so a voxel world is being remeshed or re-uploaded
   every frame. Orbital spends a further 6–7 ms on presentation, its Weave
   world copies.
4. **Editor Play is not the game.** Voxel Lab's gameplay costs the editor
   0.03 ms a step against 0.48 standalone: Play there runs almost nothing.
   The editor's script frame gives scripts no pointer aim, gestures, camera
   pan, tile sets or `Scene.go`, which the shipped session does, and steps
   effects before physics rather than after it. Slice 3 is a correctness fix
   before it is a performance one.

**Target.** In an optimised build, on all four projects: the editor's
one-step game frame is at most 1.25× the standalone host's, and at rest the
editor does not redraw. The editor's own UI is reported alongside and must
not grow. Gaps found while measuring are fixed in this update rather than
noted: `project-capture` and `project-benchmark` could not open a project
whose voxel world names `builtin:blocks`, and neither could the shipped
browser host; every host now binds the engine's own block set and textures,
as the editor always did.

## Slice 4 so far

**Two of the findings above were measurement errors.** Finding 3's editor
encoding was mostly a software GPU finishing egui's previous frame: a view's
submit blocked on it and the wait was booked as encoding. A benchmark now
waits for the GPU before each view encodes, timed as GPU, as the standalone
benchmark waits after each frame. Finding 4's 0.48 ms standalone step for
Voxel Lab is lavapipe winding down after a frame: the first phase after a GPU
wait absorbs it, and the same step measures 10 µs headless. Voxel Lab's
gameplay costs the same in both hosts.

The benchmark now also compares like with like: the editor's Game view plays
at the standalone's 1280 × 720, with no pointer resting over it (the
benchmark plays without input; a hovered element was styled every frame),
and the Scene view drawn beside a run is timed as its own phase, reported
with the panels as the editor's work rather than as the game's frame.

What changed in the editor:

- An editor at rest asks for no frames. A background watcher wakes it when a
  project file changes; loads still in flight are looked for again shortly.
  Untouched for three seconds after a second's grace, the platformer, Orbital
  and Causeway draw no frames; Voxel Lab draws its animated water, which asks.
- Weave presents the edited scene in place with undo
  (`Presenter::present_in_place`), and a run's spawned entities are settled
  in place rather than in a copy of the world.
- Each view has its own extractor and cube renderer. Sharing them, a voxel
  world's Game view recorded its frame in 8.7 ms against 3.5 alone.
- The hierarchy, console and project lists lay out only the rows in sight.

One-step game frame against the standalone host, optimised, 600 frames:

| Project | Editor | Standalone | Ratio |
|---|---|---|---|
| Causeway | 2.68 ms | 2.66 ms | 1.01× |
| Platformer | 1.60–1.67 ms | 1.24–1.29 ms | 1.25–1.32× |
| Orbital | 6.94 ms | 6.48 ms | 1.07× |
| Voxel Lab | 9.14 ms | 8.55 ms | 1.07× |

Left open: the platformer is at or just over the target. Its gap is 0.3 ms on
a 1.2 ms frame, and most of it is the step itself: physics takes 0.40 ms a
step in the editor against 0.31 standalone, on the same world (the parity
test proves it) with the same code, which points at the editor's own frame
evicting caches between steps rather than at work the editor adds. The
caching sub-items above were premised on finding 3's numbers; measured
correctly, upkeep and extraction are under a millisecond on every project.

Measured again after slices 5 to 7, with a run now recorded while it plays:
the platformer's one-step game frame is 1.46 ms in the editor against
1.09 ms standalone (1.33×), the gap split between the step (+0.11 ms),
encoding (+0.13) and extraction (+0.06); Orbital's is 6.71 against 7.53
(0.89×). Recording copies the world and the session once a second, which
costs about a step and a half at that moment (0.77 ms for the platformer,
6.7 ms for Orbital, against steps of 0.43 and 4.7 ms) and about 2% of a
step on average.

## Build profiles

Slice 2, measured the same way as the baselines: a clean build of the editor,
a rebuild after touching one of its files, and the work per frame in Play.

| Build | Clean build | Rebuild after an edit | Platformer in Play | Orbital in Play |
|---|---|---|---|---|
| dev, as it was | 110 s | 11 s | 69.7 ms | 169.6 ms |
| dev, dependencies at `opt-level = 2` | 327 s | 11 s | 40.9 ms | 109.9 ms |
| dev, and the workspace at `opt-level = 1` | 370 s | 26 s | 20.8 ms | 60.8 ms |
| `editor` profile | 364 s | 3 s | 17.6 ms | — |
| release | — | — | 16.7 ms | 49.6 ms |

The dev profile takes the second row: it costs one longer clean build and
nothing per edit, and the tests already optimised their dependencies, so a
plain build and the test suite now share those artifacts rather than each
building them. The third row is close to release but makes working on the
editor two and a half times slower to rebuild; the `editor` profile gets
release speed for using the editor instead. A rebuild after a touch with no
change is what was timed; a real edit recompiles more, in both profiles.

## Verification

Follow `AGENTS.md`'s pre-push gate for every code push. Record actual results
here as each slice lands; a checkmark reflects exercised behaviour, not an
exposed type. `docs/parity.md` records surface-specific completeness and this
checklist does not mark those surfaces done.

## Risks

- **Moving the session touches every host at once.** The browser host is the
  one every export ships, so slice 3 is checked in a real browser for every
  project before it is called done, not only compiled for `wasm32`.
- **The editor and the build stepping differently is a bug class this update
  removes, and also one it can expose.** A game that only worked because the
  editor ran scripts on an unstyled world will change when it starts running
  on a styled one. Those differences are fixed in the game, not preserved.
- **Stop's decision crosses the snapshot boundary.** An edit recorded against a
  run may name an entity the run spawned, or one it destroyed. Slice 5 states
  what happens to each case and tests it before the UI is built.
- **Recording costs memory.** Snapshot interval and run length are bounded, and
  the editor says when a recording has been trimmed rather than pretending it
  holds the whole run.
