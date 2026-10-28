# Pixel Lab sources

What `tools/pixellab/pixellab.py` produced, kept as it came back so the art
scripts that build the project's sheets can be run again without the service.

- `tileset/`: `tileset --lower "dark mossy grass, night" --upper "packed dirt
  path" --transition "worn grass edge" --tile-size 16 --view "high top-down"
  --seed 7`. Sixteen 16 by 16 tiles, each described by its four corners. The
  response's `lower` corners are the **dirt** and its `upper` the **grass**, though
  the prompts said the reverse, so `art/tiles.py` reads `lower` as dirt; a render
  showed the path inverted before this was found.
- `wick/`: `character --description "Wick, a small lamplighter courier in a worn
  hooded cloak and scarf, carrying a brass lantern on a pole, dark muted blue cloak
  with a warm amber lantern glow, young and determined" --size 32 --directions 4
  --view "low top-down" --seed 11` for the four rotations, then the walk from
  `POST /characters/animations` with `action_description` "walking steadily, legs
  stepping alternately, the lantern held at the same height the whole time", 8
  frames, all four directions, seed 3. The sword's `slash` is the same call with
  "swinging a short iron sword in a single wide horizontal slash with the free hand
  while the other hand keeps the lantern raised and steady, feet planted, then
  returning to a ready stance", 8 frames, seed 5. The south and north rows are the
  weakest: the blade is faint in one and scattered in the other. `character-id.txt` is the service's id for
  Wick, which later animations (the sword) are made against.

The canvases differ: rotations are 48 by 48, the walk is 60 by 60 and its east
frames 56 by 56. `art/wick.py` puts every frame on one cell, aligned by the feet.

- `objects/`: `POST /create-1-direction-object` with `view: top-down`, one call each:
  "a gnarled dead tree with bare twisting branches, dark bark, night" (64),
  "a mossy grey boulder, dark, night" (32) and "a tall iron lamp post with a
  brass lantern at the top glowing warm amber with a bright flame" (48). Each call
  costs **10 generations** and returns sixteen candidates as a *review* object; one
  is promoted with `POST /objects/{id}/select-frames {"indices": [0]}`, which is
  free, and exported with `GET /objects/{id}/spritesheet` (a ZIP). Frame 0 was
  taken for the lamp post; for the tree and boulder frames 0 to 4 and 0 to 3 are
  all used, so a wood is five different trees and a field four boulders. The unlit lamp post is not generated: `art/objects.py`
  darkens the lit one's warm pixels, so the pair share a silhouette.

## Tried and not used

- **`target_palette` on a tileset.** The API takes up to 256 hex colors that every
  pixel must use, on tilesets in `mode: pro` and on objects and animations. A 43
  color palette built from the art already chosen turned a pro stone-road tileset
  into dithered noise (dark blue stones, olive grass with yellow flecks), so it was
  dropped. It also costs far more than a standard tileset.
- **A second tileset for a stone road,** `--lower "worn grey cobblestone road"
  --upper "dark mossy grass"`. It came back well, but its grass is a brighter green
  than the first tileset's dark moss (9,73,33 against 23,33,19), so the two cannot
  share a field. `upper_base_tile_id` with the first tileset's terrain id did not
  reuse its grass. A deterministic recolor of its greens onto the first set's ramp
  in the art script is the way to try next.
- **A gloom as a four-direction character.** The character call defaulted to its
  human "mannequin" body and ignored the description's shape, giving a shadow person
  rather than a creature.
- **The labels `lower` and `upper` are not reliable between calls.** The first
  tileset's `lower` was the dirt although the prompt said grass; the stone road's
  `lower` was the stone, as prompted. Always check by color.
