// A glowing rim around whatever the texture leaves opaque.
//
// params.x is the rim's width in texels; the rim pulses with time. Pixels the
// sprite already covers draw as they are, so only the transparent margin
// around the picture changes.
fn sprite_shader(input: SpriteShaderInput) -> vec4<f32> {
    if input.color.a > 0.5 {
        return input.color;
    }
    let reach = sprite_texel_size() * max(input.params.x, 1.0);
    var near = 0.0;
    near = max(near, sample_sprite(input.uv + vec2<f32>(reach.x, 0.0)).a);
    near = max(near, sample_sprite(input.uv - vec2<f32>(reach.x, 0.0)).a);
    near = max(near, sample_sprite(input.uv + vec2<f32>(0.0, reach.y)).a);
    near = max(near, sample_sprite(input.uv - vec2<f32>(0.0, reach.y)).a);
    let pulse = 0.65 + 0.35 * sin(input.time * 4.0);
    return vec4<f32>(1.0, 0.82, 0.25, near * pulse);
}
