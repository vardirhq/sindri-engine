//! Copied contact snapshots, with live-world entity filtering.

use std::rc::Rc;

use decay_ir::{Path, StructShape};
use decay_runtime::{RuntimeError, Value};
use sindri_physics::Contact2d;

use super::WorldHost;
use crate::surface::contact::{CONTACT, CONTACT_FIELDS};

impl WorldHost<'_> {
    pub(super) fn physics_contacts(
        &self,
        path: &Path,
        args: &[Value],
    ) -> Result<Value, RuntimeError> {
        let entity = self.entity_argument(path, args, 0, "the body")?;
        let error = |message: &str| RuntimeError::Host(format!("{}: {message}", path.dotted()));
        let Some(physics) = self.physics.as_ref() else {
            return Err(error("needs physics, and this host is not running any"));
        };
        if !self.world.is_active(entity) {
            return Ok(Value::array(Vec::new()));
        }
        let contacts = match physics.world.contacts(entity) {
            Ok(contacts) => contacts,
            // A freshly spawned collider has not participated in a step yet.
            Err(sindri_physics::PhysicsError::MissingEntity(_))
                if self.world.get(entity).is_some_and(|data| {
                    data.components.contains_key("sindri.physics2d.collider")
                        || data
                            .components
                            .contains_key("sindri.physics2d.tilemap_collider")
                }) =>
            {
                Vec::new()
            }
            Err(failure) => return Err(error(&failure.to_string())),
        };
        Ok(Value::array(
            contacts
                .into_iter()
                .filter(|contact| self.world.is_active(contact.entity))
                .map(snapshot)
                .collect(),
        ))
    }
}

fn snapshot(contact: Contact2d) -> Value {
    Value::Struct {
        shape: Rc::new(StructShape {
            name: CONTACT.to_owned(),
            fields: CONTACT_FIELDS.into_iter().map(str::to_owned).collect(),
        }),
        fields: Rc::new(vec![
            Value::Reference(contact.entity.to_bits()),
            Value::Vec2(contact.point.map(f64::from)),
            Value::Vec2(contact.normal.map(f64::from)),
            Value::Number(f64::from(contact.normal_impulse)),
            Value::Number(f64::from(contact.tangent_impulse)),
            Value::Vec2(contact.force.map(f64::from)),
        ]),
    }
}
