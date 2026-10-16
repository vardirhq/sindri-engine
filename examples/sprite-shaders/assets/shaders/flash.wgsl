// Whitens the sprite, the usual way to show a hit.
//
// params.x is how white, 0 to 1; a script sets it on a hit and lets it fade.
fn sprite_shader(input: SpriteShaderInput) -> vec4<f32> {
    let white = vec3<f32>(1.0, 1.0, 1.0);
    return vec4<f32>(mix(input.color.rgb, white, input.params.x), input.color.a);
}
