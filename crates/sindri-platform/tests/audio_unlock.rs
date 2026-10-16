//! Which gestures open a browser's audio: every one a person makes, touch too.

use std::convert::Infallible;

use sindri_core::FixedStepConfig;
use sindri_platform::{AudioEvent, EngineHost, FrameContext, Game, InputEvent, Key, MouseButton};

struct Idle;

impl Game for Idle {
    type Error = Infallible;

    fn fixed_update(&mut self, _context: &mut FrameContext<'_>) -> Result<(), Self::Error> {
        Ok(())
    }
}

fn unlocks(event: InputEvent) -> bool {
    let mut host = EngineHost::new(Idle, FixedStepConfig::default()).expect("host");
    host.queue_input(event);
    host.audio().events().contains(&AudioEvent::Unlocked)
}

#[test]
fn a_touch_opens_audio_on_a_phone() {
    // A phone player can hold a finger down for the whole level; browsers
    // grant audio when it lifts, so both ends of a touch unlock.
    assert!(unlocks(InputEvent::TouchStarted {
        id: 1,
        x: 10.0,
        y: 20.0
    }));
    assert!(unlocks(InputEvent::TouchEnded { id: 1 }));
}

#[test]
fn keys_and_clicks_open_audio_and_movement_alone_does_not() {
    assert!(unlocks(InputEvent::KeyPressed(Key::Space)));
    assert!(unlocks(InputEvent::ButtonPressed(MouseButton::Left)));
    assert!(!unlocks(InputEvent::PointerMoved { x: 1.0, y: 2.0 }));
    assert!(!unlocks(InputEvent::TouchMoved {
        id: 1,
        x: 1.0,
        y: 2.0
    }));
}
