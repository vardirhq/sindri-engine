//! Instruments describe real cargo and make discarding it deliberate.
mod pilot;

use pilot::Pilot;
use sindri_platform::Key;
use sindri_scene::TilemapComponent;

fn active(pilot: &Pilot, id: &str) -> bool {
    pilot.run.world.is_active(pilot.run.entity(id).unwrap())
}

fn words(pilot: &Pilot, id: &str) -> String {
    pilot
        .run
        .world
        .get(pilot.run.entity(id).unwrap())
        .unwrap()
        .components["sindri.ui.text"]["text"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn cargo(pilot: &mut Pilot) {
    pilot.wait(0.1);
    let deck = pilot.run.entity("deck").unwrap();
    let mut map = pilot
        .run
        .session
        .components()
        .get::<TilemapComponent>(&pilot.run.world, deck)
        .unwrap()
        .unwrap();
    map.tiles[8 * 11 + 4] = Some(11);
    pilot
        .run
        .world
        .get_mut(deck)
        .unwrap()
        .components
        .get_mut("sindri.tilemap")
        .unwrap()["tiles"] = serde_json::to_value(map.tiles).unwrap();
    pilot.run.session.blackboard_mut().set("crates", 1.0);
    pilot.run.session.blackboard_mut().set("scrap", 1.0);
    pilot.wait(0.1);
}

#[test]
fn cargo_focus_and_confirmation_follow_the_real_crate_on_a_phone() {
    let mut pilot = Pilot::on_a_phone();
    cargo(&mut pilot);
    // A finger brings up the controls without moving the crew.
    pilot.run.session.blackboard_mut().set("touch", 1.0);
    pilot.wait(0.1);
    assert!(active(&pilot, "crate-focus"));
    assert!(active(&pilot, "context-panel"));
    assert!(words(&pilot, "context-title").contains("SCRAP CRATE"));
    assert!(words(&pilot, "hold-readout").contains("1/4"));
    assert!(active(&pilot, "cargo-fill-0"));
    assert!(!active(&pilot, "cargo-fill-1"));
    assert!(
        !active(&pilot, "use-button"),
        "no meaningless Use beside a crate"
    );
    pilot.press("drop-button");
    assert!(pilot.flag("discard_armed"));
    assert!((pilot.board("crates") - 1.0).abs() < 0.01);
    assert!(words(&pilot, "context-detail").contains("confirm"));
    pilot.wait(4.2);
    assert!(!pilot.flag("discard_armed"), "confirmation expires safely");
    assert!((pilot.board("crates") - 1.0).abs() < 0.01);
    pilot.press("drop-button");
    pilot.walk_deck(&[[5.5, -6.5]]);
    pilot.wait(0.1);
    assert!(!pilot.flag("discard_armed"), "leaving the crate cancels");
    assert!(!active(&pilot, "crate-focus"));
    pilot.walk_deck(&[[5.5, -7.5]]);
    pilot.press("drop-button");
    pilot.press("drop-button");
    pilot.wait(0.1);
    assert!(pilot.board("crates").abs() < 0.01);
    assert!(!active(&pilot, "crate-focus"));
    assert!(!active(&pilot, "cargo-fill-0"));
}

#[test]
fn escape_cancels_discard_and_tide_meter_tracks_the_phase() {
    let mut pilot = Pilot::new();
    cargo(&mut pilot);
    pilot.tap(Key::X);
    assert!(pilot.flag("discard_armed"));
    pilot.tap(Key::Escape);
    assert!(!pilot.flag("discard_armed"));
    assert!((pilot.board("crates") - 1.0).abs() < 0.01);
    pilot.run.session.blackboard_mut().set("tide_time", 90.0);
    pilot.wait(0.1);
    let width = |id| {
        pilot
            .run
            .world
            .get(pilot.run.entity(id).unwrap())
            .unwrap()
            .transform_3d
            .unwrap()
            .scale[0]
    };
    let remaining = width("tide-fill") / width("tide-track");
    assert!((remaining - 0.5).abs() < 0.01);
    assert!(words(&pilot, "tide-readout").contains("rises"));
}
