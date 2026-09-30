//! The scene the editor has open, and where it came from.
//!
//! The editor used to author a copy of the demo scene compiled into the binary,
//! which made saving meaningless: there was nowhere for a save to go, so editing
//! a transform and reopening could not be the same thing twice.
//!
//! A scene is a file. This owns the path, the document as it was last agreed
//! with disk, and the two operations that keep them in step.

use std::{
    collections::BTreeSet,
    fmt,
    path::{Path, PathBuf},
};

use sindri_core::{SceneDocument, SceneJsonError, SceneMigrator, World, WorldError};
use thiserror::Error;

use crate::prefab::ScenePrefabs;
use crate::prefab::document::{asset_root_for, is_prefab_path, prefab_text, read_as_scene};
use crate::prefab::missing::{restore, stand_in};

/// A scene document together with the file it belongs to.
///
/// The file may be a prefab instead, opened to be edited: it is worked on as a
/// scene and written back as a prefab. See `prefab::document`.
#[derive(Clone, Debug)]
pub struct SceneFile {
    path: Option<PathBuf>,
    document: SceneDocument,
    /// The prefabs its instances are made from, and written back against.
    prefabs: ScenePrefabs,
    /// A path whose folder is where this document's asset IDs resolve: the
    /// file itself for a scene, and its scene's folder for a prefab.
    anchor: Option<PathBuf>,
    /// Prefabs this document places that could not be read, and why. Their
    /// instances are placeholders until the scene is opened again.
    missing: Vec<(String, String)>,
}

impl SceneFile {
    /// Opens a scene from disk.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, SceneFileError> {
        let path = path.as_ref();
        let text = std::fs::read_to_string(path).map_err(|source| SceneFileError::Read {
            path: path.display().to_string(),
            source,
        })?;
        let (document, anchor) = if is_prefab_path(path) {
            let anchor = asset_root_for(path).join(path.file_name().unwrap_or_default());
            (
                read_as_scene(&text).map_err(SceneFileError::Prefab)?,
                anchor,
            )
        } else {
            // Migrated rather than parsed strictly: an editor that cannot open
            // a scene written by an older Sindri is an editor that loses work.
            let document = SceneDocument::from_json_migrated(&text, &SceneMigrator::builtin())?;
            (document, path.to_path_buf())
        };
        let mut prefabs = ScenePrefabs::beside(Some(&anchor));
        let missing = prefabs.read_available(&document.entities);
        let unusable = unusable(&prefabs, &missing);
        let document = if unusable.is_empty() {
            document
        } else {
            stand_in(&document, &unusable)
        };
        Ok(Self {
            path: Some(path.to_path_buf()),
            document,
            prefabs,
            anchor: Some(anchor),
            missing,
        })
    }

    /// Prefabs this document places that could not be read, and why.
    pub fn missing(&self) -> &[(String, String)] {
        &self.missing
    }

    /// Whether the open document is a prefab rather than a scene.
    pub fn is_prefab(&self) -> bool {
        self.path.as_deref().is_some_and(is_prefab_path)
    }

    /// A path in the folder this document's asset IDs resolve against.
    ///
    /// What textures, scripts and the project browser are rooted by. The
    /// file's own path for a scene; for a prefab, one beside the scene it
    /// would be placed in.
    pub fn anchor(&self) -> Option<&Path> {
        self.anchor.as_deref()
    }

    /// A scene with no file behind it.
    ///
    /// Used when the editor is started somewhere the default scene is not, so
    /// it still opens and says why rather than refusing to start. Saving is
    /// unavailable until a path exists, which the interface reflects.
    pub fn detached(document: SceneDocument) -> Self {
        Self {
            path: None,
            document,
            prefabs: ScenePrefabs::default(),
            anchor: None,
            missing: Vec::new(),
        }
    }

    /// The prefabs this scene's instances are made from.
    pub const fn prefabs(&self) -> &ScenePrefabs {
        &self.prefabs
    }

    /// The same, to read another prefab into or bring one up to date.
    pub const fn prefabs_mut(&mut self) -> &mut ScenePrefabs {
        &mut self.prefabs
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// The document as last read from or written to disk.
    ///
    /// This is what "reset to authored" resets to, so it follows a save: after
    /// saving, the file is what the scene was authored as.
    pub const fn document(&self) -> &SceneDocument {
        &self.document
    }

    /// The file's name, for an interface that has one line to spend on it.
    pub fn label(&self) -> String {
        self.path.as_ref().map_or_else(
            || "untitled scene".to_owned(),
            |path| {
                path.file_name().map_or_else(
                    || path.display().to_string(),
                    |name| name.to_string_lossy().into_owned(),
                )
            },
        )
    }

    /// Writes a world back to the file it was opened from.
    ///
    /// The bytes are canonical, so saving a scene nobody edited reproduces the
    /// file exactly and a review sees only what changed.
    pub fn save(&mut self, world: &World) -> Result<(), SceneFileError> {
        let path = self.path.clone().ok_or(SceneFileError::NoPath)?;
        self.save_as(&path, world)
    }

    /// Writes a world to a path, and adopts it.
    ///
    /// The adoption happens after the write rather than before it, so a save
    /// that fails leaves the scene attached to the file it was attached to. A
    /// detached scene — one the editor opened with no file behind it — becomes
    /// a real file this way, which is the only way it ever could.
    pub fn save_as(&mut self, path: &Path, world: &World) -> Result<(), SceneFileError> {
        let document = world.to_scene_with(&self.prefabs)?;
        // Placeholders go back to being the instances they stand in for.
        let mut written = document.clone();
        restore(&mut written);
        if is_prefab_path(path) {
            let text = prefab_text(&written).map_err(SceneFileError::Prefab)?;
            std::fs::write(path, text).map_err(|source| SceneFileError::Write {
                path: path.display().to_string(),
                source,
            })?;
        } else {
            write_scene(path, &written)?;
            self.anchor = Some(path.to_path_buf());
            self.prefabs.move_beside(path);
        }
        self.path = Some(path.to_path_buf());
        self.document = document;
        Ok(())
    }

    /// Writes a document to a path nothing is open on yet.
    ///
    /// What New Scene uses. The file is written and then opened through the
    /// ordinary path rather than adopted in memory, so a new scene proves it
    /// loads before anyone starts working in it.
    pub fn create(path: &Path, document: &SceneDocument) -> Result<(), SceneFileError> {
        write_scene(path, document)
    }

    /// Follows the file to a new path without writing anything.
    ///
    /// What renaming the open scene in the project browser needs: the editor
    /// holds the path it saves to, so a rename the editor was not told about
    /// would have the next save write the scene back under its old name and
    /// leave two of them on disk.
    pub fn adopt(&mut self, path: &Path) {
        self.path = Some(path.to_path_buf());
        if let Some(anchor) = &mut self.anchor {
            anchor.set_file_name(path.file_name().unwrap_or_default());
        }
        if !is_prefab_path(path) {
            self.anchor = Some(path.to_path_buf());
            self.prefabs.move_beside(path);
        }
    }

    /// Re-reads the file, discarding whatever the editor had in memory.
    pub fn reload(&mut self) -> Result<(), SceneFileError> {
        let path = self.path.clone().ok_or(SceneFileError::NoPath)?;
        *self = Self::open(path)?;
        Ok(())
    }
}

/// The prefabs whose instances cannot be made: the ones that could not be
/// read, and every one that places one of those, however deep.
fn unusable(prefabs: &ScenePrefabs, missing: &[(String, String)]) -> BTreeSet<String> {
    let mut unusable: BTreeSet<String> = missing.iter().map(|(id, _)| id.clone()).collect();
    loop {
        let before = unusable.len();
        for (id, prefab) in prefabs.iter() {
            let places_one = prefab.entities.iter().any(|entity| {
                entity
                    .prefab
                    .as_ref()
                    .is_some_and(|instance| unusable.contains(&instance.source))
            });
            if places_one {
                unusable.insert(id.to_owned());
            }
        }
        if unusable.len() == before {
            return unusable;
        }
    }
}

/// The path a scene chosen in a save dialog is actually written to.
///
/// A scene is `*.scene` and nothing else: that is what the project browser
/// recognises and what `SceneFile::open` is offered in a file dialog. Someone
/// typing "level" into a save box means a scene called level, and writing that
/// verbatim would produce one the browser lists as a plain file and cannot
/// reopen.
pub fn scene_path(chosen: &Path) -> PathBuf {
    let name = chosen
        .file_name()
        .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
    if name.to_lowercase().ends_with(SCENE_SUFFIX) {
        return chosen.to_path_buf();
    }
    let stem = name.strip_suffix(".json").unwrap_or(&name);
    chosen.with_file_name(format!("{stem}{SCENE_SUFFIX}"))
}

/// What a scene file is called, and what the browser reads a scene by.
const SCENE_SUFFIX: &str = ".scene";

/// Writes a document as canonical JSON.
fn write_scene(path: &Path, document: &SceneDocument) -> Result<(), SceneFileError> {
    let text = document.to_canonical_json()?;
    std::fs::write(path, text).map_err(|source| SceneFileError::Write {
        path: path.display().to_string(),
        source,
    })
}

impl fmt::Display for SceneFile {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.path {
            Some(path) => write!(formatter, "{}", path.display()),
            None => formatter.write_str("untitled scene"),
        }
    }
}

#[derive(Debug, Error)]
pub enum SceneFileError {
    #[error("could not read {path}: {source}")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("could not write {path}: {source}")]
    Write {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("this scene has no file to save to")]
    NoPath,
    #[error("{0}")]
    Prefab(String),
    #[error(transparent)]
    Scene(#[from] SceneJsonError),
    #[error(transparent)]
    World(#[from] WorldError),
}

#[cfg(test)]
mod tests {
    use sindri_core::{EntityData, SceneEntity, SceneEntityId, Transform3D};

    use super::*;

    fn authored_json() -> String {
        let mut entity = SceneEntity::new(SceneEntityId::new("cube").unwrap());
        entity.transform_3d = Some(Transform3D::default());
        let mut document = SceneDocument::default();
        document.entities.push(entity);
        document.to_canonical_json().unwrap()
    }

    fn written(directory: &Path, text: &str) -> PathBuf {
        let path = directory.join("level.scene");
        std::fs::write(&path, text).unwrap();
        path
    }

    /// A save dialog takes a name, not an extension, so the suffix is the
    /// editor's business: a scene written as `level.json` would be listed by
    /// the project browser as a plain file it cannot open.
    #[test]
    fn a_chosen_name_becomes_a_scene_file_name() {
        let cases = [
            ("level", "level.scene"),
            ("level.json", "level.scene"),
            ("level.scene", "level.scene"),
            // Already a scene, whatever case it was typed in.
            ("Level.SCENE", "Level.SCENE"),
        ];
        for (chosen, expected) in cases {
            assert_eq!(
                scene_path(&PathBuf::from("/project").join(chosen)),
                PathBuf::from("/project").join(expected),
                "{chosen} was not turned into a scene file name"
            );
        }
    }

    /// A scene with no file behind it gains one, and what it writes reopens.
    ///
    /// The case the editor could not answer at all: started where the default
    /// scene is not, it opened detached with Save disabled and no way to make
    /// a file to save into.
    #[test]
    fn a_detached_scene_can_be_saved_somewhere() {
        let directory = tempfile::tempdir().unwrap();
        let mut file = SceneFile::detached(SceneDocument::default());
        assert!(file.path().is_none() && file.save(&World::default()).is_err());

        let world = World::from_scene(&SceneDocument::from_json(&authored_json()).unwrap())
            .unwrap()
            .world;
        let path = directory.path().join("forked.scene");
        file.save_as(&path, &world).unwrap();

        assert_eq!(file.path(), Some(path.as_path()));
        assert_eq!(SceneFile::open(&path).unwrap().document(), file.document());
    }

    #[test]
    fn a_scene_edited_and_saved_reopens_as_it_was_left() {
        let directory = tempfile::tempdir().unwrap();
        let path = written(directory.path(), &authored_json());

        let mut file = SceneFile::open(&path).unwrap();
        let mut world = World::from_scene(file.document()).unwrap().world;
        let entity = world.entities().next().map(|(entity, _)| entity).unwrap();
        world.get_mut(entity).unwrap().transform_3d = Some(Transform3D {
            position: [1.5, -2.0, 3.25],
            ..Transform3D::default()
        });
        let edited = world.to_scene().unwrap();
        assert_ne!(
            &edited,
            file.document(),
            "the edit should have changed the document"
        );
        file.save(&world).unwrap();

        let reopened = SceneFile::open(&path).unwrap();
        assert_eq!(
            reopened.document(),
            &edited,
            "the edit did not survive the round trip through the file"
        );
    }

    /// Saving is only safe to offer if an untouched scene comes back unchanged.
    /// Canonical output makes that true; this is what proves it stays true.
    #[test]
    fn saving_an_unedited_scene_leaves_the_file_byte_for_byte_identical() {
        let directory = tempfile::tempdir().unwrap();
        let original = authored_json();
        let path = written(directory.path(), &original);

        let mut file = SceneFile::open(&path).unwrap();
        let world = World::from_scene(file.document()).unwrap().world;
        file.save(&world).unwrap();

        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
    }

    #[test]
    fn reloading_discards_what_was_never_saved() {
        let directory = tempfile::tempdir().unwrap();
        let path = written(directory.path(), &authored_json());
        let mut file = SceneFile::open(&path).unwrap();

        let mut world = World::from_scene(file.document()).unwrap().world;
        world.spawn(EntityData::default());
        assert_eq!(world.len(), 2);

        file.reload().unwrap();
        let reloaded = World::from_scene(file.document()).unwrap().world;
        assert_eq!(
            reloaded.len(),
            1,
            "reload should have dropped the new entity"
        );
    }

    #[test]
    fn a_detached_scene_reports_that_it_has_nowhere_to_save() {
        let document = SceneDocument::from_json(&authored_json()).unwrap();
        let mut file = SceneFile::detached(document);
        let world = World::from_scene(file.document()).unwrap().world;

        assert!(file.path().is_none());
        assert!(matches!(file.save(&world), Err(SceneFileError::NoPath)));
    }

    #[test]
    fn a_missing_file_names_itself_in_the_error() {
        let error = SceneFile::open("definitely/not/here.scene")
            .expect_err("opening a missing scene fails");
        assert!(
            error.to_string().contains("definitely/not/here.scene"),
            "{error}"
        );
    }
}
