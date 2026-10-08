# Physics Playground

A feature example and a toybox: one room full of contraptions that show what
Sindri's physics can do, and tools to poke them with. Authored entirely in
scene data, Decay and Weave. Open `sindri.toml` in the editor or export it
with `sindri-export examples/physics`.

The design and its checklist are in
[`docs/physics-playground.md`](../../docs/physics-playground.md). The scene,
prefabs and material profiles are written by
`scripts/physics_playground/generate.py`; edit the numbers there and run it
again rather than editing the generated files.

## What is in the room

- **Wrecking ball**: wind it up with a hinge position motor, let it swing
  through a castle of wood, glass and steel, or cut the rope.
- **Gantry crane**: drive the trolley, lower the hook, grab whatever is under
  it with a joint made at runtime, and drop it somewhere worse.
- **Cannon gallery**: aim and fire through hanging glass panes; switch
  continuous collision off and watch shells ghost straight through.
- **Bumper pit**: flippers, bumpers that kick, and a ball lift that carries
  what drains back up to the top.
- **Material lab**: ice, wood and rubber race down one ramp; four pads show how
  high each material bounces.
- **Seesaw and trampoline**: drop the anvil, launch the ball, retune the
  trampoline's springs.
- **Test track**: a robot that climbs steps, rides a lift it calls by standing
  on it, drops through one-way planks, shoves crates and kicks them.
- **Domino run**: fourteen dominoes, and at the end a red button that sets off
  DROP EVERYTHING. A hundred balls can knock them over too.

## Controls

| Key | Button | What it does |
| --- | --- | --- |
| G / B / N / P | GRAB, BLAST, SPAWN, PROBE | Choose what the pointer does. N again picks what SPAWN drops; P again picks the probe's query. |
| L | MASK | Which layers the probe sees. |
| Q / E | < > | Fly to the previous or next contraption, or back to the whole room. |
| 1-4 | The four action buttons | Work the selected contraption. |
| A / D, W, S, F | LEFT, RIGHT, JUMP, KICK | Drive the robot on the test track. |
| X | DROP EVERYTHING | Every joint lets go. Again puts them back. |
| O | 100 BALLS | Pours a hundred balls in from the ceiling. |
| V | GRAVITY | Earth, moon, zero-g, upside down, sideways. |
| I | DEBUG | Outlines by state, contacts and normals, velocities, and every joint from anchor to anchor. |
| R | RESET | Resets the selected contraption, or the whole room. |

The probe is dragged: press where a query starts and let go where it points.
It casts a ray, a circle or a box, or counts what overlaps a circle, and the
readout under the title says what it hit, how far, and which way the surface
faces.

`game/tests/the_physics_playground_works/` plays it through the same session
the browser runs, and the browser smoke tests play it on desktop and phone.
