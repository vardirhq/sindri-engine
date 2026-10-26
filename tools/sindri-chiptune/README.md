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

New sessions use the **Musical** composer. Its Rise develops the main motif's
rhythm and opening contour before turning the tail, so the contrast belongs to
the same piece. Four-bar questions remain open; eight-bar answers prepare a
modal cadence and reach the tonic, with a little breathing room at phrase ends.
The Return brings back the theme. Bass, melody and arpeggio share that harmonic
plan, including selective regeneration.

Four moods set major, natural minor, Dorian or Phrygian harmony. Seeds choose
progression libraries, melodic cells, bass riffs and one of six grooves:
**Drift, Bounce, Drive, Ballad, Shuffle** or **Half-time**. Cadences retain the
mode's own colors rather than forcing a major dominant into every style.

The arpeggio gives busy lead phrases more space and chooses nearby octaves
below the sounding melody. Every layer remains enabled in every section by
default; these are note-writing decisions, not random section cutoffs. Manual
mixing, octave shifts and edited notes can still deliberately overlap registers.

The Musical pad reads the bass root once per bar, so a passing fifth midway
through a riff does not accidentally change the chord. Close-position inversions
minimize movement of individual voices. Playback and the arrangement preview
use the same history of bar roots; seeking does not choose another inversion.
Chord tones remain diatonic, with sevenths in the darker moods.

**Legacy** retains the previous note generator and half-bar pad behavior. Choose
the composer under Song; the choice applies on Generate. Note locks keep the
current seed, key, mood and composer version together. Songs and snapshots save
`generation: "musical"` or `"legacy"`; older files without the field reopen as
Legacy, preserving their notes and accompaniment behavior. Unknown versions are
refused rather than silently played with different rules. The same seed and
settings, including the composer version, remain deterministic. Earlier builds
cannot read the new field.

For the previous complete generation path in the CLI, use:

```bash
sindri-chiptune song --seed 42 --generation legacy --output legacy.wav
```

`--classic` independently selects the original instrument presets and full-layer
mix; combine it with `--generation legacy` when both are wanted.

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
section masks. Sound choices use a separate random stream, so palette changes do not consume
the note composer's random stream.

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

### Develop an existing composition

- **Regenerate selected bars** under Track replaces an inclusive range of bars
  in Bass, Arpeggio, or Lead. **Visible bars** selects the current piano-roll
  window, shown by an amber strip. Other note layers and notes outside the
  range remain intact; crossing notes retain their outside portions. New
  melodic phrases follow edited bass roots, and Bass edits also update the
  derived Chords layer. Keep notes blocks regeneration. Undo restores the edit.
- **Try another ensemble** under Song or Mixer cycles through the current
  song mood's three coordinated palettes. Keep sound locks are respected.
  Notes, section participation, tuning, levels, mute/solo and room settings
  remain unchanged. Playback picks up new sounds live; Undo/Redo compares them.
- **Inspect harmony** under Track describes the selected piano-roll note's
  sounding pitch (including its octave setting), scale membership, chord tones
  at its onset, and whether it crosses a chord boundary. It uses the same
  derived chord preview as playback and labels silenced layers. It does not
  declare dissonance an error or analyze audio, effect tails, or every
  instrument's tuning/overtones.
- **Named snapshots** under Song stores up to 16 named versions of the selected
  candidate, including notes, sounds, section choices, seed, mood, key, density,
  and tempo. Save the song to persist snapshots in its `.sindrisong` file.
  Restore loads a snapshot into A and rebuilds B from its settings; Undo restores
  the previous pair of candidates and their settings. Snapshot names are unique
  in the workstation. Older sources without snapshots continue to load.

### Develop motifs, dialogue, expression, and soundtrack families

These are explicit editing actions. Opening a song or generating with the same
settings continues to use the existing composer; no development action runs
automatically. Musical edits bake into the ordinary notes and use Undo/Redo.

Under Track, the development panels share the selected source-bar range:

- **Develop a motif** copies the current melodic layer into a destination bar.
  Repeat preserves the phrase; Scale sequence moves it by diatonic steps;
  Invert contour mirrors its intervals around the first source note. A length
  factor of 2 stretches onsets and durations. Optional bass adaptation follows
  the destination harmony for Arpeggio and Lead. The destination is replaced,
  outside notes remain intact, and an overlong result is rejected. Keep notes
  locks block the edit. Bass edits also change the derived Chords layer.
- **Call and response** rewrites Arpeggio and Lead into alternating one- or
  two-bar phrases. Choose the caller and select complete pairs of phrases. The
  responder answers a scale step lower, in its own register, and resolves to
  the bass root. Bass, surrounding bars and section switches are retained.
  Both melodic layers must be unlocked. These are actual note phrases and
  rests; automatic section masking remains disabled.
- **Performance expression** shapes accents, four-bar phrase dynamics, seeded
  velocity variation, and articulation. Choose downbeats, backbeat, or
  offbeats. Note length is a percentage of the current duration, quantized to
  the existing sixteenth-note grid. Pitch and onset timing remain fixed;
  crossing boundary notes stay untouched. Zero expression and 100% length is
  neutral. Repeated edits can be undone rather than accumulated.

Under Song, **Soundtrack family** creates three named snapshots: Exploration,
Danger, and Victory. These keep the source rhythm and related melodic contours,
adapting scale, tempo, dynamics, generated percussion feel and instrument
palette. Victory resolves the last Bass and Lead notes to the tonic. Existing
mix levels, tuning, mute/solo and manual section switches are retained. The
current song and its locks are unchanged; the new versions are independent
musical alternatives. Restore them under Named snapshots to audition, edit or
export each one. Save the song to retain the family. Creating a family requires
three free snapshot slots and a unique family name; it adds all three or none.
These are separate authored cues, not an adaptive game playback API.

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
