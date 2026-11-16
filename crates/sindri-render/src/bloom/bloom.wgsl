// Bloom: the light a bright thing throws into the dark around it.
//
// Three passes over a full-screen triangle. The bright pass keeps what is
// brighter than a threshold, the blur pass spreads it, and the composite adds
// the result back over the scene. The chain runs at a quarter resolution
// because a blur is the one thing that is *better* for being cheap: the
// downsample is itself a blur, and the taps that follow reach four times as far
// across the picture for the same cost.
//
// Everything here is linear. The scene target is sRGB, so sampling it decodes;
// the chain's own targets are float, so nothing is re-encoded or clamped between
// passes; the composite writes to an sRGB target, which encodes once at the end.

struct Params {
    // xy: one texel of the source, in UV. zw: threshold and knee.
    source: vec4<f32>,
    // xy: blur direction, in texels. z: how much glow to add. w: unused.
    blur: vec4<f32>,
    // x: exposure stops, y: contrast, z: saturation, w: vignette.
    grade: vec4<f32>,
    // x: tone-map mode, y: bloom enabled. zw reserved.
    tone: vec4<f32>,
    // x: focus, y: range, z: falloff, w: blur, the depth of field's band.
    focus: vec4<f32>,
    // xy: one texel of the scene, in UV. z: depth of field on. w: unused.
    lens: vec4<f32>,
    // x: how bright the dark ends, y: its blur, z: how grey it is, w: on.
    night: vec4<f32>,
};

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var source_texture: texture_2d<f32>;
@group(0) @binding(2) var source_sampler: sampler;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

/// One triangle covering the viewport, built from the vertex index alone.
///
/// A triangle rather than two: the seam down the diagonal of a quad is a real
/// cost in a pass that runs three times a frame over every pixel, and there are
/// no vertex buffers to bind at all this way.
@vertex
fn vs_fullscreen(@builtin(vertex_index) index: u32) -> VertexOutput {
    let x = f32(i32(index) / 2) * 4.0 - 1.0;
    let y = f32(i32(index) & 1) * 4.0 - 1.0;
    var out: VertexOutput;
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>((x + 1.0) * 0.5, 1.0 - (y + 1.0) * 0.5);
    return out;
}

/// What is left of a color once everything below the threshold is taken away.
///
/// Soft rather than a straight cutoff. A hard threshold makes the glow appear
/// and vanish as a thing's brightness crosses it, which on anything that pulses
/// or fades reads as a flicker; the knee is a band either side where the
/// contribution ramps in instead.
fn above_threshold(color: vec3<f32>, threshold: f32, knee: f32) -> vec3<f32> {
    let brightness = max(color.r, max(color.g, color.b));
    let width = max(knee, 1e-4);
    var soft = clamp(brightness - threshold + width, 0.0, 2.0 * width);
    soft = soft * soft / (4.0 * width);
    let kept = max(soft, brightness - threshold) / max(brightness, 1e-4);
    return color * kept;
}

fn tone_map(color: vec3<f32>) -> vec3<f32> {
    if (params.tone.x < 0.5) {
        return color;
    }
    if (params.tone.x < 1.5) {
        return color / (vec3<f32>(1.0) + color);
    }
    // Narkowicz ACES approximation: compact, stable, and sufficient for a
    // world presentation curve without pulling a color-management system in.
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    return clamp((color * (a * color + b)) / (color * (c * color + d) + e), vec3<f32>(0.0), vec3<f32>(1.0));
}

fn grade_world(color: vec3<f32>) -> vec3<f32> {
    var graded = color * exp2(params.grade.x);
    graded = tone_map(graded);
    graded = (graded - vec3<f32>(0.5)) * params.grade.y + vec3<f32>(0.5);
    let luma = dot(graded, vec3<f32>(0.2126, 0.7152, 0.0722));
    graded = mix(vec3<f32>(luma), graded, params.grade.z);
    return max(graded, vec3<f32>(0.0));
}

@fragment
fn fs_bright(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = grade_world(textureSample(source_texture, source_sampler, in.uv).rgb);
    return vec4<f32>(above_threshold(color, params.source.z, params.source.w), 1.0);
}

// A nine-tap Gaussian, run once across and once down. Separable, so the two
// passes together cost eighteen taps rather than the eighty-one a single
// two-dimensional kernel of the same reach would.
const WEIGHTS: array<f32, 5> = array<f32, 5>(0.227027, 0.1945946, 0.1216216, 0.054054, 0.016216);

@fragment
fn fs_blur(in: VertexOutput) -> @location(0) vec4<f32> {
    let step = params.blur.xy * params.source.xy;
    var total = textureSample(source_texture, source_sampler, in.uv).rgb * WEIGHTS[0];
    for (var i = 1; i < 5; i = i + 1) {
        let offset = step * f32(i);
        total = total + textureSample(source_texture, source_sampler, in.uv + offset).rgb
            * WEIGHTS[i];
        total = total + textureSample(source_texture, source_sampler, in.uv - offset).rgb
            * WEIGHTS[i];
    }
    return vec4<f32>(total, 1.0);
}

@group(1) @binding(0) var glow_texture: texture_2d<f32>;

/// The scene with its own light added back over it.
///
/// Added rather than mixed: a glow is light arriving at the eye on top of what
/// is already there, so a bright edge over black gets brighter and the black
/// around it lifts. Mixing would wash the edge out towards the average instead,
/// which is fog.
// How many taps the lens's disc takes. A spiral of them, each a golden angle
// round from the last, covers the disc evenly at any count without the rings
// and spokes a grid of taps draws into a blur.
const LENS_TAPS: i32 = 32;
const GOLDEN_ANGLE: f32 = 2.39996323;

/// How far out of focus this row of the screen is, from 0 (sharp) to 1.
fn out_of_focus(y: f32) -> f32 {
    if (params.lens.z < 0.5) {
        return 0.0;
    }
    let off = abs(y - params.focus.x) - params.focus.y;
    return smoothstep(0.0, params.focus.z, off);
}

/// How far into the dark a color is, from 0 (seen plainly) to 1 (as dark as
/// the eye can see in).
///
/// By the light that reaches the eye, as the eye itself judges: in the dark it
/// is how little light there is, not what lit it, that takes colour and detail.
fn in_the_dark(color: vec3<f32>) -> f32 {
    if (params.night.w < 0.5) {
        return 0.0;
    }
    let luma = dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
    return 1.0 - smoothstep(0.0, params.night.x, luma);
}

/// How much a tap of this color, from this row, counts in a blur.
///
/// Taps that are sharp where they are count for less, so the band in focus
/// keeps its edge and a lit thing keeps its outline against the dark. Where
/// the eye sees in the dark, a lit tap is sharp wherever it is.
fn blur_weight(color: vec3<f32>, y: f32) -> f32 {
    let dark = in_the_dark(color);
    if (params.night.w > 0.5) {
        return 0.15 + 0.85 * dark;
    }
    return 0.15 + 0.85 * max(out_of_focus(y), dark);
}

/// The scene here, as the eye sees it: blurred as far as this row is out of
/// focus or this place is dark, and greyed as far as it is dark.
///
/// Where the eye sees in the dark (`night`), light is never blurred: the lens
/// softens only as much as a place is dark, so a lit pool is as sharp out of
/// the band in focus as in it. Without it, the lens blurs what is out of focus
/// whatever its light.
///
/// A disc rather than the bloom's separable sweep, because the blur's width
/// changes from pixel to pixel: a sweep run once across and once down mixes
/// widths into each other, and the sharp parts bleed. It takes as many taps as
/// the disc is wide, up to `LENS_TAPS`, and none where nothing blurs, which is
/// most of what a phone's frame costs.
fn through_the_lens(uv: vec2<f32>) -> vec4<f32> {
    let here = textureSample(source_texture, source_sampler, uv);
    if (params.lens.z < 0.5 && params.night.w < 0.5) {
        return here;
    }
    let dark = in_the_dark(here.rgb);
    var focus_blur = out_of_focus(uv.y) * params.focus.w;
    if (params.night.w > 0.5) {
        focus_blur = focus_blur * dark;
    }
    let radius = max(focus_blur, dark * params.night.y);
    // Where nothing blurs, the scene itself, untouched, and only greyed.
    let luma_here = dot(here.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
    if (radius < 0.0015) {
        return vec4<f32>(mix(here.rgb, vec3<f32>(luma_here), dark * params.night.z), here.a);
    }
    // Square on the screen: widths are measured in screen heights, so across
    // they are scaled by the screen's shape.
    let aspect = params.lens.y / params.lens.x;
    let scale = vec2<f32>(radius / aspect, radius);
    // A tap for every pixel or so across the disc: a narrow blur needs few.
    let across = radius / params.lens.y;
    let taps = clamp(i32(across * 2.0), 8, LENS_TAPS);
    var total = here.rgb;
    var weight = 1.0;
    for (var i = 1; i < taps; i = i + 1) {
        let t = f32(i) / f32(taps);
        let angle = f32(i) * GOLDEN_ANGLE;
        let offset = vec2<f32>(cos(angle), sin(angle)) * sqrt(t) * scale;
        let tap = uv + offset;
        let color = textureSampleLevel(source_texture, source_sampler, tap, 0.0).rgb;
        let w = blur_weight(color, tap.y);
        total = total + color * w;
        weight = weight + w;
    }
    let blurred = mix(here.rgb, total / weight, smoothstep(0.0015, 0.003, radius));
    // The dark takes colour before it takes light.
    let luma = dot(blurred, vec3<f32>(0.2126, 0.7152, 0.0722));
    let seen = mix(blurred, vec3<f32>(luma), dark * params.night.z);
    return vec4<f32>(seen, here.a);
}

@fragment
fn fs_composite(in: VertexOutput) -> @location(0) vec4<f32> {
    let scene = through_the_lens(in.uv);
    let glow = textureSample(glow_texture, source_sampler, in.uv).rgb * params.tone.y;
    var color = grade_world(scene.rgb) + glow * params.blur.z;

    let centered = in.uv * 2.0 - vec2<f32>(1.0);
    let edge = smoothstep(0.25, 1.15, dot(centered, centered));
    color = color * (1.0 - edge * params.grade.w);
    return vec4<f32>(color, scene.a);
}
