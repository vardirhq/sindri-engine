//! Headless hosts use the same checked material sources as rendered players.
use super::{files_under, text};
use sindri_core::{PROFILE_SUFFIX, ProfileDocument};
use sindri_decay::ProfileSources;
use sindri_scene::{RenderMaterialSources, SceneExtractor};
use std::path::Path;

pub(super) fn load(assets: &Path, scene: &mut SceneExtractor) -> Result<ProfileSources, String> {
    let mut profiles = ProfileSources::new();
    for (id, bytes) in files_under(assets, PROFILE_SUFFIX) {
        let profile = ProfileDocument::from_json(&text(&id, bytes)?)
            .map_err(|error| format!("{id}: {error}"))?;
        profiles.insert(id, profile);
    }
    scene.set_materials(
        RenderMaterialSources::from_profiles(
            profiles
                .ids()
                .filter_map(|id| profiles.get(id).map(|p| (id, p))),
        )
        .map_err(|error| error.to_string())?,
    );
    Ok(profiles)
}
