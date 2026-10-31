//! Everything a project is, loaded: what a host's source fills and what the
//! host installs.
//!
//! The browser player fills it from what it fetched. Natively it is read from
//! a project's directory, by the player and by an authoring host that has to
//! draw a project's scenes, so both find and decode an asset by one set of
//! rules.

use std::collections::BTreeMap;

use sindri_assets::{AudioAsset, FontAsset, ModelAsset, TextureAsset};
use sindri_core::{AssetId, SceneDocument, SpriteSheetDocument, TileSetDocument};
use sindri_decay::{PrefabSources, ProfileSources, ScriptSources};
use weave::Stylesheet;

/// A project's assets, decoded and ready to play.
pub struct ProjectAssets {
    /// Every scene the project ships, entry scene first.
    ///
    /// Keeping the whole set is what makes `Scene.go` meaningful: the entry
    /// scene is entered, and the others wait for a script to ask.
    pub scenes: Vec<(String, SceneDocument)>,
    pub scripts: ScriptSources,
    pub prefabs: PrefabSources,
    pub profiles: ProfileSources,
    /// Every imported model the project ships, decoded once.
    pub models: Vec<(AssetId, ModelAsset)>,
    pub textures: Vec<(AssetId, TextureAsset)>,
    pub fonts: Vec<(AssetId, FontAsset)>,
    pub audio: Vec<(AssetId, AudioAsset)>,
    pub sheets: BTreeMap<String, SpriteSheetDocument>,
    pub tile_sets: Vec<(AssetId, TileSetDocument)>,
    pub stylesheets: Vec<Stylesheet>,
    /// Every sprite shader the project ships, as its path and its source.
    /// Checked when the host binds it, not here.
    pub shaders: Vec<(String, String)>,
    pub asset_count: usize,
}

#[cfg(not(target_arch = "wasm32"))]
impl ProjectAssets {
    /// Reads and decodes the project in the directory `project`.
    ///
    /// # Errors
    /// A manifest, scene, script or asset that will not read or decode, named.
    pub fn read(project: &std::path::Path) -> Result<Self, String> {
        super::directory::read(project)
    }
}
