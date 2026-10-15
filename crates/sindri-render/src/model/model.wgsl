struct Uniforms {
    view_projection: mat4x4<f32>,
    world: mat4x4<f32>,
    normal: mat4x4<f32>,
    base_color: vec4<f32>,
    // metallic, roughness, alpha cutoff (-1 for opaque), double sided
    surface: vec4<f32>,
    ambient: vec4<f32>,
    // direction toward light; w is orientation sign for mirrored instances
    direction: vec4<f32>,
    light: vec4<f32>,
    eye: vec4<f32>,
    light_view_projection: mat4x4<f32>,
    shadow: vec4<f32>,
    fog_color: vec4<f32>,
    fog_params: vec4<f32>,
    emissive: vec4<f32>,
    point_lights: array<PointLight, 8>,
}
@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var color_texture: texture_2d<f32>;
@group(0) @binding(2) var color_sampler: sampler;
struct Vertex {
    @builtin(position) clip: vec4<f32>,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}
@vertex fn vs_main(@location(0) position: vec3<f32>, @location(1) normal: vec3<f32>, @location(2) uv: vec2<f32>) -> Vertex {
    var v: Vertex;
    let p = u.world * vec4(position, 1.0);
    v.clip = u.view_projection * p;
    v.position = p.xyz;
    v.normal = (u.normal * vec4(normal, 0.0)).xyz;
    v.uv = uv;
    return v;
}
@fragment fn fs_main(v: Vertex, @builtin(front_facing) front: bool) -> @location(0) vec4<f32> {
    let facing = front == (u.direction.w > 0.0);
    if !facing && u.surface.w < 0.5 { discard; }
    let base = u.base_color * textureSample(color_texture, color_sampler, v.uv);
    if base.a < u.surface.z { discard; }
    let n = safe_normalize(v.normal) * select(-1.0, 1.0, facing);
    let l = safe_normalize(u.direction.xyz);
    let view = safe_normalize(u.eye.xyz - v.position);
    let nl = max(dot(n, l), 0.0);
    let shadow_visibility = directional_visibility(v.position, u.light_view_projection, u.shadow, nl);
    let color = standard_lighting(base.rgb, n, l, view, u.surface.x, u.surface.y,
        u.ambient.rgb, u.light.rgb * shadow_visibility) + u.emissive.rgb
        + point_lighting(u.point_lights, base.rgb, n, v.position, view,
            u.surface.x, u.surface.y, true);
    return vec4(atmosphere(color, v.position, u.eye, u.fog_color, u.fog_params), 1.0);
}
