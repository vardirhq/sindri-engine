//! The houses: a door opens with its lamp, pressing interact at it goes in, and the way out leads
//! back to a village that is as it was left.

use lamplighter::Run;
use sindri_platform::{InputEvent, Key};

const STEP: f32 = 1.0 / 60.0;

fn step(run: &mut Run, frames: u32) {
    for _ in 0..frames {
        let notes = run.step(STEP);
        assert!(notes.is_empty(), "the game reported: {notes:?}");
    }
}

fn tap(run: &mut Run, key: Key) {
    run.input.apply(InputEvent::KeyPressed(key));
    step(run, 3);
    run.input.apply(InputEvent::KeyReleased(key));
    step(run, 3);
}

fn place(run: &mut Run, id: &str, at: [f32; 2]) {
    let entity = run.entity(id).unwrap_or_else(|| panic!("{id} exists"));
    let data = run.world.get_mut(entity).expect("it exists");
    let transform = data.transform_3d.as_mut().expect("a place");
    transform.position[0] = at[0];
    transform.position[1] = at[1];
}

fn playing(run: &Run, id: &str) -> bool {
    let entity = run.entity(id).unwrap_or_else(|| panic!("{id} exists"));
    run.world.is_active(entity)
}

fn open() -> Run {
    let mut run = Run::open_scene("village.scene").expect("the village opens");
    step(&mut run, 10);
    run
}

#[test]
fn a_door_stays_shut_until_its_lamp_is_lit() {
    let mut run = open();
    // The bakery's door, with no lamp lit.
    place(&mut run, "wick", [12.1, 2.5]);
    step(&mut run, 10);
    tap(&mut run, Key::E);
    assert!(playing(&run, "door-bakery"), "still in the village");
}

#[test]
fn a_lit_house_can_be_entered_and_left_and_the_village_is_as_it_was() {
    let mut run = open();
    place(&mut run, "wick", [3.5, 3.5]);
    step(&mut run, 120);
    assert!(run.board("lit") >= 1.0, "the baker's lamp is lit");
    place(&mut run, "wick", [12.1, 2.5]);
    step(&mut run, 10);
    tap(&mut run, Key::E);
    assert!(
        playing(&run, "interior-bakery.scene/door-out"),
        "inside the bakery"
    );
    assert!(!playing(&run, "door-bakery"), "the village is switched off");
    step(&mut run, 30);
    assert!(
        run.board("lit") >= 1.0,
        "the lamps lit are still counted inside"
    );
    assert!(run.board("lit") >= 1.0, "the lamps lit are still counted");
    place(&mut run, "interior-bakery.scene/wick", [0.0, -4.9]);
    step(&mut run, 10);
    tap(&mut run, Key::E);
    assert!(playing(&run, "door-bakery"), "back in the village");
    assert!(run.board("lit") >= 1.0, "and still counted");
}
