# Tween Lab

A feature example for managed Decay gameplay tweening. Open `sindri.toml` in
the editor or run the exported project in a WebGPU browser. Pages publishes it
at `https://sindri.vardir.no/examples/tween/`.

Five rows compare linear, ease, ease-in, ease-out and ease-in-out timing while
moving, scaling, rotating and changing colour. Buttons work with mouse/touch;
Space resumes, P pauses, R restarts, C cancels, and V replaces the destination
from the displayed value. Cancellation holds the value; use Restart to replay.
The first pass runs automatically and holds at completion.

The sixth row composes tweens: an orange marker crosses, waits half a second
and comes back, because its return is `Tween.after` the crossing with a
`Tween.set_delay`, while a `Tween.set_yoyo` scale that `Tween.set_loops` plays
for ever makes it breathe. The same controls pause, resume, cancel and restart
it.

Labels and controls use `assets/ui/tween.weave`: pixel font sizes keep desktop
text readable, and media rules increase them for taller screens. The scene
provides the fallback presentation.

The project uses the ordinary browser export host. All demo behaviour is in
`assets/scripts/tween-demo.decay`; no bespoke Rust game loop or JavaScript
animation implements its tweens. The source is small enough to copy into a game.
`crates/sindri-decay/tests/tween_demo.rs` opens, validates, compiles and plays it;
CI also exports and runs it through the browser smoke on desktop and portrait
viewports, observing animation, pause and each playback control.

See [the gameplay contract](../../docs/scripting.md#gameplay-tweens). Weave's
CSS-style UI animation authoring is a separate surface sharing easing math.
