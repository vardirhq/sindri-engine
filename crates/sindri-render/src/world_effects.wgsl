// Shared by textured world geometry and imported models. Bindings stay identical
// so both paths sample the same directional map and atmosphere contract.
@group(0) @binding(3) var shadow_map: texture_depth_2d;
@group(0) @binding(4) var shadow_sampler: sampler_comparison;

fn directional_visibility(world_position: vec3<f32>, light_view_projection: mat4x4<f32>,
                          shadow: vec4<f32>, diffuse: f32) -> f32 {
    let shadow_clip = light_view_projection * vec4<f32>(world_position, 1.0);
    let shadow_ndc = shadow_clip.xyz / shadow_clip.w;
    let shadow_uv = vec2<f32>(shadow_ndc.x * 0.5 + 0.5, 0.5 - shadow_ndc.y * 0.5);
    let sampled_visibility = textureSampleCompare(
        shadow_map,
        shadow_sampler,
        shadow_uv,
        shadow_ndc.z - shadow.x,
    );
    let inside_shadow_map = all(shadow_uv >= vec2<f32>(0.0))
        && all(shadow_uv <= vec2<f32>(1.0))
        && shadow_ndc.z >= 0.0
        && shadow_ndc.z <= 1.0;
    let receives_shadow = shadow.y > 0.5 && diffuse > 0.0 && inside_shadow_map;
    return select(1.0, sampled_visibility, receives_shadow);
}

fn atmosphere(lit: vec3<f32>, world_position: vec3<f32>, camera_position: vec4<f32>,
              fog_color: vec4<f32>, fog_params: vec4<f32>) -> vec3<f32> {
    let camera_distance = distance(world_position, camera_position.xyz);
    let fog_distance = max(camera_distance - fog_params.x, 0.0);
    let distance_fog = clamp(fog_distance / max(fog_params.y, 0.0001), 0.0, 1.0);
    let exponential_fog = 1.0 - exp(-fog_params.z * fog_distance);
    let height_fog = clamp(
        (fog_params.w - world_position.y) * camera_position.w,
        0.0,
        1.0,
    );
    let fog_amount = clamp(max(distance_fog, exponential_fog) + height_fog, 0.0, 1.0)
        * fog_color.a;
    return mix(lit, fog_color.rgb, fog_amount);
}
