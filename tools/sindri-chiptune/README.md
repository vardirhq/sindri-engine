# Sindri Chiptune Lab (Standalone Prototype)

Run from the repository root:

```sh
cargo run -p sindri-chiptune
```

This opens a standalone egui desktop window with mood, key, tempo, energy,
seed and output WAV controls. **Generate WAV** renders in a worker thread;
**Open WAV** launches the result in your OS's default audio application.
There is no editor integration or dependency on Sindri's runtime/audio engine.

Headless CLI remains available:

```sh
cargo run -p sindri-chiptune -- --seed 42 --output chiptune.wav
cargo run -p sindri-chiptune -- --seed 43 --bpm 140 --key 5 --energy 0.7 --output variation.wav
```

Key is a pitch class from 0 (C) through 11 (B); energy is 0.0–1.0.
The CLI defaults to the original mysterious D-minor-inspired arrangement.
Mood selection and some extra expressive controls are currently GUI-only.

Produces a mono 44.1 kHz 16-bit PCM WAV (32 bars, around 65 seconds at
118 BPM). The fixed four-section structure uses three melodic channels
(bass, arpeggios, melody) and procedural noise percussion.

This is a **listening and experimentation prototype**, not a full
AI composer or a true Game Boy hardware emulator. Different moods change
chord order, while tempo, key, energy and seed influence generated output.
Some seed changes are intentionally subtle; sophisticated motif search,
independent track regeneration, sound design and dedicated music playback
controls remain future work.

The program reuses only the repository's existing eframe dependency.
No Autotracker source code is copied.

## Quality gates

```sh
cargo fmt --all --check
RUSTFLAGS="-D warnings" cargo check -p sindri-chiptune --all-targets --all-features
cargo test -p sindri-chiptune --all-features
```
