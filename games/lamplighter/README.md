# Lamplighter

The platformer genre showcase, becoming a short real game: Wick, a courier,
lights a valley's lanterns at dusk (see `docs/lamplighter-plan.md`). The title,
level select, settings and pause shell are in place; Dusk Road is the first
level being built around the former physics showcase.

A short side-view platformer. Guide Wick across Dusk Road, light its three
checkpoint lanterns, gather optional embers, and touch the beacon at the end.
Falling sends Wick back to the last lantern.

Escape (or a pad's Start) pauses.

K or the west controller button tosses the wooden crate when nearby. Wind
pushes it along the ground; an off-centre kick makes it tumble. Its forces and
rotation are driven in Decay, with physics writing the resulting pose back.

L releases or reconnects the hanging lantern; T reels its tether in or out.
R switches its tether between two hooks, including while released.
Z cuts the cord; later tether controls do nothing until C repairs it.
C creates a fresh owned distance joint at the selected hook and tether length.
The cord disappears while the constraint is suspended.

O parks the trolley at a fixed place along its rail or releases it to reverse.
J rebuilds the trolley slider and restores its selected drive, retaining
its travel limits and independently owned spring.

B rebuilds the trolley light's spring at its current rest length; Decay continues
tuning it while the trolley remains on its independent rail.

P toggles both windmills between holding an angle and reversing.
H rebuilds the placed windmill hinge and restores its selected drive, keeping the axle
fixed and the separately spawned mechanism independent.

V places or removes a second powered windmill from a reusable prefab. Its hinge
references its own root axle and rotor, and Decay reverses the motor.

Arrow keys or A/D run; Space, W or Up jumps. A controller can use the left
stick or D-pad and the south face button. Down/S or D-pad down drops through
the raised one-way planks. On a touch screen, drag the virtual stick on the left side to steer and hold
the jump button on the right. Both work at once. Controls appear when touch
input is detected. A tap
is a hop and a held press a full jump; a jump pressed just before landing, or just after
running off a ledge, still counts. Horizontal movement accelerates and
decelerates rather than snapping between stopped and full speed, with
deliberately lighter air control after committing to a jump. Two low stone steps
and an inclined boardwalk near the start can be walked over without jumping.
The controller steps onto each riser and follows the boardwalk downhill; input
continues building intended speed while collision clips the actual movement.
A plank ferry crosses the first gap and returns. Jump aboard and stop running
to ride it; jumping leaves its carry, and Down drops through it into the pit.

Touch controls use a virtual stick on the left and a jump button on the right.
The stick appears under the left thumb; hold it while pressing jump with a
second finger. They appear after touch input is detected.

## The shell

The game starts on the title screen (`title.scene`). **Play** begins the level
the player was last in, over; **Levels** (`levels.scene`) lists the eight
levels, shows which beacons are lit and how many embers were found, and opens
any that is built; **Settings** (`settings.scene`) sets the music and effects
volumes in tenths and turns reduced motion on. Escape, a pad's Start button or
the drawn **II** button pauses a level into `pause.scene`: the level is switched
off behind it exactly as it was, and **Resume** switches it back on where it
stopped. Lighting the beacon saves the level, shows the banner, and returns to
the level select.

Four scenes and one script file (`scripts/shell.decay`) are all the shell is;
`art/dusk/shell.py` writes the scenes. What a person has done is kept with
`Save`, under plain names: `beacon_N`, `embers_N`, `level`, `volume_music`,
`volume_effects`, `reduced_motion`, and `return` (the screen Settings goes back
to). `tests/the_shell.rs` plays the route between the screens by pressing the
buttons that are drawn.

## What it is made of

There is no game code. The game is `assets/`:

- `title.scene`, `levels.scene`, `settings.scene`, `pause.scene`: the shell,
  written by `art/dusk/shell.py`; the title uses the pixel-art wordmark in
  `textures/lamplighter-logo.png`.
- `level-1.scene`: Dusk Road, its checkpoint lanterns and beacon, the embers,
  the hero and the HUD.
- `prefabs/coin.prefab`: what an ember is — its spinning sprite, sensor and tag.
  The scene places ten optional embers this way, so a change to the prefab is a
  change to each one.
- `scripts/hero.decay`: keyboard, controller and touch movement, jumping,
  embers, lantern contact and falling off.
- `scripts/checkpoint-lantern.decay`: lantern lighting, checkpoint updates,
  the light flare, sparks, and the end beacon.
- `scripts/hud.decay`: the ember count, the beacon banner, the pause button,
  music playback, and saved progress.
- `scripts/shell.decay`: the title, the level select, the pause screen and the
  settings.
- `audio/`: the jump, landing, ember and beacon sounds, plus the Tide Song
  loop. Sound Lab source files sit beside their exported audio; `sindri.toml`
  lists every clip the scripts play by name.
- `textures/`: pixel art drawn by `art/draw.py`, deterministic, so running it
  again changes nothing unless the drawing did. It draws `tiles_n.png` too,
  the tiles' normal map, from the same brick, plank and dirt layouts.
  `hero_n.png` was not drawn: the editor's Normal map tab made it from
  `hero.png`, and `hero_n.normalmap` holds the settings that make it again.

It uses, with no Rust of its own:

- a **tilemap** painted in the editor, made solid by a **Tilemap Collider 2D**,
  with grass tufts left passable;
- the scene's own **gravity**, from a **Physics 2D World**, for the dynamic crate;
- a **Character 2D** controller with one ground-filtered capsule probe and
  separate body/foot pickup sensors; **controller support** grants standing so
  plank undersides never grant a jump; Decay owns velocity, gravity and jump policy;
- visible **one-way planks** with local support normals and timed drop-through;
- **sprite animation** clips for idle, run, jump and fall;
- **keyboard, gamepad and touch input** through one movement path, including
  Sindri's built-in touch stick;
- a **camera** that follows the hero and stays inside the level;
- **screen text** filled from Decay;
- **sound effects** played from Decay on a jump, a hard landing, a coin and the
  flag;
- **2D lights**: a sun high over the level that casts shadows, a stone cave
  whose roof keeps the daylight out, two lanterns that cast the pillar's shadow
  and a low wall torch raking the normal-mapped bricks;
- **particles**: coins sparkle and the hero kicks up dust;
- **terrain**: the level's ground is painted with the `ground` terrain, so
  every open top grows grass and every gap shows a rim of earth;
- **nine-slice**: the HUD frame and the signboard by the first gap are one
  24-pixel panel, sliced so each keeps its wooden border.

## Playing it

Open `assets/level-1.scene` in the editor and press Play: the Scene view
opens in 2D, framed on the game's camera. It is also exported to the site at
`examples/lamplighter/`.

## Checked, not just run

`tests/a_run_reaches_the_flag.rs` stands the hero on the painted ground, has a
player hold right and jump at every gap and wall until it reaches the flag
without falling, hearing the jump, landing, coin and flag sounds on the way,
and checks the camera follows. `src/lib.rs` is the harness
that plays it without a window, built from the same public pieces a host uses.

`tests/cave_lights.rs` checks that the roof keeps the sun out of the cave,
that the lanterns warm it, and that the pillar shades its far side from one.
`tests/sparkle_and_dust.rs` checks the coins' sparkle and the hero's dust, and
`tests/sliced_panels.rs` that the HUD frame and signboard are drawn sliced.
`tests/painted_ground.rs` checks every ground cell is what the terrain paints.

`tests/one_way_platforms.rs` jumps through the authored planks, lands on top,
drops through using the real input action, stops on the ordinary painted floor
and lands on the planks again after the drop timer expires.

The hero now grants jump permission from solved solid contact normals, retaining
its ray for the ground-clearance display. The wind crate flashes amber on hard
landings from contact impulse. Both policies live in Decay and exercise the
general contact snapshot API added for this genre showcase.

The crate and planks share `assets/materials/wood.profile`, a reusable physics
material added for this showcase. The planks explicitly override restitution
to zero; the crate keeps the shared bounce. `tests/physics_materials.rs` checks
that both use the asset and that changing its restitution changes the crate's
rebound. Copy the profile and material component with the scene when using this
project as a starting point.

A lantern hangs from a scene-authored distance joint near the raised planks.
Decay applies the wind and draws the cord from the solved body positions;
`tests/distance_joints.rs` verifies motion, the distance bound and releasing the
tether. The separate tether entity is the pattern to copy for a distance joint.

The windmill has a separate hinge entity connecting its fixed axle and physical
rotor. Decay reverses its torque-capped motor every two seconds with
`Physics.set_hinge_motor`; `tests/hinge_motor.rs` observes both directions,
a fixed axle and removal. P toggles both windmills between holding 0.6 radians
through `Physics.set_hinge_position_motor` and their reversing velocity drive.
H recreation restores the selected mode through the windmill script. Tests
exercise both modes and a rebuild while holding. The level places `windmill-kit.prefab`, which nests
`windmill.prefab`; V spawns that same assembly. Copy these prefabs with the script
for a powered rotating part. The authored hinge keeps its original local root
reference through placement, export and runtime spawning.

The lantern trolley travels along an authored slider rail and reverses through
`Physics.set_slider_motor`. O holds it at a signed offset of 0.5 world units
using `Physics.set_slider_position_motor`; J retains the selected mode.
`tests/slider_spring.rs` also proves parking, rebuilding and resumed reversal
while retaining the independent light. Its hanging light uses a force-based spring; Decay
changes its rest length with `Physics.set_spring` and draws the cord from solved
positions. `tests/slider_spring.rs` checks bounded reversal, the changed light
height and independent joint removal. Copy these separate owner entities for a
linear mechanism or a damped suspension; keep motion rules in Decay.

`tests/spawned_joints.rs` places, reverses and removes the reusable windmill twice
through Decay, checking its fixed axle, isolated joint and fresh spawn lifecycle.

`tests/joint_controls.rs` cuts the cord while preserving the lantern body and
other constraints, then checks later controls stay inert. Repeated C repairs
recreate the constraint and visible cord while retaining hook and length choices.
It retargets the lantern between hooks, including while suspended, retunes its tether, releases it into free fall,
reconnects it and returns to the original length through keyboard-driven Decay.

`tests/saved_joints.rs` saves and reopens a Decay-spawned nested windmill through
registry-based reference remapping, then checks both motor directions, its fixed
world-space axle and independent constraint ownership.

`tests/hinge_motor.rs` also rebuilds the placed hinge twice with H while a second
windmill is present, checking scoped endpoints, fixed axle and continued reversal.

`tests/slider_spring.rs` also rebuilds the spring with B in both rest-length
phases, checking stable ownership, bounded trolley motion and continued tuning.

`tests/slider_spring.rs` rebuilds the slider twice with J, checking bounded
reversal, stable endpoint references and the independently owned suspension.

The [editor save regression](../../editor/tests/scene_joint_saves.rs) also runs
this project's real Decay windmill setup, then saves/reopens its spawned nested
mechanism through editor Save As and proves motor reversal around a fixed axle.
Editor authoring saves remap registered entity references automatically after
stable IDs are assigned; gameplay world snapshots remain a separate gap.
