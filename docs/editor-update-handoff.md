# Editor update: recovery handoff

## Start here

Work on branch `ccr-4791a75f-7rfx8g` in `vardirhq/sindri-engine`. Keep the
whole update in one pull request; do not split it into several. Read
`AGENTS.md` and `CLAUDE.md` freshly before editing, then
[the update's checklist](editor-update.md), `docs/editor-architecture.md` and
`docs/editor-direction.md`.

The user authorised every item in `docs/editor-update.md`, including the new
`sindri-runtime` crate and slices 6 and 7. Do not ask them to authorise the
scope again. Deliver checked incremental pushes: each slice is one push or
less, with smaller checkpoints preferred.

## Current checkpoint

Planning only. No code has changed. The plan was written while another pull
request's CI was running; before slice 1, bring this branch up to date with
`main` so the update starts from whatever that pull request merged.

## Next

Slice 1, measuring the whole frame. Start from `editor/src/profiler.rs`
(`Phase`) and `editor/src/native/frame.rs`, which is where one editor frame
begins and ends. The headless benchmark should reuse how `editor/tests/`
already opens projects without a window. The standalone side can use the
showcases' existing test harnesses until slice 3 replaces them. Record the
baselines in `docs/editor-update.md` before changing any build profile, so
slice 2 has a before to compare against.

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
