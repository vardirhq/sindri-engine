// Burns the sprite away through blocky noise.
//
// params.x is how far the burn has got, 0 untouched to 1 gone. Pixels just
// ahead of the burn glow orange.
fn dissolve_noise(cell: vec2<f32>) -> f32 {
    return fract(sin(dot(cell, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

fn sprite_shader(input: SpriteShaderInput) -> vec4<f32> {
    let noise = dissolve_noise(floor(input.local_uv * 16.0));
    let burn = input.params.x * 1.1;
    if noise < burn - 0.1 {
        return vec4<f32>(0.0);
    }
    if noise < burn {
        return vec4<f32>(1.0, 0.45, 0.1, input.color.a);
    }
    return input.color;
}
