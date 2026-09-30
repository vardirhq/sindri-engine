# Codex handoff: UI widgets, demo, and Orbital redesign

## User request and checkpoint status

Chris authorized implementing the missing UI widgets, creating a demo page for
them, and making Orbital Last Stand's UI look substantially better. He then
explicitly requested that the current work be pushed and instructions written
for Codex to finish. This branch is that **unfinished checkpoint**, not a ready
feature. Do not merge it or describe it as working until validation is complete.

Continue on `chatgpt/ui-widgets-checkpoint`, based on main at
`d716c74e96bd55656ad01dbe6aed1a94a1dbce79`. Read AGENTS.md and CLAUDE.md first.
Read docs/decay-agent-guide.md before editing Decay or its host APIs. No Rust
toolchain was available in the originating connector workspace. Nothing here
has been compiled or run. Only static inspection and JSON parsing were possible.

## What is written

- Scene widgets: `UiToggleComponent`, `UiTextInputComponent`,
  `UiScrollComponent`, registered through `extract/widget_registry.rs` and
  exported from sindri-scene. They use ordinary components and inspector fields.
  Toggle and checkbox share boolean interaction semantics; their art is authored.
- `ScreenUi::read_controls` and host-neutral `UiInput`: tab navigation, toggle
  activation, end-of-line Unicode input/backspace, enter submission, wheel
  scrolling, and press-identity tracking for scroll drags. Values are written
  into the world's component payloads. `changed` and `submitted` are per-step.
- `UiHierarchy` applies scroll offsets and intersects nested ancestor clip
  rectangles. Hit testing and UI shape/text/image extraction share clips.
- `FramePass::clip` adds a generic pixel scissor. The sprite, glyph and shape
  renderers apply it. Batches break at clip boundaries; empty scissors skip draws.
- InputState has a separate committed-character stream (TextInput(char)),
  translated by the desktop/browser winit host and editor input adapter.
  sindri-decay's `ui_input` converts keyboard/scroll input to scene UiInput.
  Calls were added to the shared game Session, editor runtime, and Orbital Run.
- Decay API: `Ui.is_checked`, `set_checked`, `input_text`, `set_input_text`,
  `scroll_offset`, `set_scroll_offset`, `changed`, `submitted`, `is_focused`.
  The typed environment, host dispatch and reference entries are updated.
- Weave reads the input component's runtime `focused` flag for `:focus`.
- Initial scene tests cover Unicode limits/backspace, tab ordering, disabled
  toggles, change/submit lifetime, despawned focus, and a hidden scroll child.
- `examples/ui` is a scene/Decay/Weave **scaffold** called Weave Control Room.
  It has flight settings, a callsign input, and an 18-module scroll archive.
  Font bytes are reused from the existing Chakra Petch asset; license copied.

**Orbital's visual redesign has NOT been implemented.** Its existing Weave,
scene and gameplay are unchanged except the host's widget input call.
There is no demo export/Pages/browser wiring or demo runtime regression yet.
Capability docs, changelog, generated API documents and final visual QA remain.

## First: compile and audit the checkpoint

Use a full checkout with the Rust toolchain. Run scripts/preflight.py, format,
check and focused tests for sindri-platform, sindri-render, sindri-scene,
sindri-decay, sindri-desktop, sindri-weave, editor, sindri-causeway and
orbital-baked as applicable. Check the workspace package names in Cargo.toml.
Run the complete changed Decay set through the real typed checker.

Check every CI failure before repairing, not only the first. The automatic
formatter/generator may commit; fetch fresh branch files before later edits.
Do not hand-edit docs/generated or widen the autofix workflow.

Audit these concrete risks before building further:

1. Pedantic Clippy and file sizes after rustfmt. New inline code is compact and
   will expand. Several existing files were already near 600 lines; split by
   responsibility where necessary. Newly enlarged functions may exceed 100
   lines. Check type-complexity, doc/#[must_use] and imports as well.
2. Widget defaults and payload validation: reject invalid content heights,
   lengths, offsets; ensure malformed data fails explicitly through checked
   authoring. Setter behavior should match user interaction. Decide whether
   scripted setters should emit change events; current intent is that they do not.
3. Focus/click lifetime: control input must be spent once per fixed step, not
   replayed by several rendered frames. Clear focus after disable/despawn, on
   window blur, and on outside clicks. Update focus styling for buttons/toggles
   as well as text inputs; current focused payload flag only covers text inputs.
   Hidden clipped rows must not receive keyboard focus. Gamepad navigation is
   not wired; add it if claimed, otherwise document it as remaining.
4. Scroll drags: current detection waits until movement exceeds 0.025 overlay
   units. Ensure a drag over a row cancels its click, tracks its own finger,
   respects clipping and disabled ancestors, and handles cancellation/nesting.
   Review wheel nesting and propagation. Set `captures_pointer` correctly while
   dragging outside the viewport. Update scroll geometry after input before
   hit testing the next click; styled hosts use the last drawn layout.
5. Text input currently edits at the end only. Selection, caret navigation,
   clipboard, IME preedit and mobile software keyboard activation do not exist.
   Either implement what the demo promises or clearly document the slice.
   Avoid double commits from KeyEvent.text and Ime::Commit, and prevent modifier
   shortcuts from entering text. Winit IME activation/focus is not wired.
   Text input should prevent gameplay hotkeys from firing while editing.
6. Clipping: validate scissors against the physical target at high DPI and in
   editor Scene/Game views. Preserve drawing order within each layer when clips
   split batches (shapes/text currently use BTreeMap keys containing clip).
   Test text, sprites, shapes and shadows crossing edges, nested clips and
   zero-area clips. Rotated scroll regions currently use axis-aligned bounds;
   document or reject unsupported rotation rather than silently claiming it.
7. Check schema-derived generated docs and exhaustive matches/tests affected by
   the additional UiCall variants and InputEvent variant. Add platform text
   stream tests, host translation tests and Decay API regressions.

## Finish the feature example and publish wiring

Complete examples/ui with working wide and portrait layouts. The archive's
authored first-row Y is currently 0.32 while the portrait viewport is 0.43 tall;
fix this so the first item is visible at offset zero and scrolling never shows
an empty tail. Content height must agree with the actual laid out content.
Add disabled examples and visible focus/checked feedback. Keep example behavior
in Decay; Weave owns geometry, typography and states.

Add a native regression opening the project, compiling all scripts, and driving
toggle, checkbox, Unicode input/submission, wheel/touch scroll and row selection.
Include clamp, cancellation, no-click-through and clipped hit checks.

Mirror examples/physics wiring in .github/workflows/ci.yml and pages.yml:
export examples/ui at `/sindri-engine/examples/ui/` and `/examples/ui/`, run real
desktop/phone browser smokes with screenshots, verify the route, and add a card
to site/index.html and site/directory.json. A small browser helper should test
actual interactions and console evidence, not merely a painted canvas.

## Redesign Orbital Last Stand

The target is `games/orbital-baked`, the maintained flagship; the original vector
variant remains a reference. Read its scene, ui.weave and ui/{hud,overlays,screens}.weave,
title.decay, hud.decay, upgrade-chooser.decay and existing presentation tests.

Make a cohesive premium sci-fi UI: restrained dark panels, cyan/teal primary
accents, warm danger accents, crisp readable typography, intentional spacing,
strong hierarchy and hover/active/focus feedback. Preserve the existing logo.
Make this a substantial redesign, not just changing colors.

- Title: separate identity/progress from play-mode actions. Start should clearly
  dominate Boss Rush and the starting-boss picker. Explain movement concisely.
- HUD: clearly separate sector/wave, survival clock, score/level and HP/XP. Fix
  the current HP/XP label overlap and use backed meter groups that remain legible
  over combat. Give boss health an unmistakable but compact danger hierarchy.
- Upgrades: readable names/descriptions, distinct card structure and interaction
  states. Accommodate 3/4 offers and portrait without truncation or off-screen
  choices. Keep module selection behavior in Decay.
- Pause/results: clear primary/secondary actions, meaningful score breakdown,
  strong result hierarchy, and touch-sized targets.
- Prove the new widgets in the game: for example a compact-HUD toggle with a
  real effect, an editable pilot callsign shown in the run/results, and a
  scrollable module/settings/controls area. Do not add a volume control that
  pretends to work without audio buses/master volume (those are still absent).

Existing presentation tests assert old dimensions. Update tests to verify
containment, separation, readable bounds and pointer targets as well as the
new intended geometry. Keep all game regressions green. Check
tests/the_hud_is_laid_out_once.rs: the native Run does not always use styled
pixel coordinates, so don't blindly replace its authored-band guarantee.

Capture and inspect title, gameplay HUD (including boss), upgrade choices,
pause and results on desktop and phone. Check at 960x540, 1280x720, 390x844,
short landscape and a wide portrait device. Iterate from actual screenshots.

## Final delivery gate

Update docs/ui-direction.md, weave.md/reference as needed, scripting.md,
capabilities.md, parity.md and CHANGELOG.md. Split widget parity honestly: do not
mark dropdown/radio/tabs/IME/gamepad complete just because this slice shipped.
Regenerate API docs. The UI demo proves the feature in isolation; Orbital must
prove it in a real game. Run required native, Decay, Clippy, WASM, browser,
render and Pages checks. Review the complete diff and final-head CI before
marking the draft ready. Leave merging to Chris unless he explicitly asks.
