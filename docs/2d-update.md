# 2D update: closing the 2D rendering gaps

One pull request, delivered in checked feature slices, the way
[the 3D update](3d-update.md) is. Each slice is committed and pushed before the
next starts. Runtime, editor, scripting, proof and documentation move together.

This is the 2D counterpart to the 3D update and is kept apart from it on
purpose. The 3D update's `Material` covers meshes, cubes, voxels and models; it
does not reach sprites. Its point and spot lights extend `sindri.light`, which
lights 3D geometry. Nothing below edits either, so the two pull requests can
land in either order.

## Why

Read against main (October 2026, at `d9e5bc9`), the core 2D path is at parity:
sprites, sheets, animation, tilemaps with painting and collision, procedural
shapes, project-font text, camera behavior and the post stack. `docs/parity.md`
still lists these 2D rendering rows as absent or behind:

1. **Custom shaders / materials.** No sprite can look like anything other than
   its texture times a color. Parity calls it "the single biggest ceiling on
   visual identity".
2. **2D lights and shadows.** Nothing. A 2D scene cannot be dark with a lantern
   in it.
3. **Particle system.** One effect shape: the pooled fleck burst. No emitters,
   emission shapes, curves, color ramps or editor preview.
4. **Nine-slice sprites.** Every resizable panel needs it.
5. **Autotiling.** Painting a wall run by hand is the daily cost of a tilemap.
6. **Sprite masking.** Nothing.
7. **World-space and rich text.** No damage numbers, no name plates, no color
   inside a string.

The advanced sprite color transform is also built everywhere and proven by no
game in this repository.

## Slices

### 1. Sprite shaders
- [x] A sprite shader is a project `.wgsl` asset that defines one function,
  `fn sprite_shader(input: SpriteShaderInput) -> vec4<f32>`. The engine wraps
  it in the sprite pipeline's own vertex stage and prelude: the sampled color,
  UV, the sprite's UV rect, world position, the instance's four shader
  parameters, the frame's time, and the texture and sampler for further
  samples.
- [x] `sindri.sprite` gains `shader` (an asset path, empty for none) and
  `shader_params` (four numbers). Params ride on the instance, so sprites
  sharing a texture and a shader still share a draw.
- [x] A shader that fails to compile is reported, naming the file and the
  line, and the sprite falls back to the built-in shader rather than the frame
  failing. The same holds in the browser.
- [x] Shaders load through the asset queue on every host, hot reload natively,
  and export with the project.
- [x] The inspector picks a shader from the project's `.wgsl` assets and edits
  the four params; Decay reads and writes `sprite.shader_params`.
- [x] Proof: a feature example, `examples/sprite-shaders`, with an outline, a
  dissolve, a hit flash and a wave; scripts drive the dissolve and the flash,
  the scene's clock the outline and the wave. Contract:
  `docs/sprite-shaders.md`.

### 2. 2D lights
- [x] `sindri.light2d`: a point light in the plane, with color, intensity,
  radius and falloff. A separate component from `sindri.light`, because a 2D
  light lights pictures by distance in the plane and has no direction.
- [x] `sindri.environment` gains `lighting_2d`: whether 2D lighting is on, and
  the ambient color 2D content is multiplied by where no light reaches.
- [x] When it is on, world sprites, tilemaps, shapes and flecks are lit. A
  sprite can opt out with `unlit`. The UI is never lit.
- [x] The Scene view draws each light's radius. Create offers Light 2D.
- [x] Decay reads and writes a light's color, intensity, radius, falloff and
  height; its entity's `set_active` switches it off.
- [x] Normal maps: a sprite may name a normal map beside its texture, and a
  light has a height above the plane, so light rakes across the art. A sprite
  with no normal map is lit exactly as without one. Sprites with different
  normal maps are different batches. Tilemaps name one too.

### 3. 2D shadows
- [x] `sindri.light2d.occluder`: a box or polygon that blocks 2D light. A light
  with `shadows` on is blocked by every occluder in its radius.
- [x] Shadows are a one-dimensional polar depth map per light, built on the CPU
  from occluder edges and sampled in the sprite shader, so they are
  deterministic and testable without a GPU.
- [x] A tilemap can cast shadows from its solid cells, through the tilemap
  collider's merged boxes.
- [x] The Scene view outlines occluders; a box occluder's size and a light's
  radius drag by handle. Polygon points are edited in the inspector.
- [x] Proof: the platformer gets a dark cave section lit by lanterns, with the
  level casting shadows. A sun keeps daylight out under the roof, and a low
  wall torch rakes normal-mapped bricks. Contract: `docs/2d-lights.md`.

### 4. Particle system
- [ ] `sindri.particles`: a continuous emitter with rate, bursts, lifetime,
  speed, direction and spread, an emission shape (point, circle, box, ring),
  gravity, drag, size over life, a color ramp, rotation and spin, local or
  world simulation, and a cap.
- [ ] Particles live in the existing effect pool, so they cost what flecks
  cost, and use their own random stream so a seeded run stays the same run.
- [ ] Decay plays, stops and bursts an emitter.
- [ ] The editor previews the selected emitter in the Scene view without
  entering Play.
- [ ] Proof: the platformer's coins sparkle and the hero kicks up dust.

### 5. Nine-slice sprites
- [ ] A sheet sprite can declare a border: left, top, right and bottom, in
  pixels.
- [ ] `sindri.ui.image` and `sindri.sprite` gain `draw: simple | sliced` and a
  size. A sliced image keeps its corners and stretches its edges and middle.
- [ ] The sheet slicer edits the border by dragging its guides.
- [ ] Proof: a resizable panel in the UI example.

### 6. Autotiling
- [ ] A sheet can declare terrains: a name and the sprite each of the sixteen
  edge masks or the forty-seven blob masks paints.
- [ ] The tilemap brush paints a terrain, choosing each cell and its neighbors'
  sprites from the mask. Erasing updates the neighbors too.
- [ ] Decay's `Grid` gains `set_terrain`.
- [ ] Proof: the platformer's level is painted with a terrain.

### 7. Sprite masking
- [ ] `sindri.sprite_mask`: a sprite whose shape is a mask rather than a
  picture. Sprites with `mask: inside` draw only within it, and `mask: outside`
  only outside it.
- [ ] Proof: a feature example of a keyhole reveal.

### 8. World-space and rich text
- [ ] `sindri.text`: text placed in the world like a sprite, sorted with
  sprites.
- [ ] Rich text spans in `sindri.ui.text` and `sindri.text`: color, bold and
  italic.
- [ ] Proof: damage numbers in Scorchball or Orbital.

### 9. Final integration
- [ ] Orbital's charger uses the advanced sprite color transform, so that row
  has its proof.
- [ ] Update `parity.md`, `capabilities.md`, `ROADMAP.md`, the subsystem
  contracts, the CHANGELOG, and the regenerated `docs/generated`.

## Out of scope

- **Ordering one walker against several occluders.** It only arises on
  stackable tile volumes, and no game uses one any more. It stays a parity row.
- **A shader graph.** Deferred beyond the first major release by the roadmap.
