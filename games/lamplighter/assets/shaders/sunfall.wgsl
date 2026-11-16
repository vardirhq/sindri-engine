// A sunfall's shaft: cracklight through the Lid. Brightness drifts down it in
// slow bands, as light does through moving air, and the whole of it breathes,
// so it is never quite still.
//
// params.x is how strong it is, 0 to 1; params.y shifts its rhythms, so shafts
// side by side do not breathe in step.
fn sprite_shader(input: SpriteShaderInput) -> vec4<f32> {
    let p = input.local_uv;
    let t = input.time + input.params.y;
    // Bands falling down the beam, a little aslant, crossing a slower set.
    let falling = 0.5 + 0.5 * sin(p.y * 14.0 - t * 1.4 + p.x * 3.0);
    let slow = 0.5 + 0.5 * sin(p.y * 5.0 - t * 0.5 - p.x * 2.0);
    let bands = 0.7 + 0.2 * falling + 0.1 * slow;
    let breathe = 0.85 + 0.15 * sin(t * 0.55);
    return vec4<f32>(input.color.rgb, input.color.a * bands * breathe * input.params.x);
}
