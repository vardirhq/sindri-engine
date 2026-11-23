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
    // Shadow bias and enable flag, normal texture scale, and packed texture enable flag.
    shadow: vec4<f32>,
    fog_color: vec4<f32>,
    fog_params: vec4<f32>,
    emissive: vec4<f32>,
    local_lights: array<LocalLight, 8>,
    occluded_color: vec4<f32>,
}
@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var color_texture: texture_2d<f32>;
@group(0) @binding(2) var color_sampler: sampler;
@group(0) @binding(5) var normal_texture: texture_2d<f32>;
@group(0) @binding(6) var normal_sampler: sampler;
@group(0) @binding(7) var metallic_roughness_texture: texture_2d<f32>;
@group(0) @binding(8) var metallic_roughness_sampler: sampler;
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
// Derive the tangent frame from UV0 and world-space position. This follows
// both mirrored UV islands and nonuniformly transformed model instances.
fn mapped_normal(v: Vertex, n: vec3<f32>) -> vec3<f32> {
    let scale = u.shadow.z;
    if scale == 0.0 { return n; }
    let dx = dpdx(v.position);
    let dy = dpdy(v.position);
    let du = dpdx(v.uv);
    let dv = dpdy(v.uv);
    let sampled = textureSample(normal_texture, normal_sampler, v.uv).rgb * 2.0 - 1.0;
    let det = du.x * dv.y - du.y * dv.x;
    if abs(det) < 1e-12 || dot(n, n) < 1e-20 { return n; }
    let along_u = (dx * dv.y - dy * du.y) * sign(det);
    let along_v = (dy * du.x - dx * dv.x) * sign(det);
    let projected = along_u - n * dot(n, along_u);
    if dot(projected, projected) < 1e-20 { return n; }
    let tangent = normalize(projected);
    let perpendicular = cross(n, tangent);
    let bitangent = perpendicular * sign(dot(perpendicular, along_v));
    let mapped = tangent * (sampled.x * scale) + bitangent * (sampled.y * scale) + n * sampled.z;
    if dot(mapped, mapped) < 1e-20 { return n; }
    return normalize(mapped);
}

@fragment fn fs_main(v: Vertex, @builtin(front_facing) front: bool) -> @location(0) vec4<f32> {
    let facing = front == (u.direction.w > 0.0);
    let facing_sign = select(-1.0, 1.0, facing);
    let authored = safe_normalize(v.normal);
    let n = mapped_normal(v, authored) * facing_sign;
    var factors = vec2<f32>(1.0);
    if u.shadow.w > 0.5 {
        factors = textureSample(metallic_roughness_texture, metallic_roughness_sampler, v.uv).gb;
    }
    let base = u.base_color * textureSample(color_texture, color_sampler, v.uv);
    if !facing && u.surface.w < 0.5 { discard; }
    if base.a < u.surface.z { discard; }
    let metallic = u.surface.x * factors.y;
    let roughness = u.surface.y * factors.x;
    let l = safe_normalize(u.direction.xyz);
    let view = safe_normalize(u.eye.xyz - v.position);
    let nl = max(dot(n, l), 0.0);
    let shadow_visibility = directional_visibility(v.position, u.light_view_projection, u.shadow, nl);
    let color = standard_lighting(base.rgb, n, l, view, metallic, roughness,
        u.ambient.rgb, u.light.rgb * shadow_visibility) + u.emissive.rgb
        + local_lighting(u.local_lights, base.rgb, n, v.position, view, metallic, roughness, true);
    return vec4(atmosphere(color, v.position, u.eye, u.fog_color, u.fog_params), 1.0);
}

@fragment fn fs_occluded(v: Vertex, @builtin(front_facing) front: bool) -> @location(0) vec4<f32> {
    let facing = front == (u.direction.w > 0.0);
    if !facing && u.surface.w < 0.5 { discard; }
    let base = u.base_color * textureSample(color_texture, color_sampler, v.uv);
    if base.a < u.surface.z { discard; }
    return u.occluded_color;
}
