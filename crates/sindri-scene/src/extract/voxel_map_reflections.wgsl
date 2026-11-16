// A picture's mirror image in a map's water, one piece over one square.
//
// Its parameters: x and y how far along the whole mirror image this piece's
// two ends are, from the bank (0) to the far end (1), the quad's top end first;
// z how strong the reflection is.

fn sprite_shader(input: SpriteShaderInput) -> vec4<f32> {
    let texel = sprite_texel_size();
    // Rows of it slide a pixel to and fro, as the water under it moves.
    let row = floor(input.world_position.y * 16.0);
    let slide = round(sin(row * 0.9 + input.time * 2.2) * 1.2) * texel.x;
    let left = input.uv_rect.x + texel.x * 0.5;
    let right = input.uv_rect.x + input.uv_rect.z - texel.x * 0.5;
    let picture = sample_sprite(vec2<f32>(clamp(input.uv.x + slide, left, right), input.uv.y));
    let along = mix(input.params.x, input.params.y, input.local_uv.y);
    let fade = 0.3 + 0.7 * pow(clamp(1.0 - along, 0.0, 1.0), 1.5);
    // Darker and bluer than what it mirrors, as water's own colour shows
    // through.
    let colour = picture.rgb * vec3<f32>(0.7, 0.8, 0.9);
    return vec4<f32>(colour, picture.a * fade * input.params.z);
}
