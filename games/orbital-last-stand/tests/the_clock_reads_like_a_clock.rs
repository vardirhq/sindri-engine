//! The HUD's run clock reads minutes and whole seconds, `1:05`, written by
//! the script with `padded` rather than by a number slot that could only add
//! decimals.

use orbital_last_stand::Run;

const STEP: f32 = 1.0 / 60.0;

fn step(run: &mut Run) {
    let notes = run.step(STEP);
    assert!(notes.is_empty(), "{notes:#?}");
}

fn clock(run: &Run) -> String {
    let entity = run.find("Clock").expect("the clock exists");
    run.text(entity).expect("the clock carries text")
}

#[test]
fn the_clock_reads_minutes_and_two_digit_seconds() {
    let mut run = Run::open().expect("the project opens");
    for _ in 0..6 {
        step(&mut run);
    }
    run.click("TitleStart");
    step(&mut run);
    assert_eq!(clock(&run), "0:00");

    // Past the first minute, so the seconds wrap and need their zero, on the
    // run's own clock: the director writes `Game.elapsed` from it. Played as
    // the ten-minute run is, taking whatever a level-up offers (which pauses
    // the clock) and keeping the ship alive.
    let mut upgrades = 0;
    for _ in 0..8_000 {
        if run.board("elapsed") >= 62.0 {
            break;
        }
        step(&mut run);
        if run.board("run_state") == 2.0 {
            step(&mut run);
            let offers = run.active_named("upgrade");
            run.click(&offers[upgrades % offers.len()]);
            upgrades += 1;
        }
        run.set_board("hp", run.board("max_hp"));
        let elapsed = run.board("elapsed");
        let whole = elapsed.max(0.0).floor();
        let expected = format!("{}:{:02}", (whole / 60.0).floor(), whole % 60.0);
        assert_eq!(clock(&run), expected, "at {elapsed} seconds");
    }
    assert_eq!(clock(&run), "1:02");
}
