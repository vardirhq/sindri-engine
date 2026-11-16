// A map's water (`map_water`), drawn as a sprite shader over each water square.
//
// The square is drawn with no tint and the water's colour at its depth as its
// colour offset, worked out once with the squares, so `input.color` is that
// colour. Its parameters say the rest:
//   x  nothing yet
//   y  which of its eight neighbours are land, as bits: north 1, east 2,
//      south 4, west 8, north-east 16, south-east 32, south-west 64,
//      north-west 128
//   z  how far it is from the shore in whole squares, times 16, plus its pace
//      in quarters (0 to 15)
//   w  how much it sways, glints and foams, a fifteenth at a time: sway, then
//      glints times 16, then foam times 256
//
// Everything moves in whole pixels of the block's own picture, so the water
// stays as crisp as the grass beside it.

fn water_hash(p: vec2<f32>) -> f32 {
    let q = fract(p * vec2<f32>(0.1031, 0.1030));
    let r = q + dot(q, q.yx + 33.33);
    return fract((r.x + r.y) * r.x);
}

fn water_noise(p: vec2<f32>) -> f32 {
    let cell = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = water_hash(cell);
    let b = water_hash(cell + vec2<f32>(1.0, 0.0));
    let c = water_hash(cell + vec2<f32>(0.0, 1.0));
    let d = water_hash(cell + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

fn water_bit(bits: f32, bit: f32) -> bool {
    return fract(floor(bits / bit) * 0.5) > 0.25;
}

// How far a point of the square (0..1 across, 0..1 down) is from the land
// beside it, in squares; 2 where none is near.
fn water_from_land(at: vec2<f32>, land: f32) -> f32 {
    var near = 2.0;
    if water_bit(land, 1.0) { near = min(near, at.y); }
    if water_bit(land, 2.0) { near = min(near, 1.0 - at.x); }
    if water_bit(land, 4.0) { near = min(near, 1.0 - at.y); }
    if water_bit(land, 8.0) { near = min(near, at.x); }
    if water_bit(land, 16.0) { near = min(near, length(at - vec2<f32>(1.0, 0.0))); }
    if water_bit(land, 32.0) { near = min(near, length(at - vec2<f32>(1.0, 1.0))); }
    if water_bit(land, 64.0) { near = min(near, length(at - vec2<f32>(0.0, 1.0))); }
    if water_bit(land, 128.0) { near = min(near, length(at)); }
    return near;
}

fn sprite_shader(input: SpriteShaderInput) -> vec4<f32> {
    let texel = sprite_texel_size();
    // How many of the picture's pixels a square is across.
    let pixels = max(round(input.uv_rect.zw / texel), vec2<f32>(1.0, 1.0));
    // This pixel's place in the world, snapped to the picture's pixels.
    let world = (floor(input.world_position.xy * pixels) + 0.5) / pixels;
    let at = (floor(input.local_uv * pixels) + 0.5) / pixels;

    let packed = input.params.z;
    let shore = floor(packed / 16.0);
    let pace = (packed - shore * 16.0) * 0.25;
    let amounts = input.params.w;
    let foam_amount = floor(amounts / 256.0) / 15.0;
    let glint_amount = floor((amounts - foam_amount * 15.0 * 256.0) / 16.0) / 15.0;
    let sway_amount = (amounts - foam_amount * 15.0 * 256.0 - glint_amount * 15.0 * 16.0) / 15.0;
    let t = input.time * pace;

    // The surface sways: rows of pixels slide a pixel or two to and fro, as
    // a reflection on moving water does.
    let row = floor(world.y * pixels.y);
    let slide = round(sin(row * 0.7 + t * 1.6) * 1.4 * sway_amount);
    let local = fract((input.uv - input.uv_rect.xy) / input.uv_rect.zw + vec2<f32>(slide / pixels.x, 0.0));
    let picture = sample_sprite(input.uv_rect.xy + local * input.uv_rect.zw);
    let detail = dot(picture.rgb, vec3<f32>(0.3, 0.55, 0.15));
    // The picture's own light and dark, around the water's colour.
    var colour = input.color.rgb * (0.7 + (detail - 0.35) * 0.9);

    // Light ripples drift across it in two slow layers, brighter where they
    // cross.
    let drift = world * vec2<f32>(0.9, 2.2);
    let one = water_noise(drift + vec2<f32>(t * 0.35, t * 0.12));
    let two = water_noise(drift * 1.7 - vec2<f32>(t * 0.21, -t * 0.17) + 19.0);
    let crest = smoothstep(0.62, 0.7, one * 0.6 + two * 0.4);
    colour = colour + input.color.rgb * vec3<f32>(0.5, 0.6, 0.6) * crest * sway_amount;

    // Glints: a pixel here and there on a crest catches the light for a
    // moment and is gone.
    let pixel = floor(world * pixels);
    let blink = t * 2.0 + water_hash(pixel) * 9.0;
    let lit = step(1.0 - 0.02 * glint_amount, water_hash(pixel + floor(blink) * 17.0));
    let fade = 1.0 - fract(blink);
    let glint = lit * fade * fade * smoothstep(0.45, 0.65, one);
    // Bright enough to flash under a lantern at night as well as by day.
    colour = colour + vec3<f32>(2.0, 2.1, 2.2) * glint;

    // Foam laps at the land: a band that runs up the shore and back, and a
    // thinner line following it out.
    let land = input.params.y;
    if shore < 0.5 && land > 0.5 && foam_amount > 0.0 {
        let near = water_from_land(at, land);
        let phase = water_noise(world * 0.8) * 6.28;
        let lap = 0.14 + 0.07 * sin(t * 1.1 + phase);
        let edge = water_noise(world * 3.0 + t * 0.2) * 0.05;
        let band = step(near + edge, lap);
        let line = step(abs(near - lap - 0.12 - edge), 0.035) * (0.5 + 0.5 * sin(t * 1.1 + phase + 1.6));
        let froth = max(band, line * 0.7) * foam_amount;
        colour = mix(colour, vec3<f32>(0.75, 0.82, 0.85), froth * 0.85);
    }
    return vec4<f32>(colour, input.color.a);
}
