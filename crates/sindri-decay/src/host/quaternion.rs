//! Quaternion values use Decay's copied structs, with atomic transform writes.

use std::rc::Rc;

use decay_ir::{Path, StructShape};
use decay_runtime::{RuntimeError, Value};

use super::{WorldHost, convert::as_f32};
use crate::surface::{QUAT, QUAT_FIELDS};

fn rotation_path<'a>(under: &'a [&'a str]) -> Option<&'a [&'a str]> {
    match under {
        ["transform", "rotation", rest @ ..] | ["entity", "transform", "rotation", rest @ ..] => {
            Some(rest)
        }
        _ => None,
    }
}

fn snapshot(rotation: [f32; 4]) -> Value {
    Value::Struct {
        shape: Rc::new(StructShape {
            name: QUAT.to_owned(),
            fields: QUAT_FIELDS.into_iter().map(str::to_owned).collect(),
        }),
        fields: Rc::new(rotation.map(|v| Value::Number(f64::from(v))).to_vec()),
    }
}

fn components(value: &Value) -> Option<[f64; 4]> {
    let Value::Struct { shape, fields } = value else {
        return None;
    };
    if shape.name != QUAT || shape.fields.iter().map(String::as_str).ne(QUAT_FIELDS) {
        return None;
    }
    let [
        Value::Number(x),
        Value::Number(y),
        Value::Number(z),
        Value::Number(w),
    ] = fields.as_slice()
    else {
        return None;
    };
    Some([*x, *y, *z, *w])
}

fn normalized(rotation: [f64; 4]) -> Option<[f32; 4]> {
    if !rotation.into_iter().all(f64::is_finite) {
        return None;
    }
    let largest = rotation.into_iter().map(f64::abs).fold(0.0, f64::max);
    if largest <= 0.0 {
        return None;
    }
    // Scaling first handles every finite f64 input without squared-length
    // overflow or underflow. Only unit components cross the engine f32 seam.
    let scaled = rotation.map(|v| v / largest);
    let length = scaled.into_iter().map(|v| v * v).sum::<f64>().sqrt();
    Some(scaled.map(|v| as_f32(v / length)))
}

impl WorldHost<'_> {
    pub(super) fn load_rotation(
        &self,
        subject: Option<u64>,
        path: &Path,
        under: &[&str],
    ) -> Result<Option<Value>, RuntimeError> {
        if !rotation_path(under).is_some_and(<[&str]>::is_empty) {
            return Ok(None);
        }
        let entity = self.subject(subject, path)?;
        Ok(self.transform_of(entity).map(|t| snapshot(t.rotation)))
    }

    pub(super) fn store_rotation(
        &mut self,
        subject: Option<u64>,
        path: &Path,
        under: &[&str],
        value: &Value,
    ) -> Result<bool, RuntimeError> {
        let Some(rest) = rotation_path(under) else {
            return Ok(false);
        };
        let error = |text: &str| RuntimeError::Host(format!("{}: {text}", path.dotted()));
        if !rest.is_empty() {
            if rest.len() == 1 && QUAT_FIELDS.contains(&rest[0]) {
                return Err(error(
                    "rotation components are read-only; assign a whole Quat",
                ));
            }
            return Ok(false);
        }
        let entity = self.subject(subject, path)?;
        let Some(mut transform) = self.transform_of(entity) else {
            return Ok(false);
        };
        let rotation = components(value).ok_or_else(|| error("rotation takes a Quat"))?;
        transform.rotation =
            normalized(rotation).ok_or_else(|| error("rotation must be finite and nonzero"))?;
        // Rotation has no translation: keep position, scale and the Z lock
        // byte-for-byte, including under a singular or nonuniform parent.
        let Some(data) = self.world.get_mut(entity) else {
            return Ok(false);
        };
        data.transform_3d = Some(transform);
        Ok(true)
    }
}
