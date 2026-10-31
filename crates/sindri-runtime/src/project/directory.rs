//! Reading a project from its directory: the native player's source, and an
//! authoring host's.
//!
//! Everything under `assets/` and every scene `sindri.toml` names, decoded
//! the way the browser loader decodes what it fetches, so every source hands
//! its host the same thing.

use std::collections::BTreeMap;
use std::path::Path;

use super::ProjectAssets;
use super::{assets_root, files_under, manifest};
use sindri_assets::{
    AssetBytes, AssetDecoder, AudioAssetDecoder, FontAssetDecoder, ModelAssetDecoder,
    TextureAssetDecoder,
};
use sindri_core::{
    AssetId, PREFAB_SUFFIX, PROFILE_SUFFIX, PrefabDocument, ProfileDocument, SceneDocument,
    SpriteSheetDocument, TileSetDocument,
};
use sindri_decay::{PrefabSources, ProfileSources, ScriptSources};

/// The sound files a project may hold, by the suffix its asset ID ends with.
const AUDIO_SUFFIXES: [&str; 3] = [".ogg", ".wav", ".mp3"];

fn text(id: &str, bytes: Vec<u8>) -> Result<String, String> {
    String::from_utf8(bytes).map_err(|error| format!("{id}: {error}"))
}

fn named<E: std::fmt::Display>(id: &str) -> impl FnOnce(E) -> String + '_ {
    move |error| format!("{id}: {error}")
}

fn asset_id(id: &str) -> Result<AssetId, String> {
    AssetId::new(id.to_owned()).map_err(named(id))
}

/// Every file under `assets` ending in one of `suffixes`, decoded.
fn decoded<D: AssetDecoder>(
    assets: &Path,
    suffixes: &[&str],
    decoder: &D,
) -> Result<Vec<(AssetId, D::Asset)>, String> {
    let mut found = Vec::new();
    for suffix in suffixes {
        for (id, bytes) in files_under(assets, suffix) {
            let asset = asset_id(&id)?;
            found.push((
                asset.clone(),
                decoder
                    .decode(AssetBytes::new(asset, bytes))
                    .map_err(named(&id))?,
            ));
        }
    }
    Ok(found)
}

/// Reads the project at `project`.
///
/// # Errors
/// A manifest, scene, script or asset that will not read or decode, named.
pub(super) fn read(project: &Path) -> Result<ProjectAssets, String> {
    let assets = assets_root(project);
    let (entry, sheet_ids, others) = manifest(project)?;
    let read_scene = |path: &Path| -> Result<SceneDocument, String> {
        let shown = path.display().to_string();
        let json = std::fs::read_to_string(path).map_err(named(&shown))?;
        SceneDocument::from_json(&json).map_err(named(&shown))
    };
    let mut scenes = vec![(entry.clone(), read_scene(&assets.join(&entry))?)];
    for path in others {
        let name = Path::new(&path)
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| format!("{path}: not a scene file"))?
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
    let textures = decoded(&assets, &[".png"], &TextureAssetDecoder)?;
    let models = decoded(&assets, &[".glb"], &ModelAssetDecoder)?;
    let fonts = decoded(&assets, &[".ttf", ".otf"], &FontAssetDecoder)?;
    let audio = decoded(&assets, &AUDIO_SUFFIXES, &AudioAssetDecoder)?;
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
    let mut shaders = Vec::new();
    for (id, bytes) in files_under(&assets, ".wgsl") {
        let source = text(&id, bytes)?;
        shaders.push((id, source));
    }
    let mut stylesheets = Vec::new();
    for id in sheet_ids {
        stylesheets.push(weave::compose(&id, &weave_sources).map_err(|error| {
            format!("the project's presentation could not be composed: {error}")
        })?);
    }
    count +=
        models.len() + textures.len() + fonts.len() + audio.len() + sheets.len() + tile_sets.len();
    count += stylesheets.len() + shaders.len();
    Ok(ProjectAssets {
        scenes,
        scripts,
        prefabs,
        profiles,
        models,
        textures,
        fonts,
        audio,
        sheets,
        tile_sets,
        stylesheets,
        shaders,
        asset_count: count,
    })
}
