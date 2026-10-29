//! The square beyond the fields: four lamps round a well, each opening one house and bringing
//! out one villager, with the last lit sending Wick on.

use lamplighter::Run;

const STEP: f32 = 1.0 / 60.0;

fn step(run: &mut Run, frames: u32) {
    for _ in 0..frames {
        let notes = run.step(STEP);
        assert!(notes.is_empty(), "the game reported: {notes:?}");
    }
}

fn place(run: &mut Run, id: &str, at: [f32; 2]) {
    let entity = run.entity(id).unwrap_or_else(|| panic!("{id} exists"));
    let data = run.world.get_mut(entity).expect("it exists");
    let transform = data.transform_3d.as_mut().expect("a place");
    transform.position[0] = at[0];
    transform.position[1] = at[1];
}

fn alpha(run: &Run, id: &str) -> f64 {
    let entity = run.entity(id).unwrap_or_else(|| panic!("{id} exists"));
    run.world.get(entity).expect("it exists").components["sindri.sprite"]["tint"][3]
        .as_f64()
        .expect("an alpha")
}

fn open() -> Run {
    let mut run = Run::open_scene("village.scene").expect("the village opens");
    step(&mut run, 10);
    run
}

#[test]
fn the_village_opens_with_its_people_inside() {
    let run = open();
    for person in ["baker", "child", "mender", "elder"] {
        assert!(
            alpha(&run, &format!("villager-{person}")) < 0.01,
            "{person} is not out yet"
        );
    }
    assert!(run.board("lit") < 0.5, "no lamp is lit");
}

#[test]
fn a_lit_lamp_brings_its_villager_out_and_no_other() {
    let mut run = open();
    // Lamp 2 is the baker's.
    place(&mut run, "wick", [3.5, 3.5]);
    step(&mut run, 120);
    assert!(run.board("lit") >= 1.0, "the lamp is lit");
    assert!(alpha(&run, "villager-baker") > 0.95, "the baker is out");
    assert!(alpha(&run, "villager-child") < 0.01, "the child is not");
}

#[test]
fn lighting_all_four_lamps_finishes_the_village() {
    let mut run = open();
    for at in [[-3.5, 3.5], [3.5, 3.5], [-3.5, -3.5], [3.5, -3.5]] {
        place(&mut run, "wick", at);
        step(&mut run, 30);
    }
    step(&mut run, 90);
    assert!(run.board("lit") >= 4.0, "all four are lit");
    assert!(run.board("won") > 0.5, "the way on opens");
    for person in ["baker", "child", "mender", "elder"] {
        assert!(
            alpha(&run, &format!("villager-{person}")) > 0.95,
            "{person} is out"
        );
    }
}
