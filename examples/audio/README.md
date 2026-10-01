# Sound Mixer

A feature example for audio buses. Open `sindri.toml` in the editor or run the
exported project in a WebGPU browser. Pages publishes it at
`https://sindri.vardir.no/examples/audio/`.

Every sound plays through a bus, and every bus through `master`. The theme is
an authored looping `sindri.audio.source`, so it plays on `music`; the buttons
play effects with `Audio.play` and `Audio.play_on("effects", …)`. Each slider
calls `Audio.set_volume` for its bus, which changes what is already playing as
well as what plays next, and its fill and percentage read the bus back with
`Audio.volume`. Drag a slider, tap along it, or use the arrows: up and down
choose a slider, left and right move it a step. Browsers start audio on the
first click or key.

The layout is `assets/ui/mixer.weave`: one centred column and a panel of bus
rows, with rules for a phone held upright and a phone on its side.

All behaviour is in `assets/mixer.decay`. `game/tests/the_mixer_and_input_demos_work.rs`
moves the buses by keyboard and by touch and plays an effect through its bus;
the platform's `AudioMixer` tests prove the volumes reach live voices.

See [the scripting contract](../../docs/scripting.md#audio).
