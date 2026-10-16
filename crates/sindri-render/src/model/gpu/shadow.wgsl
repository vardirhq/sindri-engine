struct Uniforms {
    light_world: mat4x4<f32>,
    base_color: vec4<f32>,
    // alpha cutoff (-1 for opaque), double sided, orientation sign, padding
    surface: vec4<f32>,
}
@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var color_texture: texture_2d<f32>;
@group(0) @binding(2) var color_sampler: sampler;
struct Vertex {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
}
@vertex fn vs_main(@location(0) position: vec3<f32>, @location(2) uv: vec2<f32>) -> Vertex {
    var v: Vertex;
    v.clip = u.light_world * vec4(position, 1.0);
    v.uv = uv;
    return v;
}
@fragment fn fs_main(v: Vertex, @builtin(front_facing) front: bool) {
    let facing = front == (u.surface.z > 0.0);
    if !facing && u.surface.y < 0.5 { discard; }
    let alpha = u.base_color.a * textureSample(color_texture, color_sampler, v.uv).a;
    if alpha < u.surface.x { discard; }
}
