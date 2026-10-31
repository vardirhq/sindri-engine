//! A project's scene, opened to be looked at.

use std::path::Path;

use sindri_core::{UnknownComponentPolicy, World};
use sindri_runtime::ProjectAssets;
use sindri_scene::SceneExtractor;

/// One scene of a project, built into a world, with everything the project
/// holds that drawing it needs.
///
/// The project is read by [`ProjectAssets::read`], the reader the native player
/// uses, so a texture or sheet is found by the same rules on a phone as in a
/// build. The scene is built as an authored world, with prefab instances made
/// from their prefabs and nothing played: what the editor's Scene view shows,
/// not a run.
pub struct OpenedScene {
    /// The file name the scene is known by, as `Scene.go` names it.
    pub name: String,
    pub world: World,
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
    /// components do not validate — each named.
    pub fn open(project: &Path, scene: &str) -> Result<Self, String> {
        let assets = ProjectAssets::read(project)?;
        let document = assets
            .scenes
            .iter()
            .find(|(name, _)| Self::file_name(name) == scene)
            .map(|(_, document)| document.clone())
            .ok_or_else(|| format!("the project lists no scene {scene}"))?;
        let extractor = sindri_runtime::scene_extractor()?;
        extractor
            .validate(&document, UnknownComponentPolicy::Preserve)
            .map_err(|error| format!("{scene}: {error}"))?;
        let world = World::from_scene_with(&document, &assets.prefabs)
            .map_err(|error| format!("{scene}: {error}"))?
            .world;
        Ok(Self {
            name: scene.to_owned(),
            world,
            assets,
            extractor,
        })
    }

    /// What the app shows under the scene's name.
    pub fn summary(&self) -> String {
        format!("{} · {} entities", self.name, self.world.entities().count())
    }

    /// The entry scene is listed by its path under `assets/`, the others by
    /// file name; both are matched by file name.
    fn file_name(listed: &str) -> &str {
        listed.rsplit('/').next().unwrap_or(listed)
    }
}
