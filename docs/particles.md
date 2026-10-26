# Particles

`sindri.particles` is an emitter: while it plays it keeps throwing particles
from where its entity stands. Its particles are flecks in the same pool a
`sindri.effect.burst` throws into (`docs/effect-scaling.md`), so they cost what
flecks cost: no entities, no components, nothing a script can hold or anything
can collide with. They draw like world sprites, through the same batching, and
2D lighting reaches them.

## Fields

| Field | Meaning | Default |
| --- | --- | --- |
| `playing` | Whether it is emitting | `true` |
| `texture` | The image every particle draws; one batch for the emitter | — |
| `rate` | Particles a second while playing | `10` |
| `max` | The most of its particles alive at once | `200` |
| `lifetime`, `lifetime_spread` | Seconds each lives, varied by that fraction of itself | `1`, `0.2` |
| `speed`, `speed_spread` | World units a second, varied likewise | `2`, `0.3` |
| `direction` | Degrees they leave in: 0 right, 90 up | `90` |
| `spread` | How wide a cone round `direction`, in degrees; 360 is every way | `30` |
| `shape` | Where they start: `point`, `circle` (inside), `box`, `ring` (on the edge) | `point` |
| `offset` | Where the shape sits from the entity: a runner's feet, not its middle | `[0, 0]` |
| `shape_size` | The shape's width and height; a circle or ring reads the width | `[1, 1]` |
| `gravity` | Added to each particle's velocity every second | `[0, 0]` |
| `drag` | How much of its speed a particle loses each second | `0` |
| `size`, `size_end` | Its size when thrown and when it dies, blended over its life | `0.1`, `0.1` |
| `colors` | Colors evenly spaced over a particle's life, blended between: two is a fade, three a flare that cools | white to clear |
| `random_rotation` | Whether each starts turned a random way | `false` |
| `spin` | Degrees a second it turns | `0` |
| `space` | `world`: particles stay where thrown (a trail). `local`: they move with the emitter (a halo) | `world` |
| `layer` | The sprite layer they draw on | `0` |

An emitter owes `rate` times each step's length and throws the whole particles
of it, carrying the remainder, so a rate of 2.5 a second really is 2.5 a second
at any frame rate. An emitter whose numbers cannot be used (a lifetime of zero,
a negative size, anything not finite) throws nothing, and the step reports it
naming the field.

## Determinism

Particles draw their directions, speeds, lifetimes and starting places from the
effect pool's own random stream, never the run's. Turning an emitter up cannot
change which enemies spawn, and a seeded run is the same run however much dust
it kicked up.

## In Decay

| Call | |
| --- | --- |
| `Effects.play(entity)` | Starts the entity's emitter (writes `playing`) |
| `Effects.stop(entity)` | Stops it; particles already out live out their life |
| `Effects.emit(entity, count)` | Throws that many at once, whether or not it plays: dust on landing |

## In the editor

Add Component offers Particles. While nothing is playing, the selected emitter
runs in the Scene view, and only that one: a scene full of chimneys would
otherwise smoke the whole time anyone edited it. Selecting something else starts
from nothing.

## Limits

One texture per emitter and no sheet animation over a particle's life. No
sub-emitters, trails or collision. A pool holds 8,192 flecks and particles
together; past that the oldest go first and the overflow is counted.

## Proof

The platformer: every coin sparkles from `prefabs/coin.prefab`, and the hero
trails dust while it runs and puffs on a hard landing, switched by
`scripts/hero.decay`. `games/lamplighter/tests/sparkle_and_dust.rs` checks both. Low Tide's treads throw
dust and its engine smokes while the crawler drives, and a swimmer and a diver
throw spray and bubbles (`games/low-tide/tests/the_basin_is_lit.rs`).

## Particle Lab

Particle Lab is an editor window and a browser tool. The preview advances the
same `Effects2d` emitter simulation as a game, using the live values in the
form. Six starter presets, a pause control, bursts, and background colors help
with exploration. The browser copies or downloads a `sindri.particles`
component as JSON and can reload pasted component JSON. In the editor, select an entity with a Particles component,
open Particle Lab and use Apply to write a validated component in one undoable
edit. A changed selection prevents accidentally applying to a different entity.

The editor loads the selected emitter’s project texture when available. Drop an image onto the preview, or use the browser image picker, to preview another texture on rotating particles. Without an
image, the preview uses circles. It does not reproduce scene lighting, so check
the final look in the Scene view. A texture reference must name an image in the
project; preset selection preserves the current reference. The emitter has the
same limits as the rest of the engine, described above.
