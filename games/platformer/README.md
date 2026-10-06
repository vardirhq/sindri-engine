# Platformer

The first genre showcase: a small side-view platformer. Run and jump across a
painted level, pick up the coins and reach the flag.

K or the west controller button tosses the wooden crate when nearby. Wind
pushes it along the ground; an off-centre kick makes it tumble. Its forces and
rotation are driven in Decay, with physics writing the resulting pose back.

L releases or reconnects the hanging lantern; T reels its tether in or out.
R switches its tether between two hooks, including while released.
Z cuts the cord; later tether controls do nothing until C repairs it.
C creates a fresh owned distance joint at the selected hook and tether length.
The cord disappears while the constraint is suspended.

B rebuilds the trolley light's spring at its current rest length; Decay continues
tuning it while the trolley remains on its independent rail.

H rebuilds the placed windmill hinge and restarts its motor, keeping the axle
fixed and the separately spawned mechanism independent.

V places or removes a second powered windmill from a reusable prefab. Its hinge
references its own root axle and rotor, and Decay reverses the motor.

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
- a **dynamic body** with a capsule collider and a **foot sensor** for pickups, plus
  **solid support contacts** for standing so plank undersides never grant a jump;
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
a fixed axle and removal. The level places `windmill-kit.prefab`, which nests
`windmill.prefab`; V spawns that same assembly. Copy these prefabs with the script
for a powered rotating part. The authored hinge keeps its original local root
reference through placement, export and runtime spawning.

The lantern trolley travels along an authored slider rail and reverses through
`Physics.set_slider_motor`. Its hanging light uses a force-based spring; Decay
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
