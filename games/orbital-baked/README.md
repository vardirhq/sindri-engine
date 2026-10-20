# Orbital Last Stand

The fleet is gone. Someone has to hold the line.

A space survival game about building an unlikely warship while an evacuation
corridor closes. Survive five sectors over ten minutes, or take a field kit into
a twelve-boss gauntlet. Firing is automatic; movement, timing your burst and
choosing a build make the difference.

```bash
cargo run -p sindri-player -- games/orbital-baked
```

Open the `games/orbital-baked` project folder in the editor to edit the same
scenes, scripts and assets.
Export with `sindri-export` to play through the shared WebGPU browser player.
The package name `orbital-baked` is historical.

## Your flight deck

- Four ships: balanced **Strider**, armored **Bastion**, agile **Kestrel** and
  **Oracle**, which starts with an Arc system and faster research. Unlocks buy
  alternative starting builds, rather than permanent damage upgrades.
- Story, Standard and Veteran change population pressure and incoming damage.
- 160 modules, 19 synergy recipes, 15 enemy types and 12 distinct bosses.
  The archive describes the actual catalog and remembers discoveries.
- Salvage earned by flying unlocks hulls. Eight commendations reward milestones.
  The flight log keeps six recent sorties, career totals and personal bests.
- A complete debrief supports another sortie or a return to the flight deck.
  Boss Rush ends after twelve bosses, starting at the selected roster entry.
- Music and effects mixers, compact HUD, first-flight guidance, reduced motion,
  reduced flashes and a persistent burst key binding.

Progress is saved on this device: native project saves or browser local storage.
Pilot names, hulls, discoveries, settings and completed sorties persist. An
in-progress battlefield is not a resumable save. Abandoning settles a loss;
flights under fifteen seconds earn no salvage.

## Controls

| Action | Keyboard | Controller | Touch |
| --- | --- | --- | --- |
| Steer | WASD / arrows | Left stick | Drag in the arena |
| Evasive burst | Left Shift, rebindable in Settings | Right bumper | BURST |
| Pause / resume | Escape | Start | PAUSE / Resume |
| Menus | Pointer or focus navigation | D-pad / south | Tap |

The burst briefly accelerates the ship and grants a short invulnerable window,
then recharges over 3.5 seconds. Collect glowing cores to research modules.
Choose one offer at a level-up; each sortie has two redraws. The Boss Rush field
kit supplies extra hull, faster fire, Arc and Nova from the start.

## Art and sound

Midnight space, mint instruments and warm planetary light frame readable vector
combat. Ship silhouettes, hull illustrations, original corridor key art, drifting
star layers, sector colors and restrained bloom share one palette. Three
composed stereo tracks crossfade between the flight deck, combat and boss fights.
See [asset sources](assets/art/README.md) for editable art and Sound Lab sources.

## Authoring and proof

Orbital remains Sindri's forcing function: all gameplay lives in Decay, data in
profiles, reusable objects in prefabs, and the interface in Weave. This edition
uses the engine's current audio buses, profiles, action rebinding, persistent
saves, controller input and responsive UI. It added general Decay text-save
access for pilot names and sortie history. No game rules live in its Rust host.
The earlier recreation plans remain historical reference; this edition's design
and acceptance are in [the release brief](../../docs/orbital-release.md).

`the_complete_game` runs real menu, purchase, progression, controller, burst,
replay and save flows. `the_build_has_teeth` exercises module benefits, difficulty,
all synergy recipes and an authored area attack. Existing tests exercise every
boss, attack family, physics, upgrade pool and campaign progression.
`the_ui_fits_every_screen` checks layout at laptop, desktop, portrait phone,
landscape phone and tablet sizes. `scripts/browser/orbital-complete.mjs` runs the
exported WebGPU game through desktop and HiDPI touch purchase-to-debrief flows
and verifies persistence after reload.

The capture tool opens the real project and presses real controls:

```bash
cargo run -p orbital-baked --bin orbital-baked-capture -- title.png 1280 720 title
```

Other captures include `hangar`, `archive`, `records`, `settings`, `pause`,
`debrief`, `spectacle`, `lab` and `boss-0` through `boss-11`.
