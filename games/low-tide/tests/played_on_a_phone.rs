//! Low Tide on a phone held upright, played with a thumb on the virtual
//! stick and taps on the buttons: no keys at all.

mod pilot;

use pilot::{Pilot, distance, frames, salvage};
use sindri_platform::Key;

fn active(pilot: &Pilot, id: &str) -> bool {
    let entity = pilot.run.entity(id).expect("the element");
    pilot.run.world.is_active(entity)
}

fn text(pilot: &Pilot, id: &str) -> String {
    let entity = pilot.run.entity(id).expect("the text");
    pilot.run.world.get(entity).expect("it exists").components["sindri.ui.text"]["text"]
        .as_str()
        .unwrap_or_default()
        .to_owned()
}

#[test]
fn the_touch_controls_wait_for_a_finger() {
    let mut pilot = Pilot::new();
    pilot.wait(0.5);
    assert!(!active(&pilot, "use-button"), "no buttons for a keyboard");
    assert!(!active(&pilot, "stick-ring"));
    assert!(
        text(&pilot, "hint").contains("WASD"),
        "and the keys are named"
    );

    // Walking with the keys keeps it that way.
    pilot.run.key(Key::D, true);
    pilot.wait(0.3);
    assert!(!active(&pilot, "use-button"));
}

#[test]
fn a_thumb_walks_the_crew_to_the_helm_and_drives() {
    let mut pilot = Pilot::on_a_phone();
    let start = pilot.crawler_world();

    // The first touch brings the controls up, and the stick draws itself
    // where the thumb landed while it steers.
    pilot.push([0.0, 1.0]);
    pilot.step();
    assert!(pilot.flag("touch"), "a finger was seen");
    assert!(active(&pilot, "use-button"), "the Use button is up");
    assert!(active(&pilot, "stick-ring"), "the stick is drawn");
    assert!(!active(&pilot, "drop-button"), "Drop only shows by a crate");
    pilot.release();
    pilot.step();
    assert!(
        !active(&pilot, "stick-ring"),
        "and goes when the thumb lifts"
    );

    pilot.take_the_helm();
    assert!(
        text(&pilot, "use-label") == "Leave",
        "Use leaves the helm now"
    );
    pilot.set_throttle(1.0);
    pilot.press("use-button");
    assert!(!pilot.flag("at_helm"), "left the helm with the button");

    pilot.wait(4.0);
    let moved = distance(start, pilot.crawler_world());
    assert!(moved > 8.0, "the crawler drove {moved} units");
    assert_eq!(
        text(&pilot, "hint"),
        "Helm to take the wheel",
        "the hint talks about the buttons, not keys"
    );
}

#[test]
fn the_view_button_keeps_the_map_north_up() {
    let mut pilot = Pilot::on_a_phone();
    pilot.take_the_helm();
    pilot.set_throttle(0.8);
    pilot.push([-1.0, 0.0]);
    for _ in 0..frames(4.0) {
        pilot.step();
    }
    pilot.release();
    pilot.press("use-button");
    pilot.wait(2.0);
    let heading = pilot.board("heading");
    assert!(heading > 0.6, "turned to port: {heading}");
    assert!((pilot.board("view_rot") - heading).abs() < 0.1, "deck view");

    pilot.press("view-button");
    pilot.wait(2.5);
    assert!(pilot.flag("prefer_north"));
    assert!(pilot.board("view_rot").abs() < 0.05, "north-up again");
}

#[test]
fn a_wreck_is_salvaged_and_jettisoned_by_touch() {
    let mut pilot = Pilot::on_a_phone();
    salvage::salvage_the_first_wreck(&mut pilot);
    salvage::jettison_it(&mut pilot);
}
