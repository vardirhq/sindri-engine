//! The prefabs a project's scenes place, read from disk.

use std::collections::BTreeMap;
use std::path::Path;

use sindri_core::{PrefabDocument, PrefabLibrary, SceneEntity};

use crate::write::ExportError;

/// Every prefab read so far, by asset ID, with the ones nested inside them.
///
/// A scene's instances have to be made before its textures can be found,
/// because a placed coin's sprite is in the coin's prefab and not in the scene.
/// So an instance's prefab is read, and every prefab *it* places, before the
/// scene is loaded; each ships for the same reason the scene does.
#[derive(Debug, Default)]
pub(crate) struct DiskPrefabs {
    loaded: BTreeMap<String, PrefabDocument>,
}

impl DiskPrefabs {
    /// Reads every prefab `entities` places, and every one those place.
    pub(crate) fn read_placed_by(
        &mut self,
        project: &Path,
        entities: &[SceneEntity],
    ) -> Result<(), ExportError> {
        let mut pending: Vec<String> = sources(entities).collect();
        while let Some(id) = pending.pop() {
            if self.loaded.contains_key(&id) {
                continue;
            }
            let path = super::gather::resolve(project, &id);
            let text = std::fs::read_to_string(&path)
                .map_err(|error| ExportError::unreadable(&path, &error))?;
            let prefab = PrefabDocument::from_json(&text)
                .map_err(|error| ExportError::Project(format!("{id}: {error}")))?;
            pending.extend(sources(&prefab.entities));
            self.loaded.insert(id, prefab);
        }
        Ok(())
    }

    /// The asset IDs of every prefab read.
    pub(crate) fn ids(&self) -> impl Iterator<Item = &str> {
        self.loaded.keys().map(String::as_str)
    }
}

impl PrefabLibrary for DiskPrefabs {
    fn prefab(&self, source: &str) -> Option<&PrefabDocument> {
        self.loaded.get(source)
    }
}

fn sources(entities: &[SceneEntity]) -> impl Iterator<Item = String> + '_ {
    entities
        .iter()
        .filter_map(|entity| entity.prefab.as_ref())
        .map(|instance| instance.source.clone())
}
