# Orbital asset sources

`fleet-0.svg` through `fleet-3.svg` are original authored vector illustrations.
Their transparent PNG exports live in `../textures/`. Rebuild with ImageMagick:

```bash
magick -background none fleet-0.svg ../textures/fleet-0.png
```

Repeat for the other three hulls. In-flight ships use authored engine shapes,
so their silhouettes remain sharp while turning and scaling.

`../textures/orbital-key-art.png` is original AI-generated corridor artwork,
created with OpenAI image generation for this game edition. The brief was an
unlettered cinematic orbital evacuation corridor: a dark planet, copper rim
light, scattered fleet debris and deep navy space, with quiet space for UI.
The title typography and interface are engine-rendered, separate from the art.

`../audio/briefing.sindrisong`, `corridor.sindrisong` and
`last-signal.sindrisong` are editable Sindri Sound Lab compositions. They were
composed with seed 734, key 2, respectively mysterious/92 BPM/intensity .28,
hopeful/118 BPM/.62 and tense/142 BPM/.84. Their Ogg exports use stereo Vorbis.
Re-render a song with `sindri-chiptune song --open FILE.sindrisong --output output.wav`;
convert that WAV with `ffmpeg -i source.wav -c:a libvorbis -q:a 4 output.ogg`.
`burst.sindrisfx` and `victory.sindrisfx` are the editable sources for their WAVs.

The remaining shape, font, audio and mine-blast assets retain their earlier
provenance. The mine blast is regenerated with the repository's isometric baker
from `../textures/detonation.isobake`.
