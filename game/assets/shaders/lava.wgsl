// Lava: a dark crust broken into plates, with fire showing in the cracks
// between them, the plates creeping and the cracks breathing brighter and
// dimmer. Drawn on the lake round a burning island and on every lava pool, so
// everything comes from the pixel's world position and the pools line up with
// the lake.
//
// The tint is the crust's color, which is how a mood darkens or warms it. The
// params are strengths:
//   x  heat: how bright the cracks burn (0..2)
//   y  how fast the crust creeps (0..1)
//   z  how much of the surface is open fire rather than crust (0..1)
//   w  alpha: 1 for the lake, which is the whole floor, less to let it fade

fn lava_hash(p: vec2<f32>) -> f32 {
    let q = fract(p * vec2<f32>(127.1, 311.7));
    return fract((q.x + q.y) * 43758.5453 + dot(q, q + 19.19));
}

fn lava_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = lava_hash(i);
    let b = lava_hash(i + vec2<f32>(1.0, 0.0));
    let c = lava_hash(i + vec2<f32>(0.0, 1.0));
    let d = lava_hash(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// How far a point is from the crack at the edge of its crust plate: 0 on the
// crack, rising into the plate. Plates are cells of a jittered grid, and the
// distance is the gap between the nearest and second-nearest centre.
fn lava_cracks(p: vec2<f32>) -> f32 {
    let cell = floor(p);
    var nearest = 9.0;
    var second = 9.0;
    for (var j = -1; j <= 1; j++) {
        for (var i = -1; i <= 1; i++) {
            let c = cell + vec2<f32>(f32(i), f32(j));
            let jitter = vec2<f32>(lava_hash(c), lava_hash(c + vec2<f32>(7.0, 3.0)));
            let d = length(p - c - jitter);
            if d < nearest {
                second = nearest;
                nearest = d;
            } else if d < second {
                second = d;
            }
        }
    }
    return second - nearest;
}

fn sprite_shader(input: SpriteShaderInput) -> vec4<f32> {
    let t = input.time;
    let creep = input.params.y;
    // The crust drifts and is warped slowly, so the plates seem to float.
    let drift = vec2<f32>(t * 0.05, t * 0.03) * creep;
    let warp = vec2<f32>(lava_noise(input.world_position.xz * 0.4 + drift),
                         lava_noise(input.world_position.zx * 0.4 - drift)) - 0.5;
    let p = input.world_position.xz * 0.8 + warp * 0.7 + drift;

    // A thin bright vein along each crack, and a faint warmth spreading into
    // the plate beside it.
    let gap = lava_cracks(p);
    let vein = 1.0 - smoothstep(0.0, 0.045, gap);
    let warmth = (1.0 - smoothstep(0.0, 0.18, gap)) * 0.22;
    // Here and there the crust has broken open into a pool of fire.
    let open = smoothstep(1.0 - input.params.z * 0.5, 1.0, lava_noise(p * 0.45 + vec2<f32>(t * 0.05, 0.0)));
    let breath = 0.7 + 0.3 * sin(t * 1.3 + lava_noise(p * 0.5) * 6.28);
    let heat = clamp((max(vein, open) + warmth) * breath, 0.0, 1.0) * input.params.x;

    let crust = input.color.rgb * (0.45 + 0.4 * lava_noise(p * 2.3));
    let fire = mix(vec3<f32>(0.8, 0.14, 0.02), vec3<f32>(1.0, 0.78, 0.32), clamp(heat - 0.7, 0.0, 1.0));
    let color = mix(crust, fire, clamp(heat, 0.0, 1.0));
    return vec4<f32>(color, clamp(input.params.w, 0.0, 1.0));
}
