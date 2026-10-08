//! The session editor Play starts: the shared runtime session every build
//! steps, assembled from what the editor has loaded for the open scene.
//!
//! Kept apart from the editor's window so that what Play plays can be
//! compared with what a build plays without opening one: the parity tests
//! start this beside `sindri_runtime::ProjectRun` and step both.

use std::path::Path;

use sindri_core::{ComponentSchemaRegistry, LoadedScenes, SceneDocument, World};
use sindri_runtime::Session;
use sindri_scene::TileSetBindings;
use weave::{Stylesheet, Viewport};

use crate::project::Project;
use crate::scripts::SceneScripts;

/// What the editor has loaded that a run plays with.
pub struct PlaySources<'a> {
    /// The schemas the open scene is read with.
    pub components: &'a ComponentSchemaRegistry,
    /// Every script, prefab and profile the project holds.
    pub scripts: &'a SceneScripts,
    /// The block sets the scene's volumes name.
    pub tile_sets: &'a TileSetBindings,
    /// The project's composed stylesheets.
    pub sheets: &'a [Stylesheet],
}

/// Where the open scene is: the project it belongs to and its own file.
/// Either is `None` for a scene outside a project, which has nowhere for
/// `Scene.go` to go.
#[derive(Clone, Copy, Debug, Default)]
pub struct OpenScene<'a> {
    pub project: Option<&'a Path>,
    pub file: Option<&'a Path>,
}

/// A session for `world`, the open scene, settled on its stylesheets at
/// `viewport` as a build starts.
///
/// # Errors
/// A project scene that will not parse, or a stylesheet that will not apply.
pub fn start(
    sources: &PlaySources<'_>,
    world: &mut World,
    open: OpenScene<'_>,
    viewport: Viewport,
) -> Result<Session, String> {
    let mut session = Session::with_sources(
        sources.components.clone(),
        sources.scripts.sources().clone(),
    )
    .with_prefabs(sources.scripts.prefabs().clone())
    .with_profiles(sources.scripts.profiles().clone())
    .with_tile_sets(sources.tile_sets.clone())
    .with_styles(sources.sheets.to_vec());
    if let Some((scenes, loaded)) = playable_scenes(world, open)? {
        session = session.with_scenes(scenes, loaded);
    }
    session
        .settle_styles(world, viewport)
        .map_err(|error| error.to_string())?;
    Ok(session)
}

/// The scenes a script's `Scene.go` can reach, named as a build names
/// them — by file name — with the open one adopted as the one playing.
#[allow(clippy::type_complexity)]
fn playable_scenes(
    world: &mut World,
    open: OpenScene<'_>,
) -> Result<Option<(Vec<(String, SceneDocument)>, LoadedScenes)>, String> {
    let (Some(root), Some(open)) = (open.project, open.file) else {
        return Ok(None);
    };
    let Ok(project) = Project::open(root) else {
        return Ok(None);
    };
    let name = |path: &Path| {
        path.file_name()
            .map(|name| name.to_string_lossy().into_owned())
    };
    let mut scenes = Vec::new();
    for path in project.scenes() {
        let Some(named) = name(&path) else {
            continue;
        };
        // The open scene is the world itself, edits and all, rather than
        // what its file said when it was last saved.
        if path == open {
            continue;
        }
        // A declared scene whose file is missing is one a door cannot
        // reach, which `Scene.go` says when it is asked; it does not stop
        // the run.
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let document = SceneDocument::from_json(&text)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        scenes.push((named, document));
    }
    let Some(current) = name(open) else {
        return Ok(None);
    };
    let mut loaded = LoadedScenes::new();
    loaded
        .adopt(world, &current)
        .map_err(|error| error.to_string())?;
    // The open scene is listed too, so a door back to it finds it.
    if let Ok(text) = std::fs::read_to_string(open)
        && let Ok(document) = SceneDocument::from_json(&text)
    {
        scenes.push((current, document));
    }
    Ok(Some((scenes, loaded)))
}
