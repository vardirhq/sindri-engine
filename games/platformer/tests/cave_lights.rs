//! The cave under the stone roof is lit by its two lanterns.
//!
//! Wick carries a light through a dark world, and the cave lanterns cast
//! shadows so the stone pillar shades the far side.

use platformer::Run;
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
        sight: None,
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
fn wick_carries_a_shadow_casting_light_into_a_dark_world() {
    let mut run = Run::open().expect("the project opens");
    assert!(run.step(1.0 / 60.0).is_empty());
    let lit = lighting(&run);
    let held = lit
        .lights
        .iter()
        .find(|light| (light.radius - 5.5).abs() < 0.01)
        .expect("Wick carries a lantern");
    let shadow = held.shadow.as_ref().expect("Wick's lantern casts shadows");
    assert!((shadow.source_radius - 0.4).abs() < 0.001);
    assert_eq!(
        shadow.map.len(),
        sindri_render::SHADOW_MAP_LEN_2D * sindri_render::SHADOW_SAMPLES_2D,
        "the carried lantern uses all area samples"
    );
    let hero = run.entity("hero").expect("Wick exists");
    let start = run.position(hero);
    assert!((held.position[0] - start[0]).abs() < 0.3);
    let nearby = lit.at([held.position[0], held.position[1] + 0.2]);
    let distant = lit.at([25.0, 5.0]);
    assert!(
        brightness(nearby) > brightness(distant) + 0.2,
        "near {nearby:?}, far {distant:?}"
    );
    assert!(
        brightness(distant) < 0.05,
        "the unlit road is dark: {distant:?}"
    );
    run.key(sindri_platform::Key::ArrowRight, true);
    for _ in 0..60 {
        assert!(run.step(1.0 / 60.0).is_empty());
    }
    let moved = run.position(hero);
    let moved_light = lighting(&run);
    let held = moved_light
        .lights
        .iter()
        .find(|light| (light.radius - 5.5).abs() < 0.01)
        .expect("the lantern is still active");
    assert!(moved[0] > start[0] + 1.0);
    assert!(
        (held.position[0] - moved[0]).abs() < 0.3,
        "the light follows Wick"
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
