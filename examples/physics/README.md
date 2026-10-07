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

## Controls

| Key | Button | What it does |
| --- | --- | --- |
| G / B / N / P | GRAB, BLAST, SPAWN, PROBE | Choose what the pointer does. N again picks what SPAWN drops. |
| Q / E | < > | Fly to the previous or next contraption, or back to the whole room. |
| 1-4 | The four action buttons | Work the selected contraption. |
| X | DROP EVERYTHING | Every joint lets go. Again puts them back. |
| O | 100 BALLS | Pours a hundred balls in from the ceiling. |
| V | GRAVITY | Earth, moon, zero-g, upside down, sideways. |
| I | DEBUG | Draws colliders, contacts, velocities and joints. |
| R | RESET | Resets the selected contraption, or the whole room. |

`game/tests/the_physics_playground_works/` plays it through the same session
the browser runs, and the browser smoke tests play it on desktop and phone.
