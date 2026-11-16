//! Wick's lantern in the wilds rides at his chest as the tilted camera sees him,
//! and swings round him: walking away from the camera it is ahead of him, so
//! behind him on screen, and drawn under him; walking towards it, in front.

use lamplighter::Run;
use sindri_platform::{InputEvent, Key};

const STEP: f32 = 1.0 / 60.0;

fn step(run: &mut Run, frames: u32) {
    for _ in 0..frames {
        let notes = run.step(STEP);
        assert!(notes.is_empty(), "the game reported: {notes:?}");
    }
}

/// The lantern's place relative to Wick, and the layer its picture is drawn on.
fn lantern(run: &Run) -> ([f32; 3], i64) {
    let entity = run.entity("wick-lantern").expect("the lantern");
    let data = run.world.get(entity).expect("it exists");
    let place = data.transform_3d.expect("a place").position;
    let layer = data.components["sindri.sprite"]["layer"]
        .as_i64()
        .expect("a whole layer");
    (place, layer)
}

fn walk(run: &mut Run, key: Key) {
    run.input.apply(InputEvent::KeyPressed(key));
    step(run, 60);
    run.input.apply(InputEvent::KeyReleased(key));
    step(run, 2);
}

#[test]
fn the_lantern_rides_at_his_chest_and_goes_behind_him() {
    let mut run = Run::open_scene("wilds.scene").expect("the wilds open");
    step(&mut run, 5);
    let (place, _) = lantern(&run);
    assert!((place[2] - 1.45).abs() < 0.1, "at his chest: {}", place[2]);

    walk(&mut run, Key::W);
    let (place, layer) = lantern(&run);
    assert!(
        place[1] > 0.2,
        "ahead of him, away from the camera: {place:?}"
    );
    assert_eq!(layer, 4, "drawn under him");

    walk(&mut run, Key::S);
    let (place, layer) = lantern(&run);
    assert!(place[1] < -0.2, "towards the camera: {place:?}");
    assert_eq!(layer, 5, "drawn over him");
}
