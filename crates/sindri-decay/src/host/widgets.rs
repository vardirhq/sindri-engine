//! Typed reads and checked writes of authored widget values.
use super::{WorldHost, convert::number};
use crate::surface::UiCall;
use decay_ir::Path;
use decay_runtime::{RuntimeError, Value};
use sindri_core::{EntityId, SceneComponent};
use sindri_scene::{UiScrollComponent, UiTextInputComponent, UiToggleComponent};

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
            _ => unreachable!("widget value call"),
        };
        let height = self
            .screen_ui
            .and_then(|ui| ui.rect(entity))
            .map(|r| r.size[1]);
        let data = self.world.get_mut(entity).ok_or_else(|| {
            RuntimeError::Host(format!("{}: entity no longer exists", path.dotted()))
        })?;
        let own_height = data.transform_3d.unwrap_or_default().scale_2d()[1];
        let payload = data.components.get_mut(component).ok_or_else(|| {
            RuntimeError::Host(format!("{}: element needs {component}", path.dotted()))
        })?;
        match call {
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
                payload[field] =
                    serde_json::json!(scroll.coerce(requested, height.unwrap_or(own_height)));
                Ok(Value::Unit)
            }
            _ => unreachable!("widget value call"),
        }
    }
}
