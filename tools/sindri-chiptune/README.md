# Standalone Chiptune Prototype

Run from the repository root:

```sh
cargo run -p sindri-chiptune -- --seed 42 --output chiptune.wav
```

Produces a mono 44.1 kHz 16-bit PCM WAV (32 bars at 118 BPM, roughly 65 seconds). No sound device or engine/editor dependency. Everything is authored from scratch; no Autotracker source code is included.

This is an **audition prototype**, not a full composer: fixed D-minor progression, four hardware-inspired channels, motif-based pulse melody, broken-chord arpeggios, bass and sparse procedural noise percussion. The CLI exposes seed and output path only; music-generation quality must be judged by listening.

Next iterations: editable pattern representation, multiple moods/presets, explicit per-track seeds and locking, candidate search and ranking, proper percussion synthesis, loop-boundary checks, and rendering/performance benchmarks.
