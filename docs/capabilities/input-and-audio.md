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
| [Sound effects made in the Sound Lab](#sound-lab) | — | ❌ | — | ✅ |
| **Not yet** | | | | |
| [`referenced_audio` gather](#referenced-audio) | ❌ | — | — | — |
| **3D foundation** | | | | |
| [Mouse displacement for 3D look](#mouse-look-delta) | 🟡 | 🟡 | 🟡 | 🟡 |
| [Pointer lock](#pointer-lock) | 🟡 | 🟡 | 🟡 | 🟡 |

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

Validated audio assets play through Rodio/CPAL natively, Web Audio in the browser, and a recording silent backend in tests.

Engine ✅ · Editor ✅ · Decay ✅ · Proof ✅ — parity row *WAV / Ogg / MP3, native + browser + silent backends*

**Tested by:** [`crates/sindri-platform/src/audio/tests.rs`](../../crates/sindri-platform/src/audio/tests.rs), [`editor/src/play_audio/tests.rs`](../../editor/src/play_audio/tests.rs), [`scripts/browser/smoke.mjs`](../../scripts/browser/smoke.mjs)

**Exercised in:** [`examples/audio`](../../examples/audio)

<details markdown="1">
<summary>Evidence</summary>

Audio is a platform service rather than simulation state. Encoded WAV, Ogg, and
MP3 assets are identified and validated by the asset layer, then registered
with an `AudioBackend`. Native hosts use Rodio/CPAL; browser hosts decode each
clip once into a Web Audio buffer and play it through one `AudioContext`,
unlocked by the first key, click or touch — resumed inside the gesture's own
handler, which iOS Safari requires, and on a finger lifting as well as landing,
so a phone player holding the platformer's stick still hears it. Headless tests
use a silent backend that records every request without needing a sound device. The
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

<a id="sound-lab"></a>

### Sound effects made in the Sound Lab

A standalone tool designs seeded game sound effects and delivers each into a project, listed for export, with the line that plays it.

Engine — · Editor ❌ · Decay — · Proof ✅ — parity row *Sound effect authoring (Sound Lab)*

**Tested by:** [`tools/sindri-chiptune/src/lab/sfx/tests.rs`](../../tools/sindri-chiptune/src/lab/sfx/tests.rs), [`tools/sindri-chiptune/src/lab/project.rs`](../../tools/sindri-chiptune/src/lab/project.rs), [`games/platformer/tests/a_run_reaches_the_flag.rs`](../../games/platformer/tests/a_run_reaches_the_flag.rs)

**Exercised in:** [`games/platformer`](../../games/platformer)

**See:** [`tools/sindri-chiptune/README.md`](../../tools/sindri-chiptune/README.md)

<details markdown="1">
<summary>Evidence</summary>

`tools/sindri-chiptune` (the Sindri Sound Lab) has a Sound FX designer beside
its music workstation. Eight kinds — coin, jump, laser, explosion, power-up,
hurt, blip and thud — are seeded regions of one synth voice: a wave, an
attack/sustain/punch/decay envelope, a pitch path with slide, curve, vibrato,
arpeggio and repeat, a square duty sweep, low- and high-pass filters and a
sample-and-hold crush. The same kind and seed always render the same samples.
Every parameter is a slider with a unit, a change replays the sound, and
**Mutate** nudges each by a few percent.

**Send to project** writes `assets/audio/<name>.wav` and the editable
`<name>.sindrisfx` beside it, adds the clip to `sindri.toml`'s
`[assets] include` (editing the manifest in place, comments kept), and offers
`Audio.play("audio/<name>.wav", 0.5);` to paste. Without that include line a
clip a script plays is found by the editor and silently left out of an
export. The same delivery runs headless:
`sindri-chiptune sfx coin --seed 5 --project games/platformer --name coin`.

The platformer's hero plays four sounds made this way — jump, landing, coin
and the flag — and its scripted run to the flag asserts it heard each one and
that each is listed for export. The tool's own tests re-render every committed
`.sindrisfx` and compare it with the WAV beside it, so reopening one edits
what ships.

The same Lab runs in a browser at `/examples/sound-lab/`, built by Pages with
wasm-pack and drawn with WebGL2. There **Send** becomes **Download** of the
same two files with the lines to add, and under 720 points wide the window
folds into one column for a phone. `scripts/browser/sound-lab.mjs` opens it at
desktop and phone sizes in both modes on every Pages build. The tool is not
part of the editor.

</details>

## Not yet

<a id="referenced-audio"></a>

### `referenced_audio` gather

Nothing gathers the clips a scene names, so dynamically named clips must be listed in `[assets].include`.

Engine ❌ · Editor — · Decay — · Proof — — parity row *`referenced_audio` gather*

**Limits:**

- The editor does not preview a scene's audio source in place.

## 3D foundation

<a id="mouse-look-delta"></a>

### Mouse displacement for 3D look

Read-only `Pointer.delta` / `Input.Pointer.delta` returns accumulated viewport-pixel mouse movement once per fixed step, positive right/down

Engine 🟡 · Editor 🟡 · Decay 🟡 · Proof 🟡 — parity row *Mouse displacement for 3D look*

**Tested by:** [`crates/sindri-decay/tests/a_script_reads_the_pointer.rs`](../../crates/sindri-decay/tests/a_script_reads_the_pointer.rs)

**Exercised in:** [`examples/orbit`](../../examples/orbit)

**See:** [`docs/scripting.md`](../../docs/scripting.md)

<details markdown="1">
<summary>Evidence</summary>

`Pointer.delta` and `Input.Pointer.delta` expose the existing platform mouse
movement accumulator as a read-only `Vec2`, including component reads. Movement
is in viewport pixels, positive right/down, spent once per fixed step; a first
arrival/re-entry adds no jump. It is displacement rather than velocity, and
independent of touch (`Gesture` remains the touch drag surface). Compiler and
runtime reject writes, while a copied vector stays mutable. Bridge regressions
exercise accumulated movement, reset, re-entry and both namespaces; LSP tests
check completion/hover from the same host metadata.

Orbit Camera Lab uses it for right-button drag-to-look, with sensitivity and
pitch limits in Decay. Its Session regression checks yaw/pitch changes, no
repeated displacement on the next step and pitch clamping. Real WebGPU
Chromium drove a right-button drag through the exported generic player; a
console observer in a disposable copy recorded exactly 60 pixels right and 40
down, once. Assets loaded, the walker reached its goal and inspected captures
showed the camera response without runtime/GPU errors. The flat-color lab used
the same local eight-color smoke threshold as its earlier capture (fourteen
colors drawn); the repository browser gate is unchanged. This is feature
example evidence; native editor Play also exercises captured delta. Explorer
game proof remains pending.
Native/browser capture is described below.

</details>

<a id="pointer-lock"></a>

### Pointer lock

`Pointer.lock()` / `unlock()` and read-only `locked`, also under `Input.Pointer`, connect Session to native/browser and native editor Play capture

Engine 🟡 · Editor 🟡 · Decay 🟡 · Proof 🟡 — parity row *Pointer lock*

**Tested by:** [`crates/sindri-platform/tests/pointer_motion.rs`](../../crates/sindri-platform/tests/pointer_motion.rs), [`scripts/browser/pointer-lock.mjs`](../../scripts/browser/pointer-lock.mjs)

**Exercised in:** [`examples/orbit`](../../examples/orbit)

**See:** [`docs/platform-host.md`](../../docs/platform-host.md)

<details markdown="1">
<summary>Evidence</summary>

The shared platform input state accepts `PointerLockChanged` as actual host
capture feedback and `PointerMotion` as relative mouse displacement. These are
platform events, not Decay APIs or capture requests. Relative motion is accepted
only while captured and focused; cursor warps and leave events do not add motion
or move/cancel UI presses during capture. Changing capture mode discards pending
motion and rebases the next absolute position. Focus loss clears capture, pending
motion and the absolute mouse position; focus regain does not recapture.
Nonfinite motion and overflowing sums are rejected atomically.

`crates/sindri-platform/tests/pointer_motion.rs` exercises these transitions,
invalid input and once-only consumption through the real `EngineHost` fixed-step
loop, including frames with no step and catchup steps. Native/browser player
hosts now emit this feedback and motion, as described below.

`Pointer.lock()` / `Input.Pointer.lock()` request capture; `unlock()` requests
release and read-only `locked` reports actual state. No request changes state
within an invocation. Successful invocation outputs collect requests in execution
order (last request wins), Session queues the latest across fixed steps, and
`DesktopApp` drains it once after update. Failed invocations do not forward their
capture intent; checkpoint restore does not replay window commands.

The windowed host uses native Locked, with X11 confinement plus raw device
motion as a fallback. Browser capture calls are caught at the JavaScript boundary
and Promise rejection is consumed; actual document lock is polled before gameplay
and relative motion. Delayed grants after explicit release are released again.
Escape, focus loss, suspension, hidden pages, failure and normal exit release
capture. Returning focus does not recapture. Motion uses native raw device counts
or browser physical pixels, positive right/down, through the existing delta API.

Bridge regressions cover actual-state reads, both namespaces, read-only writes,
invalid call signatures and failed-invocation intent. Orbit Camera Lab's Session
regression covers request draining, relative camera motion and checkpoint restore.
LSP tests cover read-only boolean metadata and capture-call completion; generated
reference/metadata share the same typed host surface.

`scripts/browser/pointer-lock.mjs` runs the exported lab in real WebGPU Chromium:
a trusted click captures, movement beyond the viewport reaches Decay as exactly
1,420 horizontal/60 vertical units once, U and Escape release, and a sandboxed
frame denies capture without false success or interrupting the goal run. The
captured-look screenshot was inspected. CI exports the same project and runs
this regression, retaining its screenshot. Its flat-color fixture uses an
eight-color minimum; the general browser smoke threshold remains unchanged.
Native X11 verification runs the same generic player in an isolated Xvfb
window on Vulkan, reaches the goal, and drives capture/raw motion/U/Escape/focus
changes with XTest. Decay observes 4,000 horizontal/120 vertical raw units in
one step, releases on each requested/focus transition and does not recapture
on focus regain. The native window screenshot was inspected alongside Chromium.
Native platforms other than this X11 run were not visually exercised here.
Native editor Play also drains Session requests after stepping and editor
transport/layout changes. Native window calls report success before input sees
capture; missing windows cannot report false success. egui raw mouse events
feed native counts without display scaling; the shared state ignores absolute
warps. Unit regressions cover accumulation across frames, once-only consumption,
release rebasing, stopping and absent windows. The actual X11 editor window runs
Orbit Camera Lab to its goal, reports 6,000 horizontal/120 vertical raw units
from an unbounded XTest move, and releases on U/Escape, pause/stop, focus loss
and hiding the Game view. An external pointer-grab probe verifies actual capture
and release; resume/focus regain/new Play do not automatically recapture. The
editor screenshot with Decay motion and goal output was inspected. Other native
editor platforms and Explorer game proof remain pending.

</details>
