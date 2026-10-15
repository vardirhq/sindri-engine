<!-- Generated from docs/capabilities/input-and-audio.toml by `cargo run -p sindri-capabilities -- --write`. Edit the source, not this file. -->

# Input and audio

Platform-independent input — keyboard, mouse, touch, gamepads and named
actions — and audio playback through a mixer of named buses.

Each entry links its tests and the projects that exercise it. Surface marks match the row in [parity](../parity.md) where one is named: ✅ works and is exercised · 🟡 a useful slice, with the gap named · ❌ absent · — not applicable.

| Capability | Engine | Editor | Decay | Proof |
| --- | :-: | :-: | :-: | :-: |
| **Input** | | | | |
| [Keyboard, mouse, touch behind the platform boundary](#keyboard-mouse-touch) | ✅ | ✅ | ✅ | ✅ |
| [Unified pointer, bounded fingers](#unified-pointer) | ✅ | ✅ | ✅ | ✅ |
| [Gamepad, by player slot](#gamepads) | ✅ | 🟡 | ✅ | ✅ |
| [Action mapping (named actions, rebindable)](#input-actions) | ✅ | 🟡 | ✅ | ✅ |
| **Audio** | | | | |
| [WAV / Ogg / MP3, native + browser + silent backends](#audio-backends) | ✅ | ✅ | ✅ | ✅ |
| [Buses / mixer / master volume](#audio-buses) | ✅ | 🟡 | ✅ | ✅ |
| **Not yet** | | | | |
| [`referenced_audio` gather](#referenced-audio) | ❌ | — | — | — |

## Input

<a id="keyboard-mouse-touch"></a>

### Keyboard, mouse, touch behind the platform boundary

Platform-independent events accumulate into an `InputState` of held, pressed and released keys and buttons.

Engine ✅ · Editor ✅ · Decay ✅ · Proof ✅ — parity row *Keyboard, mouse, touch behind the platform boundary*

**Tested by:** [`crates/sindri-platform/src/input/state_tests.rs`](../../crates/sindri-platform/src/input/state_tests.rs), [`crates/sindri-decay/tests/input_gathers_every_way_in.rs`](../../crates/sindri-decay/tests/input_gathers_every_way_in.rs)

**Exercised in:** [`games/platformer`](../../games/platformer)

<details markdown="1">
<summary>Evidence</summary>

Keyboard, mouse, and touch input arrive as platform-independent events and
accumulate into an `InputState` that answers held, pressed-this-frame, and
released-this-frame for keys and mouse buttons, plus pointer position, pointer
delta, scroll delta, and window focus. `axis(negative, positive)` returns -1,
0, or 1 and gives zero when both are held, so opposed movement keys cannot
resolve by event order.

The editor routes all of it through the Game view during Play, in **that
view's own pixels**, so a script reads the same position there as in the real
build. A pointer outside the view is reported as gone rather than clamped to
its edge, because a game told the person is pointing at somewhere they are not
is worse than a game told they are not pointing at all.

</details>

<a id="unified-pointer"></a>

### Unified pointer, bounded fingers

Touch is kept beside the mouse; `pointer_position` and `pointer_down` unify them where a game reads them.

Engine ✅ · Editor ✅ · Decay ✅ · Proof ✅ — parity row *Unified pointer, bounded fingers*

**Tested by:** [`crates/sindri-platform/src/input/state_tests.rs`](../../crates/sindri-platform/src/input/state_tests.rs), [`crates/sindri-decay/tests/a_script_reads_the_pointer.rs`](../../crates/sindri-decay/tests/a_script_reads_the_pointer.rs)

**Exercised in:** [`examples/physics`](../../examples/physics)

<details markdown="1">
<summary>Evidence</summary>

Touch is held beside the mouse rather than folded into it, because they are
different facts: a mouse has one position and is always somewhere, while
fingers arrive and leave and there may be several. Fingers are bounded at ten,
ordered by the id their host gave them so one keeps its place while it stays
down, and let go of when a window loses focus — a finger cannot be reported as
lifted once the window has stopped hearing about it. What *unifies* the two is
a separate question and is answered where a game reads it: `pointer_position`
is the mouse if there is one and the first finger otherwise, and
`pointer_down(Left)` is the left button or any finger.

</details>

<a id="gamepads"></a>

### Gamepad, by player slot

Desktop and browser gamepads, read by player slot with join/leave edges, positional buttons and round dead zones.

Engine ✅ · Editor 🟡 · Decay ✅ · Proof ✅ — parity row *Gamepad, by player slot*

**Tested by:** [`crates/sindri-platform/src/input/gamepad_tests.rs`](../../crates/sindri-platform/src/input/gamepad_tests.rs), [`crates/sindri-decay/tests/a_script_reads_a_gamepad.rs`](../../crates/sindri-decay/tests/a_script_reads_a_gamepad.rs)

**Exercised in:** [`games/scorchball`](../../games/scorchball)

<details markdown="1">
<summary>Evidence</summary>

Gamepads are read on desktop and in the browser (`GamepadReader`, over
`gilrs`, behind the platform's default `gamepad` feature, which links libudev
on Linux the way audio links ALSA). Every host that draws a window asks for
them once a frame, and so does the editor's Play. A game reads pads by **player
slot** rather than by device: a face button or Start on an unclaimed pad claims
the first free slot, a slot stays claimed while its pad is connected and is
given back when it goes, and joins and leaves are one-frame edges that never
name two slots at once, so a second pad pressing in the same frame joins the
next. The press that claims a slot is not reported through it, so joining does
not also jump. Slot 0 reads every pad, for a game with one player. Buttons are
named by position (`south`, `right_bumper`, `dpad_up`), sticks read in screen
axes with a round dead zone taken out, and focus loss lets go of every button
without dropping a player. Scripts read `Gamepad`; an action binding reads any
pad as `gamepad.south` or `gamepad.axis.left_x`.

</details>

<a id="input-actions"></a>

### Action mapping (named actions, rebindable)

A scene declares named actions; scripts read them by name and rebind them while the game runs.

Engine ✅ · Editor 🟡 · Decay ✅ · Proof ✅ — parity row *Action mapping (named actions, rebindable)*

**Tested by:** [`crates/sindri-platform/src/input/action/map.rs`](../../crates/sindri-platform/src/input/action/map.rs), [`crates/sindri-platform/src/input/action/state.rs`](../../crates/sindri-platform/src/input/action/state.rs), [`games/platformer/tests/a_run_reaches_the_flag.rs`](../../games/platformer/tests/a_run_reaches_the_flag.rs), [`game/tests/the_mixer_and_input_demos_work.rs`](../../game/tests/the_mixer_and_input_demos_work.rs)

**Exercised in:** [`games/platformer`](../../games/platformer), [`examples/input`](../../examples/input)

**Limits:**

- Actions are per scene, not per project or per player.
- A rebinding is not saved between sessions.

<details markdown="1">
<summary>Evidence</summary>

Input actions are declared by a scene's `sindri.input.actions` component, in
the actions-document shape `ActionMap` reads. The script runner reads the
declaration when it changes and every action's value from the step's input
before any script runs; `Action.held`, `pressed`, `released`, `axis` and
`vector` answer by name, and `Action.bindings`, `Action.rebind` and
`Action.last_pressed` rebind one while the game runs, written back into the
component. The platformer's hero runs and jumps by actions, and
`examples/input` rebinds its boost (also covered by the browser smoke).

</details>

## Audio

<a id="audio-backends"></a>

### WAV / Ogg / MP3, native + browser + silent backends

Validated audio assets play through Rodio/CPAL natively, media elements in the browser, and a recording silent backend in tests.

Engine ✅ · Editor ✅ · Decay ✅ · Proof ✅ — parity row *WAV / Ogg / MP3, native + browser + silent backends*

**Tested by:** [`crates/sindri-platform/src/audio/tests.rs`](../../crates/sindri-platform/src/audio/tests.rs), [`editor/src/play_audio/tests.rs`](../../editor/src/play_audio/tests.rs), [`scripts/browser/smoke.mjs`](../../scripts/browser/smoke.mjs)

**Exercised in:** [`examples/audio`](../../examples/audio)

<details markdown="1">
<summary>Evidence</summary>

Audio is a platform service rather than simulation state. Encoded WAV, Ogg, and
MP3 assets are identified and validated by the asset layer, then registered
with an `AudioBackend`. Native hosts use Rodio/CPAL, browser hosts use media
elements with an explicit user-interaction unlock, and headless tests use a
silent backend that records every request without needing a sound device. The
shared boundary supports one-shot and looping playback plus per-voice stop and
global pause, resume, and stop.

Scenes can author `sindri.audio.source` with a logical clip ID, autoplay,
looping, and a normalized volume. The generic component inspector can add and
edit it, and the project browser recognises WAV/Ogg/MP3 files as audio assets.
Decay exposes typed `Audio.play`, `loop`, `stop_all`, `pause_all`, and
`resume_all` calls while only emitting intent, so the language itself retains
its no-I/O boundary. Gather exercises the path end to end with background,
pickup, and victory sounds. The looping background music has been observed
playing in a real browser through `scripts/browser/smoke.mjs`, which fails if a
clip a page asked for did not play. The editor can audition a selected clip
through its own audio backend.

</details>

<a id="audio-buses"></a>

### Buses / mixer / master volume

Every voice plays through a named bus under `master`; scripts move and read bus volumes.

Engine ✅ · Editor 🟡 · Decay ✅ · Proof ✅ — parity row *Buses / mixer / master volume*

**Tested by:** [`crates/sindri-platform/src/audio/mixer.rs`](../../crates/sindri-platform/src/audio/mixer.rs), [`games/orbital-baked/tests/the_mixer.rs`](../../games/orbital-baked/tests/the_mixer.rs), [`game/tests/the_mixer_and_input_demos_work.rs`](../../game/tests/the_mixer_and_input_demos_work.rs)

**Exercised in:** [`games/orbital-baked`](../../games/orbital-baked), [`examples/audio`](../../examples/audio)

**Limits:**

- There are no bus effects, ducking or snapshots.

<details markdown="1">
<summary>Evidence</summary>

`AudioMixer` in `sindri-platform` sits in front of any backend: it starts a
voice at its own volume times its bus's times the master's, and
`set_bus_volume` re-applies the gain to every live voice the bus reaches, which
each backend supports (`set_volume`, `is_active`). Decay's `Audio.play` routes
to `effects`, `Audio.loop` to `music`, `play_on`/`loop_on` to any bus, and
`Audio.set_volume` and `Audio.volume` move and read a bus; an authored source
names its `bus` or follows its looping. The game session routes scripts'
requests and autoplay through the mixer. Orbital's pause screen has Master,
Music and Effects sliders, saved between sessions, and `examples/audio` is the
Sound Mixer feature example.

</details>

## Not yet

<a id="referenced-audio"></a>

### `referenced_audio` gather

Nothing gathers the clips a scene names, so dynamically named clips must be listed in `[assets].include`.

Engine ❌ · Editor — · Decay — · Proof — — parity row *`referenced_audio` gather*

**Limits:**

- The editor does not preview a scene's audio source in place.
