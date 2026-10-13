//! Cleanup runs before toys consume the collision snapshot from that step's solve.

use std::time::Duration;

use sindri_platform::{InputEvent, Key};

use crate::{open, support::STEP};

#[test]
fn resetting_the_room_after_a_ball_contacts_glass_keeps_scripts_running() {
    let mut playground = open();
    let pane = playground.id("pane-0");
    let before = playground.tagged("loose");
    playground.key(Key::N);
    let position = playground.at("pane-0");
    let pixel = playground.pixel_of_world(position);
    playground.press_at(pixel);
    assert_eq!(playground.tagged("loose"), before + 1);
    let ball = playground
        .world
        .entities()
        .find(|(_, data)| data.name.as_deref() == Some("Ball"))
        .map(|(entity, _)| entity)
        .expect("the pointer spawned a ball");
    // The next fixed step solves the new contact first, then the director's
    // RESET deletes the ball before GlassPane consumes that contact.
    playground.input.apply(InputEvent::KeyPressed(Key::R));
    let report = playground
        .session
        .step(
            &mut playground.world,
            &playground.input,
            (playground.size[0], playground.size[1]),
            STEP,
        )
        .unwrap();
    assert!(
        playground.session.physics().events().iter().any(|event| {
            (event.first == pane && event.second == ball)
                || (event.first == ball && event.second == pane)
        }),
        "the solve reports the pane/ball contact in the cleanup step"
    );
    assert!(
        playground.world.get(ball).is_none(),
        "RESET removed the contact partner"
    );
    assert!(
        report.scripts.failures.is_empty(),
        "{:?}",
        report.scripts.failures
    );
    assert!(report.problems.is_empty(), "{:?}", report.problems);
    playground.input.apply(InputEvent::KeyReleased(Key::R));
    playground.input.begin_frame(Duration::from_secs_f32(STEP));
    playground.release();
    assert_eq!(playground.tagged("loose"), before);
    playground.play(30);
}
