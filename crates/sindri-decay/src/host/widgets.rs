//! Typed reads and checked writes of authored widget values.
//!
//! A setter writes the value inside what the widget can hold -- a field's
//! `max_length`, the distance a region can scroll -- and does not report it
//! through `Ui.changed`. That answers what the person did this step, and a
//! script that heard its own writes back would answer itself for ever.
use super::{WorldHost, convert::number};
use crate::surface::UiCall;
use decay_ir::Path;
use decay_runtime::{RuntimeError, Value};
use sindri_core::{EntityId, SceneComponent};
use sindri_scene::{
    UiDropdownComponent, UiScrollComponent, UiTextInputComponent, UiToggleComponent,
    dropdown_options,
};

impl WorldHost<'_> {
    pub(super) fn widget_call(
        &mut self,
        call: UiCall,
        path: &Path,
        args: &[Value],
        entity: EntityId,
    ) -> Result<Value, RuntimeError> {
        let (component, field) = match call {
            UiCall::Checked | UiCall::SetChecked => (UiToggleComponent::TYPE_NAME, "checked"),
            UiCall::InputText | UiCall::SetInputText => (UiTextInputComponent::TYPE_NAME, "value"),
            UiCall::ScrollOffset | UiCall::SetScrollOffset => {
                (UiScrollComponent::TYPE_NAME, "offset")
            }
            UiCall::Selected | UiCall::SetSelected => (UiDropdownComponent::TYPE_NAME, "selected"),
            UiCall::Open => (UiDropdownComponent::TYPE_NAME, "open"),
            _ => unreachable!("widget value call"),
        };
        let options = dropdown_options(self.world, entity).len();
        let height = self
            .screen_ui
            .and_then(|ui| ui.rect(entity))
            .map(|r| r.size[1]);
        let measured = self
            .screen_ui
            .and_then(|ui| ui.scroll_content(entity))
            .unwrap_or(0.0);
        let data = self.world.get_mut(entity).ok_or_else(|| {
            RuntimeError::Host(format!("{}: entity no longer exists", path.dotted()))
        })?;
        let own_height = data.transform_3d.unwrap_or_default().scale_2d()[1];
        let payload = data.components.get_mut(component).ok_or_else(|| {
            RuntimeError::Host(format!("{}: element needs {component}", path.dotted()))
        })?;
        match call {
            UiCall::Selected => Ok(Value::Number(
                payload
                    .get(field)
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(0.0),
            )),
            UiCall::Open => Ok(Value::Bool(
                payload
                    .get(field)
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false),
            )),
            UiCall::SetSelected => {
                let requested = number(path, args.get(1).unwrap_or(&Value::Null))?;
                if !requested.is_finite() || requested < 0.0 {
                    return Err(RuntimeError::Host(format!(
                        "{}: an option is counted from zero, not {requested}",
                        path.dotted()
                    )));
                }
                // Kept to the options there are, as a slider keeps to its range.
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let index = (requested as usize).min(options.saturating_sub(1));
                payload[field] = serde_json::json!(index);
                Ok(Value::Unit)
            }
            UiCall::Checked => Ok(Value::Bool(
                payload
                    .get(field)
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false),
            )),
            UiCall::InputText => Ok(Value::String(
                payload
                    .get(field)
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("")
                    .to_owned(),
            )),
            UiCall::ScrollOffset => Ok(Value::Number(
                payload
                    .get(field)
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(0.0),
            )),
            UiCall::SetChecked => {
                let Some(Value::Bool(checked)) = args.get(1) else {
                    return Err(RuntimeError::Host("set_checked needs a bool".to_owned()));
                };
                payload[field] = serde_json::json!(checked);
                Ok(Value::Unit)
            }
            UiCall::SetInputText => {
                let Some(Value::String(text)) = args.get(1) else {
                    return Err(RuntimeError::Host("set_input_text needs text".to_owned()));
                };
                let input = serde_json::from_value::<UiTextInputComponent>(payload.clone())
                    .map_err(|e| RuntimeError::Host(e.to_string()))?;
                payload[field] = serde_json::json!(input.coerce(text));
                Ok(Value::Unit)
            }
            UiCall::SetScrollOffset => {
                let requested = number(path, args.get(1).unwrap_or(&Value::Null))?;
                if !requested.is_finite() {
                    return Err(RuntimeError::Host(
                        "scroll offset must be finite".to_owned(),
                    ));
                }
                #[allow(clippy::cast_possible_truncation)]
                let requested = requested as f32;
                let scroll = serde_json::from_value::<UiScrollComponent>(payload.clone())
                    .map_err(|e| RuntimeError::Host(e.to_string()))?;
                payload[field] = serde_json::json!(scroll.coerce(
                    requested,
                    height.unwrap_or(own_height),
                    measured,
                ));
                Ok(Value::Unit)
            }
            _ => unreachable!("widget value call"),
        }
    }

    /// Where a text field's caret and selection are, in characters.
    pub(super) fn caret_call(
        &self,
        call: UiCall,
        path: &Path,
        entity: EntityId,
    ) -> Result<Value, RuntimeError> {
        let caret = self
            .screen_ui
            .and_then(|ui| ui.caret(self.world, entity))
            .ok_or_else(|| {
                RuntimeError::Host(format!(
                    "{}: element needs {}",
                    path.dotted(),
                    UiTextInputComponent::TYPE_NAME
                ))
            })?;
        let selected = caret.selection();
        let at = match call {
            UiCall::Caret => caret.caret,
            UiCall::SelectionStart => selected.start,
            _ => selected.end,
        };
        #[allow(clippy::cast_precision_loss)]
        Ok(Value::Number(at as f64))
    }
}
