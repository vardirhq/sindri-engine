//! Checked writes to the existing scene environment, without a parallel state.
use super::{WorldHost, convert::number};
use crate::surface::environment::{COMPONENT, EnvironmentField};
use decay_ir::Path;
use decay_runtime::{RuntimeError, Value};
use sindri_core::EntityId;
use sindri_scene::EnvironmentComponent;

fn error(path: &Path, text: impl std::fmt::Display) -> RuntimeError {
    RuntimeError::Host(format!("{}: {text}", path.dotted()))
}

fn checked(payload: &serde_json::Value, path: &Path) -> Result<EnvironmentComponent, RuntimeError> {
    serde_json::from_value::<EnvironmentComponent>(payload.clone())
        .map_err(|cause| error(path, cause))?
        .validate()
        .map_err(|cause| error(path, cause))
}

fn color_name(under: &[&str]) -> Option<&'static str> {
    match under {
        ["environment", "ambient_color"] | ["entity", "environment", "ambient_color"] => {
            Some("ambient_color")
        }
        ["environment", "background"] | ["entity", "environment", "background"] => {
            Some("background")
        }
        ["environment", "fog", "color"] | ["entity", "environment", "fog", "color"] => {
            Some("fog.color")
        }
        _ => None,
    }
}

fn color(environment: EnvironmentComponent, name: &str) -> [f32; 4] {
    match name {
        "background" => environment.background,
        "fog.color" => {
            let [r, g, b] = environment.fog.color;
            [r, g, b, 1.0]
        }
        _ => {
            let [r, g, b] = environment.ambient_color;
            [r, g, b, 1.0]
        }
    }
}

/// The JSON slot a color lives in: top level, or `color` inside `fog`.
fn color_slot<'a>(
    payload: &'a mut serde_json::Value,
    current: EnvironmentComponent,
    name: &str,
) -> &'a mut serde_json::Value {
    if name == "fog.color" {
        &mut section(payload, current, "fog")["color"]
    } else {
        &mut payload[name]
    }
}

/// A section of the payload such as `fog`, filled from the checked view when
/// the scene never authored it, so a write keeps every other field's default.
fn section<'a>(
    payload: &'a mut serde_json::Value,
    current: EnvironmentComponent,
    name: &str,
) -> &'a mut serde_json::Value {
    if payload.get(name).is_none() {
        let full = serde_json::to_value(current).unwrap_or_default();
        payload[name] = full.get(name).cloned().unwrap_or_default();
    }
    &mut payload[name]
}

/// What the checked view holds at `section.key`, defaults included.
fn nested(environment: EnvironmentComponent, section: &str, key: &str) -> serde_json::Value {
    serde_json::to_value(environment)
        .ok()
        .and_then(|full| full.get(section)?.get(key).cloned())
        .unwrap_or_default()
}

fn finite(value: f64, path: &Path) -> Result<(), RuntimeError> {
    if value.is_finite() && value.abs() <= f64::from(f32::MAX) {
        Ok(())
    } else {
        Err(error(
            path,
            "environment values must be finite and within f32 range",
        ))
    }
}

impl WorldHost<'_> {
    fn environment_payload(
        &self,
        entity: EntityId,
        path: &Path,
    ) -> Result<&serde_json::Value, RuntimeError> {
        self.world
            .get(entity)
            .and_then(|data| data.components.get(COMPONENT))
            .ok_or_else(|| error(path, "needs a sindri.environment on this entity"))
    }

    fn commit_environment(
        &mut self,
        entity: EntityId,
        payload: serde_json::Value,
        path: &Path,
    ) -> Result<bool, RuntimeError> {
        checked(&payload, path)?;
        self.world
            .get_mut(entity)
            .ok_or_else(|| error(path, "entity no longer exists"))?
            .components
            .insert(COMPONENT.into(), payload);
        Ok(true)
    }

    pub(super) fn load_environment(
        &self,
        entity: EntityId,
        field: EnvironmentField,
        path: &Path,
    ) -> Result<Value, RuntimeError> {
        let environment = checked(self.environment_payload(entity, path)?, path)?;
        Ok(match field {
            EnvironmentField::AmbientIntensity => {
                Value::Number(f64::from(environment.ambient_intensity))
            }
            EnvironmentField::AmbientColor(index) => {
                Value::Number(f64::from(color(environment, "ambient_color")[index]))
            }
            EnvironmentField::Background(index) => {
                Value::Number(f64::from(environment.background[index]))
            }
            EnvironmentField::FogColor(index) => {
                Value::Number(f64::from(color(environment, "fog.color")[index]))
            }
            EnvironmentField::Number(section, key) => {
                Value::Number(nested(environment, section, key).as_f64().unwrap_or(0.0))
            }
            EnvironmentField::Flag(section, key) => {
                Value::Bool(nested(environment, section, key).as_bool().unwrap_or(false))
            }
        })
    }

    pub(super) fn store_environment(
        &mut self,
        entity: EntityId,
        field: EnvironmentField,
        path: &Path,
        value: &Value,
    ) -> Result<bool, RuntimeError> {
        let mut payload = self.environment_payload(entity, path)?.clone();
        let current = checked(&payload, path)?;
        if let EnvironmentField::Flag(name, key) = field {
            let Value::Bool(on) = value else {
                return Err(error(path, "takes true or false"));
            };
            section(&mut payload, current, name)[key] = serde_json::Value::Bool(*on);
            return self.commit_environment(entity, payload, path);
        }
        let number = number(path, value)?;
        finite(number, path)?;
        let (name, index) = match field {
            EnvironmentField::Flag(..) => unreachable!("handled above"),
            EnvironmentField::Number(name, key) => {
                section(&mut payload, current, name)[key] = serde_json::Value::from(number);
                return self.commit_environment(entity, payload, path);
            }
            EnvironmentField::FogColor(3) => {
                if (number - 1.0).abs() > f64::EPSILON {
                    return Err(error(path, "fog color requires alpha one"));
                }
                return Ok(true);
            }
            EnvironmentField::FogColor(index) => ("fog.color", index),
            EnvironmentField::AmbientIntensity => {
                if number < 0.0 {
                    return Err(error(path, "ambient intensity must be nonnegative"));
                }
                payload["ambient_intensity"] = serde_json::Value::from(number);
                return self.commit_environment(entity, payload, path);
            }
            EnvironmentField::AmbientColor(3) => {
                if (number - 1.0).abs() > f64::EPSILON {
                    return Err(error(path, "ambient color requires alpha one"));
                }
                return Ok(true);
            }
            EnvironmentField::AmbientColor(index) => ("ambient_color", index),
            EnvironmentField::Background(index) => ("background", index),
        };
        let channels = color(current, name);
        let count = if name == "background" { 4 } else { 3 };
        let slot = color_slot(&mut payload, current, name);
        if slot.is_null() {
            *slot = serde_json::json!(&channels[..count]);
        }
        slot[index] = serde_json::Value::from(number);
        self.commit_environment(entity, payload, path)
    }

    pub(super) fn load_environment_color(
        &self,
        subject: Option<u64>,
        path: &Path,
        under: &[&str],
    ) -> Result<Option<Value>, RuntimeError> {
        let Some(name) = color_name(under) else {
            return Ok(None);
        };
        let current = checked(
            self.environment_payload(self.subject(subject, path)?, path)?,
            path,
        )?;
        Ok(Some(Value::Color(color(current, name).map(f64::from))))
    }

    pub(super) fn store_environment_color(
        &mut self,
        subject: Option<u64>,
        path: &Path,
        under: &[&str],
        value: &Value,
    ) -> Result<bool, RuntimeError> {
        let Some(name) = color_name(under) else {
            return Ok(false);
        };
        let Value::Color(channels) = value else {
            return Err(error(path, "environment color takes a Color"));
        };
        for channel in channels {
            finite(*channel, path)?;
        }
        let count = if name == "background" { 4 } else { 3 };
        if count == 3 && (channels[3] - 1.0).abs() > f64::EPSILON {
            return Err(error(path, "this color requires alpha one"));
        }
        let entity = self.subject(subject, path)?;
        let mut payload = self.environment_payload(entity, path)?.clone();
        let current = checked(&payload, path)?;
        *color_slot(&mut payload, current, name) = serde_json::json!(&channels[..count]);
        self.commit_environment(entity, payload, path)
    }
}
