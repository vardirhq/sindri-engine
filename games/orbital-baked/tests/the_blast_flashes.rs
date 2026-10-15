//! A mine's blast opens white-hot. A tint can only darken a sprite, so the
//! flash is added through the sprite's colour offset, and it is shed within
//! the blast's first few frames.

use orbital_baked::Run;
use sindri_core::EntityId;

const STEP: f32 = 1.0 / 60.0;

fn step(run: &mut Run) {
    let notes = run.step(STEP);
    assert!(notes.is_empty(), "{notes:#?}");
}

fn heat(run: &Run, blast: EntityId) -> Option<f64> {
    run.world.get(blast).map(|data| {
        data.components["sindri.sprite"]["color_transform"]["offset"][0]
            .as_f64()
            .unwrap_or(0.0)
    })
}

#[test]
fn a_blast_opens_white_hot_and_cools() {
    let mut run = Run::open().expect("the project opens");
    for _ in 0..6 {
        step(&mut run);
    }
    run.click("TitleStart");
    step(&mut run);
    let prefab = run
        .session
        .prefabs()
        .get("prefabs/detonation.prefab")
        .expect("the blast ships")
        .clone();
    let blast = run.world.spawn_prefab(&prefab).expect("spawns").root;

    step(&mut run);
    let opening = heat(&run, blast).expect("the blast is there");
    assert!(opening > 0.4, "the blast opened at {opening}");

    for _ in 0..12 {
        step(&mut run);
    }
    if let Some(later) = heat(&run, blast) {
        assert!(later < 1.0e-6, "the flash was never shed: {later}");
    }
}
