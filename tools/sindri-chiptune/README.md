# Sindri Chiptune Lab: Composer V2

Standalone Rust composition laboratory. **No Sindri editor or runtime integration.**

```sh
cargo run -p sindri-chiptune
```

The redesigned desktop UI contains a four-track instrument rack, composition settings, four-section
arrangement information, an eight-bar piano-roll comparison for two seed
candidates, A/B auditioning through a continuous sample source and explicit WAV export. Each part has an independent preset, volume, mute, solo and (for tonal parts) octave shift. These modify the render, not the composed notes.

## What V2 changes

- Hierarchical composition: a style determines scale, harmonic progression,
  groove, note density and melodic movement, with a four-section energy curve.
- Four genuinely different strategies: mysterious exploration (modal,
  spacious), hopeful adventure (major, bouncing), tense action (chromatic,
  driving), and melancholic reflection (minor, slow-moving).
- Seeded 64-candidate motif search with a small structural scoring function.
  Chosen motifs repeat and develop across sections; the middle section
  introduces a second theme.
- Style-dependent bass rhythm, chord arpeggiation, melody rests, instrument
  articulation and pulse duty.
- A/B comparison displays the first eight bars of actual generated note
  events for the current seed and the next seed. Play either without saving
  a file. WAV export saves the selected base seed.
- Independent random streams for the planning, bass, arpeggio and lead.

Note-data generation is deterministic for the same settings and seed.
The synthesizer is inspired by early handheld hardware but is not a precise
hardware emulator. Human listening is still required to judge musical quality.

## Headless export

```sh
cargo run -p sindri-chiptune -- --seed 42 --output composition.wav
cargo run -p sindri-chiptune -- --seed 43 --bpm 140 --key 5 --energy 0.7 --output variation.wav
```

GUI mood changes the generation style. The CLI currently defaults to
Mysterious; CLI mood flags are not part of this iteration.

## Boundaries / known limitations

Everything stays under `tools/sindri-chiptune/` and depends only on
eframe and native Rodio. The synthesizer offers 12 tonal presets and 3 percussion kits; these are software-synthesis interpretations, not cycle-accurate chip emulations. Preview is rendered to memory, not streamed
incrementally; long previews must render before playback. Linux audio may
require ALSA development packages. The structure is still fixed at 32 bars.
Candidate scoring is explicit heuristic search, not ML or AI. A/B comparison
is not yet a full selective-track editing system.

## Validation

```sh
cargo fmt --all --check
RUSTFLAGS="-D warnings" cargo check -p sindri-chiptune --all-targets --all-features
cargo clippy -p sindri-chiptune --all-targets --all-features -- -D warnings
cargo test -p sindri-chiptune --all-features
```

Tests include deterministic generation, note bounds, contrasting mood
arrangements, and seed-dependent melodic pitch changes. Listening against
the original prototype is the next subjective acceptance gate.


## Live workflow (experimental)

**Play A** starts continuous synthesis rather than pre-rendering 32 bars.
While audio plays, adjust instrument preset, mute, solo, level, octave, energy
or tempo. The audio thread reads parameter updates between 256-sample blocks,
without rebuilding a WAV or restarting the transport. **Switch B** updates
the song notes using the next seed; **Pause**, **Stop**, **Loop**, and bar
seek are available on the transport. Export continues to use offline rendering
and does not interrupt live playback.

The real-time source lives in `src/live.rs`, separate from `composer.rs`,
`instruments.rs`, and the egui presentation. The source uses a message queue
for parameter updates and keeps note cursors in the audio thread. This is an
experimental transport, not yet a fully optimized, sample-accurate game-audio
engine. Performance, synchronization at song transitions, and auditory quality
still require local hands-on validation.
