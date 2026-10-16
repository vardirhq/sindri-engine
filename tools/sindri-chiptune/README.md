# Sindri Sound Lab

Game sound effects and chiptune music, made in one window and sent straight
into a Sindri project, ready for a script to play.

```sh
cargo run -p sindri-chiptune          # open the Sound Lab
```

Started inside a project folder, the Lab sends there without asking; otherwise
**Project** in the top bar picks one. It is a standalone native tool: it does
not depend on the editor or the runtime, and the editor does not embed it.

## Sound FX

Pick a kind (**Coin, Jump, Laser, Explosion, Power-up, Hurt, Blip, Thud**) and
each click is a new variation, played at once. The kinds are seeded regions of
one synth voice, so the same kind and seed always render the same samples.

- **Shape it.** Every parameter is a slider with a unit: the wave (square,
  triangle, saw, sine, noise) and its duty sweep; attack, sustain, punch and
  decay; the starting pitch, slide, curve and a falling cutoff that ends the
  sound; vibrato, an arpeggio jump and a repeat; low-pass, high-pass, crush and
  volume. A change replays the sound when the drag ends.
- **Explore around it.** **Mutate** (M) nudges every parameter a few percent;
  **New variation** (R) draws another sound of the kind; **Recent** keeps the
  last fourteen to go back to.
- **Use it.** **Send to project** writes `assets/audio/<name>.wav` and its
  editable `<name>.sindrisfx` beside it, adds the clip to `[assets] include` in
  `sindri.toml` (editing the manifest in place, comments kept), and shows the
  line to paste: `Audio.play("audio/<name>.wav", 0.5);`. The include line
  matters: a clip a script names is invisible to an export's walk of the
  scene, and without it the sound plays in the editor and is missing from the
  shipped game.

A `.sindrisfx` is a small JSON file: the kind and seed it started from, and
every parameter by name. **Open** brings one back to reshape and send again.

## Music

The workstation composes 32-bar chiptune songs in four voices.

1. Pick a mood, key, density and seed, then **Generate A + B**. The two
   candidates are cached; drafts never change the song that is playing.
2. **Play** (Space), loop, jump back, or click the song overview to go to a
   section and seek. The oscilloscope and meter in the top bar show the live
   output.
3. Edit Bass, Arpeggio or Lead in the piano roll: click to add, drag to move,
   right-click or Delete to remove, arrows to nudge (Shift+arrows by an
   octave). The other two voices are drawn faintly behind, and brightness is
   velocity. One voice per part: an overlapping edit is refused and rolled
   back. Undo/redo (Ctrl+Z / Ctrl+Y) keeps 64 edits.
4. Change sounds, levels, mute/solo and octave in the mixer while it plays.
5. **Save** (Ctrl+S) writes a `.sindrisong` holding the edited notes, tempo,
   key, mood and mixer; **Open** reopens it. **Send to project** writes the
   rendered song and its `.sindrisong`, and offers
   `Audio.loop("audio/<name>.wav", 0.6);`.

Four styles use major, minor, Dorian or Phrygian harmony, with chords stacked
in the style's scale, a seeded hook that resolves accents to chord tones, a
question/answer phrase, cadence breaths and a tonic ending. Percussion is
generated from mood and density. These are software interpretations of chip
sounds, not cycle-accurate emulations.

MIDI export writes format 1 at 480 ticks per quarter: tempo, 4/4 and the three
tonal parts. MIDI import accepts format 0/1 with constant 60–220 BPM, 4/4 and up
to three monophonic tonal channels within 32 bars, quantized to sixteenths;
polyphony, percussion, pitch bends, sustain and tempo changes are refused with a
reason.

## Headless

```sh
sindri-chiptune sfx coin --seed 5 --output coin.wav --source coin.sindrisfx
sindri-chiptune sfx --open coin.sindrisfx --output coin.wav
sindri-chiptune sfx jump --seed 4 --project games/platformer --name jump
sindri-chiptune song --seed 42 --mood hopeful --output theme.wav
sindri-chiptune song --open theme.sindrisong --project my-game --name theme
```

`--project` does exactly what the window's Send does. `--help` lists every
option. Linux builds need ALSA headers.

## Proof

The platformer's jump, landing, coin and flag sounds were made and sent with
this tool (`games/platformer/assets/audio/`). Its scripted run to the flag
asserts it heard all four and that each is listed for export, and this crate's
tests re-render every committed `.sindrisfx` and compare it with the WAV
beside it.

## Checks

```sh
cargo fmt --all --check
cargo clippy -p sindri-chiptune --all-targets --all-features -- -D warnings
cargo test -p sindri-chiptune --all-features
cargo test -p platformer --test a_run_reaches_the_flag
cargo check -p sindri-chiptune --all-features --target wasm32-unknown-unknown
```

The tests cover every effect kind over many seeds (deterministic, audible,
bounded, click-free endings), pitch direction per kind, mutation, `.sindrisfx`
and `.sindrisong` round trips and refusals, project delivery and manifest
editing, in-key monophonic composition, MIDI round trips and malformed input,
undo/redo, live tempo and edit continuity, and drawing both modes at three
window sizes. Layouts were captured from the real window under Xvfb.

Not yet: hearing it through a real audio device in CI, a browser build, music
short enough to ship (a full song is several megabytes of WAV), an editable
drum score, and an editor panel.
