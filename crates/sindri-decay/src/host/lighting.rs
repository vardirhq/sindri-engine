//! `Lighting.*`: what the scene's 2D lights and occluders say about a place.

use decay_ir::Path;
use decay_runtime::{RuntimeError, Value};

use super::WorldHost;
use crate::surface::lighting::LightingCall;

fn point(path: &Path, value: &Value) -> Result<[f32; 2], RuntimeError> {
    match value {
        Value::Vec2(parts)
            if parts
                .iter()
                .all(|part| part.is_finite() && part.abs() <= f64::from(f32::MAX)) =>
        {
            #[allow(clippy::cast_possible_truncation)]
            Ok([parts[0] as f32, parts[1] as f32])
        }
        _ => Err(RuntimeError::Host(format!(
            "{} requires finite Vec2 points",
            path.dotted()
        ))),
    }
}

impl WorldHost<'_> {
    pub(super) fn lighting_call(
        &self,
        call: LightingCall,
        path: &Path,
        args: &[Value],
    ) -> Result<Value, RuntimeError> {
        let Some(lighting) = self.lighting else {
            return Err(RuntimeError::Host(format!(
                "{} requires a scene with 2D lighting turned on",
                path.dotted()
            )));
        };
        match (call, args) {
            (LightingCall::Visible, [from, to]) => Ok(Value::Bool(
                lighting.visible(point(path, from)?, point(path, to)?),
            )),
            (LightingCall::Level, [at]) => {
                Ok(Value::Number(f64::from(lighting.level(point(path, at)?))))
            }
            _ => Err(RuntimeError::Host(format!(
                "{} takes the points it asks about",
                path.dotted()
            ))),
        }
    }
}
