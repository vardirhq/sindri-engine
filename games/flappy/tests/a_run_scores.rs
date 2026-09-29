//! The one-button showcase played headlessly: the bird waits, flaps, the world
//! scrolls, and the authored collision path can end and restart a run.

use flappy::Run;
use sindri_platform::Key;

const STEP: f32 = 1.0 / 60.0;

fn step(run: &mut Run) {
    let notes = run.step(STEP);
    assert!(notes.is_empty(), "the game reported: {notes:?}");
}

fn tap(run: &mut Run) {
    run.key(Key::Space, true);
    step(run);
    run.key(Key::Space, false);
}

#[test]
fn the_bird_waits_until_the_first_flap() {
    let mut run = Run::open().expect("the project opens");
    let bird = run.entity("bird").expect("the bird");
    for _ in 0..30 {
        step(&mut run);
    }
    let [x, y] = run.position(bird);
    assert!(
        (x + 3.5).abs() < 0.02 && (y - 0.5).abs() < 0.02,
        "waiting at {x}, {y}"
    );
    assert!(run.board("started") < 1.0);

    tap(&mut run);
    assert!(run.board("started") > 0.0);
    for _ in 0..8 {
        step(&mut run);
    }
    assert!(run.position(bird)[1] > y, "the flap lifts the bird");
}

#[test]
fn the_course_scrolls_after_play_starts() {
    let mut run = Run::open().expect("the project opens");
    let pipe = run.entity("pipe-a-top").expect("the first pipe");
    let before = run.position(pipe)[0];
    tap(&mut run);
    for frame in 0..45 {
        if frame == 22 {
            tap(&mut run);
        } else {
            step(&mut run);
        }
    }
    let after = run.position(pipe)[0];
    assert!(
        after < before - 1.0,
        "the pipe moved from {before} to {after}"
    );
}

#[test]
fn a_failed_run_can_restart() {
    let mut run = Run::open().expect("the project opens");
    let bird = run.entity("bird").expect("the bird");
    tap(&mut run);
    for _ in 0..180 {
        step(&mut run);
        if run.board("dead") > 0.0 {
            break;
        }
    }
    assert!(
        run.board("dead") > 0.0,
        "gravity eventually ends an unattended run"
    );

    tap(&mut run);
    let [x, y] = run.position(bird);
    assert!(run.board("dead") < 1.0 && run.board("started") < 1.0);
    assert!(
        (x + 3.5).abs() < 0.05 && (y - 0.5).abs() < 0.05,
        "restarted at {x}, {y}"
    );
}
