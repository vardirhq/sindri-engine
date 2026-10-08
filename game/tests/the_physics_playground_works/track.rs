//! The test track's robot and the domino run's red button.

use sindri_platform::Key;

use crate::support::{Playground, said};
use crate::{hold, open, select};

/// Steps until `done` holds, or panics naming what never happened.
fn until(
    playground: &mut Playground,
    steps: usize,
    what: &str,
    done: impl Fn(&Playground) -> bool,
) {
    for _ in 0..steps {
        if done(playground) {
            return;
        }
        playground.step();
    }
    panic!("{what} never happened");
}

#[test]
fn the_robot_kicks_climbs_rides_the_lift_and_drops_through_a_plank() {
    let mut playground = open();
    select(&mut playground, "TEST TRACK");
    let start = playground.at("robot");
    let crate_start = playground.at("track-crate-0");

    // Turn round and kick the crates behind it off the deck.
    hold(&mut playground, Key::A, 4);
    playground.key(Key::F);
    playground.play(30);
    assert!(said("Playground robot kick"));
    let kicked = playground.at("track-crate-0");
    assert!(
        (kicked[0] - crate_start[0]).abs() > 1.0 || kicked[1] < crate_start[1] - 1.0,
        "{crate_start:?} -> {kicked:?}"
    );

    // Up three steps without a jump, down the slope, onto the lift.
    playground
        .input
        .apply(sindri_platform::InputEvent::KeyPressed(Key::D));
    let mut highest = start[1];
    until(&mut playground, 400, "the robot reaching the lift", |p| {
        p.at("robot")[0] > 8.0
    });
    for _ in 0..20 {
        playground.step();
        highest = highest.max(playground.at("robot")[1]);
    }
    playground
        .input
        .apply(sindri_platform::InputEvent::KeyReleased(Key::D));
    assert!(highest > start[1] - 2.0, "never on the steps");

    // The lift carries it up to the planks.
    until(
        &mut playground,
        300,
        "the lift carrying the robot up",
        |p| p.at("robot")[1] > -1.8,
    );
    let top = playground.at("robot");
    assert!((top[0] - 8.5).abs() < 1.0, "rode the lift: {top:?}");

    // Off the lift onto the one-way plank, then down through it.
    hold(&mut playground, Key::A, 40);
    playground.play(10);
    let on_plank = playground.at("robot");
    assert!(
        on_plank[0] < 7.4 && on_plank[1] > -2.0,
        "on the plank: {on_plank:?}"
    );
    playground.key(Key::S);
    playground.play(40);
    assert!(said("Playground robot drops through"));
    assert!(playground.at("robot")[1] < on_plank[1] - 1.5);

    playground.key(Key::R);
    playground.play(3);
    let home = playground.at("robot");
    assert!((home[0] - start[0]).abs() < 0.05 && (home[1] - start[1]).abs() < 0.2);
}

#[test]
fn the_domino_run_presses_the_red_button_and_the_button_drops_everything() {
    let mut playground = open();
    select(&mut playground, "DOMINO RUN");
    // Disarmed, a press is only confetti.
    playground.key(Key::Digit4);
    playground.key(Key::Digit3);
    until(&mut playground, 120, "a disarmed press", |_| {
        said("Playground red button pressed while disarmed")
    });
    assert!(!said("Playground DROP EVERYTHING"));
    playground.key(Key::Digit4);
    playground.play(90);

    playground.key(Key::Digit1);
    until(
        &mut playground,
        900,
        "the last domino pressing the button",
        |_| said("Playground red button") && said("Playground DROP EVERYTHING"),
    );
    assert_eq!(playground.text("btn-drop-label"), "PUT IT BACK");
}
