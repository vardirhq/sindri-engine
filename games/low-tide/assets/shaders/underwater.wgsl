// A wreck seen through brine: the water bends it, sunlight ripples over it,
// and the deeper it lies the more the water's own colour covers it.
//
// params.x is how far the water bends it, as a share of its size; params.y how
// bright the rippling light is; params.z how much of the water's colour covers
// it, 0 to 1; params.w its opacity. The sprite's own tint is not used: the
// water decides its colour.
fn sprite_shader(input: SpriteShaderInput) -> vec4<f32> {
    let p = input.world_position.xy;
    let t = input.time;
    let bend = vec2<f32>(
        sin(p.y * 1.7 + t * 1.3) + 0.5 * sin(p.x * 2.9 - t * 0.8),
        cos(p.x * 1.5 + t * 1.1) + 0.5 * cos(p.y * 3.3 + t * 0.7),
    ) * input.params.x;
    let across = input.local_uv + bend;
    if across.x < 0.0 || across.x > 1.0 || across.y < 0.0 || across.y > 1.0 {
        return vec4<f32>(0.0);
    }
    let picture = sample_sprite(input.uv_rect.xy + across * input.uv_rect.zw);
    let water = vec3<f32>(0.08, 0.33, 0.36);
    var rgb = mix(picture.rgb, water, input.params.z);
    // Two crossing ripples, sharpened, are the light caught by the surface.
    let a = sin(p.x * 2.3 + p.y * 1.1 + t * 1.6);
    let b = sin(p.y * 2.7 - p.x * 0.9 - t * 1.2);
    let caustic = pow(max(0.0, 1.0 - abs(a + b) * 0.5), 6.0);
    rgb = rgb + vec3<f32>(0.55, 0.85, 0.8) * caustic * input.params.y;
    return vec4<f32>(rgb, picture.a * input.params.w);
}
