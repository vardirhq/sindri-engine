//! The Physics Playground, played: the room's tools, its global buttons and
//! every contraption, observed through what a person would see move.

mod support;

use sindri_platform::Key;
use support::{Playground, said};

const WIDE: (f32, f32) = (1280.0, 720.0);

fn open() -> Playground {
    Playground::open(WIDE.0, WIDE.1)
}

#[test]
fn the_room_opens_on_the_whole_room_with_every_control_on_screen() {
    let playground = open();
    assert_eq!(playground.text("toy-label"), "THE WHOLE ROOM");
    assert!(
        playground.text("stats").contains("bodies"),
        "{}",
        playground.text("stats")
    );
    for name in [
        "btn-drop",
        "btn-balls",
        "btn-gravity",
        "btn-debug",
        "btn-reset",
        "tool-grab",
        "tool-blast",
        "tool-spawn",
        "tool-probe",
        "toy-prev",
        "toy-next",
        "act-1",
        "act-4",
    ] {
        assert!(
            playground
                .session
                .screen_ui()
                .rect(playground.id(name))
                .is_some(),
            "{name}"
        );
    }
}

#[test]
fn a_hundred_balls_pour_in_and_settle_without_a_failure() {
    let mut playground = open();
    let before = playground.tagged("loose");
    playground.click("btn-balls");
    playground.play(120);
    assert!(
        playground.tagged("loose") >= before + 100,
        "{}",
        playground.tagged("loose")
    );
    playground.play(120);
}

#[test]
fn grabbing_a_crate_lifts_it_and_letting_go_throws_it() {
    let mut playground = open();
    let start = playground.at("pile-crate-2");
    let pixel = playground.pixel_of_world(start);
    playground.press_at(pixel);
    for step in 1..=20u8 {
        let target = [
            start[0] + f32::from(step) * 0.2,
            start[1] + f32::from(step) * 0.35,
        ];
        let pixel = playground.pixel_of_world(target);
        playground.move_to(pixel);
    }
    let lifted = playground.at("pile-crate-2");
    assert!(lifted[1] > start[1] + 4.0, "{start:?} -> {lifted:?}");
    assert!(said("Playground grabbed"));
    playground.release();
    playground.play(10);
    let thrown = playground.at("pile-crate-2");
    assert!(
        thrown[0] > lifted[0] + 0.5,
        "kept its speed: {lifted:?} -> {thrown:?}"
    );
}

#[test]
fn a_blast_scatters_the_pile() {
    let mut playground = open();
    playground.key(Key::B);
    let ball = playground.at("pile-ball-8");
    let pixel = playground.pixel_of_world([ball[0] - 0.6, ball[1] - 0.4]);
    playground.press_at(pixel);
    playground.release();
    playground.play(15);
    let after = playground.at("pile-ball-8");
    assert!(
        after.iter().zip(ball).any(|(a, b)| (a - b).abs() > 1.0),
        "{ball:?} -> {after:?}"
    );
    assert!(said("Playground blast"));
}

#[test]
fn spawning_drops_the_chosen_thing_where_the_pointer_is() {
    let mut playground = open();
    playground.click("tool-spawn");
    playground.click("tool-spawn");
    assert_eq!(playground.text("tool-spawn-label"), "SPAWN CRATE");
    let before = playground.tagged("loose");
    let pixel = playground.pixel_of_world([0.0, 6.0]);
    playground.press_at(pixel);
    playground.release();
    assert_eq!(playground.tagged("loose"), before + 1);
}

#[test]
fn gravity_cycles_through_five_worlds_and_upside_down_lifts_the_pile() {
    let mut playground = open();
    let names = ["MOON", "ZERO-G", "UPSIDE DOWN"];
    for name in names {
        playground.click("btn-gravity");
        assert_eq!(
            playground.text("btn-gravity-label"),
            format!("GRAVITY: {name}")
        );
    }
    let start = playground.at("pile-ball-3");
    playground.play(90);
    assert!(playground.at("pile-ball-3")[1] > start[1] + 3.0);
    playground.click("btn-gravity");
    playground.click("btn-gravity");
    assert_eq!(playground.text("btn-gravity-label"), "GRAVITY: EARTH");
}

#[test]
fn reset_puts_the_room_back() {
    let mut playground = open();
    let start = playground.at("pile-crate-0");
    playground.key(Key::B);
    let pixel = playground.pixel_of_world([start[0], start[1] - 0.3]);
    playground.press_at(pixel);
    playground.release();
    playground.play(30);
    let moved = playground.at("pile-crate-0");
    assert!((moved[0] - start[0]).abs() + (moved[1] - start[1]).abs() > 0.3);
    playground.click("btn-reset");
    playground.play(2);
    let back = playground.at("pile-crate-0");
    assert!(
        (back[0] - start[0]).abs() < 0.05 && (back[1] - start[1]).abs() < 0.05,
        "{back:?}"
    );
}
