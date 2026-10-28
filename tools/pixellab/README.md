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
python3 tools/pixellab/pixellab.py download <character id> --spritesheet --out art/wick
python3 tools/pixellab/pixellab.py call POST /create-image-pixen --json '{"description": "a stone well"}' --out art/well
```

Add `--dry-run` to any command to see the request it would send. Whatever an
endpoint accepts that a command has no flag for goes in with `--set key=value`
(read as JSON when it parses, else as text); `spec <path>` lists the fields from the
API's own OpenAPI document, which needs no key.

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
