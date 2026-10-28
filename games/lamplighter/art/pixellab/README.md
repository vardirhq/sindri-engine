# Pixel Lab sources

What `tools/pixellab/pixellab.py` produced, kept as it came back so the art
scripts that build the project's sheets can be run again without the service.

- `tileset/`: `tileset --lower "dark mossy grass, night" --upper "packed dirt
  path" --transition "worn grass edge" --tile-size 16 --view "high top-down"
  --seed 7`. Sixteen 16 by 16 tiles, each described by its four corners.
- `wick/`: `character --description "Wick, a small lamplighter courier in a worn
  hooded cloak and scarf, carrying a brass lantern on a pole, dark muted blue cloak
  with a warm amber lantern glow, young and determined" --size 32 --directions 4
  --view "low top-down" --seed 11` for the four rotations, then the walk from
  `POST /characters/animations` with `action_description` "walking steadily, legs
  stepping alternately, the lantern held at the same height the whole time", 8
  frames, all four directions, seed 3. `character-id.txt` is the service's id for
  Wick, which later animations (the sword) are made against.

The canvases differ: rotations are 48 by 48, the walk is 60 by 60 and its east
frames 56 by 56. `art/wick.py` puts every frame on one cell, aligned by the feet.
