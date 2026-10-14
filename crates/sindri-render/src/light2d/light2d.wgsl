// 2D lighting, shared by the sprite and shape pipelines.
//
// Lights are points in the plane. A pixel takes the ambient color plus each
// light's color, scaled by how far into the light's radius it sits and, where
// the light casts shadows, by how much of it the light's polar shadow map says
// it can see. Both pipelines declare these at bindings 3 and 4.

struct SindriLight2d {
    // x, y, height above the plane, radius.
    position: vec4<f32>,
    // rgb already multiplied by intensity; w is the falloff exponent.
    color: vec4<f32>,
    // x is 1 when the light casts shadows, y how far past an occluding edge
    // light still reaches.
    shadow: vec4<f32>,
}

struct SindriLights2d {
    // rgb ambient; w is 1 when 2D lighting is on.
    ambient: vec4<f32>,
    // x is how many lights are set.
    header: vec4<f32>,
    lights: array<SindriLight2d, 16>,
}

@group(0) @binding(3)
var<uniform> sindri_lights_2d: SindriLights2d;

// One row per light, one texel per direction: the distance from the light to
// the nearest occluding edge that way.
@group(0) @binding(4)
var sindri_shadow_map_2d: texture_2d<f32>;

const SINDRI_TAU: f32 = 6.283185307179586;

fn sindri_lighting_2d_on() -> bool {
    return sindri_lights_2d.ambient.w > 0.5;
}

// How much of light `index` reaches a point `distance` from it in direction
// `away`, 0 to 1. Three neighbouring directions are averaged, which softens
// a shadow's edge by one bin.
fn sindri_shadow_2d(index: u32, away: vec2<f32>, distance: f32, reach: f32) -> f32 {
    let bins = i32(textureDimensions(sindri_shadow_map_2d).x);
    let turn = (atan2(away.y, away.x) + SINDRI_TAU * 0.5) / SINDRI_TAU;
    let center = clamp(i32(floor(turn * f32(bins))), 0, bins - 1);
    var seen = 0.0;
    for (var k = -1; k <= 1; k++) {
        let bin = (center + k + bins) % bins;
        let edge = textureLoad(sindri_shadow_map_2d, vec2<i32>(bin, i32(index)), 0).r;
        seen += select(0.0, 1.0, distance <= edge + reach);
    }
    return seen / 3.0;
}

// The light reaching `world`: ambient plus every light. `normal` is used only
// when `use_normal` is set, so art without a normal map is lit by distance
// alone.
fn sindri_light_2d(world: vec2<f32>, normal: vec3<f32>, use_normal: bool) -> vec3<f32> {
    var total = sindri_lights_2d.ambient.rgb;
    let count = u32(sindri_lights_2d.header.x);
    for (var i = 0u; i < count; i++) {
        let light = sindri_lights_2d.lights[i];
        let to_light = light.position.xy - world;
        let distance = length(to_light);
        let radius = light.position.w;
        if distance >= radius {
            continue;
        }
        var amount = pow(max(1.0 - distance / radius, 0.0), light.color.w);
        if use_normal {
            let toward = normalize(vec3<f32>(to_light, light.position.z));
            amount *= max(dot(normal, toward), 0.0);
        }
        if light.shadow.x > 0.5 {
            amount *= sindri_shadow_2d(i, -to_light, distance, light.shadow.y);
        }
        total += light.color.rgb * amount;
    }
    return total;
}
