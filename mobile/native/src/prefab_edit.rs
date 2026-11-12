//! A prefab file opened in the object workspace beside the open scene.
//!
//! The workspace edits one object in its own units, which is exactly what a
//! prefab is: one root and what hangs from it. So a prefab opens as a small
//! document of its own, with its own world and undo history, while the scene
//! stays open behind it. Saving writes the prefab through `sindri-authoring`,
//! the desktop editor's own prefab path, and brings every instance of it in
//! the open scene up to date as one undo step in the scene's history, keeping
//! what each instance overrides; the host then saves the scene as it saves
//! every edit.

use sindri_authoring::{SceneFile, load_world, prefab};
use sindri_core::{EntityId, World};

use crate::{Editing, OpenedScene};

/// A prefab being edited: the file, the world it was built into, and the
/// editing session over it, with its root selected.
pub struct PrefabSession {
    /// The prefab's ID as the open scene places it: its path from the
    /// scene's folder.
    pub id: String,
    pub file: SceneFile,
    pub world: World,
    pub editing: Editing,
}

impl PrefabSession {
    /// Opens a prefab by its project asset ID, as the Add sheet lists it, or
    /// by the ID the open scene places it by, as an instance names it. In a
    /// project whose scenes sit in `assets/` the two are the same.
    ///
    /// # Errors
    /// A prefab the project does not hold, one the scene cannot place, one
    /// that will not read, or one whose components do not validate, named.
    pub fn open(opened: &OpenedScene, asset: &str) -> Result<Self, String> {
        let path = if opened.assets.prefabs.ids().any(|id| id == asset) {
            sindri_runtime::project::assets_root(&opened.project).join(asset)
        } else {
            opened
                .file
                .prefabs()
                .path_for(asset)
                .filter(|path| path.is_file())
                .ok_or_else(|| format!("{asset} is not in the project"))?
        };
        let id = opened
            .file
            .prefabs()
            .id_for(&path)
            .ok_or_else(|| format!("{asset} cannot be placed from this scene"))?;
        let file = SceneFile::open(&path).map_err(|error| format!("{asset}: {error}"))?;
        let world =
            load_world(&opened.extractor, &file).map_err(|error| format!("{asset}: {error}"))?;
        let root = root_of(&world).ok_or_else(|| format!("{asset} has no root"))?;
        let mut editing = Editing::new(&world, &opened.extractor)?;
        editing.selected = Some(root);
        Ok(Self {
            id,
            file,
            world,
            editing,
        })
    }

    /// Writes the prefab, and brings the open scene's instances of it up to
    /// date as one undo step there. Answers whether the scene changed, which
    /// the caller then saves the way it saves every scene edit, through its
    /// recovery copy.
    ///
    /// # Errors
    /// A prefab that cannot be written, or instances that cannot be brought
    /// up to date. A prefab that was written stays written.
    pub fn save(&mut self, opened: &mut OpenedScene, scene: &mut Editing) -> Result<bool, String> {
        self.file
            .save(&self.world, opened.extractor.components())
            .map_err(|error| format!("{}: {error}", self.id))?;
        self.editing.saved();
        let path = self
            .file
            .path()
            .ok_or_else(|| format!("{} has no file", self.id))?
            .to_path_buf();
        let document = prefab::load(&path)?;
        let buffer = prefab::follow(
            &opened.world,
            opened.file.prefabs_mut(),
            vec![(self.id.clone(), document)],
        )?;
        if buffer.is_empty() {
            return Ok(false);
        }
        scene.history.break_merge_run();
        let name = self.id.rsplit('/').next().unwrap_or(&self.id);
        scene
            .history
            .apply(
                buffer.into_transaction(format!("Update instances of {name}")),
                &mut opened.world,
            )
            .map_err(|error| error.to_string())?;
        scene.invalidate_outline();
        Ok(true)
    }
}

/// The prefab's one root: the entity with no parent.
fn root_of(world: &World) -> Option<EntityId> {
    world
        .entities()
        .find(|(_, data)| data.parent.is_none())
        .map(|(entity, _)| entity)
}
