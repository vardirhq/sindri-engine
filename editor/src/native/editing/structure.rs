//! What may not be deleted or moved because a prefab owns its shape.
//!
//! An instance saves as its prefab plus overrides, and an override can change
//! what an entity carries but not whether it exists or where it hangs. So an
//! entity inside an instance, below its root, keeps its place: deleting or
//! moving it is refused with the way out, which is to unpack the instance or to
//! change the prefab. The root itself is the scene's, and moves like anything.

use sindri_core::EntityId;

use super::super::EditorApp;

impl EditorApp {
    /// Whether any of `entities` is inside an instance below its root, saying
    /// so if one is.
    pub(super) fn refuses_restructuring(&mut self, entities: &[EntityId], doing: &str) -> bool {
        let locked = entities.iter().find(|entity| {
            self.world
                .get(**entity)
                .and_then(|data| data.prefab.as_ref())
                .is_some_and(|link| !link.root && self.world.instance_root(**entity).is_some())
        });
        let Some(&entity) = locked else {
            return false;
        };
        let name = self
            .world
            .get(entity)
            .and_then(|data| data.name.clone())
            .unwrap_or_else(|| "This entity".to_owned());
        self.report(format!(
            "Cannot {doing} {name}: it is part of a prefab instance. Unpack the instance, \
             or change the prefab itself."
        ));
        true
    }
}
