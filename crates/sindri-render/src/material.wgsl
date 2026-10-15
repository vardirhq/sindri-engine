struct PointLight {
    position_range: vec4<f32>,
    color_intensity: vec4<f32>,
}

fn safe_normalize(v: vec3<f32>) -> vec3<f32> {
    return v * inverseSqrt(max(dot(v, v), 0.0000001));
}

// Finite center, inverse-square falloff beyond one world unit, and a smooth
// zero at range. Local lights are independent of the directional shadow map.
fn point_lighting(lights: array<PointLight, 8>, base: vec3<f32>, normal: vec3<f32>,
                  position: vec3<f32>, view: vec3<f32>, metallic: f32,
                  roughness: f32, standard: bool) -> vec3<f32> {
    var color = vec3<f32>(0.0);
    for (var i = 0u; i < 8u; i += 1u) {
        let light = lights[i];
        let delta = light.position_range.xyz - position;
        let distance_squared = dot(delta, delta);
        let range = light.position_range.w;
        if light.color_intensity.w > 0.0 && range > 0.0 && distance_squared < range * range {
            let relative = sqrt(distance_squared) / range;
            let window = 1.0 - relative * relative;
            let radiance = light.color_intensity.rgb * light.color_intensity.w
                * window * window / max(distance_squared, 1.0);
            let direction = safe_normalize(delta);
            if standard {
                color += standard_lighting(base, normal, direction, view, metallic,
                    roughness, vec3<f32>(0.0), radiance);
            } else {
                color += base * radiance * max(dot(normal, direction), 0.0);
            }
        }
    }
    return color;
}
// Direct GGX plus the shared ambient approximation; no environment map.
fn standard_lighting(base: vec3<f32>, n: vec3<f32>, l: vec3<f32>, view: vec3<f32>,
                     metallic: f32, roughness: f32, ambient: vec3<f32>, radiance: vec3<f32>) -> vec3<f32> {
    let nl = max(dot(n, l), 0.0);
    let h = safe_normalize(l + view);
    let nv = max(dot(n, view), 0.0001);
    let nh = max(dot(n, h), 0.0);
    let vh = max(dot(view, h), 0.0);
    let rough = max(roughness, 0.045);
    let a = rough * rough;
    let a2 = a * a;
    let d = nh * nh * (a2 - 1.0) + 1.0;
    let distribution = a2 / max(3.14159265 * d * d, 0.000001);
    let k = (rough + 1.0) * (rough + 1.0) / 8.0;
    let visibility = nv / (nv * (1.0 - k) + k) * nl / (nl * (1.0 - k) + k);
    let f0 = mix(vec3(0.04), base, metallic);
    let fresnel = f0 + (vec3(1.0) - f0) * pow(1.0 - vh, 5.0);
    let specular = distribution * visibility * fresnel / max(4.0 * nv * nl, 0.0001);
    let diffuse = (vec3(1.0) - fresnel) * (1.0 - metallic) * base / 3.14159265;
    return base * ambient + (diffuse + specular) * radiance * nl;
}
