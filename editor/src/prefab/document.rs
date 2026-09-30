//! A prefab opened as the editor's document, rather than placed in one.
//!
//! A prefab is a scene fragment with one root, so the editor edits it the way
//! it edits a scene: the same world, hierarchy, inspector and gizmos. Two
//! things differ. It is read and written as a prefab, with its own format
//! version and its one-root rule. And its asset IDs do not resolve against its
//! own folder: `prefabs/coin.prefab` names `textures/coin.png` the way the scene
//! that places it does, so they resolve against that scene's folder.

use std::path::{Path, PathBuf};

use sindri_core::{
    PREFAB_FORMAT_VERSION, PREFAB_SUFFIX, PrefabDocument, SCENE_FORMAT_VERSION, SceneDocument,
};

use crate::project::Project;

/// Whether `path` names a prefab.
#[must_use]
pub fn is_prefab_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.to_lowercase().ends_with(PREFAB_SUFFIX))
}

/// The folder a prefab's asset IDs resolve against.
///
/// The folder of its project's main scene, which is where every scene that
/// could place it resolves too. Without a project, the nearest folder above it
/// that holds a scene; and failing that its own.
#[must_use]
pub fn asset_root_for(prefab: &Path) -> PathBuf {
    let own = prefab.parent().unwrap_or(prefab).to_path_buf();
    if let Some(root) = crate::project::manifest::root_for(prefab)
        && let Ok(project) = Project::open(&root)
        && let Some(scene) = project.main_scene()
        && let Some(folder) = scene.parent()
        && prefab.starts_with(folder)
    {
        return folder.to_path_buf();
    }
    own.ancestors()
        .find(|folder| holds_a_scene(folder))
        .map_or(own.clone(), Path::to_path_buf)
}

fn holds_a_scene(folder: &Path) -> bool {
    std::fs::read_dir(folder).is_ok_and(|entries| {
        entries.flatten().any(|entry| {
            entry
                .path()
                .extension()
                .is_some_and(|found| found == "scene")
        })
    })
}

/// A prefab's text, as the scene document the editor works on.
///
/// # Errors
/// A prefab that will not parse or is not valid, as its own error says.
pub fn read_as_scene(text: &str) -> Result<SceneDocument, String> {
    let prefab = PrefabDocument::from_json(text).map_err(|error| error.to_string())?;
    Ok(SceneDocument {
        format_version: SCENE_FORMAT_VERSION,
        metadata: prefab.metadata,
        entities: prefab.entities,
    })
}

/// The canonical text of a prefab holding what `scene` holds.
///
/// # Errors
/// A document that is not a prefab — above all one with more than one root,
/// which a prefab cannot have and a scene being edited easily can.
pub fn prefab_text(scene: &SceneDocument) -> Result<String, String> {
    PrefabDocument {
        format_version: PREFAB_FORMAT_VERSION,
        metadata: scene.metadata.clone(),
        entities: scene.entities.clone(),
    }
    .to_canonical_json()
    .map_err(|error| format!("this is not a prefab any more: {error}"))
}
