//! The prefabs an open scene places, read from the project on disk.
//!
//! An instance is made from its prefab when the scene opens and written back as
//! a reference when it saves, so both need the prefab there and then. The
//! script loader also loads every prefab, but in the background: a scene cannot
//! wait a frame for its own entities to exist. These are read synchronously,
//! beside the scene, which is where every asset ID resolves.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use sindri_core::{PREFAB_SUFFIX, PrefabDocument, PrefabLibrary, SceneEntity};

/// Prefab documents by asset ID, and the directory those IDs resolve against.
#[derive(Clone, Debug, Default)]
pub struct ScenePrefabs {
    root: Option<PathBuf>,
    loaded: BTreeMap<String, PrefabDocument>,
    /// What the project's file watcher last read for each prefab.
    ///
    /// Kept apart from `loaded` so a change is noticed once, when the watcher
    /// reports it, and not every frame the two differ — which they do, for a
    /// moment, after the editor itself writes a prefab and before the watcher
    /// reads it back.
    heard: BTreeMap<String, PrefabDocument>,
}

impl ScenePrefabs {
    /// An empty set resolving against the directory `scene` is in.
    #[must_use]
    pub fn beside(scene: Option<&Path>) -> Self {
        Self {
            root: scene.and_then(Path::parent).map(Path::to_path_buf),
            loaded: BTreeMap::new(),
            heard: BTreeMap::new(),
        }
    }

    /// Where asset IDs resolve.
    #[must_use]
    pub fn root(&self) -> Option<&Path> {
        self.root.as_deref()
    }

    /// Follows the scene to another directory, keeping what is loaded.
    pub fn move_beside(&mut self, scene: &Path) {
        self.root = scene.parent().map(Path::to_path_buf);
    }

    /// Reads every prefab `entities` place, and every one those place.
    ///
    /// # Errors
    /// The first prefab that cannot be read or is not valid, named.
    pub fn read_placed_by(&mut self, entities: &[SceneEntity]) -> Result<(), String> {
        let mut pending: Vec<String> = placed_by(entities).collect();
        while let Some(id) = pending.pop() {
            if self.loaded.contains_key(&id) {
                continue;
            }
            let prefab = self.read(&id)?;
            pending.extend(placed_by(&prefab.entities));
            self.loaded.insert(id, prefab);
        }
        Ok(())
    }

    /// Reads every prefab `entities` place that can be read, answering with
    /// each one that could not and why.
    pub fn read_available(&mut self, entities: &[SceneEntity]) -> Vec<(String, String)> {
        let mut failed: Vec<(String, String)> = Vec::new();
        let mut pending: Vec<String> = placed_by(entities).collect();
        while let Some(id) = pending.pop() {
            if self.loaded.contains_key(&id) || failed.iter().any(|(gone, _)| *gone == id) {
                continue;
            }
            match self.read(&id) {
                Ok(prefab) => {
                    pending.extend(placed_by(&prefab.entities));
                    self.loaded.insert(id, prefab);
                }
                Err(reason) => failed.push((id, reason)),
            }
        }
        failed
    }

    /// Reads one prefab from disk, and the ones nested in it.
    ///
    /// # Errors
    /// As [`Self::read_placed_by`].
    pub fn read_one(&mut self, id: &str) -> Result<&PrefabDocument, String> {
        if !self.loaded.contains_key(id) {
            let prefab = self.read(id)?;
            self.read_placed_by(&prefab.entities)?;
            self.loaded.insert(id.to_owned(), prefab);
        }
        Ok(&self.loaded[id])
    }

    /// Replaces what is held for `id`, answering with what it was.
    pub fn replace(&mut self, id: &str, prefab: PrefabDocument) -> Option<PrefabDocument> {
        self.loaded.insert(id.to_owned(), prefab)
    }

    /// Records what the file watcher read for `id`, answering whether it is a
    /// change the instances have not been made from — an edit to the prefab
    /// made somewhere else.
    pub fn heard(&mut self, id: &str, read: &PrefabDocument) -> bool {
        if self.heard.get(id) == Some(read) {
            return false;
        }
        self.heard.insert(id.to_owned(), read.clone());
        self.loaded.get(id).is_some_and(|held| held != read)
    }

    /// Every prefab held, by asset ID.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &PrefabDocument)> {
        self.loaded.iter().map(|(id, prefab)| (id.as_str(), prefab))
    }

    /// The asset ID a prefab file on disk is placed by, if it is under the
    /// scene's directory.
    #[must_use]
    pub fn id_for(&self, path: &Path) -> Option<String> {
        let relative = path.strip_prefix(self.root.as_ref()?).ok()?;
        let id = relative.to_string_lossy().replace('\\', "/");
        id.ends_with(PREFAB_SUFFIX).then_some(id)
    }

    /// Where the prefab placed as `id` is on disk.
    #[must_use]
    pub fn path_for(&self, id: &str) -> Option<PathBuf> {
        self.root.as_ref().map(|root| root.join(id))
    }

    fn read(&self, id: &str) -> Result<PrefabDocument, String> {
        let path = self
            .path_for(id)
            .ok_or_else(|| format!("{id} cannot be found: this scene has no directory yet"))?;
        super::load(&path)
    }
}

impl PrefabLibrary for ScenePrefabs {
    fn prefab(&self, source: &str) -> Option<&PrefabDocument> {
        self.loaded.get(source)
    }
}

fn placed_by(entities: &[SceneEntity]) -> impl Iterator<Item = String> + '_ {
    entities
        .iter()
        .filter_map(|entity| entity.prefab.as_ref())
        .map(|instance| instance.source.clone())
}
