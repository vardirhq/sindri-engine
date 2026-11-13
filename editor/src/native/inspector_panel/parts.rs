//! Named parts in the inspector: adding a second light or emitter to an
//! entity that has one, and renaming a part.
//!
//! An entity holds one component of each kind, and a second of a kind the
//! scene draws several of is a named part (`sindri.light2d#halo`;
//! `docs/object-parts.md`). Its name is what a script asks for
//! (`this.entity.light2d("halo")`), so a part is added under a free name and
//! can be given a better one.

use sindri_core::{CommandBuffer, EntityId, WorldCommand, is_part_name, part_key, split_part};

use super::super::EditorApp;

/// Whether `key`'s kind can have named parts.
pub(crate) fn has_parts(key: &str) -> bool {
    sindri_scene::PART_KINDS.contains(&split_part(key).0)
}

/// The first key of kind `type_name` an entity holding `taken` does not
/// use: the type itself, then `type#light-2`, `type#light-3`…
pub(crate) fn free_key(type_name: &str, taken: impl Fn(&str) -> bool) -> String {
    if !taken(type_name) {
        return type_name.to_owned();
    }
    let short = type_name
        .rsplit('.')
        .next()
        .unwrap_or(type_name)
        .trim_end_matches("2d");
    // An entity holds a handful of components; ten thousand of one kind is
    // not a scene, and the last name is answered rather than looping.
    (2..10_000)
        .map(|index| part_key(type_name, Some(&format!("{short}-{index}"))))
        .find(|key| !taken(key))
        .unwrap_or_else(|| part_key(type_name, Some(&format!("{short}-10000"))))
}

impl EditorApp {
    /// Renames the part `from` of `entity` to the part named `name`, as one
    /// undo step.
    pub(super) fn rename_part(&mut self, entity: EntityId, from: &str, name: &str) {
        let name = name.trim();
        if !is_part_name(name) {
            self.report(format!(
                "A part's name is letters, digits, '-' and '_', so `{name}` cannot name one"
            ));
            return;
        }
        let (type_name, _) = split_part(from);
        let to = part_key(type_name, Some(name));
        let Some(data) = self.world.get(entity) else {
            return;
        };
        if data.components.contains_key(&to) {
            self.report(format!("This object already has a part named {name}"));
            return;
        }
        let Some(payload) = data.components.get(from).cloned() else {
            return;
        };
        let mut buffer = CommandBuffer::new();
        buffer.push(WorldCommand::RemoveComponent {
            entity,
            type_name: from.to_owned(),
        });
        buffer.push(WorldCommand::SetComponent {
            entity,
            type_name: to,
            payload,
        });
        self.break_merge_runs();
        if let Err(error) = self.apply_edit(buffer.into_transaction(format!("Name part {name}"))) {
            self.report(error.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{free_key, has_parts};

    #[test]
    fn a_second_light_is_added_under_a_free_name() {
        let taken = ["sindri.light2d", "sindri.light2d#light-2"];
        assert_eq!(
            free_key("sindri.light2d", |key| taken.contains(&key)),
            "sindri.light2d#light-3"
        );
        assert_eq!(free_key("sindri.particles", |_| false), "sindri.particles");
        assert_eq!(
            free_key("sindri.particles", |key| key == "sindri.particles"),
            "sindri.particles#particles-2"
        );
    }

    #[test]
    fn only_kinds_the_scene_draws_several_of_have_parts() {
        assert!(has_parts("sindri.light2d"));
        assert!(has_parts("sindri.particles#glow"));
        assert!(!has_parts("sindri.sprite"));
        assert!(!has_parts("sindri.light2d.occluder"));
    }
}
