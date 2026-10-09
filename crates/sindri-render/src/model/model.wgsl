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
fn safe_normalize(v: vec3<f32>) -> vec3<f32> {
    return v * inverseSqrt(max(dot(v, v), 0.0000001));
}
@fragment fn fs_main(v: Vertex, @builtin(front_facing) front: bool) -> @location(0) vec4<f32> {
    let facing = front == (u.direction.w > 0.0);
    if !facing && u.surface.w < 0.5 { discard; }
    let base = u.base_color * textureSample(color_texture, color_sampler, v.uv);
    if base.a < u.surface.z { discard; }
    let n = safe_normalize(v.normal) * select(-1.0, 1.0, facing);
    let l = safe_normalize(u.direction.xyz);
    let view = safe_normalize(u.eye.xyz - v.position);
    let h = safe_normalize(l + view);
    let nl = max(dot(n, l), 0.0);
    let nv = max(dot(n, view), 0.0001);
    let nh = max(dot(n, h), 0.0);
    let vh = max(dot(view, h), 0.0);
    let rough = max(u.surface.y, 0.045);
    let a = rough * rough;
    let a2 = a * a;
    let d = nh * nh * (a2 - 1.0) + 1.0;
    let distribution = a2 / max(3.14159265 * d * d, 0.000001);
    let k = (rough + 1.0) * (rough + 1.0) / 8.0;
    let visibility = nv / (nv * (1.0 - k) + k) * nl / (nl * (1.0 - k) + k);
    let f0 = mix(vec3(0.04), base.rgb, u.surface.x);
    let fresnel = f0 + (vec3(1.0) - f0) * pow(1.0 - vh, 5.0);
    let specular = distribution * visibility * fresnel / max(4.0 * nv * nl, 0.0001);
    let diffuse = (vec3(1.0) - fresnel) * (1.0 - u.surface.x) * base.rgb / 3.14159265;
    // Ambient is the existing world-light approximation, without an IBL map.
    let color = base.rgb * u.ambient.rgb + (diffuse + specular) * u.light.rgb * nl;
    return vec4(color, 1.0);
}
