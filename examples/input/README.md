# Input Actions

A feature example for input actions. Open `sindri.toml` in the editor or run
the exported project in a WebGPU browser. Pages publishes it at
`https://sindri.vardir.no/examples/input/`.

The ship reads what the player means rather than what they pressed. The
scene's `sindri.input.actions` component declares two actions: `move`, a
direction bound to WASD, the arrow keys and a pad's d-pad, and `boost`, a
button bound to Space and a pad's South button. `Action.vector("move")` and
`Action.held("boost")` answer to whichever is being used.

**REBIND BOOST** (or K) waits for the next key or pad button and makes it
boost's first binding with `Action.rebind`; **ADD BOOST KEY** adds one beside
the others. The labels list each action's bindings as `Action.bindings` reads
them, so a rebinding shows up at once. Escape cancels.

All behaviour is in `assets/input.decay`. `crates/sindri-decay/tests/input_demo.rs`
moves the ship, boosts it, rebinds boost to J and boosts with J; the browser
smoke does the same through a page.

See [the scripting contract](../../docs/scripting.md#input-actions).
