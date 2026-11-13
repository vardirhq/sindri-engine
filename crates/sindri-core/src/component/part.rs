//! Several parts of one kind on one entity.
//!
//! An entity's components are a map from type name to payload, so it holds one
//! of each kind. A second of a kind is a second key: the type name, `#`, and
//! the part's name — `sindri.light2d` and `sindri.light2d#halo` are a lamp's
//! beam and its halo. The unnamed key stays the entity's first part, so every
//! scene, script and tool that reads `sindri.light2d` keeps reading what it
//! always did. A file from a newer editor still opens in an older one, which
//! keeps an unknown key as an unknown payload.
//!
//! Every registry lookup resolves a part's key to its type, so a named part is
//! validated, described and given meanings exactly as the unnamed one is.
//! `docs/object-parts.md` is the design.

use crate::{EntityId, SceneComponent, World};

use super::{ComponentRegistryError, ComponentSchemaRegistry};

/// What separates a component's type from a part's name in a component key.
pub const PART_SEPARATOR: char = '#';

/// A component key split into its type and its part's name, `None` for the
/// unnamed part.
#[must_use]
pub fn split_part(key: &str) -> (&str, Option<&str>) {
    match key.split_once(PART_SEPARATOR) {
        Some((type_name, part)) => (type_name, Some(part)),
        None => (key, None),
    }
}

/// The component key of `part` of `type_name`: the type itself for the
/// unnamed part.
#[must_use]
pub fn part_key(type_name: &str, part: Option<&str>) -> String {
    match part {
        Some(part) => format!("{type_name}{PART_SEPARATOR}{part}"),
        None => type_name.to_owned(),
    }
}

/// Whether `name` can name a part: letters, digits, `-` and `_`, as a script
/// or a person would type it, and not empty.
#[must_use]
pub fn is_part_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
}

impl ComponentSchemaRegistry {
    /// Every part of kind `T` on every *active* entity, decoded, with each
    /// part's name, `None` for the unnamed one.
    ///
    /// What a system that draws or steps every part of a kind asks, a lamp's
    /// beam and its halo alike. [`Self::query`] still answers with the unnamed
    /// part alone, for the many readers that mean "the entity's light".
    pub fn query_parts<T: SceneComponent>(
        &self,
        world: &World,
    ) -> Result<Vec<(EntityId, Option<String>, T)>, ComponentRegistryError> {
        self.require_registered(T::TYPE_NAME)?;
        let mut parts = Vec::new();
        for (entity_id, _) in world.entities() {
            for (part, component) in self.get_parts::<T>(world, entity_id)? {
                parts.push((entity_id, part, component));
            }
        }
        Ok(parts)
    }

    /// Every part of kind `T` on one *active* entity, decoded, with each
    /// part's name, `None` for the unnamed one; in key order, so the unnamed
    /// part comes first.
    pub fn get_parts<T: SceneComponent>(
        &self,
        world: &World,
        entity_id: EntityId,
    ) -> Result<Vec<(Option<String>, T)>, ComponentRegistryError> {
        self.require_registered(T::TYPE_NAME)?;
        if !world.is_active(entity_id) {
            return Ok(Vec::new());
        }
        let Some(entity) = world.get(entity_id) else {
            return Ok(Vec::new());
        };
        let mut parts = Vec::new();
        for (key, payload) in &entity.components {
            let (type_name, part) = split_part(key);
            if type_name != T::TYPE_NAME {
                continue;
            }
            let component = T::deserialize(payload).map_err(|source| {
                ComponentRegistryError::InvalidPayload {
                    entity: entity.source_id.as_ref().map_or_else(
                        || format!("{entity_id:?}"),
                        |source_id| source_id.as_str().to_owned(),
                    ),
                    type_name: key.clone(),
                    source,
                }
            })?;
            parts.push((part.map(str::to_owned), component));
        }
        Ok(parts)
    }
}

#[cfg(test)]
mod tests {
    use super::{is_part_name, part_key, split_part};

    #[test]
    fn a_key_splits_into_its_type_and_part() {
        assert_eq!(split_part("sindri.light2d"), ("sindri.light2d", None));
        assert_eq!(
            split_part("sindri.light2d#halo"),
            ("sindri.light2d", Some("halo"))
        );
        assert_eq!(
            part_key("sindri.light2d", Some("halo")),
            "sindri.light2d#halo"
        );
        assert_eq!(part_key("sindri.light2d", None), "sindri.light2d");
    }

    #[test]
    fn a_part_name_is_a_plain_word() {
        assert!(is_part_name("halo"));
        assert!(is_part_name("glow-2"));
        assert!(!is_part_name(""));
        assert!(!is_part_name("two words"));
        assert!(!is_part_name("a#b"));
        assert!(!is_part_name("a.b"));
    }
}
