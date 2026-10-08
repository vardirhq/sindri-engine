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

One wide room, 60 by 28 world units, packed with contraptions that share the
space. The camera opens on the whole room and flies in to a contraption when
its chip is selected. Contraptions are scene entities rather than prefabs: an
authored joint can only name bodies in its own scene or prefab, and the crane
must be able to pick up anything the room was built with. Each part remembers
where the scene put it, so resetting a contraption puts its parts back and
turns its joints on again.

Gameplay is entirely Decay, layout and styling are Weave, and geometry is
scene and prefab data written by `scripts/physics_playground/generate.py`.
Where the public surface lacked something genuinely missing, the engine was
fixed rather than worked around, and the rest is recorded below and in
`docs/parity.md`.

## Contraptions

| Contraption | What you do | What it exercises |
| --- | --- | --- |
| Wrecking ball | Wind it up, let go, cut it loose | Hinge position motor, motor release, joint enable, compound bodies, a tower of mixed materials |
| Gantry crane | Drive the trolley, lower the hook, grab and drop | Slider position motor chasing a moving target (a velocity motor sags under load), spring winch (`set_spring`), a distance joint created at runtime, a box cast to find the load |
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

- **Grab** ties a kinematic hand to whatever is under the pointer with a short
  rope, so things swing from where you hold them and keep their speed when
  thrown.
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

A second scene, the Voxel Quarry, carries the 3D foundation. K or the 3D
QUARRY button goes there with `Scene.go`, and back; the room waits, switched
off, as it was. The floor is a `sindri.voxel_world` with stepped stone walls
and a voxel collider. Crates are 3D rigid bodies resting on it. Clicking the
ground digs out the block under the pointer; DIG UNDER finds the ground under
every crate with a 3D ray and opens a pit there, so each drops into its own
hole; DROP CRATES rains more in; RESET fills every hole and puts the crates
back. A laser sweeps across firing a 3D ray straight down and reads out the
block or crate it hits and how far.

## Known public-surface gaps found here

- Decay could not change world gravity, and a gravity change left sleeping
  bodies where they rested. Fixed here: `Physics.set_gravity` changes the
  authored world settings, and a change wakes every dynamic body.
- A path through an entity held in a struct field (`hit.entity.transform`)
  passed the checker and failed at runtime. Fixed in the Decay lowering.
- A switched-off scene's environment still counted, so a project whose two
  scenes each had one failed on `Scene.go`. Fixed: environments, like every
  other component, take part only while active.
- `project-capture` played only a project's main scene. Fixed: it loads every
  scene `sindri.toml` lists, as the export does.
- The layered voxel generator makes an endless plain, solid all the way down,
  so the quarry is a floor with walls raised by edits rather than an island.
  The game hosts do not bind the engine's built-in blocks, so the quarry ships
  its own block set and textures.
- Decay cannot ask whether a UI element is laid out yet, and `Ui.position`
  on one that is not stops the frame. Coming back from the quarry, the
  director waits a few frames before measuring the bars.
- The editor's own Play has no headless harness, so the editor proof is that
  it compiles every script and loads every prefab the scripts spawn; Play
  itself was not exercised here.
- Decay cannot change a collider's material at runtime, so material comparisons
  use separate authored bodies.
- There is no simulation time scale, and no in-game debug drawing of colliders:
  the overlay is drawn in Decay with pooled shapes. Decay cannot read a
  collider's shape or a joint's anchors, so the generator tags round colliders
  and hands the overlay every joint's bodies and anchors.

## Checklist

- [x] Room, camera and pointer mapping, tools, global controls and HUD
- [x] Wrecking ball and gantry crane
- [x] Cannon and glass gallery
- [x] Ball lift, hopper, bumper pit and flippers
- [x] Material lab, seesaw and trampoline
- [x] Test track robot, dominoes and red button
- [x] Probe tool and debug overlay
- [x] Voxel Quarry 3D annex
- [x] Session regressions, browser smoke on desktop and phone, editor load
- [x] Documentation: README, parity, capabilities, changelog, pre-alpha exit
