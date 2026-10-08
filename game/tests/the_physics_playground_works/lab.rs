//! The probe tool's queries and the debug overlay's readout.

use sindri_platform::Key;

use crate::open;
use crate::support::Playground;

/// Drags the probe from one world point to another and holds it there.
fn probe(playground: &mut Playground, from: [f32; 2], to: [f32; 2]) -> String {
    let start = playground.pixel_of_world(from);
    playground.press_at(start);
    let end = playground.pixel_of_world(to);
    playground.move_to(end);
    playground.release();
    playground.play(2);
    playground.text("readout")
}

#[test]
fn the_probe_casts_through_a_mask_and_counts_an_area() {
    let mut playground = open();
    playground.key(Key::P);
    assert_eq!(playground.text("tool-probe-label"), "RAY");
    let crate_top = playground.at("pile-crate-7");
    let from = [crate_top[0], -9.0];
    let down = [crate_top[0], -14.5];

    let said = probe(&mut playground, from, down);
    assert!(
        said.starts_with("RAY through EVERYTHING: a body at"),
        "{said}"
    );
    assert!(said.contains("normal (0.00, 1.00)"), "{said}");

    // Solids only: the ray goes through the crates to the floor.
    playground.key(Key::L);
    let said = probe(&mut playground, from, down);
    assert!(
        said.starts_with("RAY through SOLIDS: the room at 5.00"),
        "{said}"
    );

    playground.key(Key::P);
    assert_eq!(playground.text("tool-probe-label"), "CIRCLE CAST");
    playground.key(Key::P);
    playground.key(Key::P);
    assert_eq!(playground.text("tool-probe-label"), "AREA");
    playground.key(Key::L);
    let said = probe(
        &mut playground,
        [crate_top[0], -13.0],
        [crate_top[0] + 2.0, -13.0],
    );
    assert!(said.starts_with("AREA through PROPS: "), "{said}");
    let count: f32 = said["AREA through PROPS: ".len()..]
        .split(' ')
        .next()
        .and_then(|n| n.parse().ok())
        .expect("a count");
    assert!(count >= 5.0, "{said}");
}

#[test]
fn debug_draws_the_room_and_counts_what_it_sees() {
    let mut playground = open();
    playground.play(10);
    playground.key(Key::I);
    playground.play(3);
    assert_eq!(playground.text("btn-debug-label"), "DEBUG ON");
    let said = playground.text("readout");
    assert!(said.contains(" in view, "), "{said}");
    assert!(said.ends_with("19 of 19 joints on"), "{said}");
    playground.key(Key::X);
    playground.play(2);
    assert!(
        playground.text("readout").ends_with("0 of 19 joints on"),
        "{}",
        playground.text("readout")
    );
    playground.key(Key::I);
    playground.play(2);
    assert_eq!(playground.text("readout"), "");
}
