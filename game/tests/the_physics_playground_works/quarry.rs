//! The Voxel Quarry: 3D bodies on a voxel collider, holes dug under them,
//! a 3D ray, and the way there and back.

use sindri_platform::Key;

use crate::open;
use crate::support::{said, times};

#[test]
fn crates_rest_on_voxels_fall_into_dug_holes_and_the_room_waits() {
    let mut playground = open();
    playground.key(Key::K);
    playground.play(90);
    assert!(said("Playground off to the quarry"));
    let resting = playground.at("q-crate-1");
    // On the floor, whose surface is at 7, not through it.
    assert!(resting[1] > 7.3 && resting[1] < 8.7, "{resting:?}");
    assert!(
        playground.text("q-readout").starts_with("3D RAY / "),
        "{}",
        playground.text("q-readout")
    );

    playground.key(Key::Digit2);
    playground.play(90);
    assert!(said("Playground quarry dug under"));
    let fallen = playground.at("q-crate-1");
    assert!(fallen[1] < resting[1] - 0.8, "{resting:?} -> {fallen:?}");

    playground.key(Key::R);
    playground.play(60);
    let home = playground.at("q-crate-1");
    assert!(
        (home[1] - resting[1]).abs() < 0.3,
        "{resting:?} -> {home:?}"
    );

    // Back to the room, which was only switched off.
    let before = times("Playground quarry back to the room");
    playground.key(Key::K);
    playground.play(5);
    assert!(times("Playground quarry back to the room") > before);
    assert_eq!(playground.text("toy-label"), "THE WHOLE ROOM");
}
