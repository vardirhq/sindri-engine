# Platformer

The first genre showcase: a small side-view platformer. Run and jump across a
painted level, pick up the coins and reach the flag.

Arrow keys or A/D run; Space, W or Up jumps. A controller can use the left
stick or D-pad and the south face button. Down/S or D-pad down drops through
the raised one-way planks. On a touch screen, the first finger
makes Sindri's virtual stick and a second finger jumps. A tap is a hop and a
held press a full jump; a jump pressed just before landing, or just after
running off a ledge, still counts. Horizontal movement accelerates and
decelerates rather than snapping between stopped and full speed, with
deliberately lighter air control after committing to a jump.

The touch path is functional input, but the platformer does not yet draw its
mobile controls. Visible on-screen affordances for the stick and jump action
remain part of the mobile presentation work rather than being hidden behind a
claim that two invisible fingers somehow constitute a finished mobile UI.

## What it is made of

There is no game code. The game is `assets/`:

- `platformer.scene`: the level, the hero, the coins, the flag and the HUD.
- `prefabs/coin.prefab`: what a coin is — its spinning sprite, sensor and tag.
  The scene places ten instances of it, each saying only where it stands, so
  a change to the prefab is a change to every coin.
- `scripts/hero.decay`: keyboard, controller and touch movement, jumping, coins,
  the flag and falling off.
- `scripts/hud.decay`: the coin count and the banner.
- `textures/`: pixel art drawn by `art/draw.py`, deterministic, so running it
  again changes nothing unless the drawing did.

It uses, with no Rust of its own:

- a **tilemap** painted in the editor, made solid by a **Tilemap Collider 2D**,
  with grass tufts left passable;
- the scene's own **gravity**, from a **Physics 2D World**;
- a **dynamic body** with a capsule collider and a **foot sensor** for pickups, plus a
  downward **support ray** for standing so plank undersides never grant a jump;
- visible **one-way planks** with local support normals and timed drop-through;
- **sprite animation** clips for idle, run, jump and fall;
- **keyboard, gamepad and touch input** through one movement path, including
  Sindri's built-in touch stick;
- a **camera** that follows the hero and stays inside the level;
- **screen text** filled from Decay.

## Playing it

Open `assets/platformer.scene` in the editor and press Play: the Scene view
opens in 2D, framed on the game's camera. It is also exported to the site at
`examples/platformer/`.

## Checked, not just run

`tests/a_run_reaches_the_flag.rs` stands the hero on the painted ground, has a
player hold right and jump at every gap and wall until it reaches the flag
without falling, and checks the camera follows. `src/lib.rs` is the harness
that plays it without a window, built from the same public pieces a host uses.

`tests/one_way_platforms.rs` jumps through the authored planks, lands on top,
drops through using the real input action, stops on the ordinary painted floor
and lands on the planks again after the drop timer expires.
