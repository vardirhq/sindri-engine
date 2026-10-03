# Low Tide

The moving-home genre showcase: a top-down game about a base that travels.

The ocean drained away generations ago, and now it is coming back as one slow
wall of water crossing the old seabed. You crew a *crawler*, a house on treads,
and drive it north across the salt to the Rise, the high ground the Tide never
reaches. Stop at wrecks on the way for scrap, but every crate in the hold makes
the crawler heavier, and heavier is slower.

## Playing it

- **WASD** or the arrows walk. Up is always up on screen.
- **E** (or Space) takes the helm when standing at the wheel, and leaves it.
  At the helm, **W/S** move the throttle lever and **A/D** steer. The lever
  stays where you leave it, so the crawler drives on while you walk away.
- Walk out of the starboard hatch and down the **ramp** to go ashore. **E** at
  a wreck salvages a crate of scrap; carry it back up the ramp and it goes
  into the hold.
- **X** next to a crate in the hold throws it over the side: lighter, and
  poorer.
- **Tab** keeps the map north-up everywhere.

Walking the deck, the view turns with the crawler so its floor plan stays
upright and the salt turns past it. At the helm and ashore it stays north-up,
like a map. You win by reaching the Rise, and lose if the Tide reaches the
middle of the deck, or reaches you while you're ashore.

Open `assets/low-tide.scene` in the editor and press Play.

## What it is made of

There is no game code. The game is `assets/`:

- `low-tide.scene`: the Basin, the crawler and its deck, the wrecks, the Tide
  and the HUD.
- `scripts/voyage.decay`: the shared state, and how a voyage is won or lost.
- `scripts/crawler.decay`: throttle, rudder, weight, soft sand, and the hold.
- `scripts/crew.decay`: walking the deck in its own coordinates, the helm,
  going ashore and coming back aboard, salvage and jettison.
- `scripts/view.decay`: the camera's follow and its turn between deck view and
  helm view.
- `scripts/tide.decay`, `scripts/wreck.decay`, `scripts/hud.decay`.
- `textures/`: drawn by `art/draw.py`.

`art/scene.py` writes the scene. The crawler's floor plan is a block of text at
the top of that file, and the Basin's dunes and cracks are scattered from a
fixed seed. Both scripts are pure Python and deterministic, so running them
again changes nothing unless they changed.

It uses, with no Rust of its own:

- **child transforms** on a rotating parent: the deck is a tilemap under the
  crawler, the crew is under the deck, and both turn with it;
- **`World.set_parent`** at the ramp, moving the crew between the deck and the
  world;
- **`Grid.tile` and `Grid.set_tile`** from Decay: the deck's walls and
  furniture, the Basin's soft sand, crates stowed and thrown overboard;
- a **camera roll** from a script, eased between two views;
- **input actions**, **paper-doll sprite animation** chosen by on-screen
  direction, **typed messages** between scripts, **shared state** and an
  **enum**.

## Drawn to turn

Everything is drawn straight down from above, as a floor plan: walls are thick
strips with no front faces and no shadow is baked in. That is what lets the
crawler turn to any angle and still look right. The one shadow is a shape that
always falls the same way in the world, which helps the turning read. The crew
are paper dolls with a front, a back and a side, turned against the crawler and
the camera so they always stand upright on screen. The Tide is the only cool
colour, so your eye always finds it.

## Checked, not just run

`tests/a_voyage_is_played.rs` plays it with the keys a person would press. The
crew walks round the mess table to the helm and drives away, then walks the
deck while it moves. A hard turn checks that the view turns with the deck and
that "down" on screen is still down the deck. The crew stops by a wreck, goes
ashore round the bow, salvages a crate, carries it up the ramp, and finds it in
the hold with the crawler slower for it. A crawler that waits is taken by the
Tide, and one driven north outruns it to the Rise. `src/lib.rs` is the harness
that plays it without a window.

## Not yet

This is the first prototype. The doc it came from also has crew needs, more
rooms to build, sails and wind, Scrappers who board you, docking with
caravan-towns and a longer journey across regions. Ashore, the crew walks over
the crawler's hull rather than round it, and the tuning (`@export` fields on
the crawler, the Tide and the crew) is a first guess.
