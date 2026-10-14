# Sprite shaders

A project can draw any sprite through its own fragment shader: a `.wgsl` file
under the project's assets that defines one function. This is the contract for
what that function is handed, how a file becomes a shader on every host, and
what happens when it is wrong.

## What a project writes

```wgsl
fn sprite_shader(input: SpriteShaderInput) -> vec4<f32> {
    return vec4<f32>(mix(input.color.rgb, vec3<f32>(1.0), input.params.x), input.color.a);
}
```

That is the whole file. The engine places it between its own prelude and
epilogue (`crates/sindri-render/src/sprite_shader/`) and builds the sprite
pipeline around it, so the vertex stage, the bindings, instancing, depth and
blending are the engine's, exactly as for a sprite with no shader. A file may
define helpers of its own; names starting `sindri_` are the engine's.

`SpriteShaderInput` carries:

| Field | Meaning |
| --- | --- |
| `color` | The texture sample with tint and colour transform applied: the pixel as it would draw with no shader |
| `uv` | Where this pixel samples the texture, in normalized texture space |
| `local_uv` | Where it sits across the sprite, 0..1, whichever part of a sheet the sprite draws |
| `uv_rect` | The part of the texture the sprite draws: x, y, width, height |
| `world_position` | The pixel's world position |
| `params` | The sprite's four `shader_params` |
| `time` | Seconds the scene has played |

Two helpers reach the texture again: `sample_sprite(uv)`, which samples the
full-resolution level and so is safe inside a branch, and
`sprite_texel_size()`. A returned color's alpha blends as any sprite's does,
and the corner shading a voxel or tile face carries is applied afterwards.

## Authoring

`sindri.sprite` names the file in `shader` (an asset path, empty for the
built-in look) and carries `shader_params`, four numbers whose meaning is the
shader's. The inspector picks a shader from the project's `.wgsl` files and
edits the four numbers; Decay reads and writes
`this.sprite.shader_params.{x,y,z,w}` (see
[scripting](scripting.md#the-entity-a-script-is-on)). The parameters ride on each instance,
so sprites sharing a texture and a shader share a draw however differently
they are parameterised; a different shader is a different batch.

## Hosts

Every host checks a shader as it binds it, by parsing and validating the
composed module with naga, before any pipeline exists.

- The editor loads the shaders a scene and its prefabs name through the asset
  queue, watches them, and reloads on save.
- `sindri-player` reads them from the project directory natively and from the
  export manifest in the browser.
- `sindri-export` ships each shader a scene names and refuses to export one
  that does not compile, so a broken shader is found before a browser sees it.
- `project-capture` and `project-benchmark` bind them as the player does.

The renderer builds a shader's pipelines once per load and keys them by path;
a reload replaces them.

## Failure

A source that does not compile is reported with the file and the line in the
project's own numbering (`shaders/flash.wgsl:3: ...`). A file with no
`fn sprite_shader` says so. The sprite then draws with the built-in look rather
than the frame failing, and a reload that breaks a previously good shader drops
the old version too, so what is on screen never hides the break. A shader
parameter that is not finite is refused at extraction, as a colour transform's
is.

## Limits

One texture per sprite; a shader cannot bind more. There is no material asset:
the four parameters are the only per-sprite input. Shaders apply to
`sindri.sprite` only, not to tiles, shapes, text or UI images. Validation
places a type error on the line of the function it is in rather than on the
statement.

## Proof

`examples/sprite-shaders` draws an outline, a dissolve, a hit flash and a wave,
with Decay scripts driving the dissolve and the flash.
`game/tests/the_sprite_shader_demo_works.rs` compiles every shader it names and
plays the scripts; `crates/sindri-gpu/tests/sprite_shaders.rs` reads drawn
pixels back to show the shader, its per-instance parameters and the clock reach
the screen. Low Tide draws its submerged wrecks through
`shaders/underwater.wgsl`, bent and rippled with light by the brine above them.
