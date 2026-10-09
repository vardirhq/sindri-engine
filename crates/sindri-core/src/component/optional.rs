//! A field that is absent until someone adds it.
//!
//! A camera's behaviour may follow a target or not, and may be confined to an
//! area or not: `follow` and `confine` are `null` in a fresh one, which is the
//! honest blank. But a template that holds `null` there says nothing about
//! what a follow *is*, so nothing under it could be described — the target
//! that names an entity, the smoothing that is a rate — and an editor showed
//! the field as a readout that could never be filled in.
//!
//! A registration says what the field holds once added. The component as it
//! would be with it added is decoded at registration, so a field that cannot
//! be added safely is a startup error; paths under it resolve against it, so
//! they can be described like any other; and a tool asks for it to offer Add,
//! writing exactly this, and Remove, writing `null` back.

use serde_json::Value;

use super::{ComponentRegistryError, ComponentSchemaRegistry, SceneComponent, validate_payload};

/// One field that is `null` until added.
#[derive(Clone, Debug)]
pub(super) struct OptionalField {
    /// Where it lives, as a dotted path through objects.
    path: String,
    /// What it holds once added.
    added: Value,
    /// The whole field template with it added, which paths under it resolve
    /// against.
    form: Value,
}

impl super::ComponentRegistration {
    /// What the template holds at `path` once the optional fields are added.
    pub(super) fn optional_exemplar(&self, path: &str) -> Option<&Value> {
        self.optionals
            .iter()
            .find_map(|optional| super::meaning::exemplar(&optional.form, path))
    }
}

/// The value at a dotted object path, for writing.
fn at_mut<'a>(value: &'a mut Value, path: &str) -> Option<&'a mut Value> {
    path.split('.')
        .try_fold(value, |here, step| here.as_object_mut()?.get_mut(step))
}

impl ComponentSchemaRegistry {
    /// Says that the field at `path`, `null` in the template, holds `added`
    /// once someone adds it.
    ///
    /// # Errors
    /// The type is not registered or has no template, the template has no
    /// field at `path` or holds something other than `null` there, or the
    /// component with `added` in place does not decode.
    pub fn describe_optional<T: SceneComponent>(
        &mut self,
        path: &'static str,
        added: Value,
    ) -> Result<(), ComponentRegistryError> {
        let registration = self
            .registrations
            .get_mut(T::TYPE_NAME)
            .ok_or(ComponentRegistryError::NotRegistered(T::TYPE_NAME))?;
        let mut form = registration
            .fields
            .clone()
            .ok_or(ComponentRegistryError::DescribedWithoutFields(T::TYPE_NAME))?;
        let slot =
            at_mut(&mut form, path).ok_or_else(|| ComponentRegistryError::UnknownFieldPath {
                type_name: T::TYPE_NAME,
                path: path.to_owned(),
            })?;
        if !slot.is_null() {
            return Err(ComponentRegistryError::NotOptional {
                type_name: T::TYPE_NAME,
                path: path.to_owned(),
            });
        }
        *slot = added.clone();
        validate_payload::<T>(&form).map_err(|source| {
            ComponentRegistryError::OptionalMismatch {
                type_name: T::TYPE_NAME,
                path: path.to_owned(),
                source,
            }
        })?;
        registration.optionals.push(OptionalField {
            path: path.to_owned(),
            added,
            form,
        });
        Ok(())
    }

    /// What the optional field at `path` holds once added, if the field there
    /// is one.
    #[must_use]
    pub fn optional(&self, type_name: &str, path: &str) -> Option<&Value> {
        self.registrations
            .get(type_name)?
            .optionals
            .iter()
            .find(|optional| optional.path == path)
            .map(|optional| &optional.added)
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;
    use serde_json::json;

    use super::*;
    use crate::FieldMeaning;

    #[derive(Debug, Deserialize)]
    struct Follow {
        #[allow(dead_code)]
        target: String,
        #[allow(dead_code)]
        smoothing: f32,
    }

    #[derive(Debug, Deserialize)]
    struct Rig {
        #[allow(dead_code)]
        follow: Option<Follow>,
    }

    impl SceneComponent for Rig {
        const TYPE_NAME: &'static str = "test.rig";
    }

    fn registry() -> ComponentSchemaRegistry {
        let mut registry = ComponentSchemaRegistry::default();
        registry
            .register_with_default::<Rig>("Rig", json!({ "follow": null }))
            .unwrap();
        registry
    }

    #[test]
    fn an_optional_field_can_be_described_under() {
        let mut registry = registry();
        registry
            .describe_optional::<Rig>("follow", json!({ "target": "", "smoothing": 8.0 }))
            .unwrap();
        registry
            .describe::<Rig>([("follow.target", FieldMeaning::Entity)])
            .expect("the path resolves against the field as added");
        assert_eq!(
            registry.meaning("test.rig", "follow.target"),
            Some(&FieldMeaning::Entity)
        );
        assert_eq!(
            registry.optional("test.rig", "follow"),
            Some(&json!({ "target": "", "smoothing": 8.0 }))
        );
    }

    #[test]
    fn what_is_added_has_to_decode() {
        let mut registry = registry();
        assert!(matches!(
            registry.describe_optional::<Rig>("follow", json!({ "target": 3 })),
            Err(ComponentRegistryError::OptionalMismatch { .. })
        ));
    }

    #[test]
    fn a_registration_knows_the_order_its_type_declares() {
        #[derive(Debug, Deserialize)]
        struct Ordered {
            #[allow(dead_code)]
            zebra: f32,
            #[allow(dead_code)]
            apple: f32,
        }
        impl SceneComponent for Ordered {
            const TYPE_NAME: &'static str = "test.ordered";
        }
        let mut registry = ComponentSchemaRegistry::default();
        registry
            .register_with_default::<Ordered>("Ordered", json!({ "zebra": 0.0, "apple": 0.0 }))
            .unwrap();
        assert_eq!(registry.field_order("test.ordered"), ["zebra", "apple"]);
    }
}
