//! The scene the editor has open, and where it came from.
//!
//! The editor used to author a copy of the demo scene compiled into the binary,
//! which made saving meaningless: there was nowhere for a save to go, so editing
//! a transform and reopening could not be the same thing twice.
//!
//! A scene is a file. This owns the path, the document as it was last agreed
//! with disk, and the two operations that keep them in step.

use std::{
    fmt,
    path::{Path, PathBuf},
};

use sindri_core::{SCENE_SUFFIX, SceneDocument, SceneJsonError, SceneMigrator, World, WorldError};
use thiserror::Error;

/// A scene document together with the file it belongs to.
#[derive(Clone, Debug)]
pub struct SceneFile {
    path: Option<PathBuf>,
    document: SceneDocument,
}

impl SceneFile {
    /// Opens a scene from disk.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, SceneFileError> {
        let path = path.as_ref();
        let text = std::fs::read_to_string(path).map_err(|source| SceneFileError::Read {
            path: path.display().to_string(),
            source,
        })?;
        // Migrated rather than parsed strictly: an editor that cannot open a
        // scene written by an older Sindri is an editor that loses work.
        Ok(Self {
            path: Some(path.to_path_buf()),
            document: SceneDocument::from_json_migrated(&text, &SceneMigrator::builtin())?,
        })
    }

    /// A scene with no file behind it.
    ///
    /// Used when the editor is started somewhere the default scene is not, so
    /// it still opens and says why rather than refusing to start. Saving is
    /// unavailable until a path exists, which the interface reflects.
    pub const fn detached(document: SceneDocument) -> Self {
        Self {
            path: None,
            document,
        }
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// The document as last read from or written to disk.
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

    pub fn save(&mut self, world: &World) -> Result<(), SceneFileError> {
        let path = self.path.clone().ok_or(SceneFileError::NoPath)?;
        self.save_as(&path, world)
    }

    pub fn save_as(&mut self, path: &Path, world: &World) -> Result<(), SceneFileError> {
        let document = world.to_scene()?;
        write_scene(path, &document)?;
        self.path = Some(path.to_path_buf());
        self.document = document;
        Ok(())
    }

    pub fn create(path: &Path, document: &SceneDocument) -> Result<(), SceneFileError> {
        write_scene(path, document)
    }

    pub fn adopt(&mut self, path: &Path) {
        self.path = Some(path.to_path_buf());
    }

    pub fn reload(&mut self) -> Result<(), SceneFileError> {
        let path = self.path.clone().ok_or(SceneFileError::NoPath)?;
        *self = Self::open(path)?;
        Ok(())
    }
}

/// The path a scene chosen in a save dialog is actually written to.
///
/// New scenes use Sindri's native `.scene` extension. Choosing an old
/// `.scene.json` name migrates that new file to the native name; existing
/// legacy files remain readable when opened directly.
pub fn scene_path(chosen: &Path) -> PathBuf {
    let name = chosen
        .file_name()
        .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
    if name.to_lowercase().ends_with(SCENE_SUFFIX) {
        return chosen.to_path_buf();
    }
    let lower = name.to_lowercase();
    let stem = if lower.ends_with(".scene.json") {
        &name[..name.len() - ".scene.json".len()]
    } else if lower.ends_with(".json") {
        &name[..name.len() - ".json".len()]
    } else {
        &name
    };
    chosen.with_file_name(format!("{stem}{SCENE_SUFFIX}"))
}

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
        let path = directory.join("scene.scene");
        std::fs::write(&path, text).unwrap();
        path
    }

    #[test]
    fn a_chosen_name_becomes_a_scene_file_name() {
        let cases = [
            ("level", "level.scene"),
            ("level.json", "level.scene"),
            ("level.scene.json", "level.scene"),
            ("level.scene", "level.scene"),
            ("Level.Scene", "Level.Scene"),
        ];
        for (chosen, expected) in cases {
            assert_eq!(
                scene_path(&PathBuf::from("/project").join(chosen)),
                PathBuf::from("/project").join(expected),
                "{chosen} was not turned into a scene file name"
            );
        }
    }

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
    fn a_legacy_scene_still_opens() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("old.scene.json");
        std::fs::write(&path, authored_json()).unwrap();
        assert!(SceneFile::open(path).is_ok());
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
        assert_ne!(&edited, file.document());
        file.save(&world).unwrap();

        let reopened = SceneFile::open(&path).unwrap();
        assert_eq!(reopened.document(), &edited);
    }

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
        assert_eq!(reloaded.len(), 1);
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
        assert!(error.to_string().contains("definitely/not/here.scene"));
    }
}
