//! Wick holds his lantern, as his art draws it: the floating lantern's picture is
//! not drawn, and its lights ride at the lantern in his hand, on his right, his
//! left when he faces west (the art mirrored), and at the height his art holds it.

use lamplighter::Run;
use sindri_platform::{InputEvent, Key};

const STEP: f32 = 1.0 / 60.0;

fn step(run: &mut Run, frames: u32) {
    for _ in 0..frames {
        let notes = run.step(STEP);
        assert!(notes.is_empty(), "the game reported: {notes:?}");
    }
}

/// Where the lantern is, from Wick's feet, and how opaque its own picture is.
fn lantern(run: &Run) -> ([f32; 3], f64) {
    let entity = run.entity("wick-lantern").expect("the lantern exists");
    let data = run.world.get(entity).expect("it exists");
    let at = data.transform_3d.expect("a place").position;
    let alpha = data.components["sindri.sprite"]["tint"][3]
        .as_f64()
        .expect("a tint");
    (at, alpha)
}

/// Turns Wick to face a way.
fn face(run: &mut Run, key: Key) {
    run.input.apply(InputEvent::KeyPressed(key));
    step(run, 2);
    run.input.apply(InputEvent::KeyReleased(key));
    step(run, 10);
}

#[test]
fn the_lantern_is_in_his_hand_on_the_side_his_art_holds_it() {
    for scene in ["world.scene", "wilds.scene"] {
        let mut run = Run::open_scene(scene).expect("the scene opens");
        step(&mut run, 5);
        // The gloom hold still, so nothing throws Wick off the way he faces.
        run.set_board("truce", 1.0);
        for (key, side) in [(Key::D, 1.0), (Key::W, 1.0), (Key::S, 1.0), (Key::A, -1.0)] {
            face(&mut run, key);
            let (at, alpha) = lantern(&run);
            assert!(
                (at[0] - 0.64 * side).abs() < 1.0e-3,
                "{scene}, {key:?}: at his hand: {at:?}"
            );
            assert!(alpha < 1.0e-3, "{scene}: its own picture is not drawn");
        }
        let (at, _) = lantern(&run);
        let up = if scene == "wilds.scene" { at[2] } else { at[1] };
        assert!(
            up > 0.9 && up < 1.7,
            "{scene}: at the height of his hand: {at:?}"
        );
    }
}
