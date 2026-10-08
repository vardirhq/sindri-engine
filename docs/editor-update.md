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

- [ ] **1. Measure the whole frame.** The Profiler times egui layout and
  tessellation, extraction, GPU submission, present wait and idle, beside the
  existing gameplay phases. A headless editor-frame benchmark opens Causeway,
  the platformer, Orbital Last Stand and Voxel Lab, plays a scripted run, and
  reports frame-time percentiles. Baselines for debug and optimised builds,
  and the standalone host for the same projects, are recorded below, with the
  frame-time target chosen from them.
- [ ] **2. Build profiles.** The dev profile optimises dependencies (and the
  workspace as far as compile time allows, measured), and there is one
  documented way to launch the editor optimised. README,
  `scripts/capture-editor.sh` and `docs/editor-architecture.md` agree with it.
  The slice-1 benchmark records the before and after.
- [ ] **3. One runtime session.** `sindri-runtime` exists with the session,
  styling settlement, physics and audio wiring the shipped host uses today.
  `sindri-causeway` runs on it natively and in the browser; editor Play runs
  on it and `fixed_step` / `EditorFrame` are gone; the genre showcases' and
  examples' test harnesses use it instead of their own `Run`. Proof: the same
  scripted input over the same scene ends in an identical world in the editor
  session and the standalone session, for the platformer, Scorchball, Low
  Tide and Orbital. Scripts in editor Play run on a styled world, as in the
  build. WASM and real-browser checks pass for every exported project.
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
- [ ] **5. Edit while playing, with a decision at Stop.** Authoring stays
  enabled during a run. An edit made while playing is a command applied to the
  running world and recorded against the run; Stop lists those edits and
  applies the ones kept to the restored snapshot as ordinary history
  entries. Save during a run still refuses, with the reason. Edits whose
  target the run destroyed or spawned are explained rather than silently
  dropped. Proof: tune the platformer hero's jump during a run, keep it, and
  the saved scene carries it; discard and it does not.
  `docs/editor-direction.md` and the authoring-guard contract are updated.
- [ ] **6. Record and scrub a run.** A run records periodic session snapshots
  and its input log; the Timeline scrubs to any recorded step by restoring the
  nearest snapshot and replaying, deterministically. Scrubbing pauses the run;
  resuming continues from the scrubbed step and discards the recorded future.
  Memory is bounded and the bound is stated. Proof: scrub an Orbital Last
  Stand run back to an earlier wave and resume; a replayed range is identical
  to the recorded one.
- [ ] **7. Close the remaining audit gaps.**
  - Gizmos for the shapes that still have none. Colliders, cameras and lights
    have them; audit joints, 3D colliders, character controllers, audio
    falloff and effect reach first, then build each one that is missing.
  - Console problems grouped by cause, naming the entities under each.
  - The six right-click surfaces `docs/editor-authoring-audit.md` §6 lists.
  - Angle, Mask and Entity inspector controls, sprite thumbnails, and Camera
    Behaviour's declared meanings (`docs/editor-usability-audit.md` §6).
- [ ] **8. Final integration.** Causeway plays in the editor at the target;
  `parity.md`, `capabilities.md`, `editor-architecture.md`,
  `editor-direction.md`, the audits' status lines, `CHANGELOG.md` and
  `ROADMAP.md` are current; workspace, WASM, browser and editor capture checks
  pass; the final diff from `main` is reviewed; CI is green on the final head.

## Baselines

Recorded by slice 1. Empty until then.

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
