//! Player-facing input values and gameplay control of the world camera.

use decay_ir::Path;
use decay_runtime::{RuntimeError, Value};

use super::WorldHost;
use super::convert::{as_f32, number};
use crate::surface::{AimValue, CameraCall, CameraValue, GestureValue, PointerValue, StickValue, TouchCall};

impl WorldHost<'_> {
    pub(super) fn stick_value(&self, value: StickValue) -> Value {
        let stick = self.context.input.stick();
        let pushed = stick.value();
        match value {
            StickValue::X => Value::Number(f64::from(pushed[0])),
            StickValue::Y => Value::Number(f64::from(pushed[1])),
            StickValue::Held => Value::Bool(stick.is_engaged()),
            StickValue::Direction => Value::Vec2([f64::from(pushed[0]), f64::from(pushed[1])]),
            StickValue::AnchorX => Value::Number(f64::from(stick.anchor(self.context.input.presses()).unwrap_or([0.0, 0.0])[0])),
            StickValue::AnchorY => Value::Number(f64::from(stick.anchor(self.context.input.presses()).unwrap_or([0.0, 0.0])[1])),
        }
    }

    pub(super) fn camera_value(&self, value: CameraValue) -> Value {
        let pan = self.camera_pan.unwrap_or([0.0; 3]);
        Value::Number(f64::from(match value { CameraValue::PanX => pan[0], CameraValue::PanY => pan[1], CameraValue::PanZ => pan[2] }))
    }

    pub(super) fn camera_call(&mut self, call: CameraCall, path: &Path, args: &[Value]) -> Result<Value, RuntimeError> {
        let numeric = |index: usize| -> Result<f32, RuntimeError> {
            let value = number(path, args.get(index).unwrap_or(&Value::Null))?;
            if !value.is_finite() {
                return Err(RuntimeError::Host(format!("{} takes finite numbers", path.dotted())));
            }
            Ok(as_f32(value))
        };
        let changed = match call {
            CameraCall::AddTrauma => {
                let amount = numeric(0)?;
                if amount < 0.0 { return Err(RuntimeError::Host(format!("{} takes a non-negative trauma amount", path.dotted()))); }
                sindri_scene::add_camera_trauma(self.world, amount)
            }
            CameraCall::Follow => {
                let target = self.entity_argument(path, args, 0, "the follow target")?;
                sindri_scene::set_camera_follow_target(self.world, target)
            }
            CameraCall::ClearFollow => sindri_scene::clear_camera_follow(self.world),
            CameraCall::FollowOffset => sindri_scene::set_camera_follow_offset(self.world, [numeric(0)?, numeric(1)?, numeric(2)?]),
            CameraCall::DeadZone => sindri_scene::set_camera_dead_zone(self.world, [numeric(0)?, numeric(1)?]),
            CameraCall::Smoothing => sindri_scene::set_camera_smoothing(self.world, numeric(0)?),
            CameraCall::MaxSpeed => sindri_scene::set_camera_max_speed(self.world, numeric(0)?),
            CameraCall::Bounds => sindri_scene::set_camera_bounds(self.world, [numeric(0)?, numeric(1)?], [numeric(2)?, numeric(3)?]),
            CameraCall::ClearBounds => sindri_scene::clear_camera_bounds(self.world),
            CameraCall::Shake => sindri_scene::set_camera_shake(self.world, numeric(0)?, numeric(1)?, numeric(2)?),
        };
        if !changed {
            return Err(RuntimeError::Host(format!("{} could not change the camera behavior; it needs exactly one authored behavior camera and valid settings", path.dotted())));
        }
        Ok(Value::Unit)
    }

    pub(super) fn gesture_value(&self, value: GestureValue) -> Value {
        use GestureValue::{DragX, DragY, Dragging, Held, HoldX, HoldY, Pinch, Pinching, TapX, TapY, Tapped};
        let Some(gestures) = self.gestures else {
            return match value { Tapped | Held | Dragging | Pinching => Value::Bool(false), Pinch => Value::Number(1.0), _ => Value::Number(0.0) };
        };
        let at = |point: Option<[f32; 2]>, axis: usize| Value::Number(point.map_or(0.0, |point| f64::from(point[axis])));
        match value {
            Tapped => Value::Bool(gestures.tap().is_some()), TapX => at(gestures.tap(), 0), TapY => at(gestures.tap(), 1),
            Held => Value::Bool(gestures.long_press().is_some()), HoldX => at(gestures.long_press(), 0), HoldY => at(gestures.long_press(), 1),
            Dragging => Value::Bool(gestures.drag().is_some()), DragX => at(gestures.drag(), 0), DragY => at(gestures.drag(), 1),
            Pinching => Value::Bool(gestures.pinch().is_some()), Pinch => Value::Number(f64::from(gestures.pinch().unwrap_or(1.0))),
        }
    }

    pub(super) fn aim_value(&self, value: AimValue) -> Value {
        let Some(aim) = self.aim else { return if matches!(value, AimValue::Hit) { Value::Bool(false) } else { Value::Number(0.0) }; };
        match value {
            AimValue::Hit => Value::Bool(true), AimValue::X => Value::Number(f64::from(aim.cell.x)), AimValue::Y => Value::Number(f64::from(aim.cell.y)),
            AimValue::Z => Value::Number(f64::from(aim.cell.z)), AimValue::PlaceX => Value::Number(f64::from(aim.against.x)),
            AimValue::PlaceY => Value::Number(f64::from(aim.against.y)), AimValue::PlaceZ => Value::Number(f64::from(aim.against.z)),
        }
    }

    pub(super) fn pointer_value(&self, value: PointerValue) -> Value {
        let position = self.context.input.pointer_position();
        let overlay = || self.screen_ui.and_then(sindri_scene::ScreenUi::pointer_overlay).unwrap_or([0.0, 0.0]);
        match value {
            PointerValue::Inside => Value::Bool(position.is_some()),
            PointerValue::OverUi => Value::Bool(self.screen_ui.is_some_and(sindri_scene::ScreenUi::captures_pointer)),
            PointerValue::Position => { let [x, y] = position.unwrap_or([0.0, 0.0]); Value::Vec2([f64::from(x), f64::from(y)]) }
            PointerValue::Overlay => { let [x, y] = overlay(); Value::Vec2([f64::from(x), f64::from(y)]) }
            PointerValue::X => Value::Number(f64::from(position.unwrap_or([0.0, 0.0])[0])),
            PointerValue::Y => Value::Number(f64::from(position.unwrap_or([0.0, 0.0])[1])),
            PointerValue::OverlayX => Value::Number(f64::from(overlay()[0])), PointerValue::OverlayY => Value::Number(f64::from(overlay()[1])),
        }
    }

    pub(super) fn touch_call(&self, call: TouchCall, path: &Path, args: &[Value]) -> Result<Value, RuntimeError> {
        let index = number(path, args.first().unwrap_or(&Value::Null))?;
        if !index.is_finite() || index.fract() != 0.0 || index < 0.0 {
            return Err(RuntimeError::Host(format!("{} takes which finger, counting from zero, and the script gave {index}", path.dotted())));
        }
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let position = self.context.input.touch_at(index as usize).ok_or_else(|| RuntimeError::Host(format!("{} was asked for finger {index}, and {} are down", path.dotted(), self.context.input.touch_count())))?;
        Ok(Value::Number(f64::from(match call { TouchCall::X => position[0], TouchCall::Y => position[1] })))
    }
}
