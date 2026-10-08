# Physics Playground redesign

`examples/physics` is the feature example for the physics system. After the
Physics Patch it was still a query lab with two falling bodies. This document is
the design for its replacement, and its checklist moves with the work.

`docs/pre-alpha-exit.md` asks that the showcase present physics "as something
worth playing with, not merely a collection of rectangles politely falling onto
other rectangles". The measure is three people:

- someone new to Sindri opens it and can tell the engine has a serious physics
  system;
- someone who knows physics engines can experiment with it and inspect what
  happens;
- someone with the maturity of a twelve-year-old presses DROP EVERYTHING and
  enjoys what follows.

## Shape

One wide room, about 60 by 28 world units, packed with contraptions that share
the space. The camera opens on the whole room and flies in to a contraption
when its chip is selected. Each contraption is a prefab, so resetting one means
despawning it and spawning it again, and every joint in it is authored once.

Gameplay is entirely Decay, layout and styling are Weave, and geometry is
scene and prefab data written by `scripts/generate-physics-playground.py`. No
Rust is added for the playground. Where the public surface lacks something, the
workaround uses public authoring, and the gap is recorded in `docs/parity.md`.

## Contraptions

| Contraption | What you do | What it exercises |
| --- | --- | --- |
| Wrecking ball | Wind it up, let go, cut it loose | Hinge position motor, motor release, joint enable, compound bodies, a tower of mixed materials |
| Gantry crane | Drive the trolley, lower the hook, grab and drop | Slider velocity motor, spring winch (`set_spring`), joints created at runtime, overlap queries |
| Cannon and glass gallery | Aim, fire, switch CCD off and watch shells ghost through glass | Hinge position motor aim, impulses, continuous collision, thin hinged panes, sensors |
| Ball lift and bumper pit | Run the lift, open the hopper, work the flippers | Slider position motor, trapdoor hinge, contact-driven bumper kicks, flipper limits, restitution |
| Material lab | Race blocks down ice, wood and rubber; drop balls on four pads | Physics material profiles, friction, restitution |
| Seesaw and trampoline | Drop the anvil, launch the ball, bounce on the springs | Hinge limits, spring joints, mass |
| Test track | Drive the robot up steps and down a slope, ride the lift it calls by standing on it, drop through planks, shove and kick crates | Character 2D steps, slopes and snap, one-way platforms, kinematic platform carry, impulse at a point, the room's gravity read with `Physics.gravity` |
| Dominoes and the red button | Knock the first one over | Stacking, contacts, a spring-loaded slider button that sets off DROP EVERYTHING |

The contraptions are connected: balls drain along the floor to the lift, the
lift tips them into the bumper pit, the bumper pit drains back to the floor,
and the domino run ends on the button that wrecks the room.

## Tools and global controls

The pointer always holds one tool:

- **Grab** joins a kinematic hand to whatever is under the pointer with a hinge
  created at the grabbed point, so things swing from where you hold them and
  can be thrown.
- **Blast** applies radial impulses at points inside an overlap circle.
- **Spawn** drops the selected object: ball, crate, plank, anvil or bowling ball.
- **Probe** is the old playground's query lab: drag a ray, circle cast, box cast
  or area query and see the hit, its normal and its distance, filtered by layer.

Global buttons: DROP EVERYTHING (disable every joint in the room), 100 BALLS,
GRAVITY (earth, moon, zero-g, upside down, sideways), DEBUG and RESET. Each has
a key; the bar is touch-sized.

DEBUG draws, without replacing the normal look, collider outlines coloured by
state (resting, moving, static, sensor), contact points with normals scaled by
their impulse, velocity arrows, and every authored joint from anchor to anchor
(green while on, red once cut), plus a live count of what is in view, awake,
touching and joined. The probe reads out what it hit, where, how far and the
surface normal; P cycles its query and L (or the MASK button) its layers.

## The 3D annex

A second scene, the Voxel Quarry, carries the 3D foundation: a voxel island
with a voxel collider, crates and barrels dropped onto it, holes dug under them,
and a 3D ray that reports what it hits.

## Known public-surface gaps found here

- Decay could not change world gravity, and a gravity change left sleeping
  bodies where they rested. Fixed here: `Physics.set_gravity` changes the
  authored world settings, and a change wakes every dynamic body.
- A path through an entity held in a struct field (`hit.entity.transform`)
  passed the checker and failed at runtime. Fixed in the Decay lowering.
- Decay cannot change a collider's material at runtime, so material comparisons
  use separate authored bodies.
- There is no simulation time scale, and no in-game debug drawing of colliders:
  the overlay is drawn in Decay with pooled shapes. Decay cannot read a
  collider's shape or a joint's anchors, so the generator tags round colliders
  and hands the overlay every joint's bodies and anchors.

## Checklist

- [ ] Room, camera and pointer mapping, tools, global controls and HUD
- [x] Wrecking ball and gantry crane
- [x] Cannon and glass gallery
- [x] Ball lift, hopper, bumper pit and flippers
- [x] Material lab, seesaw and trampoline
- [x] Test track robot, dominoes and red button
- [x] Probe tool and debug overlay
- [ ] Voxel Quarry 3D annex
- [ ] Session regressions, browser smoke on desktop and phone, editor Play
- [ ] Documentation: README, parity, capabilities, changelog, pre-alpha exit
