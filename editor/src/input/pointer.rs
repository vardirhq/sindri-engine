//! Game viewport pointer routing.

use eframe::egui;
use sindri_platform::InputEvent;

use super::{BUTTONS, EditorInput, SCROLL_LINE_PIXELS};

impl EditorInput {
    /// This frame's pointer and fingers, in the Game view's own pixels.
    pub(super) fn update_pointer(&mut self, context: &egui::Context, view: Option<egui::Rect>) {
        let Some(view) = view else {
            self.state.apply(InputEvent::PointerLeft);
            return;
        };
        // Positions arrive from egui in points, relative to the window. A
        // script reads them relative to the viewport, which is what every
        // other host reports and what makes a position mean the same thing in
        // editor Play as in the real build.
        //
        // Scaled to physical pixels, because that is the viewport's own space:
        // the Game view is rendered at `physical_viewport_dimension`, and a
        // screen element's hit rect comes from dividing a position by that. In
        // points, a position is the scale factor too small, so on a display
        // reporting anything but 1.0 every press in Play lands above and left
        // of where the person actually pressed -- the same fault that stopped a
        // phone from reaching a button, hidden here behind the 1.0 that an
        // ordinary desktop and every test both report.
        let scale = context.pixels_per_point();
        let local = |position: egui::Pos2| {
            (
                (position.x - view.min.x) * scale,
                (position.y - view.min.y) * scale,
            )
        };

        context.input(|input| {
            // Raw native counts arrive independently of absolute cursor warps.
            // InputState ignores them unless actual capture has been reported.
            for event in &input.events {
                if let egui::Event::MouseMoved(delta) = event {
                    self.state.apply(InputEvent::PointerMotion {
                        x: delta.x,
                        y: delta.y,
                    });
                }
            }
            // Wheel movement over the Game view, in the physical pixels every
            // other position here is in. A notch is a line, which the desktop
            // host counts as fifty pixels, so a scroll list moves as far in
            // Play as it does in the build.
            if input.pointer.latest_pos().is_some_and(|p| view.contains(p)) {
                for event in &input.events {
                    if let egui::Event::MouseWheel { unit, delta, .. } = event {
                        let per = match unit {
                            egui::MouseWheelUnit::Point => scale,
                            egui::MouseWheelUnit::Line => SCROLL_LINE_PIXELS * scale,
                            egui::MouseWheelUnit::Page => view.height() * scale,
                        };
                        self.state.apply(InputEvent::Scrolled {
                            x: delta.x * per,
                            y: delta.y * per,
                        });
                    }
                }
            }
            // A pointer over the inspector is not over the game. Reporting it
            // anyway would let a script aim at a panel, and clamping it to the
            // edge would be worse: the game would think the person is pointing
            // at somewhere they are not.
            match input.pointer.latest_pos() {
                Some(position) if view.contains(position) => {
                    let (x, y) = local(position);
                    self.state.apply(InputEvent::PointerMoved { x, y });
                }
                _ => self.state.apply(InputEvent::PointerLeft),
            }

            for event in &input.events {
                match event {
                    egui::Event::PointerButton {
                        pos,
                        button,
                        pressed,
                        ..
                    } => {
                        let Some((_, button)) = BUTTONS.iter().find(|(known, _)| known == button)
                        else {
                            continue;
                        };
                        // A press that began outside the view is not the game's,
                        // but the release that ends it is — otherwise a button
                        // pressed on a panel and released over the game would
                        // leave the game holding a button nobody pressed, and
                        // one pressed on the game and released off it would
                        // stay down for ever.
                        // While captured, absolute cursor warps can leave the
                        // viewport; the game still owns those button presses.
                        if *pressed && !self.state.pointer_locked() && !view.contains(*pos) {
                            continue;
                        }
                        self.state.apply(if *pressed {
                            InputEvent::ButtonPressed(*button)
                        } else {
                            InputEvent::ButtonReleased(*button)
                        });
                    }
                    egui::Event::Touch { id, phase, pos, .. } => {
                        let (x, y) = local(*pos);
                        let id = id.0;
                        self.state.apply(match phase {
                            egui::TouchPhase::Start if view.contains(*pos) => {
                                InputEvent::TouchStarted { id, x, y }
                            }
                            // A finger that started outside the view is not the
                            // game's, and the platform ignores a move for one
                            // that never started — so this needs no second
                            // check to stay consistent.
                            egui::TouchPhase::Start | egui::TouchPhase::Move => {
                                InputEvent::TouchMoved { id, x, y }
                            }
                            egui::TouchPhase::End | egui::TouchPhase::Cancel => {
                                InputEvent::TouchEnded { id }
                            }
                        });
                    }
                    _ => {}
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn view() -> egui::Rect {
        egui::Rect::from_min_size(egui::pos2(100.0, 40.0), egui::vec2(320.0, 200.0))
    }

    fn frame(context: &egui::Context, input: &mut EditorInput, events: Vec<egui::Event>) {
        context.begin_pass(egui::RawInput {
            events,
            ..Default::default()
        });
        input.update(context, true, Some(view()));
        context.end_pass().textures_delta.clear();
    }

    fn delta(input: &EditorInput, expected: [f32; 2]) {
        for (actual, expected) in input.state().pointer_delta().into_iter().zip(expected) {
            assert!((actual - expected).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn captured_raw_counts_ignore_warps_and_accumulate_until_a_step() {
        let context = egui::Context::default();
        context.set_pixels_per_point(2.0);
        let mut input = EditorInput::default();
        frame(
            &context,
            &mut input,
            vec![egui::Event::PointerMoved(egui::pos2(150.0, 90.0))],
        );
        input.pointer_lock_changed(true);
        frame(
            &context,
            &mut input,
            vec![
                egui::Event::MouseMoved(egui::vec2(1500.0, 30.0)),
                egui::Event::PointerMoved(egui::pos2(0.0, 0.0)),
            ],
        );
        frame(
            &context,
            &mut input,
            vec![
                egui::Event::MouseMoved(egui::vec2(20.0, -10.0)),
                egui::Event::PointerGone,
            ],
        );
        delta(&input, [1520.0, 20.0]);
        assert_eq!(input.state().pointer_position(), Some([100.0, 100.0]));
        input.spend(Duration::from_secs_f32(1.0 / 60.0));
        delta(&input, [0.0, 0.0]);
        assert!(input.state().pointer_locked());
        input.pointer_lock_changed(false);
        frame(
            &context,
            &mut input,
            vec![egui::Event::PointerMoved(egui::pos2(160.0, 100.0))],
        );
        delta(&input, [0.0, 0.0]);
    }

    #[test]
    fn unlocked_raw_motion_is_ignored_and_stopping_clears_capture() {
        let context = egui::Context::default();
        let mut input = EditorInput::default();
        frame(
            &context,
            &mut input,
            vec![egui::Event::MouseMoved(egui::vec2(30.0, 20.0))],
        );
        delta(&input, [0.0, 0.0]);
        input.pointer_lock_changed(true);
        frame(
            &context,
            &mut input,
            vec![egui::Event::MouseMoved(egui::vec2(30.0, 20.0))],
        );
        input.update(&context, false, Some(view()));
        assert!(!input.state().pointer_locked());
        delta(&input, [0.0, 0.0]);
    }
}
