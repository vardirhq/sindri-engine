//! The sword: the attack button swings the blade that lands once, in the
//! circle ahead of the way Wick faces, and a gloom goes out on its third blow.

use lamplighter::Run;
use sindri_platform::{InputEvent, Key};

const STEP: f32 = 1.0 / 60.0;

fn step(run: &mut Run, frames: u32) {
    for _ in 0..frames {
        let notes = run.step(STEP);
        assert!(notes.is_empty(), "the game reported: {notes:?}");
    }
}

fn tap(run: &mut Run, key: Key, frames: u32) {
    run.input.apply(InputEvent::KeyPressed(key));
    step(run, frames);
    run.input.apply(InputEvent::KeyReleased(key));
}

fn position(run: &Run, id: &str) -> Option<[f32; 2]> {
    let entity = run.entity(id)?;
    let at = run.world.get(entity)?.transform_3d?.position;
    Some([at[0], at[1]])
}

fn clip(run: &Run) -> String {
    let sprite = run.entity("wick-sprite").expect("Wick has a sprite");
    run.world.get(sprite).expect("it exists").components["sindri.animation.sprite"]["playing"]
        .as_str()
        .expect("a clip")
        .to_owned()
}

/// Walks east a few frames at a time until `target` is within a blade's reach.
fn walk_until_in_reach(run: &mut Run, target: &str) {
    for _ in 0..200 {
        let (wick, gloom) = (position(run, "wick"), position(run, target));
        let (Some(wick), Some(gloom)) = (wick, gloom) else {
            return;
        };
        if gloom[0] - wick[0] < 1.4 {
            return;
        }
        tap(run, Key::D, 3);
    }
    panic!("Wick never got within reach of {target}");
}

/// Walks east until the first gloom blocks the way, then stops.
fn up_to_the_gloom() -> Run {
    let mut run = Run::open().expect("the project opens");
    // The gloom lurk and hold still, so the blade is tried on one that does.
    run.set_board("truce", 1.0);
    step(&mut run, 5);
    tap(&mut run, Key::D, 70);
    step(&mut run, 30);
    run
}

#[test]
fn a_swing_plays_the_slash_and_goes_back_to_standing() {
    let mut run = up_to_the_gloom();
    assert_eq!(clip(&run), "idle-east");
    tap(&mut run, Key::J, 2);
    step(&mut run, 5);
    assert_eq!(clip(&run), "slash-east", "the blade is out");
    step(&mut run, 45);
    assert_eq!(clip(&run), "idle-east", "and back in");
}

#[test]
fn three_blows_put_out_the_gloom_ahead_and_leave_the_one_behind() {
    let mut run = up_to_the_gloom();
    let start = position(&run, "gloom-1").expect("a gloom stands ahead");
    assert!((run.board("gloom_hits")).abs() < f32::EPSILON);

    tap(&mut run, Key::J, 2);
    step(&mut run, 40);
    assert!(
        (run.board("gloom_hits") - 1.0).abs() < f32::EPSILON,
        "one blow lands once"
    );
    let thrown = position(&run, "gloom-1").expect("it is still there");
    assert!(
        thrown[0] > start[0] + 0.3,
        "and throws it back: {start:?} to {thrown:?}"
    );

    for _ in 0..2 {
        // The gloom was thrown away; walk up to it again, as a player would,
        // before the next blow.
        walk_until_in_reach(&mut run, "gloom-1");
        step(&mut run, 20);
        tap(&mut run, Key::J, 2);
        step(&mut run, 40);
    }
    assert!(
        (run.board("gloom_out") - 1.0).abs() < f32::EPSILON,
        "the third puts it out"
    );
    assert!(position(&run, "gloom-1").is_none(), "and it is gone");
    assert!(
        position(&run, "gloom-2").is_some(),
        "the one behind Wick was never touched"
    );
}
