// Sways the picture sideways, row by row, like a flag or heat haze.
//
// params.x is how far each row moves, as a share of the sprite's width, and
// params.y how many waves fit down it. Samples stay inside the sprite's own
// part of the texture, so this works on one frame of a sheet too.
fn sprite_shader(input: SpriteShaderInput) -> vec4<f32> {
    let shift = sin(input.local_uv.y * input.params.y * 6.2831853 + input.time * 3.0)
        * input.params.x;
    let across = input.local_uv.x + shift;
    if across < 0.0 || across > 1.0 {
        return vec4<f32>(0.0);
    }
    let uv = input.uv_rect.xy + vec2<f32>(across, input.local_uv.y) * input.uv_rect.zw;
    // `sample_sprite` reads the texture untinted, which is fine for a sprite
    // drawn in its own colors.
    return sample_sprite(uv);
}
