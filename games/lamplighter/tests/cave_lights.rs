//! The cave under the stone roof is lit by its two lanterns.
//!
//! Daylight comes from a sun high over the level that casts shadows, so the
//! roof keeps it out of the cave; inside, what is near a lantern is warm and
//! what is far from both is only the ambient dark. The lanterns cast shadows
//! too, so the stone pillar between them shades the side facing away.

use lamplighter::Run;
use sindri_render::{Light2d, Lighting2d, Viewport};
use sindri_scene::{CameraView, TextureBindings};

fn lighting(run: &Run) -> Lighting2d {
    run.scene
        .extract(
            &run.world,
            Viewport::new(960, 540),
            CameraView::default(),
            &TextureBindings::new(),
        )
        .expect("the level extracts")
        .sprite_environment()
        .lighting
        .clone()
        .expect("the level turns 2D lighting on")
}

/// The same lighting with only the lights `keep` picks.
fn only(lit: &Lighting2d, keep: impl Fn(&Light2d) -> bool) -> Lighting2d {
    Lighting2d {
        ambient: lit.ambient,
        lights: lit
            .lights
            .iter()
            .filter(|light| keep(light))
            .cloned()
            .collect(),
    }
}

fn brightness(color: [f32; 3]) -> f32 {
    color.iter().sum::<f32>() / 3.0
}

#[test]
fn daylight_stays_outside_and_the_lanterns_light_the_cave() {
    let mut run = Run::open().expect("the project opens");
    assert!(run.step(1.0 / 60.0).is_empty());
    let lit = lighting(&run);
    assert!(
        lit.lights
            .iter()
            .filter(|light| light.shadow.is_some())
            .count()
            >= 3,
        "the sun and both cave lanterns cast shadows"
    );
    let outdoors = lit.at([8.0, 3.5]);
    let sun = only(&lit, |light| light.radius > 50.0);
    let daylight_in_the_cave = sun.at([40.0, 3.5]);
    let beside_a_lantern = lit.at([37.5, 6.5]);
    assert!(
        brightness(outdoors) > 0.9,
        "daylight outdoors: {outdoors:?}"
    );
    assert!(
        daylight_in_the_cave.map(f32::to_bits) == sun.ambient.map(f32::to_bits),
        "the roof keeps daylight out: {daylight_in_the_cave:?}"
    );
    assert!(
        beside_a_lantern[0] > beside_a_lantern[2] + 0.2,
        "warm beside a lantern: {beside_a_lantern:?}"
    );
}

#[test]
fn the_pillar_shades_its_far_side_from_a_lantern() {
    let mut run = Run::open().expect("the project opens");
    assert!(run.step(1.0 / 60.0).is_empty());
    let west = only(&lighting(&run), |light| {
        (light.position[0] - 37.5).abs() < 0.1
    });
    assert_eq!(west.lights.len(), 1, "the west lantern");
    // The pillar stands on columns 40 and 41; the west lantern hangs at 37.5.
    let facing = west.at([39.8, 5.0]);
    let behind = west.at([42.3, 5.0]);
    assert!(
        brightness(facing) > brightness(behind) + 0.1,
        "facing {facing:?} against behind {behind:?}"
    );
}
