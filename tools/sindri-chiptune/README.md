# Sindri Sound Lab

A standalone native chiptune workstation. No Sindri editor or runtime integration.

```sh
cargo run -p sindri-chiptune
```

The persistent transport sits above a composition inspector, editable piano roll
and four-part mixer. On compact windows the mixer joins the scrollable inspector.
Amber marks transport/actions; blue, purple, mint and copper identify the voices.

## Compose, edit, listen

1. Choose mood, key, seed and density, then **Generate A + B**. The two scores are
   cached; painting the UI never recomposes the song. Generation replaces both
   candidates and clears note-edit history.
2. Select A or B and press **Play**. Switching candidates or generating a new
   pair intentionally restarts playback. Pause/resume, stop, loop, restart and
   clicking the bar ruler control the transport.
3. Choose Bass, Arpeggio or Lead. Click empty space to add a note, click a note to
   select it, drag to move it or right-click to delete it. The selected-note
   inspector changes pitch, duration and velocity. Notes snap to sixteenth
   steps. Undo/redo retains up to 64 edits. One voice per tonal part means
   overlapping edits are rejected and rolled back.
4. Choose an eight-bar section. Scroll the piano roll horizontally to see all
   eight bars. **Fit notes** and the pitch-window control choose the displayed
   two-octave register; notes outside that register remain in the score.
5. Adjust sounds, levels, mute/solo, octave or tempo while listening. Note edits
   and tempo changes preserve musical position. Mood/key/seed/density wait for
   Generate. These draft changes cannot silently alter the playing score.
6. Export the **selected** score as WAV or MIDI. WAV includes the rack settings
   and generated percussion; MIDI contains raw tonal notes and song tempo.

## Musical structure

Four styles use major, minor, Dorian or Phrygian harmony. Chords stack thirds in
that style's scale instead of applying one major/minor chord shape everywhere.
A seeded motif search favors bounded, balanced contours. The lead keeps a hook
in the global key, resolves accents to chord tones, repeats a question/answer
phrase, leaves cadence breaths and lands on the tonic. The rise uses a second
motif and the return recalls the original. Intro and return strip back activity.
The percussion voice combines a pitched kick, backbeat, short hats and fills.

The synthesizer offers 12 tonal presets and 3 percussion kits. These are software
interpretations, not cycle-accurate hardware emulations. Structural tests do not
prove a song is enjoyable; listening remains the subjective acceptance gate.

## MIDI interchange

**Export MIDI** writes Standard MIDI File format 1: a tempo/time-signature track
and three tonal channels at 480 ticks per quarter. Notes, durations and velocities
round-trip; synth presets, mixer gains, mute/solo and octave shifts are not MIDI
note edits and are not encoded. The procedural percussion voice is not exported.

**Import MIDI** replaces the selected candidate after validation. It accepts
format 0/1, metrical timing, constant 60–220 BPM, 4/4 and at most three nonempty
monophonic tonal channels, mapped in channel order. Imported timing is quantized
to sixteenths and must fit 32 bars. Unsupported polyphony, percussion, sustain,
pitch bends, tempo changes, meters and malformed files produce explicit errors.
Other controller/program/metadata events are not interpreted. This is bounded
note interchange, not a general DAW or hardware-MIDI interface.

## Headless WAV export

```sh
cargo run -p sindri-chiptune -- --seed 42 --mood hopeful --output composition.wav
cargo run -p sindri-chiptune -- --seed 43 --bpm 140 --key 5 --energy 0.7 --output variation.wav
```

`--mood`: mysterious, hopeful, tense or melancholic. Linux builds need ALSA
headers. Desktop modules and dependencies are excluded from the WASM build; its
binary is an inert stub, not a browser workstation. No new dependencies were added.

## Validation / remaining limits

```sh
cargo fmt --all --check
RUSTFLAGS="-D warnings" cargo check -p sindri-chiptune --all-targets --all-features
cargo clippy -p sindri-chiptune --all-targets --all-features -- -D warnings
cargo test -p sindri-chiptune --all-features
cargo check -p sindri-chiptune --all-features --target wasm32-unknown-unknown
```

Tests cover monophonic in-key generation over many seeds, tonic resolution, motif
recurrence, MIDI round-trip/malformed input, cached scores, undo/redo, overlap
rollback, compact/wide layout, live edits, tempo continuity and non-loop completion.
Native layout has been captured under Xvfb. Real audio-device audition remains
unverified here. Arrangement length is fixed at 32 bars; no project persistence,
editable drum score, MIDI controllers, effects or editor integration yet.
