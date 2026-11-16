// The engine's half of every sprite shader, placed before the project's.
//
// Everything here is named `sindri_` or is part of the documented surface:
// `SpriteShaderInput`, `sample_sprite` and `sprite_texel_size`. A project's
// shader defines `sprite_shader` and may define anything else it likes.

struct SindriUniforms {
    view_projection: mat4x4<f32>,
    // x is the frame's time in seconds; y is 1 when the batch has a normal
    // map; z is how it is masked: 0 not, 1 inside, 2 outside.
    time: vec4<f32>,
    // From the world into the mask quad's own -0.5..0.5 space.
    mask_from_world: mat4x4<f32>,
    // The part of the mask texture its quad shows.
    mask_uv: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> sindri_uniforms: SindriUniforms;

@group(0) @binding(1)
var sprite_texture: texture_2d<f32>;

@group(0) @binding(2)
var sprite_sampler: sampler;

struct SindriVertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) model_0: vec4<f32>,
    @location(3) model_1: vec4<f32>,
    @location(4) model_2: vec4<f32>,
    @location(5) model_3: vec4<f32>,
    @location(6) tint: vec4<f32>,
    // x, y, width, height in normalized texture space.
    @location(7) uv_rect: vec4<f32>,
    @location(8) color_multiply: vec4<f32>,
    @location(9) color_offset: vec4<f32>,
    // Brightness at each corner, from the quad's own [0,0] round to [0,1].
    @location(10) corner_shade: vec4<f32>,
    @location(11) shader_params: vec4<f32>,
    // x is 1 when 2D lights light this sprite, y when it is lit from where it
    // stands, z how much it sways in the wind; w is reserved.
    @location(12) flags: vec4<f32>,
}

struct SindriVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) tint: vec4<f32>,
    @location(2) color_multiply: vec4<f32>,
    @location(3) color_offset: vec4<f32>,
    @location(4) shade: f32,
    @location(5) world_position: vec3<f32>,
    @location(6) local_uv: vec2<f32>,
    @location(7) @interpolate(flat) uv_rect: vec4<f32>,
    @location(8) @interpolate(flat) params: vec4<f32>,
    @location(9) @interpolate(flat) flags: vec4<f32>,
    // The sprite's own right and up in the world, for turning a normal map.
    @location(10) @interpolate(flat) axes: vec4<f32>,
}

// How far wind moves a point of a swaying sprite: nothing at its foot, most at
// its top, along its own width. Each sprite has its own phase from where its
// foot is, and gusts roll slowly across the world over that, so a wood sways
// in waves rather than in step.
fn sindri_sway(model: mat4x4<f32>, up: f32, amount: f32) -> vec3<f32> {
    let foot = (model * vec4<f32>(0.0, -0.5, 0.0, 1.0)).xy;
    let height = clamp(up + 0.5, 0.0, 1.0);
    let t = sindri_uniforms.time.x;
    let gust = 0.55 + 0.45 * sin(t * 0.45 + foot.x * 0.11 - foot.y * 0.07);
    let swing = sin(t * 1.9 + foot.x * 0.9 + foot.y * 0.6) * 0.7
        + sin(t * 3.1 + foot.x * 1.7) * 0.3;
    // Its width, the same way round whether or not it is mirrored.
    let across = model[0].xyz * sign(model[0].x + 0.0001);
    return across * amount * height * height * swing * gust;
}

@vertex
fn sindri_vertex(input: SindriVertexInput) -> SindriVertexOutput {
    let model = mat4x4<f32>(input.model_0, input.model_1, input.model_2, input.model_3);
    var world = model * vec4<f32>(input.position, 1.0);
    if input.flags.z != 0.0 {
        world = vec4<f32>(world.xyz + sindri_sway(model, input.position.y, input.flags.z), world.w);
    }
    var output: SindriVertexOutput;
    output.position = sindri_uniforms.view_projection * world;
    // The quad's own coordinates run 0..1; the rect says which part of
    // the texture that maps onto, so the whole texture is 0, 0, 1, 1.
    output.uv = input.uv_rect.xy + input.uv * input.uv_rect.zw;
    output.tint = input.tint;
    output.color_multiply = input.color_multiply;
    output.color_offset = input.color_offset;
    // The quad's own coordinates run -0.5..0.5, so this is where the vertex
    // sits across it. Bilinear between the four corner values, interpolated
    // over the face for free by the rasteriser.
    let across = input.position.xy + vec2<f32>(0.5, 0.5);
    let lower = mix(input.corner_shade.x, input.corner_shade.y, across.x);
    let upper = mix(input.corner_shade.w, input.corner_shade.z, across.x);
    output.shade = mix(lower, upper, across.y);
    output.world_position = world.xyz;
    output.local_uv = input.uv;
    output.uv_rect = input.uv_rect;
    output.params = input.shader_params;
    output.flags = input.flags;
    output.axes = vec4<f32>(input.model_0.xy, input.model_1.xy);
    return output;
}

/// What a sprite shader is handed for one pixel.
struct SpriteShaderInput {
    /// The texture sample with the sprite's tint and color transform applied:
    /// what the pixel would be without a shader.
    color: vec4<f32>,
    /// Where this pixel samples the texture, in normalized texture space.
    uv: vec2<f32>,
    /// Where this pixel sits across the sprite, 0..1 left to right and top to
    /// bottom, whichever part of a sheet the sprite draws.
    local_uv: vec2<f32>,
    /// The part of the texture this sprite draws: x, y, width, height.
    uv_rect: vec4<f32>,
    /// Where this pixel is in the world.
    world_position: vec3<f32>,
    /// The sprite's four authored shader parameters.
    params: vec4<f32>,
    /// Seconds since the scene started playing.
    time: f32,
}

/// The sprite's texture at `uv`, without tint. Safe anywhere, including
/// inside a branch, because it samples the full-resolution level explicitly.
fn sample_sprite(uv: vec2<f32>) -> vec4<f32> {
    return textureSampleLevel(sprite_texture, sprite_sampler, uv, 0.0);
}

/// One texel's size in normalized texture space.
fn sprite_texel_size() -> vec2<f32> {
    let size = textureDimensions(sprite_texture, 0);
    return vec2<f32>(1.0, 1.0) / vec2<f32>(f32(size.x), f32(size.y));
}

// ---- project shader ----
