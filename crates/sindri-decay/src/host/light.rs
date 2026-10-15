//! 3D lights use typed defaults and validate a candidate before mutating payloads.
use super::{WorldHost, convert::number};
use crate::surface::light::{COMPONENT, LightField};
use decay_ir::Path;
use decay_runtime::{RuntimeError, Value};
use sindri_core::EntityId;
use sindri_scene::LightComponent;

fn error(path: &Path, text: impl std::fmt::Display) -> RuntimeError {
    RuntimeError::Host(format!("{}: {text}", path.dotted()))
}

fn is_color(under: &[&str]) -> bool {
    matches!(under, ["light", "color"] | ["entity", "light", "color"])
}

fn checked(payload: &serde_json::Value, path: &Path) -> Result<LightComponent, RuntimeError> {
    serde_json::from_value::<LightComponent>(payload.clone())
        .map_err(|cause| error(path, cause))?
        .validate()
        .map_err(|cause| error(path, cause))
}

impl WorldHost<'_> {
    fn light_payload(
        &self,
        entity: EntityId,
        path: &Path,
    ) -> Result<&serde_json::Value, RuntimeError> {
        self.world
            .get(entity)
            .and_then(|data| data.components.get(COMPONENT))
            .ok_or_else(|| error(path, "needs a sindri.light on this entity"))
    }

    pub(super) fn load_light(
        &self,
        entity: EntityId,
        field: LightField,
        path: &Path,
    ) -> Result<Value, RuntimeError> {
        let light = checked(self.light_payload(entity, path)?, path)?;
        Ok(match field {
            LightField::Enabled => Value::Bool(light.enabled),
            _ => Value::Number(f64::from(match field {
                LightField::Color(3) => 1.0,
                LightField::Color(index) => light.color[index],
                LightField::Intensity => light.intensity,
                LightField::Range => light.range,
                LightField::InnerAngle => light.inner_angle,
                LightField::OuterAngle => light.outer_angle,
                LightField::Enabled => unreachable!("handled above"),
            })),
        })
    }

    fn commit_light(
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

    pub(super) fn store_light(
        &mut self,
        entity: EntityId,
        field: LightField,
        path: &Path,
        value: &Value,
    ) -> Result<bool, RuntimeError> {
        let mut payload = self.light_payload(entity, path)?.clone();
        let current = checked(&payload, path)?;
        if let LightField::Enabled = field {
            let Value::Bool(enabled) = value else {
                return Err(error(path, "enabled takes a bool"));
            };
            payload["enabled"] = serde_json::Value::from(*enabled);
        } else {
            let number = number(path, value)?;
            if !number.is_finite() || number.abs() > f64::from(f32::MAX) {
                return Err(error(
                    path,
                    "light values must be finite and within f32 range",
                ));
            }
            match field {
                LightField::Color(3) => {
                    if (number - 1.0).abs() > f64::EPSILON {
                        return Err(error(path, "3D light colors require alpha one"));
                    }
                    return Ok(true);
                }
                LightField::Color(index) => {
                    if number < 0.0 {
                        return Err(error(path, "light RGB must be nonnegative"));
                    }
                    if payload.get("color").is_none() {
                        payload["color"] =
                            serde_json::to_value(current.color).expect("finite light color");
                    }
                    payload["color"][index] = serde_json::Value::from(number);
                }
                _ => {
                    let name = match field {
                        LightField::Intensity => "intensity",
                        LightField::Range => "range",
                        LightField::InnerAngle => "inner_angle",
                        LightField::OuterAngle => "outer_angle",
                        _ => unreachable!("handled above"),
                    };
                    payload[name] = serde_json::Value::from(number);
                }
            }
        }
        self.commit_light(entity, payload, path)
    }

    pub(super) fn load_light_color(
        &self,
        subject: Option<u64>,
        path: &Path,
        under: &[&str],
    ) -> Result<Option<Value>, RuntimeError> {
        if !is_color(under) {
            return Ok(None);
        }
        let light = checked(
            self.light_payload(self.subject(subject, path)?, path)?,
            path,
        )?;
        Ok(Some(Value::Color([
            f64::from(light.color[0]),
            f64::from(light.color[1]),
            f64::from(light.color[2]),
            1.0,
        ])))
    }

    pub(super) fn store_light_color(
        &mut self,
        subject: Option<u64>,
        path: &Path,
        under: &[&str],
        value: &Value,
    ) -> Result<bool, RuntimeError> {
        if !is_color(under) {
            return Ok(false);
        }
        let channels = value
            .components()
            .filter(|v| v.len() == 4)
            .ok_or_else(|| error(path, "light color takes a Color"))?;
        if !channels
            .iter()
            .all(|v| v.is_finite() && *v >= 0.0 && *v <= f64::from(f32::MAX))
            || (channels[3] - 1.0).abs() > f64::EPSILON
        {
            return Err(error(
                path,
                "3D light colors require finite RGB and alpha one",
            ));
        }
        let entity = self.subject(subject, path)?;
        let mut payload = self.light_payload(entity, path)?.clone();
        checked(&payload, path)?;
        payload["color"] = serde_json::json!(&channels[..3]);
        self.commit_light(entity, payload, path)
    }
}
