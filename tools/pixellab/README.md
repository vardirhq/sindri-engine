# Pixel Lab client

A command-line client for the [Pixel Lab](https://pixellab.ai) API, for generating
pixel art from a game's art scripts: top-down tilesets, characters with four or
eight directions, animations, and anything else the API offers. Standard library
Python only.

```bash
export PIXELLAB_API_KEY=...                       # your account page's API token
python3 tools/pixellab/pixellab.py balance
python3 tools/pixellab/pixellab.py spec                    # every endpoint
python3 tools/pixellab/pixellab.py spec /create-tileset    # what one accepts
python3 tools/pixellab/pixellab.py tileset --lower grass --upper "dirt path" --tile-size 16 --out art/grass
python3 tools/pixellab/pixellab.py character --description "a hooded lamplighter" --directions 4 --out art/wick
python3 tools/pixellab/pixellab.py animate --character <id> --name walk --description "walking" --out art/wick
python3 tools/pixellab/pixellab.py animate --character <id> --name slash --directions south,west --out art/wick
python3 tools/pixellab/pixellab.py object --description "a dead tree" --size 64 --keep 0,1,2,3 --out art/trees
python3 tools/pixellab/pixellab.py download <character id> --spritesheet --out art/wick
python3 tools/pixellab/pixellab.py call POST /create-image-pixen --json '{"description": "a stone well"}' --out art/well
```

Add `--dry-run` to any command to see the request it would send. Whatever an
endpoint accepts that a command has no flag for goes in with `--set key=value`
(read as JSON when it parses, else as text); `spec <path>` lists the fields from the
API's own OpenAPI document, which needs no key.

What the first real runs showed, which the mock does not:

- A tileset or a character costs one generation and a walk four, but a map
  **object costs ten** and returns sixteen candidates as a *review* object.
  Promoting a candidate (`--keep 0,1,2`, which calls `select-frames`) is free, so
  keep several: a wood of five trees costs what one tree does.
- `animate` calls `/characters/animations`, which makes one job per direction;
  the older `/animate-character` made only the south one. Leave `--directions` out
  for every direction the character has.
- An object made as a *creature* came out better than the same description as a
  character, whose default body is human whatever the description says.
- Image canvases are not the size asked for (48 for 32, 60 for a walk, 56 for its
  east frames) and the `lower` and `upper` corner labels of a tileset are not
  reliably the terrains the prompts named, so check by looking.
- `call --wait` on a request that starts several jobs keeps every job under `jobs`.
- `target_palette` takes hex colors every pixel must use; on a pro tileset at 16
  pixels it produced noise.

Generation is asynchronous. The client polls the job, waits out the API's 423
("still being made"), retries rate limits and transient errors with backoff, saves
any inline image as a PNG beside a `response.json` with the image data left out,
and unpacks character ZIPs without letting one write outside its folder.

The key is read from the environment and never written anywhere. Costs are the
account's: check `balance` before a large run.

## What is and is not checked

`test_pixellab.py` runs the client against a local stand-in that speaks the shapes
the OpenAPI document describes (`python3 -m unittest tools/pixellab/test_pixellab.py`).
`spec` has been run against the real API. Generation itself has not been run against
the real service from this repository yet, so the first real `tileset` or `character`
run is also the first check of how the real responses are shaped; the client saves
whatever comes back, so nothing is lost if a field is not where it was expected.

Turning what comes back into a Sindri `.sheet` (a `blob` terrain from a tileset, a
sprite sheet from a character) is the game's own art script, as
`games/platformer/art/pixellab/wick.py` does for a character exported by hand.
