# 2D lights and shadows

A 2D scene can be lit: the environment sets an ambient color, point lights in
the plane add their own, normal maps let a low light rake across the art, and
occluders cast shadows. This is the contract for what each piece means, what
is lit, and where the limits are.

## Turning it on

```json
"sindri.environment": {
  "lighting_2d": { "enabled": true, "ambient": [0.3, 0.3, 0.42] }
}
```

Off by default, and off means every picture is its own color, exactly as
before. On, every lit pixel is multiplied by the light reaching it: the
ambient color plus each light's contribution. The clear color is never lit.

What is lit: world sprites, tilemaps, voxel worlds viewed as maps, flecks and
world shapes. A sprite opts out
with `"unlit": true` (a glow, a pickup that should read in the dark); an
additive shape is light itself and is never darkened. The UI and screen-space
content are never lit.

## Lights

`sindri.light2d` is a point of light at its entity's world position:

| Field | Meaning | Default |
| --- | --- | --- |
| `enabled` | Whether it lights anything | `true` |
| `color` | RGBA; alpha scales the intensity, so a script can fade it like a sprite | warm white |
| `intensity` | Multiplies the color; above 1 brightens past the art | `1` |
| `radius` | How far it reaches, in world units | `4` |
| `falloff` | How it fades toward the radius: `pow(1 - d / radius, falloff)`. 1 is linear, 2 quadratic, near 0 nearly flat | `1.5` |
| `height` | How far above the plane it sits, read only by normal-mapped art | `1` |
| `shadows` | Whether occluders block it | `false` |
| `shadow_reach` | How far past an occluding edge light still reaches, so the face of a wall the light falls on is lit | `0.1` |

It is its own component rather than a kind of `sindri.light`: a 3D light shades
surfaces by the way they face, and a 2D light lights pictures, which face only
the viewer. A frame carries at most 16 lights (`MAX_LIGHTS_2D`): the ones whose
reach comes nearest the camera, so a level can hold many more than it ever shows
at once. A light that is not finite, or has no radius or falloff, is refused and
named.

A large, nearly flat light high over a level (`radius` 80, `falloff` 0.05) is a
sun. With `shadows` on, a roof keeps it out of a cave.

## Normal maps

A sprite or a tilemap may name `normal_map`: a second image laid out exactly
like its texture, whose colors are the directions the surface faces (red is
right, green is up the world, blue is out of the picture; the usual
"OpenGL" convention). The same sprite rectangle reads both images, so a sheet
and its normal map are drawn as one. Rotating or flipping a sprite turns its
normals with it.

With a normal map, a light's contribution is also multiplied by how squarely
the surface faces it, `max(dot(normal, normalize(to_light, height)), 0)`. A low
light rakes across the art and shows its relief; a high one lights it almost
flat. A sprite with no normal map is lit by distance alone, exactly as without
one. Sprites with different normal maps are different batches.

A voxel world viewed as a map draws each column from whichever texture its
block's top face names, so it names its normal maps by texture instead:

```json
"sindri.voxel_world": {
  "view": "map",
  "normal_maps": { "textures/terrain.png": "textures/terrain_n.png" }
}
```

A square whose texture has no entry, or whose normal map has not bound, is lit
by distance alone. The flood overlay reads the normal map of the texture its
water block draws from.

The platformer's `art/draw.py` writes `textures/tiles_n.png` from the same
brick, plank and dirt layouts as `tiles.png`, as a worked example. A map need
not be drawn: the editor's Normal map tab makes one from a sprite and lights it
with these same lights as it is tuned (`docs/normal-map-studio.md`); the
platformer's `hero_n.png` was made there.

## Shadows

`sindri.light2d.occluder` blocks lights that cast shadows:

| `shape` | Edges |
| --- | --- |
| `box` | A rectangle of `size`, centered on the entity |
| `polygon` | The closed outline through `points`, at least two |
| `tilemap` | The solid cells of the tilemap on the same entity, merged into boxes exactly as `sindri.physics2d.tilemap_collider` merges them; that collider's `passable` sprites let light through too |

All in the entity's own space, placed by its world transform. Each shadow
casting light gets a one-dimensional polar depth map, built on the CPU from
every occluder edge within its radius: for each of 256 directions, the distance
to the nearest edge. A pixel further than that, past the light's
`shadow_reach`, is in shadow; three neighbouring directions are averaged, which
softens the edge by about a degree and a half. Built on the CPU because it is
the same arithmetic on every host and testable without a GPU
(`Lighting2d::at` answers what a point receives).

Limits: the maps are rebuilt every frame from every occluder, which is cheap
for a handful of lights and a level's few dozen merged boxes and is not meant
for hundreds of shadow casting lights. A shadow's edge resolution is angular,
so a light far from what it shadows draws a softer, coarser edge. Occluders
block light only; they do not darken themselves beyond `shadow_reach`, which is
why a sun's reach is set to the depth of the ground it lights.

## In the editor

Create offers Light 2D, and Add Component offers Light 2D and Light Occluder
2D. The Scene view draws every light's
radius and every occluder's outline; a selected light's radius and a selected
box occluder's size drag by their handles as one undo step. Polygon points are
edited in the inspector.

## In Decay

`this.light2d` and `this.entity.light2d` read and write `color` (and its
`r`, `g`, `b`, `a`), `intensity`, `radius`, `falloff` and `height`. A light is
switched off with its entity's `set_active`.

## Proof

The platformer's cave: a stone roof and back wall, two hanging lanterns and a
low wall torch, with the level's tiles normal-mapped. The sun casts the roof's
shadow over the cave, the lanterns cast the pillar's, and the torch rakes the
bricks. `games/platformer/tests/cave_lights.rs` checks the shadows,
`crates/sindri-gpu/tests/sprite_lights.rs` reads the pixels back.

Low Tide lights a voxel map (a genre showcase using a capability that existed;
normal maps on a map's squares were added for it). Its sun is one wide light
kept to the north-west of the camera and recoloured by `scripts/daylight.decay`
as the tide turns: day at low water, a golden hour as it runs out, moonlight at
high water. The crawler's cabin lamps, headlights and the crew's lantern come on
at dusk, and a wreck's box occluder throws a shadow in the headlights.
`art/normals.py` writes every lit picture's normal map from the picture itself,
reading brightness as height. `games/low-tide/tests/the_basin_is_lit.rs` checks
the cycle and the lamps.
