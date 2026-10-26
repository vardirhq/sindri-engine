# Sindri Sound Lab

Game sound effects and chiptune music, made in one window and sent straight
into a Sindri project, ready for a script to play.

```sh
cargo run -p sindri-chiptune          # open the Sound Lab
```

Started inside a project folder, the Lab sends there without asking; otherwise
**Project** in the top bar picks one. It is a standalone tool: it does not
depend on the editor or the runtime, and the editor does not embed it.

## In a browser

The same Lab runs on the site at
[`/examples/sound-lab/`](https://sindri.vardir.no/examples/sound-lab/)
(`?mode=sfx` opens on the effects designer). A browser cannot reach a project
folder, so **Send to project** becomes **Download**: the WAV and its editable
source, named for `assets/audio/`, and the lines to add to `sindri.toml` and to
a script. Open, Save, WAV and MIDI export are downloads and file pickers.

It draws with WebGL2 rather than WebGPU: the Lab is a flat interface, and Chrome
on Android exposes WebGPU more widely than its drivers can serve. Under 720
points wide it folds into one scrolling column with larger touch targets, so it
works on a phone: tap to add a note, long-press to delete one. Audio goes
through Web Audio and starts on the first tap, as browsers require.

```sh
wasm-pack build tools/sindri-chiptune --target web --release --out-dir pkg
mkdir -p target/sound-lab && cp tools/sindri-chiptune/web/index.html target/sound-lab/
cp -R tools/sindri-chiptune/pkg target/sound-lab/
node scripts/browser/sound-lab.mjs target/sound-lab   # desktop and phone, both modes
```

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

The workstation composes 32-bar chiptune songs: bass, arpeggio and lead you can
edit, chords and drums that follow them, mixed in stereo.

1. Pick a mood, key, density and seed, then **Generate A + B**. The two
   candidates are cached; drafts never change the song that is playing.
   **Keep on Generate** locks a part in that candidate so the next generation
   preserves it. **Regenerate track** (R) replaces only the selected part,
   using the same harmony and keeping the other two parts. Locks are session state;
   the existing `.sindrisong` format is unchanged. While any track is locked,
   Generate keeps the current seed, key and mood so replacement parts stay in
   the same harmonic context; unlock all tracks to apply new harmony settings.
   On Lead, **Vary melody** (V) changes selected pitches in the current melody
   while keeping its rhythm, repeated phrases, and resolving note. The variation
   can be undone like other score edits.
2. **Play** (Space), loop, jump back, or click the song overview to go to a
   section and seek. The oscilloscope and meter in the top bar show the live
   output. The named Bass, Arpeggio and Lead rows select the piano-roll voice;
   each row also has its own generation lock and quick mute/solo controls.
3. Edit Bass, Arpeggio or Lead in the piano roll: click to add, drag to move,
   right-click or Delete to remove, arrows to nudge (Shift+arrows by an
   octave). The other two voices are drawn faintly behind, and brightness is
   velocity. One voice per part: an overlapping edit is refused and rolled
   back. Undo/redo (Ctrl+Z / Ctrl+Y) keeps 64 edits.
4. Change sounds, levels, mute/solo and octave in the mixer while it plays,
   and how much **Room & echo** the whole mix has.
5. **Save** (Ctrl+S) writes a `.sindrisong` holding the edited notes, tempo,
   key, mood and mixer; **Open** reopens it. **Send to project** writes the
   rendered song and its `.sindrisong`, and offers
   `Audio.loop("audio/<name>.wav", 0.6);`.

### Writing

A song is written to repeat, so it sounds like one piece rather than a run of
random bars. Each section loops four chords, one per bar, and the melody is a
period: a one-bar motif built on chord tones at the strong beats, carried
through the loop with stepwise voice-leading, ending each phrase on a cadence
that asks a question (resting on the fifth) or answers it (home on the root).
The theme comes back note for note in the return, the bass plays one riff for
the whole song, and the drums keep one beat that repeats every bar, with a fill
at the end of each phrase.

Variety comes from what is chosen, per song, by the seed. Four moods set major,
minor, Dorian or Phrygian harmony; each draws its verse and rise loops from its
own library of progressions, and a groove from those that suit it: **Drift,
Bounce, Drive, Ballad, Shuffle** (triplet swing) or **Half-time**. The groove
picks the drum beat from its own set, the motif's rhythm (curated, or generated
from the groove's onsets), the bass riff, cadence shapes, and the swing, which
moves every part, not only the hats. The register, contour direction and
arpeggio figure are drawn too, so two seeds of one mood can differ in beat,
chords and tune.

The arpeggio changes with the section: sparse in the intro, a seeded figure in
the theme, a sixteenth-note run in the rise. The **chords** are read from the
bass line at each downbeat, stacked in the song's scale (with sevenths in the
darker moods) and voiced to move as little as possible, so they follow every
bass edit.

### Sound

Every part plays through a stateful synthesizer voice:

- **Clean oscillators.** Band-limited (PolyBLEP) saw and pulse waves keep the
  chip character without aliasing. Measured above 16 kHz, the melodic parts sit
  60–90 dB down; the first renderer had more energy there than at 12–16 kHz.
- **Patches.** Each of the 12 sounds is two oscillator layers, unison
  detuning, a resonant filter with its own envelope, an amp envelope, delayed
  vibrato, glide and drive, adjusted for the part it plays: a lead sings, a
  bass stays tight and centred, a pad breathes.
- **A real kit.** The kick has a click and a falling pitch, the snare mixes
  tone and band-passed noise, and the hats are six metallic oscillators,
  high-passed, with closed hats choking open ones. The three kits differ in
  tuning, length, drive and grit.
- **A mix.** Parts are placed across the stereo field. The bass and chords duck
  under the kick, and arpeggio, lead and chords send to a stereo reverb and a
  ping-pong echo timed to a dotted eighth. The bus is glued by a compressor,
  given a gentle high shelf, and limited at -0.8 dBFS. Songs land around
  -14 LUFS with no clipping.
- **Seamless loops.** An exported song starts already carrying the tails its
  own ending leaves, so a game looping it hears no seam.

These are software interpretations of chip sounds, not cycle-accurate
emulations.

MIDI export writes format 1 at 480 ticks per quarter: tempo, 4/4 and the three
tonal parts. MIDI import accepts format 0/1 with constant 60–220 BPM, 4/4 and up
to three monophonic tonal channels within 32 bars, quantized to sixteenths;
polyphony, percussion, pitch bends, sustain and tempo changes are refused with a
reason.

## Headless

```sh
sindri-chiptune sfx coin --seed 5 --output coin.wav --source coin.sindrisfx
sindri-chiptune sfx --open coin.sindrisfx --output coin.wav
sindri-chiptune sfx jump --seed 4 --project games/lamplighter --name jump
sindri-chiptune song --seed 42 --mood hopeful --output theme.wav
sindri-chiptune song --open theme.sindrisong --project my-game --name theme
```

`--project` does exactly what the window's Send does. `--help` lists every
option. Linux builds need ALSA headers.

## Proof

The platformer's jump, landing, coin and flag sounds were made and sent with
this tool (`games/lamplighter/assets/audio/`). Its scripted run to the flag
asserts it heard all four and that each is listed for export, and this crate's
tests re-render every committed `.sindrisfx` and compare it with the WAV
beside it.

## Checks

```sh
cargo fmt --all --check
cargo clippy -p sindri-chiptune --all-targets --all-features -- -D warnings
cargo test -p sindri-chiptune --all-features
cargo test -p lamplighter --test a_run_reaches_the_flag
cargo check -p sindri-chiptune --all-features --target wasm32-unknown-unknown
```

The tests cover every effect kind over many seeds (deterministic, audible,
bounded, click-free endings), pitch direction per kind, mutation, `.sindrisfx`
and `.sindrisong` round trips and refusals, project delivery and manifest
editing, in-key monophonic composition that repeats its theme, riff and beat and
moves mostly by step, a beat that repeats for every groove, MIDI round trips and malformed input,
undo/redo, live tempo and edit continuity, and drawing both modes at three
window sizes. Layouts were captured from the real window under Xvfb.

Pages builds the browser Lab and its smoke test opens it at desktop and phone
sizes in both modes, failing on any page error or an unpainted canvas.

Not yet: hearing it through a real audio device in CI, music short enough to
ship (a full stereo song is about 11 MB of WAV), an editable drum score or
chord track, and an editor panel.

### Arrangement and responsive controls

The arrangement displays all five sounding layers: Bass, Arpeggio, Lead, Drums,
and Chords. Each row selects its sound inspector and exposes mute and solo.
The three melodic tracks also have generation locks. Drum patterns and chord
voicings are read-only previews of the same generators used by playback; chords
follow bass edits, and their preset and level remain independently adjustable.

On phones the piano roll shows four bars at a time, with 1–4 / 5–8 page controls
for each eight-bar section. Desktop shows the full section. Fit frames the
visible notes, and the velocity strip reflects their current dynamics. The
Song, Track, Mixer and File / Export controls sit in a resizable bottom sheet on phones and
in the left inspector on desktop.

### Seeded ensembles and section participation

New songs use the seed and mood to choose one of three coordinated instrument
palettes for that mood. All five layers remain enabled across every section,
so the note composer's natural entrances and phrases are not cut off by random
section masks. Sound choices use a separate random stream, so the existing
note composer keeps its seeded output.

The A and B candidates retain their own sounds and mixes. Generate changes
unlocked instrument presets, preserving volume, octave, mute/solo, and Room &
echo. Under Song, disable **Seed instrument sounds** to keep sounds manual.
**Reset layer sections** enables every section for unlocked layers on Generate;
disable it to retain manual section choices. Under Track, **Keep notes**,
**Keep sound**, and **Keep arrangement** are independent.
**Plays in** lets you switch each layer on or off in Intro, Theme, Rise, and
Return. Arrangement rows display REST in inactive sections; the editable notes
remain intact. Solo respects section participation. Layer transitions fade
smoothly and existing room/echo tails can ring out.

Songs save the actual presets and four section masks, rather than regenerating
them on open. Older `.sindrisong` files default to the previous all-layers mix.
The CLI's `--classic` flag retains the original presets and full-layer behavior
for newly composed songs; `--open` always retains the saved mix. MIDI export
clips the three melodic parts at section rests while the editable source keeps
all notes. MIDI still carries note events rather than Sound Lab's custom synth
patches or its generated percussion/chord layers.

### Numbers, phrases, and random seeds

Song seeds accept a 64-bit number or a phrase such as `forest at dawn`. Numeric
values retain their existing meaning. Phrases use a fixed FNV-1a/64 UTF-8 hash
(version 1), with outer whitespace trimmed; case and internal spacing matter.
They are limited to 128 characters. The resolved number is shown beneath the
phrase and retained with it in candidate A's saved source. Candidate B uses that
number plus one and saves its exact numeric seed.

The dice button chooses a fresh full-range 64-bit seed from the operating system
or browser Crypto API, rather than advancing the previous number. It updates
the draft; use Generate to replace the current song. In the CLI:

```bash
sindri-chiptune song --seed "forest at dawn" --source dawn.sindrisong
sindri-chiptune song --random-seed --source surprise.sindrisong
sindri-chiptune song --seed 42 --classic --output classic.wav
```
