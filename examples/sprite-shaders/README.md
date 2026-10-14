# Sprite Shaders

A feature example for project sprite shaders. Open `sindri.toml` in the editor
or run the exported project in a WebGPU browser. Pages publishes it at
`https://sindri.vardir.no/examples/sprite-shaders/`.

Five copies of one badge draw left to right with no shader and then through
four shaders in `assets/shaders/`:

- `outline.wgsl` rims the opaque shape with a pulsing glow. Its width in
  texels is the first shader parameter, authored in the scene.
- `dissolve.wgsl` burns the sprite away through blocky noise, with a glowing
  edge. The `Burn` script sweeps the first parameter from 0 to 1 and back.
- `flash.wgsl` whitens the sprite. The `HitFlash` script sets the first
  parameter to 1 on a hit, every 1.2 seconds or when Space is pressed, and
  fades it out.
- `wave.wgsl` sways the picture row by row. Amplitude and wave count are the
  first two parameters; the motion comes from the frame's time.

Each shader is one function, `fn sprite_shader(input: SpriteShaderInput) ->
vec4<f32>`, which the engine wraps in the sprite pipeline. Edit one while the
editor has the scene open and saving reloads it; a mistake is reported in the
console with the file and line, and the sprite draws with the built-in look
until it is fixed.

`assets/scripts/shader-demo.decay` holds both scripts. Shader parameters are
written as decimals in the scene (`0.0`, not `0`): a script writes a value back
in the number kind the scene stored, so an integer would round every write.

`game/tests/the_sprite_shader_demo_works.rs` compiles every shader the scene
names and plays the scripts. See [the sprite shader contract](../../docs/sprite-shaders.md).
