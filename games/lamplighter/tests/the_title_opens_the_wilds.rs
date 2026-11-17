//! The title opens the wilds, and pausing there resumes the wilds.

use lamplighter::Run;
use sindri_platform::{InputEvent, Key};

const STEP: f32 = 1.0 / 60.0;

fn step(run: &mut Run, frames: u32) {
    for _ in 0..frames {
        let notes = run.step(STEP);
        assert!(notes.is_empty(), "the game reported: {notes:?}");
    }
}

fn tap(run: &mut Run, id: &str) {
    run.present().expect("the screen lays out");
    let at = run
        .on_screen(
            run.entity(id)
                .unwrap_or_else(|| panic!("{id} is on the title")),
        )
        .expect("on screen");
    run.input.apply(InputEvent::TouchStarted {
        id: 1,
        x: at[0],
        y: at[1],
    });
    step(run, 1);
    run.input.apply(InputEvent::TouchEnded { id: 1 });
    step(run, 3);
}

fn press(run: &mut Run, key: Key) {
    run.input.apply(InputEvent::KeyPressed(key));
    step(run, 1);
    run.input.apply(InputEvent::KeyReleased(key));
    step(run, 3);
}

#[test]
fn play_opens_the_wilds_and_pause_resumes_them() {
    let mut run = Run::open_title().expect("the title opens");
    step(&mut run, 3);
    tap(&mut run, "play");
    assert_eq!(run.session.scene(), Some("wilds.scene"));
    step(&mut run, 10);
    press(&mut run, Key::Escape);
    assert_eq!(run.session.scene(), Some("pause.scene"));
    press(&mut run, Key::Escape);
    assert_eq!(
        run.session.scene(),
        Some("wilds.scene"),
        "back where he was"
    );
}

#[test]
fn the_title_has_no_dim_fields_button() {
    let run = Run::open_title().expect("the title opens");
    assert!(run.entity("fields").is_none());
}
