# Editor update: recovery handoff

## Start here

Work on branch `ccr-4791a75f-7rfx8g` in `vardirhq/sindri-engine`, draft PR
#503. Keep the whole update in one pull request; do not split it into several. Read
`AGENTS.md` and `CLAUDE.md` freshly before editing, then
[the update's checklist](editor-update.md), `docs/editor-architecture.md` and
`docs/editor-direction.md`.

The user authorised every item in `docs/editor-update.md`, including the new
`sindri-runtime` crate and slices 6 and 7. Do not ask them to authorise the
scope again. Deliver checked incremental pushes: each slice is one push or
less, with smaller checkpoints preferred.

## Current checkpoint

Draft PR #503. Slices 1, 2, 3 and 5 to 8 are checked; slice 4 is open on the platformer, 1.33× against the 1.25× target (see the plan). That is what keeps the PR a draft.
Slice 3 left:

- `sindri-runtime` holds the session (from Causeway), `ProjectRun` (open any
  project directory, on its main scene or another with `open_scene`, and
  play it headless: keys, pads, tags, `on_screen`, board) and `StepReport`
  (prints, failures, problems, phase times; `notes()` and `log()`).
  `ProjectRun::step` presents after stepping — styles in place, lays the
  screen out, undoes — because a styled session hit-tests clicks against the
  last frame drawn.
- Editor Play runs on the session; `play_session::start` assembles it from
  what the editor loaded, outside the window.
- `sindri-player` is the generic host: the browser module every export
  ships (`sindri_player.js`) and `sindri-player <project>` natively.
- Every game's harness is a `ProjectRun`: platformer, Flappy, Low Tide,
  Scorchball, Orbital, the Camera Lab.
- `editor/tests/play_matches_the_build.rs` is the parity proof for the
  platformer, Scorchball, Low Tide and Orbital.
- Voxel Lab keeps its own browser app on purpose: it is an engine lab like
  Shapes Lab, whose Rust terrain and camera are the subject, not a scene and
  Decay project. Slice 3 does not name it.
- The browser smoke's intermittent "red button" failure was the Physics
  Playground's domino run ending marginally on its button (main had it too);
  fixed in the playground, with `game/tests/the_physics_playground_works/smoke.rs`.

Slice 4 so far: the editor benchmark times the GPU it waits on as GPU (the
platformer's "encoding" was a software GPU finishing egui's frame); an
editor at rest asks for no frames, a background watcher (`native/wake.rs`)
wakes it when a project file changes, and the benchmark counts frames drawn
in three seconds untouched; the hierarchy, console and project lists lay out
only rows in sight (`ui/widgets/lazy.rs`). On lavapipe the first phase
after a GPU wait absorbs the driver winding down (about 0.4 ms), which is
why `project-benchmark` times Voxel Lab's step at 0.49 ms against 10 µs
headless; it is not a gameplay difference.

Gate every push with fmt, warning-denied Clippy and the changed crates'
tests, run unpiped so a failure stops the push. Debug info off keeps the
build inside the disk allowance (`CARGO_PROFILE_DEV_DEBUG=0`,
`CARGO_PROFILE_TEST_DEBUG=0`).

## Notes for later slices

- Slice 4's in-place presentation is already how a run's Game view draws
  (`editor/src/native/viewport/mod.rs`, `session.style` then `record_drawn`
  then undo); the editing views still resolve through `ProjectStyles`.
- `scripts/frame-benchmark.py run <project> --profile editor` measures the
  slice-4 target; the baselines are in `docs/editor-update.md`.
- Under Xvfb, `xdotool` clicks, drags and wheel reach the editor but its key
  presses do not (F2 does nothing), so keyboard paths are proved through a
  real egui frame in tests (`native/tests/shortcuts.rs`) rather than by hand.
