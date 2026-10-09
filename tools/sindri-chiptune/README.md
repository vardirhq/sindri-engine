# Sindri Chiptune Lab

A standalone, experimental music generator. Nothing in the Sindri editor,
runtime, Decay, or player depends on this binary.

## Launch

```sh
cargo run -p sindri-chiptune
```

The desktop UI exposes mood, key, tempo, energy, and seed. **Preview**
composes and renders in memory, then plays directly through the default
audio device using Rodio. No WAV file is created during preview.
**Stop** halts playback. **Export WAV** writes a mono 44.1 kHz PCM
file only when requested. Preview rendering runs on a background thread.

**New variation** advances the deterministic seed. Changing seed now
changes motif contour, rhythmic placement, arpeggio order, bass variation,
and note selection. Mood changes chord progression and harmonic color;
energy changes note densities and dynamics; tempo and key remain musical
controls. The four-section 32-bar arrangement is still fixed, and the
music is still an experiment, not yet a full composition engine.

The CLI remains usable:

```sh
cargo run -p sindri-chiptune -- --seed 42 --output chiptune.wav
cargo run -p sindri-chiptune -- --seed 43 --bpm 140 --key 5 --energy 0.7 --output variation.wav
```

CLI key is a pitch class from 0 (C) to 11 (B). Energy is 0.0 to 1.0.
The desktop app requires an audio device for preview. WAV export works
without one. On Linux the Rodio backend may require ALSA development
libraries to compile, as with the engine's existing native audio stack.

Composition, sample generation, and UI live in separate functions so they
can be extracted into libraries if the experiment proves worthwhile.
The standalone UI is not plugged into Sindri.

## Checks

```sh
cargo fmt --all --check
RUSTFLAGS="-D warnings" cargo check -p sindri-chiptune --all-targets --all-features
cargo clippy -p sindri-chiptune --all-targets --all-features -- -D warnings
cargo test -p sindri-chiptune --all-features
```

The core's tests verify determinism, note bounds, and that seed,
mood, and energy materially change generated note data.
