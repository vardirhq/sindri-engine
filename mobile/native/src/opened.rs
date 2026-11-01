//! A project's scene, opened to be edited and saved.

use std::path::{Path, PathBuf};

use sindri_authoring::{SceneFile, load_world};
use sindri_core::World;
use sindri_runtime::ProjectAssets;
use sindri_scene::SceneExtractor;

/// One scene of a project, built into a world, with the file it came from and
/// everything the project holds that drawing it needs.
///
/// The scene is opened and saved through `sindri-authoring`, the same code the
/// desktop editor uses, so a save made on the phone opens on desktop, and the
/// reverse, without conversion: older scenes are migrated as they open, prefab
/// instances are made from their prefabs, and a save is canonical JSON that
/// writes instances back as references. The project's assets are read by
/// [`ProjectAssets::read`], the native player's reader, so a texture or sheet
/// is found by the same rules on a phone as in a build.
pub struct OpenedScene {
    /// The file name the scene is known by, as `Scene.go` names it.
    pub name: String,
    pub world: World,
    /// The file the world is read from and saved to.
    pub file: SceneFile,
    /// The project's decoded assets. A renderer takes the ones it uploads.
    pub assets: ProjectAssets,
    /// The schemas the scene is read and drawn with.
    pub extractor: SceneExtractor,
}

impl OpenedScene {
    /// Opens the scene called `scene` (a file name, such as `world.scene`) in
    /// the project at `project`.
    ///
    /// # Errors
    /// A project that will not read, a scene it does not list, or a scene whose
    /// file will not open or whose components do not validate — each named.
    pub fn open(project: &Path, scene: &str) -> Result<Self, String> {
        let path = scene_path(project, scene)?;
        let file = SceneFile::open(&path).map_err(|error| format!("{scene}: {error}"))?;
        let extractor = sindri_runtime::scene_extractor()?;
        let world = load_world(&extractor, &file).map_err(|error| format!("{scene}: {error}"))?;
        let assets = ProjectAssets::read(project)?;
        Ok(Self {
            name: scene.to_owned(),
            world,
            file,
            assets,
            extractor,
        })
    }

    /// Writes the world back to its file, as desktop's Save does.
    ///
    /// # Errors
    /// A world that cannot be written as a scene, or a file that cannot be
    /// written; the file is left as it was in either case.
    pub fn save(&mut self) -> Result<(), String> {
        self.file
            .save(&self.world, self.extractor.components())
            .map_err(|error| format!("{}: {error}", self.name))
    }

    /// What the app shows under the scene's name.
    pub fn summary(&self) -> String {
        format!("{} · {} entities", self.name, self.world.entities().count())
    }
}

/// Where the scene `scene` lives in the project at `project`, among the scenes
/// `sindri.toml` names: the main scene under `assets/`, the others by their
/// path from the project, each matched by file name as `Scene.go` names them.
fn scene_path(project: &Path, scene: &str) -> Result<PathBuf, String> {
    let (main, _, others) = sindri_runtime::project::manifest(project)?;
    std::iter::once(sindri_runtime::project::assets_root(project).join(main))
        .chain(others.iter().map(|other| project.join(other)))
        .find(|path| path.file_name().is_some_and(|name| name == scene))
        .ok_or_else(|| format!("the project lists no scene {scene}"))
}
