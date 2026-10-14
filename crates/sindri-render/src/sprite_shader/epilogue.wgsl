
// ---- end of project shader ----

fn sindri_shader_input(input: SindriVertexOutput) -> SpriteShaderInput {
    let sampled = textureSample(sprite_texture, sprite_sampler, input.uv);
    var shader_input: SpriteShaderInput;
    shader_input.color = sampled * input.tint * input.color_multiply + input.color_offset;
    shader_input.uv = input.uv;
    shader_input.local_uv = input.local_uv;
    shader_input.uv_rect = input.uv_rect;
    shader_input.world_position = input.world_position;
    shader_input.params = input.params;
    shader_input.time = sindri_uniforms.time.x;
    return shader_input;
}

@fragment
fn sindri_fragment(input: SindriVertexOutput) -> @location(0) vec4<f32> {
    let color = sprite_shader(sindri_shader_input(input));
    return vec4<f32>(color.rgb * input.shade, color.a);
}

// Opaque geometry, which writes depth.
//
// The discard is what keeps a cutout honest: a fully transparent pixel that
// wrote depth would hide whatever is behind it while showing nothing itself,
// so the shape a texture cuts out would punch a hole in the world.
@fragment
fn sindri_fragment_solid(input: SindriVertexOutput) -> @location(0) vec4<f32> {
    let color = sprite_shader(sindri_shader_input(input));
    if color.a < 0.5 {
        discard;
    }
    return vec4<f32>(color.rgb * input.shade, 1.0);
}
