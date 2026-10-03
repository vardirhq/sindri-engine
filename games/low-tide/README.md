# Low Tide

The moving-home genre showcase: a top-down game about a base that travels.

The ocean drained away generations ago and left the Basin: an old sea floor of
salt flats, salt crust, mud, dunes, dead reef and dried kelp, with the old
islands standing up out of it, grassed and wooded, and brine left in the
deeps. It goes on in every direction. You crew a *crawler*, a house on treads,
and drive it wherever you like. Soft and rough ground slow it, and brine and
cliffs stop it. Stop at wrecks for scrap, but every crate in the hold makes the
crawler heavier, and heavier is slower.

The Tide returns as a repeating flood season. Low water lasts three minutes,
then the brine rises for 90 seconds, holds high for 45 and recedes for 90.
The HUD gives a countdown: head uphill before the lowlands flood. A caught
crawler waits for the ebb and loses one unsecured crate per season. Caught
ashore, you lose carried salvage and wade at reduced speed, but can still
escape or board the ramp. Flooded wrecks cannot be salvaged. Nothing kills
you or ends the voyage, and the crawler can drive again when its ground dries.

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

On a phone, the controls appear with the first touch. Drag anywhere off the
buttons to walk (a ring shows where your thumb landed), and the same drag
works the throttle and rudder at the helm. **Use** takes the helm, leaves it
and salvages, and is labelled for whichever it would do. **Drop** appears
beside a crate in the hold, and **View** keeps the map north-up.

Walking the deck, the view turns with the crawler so its floor plan stays
upright and the salt turns past it. At the helm and ashore it stays north-up,
like a map. Stepping ashore drops the anchor, so the crawler waits for you.

Open `assets/low-tide.scene` in the editor and press Play.

## What it is made of

There is no game code. The game is `assets/`:

- `low-tide.scene`: the Basin, the crawler and its deck, the wrecks and the
  HUD.
- `basin.tileset`: the Basin's blocks, each tagged with what it is to the
  game: `soft`, `rough`, `liquid`.
- `scripts/voyage.decay`: the shared state, and how the Basin is read.
- `scripts/tide.decay`: the season and its reversible water level, tunable in the scene.
- `scripts/crawler.decay`: throttle, rudder, weight, the ground's going,
  running aground, and the hold.
- `scripts/crew.decay`: walking the deck in its own coordinates, the helm,
  going ashore and coming back aboard, salvage and jettison.
- `scripts/view.decay`: the camera's follow and its turn between deck view and
  helm view.
- `scripts/touch.decay`: the phone's controls, shown from the first touch:
  the stick's ring and the Use, Drop and View buttons.
- `scripts/wreck.decay`, `scripts/hud.decay`.
- `textures/`: drawn by `art/draw.py`, and the Basin's blocks with their
  block set by `art/terrain.py`.

`art/scene.py` writes the scene. The crawler's floor plan is a block of text at
the top of that file. The scripts are pure Python and deterministic, so
running them again changes nothing unless they changed.

## Visual style

The crawler stays a straight-down floor plan. Its bridge has worn metal plates,
the living quarters have warm boards and stitched bunks, and the machinery hold
has a grated floor. A cosmetic tilemap under the deck gives each room its
material without changing the logical tiles used for walking or stowing cargo.
Furniture and metal fittings use small contact shadows and highlights that
rotate with the crawler; no directional world lighting is baked into them.
Terrain materials have eight seeded variants each: fine salt grains and
fractures, wind-shaped sand, coral remains, curled kelp and quiet brine ripples.
All art comes from the deterministic Python sources in `art/`.

## The Basin

The Basin is a **voxel world viewed as a map**: the engine's natural terrain,
the same generator a 3D voxel game uses, drawn flat from above with each
column showing the top of its highest block, lit by height. Its settings are
`BASIN` in `art/scene.py`. Warmth falls with height, so the biomes sort
themselves by it: the cold high ground is the old islands, and the warm floor
is salt where it is dry, mud where it is wet, dead reef between and kelp where
it is cool. The sea is the generator's own, left as brine in the deeps.

The same settings with the engine's own blocks and land biomes
(`NaturalTerrainDocument::with_builtin_blocks`) make an ordinary overland
world instead: grassland, forest, swamp, desert, badlands, taiga and tundra.
Nothing in the scripts would change.

It uses, with no Rust of its own:

- **child transforms** on a rotating parent: the deck is a tilemap under the
  crawler, the crew is under the deck, and both turn with it;
- **`World.set_parent`** at the ramp, moving the crew between the deck and the
  world;
- **`Grid.tile` and `Grid.set_tile`** from Decay: the deck's walls and
  furniture, crates stowed and thrown overboard;
- a **voxel world viewed as a map**, read with **`Grid.surface`**,
  **`Grid.height`** and **`Grid.tagged`**;
- a **camera roll** from a script, eased between two views;
- **Sindri's virtual stick** and **screen UI buttons** for touch;
- **input actions**, **paper-doll sprite animation** chosen by on-screen
  direction, **typed messages** between scripts, **shared state** and an
  **enum**.

## Drawn to turn

Everything is drawn straight down from above, as a floor plan: walls are thick
strips with no front faces and no shadow is baked in. That is what lets the
crawler turn to any angle and still look right. The one shadow is a shape that
always falls the same way in the world, which helps the turning read. The crew
are paper dolls with a front, a back and a side, turned against the crawler and
the camera so they always stand upright on screen.

## Checked, not just run

`tests/a_voyage_is_played.rs` plays it with the keys a person would press. The
crew walks round the mess table to the helm and drives away, then walks the
deck while it moves. A hard turn checks that the view turns with the deck and
that "down" on screen is still down the deck. The crew stops by a wreck, goes
ashore round the bow, salvages a crate, carries it up the ramp, and finds it in
the hold with the crawler slower for it, then throws it over the side again.
The Basin is checked to be varied around the start, a crawler steered at the
nearest brine runs aground rather than driving in, and one stepped off at full
throttle stops within walking distance.

`tests/played_on_a_phone.rs` plays it on a phone-sized screen with fingers
only: a thumb on the stick walks to the helm and drives, the buttons take and
leave the helm, View turns the map north-up, the whole salvage run is played
again by touch and ends with Drop. It also checks that a keyboard player never sees the touch controls.

`src/lib.rs` is the harness that plays it without a window. It lays out and
hit-tests screen UI as a host does, so a finger pressing a button is a real
press.

## Not yet

Next: gathering across the Basin, then building onto the crawler. Ashore, the
crew walks over the crawler's hull rather than round it, wrecks are placed by
hand near the start rather than across the world, and the tuning (`@export`
fields on the crawler and the crew) is a first guess.

`tests/a_flood_season.rs` checks the rise, high water, ebb and next season,
cargo washed out once, driving again after the ebb, caught crew escaping with
keys and at 390×844 by touch, and unchanged terrain with builtin overland blocks.
The map extractor separately checks water at height boundaries and retention
of column chunks across flood changes (while block edits still invalidate them).
