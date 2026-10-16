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
    base_color: vec4<f32>,
    material: vec4<f32>,
    emissive: vec4<f32>,
    local_lights: array<LocalLight, 8>,
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
    @location(2) normal: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) world_position: vec3<f32>,
    @location(2) ambient_occlusion: f32,
    @location(3) normal: vec3<f32>,
}

// Cofactors divided by the determinant are the inverse transpose. This uses
// the existing model uniform and preserves nonuniform/mirrored transforms.
fn transform_normal(normal: vec3<f32>) -> vec3<f32> {
    let a = uniforms.model[0].xyz;
    let b = uniforms.model[1].xyz;
    let c = uniforms.model[2].xyz;
    let cofactors = mat3x3<f32>(cross(b, c), cross(c, a), cross(a, b));
    let determinant = dot(a, cofactors[0]);
    if abs(determinant) < 0.00000000000000000001 { return vec3<f32>(0.0); }
    return cofactors * normal / determinant;
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = uniforms.model_view_projection * vec4<f32>(input.position, 1.0);
    output.world_position = (uniforms.model * vec4<f32>(input.position, 1.0)).xyz;
    output.uv = input.uv + uniforms.surface.xy;
    output.ambient_occlusion = input.ambient_occlusion;
    output.normal = transform_normal(input.normal);
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let sampled = uniforms.base_color * textureSample(cube_texture, cube_sampler, input.uv);
    // A cutout surface has holes where its art does, rather than a square.
    if sampled.a < uniforms.surface.w {
        discard;
    }
    let dx = dpdx(input.world_position);
    let dy = dpdy(input.world_position);
    // A framebuffer's rows run downward, so screen-right crossed with
    // screen-down points into the surface. Crossed the other way it points
    // out of it, toward the viewer, which is the side the light has to reach.
    let geometric_normal = normalize(cross(dy, dx));
    var normal = geometric_normal;
    if dot(input.normal, input.normal) > 0.0 {
        normal = normalize(input.normal);
    }
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
    var lit = sampled.rgb * (ambient + directional) * contact_visibility
        + sampled.rgb * uniforms.surface.z;
    if uniforms.material.z > 0.5 {
        lit = standard_lighting(sampled.rgb, normal, -light_direction,
            safe_normalize(uniforms.camera_position.xyz - input.world_position),
            uniforms.material.x, uniforms.material.y, ambient,
            uniforms.directional_color.rgb * uniforms.directional_direction.a * visibility)
            * contact_visibility + sampled.rgb * uniforms.surface.z + uniforms.emissive.rgb;
    }
    lit += local_lighting(uniforms.local_lights, sampled.rgb, normal, input.world_position,
        safe_normalize(uniforms.camera_position.xyz - input.world_position),
        uniforms.material.x, uniforms.material.y, uniforms.material.z > 0.5) * contact_visibility;
    return vec4<f32>(atmosphere(lit, input.world_position, uniforms.camera_position,
        uniforms.fog_color, uniforms.fog_params), sampled.a);
}
