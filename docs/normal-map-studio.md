# Normal Map Studio

The 2D update gave sprites and tilemaps a `normal_map` and gave lights a
`height`, so a low light rakes across the art and shows its relief
(`docs/2d-lights.md`). What it did not give anybody is a way to make a normal
map. The platformer's `tiles_n.png` was written by a Python script from height
functions typed out by hand, which is fine for a worked example and is not how
an artist works.

The Normal Map Studio makes one from the sprite that already exists, shows it
lit exactly as the game will light it, and saves it beside the sprite so the
sprite can name it.

This document is the plan the studio was built to, and becomes its contract as
each slice lands. The checklist at the end moves with the implementation.

Which kind of capability this is, in the terms `AGENTS.md` uses: an authoring
tool for a capability the engine already had. Its proof is a game drawing a
normal map the studio made.

## Where it lives

There is no separate pixel editor in this repository; the editor's image
workflow is the Sprite sheet tab, where an image chosen in the project browser
is shown on its own canvas and sliced. The studio is a second tab beside it,
opened from the same image, so it is part of the editor rather than an
application of its own.

Two halves, split where the dependency boundary already is:

- **Generation is a library, in `sindri-assets`** (`sindri_assets::normal_map`).
  It takes decoded RGBA8 pixels and settings and returns RGBA8 pixels. It has
  no GPU, window or editor dependency and runs on every target, so a future
  `sindri asset` command, a test, or an art script can make the same map the
  editor makes, byte for byte. `sindri-assets` already owns decoding images
  and already carries the `image` crate that encodes the result, so nothing
  new enters the dependency graph.
- **The studio is editor UI, in `sindri-editor`**: the panel, the previews,
  saving, and pointing sprites at the result through checked, undoable
  commands.

## Generation

A tangent-space normal map is derived from a height field. The studio offers
two ways to get one:

- **Luminance**: the sprite's own brightness, weighted as perceived
  (Rec. 709). Bright reads as raised. This is a guess, often a good one for
  stone and metal and a poor one for a painted highlight or a dark rivet, which
  is why the result is always shown lit and why it can be inverted.
- **Height map**: an explicit grayscale image laid out exactly like the sprite,
  for an artist who painted the relief they want.

From the height field, each texel's normal is the slope across its neighbours,
scaled by **strength**:

```text
normal = normalize(-dh/dx * strength, dh/drow * strength, 1)
```

which is the convention the renderer reads and the one the platformer's
`art/draw.py` already writes: red is right, green is up the world (rows run
down the picture, so a height rising down the rows faces up), blue is out of
the picture. Encoded as `(n * 0.5 + 0.5) * 255`, so flat is `(128, 128, 255)`.

The settings:

| Setting | Meaning |
| --- | --- |
| `method` | `luminance` or `height_map` |
| `strength` | How steep a given change of height is. 0 is flat |
| `blur` | Radius in texels of a smoothing pass over the height. 0 keeps every texel's own height, which is what pixel art wants |
| `detail` | How much of the unsmoothed relief is laid back over the smoothed one, 0 to 1. With a blur, this is how broad shapes and fine grain are balanced |
| `invert` | Dark reads as raised instead |
| `alpha_height` | Transparent texels count as the lowest height, so a sprite's silhouette bevels toward its outline. Off for opaque tiles, where it changes nothing anyway |
| `edges` | `clamp` (an edge repeats its last texel) or `wrap` (the far side continues, for a texture that tiles) |
| `cells` | Whether a sliced sheet's cells are treated as separate pictures, so a tile's slope never reads its neighbour in the sheet |
| `green` | `up` (Sindri, OpenGL) or `down` (DirectX), for a map that will be used elsewhere. Sindri reads `up`; the preview shows what Sindri would do with either |

**Presets** set these to a starting point for a material: metal, wood,
stone, fabric and terrain. They are only values; nothing recognises a
material, and every value stays editable afterwards.

What is preserved: the source's size exactly, and its alpha, copied into the
map's alpha so the map is transparent where the sprite is. No step resamples,
so every normal belongs to exactly one texel and a pixel-art map stays as
crisp as its sprite. The game samples both images with the same nearest
sampler and the same sprite rectangle (`docs/2d-lights.md`), so the map lines
up texel for texel.

Generation is deterministic: the same pixels and settings make the same bytes
on every host. The tests hold that with fixed inputs.

## Previews

One canvas, four modes, chosen in the strip along its bottom:

- **Texture**: the sprite as it is, unlit.
- **Normal**: the map, or one channel of it as gray.
- **Lit**: the sprite and the map drawn together under a light that follows a
  drag across the canvas.
- **3D**: the same lit plane seen from a camera that orbits, tilts and zooms.

### The lit preview is the game's renderer

The lit preview does not imitate the game's lighting. It draws the sprite with
`sindri_render::SpriteBatchRenderer`, the same pipeline the Scene view, the
Game view and every exported build draw sprites with, through the same shader
(`sprite_shader/epilogue.wgsl` and `light2d/light2d.wgsl`), with the normal map
bound where a sprite's `normal_map` is bound and the light handed over as the
same `Lighting2d` a scene's `sindri.light2d` becomes. The texture and the map
are uploaded as a game uploads them, so the sRGB round trip the shader undoes
is the one it undoes in the game. There is no second lighting model to drift
from the first. A GPU test renders a sprite both ways, through the studio and
through a scene's extraction, and requires the same pixels.

The light's controls are the fields of `sindri.light2d`: color, intensity,
radius, falloff and height, plus the scene's ambient, so a value that looks
right in the studio can be typed into the scene unchanged.

### Comparing

A split divides the canvas: one side drawn without the map and one with it,
with the divider dragged across. "Without" is a choice between the sprite unlit
(what it looked like before any lighting) and the sprite lit by distance alone
(what the 2D update draws for a sprite with no normal map); the second is the
honest comparison, because it isolates what the map adds.

### 3D inspection

The 3D view is the same sprite pipeline with a perspective camera. The surface
stays a flat rectangle in its own plane, as it is in the game; the camera
orbits around it, and the light sits at its height above the plane and is
drawn as a marker with a line down to where it stands. 2D lighting has no
view-dependent term, so what changes with the angle is only the angle, and the
silhouette never moves: the view cannot suggest that a normal map is geometry,
because it draws none. Displacing the plane by the height field would be a
different thing to show, and is left for later.

The 3D renderer in this repository does not read normal maps
(`docs/parity.md`, *Lighting*), so the 3D view inspects the 2D convention
under a 3D camera rather than a 3D material. Showing the map on a 3D material
waits on the 3D renderer gaining normal mapping.

## Saving and using it

Save writes two files beside the source:

- `<name>_n.png`: the map, an ordinary texture. Any sprite or tilemap can
  name it as its `normal_map`; nothing about it is special to the studio.
- `<name>_n.normalmap`: the settings that made it, as JSON, naming the source
  (and height map) relative to itself. Opening either the source or this
  file in the studio restores them, so when the art changes the map is made
  again from the new art with the same settings, not painted again by hand.

`Use on sprites` points every sprite and tilemap in the open scene that draws
from the source texture at the saved map, as one undoable step.

## Checklist

- [x] Plan (this document).
- [x] Generation library with deterministic tests.
- [ ] Studio tab with texture, normal and channel views.
- [ ] Lit preview through the sprite pipeline; split comparison.
- [ ] 3D inspection.
- [ ] Save, the settings sidecar, reopening it, and pointing sprites at it.
- [ ] Presets and polish; validated on Low Tide and platformer textures.
- [ ] A game drawing a studio-made map.
