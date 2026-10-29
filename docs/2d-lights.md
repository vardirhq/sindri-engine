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
| `shadow_source_radius` | Emitting-area radius in world units; zero is a point light | `0` |
| `cone_angle` | How wide the beam is, in degrees across; 360 lights every way | `360` |
| `cone_direction` | Where the beam points, in degrees: 0 is right and 90 is up | `0` |
| `cone_softness` | How much of the cone's width fades out at its edge: 0 is hard, 1 is brightest only along the middle | `0.3` |

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
casting light sees exactly what no occluder edge within its radius blocks. That
region is described, round the light, as a sorted list of angular intervals: each
runs from the direction of one edge's end to the next and holds the line of the
one edge nearest the light across it (a unit normal and the line's distance from
the light, so the distance along a ray is `d / (n . u)`). A point is in shadow when
it lies past that edge by more than the light's `shadow_reach`. A shadow therefore
ends at the true angle of an edge's end, at any distance, and two walls meeting
cast a true corner; nothing is quantized to a table of directions.

The intervals come from an angular sweep on the CPU, one per shadowed light, and
the shader finds a pixel's interval by binary search and applies the same
formula, so the arithmetic is the same on every host and testable without a GPU:
`Lighting2d::at` answers what a point receives, and `sindri_render::sees` answers
whether a source sees a point at all. One wall is three intervals; a source holds
up to 256, enough for a level's few dozen merged boxes, and past that the narrowest
slivers fold into their neighbors, so a crowd of tiny edges loses fine detail of
shadow and never a whole edge.

A `cone_angle` below 360 makes the light a spot light: it shines only inside a
beam of that width pointed `cone_direction` degrees round from right, fading over
the outer `cone_softness` of its width, and reaches `radius` along the beam. The
beam is a multiplier on the light, so it composes with falloff, shadows, normal
maps and a script that turns it: write `cone_direction` each frame to sweep it.
`Lighting.level` and the cast shadows (`sindri.shadow2d`) honour the beam too, so a
thing outside it is dark to a script and casts nothing from that light. A beam
does not shorten the shadow map: occluders are still found all the way round.

A positive `shadow_source_radius` uses five sources from points across the light's
emitting area, averaging their visibility to approximate an area-light penumbra.
The CPU query and GPU renderer use the same samples. It is a five-sample
approximation, so the penumbra is five steps, not a smooth ramp; it is not a
continuous area-light solution. It costs five sweeps per shadowed light.
Lamplighter uses it on Wick’s lantern and its roadside lamps. A point light's
shadow edge is hard: that is the look, and the area samples are the softening.

Limits: the intervals are rebuilt every frame from every occluder, which is cheap
for a handful of lights and a level's few dozen merged boxes and is not meant
for hundreds of shadow casting lights. Occluders block light only; they do not
darken themselves beyond `shadow_reach`, which is why a sun's reach is set to the
depth of the ground it lights.

## What a viewer sees

A light decides what is lit; `sindri.sight2d` decides what is seen. Put it on the
entity whose eyes the player has, and everything it cannot see in a straight line,
past the edges of occluders, is multiplied by `unseen` and so goes dim. It is dimmed
and not hidden: the light itself does not change, and a lamp across the field still
shows where it is. It is how a game says "you can see here" and "you cannot see
behind that tree", in the way of Monaco, and it takes the same occluders as the
shadows do.

| Field | Meaning | Default |
| --- | --- | --- |
| `enabled` | Whether it is the viewer | `true` |
| `radius` | How far it sees, fading over the last quarter; zero sees as far as there is | `0` |
| `unseen` | What lit content out of sight is multiplied by, 0 to 1 (0 is black) | `0.3` |
| `reach` | How far past an occluding edge it still sees, so the face of a wall it looks at is seen | `0.3` |

Only the first enabled viewer in the scene counts, and nothing is dimmed unless the
environment's `lighting_2d` is on. Occluders are found out to `radius`, or all the
way when it is zero. A thing with `sindri.shadow2d` is seen from its feet, like a
shadow on it, so a tree is not hidden behind its own trunk. In a scene that is
already dark the dimming is slight: it shows most where the ambient is bright.

## Asking the light from a script

A game's rules cannot depend on the camera, so Decay asks a separate question of
the same lights and occluders, all of them wherever the camera is:

- `Lighting.visible(from, to)` is whether the straight line between two world
  points crosses no occluder edge. Touching an edge's end, or standing on it, does
  not block. This is line of sight.
- `Lighting.level(point)` is how bright it is at a point: the luminance of the
  ambient plus every light that reaches it, 0 in the dark and 1 for a white light at
  full strength, more where lights overlap. A light with `shadows` on is blocked by
  occluders; one with it off shines through them.

Both treat a light as its centre, so a soft penumbra is a hard edge to a script.
They fail in a scene with `lighting_2d` off rather than answering that everything
is dark.

## Shadows on the ground

An occluder decides what a light cannot see through. It is not what a viewer
looks at: that is the dark patch on the ground saying a tree stands there and
which way the light is. `sindri.shadow2d` adds it to anything with a picture, in
two parts, either of which it may have:

| Field | Meaning | Default |
| --- | --- | --- |
| `contact_size` | Width and depth of a soft dark ellipse under the thing, in world units; zero in either leaves it out | `[1.0, 0.4]` |
| `contact_offset` | Where its middle is, from the entity's position | `[0, 0]` |
| `contact_opacity` | How dark it is at its middle | `0.35` |
| `cast` | Whether the thing's own picture is also cast away from the lights | `false` |
| `cast_opacity` | How dark a cast shadow is where a light reaches its full strength | `0.45` |
| `height` | How tall the thing is, in world units, which sets how long it casts; zero takes the height its picture is drawn at | `0` |
| `max_length` | The longest a cast shadow gets | `2.5` |
| `lit_from_base` | Whether the thing itself is lit as something standing: whether a shadow covers it is asked at its feet, not at each pixel | `true` |

The **contact shadow** is a feathered black ellipse drawn one layer below the
thing's sprite, so it says the thing rests on the ground whatever lights are
about. The **cast shadow** is the sprite's own picture, in black, laid on the
ground: its feet stay where they are and its head is carried away from the light,
so it is a card lying down and pointing away from it. It is `height * distance /
light height` long (so a taller thing, and a light further from it or lower, cast
longer) up to `max_length`, as wide as the picture, and as dark as the light that
reaches its feet. Of the lights in reach the two strongest each cast one, so two
lamps give a thing two shadows. A light that the thing carries, such as Wick's
lantern, casts nothing from it. A cast shadow is always an area, whichever side
the light is on, because it lies across the ground rather than standing in the
picture's plane.

Both are drawn over the lit ground and are not lit themselves, so they darken what
light reaches and change nothing in the dark, and neither blocks light: that is the
occluder's work, and a thing usually wants both, the occluder as a small ellipse on
its footprint (a box centered on its foot shadows a rectangle under it whatever the
thing is). Casting needs the environment's `lighting_2d` on.

Limits: the cast shadow is the sprite's picture and not its real shape from above,
so a tree casts the silhouette it shows, not its crown's outline; it falls on a
flat ground at the object's own height, so it does not climb a wall or a step; and
the ground in front of a thing is lit normally, and the thing itself is lit from its feet (`lit_from_base`), so the occluder round its base, which stops light reaching what is behind it, does not also darken the side facing the light. A lit-from-base thing is lit by every light in range that nothing else blocks, from whichever side, and the shadow it casts is what says which way the light is.

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

Lamplighter uses a carried lantern in a dark level, with lit roadside lanterns
and two hanging cave lanterns. The level tiles are normal-mapped, and the cave
pillar casts a shadow from the lanterns. `games/platformer/tests/cave_lights.rs` checks the shadows,
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
