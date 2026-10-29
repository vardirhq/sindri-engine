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
    // light still reaches; z is the emitting area's radius.
    shadow: vec4<f32>,
    // A spot light's beam: x, y the unit direction, z and w the cosines of the
    // angle at which it fades out and the angle inside which it is full. z below
    // -1 means no beam. See `LightCone2d::factor`, which this agrees with.
    cone: vec4<f32>,
}

struct SindriLights2d {
    // rgb ambient; w is 1 when 2D lighting is on.
    ambient: vec4<f32>,
    // x is how many lights are set.
    header: vec4<f32>,
    // The viewer: x, y its place, z how far it sees (zero is as far as there is),
    // w what lies outside its sight is multiplied by.
    sight: vec4<f32>,
    // x is 1 when there is a viewer, y how far past an occluding edge it sees.
    sight_edge: vec4<f32>,
    lights: array<SindriLight2d, 16>,
}

@group(0) @binding(3)
var<uniform> sindri_lights_2d: SindriLights2d;

// Five rows per light, one texel per angular interval: where it starts (an angle
// from -pi), the unit normal of the edge that blocks it, and that edge's line's
// distance from the source, negative where nothing blocks. Point lights use only
// the first row. See `shadow.rs`, which builds them and which this must agree with.
@group(0) @binding(4)
var sindri_shadow_map_2d: texture_2d<f32>;

fn sindri_lighting_2d_on() -> bool {
    return sindri_lights_2d.ambient.w > 0.5;
}

// How much of light `index` reaches a point `distance` from it in direction
// `away`, 0 to 1: the share of its samples with no blocking edge between them
// and the point. Each finds the interval its direction falls in by binary
// search and compares the point's distance with the edge's, exactly.
fn sindri_shadow_2d(index: u32, world: vec2<f32>, origin: vec2<f32>, reach: f32, source_radius: f32) -> f32 {
    let intervals = i32(textureDimensions(sindri_shadow_map_2d).x);
    let offsets = array<vec2<f32>, 5>(vec2<f32>(0.0, 0.0), vec2<f32>(-0.75, 0.0),
        vec2<f32>(0.75, 0.0), vec2<f32>(0.0, -0.75), vec2<f32>(0.0, 0.75));
    let count = select(1u, 5u, source_radius > 0.0);
    var seen = 0.0;
    for (var sample = 0u; sample < count; sample++) {
        let away = world - (origin + offsets[sample] * source_radius);
        let distance = length(away);
        let angle = atan2(away.y, away.x);
        let row = i32(index * 5u + sample);
        var low = 0;
        var high = intervals - 1;
        while low < high {
            let middle = (low + high + 1) / 2;
            if textureLoad(sindri_shadow_map_2d, vec2<i32>(middle, row), 0).x <= angle {
                low = middle;
            } else {
                high = middle - 1;
            }
        }
        let interval = textureLoad(sindri_shadow_map_2d, vec2<i32>(low, row), 0);
        if interval.w < 0.0 {
            seen += 1.0;
        } else {
            let facing = dot(interval.yz, away) / max(distance, 1.0e-9);
            let edge = interval.w / max(facing, 1.0e-6);
            seen += select(0.0, 1.0, distance <= edge + reach);
        }
    }
    return seen / f32(count);
}

// What a point is multiplied by for being out of the viewer's sight: 1 where it
// is seen, the viewer's `unseen` where it is not. The viewer's shadow map is the
// row after every light's. See `Sight2d::factor`, which this agrees with.
fn sindri_sight_2d(world: vec2<f32>) -> f32 {
    if sindri_lights_2d.sight_edge.x < 0.5 {
        return 1.0;
    }
    let place = sindri_lights_2d.sight.xy;
    let radius = sindri_lights_2d.sight.z;
    let unseen = sindri_lights_2d.sight.w;
    var seen = sindri_shadow_2d(16u, world, place, sindri_lights_2d.sight_edge.y, 0.0);
    if radius > 0.0 {
        seen *= 1.0 - smoothstep(radius * 0.75, radius, length(world - place));
    }
    return unseen + (1.0 - unseen) * seen;
}

// The light reaching `world`: ambient plus every light. `normal` is used only
// when `use_normal` is set, so art without a normal map is lit by distance
// alone.
fn sindri_light_2d(world: vec2<f32>, normal: vec3<f32>, use_normal: bool) -> vec3<f32> {
    return sindri_light_2d_from(world, world, normal, use_normal);
}

// The same, for something standing on the ground at `base`: how far each light
// reaches is worked out at the pixel, which is what shades one end of it
// brighter than the other, but whether a shadow covers it is asked at its feet,
// so the occluder round its own base does not darken the side facing the light.
fn sindri_light_2d_from(world: vec2<f32>, base: vec2<f32>, normal: vec3<f32>, use_normal: bool) -> vec3<f32> {
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
        if light.cone.z > -1.5 && distance > 0.000001 {
            let across = dot(-to_light / distance, light.cone.xy);
            let t = clamp((across - light.cone.z) / (light.cone.w - light.cone.z), 0.0, 1.0);
            amount *= t * t * (3.0 - 2.0 * t);
        }
        if use_normal {
            let toward = normalize(vec3<f32>(to_light, light.position.z));
            amount *= max(dot(normal, toward), 0.0);
        }
        if light.shadow.x > 0.5 {
            amount *= sindri_shadow_2d(i, base, light.position.xy, light.shadow.y, light.shadow.z);
        }
        total += light.color.rgb * amount;
    }
    return total * sindri_sight_2d(base);
}
