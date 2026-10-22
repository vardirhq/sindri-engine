// The sea round Causeway's islands: shallow and clear at the shore, deep
// further out, a slow swell rolling across it, light glinting off the crests
// and foam breaking along the island's edge.
//
// Drawn on flat tiles laid over the world, so everything comes from the
// pixel's world position. The tile's tint is the deep water's color, which is
// how a mood recolors the sea. The params are strengths:
//   x  swell, how much the rolling bands lighten and darken (0..1)
//   y  foam along the shore (0..1)
//   z  glints off the crests (0..1)
//   w  how far the island's shallows reach, in world units

// The island's footprint, an 8 x 8 board from (24, 24), and the causeway
// running east from its middle.
const ISLAND_MIN: vec2<f32> = vec2<f32>(24.0, 24.0);
const ISLAND_MAX: vec2<f32> = vec2<f32>(32.0, 32.0);

fn sea_hash(p: vec2<f32>) -> f32 {
    let q = fract(p * vec2<f32>(127.1, 311.7));
    return fract((q.x + q.y) * 43758.5453 + dot(q, q + 19.19));
}

fn sea_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = sea_hash(i);
    let b = sea_hash(i + vec2<f32>(1.0, 0.0));
    let c = sea_hash(i + vec2<f32>(0.0, 1.0));
    let d = sea_hash(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// How far a point is outside the island's square, rounded at the corners.
fn shore_distance(p: vec2<f32>) -> f32 {
    let centre = (ISLAND_MIN + ISLAND_MAX) * 0.5;
    let half = (ISLAND_MAX - ISLAND_MIN) * 0.5 - vec2<f32>(0.6);
    let q = abs(p - centre) - half;
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - 0.6;
}

fn sprite_shader(input: SpriteShaderInput) -> vec4<f32> {
    let p = input.world_position.xz;
    let t = input.time;
    let deep = input.color.rgb;
    let reach = max(input.params.w, 0.5);

    // Shallows: the deep color lifted towards turquoise near the shore.
    let d = shore_distance(p);
    let shallow_mix = 1.0 - smoothstep(0.0, reach, d);
    let shallow = deep * 1.6 + vec3<f32>(0.02, 0.12, 0.1);
    var color = mix(deep, shallow, shallow_mix * shallow_mix);

    // The swell: broad bands rolling diagonally, broken up by slow noise.
    let rolling = sin(dot(p, vec2<f32>(0.42, 0.31)) - t * 0.9)
        + 0.6 * sin(dot(p, vec2<f32>(-0.23, 0.57)) - t * 0.65);
    let warp = sea_noise(p * 0.35 + vec2<f32>(t * 0.05, -t * 0.04));
    let swell = (rolling * 0.25 + (warp - 0.5)) * input.params.x;
    color *= 1.0 + swell * 0.35;

    // Glints: small bright flecks where crests catch the light, drifting.
    let fleck = sea_noise(p * 7.0 + vec2<f32>(t * 0.6, t * 0.45))
        * sea_noise(p * 11.0 - vec2<f32>(t * 0.4, -t * 0.7));
    let crest = smoothstep(0.6, 1.4, rolling);
    let glint = smoothstep(0.7, 0.85, fleck) * crest * input.params.z * 0.9;
    color += vec3<f32>(1.0, 0.97, 0.9) * max(glint, 0.0);

    // Foam: a broken line hugging the shore, washing in and out.
    let wash = 0.25 + 0.2 * sin(t * 1.3 + p.x * 0.7 + p.y * 0.5);
    let edge = 1.0 - smoothstep(0.0, wash, abs(d - 0.05));
    let lace = smoothstep(0.35, 0.75, sea_noise(p * 4.0 + vec2<f32>(t * 0.3, t * 0.2)));
    let ring = (1.0 - smoothstep(0.0, 0.06, abs(d - 0.35 - wash))) * lace * 0.25;
    let foam = clamp((edge * (0.5 + lace * 0.5) + ring) * input.params.y, 0.0, 1.0);
    color = mix(color, vec3<f32>(0.8, 0.88, 0.9) * (0.45 + deep * 1.5), foam);

    // Clearer at the shore, so the seabed shows through, and deeper out.
    let alpha = mix(input.color.a, input.color.a * 0.72, shallow_mix) + foam * 0.3;
    return vec4<f32>(color, clamp(alpha, 0.0, 1.0));
}
