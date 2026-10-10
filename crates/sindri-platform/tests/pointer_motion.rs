//! Capture feedback and relative motion without a window or GPU.

use std::convert::Infallible;
use std::time::Duration;

use sindri_core::FixedStepConfig;
use sindri_platform::{EngineHost, FrameContext, Game, InputEvent, InputState, MouseButton};

#[track_caller]
fn assert_motion(actual: [f32; 2], expected: [f32; 2]) {
    for (actual, expected) in actual.into_iter().zip(expected) {
        assert!((actual - expected).abs() < f32::EPSILON);
    }
}

fn captured_input() -> InputState {
    let mut input = InputState::default();
    input.apply(InputEvent::PointerMoved { x: 20.0, y: 30.0 });
    input.apply(InputEvent::PointerLockChanged(true));
    assert!(input.pointer_locked());
    input
}

#[test]
fn relative_motion_needs_actual_capture() {
    let mut input = InputState::default();
    input.apply(InputEvent::PointerMotion { x: 5.0, y: -7.0 });
    assert_motion(input.pointer_delta(), [0.0, 0.0]);
    assert!(!input.pointer_locked());

    input.apply(InputEvent::PointerLockChanged(true));
    input.apply(InputEvent::PointerMotion { x: 5.0, y: -7.0 });
    input.apply(InputEvent::PointerMotion { x: -2.0, y: 3.0 });
    assert_motion(input.pointer_delta(), [3.0, -4.0]);
    assert!(
        input.pointer().is_none(),
        "relative motion invents no screen position"
    );
}

#[test]
fn captured_motion_ignores_cursor_warps_and_leave_events() {
    let mut input = captured_input();
    input.apply(InputEvent::ButtonPressed(MouseButton::Right));
    input.apply(InputEvent::PointerMotion { x: 8.0, y: 9.0 });
    input.apply(InputEvent::PointerMoved { x: 500.0, y: 400.0 });
    input.apply(InputEvent::PointerLeft);
    assert_motion(input.pointer_delta(), [8.0, 9.0]);
    assert_motion(
        input.pointer().expect("the original screen position"),
        [20.0, 30.0],
    );
    assert_motion(
        input
            .presses()
            .primary()
            .expect("the right press")
            .position(),
        [20.0, 30.0],
    );
    assert_eq!(input.presses().live(), 1);
}

#[test]
fn capture_transitions_discard_pending_motion_and_rebase_absolute_input() {
    let mut input = InputState::default();
    input.apply(InputEvent::PointerMoved { x: 1.0, y: 2.0 });
    input.apply(InputEvent::PointerMoved { x: 11.0, y: 12.0 });
    input.apply(InputEvent::PointerLockChanged(true));
    assert_motion(input.pointer_delta(), [0.0, 0.0]);
    input.apply(InputEvent::PointerMotion { x: 8.0, y: 9.0 });
    input.apply(InputEvent::PointerLockChanged(false));
    assert_motion(input.pointer_delta(), [0.0, 0.0]);
    input.apply(InputEvent::PointerMotion { x: 100.0, y: 100.0 });
    input.apply(InputEvent::PointerMoved { x: 300.0, y: 400.0 });
    assert_motion(input.pointer_delta(), [0.0, 0.0]);
    input.apply(InputEvent::PointerMoved { x: 303.0, y: 396.0 });
    assert_motion(input.pointer_delta(), [3.0, -4.0]);
}

#[test]
fn duplicate_feedback_and_fixed_step_reset_preserve_capture() {
    let mut input = captured_input();
    input.apply(InputEvent::PointerMotion { x: 8.0, y: 9.0 });
    input.apply(InputEvent::PointerLockChanged(true));
    assert_motion(input.pointer_delta(), [8.0, 9.0]);
    input.begin_frame(Duration::from_millis(10));
    assert_motion(input.pointer_delta(), [0.0, 0.0]);
    assert!(input.pointer_locked());
}

#[test]
fn losing_focus_releases_capture_and_drops_stale_motion() {
    let mut input = captured_input();
    input.apply(InputEvent::ButtonPressed(MouseButton::Right));
    input.apply(InputEvent::PointerMotion { x: 8.0, y: 9.0 });
    input.apply(InputEvent::FocusChanged(false));
    input.apply(InputEvent::PointerLockChanged(true));
    input.apply(InputEvent::PointerMotion { x: 100.0, y: 100.0 });
    assert!(!input.pointer_locked());
    assert!(!input.button_down(MouseButton::Right));
    assert_eq!(input.presses().live(), 0);
    assert!(input.pointer().is_none());
    assert_motion(input.pointer_delta(), [0.0, 0.0]);
    input.apply(InputEvent::FocusChanged(true));
    assert!(!input.pointer_locked(), "focus does not request recapture");
    input.apply(InputEvent::PointerMoved { x: 300.0, y: 400.0 });
    assert_motion(input.pointer_delta(), [0.0, 0.0]);
}

#[test]
fn malformed_and_overflowing_relative_motion_is_rejected_atomically() {
    let mut input = captured_input();
    input.apply(InputEvent::PointerMotion { x: 8.0, y: 9.0 });
    for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        input.apply(InputEvent::PointerMotion {
            x: invalid,
            y: 100.0,
        });
        input.apply(InputEvent::PointerMotion {
            x: 100.0,
            y: invalid,
        });
        assert_motion(input.pointer_delta(), [8.0, 9.0]);
    }
    input.begin_frame(Duration::ZERO);
    input.apply(InputEvent::PointerMotion {
        x: f32::MAX,
        y: 1.0,
    });
    input.apply(InputEvent::PointerMotion {
        x: f32::MAX,
        y: 100.0,
    });
    assert_motion(input.pointer_delta(), [f32::MAX, 1.0]);
}

#[test]
fn malformed_absolute_input_does_not_poison_position_or_ui_presses() {
    let mut input = InputState::default();
    input.apply(InputEvent::PointerMoved { x: 20.0, y: 30.0 });
    input.apply(InputEvent::ButtonPressed(MouseButton::Right));
    input.apply(InputEvent::PointerMoved {
        x: f32::NAN,
        y: 80.0,
    });
    input.apply(InputEvent::PointerMoved { x: 23.0, y: 26.0 });
    assert_motion(input.pointer_delta(), [3.0, -4.0]);
    assert_motion(
        input.presses().primary().expect("right press").position(),
        [23.0, 26.0],
    );
}

#[derive(Default)]
struct MotionRecorder {
    steps: Vec<([f32; 2], bool)>,
}

impl Game for MotionRecorder {
    type Error = Infallible;

    fn fixed_update(&mut self, context: &mut FrameContext<'_>) -> Result<(), Self::Error> {
        self.steps.push((
            context.input.pointer_delta(),
            context.input.pointer_locked(),
        ));
        Ok(())
    }
}

#[test]
fn relative_motion_survives_no_step_and_is_spent_once_during_catchup() {
    let config = FixedStepConfig {
        step: Duration::from_millis(10),
        max_frame_delta: Duration::from_millis(100),
        max_steps_per_frame: 8,
    };
    let mut host = EngineHost::new(MotionRecorder::default(), config).expect("host");
    host.start().expect("start");
    host.queue_input(InputEvent::PointerLockChanged(true));
    host.queue_input(InputEvent::PointerMotion { x: 5.0, y: -7.0 });
    host.advance(Duration::from_millis(5))
        .expect("no fixed step");
    assert!(host.game().steps.is_empty());
    host.queue_input(InputEvent::PointerMotion { x: -2.0, y: 3.0 });
    host.advance(Duration::from_millis(25))
        .expect("three fixed steps");
    let steps = &host.game().steps;
    assert_eq!(steps.len(), 3);
    assert_motion(steps[0].0, [3.0, -4.0]);
    assert!(
        steps.iter().all(|&(_, locked)| locked),
        "capture persists through catchup"
    );
    assert_motion(steps[1].0, [0.0, 0.0]);
    assert_motion(steps[2].0, [0.0, 0.0]);
}
