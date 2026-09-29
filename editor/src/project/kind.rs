//! What kind of thing a file in the project is, judged by its name.

use super::sheet::SHEET_SUFFIX;

/// What kind of thing an entry is, as far as the browser can tell.
///
/// From the extension, because that is all a file offers before something opens
/// it. `Other` is deliberate: an unrecognised file is still listed, since a
/// browser that hides what it does not understand is a browser you cannot trust
/// to be showing you the directory.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssetKind {
    Folder,
    Scene,
    Texture,
    Sprite,
    Sheet,
    Mesh,
    Prefab,
    Profile,
    TileSet,
    Script,
    Stylesheet,
    Font,
    Audio,
    Other,
}

impl AssetKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Folder => "Folder",
            Self::Scene => "Scene",
            Self::Texture => "Texture",
            Self::Sprite => "Sprite",
            Self::Sheet => "Sheet",
            Self::Mesh => "Mesh",
            Self::Prefab => "Prefab",
            Self::Profile => "Profile",
            Self::TileSet => "Tile Set",
            Self::Script => "Script",
            Self::Stylesheet => "Weave",
            Self::Font => "Font",
            Self::Audio => "Audio",
            Self::Other => "File",
        }
    }

    #[must_use]
    pub fn of_path(path: &std::path::Path) -> Self {
        if path.is_dir() {
            return Self::Folder;
        }
        Self::of_file(&path.to_string_lossy())
    }

    /// What a file of this name is, judged by its extension.
    ///
    /// Native `.scene` files are canonical. `.scene.json` remains recognized so
    /// projects authored by older Sindri versions do not turn into anonymous
    /// JSON files in the browser.
    pub(crate) fn of_file(name: &str) -> Self {
        let lower = name.to_lowercase();
        if lower.ends_with(sindri_core::SCENE_SUFFIX)
            || lower.ends_with(sindri_core::LEGACY_SCENE_SUFFIX)
        {
            return Self::Scene;
        }
        if lower.ends_with(SHEET_SUFFIX) {
            return Self::Sheet;
        }
        if lower.ends_with(sindri_core::PREFAB_SUFFIX) {
            return Self::Prefab;
        }
        if lower.ends_with(sindri_core::PROFILE_SUFFIX) {
            return Self::Profile;
        }
        if lower.ends_with(sindri_core::TILESET_SUFFIX) {
            return Self::TileSet;
        }
        match lower.rsplit_once('.').map(|(_, extension)| extension) {
            Some("png" | "jpg" | "jpeg" | "webp" | "bmp" | "ktx2" | "dds") => Self::Texture,
            Some("gltf" | "glb" | "obj" | "fbx") => Self::Mesh,
            Some("decay" | "rs" | "ts" | "js" | "wgsl") => Self::Script,
            Some("weave") => Self::Stylesheet,
            Some("ttf" | "otf" | "woff" | "woff2") => Self::Font,
            Some("wav" | "ogg" | "mp3") => Self::Audio,
            _ => Self::Other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AssetKind;

    #[test]
    fn native_and_legacy_scene_names_are_scenes() {
        assert_eq!(AssetKind::of_file("level.scene"), AssetKind::Scene);
        assert_eq!(AssetKind::of_file("level.scene.json"), AssetKind::Scene);
        assert_eq!(AssetKind::of_file("LEVEL.SCENE"), AssetKind::Scene);
    }

    #[test]
    fn arbitrary_json_is_not_a_scene() {
        assert_eq!(AssetKind::of_file("settings.json"), AssetKind::Other);
    }
}
