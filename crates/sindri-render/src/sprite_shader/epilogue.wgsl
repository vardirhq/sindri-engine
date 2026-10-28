
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

// The batch's normal map, or a flat one. Bound like a color texture, so it
// arrives decoded from sRGB and is encoded back before use.
@group(0) @binding(5)
var sindri_normal_map: texture_2d<f32>;

fn sindri_srgb_encode(linear: vec3<f32>) -> vec3<f32> {
    let low = linear * 12.92;
    let high = 1.055 * pow(max(linear, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.4)) - 0.055;
    return select(high, low, linear <= vec3<f32>(0.0031308));
}

// The sprite's color as the frame's 2D lights leave it.
fn sindri_lit(input: SindriVertexOutput, rgb: vec3<f32>) -> vec3<f32> {
    if input.flags.x < 0.5 || !sindri_lighting_2d_on() {
        return rgb;
    }
    let use_normal = sindri_uniforms.time.y > 0.5;
    var normal = vec3<f32>(0.0, 0.0, 1.0);
    if use_normal {
        let encoded = sindri_srgb_encode(
            textureSampleLevel(sindri_normal_map, sprite_sampler, input.uv, 0.0).rgb,
        );
        let local = encoded * 2.0 - vec3<f32>(1.0);
        let right = normalize(input.axes.xy);
        let up = normalize(input.axes.zw);
        normal = normalize(vec3<f32>(right * local.x + up * local.y, local.z));
    }
    var base = input.world_position.xy;
    if input.flags.y > 0.5 {
        // The quad's own coordinates run 0..1 down the picture, and its axes are
        // the model's two columns, so the feet are the bottom middle.
        base = base - (input.local_uv.x - 0.5) * input.axes.xy - (1.0 - input.local_uv.y) * input.axes.zw;
    }
    return rgb * sindri_light_2d_from(input.world_position.xy, base, normal, use_normal);
}

// The batch's mask, or anything when it has none.
@group(0) @binding(6)
var sindri_mask_texture: texture_2d<f32>;

// How much of this pixel the batch's mask lets through: the mask's alpha
// inside its quad, nothing outside it, and the other way round for outside.
fn sindri_masked(world: vec3<f32>) -> f32 {
    let mode = sindri_uniforms.time.z;
    if mode < 0.5 {
        return 1.0;
    }
    let local = (sindri_uniforms.mask_from_world * vec4<f32>(world, 1.0)).xy;
    var coverage = 0.0;
    if all(abs(local) <= vec2<f32>(0.5)) {
        // Texture rows run down, the quad's up.
        let across = vec2<f32>(local.x + 0.5, 0.5 - local.y);
        let uv = sindri_uniforms.mask_uv.xy + across * sindri_uniforms.mask_uv.zw;
        coverage = textureSampleLevel(sindri_mask_texture, sprite_sampler, uv, 0.0).a;
    }
    if mode > 1.5 {
        return 1.0 - coverage;
    }
    return coverage;
}

@fragment
fn sindri_fragment(input: SindriVertexOutput) -> @location(0) vec4<f32> {
    let color = sprite_shader(sindri_shader_input(input));
    let alpha = color.a * sindri_masked(input.world_position);
    return vec4<f32>(sindri_lit(input, color.rgb) * input.shade, alpha);
}

// Opaque geometry, which writes depth.
//
// The discard is what keeps a cutout honest: a fully transparent pixel that
// wrote depth would hide whatever is behind it while showing nothing itself,
// so the shape a texture cuts out would punch a hole in the world.
@fragment
fn sindri_fragment_solid(input: SindriVertexOutput) -> @location(0) vec4<f32> {
    let color = sprite_shader(sindri_shader_input(input));
    if color.a * sindri_masked(input.world_position) < 0.5 {
        discard;
    }
    return vec4<f32>(sindri_lit(input, color.rgb) * input.shade, 1.0);
}
