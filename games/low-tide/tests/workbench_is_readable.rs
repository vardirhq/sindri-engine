//! Blueprint selection is a real menu on keys and glass.
mod pilot;

use pilot::{Pilot, construction};
use sindri_platform::Key;

fn words(pilot: &Pilot, id: &str) -> String {
    let entity = pilot.run.entity(id).unwrap();
    pilot.run.world.get(entity).unwrap().components["sindri.ui.text"]["text"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn active(pilot: &Pilot, id: &str) -> bool {
    pilot.run.world.is_active(pilot.run.entity(id).unwrap())
}

#[test]
fn touch_selects_a_blueprint_and_returns_to_play_without_spending_missing_cargo() {
    let mut pilot = Pilot::on_a_phone();
    construction::bench(&mut pilot);
    assert!(!active(&pilot, "use-button"));
    assert!(!active(&pilot, "view-button"));
    assert!(!active(&pilot, "drop-button"));
    pilot.press("plan-engine");
    assert!(words(&pilot, "build-words").contains("power +35%"));
    assert!(words(&pilot, "build-words").contains("Ore 0/2"));
    assert!(active(&pilot, "plan-engine-selected"));
    assert!(!active(&pilot, "plan-extension-selected"));
    assert_eq!(words(&pilot, "build-confirm-label"), "Locked");
    pilot.use_it();
    pilot.wait(0.1);
    assert!(pilot.board("engine_level").abs() < 0.01);
    assert!(pilot.board("crates").abs() < 0.01);
    assert!(words(&pilot, "build-status").contains("Missing cargo"));
    pilot.press("plan-bunk");
    assert!(words(&pilot, "build-words").contains("Fibre 0/2"));
    construction::close(&mut pilot);
    assert!(!active(&pilot, "build-panel"));
    assert!(!active(&pilot, "plan-bunk-selected"));
    assert!(active(&pilot, "use-button"));
}

#[test]
fn keyboard_cycles_plans_and_escape_closes_the_menu() {
    let mut pilot = Pilot::new();
    construction::bench(&mut pilot);
    pilot.tap(Key::Q);
    assert!(words(&pilot, "build-words").contains("A bed"));
    pilot.tap(Key::Q);
    assert!(words(&pilot, "build-words").contains("power +35%"));
    pilot.tap(Key::Escape);
    assert!(!pilot.flag("building"));
    assert!(!active(&pilot, "build-shade"));
}
