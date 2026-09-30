//! Validating Decay's ray arguments and returning a copied hit snapshot.

use std::rc::Rc;

use decay_ir::{Path, StructShape};
use decay_runtime::{RuntimeError, Value};
use sindri_core::EntityId;
use sindri_physics::{RayHit2d, RaycastFilter2d};

use super::{
    WorldHost,
    convert::{as_f32, number},
};
use crate::surface::raycast::{HIT_FIELDS, RAY_HIT};

impl WorldHost<'_> {
    pub(super) fn physics_raycast(
        &self,
        path: &Path,
        args: &[Value],
    ) -> Result<Value, RuntimeError> {
        let error = |message: &str| RuntimeError::Host(format!("{}: {message}", path.dotted()));
        let origin = vector(path, args.first())?;
        let direction = vector(path, args.get(1))?;
        let distance = as_f32(number(path, args.get(2).unwrap_or(&Value::Null))?);
        let mask = number(path, args.get(3).unwrap_or(&Value::Null))?;
        if !mask.is_finite()
            || !(0.0..=f64::from(u32::MAX)).contains(&mask)
            || mask.fract().abs() > 0.0
        {
            return Err(error("mask must be a whole number from 0 to 4294967295"));
        }
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let mask = mask as u32;
        let Some(Value::Bool(include_sensors)) = args.get(4) else {
            return Err(error("include_sensors must be bool"));
        };
        let exclude = match args.get(5) {
            Some(Value::Null) => None,
            Some(Value::Reference(bits)) => Some(EntityId::from_bits(*bits)),
            _ => return Err(error("exclude must be an Entity or null")),
        };
        let Some(physics) = self.physics.as_ref() else {
            return Err(error("needs physics, and this host is not running any"));
        };
        let hit = physics
            .world
            .raycast_where(
                origin,
                direction,
                distance,
                RaycastFilter2d {
                    mask,
                    include_sensors: *include_sensors,
                    exclude,
                },
                |entity| self.world.is_active(entity),
            )
            .map_err(|failure| error(&failure.to_string()))?;
        Ok(hit.map_or(Value::Null, snapshot))
    }
}

fn vector(path: &Path, value: Option<&Value>) -> Result<[f32; 2], RuntimeError> {
    let Some(Value::Vec2([x, y])) = value else {
        return Err(RuntimeError::Host(format!(
            "{} takes Vec2 origin and direction",
            path.dotted()
        )));
    };
    Ok([as_f32(*x), as_f32(*y)])
}

fn snapshot(hit: RayHit2d) -> Value {
    Value::Struct {
        shape: Rc::new(StructShape {
            name: RAY_HIT.to_owned(),
            fields: HIT_FIELDS.into_iter().map(str::to_owned).collect(),
        }),
        fields: Rc::new(vec![
            Value::Reference(hit.entity.to_bits()),
            Value::Vec2(hit.point.map(f64::from)),
            Value::Vec2(hit.normal.map(f64::from)),
            Value::Number(f64::from(hit.distance)),
        ]),
    }
}
