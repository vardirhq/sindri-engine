# Orbital Last Stand

Orbital Last Stand, drawn as neon vector shapes the way the original
(`MadsenDev/tester-repo`) draws it. The crate is still called `orbital-baked`
because for a while every hull here was a baked 3D sprite sheet; that look was
softer and lower in contrast than the original's, so it was redrawn with the
engine's shape renderer. Only the mine blast is still baked.

It is the forcing function described in `AGENTS.md` — a game built only through
the editor and Decay, whose job is to find what the engine cannot do yet. The
plans, audits and evidence written against earlier versions are kept in `docs/`
as history.

## How it is drawn

Every body is a `sindri.shape`:

- **Enemies** are filled polygons in the original's warm colours, with as many
  sides as the original gives each kind. A hit flashes the fill white; the
  charger heats from amber to white as it winds up.
- **Asteroids** author eight jittered points with `World.set_shape_point`, so
  no two rocks share an outline, and brighten their rim as they crack.
- **The ship** is the original's dart with an inner chevron.
- **Bosses** are one polygon each, from five sides for the Harrower to sixteen
  for the Last Light, with a counter-turning rim and core (`boss-ring.decay`)
  that thickens and pulses below half health. The Spine sheds sides as its
  plates go: ten, eight, five.
- **The backdrop and grid** take each sector's colours from the original and
  drift rather than turn.

Colours are the original's hex values converted to linear light, because the
renderer blends in linear and an sRGB value written straight in comes out pale.
Bloom supplies the glow the original's canvas draws with shadow blur.

The mine blast (`textures/detonation.isobake`) stays a baked sheet played by
`sindri.animation.sprite`, and flashes white-hot through the sprite colour
offset. Re-bake it with:

```
cd tools/isometric-baker
node src/cli.ts ../../games/orbital-baked/assets/textures/detonation.isobake \
  --out ../../games/orbital-baked/assets
```

## Seeing a boss on purpose

The bosses arrive on a timer, so most of them are a long wait away. The title
screen's rush mode opens on whichever boss its picker is showing, and the
capture binary can do the same from the command line:

```
cargo run -p orbital-baked --bin orbital-baked-capture -- \
  aegis.png 960 540 boss-11
```

`boss-0` is the Warden and `boss-11` is the Aegis; the shot presses the picker
that many times, starts the rush, and photographs the fight seven seconds in.
The roster in between is the Harrower, Prism, Singularity, Crown, Brood, Mirror,
Architect, Spine, Leviathan and Last Light, in that order.

## A thing that only shows up on a boss

The Aegis's shield plates were spawned as children of the hull, which is how a
prefab says "these belong to it" and is what `docs/prefabs.md` describes. It
drew and collided at the arena's origin instead, because a world sprite and a
physics body both read an entity's own transform and neither folds a parent's
into it — shape extraction does, which is why the elite markers that had the
same bug were fixed and this one was not noticed. `docs/physics.md` already says
not to parent a body; the plates are siblings now and follow the boss by reading
where it is. Worth knowing before parenting anything that has to be *hit*.
