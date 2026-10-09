//! Everything a project is, loaded: what both of the player's sources fill
//! and what it installs.

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
    pub asset_count: usize,
}
