//! Reading a project from its directory: the player's source natively.
//!
//! Everything under `assets/` and every scene `sindri.toml` names, decoded
//! the way the browser loader decodes what it fetches, so the two sources
//! hand the player the same thing.

use std::collections::BTreeMap;
use std::path::Path;

use sindri_assets::{
    AssetBytes, AssetDecoder, AudioAssetDecoder, FontAssetDecoder, TextureAssetDecoder,
};
use sindri_core::{
    AssetId, PREFAB_SUFFIX, PROFILE_SUFFIX, PrefabDocument, ProfileDocument, SceneDocument,
    SpriteSheetDocument, TileSetDocument,
};
use sindri_decay::{PrefabSources, ProfileSources, ScriptSources};
use sindri_runtime::project::{files_under, manifest};

use crate::error::PlayerError;
use crate::project::ProjectAssets;

/// The sound files a project may hold, by the suffix its asset ID ends with.
const AUDIO_SUFFIXES: [&str; 3] = [".ogg", ".wav", ".mp3"];

fn text(id: &str, bytes: Vec<u8>) -> Result<String, PlayerError> {
    String::from_utf8(bytes).map_err(|error| PlayerError::Project(format!("{id}: {error}")))
}

fn named<E: std::fmt::Display>(id: &str) -> impl FnOnce(E) -> PlayerError + '_ {
    move |error| PlayerError::Project(format!("{id}: {error}"))
}

fn asset_id(id: &str) -> Result<AssetId, PlayerError> {
    AssetId::new(id.to_owned()).map_err(named(id))
}

/// Reads the project at `project`.
///
/// # Errors
/// A manifest, scene, script or asset that will not read or decode, named.
pub(crate) fn read(project: &Path) -> Result<ProjectAssets, PlayerError> {
    let assets = project.join("assets");
    let (entry, sheet_ids, others) = manifest(project).map_err(PlayerError::Project)?;
    let read_scene = |path: &Path| -> Result<SceneDocument, PlayerError> {
        let shown = path.display().to_string();
        let json = std::fs::read_to_string(path).map_err(named(&shown))?;
        SceneDocument::from_json(&json).map_err(named(&shown))
    };
    let mut scenes = vec![(entry.clone(), read_scene(&assets.join(&entry))?)];
    for path in others {
        let name = Path::new(&path)
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| PlayerError::Project(format!("{path}: not a scene file")))?
            .to_owned();
        scenes.push((name, read_scene(&project.join(&path))?));
    }

    let mut count = scenes.len();
    let mut scripts = ScriptSources::new();
    for (id, bytes) in files_under(&assets, ".decay") {
        let source = text(&id, bytes)?;
        scripts.insert(id, source);
        count += 1;
    }
    let mut prefabs = PrefabSources::new();
    for (id, bytes) in files_under(&assets, PREFAB_SUFFIX) {
        let prefab = PrefabDocument::from_json(&text(&id, bytes)?).map_err(named(&id))?;
        prefabs.insert(id, prefab);
        count += 1;
    }
    let mut profiles = ProfileSources::new();
    for (id, bytes) in files_under(&assets, PROFILE_SUFFIX) {
        let profile = ProfileDocument::from_json(&text(&id, bytes)?).map_err(named(&id))?;
        profiles.insert(id, profile);
        count += 1;
    }
    let mut textures = Vec::new();
    for (id, bytes) in files_under(&assets, ".png") {
        let asset = asset_id(&id)?;
        textures.push((
            asset.clone(),
            TextureAssetDecoder.decode(AssetBytes::new(asset, bytes))?,
        ));
    }
    let mut fonts = Vec::new();
    for suffix in [".ttf", ".otf"] {
        for (id, bytes) in files_under(&assets, suffix) {
            let asset = asset_id(&id)?;
            fonts.push((
                asset.clone(),
                FontAssetDecoder.decode(AssetBytes::new(asset, bytes))?,
            ));
        }
    }
    let mut audio = Vec::new();
    for suffix in AUDIO_SUFFIXES {
        for (id, bytes) in files_under(&assets, suffix) {
            let asset = asset_id(&id)?;
            audio.push((
                asset.clone(),
                AudioAssetDecoder.decode(AssetBytes::new(asset, bytes))?,
            ));
        }
    }
    let mut sheets = BTreeMap::new();
    for (id, bytes) in files_under(&assets, ".sheet") {
        let sheet = SpriteSheetDocument::from_json(&text(&id, bytes)?).map_err(named(&id))?;
        sheets.insert(id, sheet);
    }
    let mut tile_sets = Vec::new();
    for (id, bytes) in files_under(&assets, ".tileset") {
        let tile_set = TileSetDocument::from_json(&text(&id, bytes)?).map_err(named(&id))?;
        tile_sets.push((asset_id(&id)?, tile_set));
    }
    let mut weave_sources = BTreeMap::new();
    for (id, bytes) in files_under(&assets, ".weave") {
        let source = text(&id, bytes)?;
        weave_sources.insert(id, source);
    }
    let mut stylesheets = Vec::new();
    for id in sheet_ids {
        stylesheets.push(
            weave::compose(&id, &weave_sources)
                .map_err(|error| PlayerError::Weave(error.to_string()))?,
        );
    }
    count += textures.len() + fonts.len() + audio.len() + sheets.len() + tile_sets.len();
    count += stylesheets.len();
    Ok(ProjectAssets {
        scenes,
        scripts,
        prefabs,
        profiles,
        textures,
        fonts,
        audio,
        sheets,
        tile_sets,
        stylesheets,
        asset_count: count,
    })
}
