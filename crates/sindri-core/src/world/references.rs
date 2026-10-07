//! Saving runtime-local prefab references as registered stable scene references.
use serde_json::Value;

use crate::{ComponentSchemaRegistry, EntityId, FieldMeaning, PrefabLibrary, SceneDocument, World};

use super::WorldError;

impl World {
    /// Saves a world, remapping registered entity fields in runtime prefab spawns.
    ///
    /// Uses `FieldMeaning::Entity`, including dotted paths through `[]` lists.
    /// Stable IDs must already exist. Unknown components and undescribed fields
    /// are preserved unchanged; empty references remain explicitly unbound.
    /// The live world is never edited, and runtime identity is never serialized.
    ///
    /// # Errors
    /// As [`Self::to_scene_with`], plus a nonempty reference that cannot resolve
    /// inside its spawn, an unstable target, or a reference field of the wrong type.
    pub fn to_scene_with_references(
        &self,
        prefabs: &dyn PrefabLibrary,
        components: &ComponentSchemaRegistry,
    ) -> Result<SceneDocument, WorldError> {
        let mut saved = self.clone();
        for (owner, data) in self.entities() {
            if data.prefab_identity.is_none() {
                continue;
            }
            for (type_name, payload) in &data.components {
                let mut remapped = payload.clone();
                for (path, meaning) in components.meanings(type_name) {
                    if matches!(meaning, FieldMeaning::Entity) {
                        remap(self, owner, type_name, path, &mut remapped, path)?;
                    }
                }
                saved
                    .get_mut(owner)
                    .expect("a cloned world retains its handles")
                    .components
                    .insert(type_name.clone(), remapped);
            }
        }
        saved.to_scene_with(prefabs)
    }
}

fn remap(
    world: &World,
    owner: EntityId,
    component: &str,
    field: &str,
    value: &mut Value,
    remaining: &str,
) -> Result<(), WorldError> {
    if remaining.is_empty() {
        return remap_target(world, owner, component, field, value);
    }
    if !value.is_object() {
        return Err(invalid(owner, component, field, "expected an object"));
    }
    let (segment, rest) = remaining.split_once('.').unwrap_or((remaining, ""));
    let key = segment.strip_suffix("[]").unwrap_or(segment);
    let Some(child) = value.get_mut(key) else {
        return Ok(()); // Optional fields remain absent.
    };
    if segment.ends_with("[]") {
        if let Some(items) = child.as_array_mut() {
            for item in items {
                remap(world, owner, component, field, item, rest)?;
            }
            return Ok(());
        }
        return Err(invalid(owner, component, field, "expected a list"));
    }
    remap(world, owner, component, field, child, rest)
}

fn remap_target(
    world: &World,
    owner: EntityId,
    component: &str,
    field: &str,
    value: &mut Value,
) -> Result<(), WorldError> {
    let target = value
        .as_str()
        .ok_or_else(|| invalid(owner, component, field, "expected text"))?;
    if target.is_empty() {
        return Ok(());
    }
    let entity = world.prefab_entity(owner, target).ok_or_else(|| {
        invalid(
            owner,
            component,
            field,
            &format!("unresolved local target '{target}'"),
        )
    })?;
    let id = world
        .get(entity)
        .and_then(|data| data.source_id.as_ref())
        .ok_or(WorldError::UnstableEntity(entity))?;
    *value = Value::String(id.as_str().to_owned());
    Ok(())
}

fn invalid(owner: EntityId, component: &str, field: &str, detail: &str) -> WorldError {
    WorldError::InvalidPrefabReference {
        owner,
        component: component.to_owned(),
        field: field.to_owned(),
        detail: detail.to_owned(),
    }
}
