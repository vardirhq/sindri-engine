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

Draft PR #503. Slices 1 and 2 are checked. Slice 3 is in progress:

- `sindri-runtime` holds the session (from Causeway), `ProjectRun` (open any
  project directory and play it, with test conveniences: keys, pads, tags,
  `on_screen`, board) and `StepReport` (prints, failures, problems, phase
  times; `notes()` and `log()`).
- Editor Play runs on the session (`editor/src/native/runtime/play.rs`).
- `sindri-player` is the generic host: the browser module every export
  ships (`sindri_player.js`) and `sindri-player <project>` natively. Saves are
  keyed per project. `MaybeAudio` plays silently without a sound device.
- Harnesses on `ProjectRun`: platformer, Flappy, Low Tide, Scorchball, the
  Camera Lab (whose own app is gone).

Left in slice 3: Orbital's harness (`games/orbital-baked/src/lib.rs`); Voxel
Lab, which has its own browser app and renderer that Pages builds separately
(`games/voxel-lab/src/browser.rs`); the parity test (the same scripted input
ends in an identical world through the editor's Play session and through
`ProjectRun`); real-browser checks of every export; and docs (`parity.md`,
`capabilities.md`, the `docs/editor-update.md` slice 3 entry).

Gate every push with fmt, warning-denied Clippy and the changed crates'
tests, run unpiped so a failure stops the push. Debug info off keeps the
build inside the disk allowance (`CARGO_PROFILE_DEV_DEBUG=0`,
`CARGO_PROFILE_TEST_DEBUG=0`).

## Notes for later slices

- `sindri-causeway` is the WebAssembly host every export ships
  (`crates/sindri-export/src/page.rs`, `HOST_MODULE`), so its session is
  effectively the engine's player already. Slice 3 moves it rather than
  writing a new one: `game/src/session.rs`, `game/src/session/*` and
  `game/src/styling.rs`.
- Showcase harnesses that hand-assemble a loop today: `games/platformer`,
  `games/scorchball`, `games/low-tide`, `games/flappy`, `games/orbital-baked`
  and `examples/camera`, each in `src/lib.rs`.
- `game/src/styling.rs` explains the in-place presentation the editor should
  adopt in slice 4; `sindri_weave::Presenter` and its `Undo` are the API.
