//! A shot that lands shows what it did: world text over the hit, which rises,
//! fades and goes.

use orbital_baked::Run;
use sindri_core::EntityId;

const STEP: f32 = 1.0 / 60.0;

fn first_number(run: &Run) -> Option<EntityId> {
    run.world
        .entities()
        .find(|(_, data)| data.components.contains_key("sindri.text"))
        .map(|(entity, _)| entity)
}

#[test]
fn a_landed_shot_floats_its_damage_and_clears_it() {
    let mut run = Run::open().expect("the project opens");
    for _ in 0..6 {
        run.step(STEP);
    }
    run.click("TitleStart");
    let mut seen = None;
    for _ in 0..(60 * 30) {
        run.step(STEP);
        if run.count("damage_number") > 0 {
            seen = first_number(&run);
            break;
        }
    }
    let number = seen.expect("a shot landed within half a minute");
    let text = &run.world.get(number).expect("showing").components["sindri.text"];
    assert!(
        text["text"]
            .as_str()
            .is_some_and(|words| words.contains("{.0}")),
        "{text}"
    );
    assert!(
        text["values"][0].as_f64().is_some_and(|value| value > 0.0),
        "{text}"
    );

    // Every number goes within its life, so a long fight does not pile up
    // text, however many new ones arrive meanwhile.
    for _ in 0..60 {
        run.step(STEP);
    }
    assert!(run.world.get(number).is_none(), "the number cleared");
}
