//! Mouse displacement and the transition between absolute and captured input.

use super::InputEvent;

#[derive(Clone, Debug, Default)]
pub(super) struct MouseState {
    pub(super) position: Option<[f32; 2]>,
    pub(super) delta: [f32; 2],
    pub(super) locked: bool,
    // Retain the last absolute position for UI, but never subtract across a
    // capture transition: hosts may warp or restore the cursor on release.
    baseline: bool,
}

impl MouseState {
    pub(super) fn apply(&mut self, event: InputEvent, focused: bool) {
        match event {
            InputEvent::PointerMoved { x, y } if !self.locked => {
                if !x.is_finite() || !y.is_finite() {
                    return;
                }
                if self.baseline
                    && let Some([previous_x, previous_y]) = self.position
                {
                    self.accumulate([x - previous_x, y - previous_y]);
                }
                self.position = Some([x, y]);
                self.baseline = true;
            }
            InputEvent::PointerMotion { x, y } if self.locked && focused => {
                self.accumulate([x, y]);
            }
            InputEvent::PointerLeft if !self.locked => {
                self.position = None;
                self.baseline = false;
            }
            InputEvent::PointerLockChanged(locked) => {
                let locked = locked && focused;
                if self.locked != locked {
                    self.locked = locked;
                    self.delta = [0.0, 0.0];
                    self.baseline = false;
                }
            }
            InputEvent::FocusChanged(false) => *self = Self::default(),
            _ => {}
        }
    }

    fn accumulate(&mut self, motion: [f32; 2]) {
        let delta = [self.delta[0] + motion[0], self.delta[1] + motion[1]];
        // Reject the entire event, including finite values whose sum overflows.
        // One malformed axis must not poison all subsequent camera updates.
        if motion.into_iter().all(f32::is_finite) && delta.into_iter().all(f32::is_finite) {
            self.delta = delta;
        }
    }

    pub(super) fn begin_frame(&mut self) {
        self.delta = [0.0, 0.0];
    }
}
