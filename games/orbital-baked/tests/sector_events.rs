//! The original's sector events: the first at 0:42, each bending the run for
//! 13 to 18 seconds under a banner, then a quiet spell before the next.

use orbital_baked::Run;

const STEP: f32 = 1.0 / 60.0;

fn step(run: &mut Run) {
    let notes = run.step(STEP);
    assert!(notes.is_empty(), "{notes:#?}");
    // A chooser holds the run still; take whatever it offers.
    let offers = run.active_named("upgrade");
    if let Some(first) = offers.first() {
        run.click(first);
    }
    run.set_board("hp", 100_000.0);
}

fn run_until(run: &mut Run, seconds: f32) {
    while run.board("elapsed") < seconds {
        step(run);
    }
}

#[test]
fn the_first_event_arrives_at_forty_two_seconds_and_ends() {
    let mut run = Run::open().expect("the project opens");
    for _ in 0..6 {
        step(&mut run);
    }
    run.click("TitleStart");
    step(&mut run);

    run_until(&mut run, 41.5);
    assert_eq!(run.board("event"), 0.0, "nothing happens before 0:42");

    run_until(&mut run, 42.2);
    let id = run.board("event");
    assert!(
        (1.0..=6.0).contains(&id),
        "an event began at 0:42 (got {id})"
    );
    assert!(run.board("event_banner") > 0.0, "and its banner is up");
    let panel = run.find("EventPanel").expect("the HUD has an event banner");
    assert!(
        !run.world.get(panel).expect("the banner").disabled,
        "the banner is showing"
    );

    // The longest event is 18 seconds.
    run_until(&mut run, 42.0 + 18.5);
    assert_eq!(run.board("event"), 0.0, "the event ended");
    for key in [
        "event_fire",
        "event_enemy_speed",
        "event_magnet",
        "event_spawn",
        "event_xp",
        "event_hurt",
        "event_score",
    ] {
        assert_eq!(run.board(key), 1.0, "{key} is back to normal");
    }
}

#[test]
fn a_rush_has_no_events() {
    let mut run = Run::open().expect("the project opens");
    for _ in 0..6 {
        step(&mut run);
    }
    run.click("TitleRush");
    step(&mut run);
    run_until(&mut run, 45.0);
    assert_eq!(run.board("event"), 0.0, "a rush is bosses only");
}
