//! The charger heats up before it dashes. Its amber body runs towards white as
//! the wind-up peaks, and the heat is shed again once the dash is spent.

use orbital_baked::Run;
use sindri_core::EntityId;

const STEP: f32 = 1.0 / 60.0;

/// The blue of the dart's amber body, in linear light. White-hot is one.
const AMBER_BLUE: f64 = 0.1022;

fn step(run: &mut Run) {
    let notes = run.step(STEP);
    assert!(notes.is_empty(), "{notes:#?}");
}

fn heat(run: &Run, charger: EntityId) -> f64 {
    run.world
        .get(charger)
        .expect("the charger is there")
        .components["sindri.shape"]["fill"][2]
        .as_f64()
        .map_or(0.0, |blue| (blue - AMBER_BLUE) / (1.0 - AMBER_BLUE))
}

#[test]
fn a_charger_glows_through_its_wind_up_and_cools_after() {
    let mut run = Run::open().expect("the project opens");
    for _ in 0..6 {
        step(&mut run);
    }
    run.click("TitleStart");
    step(&mut run);
    // Alone with the player, so nothing else charges or dies in its way.
    let director = run.find("Director").expect("the director exists");
    run.world
        .get_mut(director)
        .expect("director remains")
        .disabled = true;
    let prefab = run
        .session
        .prefabs()
        .get("prefabs/charger.prefab")
        .expect("the charger ships")
        .clone();
    let charger = run.world.spawn_prefab(&prefab).expect("spawns").root;

    let mut hottest: f64 = 0.0;
    let mut cooled = false;
    for _ in 0..(60 * 6) {
        step(&mut run);
        if run.world.get(charger).is_none() {
            break;
        }
        let now = heat(&run, charger);
        if hottest > 0.4 && now < 1.0e-6 {
            cooled = true;
            break;
        }
        hottest = hottest.max(now);
    }
    assert!(hottest > 0.4, "the wind-up never glowed: {hottest}");
    assert!(cooled, "the glow was never shed");
}
