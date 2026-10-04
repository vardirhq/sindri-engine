# Low Tide

The moving-home genre showcase: a top-down game about a base that travels.

The ocean drained away generations ago and left the Basin: an old sea floor of
salt flats, salt crust, mud, dunes, dead reef and dried kelp, with the old
islands standing up out of it, grassed and wooded, and brine left in the
deeps. It goes on in every direction. You crew a *crawler*, a house on treads,
and drive it wherever you like. Soft and rough ground slow it, and brine and
cliffs stop it. Gather wood from island trees, fibre from leaves and kelp, stone and ore from
rock and reef, salt from the flats, and scrap from wrecks. Carry each bundle
aboard: the starter has four cargo slots, and every bundle makes it heavier.
You begin with a narrow seven-by-nine crawler: helm, one engine, a ramp and
a basic workbench. Larger rooms and storage are earned through construction.

The Tide returns as a repeating flood season. Low water lasts three minutes,
then the brine rises for 90 seconds, holds high for 45 and recedes for 90.
The HUD gives a countdown: head uphill before the lowlands flood. A caught
crawler waits for the ebb and loses one unsecured crate per season. Caught
ashore, you keep held cargo and can wade out at reduced speed. Empty hands
can swim through deeper water and board the ramp; ordinary crates are too
bulky to carry voluntarily into swimming water. Flooded wrecks cannot be salvaged. Nothing kills
you or ends the voyage, and the crawler can drive again when its ground dries.

## Playing it

- **WASD** or the arrows walk. Up is always up on screen.
- **E** (or Space) takes the helm when standing at the wheel, and leaves it.
  At the helm, **W/S** move the throttle lever and **A/D** steer. The lever
  stays where you leave it, so the crawler drives on while you walk away.
- Walk out of the starboard hatch and down the **ramp** to go ashore. **E** at
  a highlighted wreck crate picks up its shown resource. Carry it back up
  the ramp to stow it in the hold. Away from a wreck, **E** harvests the nearest dry resource
  block within reach. Each harvest removes that block from the Basin. Carry
  one bundle at a time; the HUD and cargo art show its kind. A full hold
  refuses another bundle.
- **X / Drop** while carrying sets the crate down ashore for later pickup.
  Next to stored hold cargo, press twice to discard it.
- Swim to a **golden float** in permanent brine and press **E / Dive** for a
  shallow side-view wreck dive. Follow the yellow guide, manage air and recover
  one sealed ore bundle. Return to the guide at the surface and **E / Surface**
  puts it on your float. Swim home and board the ramp to stow it. **X / Esc / Abort**
  abandons the dive without the ore; running out of air does the same. Surfacing
  returns to your entry point in the water, never teleports aboard. The ebb does
  not drain permanent pools, and revisiting a recovered site leaves its cache empty.
- **Tab** keeps the map north-up everywhere.

On a phone, the controls appear with the first touch. Drag anywhere off the
buttons to walk (a ring shows where your thumb landed), and the same drag
works the throttle and rudder at the helm. **Use** takes the helm, leaves it
and gathers or picks up cargo, labelled for the available action. **Drop**
appears while carrying or beside a crate in the hold. **North / Follow**
changes the camera mode.

Walking the deck, the view turns with the crawler so its floor plan stays
upright and the salt turns past it. At the helm and ashore it stays north-up,
like a map. Walking zooms closer; the helm widens the view, and boarding restores
whole-crawler framing even on portrait screens. Stepping ashore drops the anchor, so the crawler waits for you.

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
- `scripts/gather.decay`: resource kinds, harvest targets and typed inventory.
- `scripts/wreck-field.decay`: deterministic wreck sites in 48-unit regions,
  streamed as prefabs around the crew. Wrecks beyond 120 units unload; their
  individual cargo mask is remembered for the voyage, so returning cannot refill them.
- `scripts/dive-sites.decay`: streamed permanent-water wrecks and their golden floats.
- `scripts/dive.decay`: shallow swimming entry, air, ore cache and return to the surface.
- `prefabs/sunken-wreck.prefab`: the submerged wreck marker.
- `prefabs/wreck.prefab`: the streamed wreck, generated by `art/scene.py`.
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
A worn runner and exposed heating pipes make the cabin feel inhabited. The
instrument panel stays compact until flood notices need more room. Two animated
tread belts read actual speed and steering through `scripts/tread.decay`; they
carry their visible upper run toward the bow in forward travel, pivot in
opposite directions, pause in a flood and resume after the ebb. The
starboard belt leaves a gap at the boarding ramp. Belts are built from one-tile
segments so construction can extend them without stretching the cleats. They
are visual children of the deck; logical tread cells still block walking.
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

Construction uses fixed blueprints rather than free placement; the bunk is
cosmetic and there is no save/load persistence yet. Ashore, the crew walks over the crawler's hull rather than round it, and the tuning
(`@export` fields on the crawler and the crew) is a first guess.

`tests/a_flood_season.rs` checks the rise, high water, ebb and next season,
cargo washed out once, driving again after the ebb, caught crew escaping with
keys and at 390×844 by touch, and unchanged terrain with builtin overland blocks.
The map extractor separately checks water at height boundaries and retention
of column chunks across flood changes (while block edits still invalidate them).

Gathering uses existing engine capabilities: `Grid.surface`/`height` to choose
resources, sparse `Grid.set_block` edits to harvest them, typed shared state
and deck tiles for inventory, and prefab spawning for wrecks. One authored
wreck remains near the start for the introductory salvage run. Rock columns
have a deterministic one-in-four chance of ore; other rock and reef yield
stone. Leaves yield fibre and expose the wood underneath. Resources do not
regrow during a voyage. `tests/gathering_fills_the_hold.rs` plays harvesting,
stowing and jettisoning with keys and phone touch.

## Build your home

Walk beside the workbench and press E (or Workbench on a phone). Q / Next
cycles plans, E / Build constructs, and X / Close or Escape returns to walking.
Gamepads use south to open/build, east to cycle and west to close. The crawler
continues at its existing throttle while you build.

| Blueprint | Cargo cost | Result | Permanent mass |
| --- | --- | --- | --- |
| Stern extension | 2 wood + 2 scrap | Two walkable rows, four cargo slots; maximum three extensions | +18 |
| Bunk | 2 wood + 2 fibre | One bed beside the bench | +4 |
| Stronger engine | 2 ore + 2 scrap | +35% engine speed and acceleration before weight and terrain | +12 |

Only bundles physically stowed in the hold pay for construction. Missing cargo,
a duplicate installation or the chassis limit consumes nothing. The centre
passage and boarding ramp remain clear; the deck origin stays fixed as the
stern, segmented treads, camera centre and shadow grow. Extra structure is
permanent weight, so more room has a movement cost. The starter grows from four
to sixteen cargo slots. `tests/a_home_grows.rs` plays gathering, salvage,
construction while driving and walking into the new space with keys and phone
touch; it also checks upgrades, duplicate refusal and the chassis limit.

These are Decay game rules using existing tile, UI and animation APIs; no new
engine host API or general construction editor is introduced.

## Shallow wreck diving

Permanent brine pools contain marked submerged wrecks. Swim from shore to a
golden float and press E / Dive within four units of its wreck. The hold needs
one free slot and your hands must be empty. Temporarily flooded land wrecks
remain shore salvage sites: swimming over a flat does not open a deep interior.

The view switches to an authored side-view wreck interior. WASD, the d-pad or
the touch stick swims; hull panels block movement. Air lasts thirty seconds
underwater and refills at the surface. Use E / Recover beside the sealed ore
cache, then return up the yellow guide and press E / Surface. The ore is
secured to a float alongside the swimmer; haul it back up the crawler ramp to
stow one real ore bundle. Carrying it slows both diving and surface travel.

X / Abort, Escape or empty air surfaces at the entry point without the ore.
The tide continues, but an ebb does not drain a permanent pool or recall its
diver. A successfully recovered cache stays empty for that wreck for the rest
of the voyage, including after its marker streams out and back. Shore crates
and underwater caches belong to distinct wreck sites.

This first slice uses one authored shallow wreck interior; it is not a
cross-section of generated voxel terrain. Diving rooms/bells, air tanks, fins,
lamps, deeper sites, different interiors and boat upgrades remain follow-up
progression. `tests/a_high_tide_dive.rs` plays the full trip on keyboard and
phone touch and checks hull collision, air loss, temporary-flat refusal,
ebb continuity and duplicate loot prevention across streaming.

## Persistence

There is no Low Tide autosave or load yet, on desktop or the web. Cargo,
construction, harvested terrain and depleted wrecks are remembered only while
the current session runs. Closing or reloading the game starts a new voyage.
The engine offers a versioned number/flag Save API, but Low Tide does not use
it. Full autosave needs a versioned voyage snapshot (including sparse terrain
edits and wreck history), project-specific host storage and reload tests. The
shared browser host currently uses a common save key; it must be isolated per
project before calling Low Tide's web progression persistent.

## Interface

The workbench shows all three blueprints together. Click or tap a row to
select it; the highlighted row shows availability before spending anything.
The details show what the upgrade adds, its permanent weight and how much
required cargo is aboard. Installed upgrades and a fully extended chassis
are marked explicitly. Build consumes actual hold bundles; missing materials
and duplicates still spend nothing. Q cycles plans, E builds and X / Escape
closes. Dedicated menu buttons also work with a mouse or phone touch.

While the menu is open, movement stops and the gameplay touch controls are
hidden; the voyage and tide continue. Closing restores them. The normal HUD
prioritizes hold space, speed, tide countdown, terrain and immediate danger;
the workbench carries the full material inventory. The North / Follow compass button sits beside the narrower status header. The menu and header fit the viewport width.

### Instrument and interaction styling

The opaque dark panels, brass accents and icon sheet share the crawler's
metal-and-wood palette. Hold segments represent actual occupied slots and
turn red when full; the speed meter shows speed relative to the current
maximum. The tide bar is the time remaining in the current phase, using its
authored duration. It does not predict water depth. Keyboard and phone layouts
use the same data; action buttons are centered inside the phone's context panel.

Near cargo, a gold outline marks the exact bundle the action targets and
the panel names its resource. Drop / X first asks for confirmation; a second
press within four seconds discards that same crate. Walking away, switching
targets, Escape or the timeout cancels. Diving Abort remains immediate.
Use appears only when a real interaction is available. North / Follow describes
the camera-mode action instead of a generic View label.

The frame fittings and lamp glows are authored decorative sprites, not new
dynamic lighting or additional furniture. The reference-inspired pass does
not add a minimap, arbitrary object movement or per-crate inventory screens.

### Physical wreck cargo

Each wreck carries three scrap crates and one each of wood, ore and fibre.
Walk within reach of the crate you want: its outline and contextual card name
its resource, and E / Pick up takes that specific crate. Carry one at a time.
X / Drop puts carried cargo on the ground immediately; it remains a real crate
and can be picked up again. Hold counts and weight change only when boarding
stows it. A full hold still refuses another pickup.

Taken wreck slots stay empty when their prefab unloads and returns. Wreck
crates are attached to their hull and become submerged during a flood; loose
crates set down ashore wash away once. These objects and their history last
for the current voyage only, with no desktop or web session save yet. The
side-view dive cache remains separate from the surface crate inventory.

## Next construction and exploration slices

These are follow-up work, not implemented upgrades:

1. **Construction preview:** choose a module and show its footprint on the deck before spending cargo. Expose occupied cells, access to the helm/ramp and missing materials before confirming. Begin with fixed attachment sockets; keep the existing hull origin stable and validate boarding after every extension.
2. **Diving room:** a visible module with air refill and a tether/float rack. Let it unlock deeper permanent-water sites and distinctive mechanical salvage. Basic shallow diving remains available from swimming; the room is an upgrade, never an entry gate for tidal flats.
3. **Places with character:** distinct shallow wreck interiors, named caches, visible crate choices and a few recoverable keepsakes for the cabin. Deep sites should provide useful parts unavailable on dry flats, rather than duplicate generic shore ore.
4. **Voyage saves:** versioned project-isolated desktop/web storage for hull modules, cargo, carried/dropped crates, tide, generated terrain edits and recovered sites. Restore safely across swimming/diving transitions, with restart and browser reload tests. The current voyage is session-only: there is no autosave yet.

The first slice deliberately fixes swimming, shallow diving and camera framing before deeper upgrade progression. The side-view interior is authored gameplay, not a geometrically extracted cross-section of the voxel pool. Caches and visited sites retain history only for the current voyage until the save slice is built.
