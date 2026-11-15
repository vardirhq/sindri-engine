//! Copied contact snapshots, with live-world entity filtering.

use std::rc::Rc;

use decay_ir::{Path, StructShape};
use decay_runtime::{RuntimeError, Value};
use sindri_core::EntityId;
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
                .map(|contact| self.snapshot(entity, contact))
                .collect(),
        ))
    }

    /// The name a piece of `entity`'s collider was authored with, `""` for
    /// one with none: physics keeps pieces by their place in the list, and
    /// the names stay with the scene (`docs/object-parts.md`).
    pub(super) fn piece_name(&self, entity: EntityId, piece: u16) -> String {
        let collider = self
            .world
            .get(entity)
            .and_then(|data| data.components.get("sindri.physics2d.collider"));
        let authored = match collider.and_then(|payload| payload.get("pieces")) {
            Some(pieces) => pieces.get(usize::from(piece)),
            // The single form is one piece, the payload itself.
            None if piece == 0 => collider,
            None => None,
        };
        authored
            .and_then(|piece| piece.get("name"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned()
    }

    fn snapshot(&self, entity: EntityId, contact: Contact2d) -> Value {
        snapshot(
            contact,
            self.piece_name(entity, contact.piece),
            self.piece_name(contact.entity, contact.other_piece),
        )
    }
}

fn snapshot(contact: Contact2d, piece: String, other_piece: String) -> Value {
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
            Value::String(piece),
            Value::String(other_piece),
        ]),
    }
}
