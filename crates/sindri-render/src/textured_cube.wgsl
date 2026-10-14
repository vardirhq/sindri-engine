struct Uniforms {
    model_view_projection: mat4x4<f32>,
    model: mat4x4<f32>,
    ambient: vec4<f32>,
    directional_direction: vec4<f32>,
    directional_color: vec4<f32>,
    light_view_projection: mat4x4<f32>,
    shadow: vec4<f32>,
    fog_color: vec4<f32>,
    fog_params: vec4<f32>,
    camera_position: vec4<f32>,
    // uv offset (animation frame), glow, alpha cutoff.
    surface: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(0) @binding(1)
var cube_texture: texture_2d<f32>;

@group(0) @binding(2)
var cube_sampler: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(14) ambient_occlusion: f32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) world_position: vec3<f32>,
    @location(2) ambient_occlusion: f32,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = uniforms.model_view_projection * vec4<f32>(input.position, 1.0);
    output.world_position = (uniforms.model * vec4<f32>(input.position, 1.0)).xyz;
    output.uv = input.uv + uniforms.surface.xy;
    output.ambient_occlusion = input.ambient_occlusion;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let sampled = textureSample(cube_texture, cube_sampler, input.uv);
    // A cutout surface has holes where its art does, rather than a square.
    if sampled.a < uniforms.surface.w {
        discard;
    }
    let dx = dpdx(input.world_position);
    let dy = dpdy(input.world_position);
    // A framebuffer's rows run downward, so screen-right crossed with
    // screen-down points into the surface. Crossed the other way it points
    // out of it, toward the viewer, which is the side the light has to reach.
    let normal = normalize(cross(dy, dx));
    let light_direction = normalize(uniforms.directional_direction.xyz);
    let diffuse = max(dot(normal, -light_direction), 0.0);
    let ambient = uniforms.ambient.rgb * uniforms.ambient.a;
    let visibility = directional_visibility(input.world_position,
        uniforms.light_view_projection, uniforms.shadow, diffuse);
    let directional = uniforms.directional_color.rgb
        * uniforms.directional_direction.a * diffuse * visibility;
    let contact_visibility = mix(1.0, input.ambient_occlusion, uniforms.shadow.z);
    // A glowing surface lights itself: shadow and the sun's angle take none of
    // it away, and what exceeds one is left for the bloom pass to find.
    let lit = sampled.rgb * (ambient + directional) * contact_visibility
        + sampled.rgb * uniforms.surface.z;
    return vec4<f32>(atmosphere(lit, input.world_position, uniforms.camera_position,
        uniforms.fog_color, uniforms.fog_params), sampled.a);
}
