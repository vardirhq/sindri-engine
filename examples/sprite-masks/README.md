# Sprite Masks

A feature example for sprite masks. Open `sindri.toml` in the editor or run the
exported project in a WebGPU browser. Pages publishes it at
`https://sindri.vardir.no/examples/sprite-masks/`.

A keyhole wanders over a night sky in a slow figure of eight:

- `keyhole` carries `sindri.sprite_mask` with the keyhole picture, named
  `keyhole`, and the `Wander` script in `assets/scripts/keyhole.decay` that
  moves it.
- `night` is a dark sprite masked `outside` the keyhole, so the sunny meadow
  under it shows only through the hole.
- `gem` is masked `inside` the keyhole, so it is only ever seen when the hole
  passes over it.

The art is drawn by `art/draw.py`. `game/tests/the_sprite_mask_demo_works.rs`
plays the scene and checks both masks. See [the sprite mask
contract](../../docs/sprite-masks.md).
