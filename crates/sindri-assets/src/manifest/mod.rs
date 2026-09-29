//! What a project ships, and what each of its assets is supposed to be.
//!
//! A scene names `textures/badge.png`. On a developer's machine that resolves to
//! a file, and a wrong one fails loudly. On static web hosting it resolves to a
//! URL, and the ways it can be wrong are quieter: a truncated response, a stale
//! entry in a CDN, a deploy that replaced half the files. The bytes arrive, they
//! decode, and the picture is last week's.
//!
//! A manifest is the project saying, once and in advance, what each asset is.
//! Two things follow from that. A build knows what to publish without walking a
//! directory at deploy time, and a load can check what arrived against what was
//! promised rather than trusting whatever came back.
//!
//! The hash is SHA-256 of the bytes as they are stored, before any decoding. It
//! is not a security boundary — anyone who can replace an asset can replace the
//! manifest beside it — but it is the same digest the browser's subresource
//! integrity uses, so the day this feeds a `<link integrity>` the numbers are
//! already the right ones.

use std::{collections::BTreeMap, fmt, str::FromStr};

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};
use sindri_core::{
    AssetId, AssetLoadError, AssetLoadErrorKind, LEGACY_PREFAB_SUFFIX, LEGACY_PROFILE_SUFFIX,
    LEGACY_SCENE_SUFFIX, LEGACY_SHEET_SUFFIX, LEGACY_TILESET_SUFFIX, PREFAB_SUFFIX,
    PROFILE_SUFFIX, SCENE_SUFFIX, SHEET_SUFFIX, TILESET_SUFFIX,
};
use thiserror::Error;

#[cfg(test)]
mod tests;

pub const MANIFEST_FORMAT_VERSION: u32 = 1;
const ALGORITHM: &str = "sha256";

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ContentHash([u8; 32]);

impl ContentHash {
    pub fn of(bytes: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        Self(hasher.finalize().into())
    }

    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for ContentHash {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(ALGORITHM)?;
        formatter.write_str(":")?;
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl FromStr for ContentHash {
    type Err = ManifestError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let digits = value
            .strip_prefix(ALGORITHM)
            .and_then(|rest| rest.strip_prefix(':'))
            .ok_or_else(|| ManifestError::UnknownAlgorithm(value.to_owned()))?;
        if digits.len() != 64 {
            return Err(ManifestError::MalformedHash(value.to_owned()));
        }
        let mut bytes = [0_u8; 32];
        for (byte, pair) in bytes.iter_mut().zip(digits.as_bytes().chunks_exact(2)) {
            let pair = std::str::from_utf8(pair)
                .map_err(|_| ManifestError::MalformedHash(value.to_owned()))?;
            *byte = u8::from_str_radix(pair, 16)
                .map_err(|_| ManifestError::MalformedHash(value.to_owned()))?;
        }
        Ok(Self(bytes))
    }
}

impl Serialize for ContentHash {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for ContentHash {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetKind {
    Scene,
    Script,
    Prefab,
    Profile,
    Texture,
    Sheet,
    TileSet,
    Font,
    Audio,
    Other,
}

impl AssetKind {
    pub const ALL: [Self; 10] = [
        Self::Scene,
        Self::Prefab,
        Self::Profile,
        Self::Sheet,
        Self::TileSet,
        Self::Script,
        Self::Texture,
        Self::Font,
        Self::Audio,
        Self::Other,
    ];

    #[must_use]
    pub fn for_id(id: &str) -> Self {
        match id.rsplit_once('.').map(|(_, extension)| extension) {
            Some("wav" | "ogg" | "mp3" | "flac") => Self::Audio,
            Some("png" | "jpg" | "jpeg") => Self::Texture,
            Some("ttf" | "otf") => Self::Font,
            Some("decay") => Self::Script,
            _ if id.ends_with(PREFAB_SUFFIX) || id.ends_with(LEGACY_PREFAB_SUFFIX) => {
                Self::Prefab
            }
            _ if id.ends_with(PROFILE_SUFFIX) || id.ends_with(LEGACY_PROFILE_SUFFIX) => {
                Self::Profile
            }
            _ if id.ends_with(SHEET_SUFFIX) || id.ends_with(LEGACY_SHEET_SUFFIX) => Self::Sheet,
            _ if id.ends_with(TILESET_SUFFIX) || id.ends_with(LEGACY_TILESET_SUFFIX) => {
                Self::TileSet
            }
            _ if id.ends_with(SCENE_SUFFIX) || id.ends_with(LEGACY_SCENE_SUFFIX) => Self::Scene,
            _ => Self::Other,
        }
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Scene => "scene",
            Self::Script => "script",
            Self::Prefab => "prefab",
            Self::Profile => "profile",
            Self::Texture => "texture",
            Self::Sheet => "sheet",
            Self::TileSet => "tile_set",
            Self::Font => "font",
            Self::Audio => "audio",
            Self::Other => "other",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ManifestEntry {
    #[serde(default = "other_kind", skip_serializing_if = "is_other")]
    pub kind: AssetKind,
    pub bytes: u64,
    pub hash: ContentHash,
}

const fn other_kind() -> AssetKind {
    AssetKind::Other
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_other(kind: &AssetKind) -> bool {
    matches!(*kind, AssetKind::Other)
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AssetManifest {
    format_version: u32,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    content_root: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    entry_scene: Option<AssetId>,
    assets: BTreeMap<AssetId, ManifestEntry>,
}

impl Default for AssetManifest {
    fn default() -> Self {
        Self {
            format_version: MANIFEST_FORMAT_VERSION,
            content_root: String::new(),
            entry_scene: None,
            assets: BTreeMap::new(),
        }
    }
}

impl AssetManifest {
    pub fn new() -> Self {
        Self::default()
    }

    pub const fn format_version(&self) -> u32 {
        self.format_version
    }

    pub fn content_root(&self) -> &str {
        &self.content_root
    }

    pub fn set_content_root(&mut self, root: impl Into<String>) {
        self.content_root = root.into();
    }

    pub fn insert(&mut self, id: AssetId, bytes: &[u8]) -> Option<ManifestEntry> {
        let kind = AssetKind::for_id(id.as_str());
        self.insert_as(id, kind, bytes)
    }

    pub fn insert_as(
        &mut self,
        id: AssetId,
        kind: AssetKind,
        bytes: &[u8],
    ) -> Option<ManifestEntry> {
        if kind == AssetKind::Scene && self.entry_scene.is_none() {
            self.entry_scene = Some(id.clone());
        }
        self.assets.insert(
            id,
            ManifestEntry {
                kind,
                bytes: bytes.len() as u64,
                hash: ContentHash::of(bytes),
            },
        )
    }

    pub fn get(&self, id: &AssetId) -> Option<&ManifestEntry> {
        self.assets.get(id)
    }

    pub fn len(&self) -> usize {
        self.assets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.assets.is_empty()
    }

    pub fn assets(&self) -> impl ExactSizeIterator<Item = (&AssetId, &ManifestEntry)> {
        self.assets.iter()
    }

    pub fn ids_of(&self, kind: AssetKind) -> impl Iterator<Item = &AssetId> {
        let entry_scene = if kind == AssetKind::Scene {
            self.entry_scene.as_ref()
        } else {
            None
        };
        entry_scene.into_iter().chain(
            self.assets
                .iter()
                .filter(move |(id, entry)| entry.kind == kind && entry_scene != Some(*id))
                .map(|(id, _)| id),
        )
    }

    pub fn verify(&self, id: &AssetId, bytes: &[u8]) -> Result<(), AssetLoadError> {
        let Some(entry) = self.assets.get(id) else {
            return Ok(());
        };
        if entry.bytes != bytes.len() as u64 {
            return Err(AssetLoadError::new(
                id.clone(),
                AssetLoadErrorKind::InvalidData,
                format!(
                    "the manifest expects {} bytes and {} arrived",
                    entry.bytes,
                    bytes.len()
                ),
            ));
        }
        let hash = ContentHash::of(bytes);
        if hash != entry.hash {
            return Err(AssetLoadError::new(
                id.clone(),
                AssetLoadErrorKind::InvalidData,
                format!(
                    "the manifest expects {} and the bytes are {hash}",
                    entry.hash
                ),
            ));
        }
        Ok(())
    }

    pub fn to_canonical_json(&self) -> Result<String, ManifestError> {
        let mut text = serde_json::to_string_pretty(self)
            .map_err(|error| ManifestError::Json(error.to_string()))?;
        text.push('\n');
        Ok(text)
    }

    pub fn from_json(text: &str) -> Result<Self, ManifestError> {
        let manifest: Self =
            serde_json::from_str(text).map_err(|error| ManifestError::Json(error.to_string()))?;
        if manifest.format_version != MANIFEST_FORMAT_VERSION {
            return Err(ManifestError::UnsupportedVersion(manifest.format_version));
        }
        Ok(manifest)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn of_directory(root: &std::path::Path) -> Result<Self, ManifestError> {
        let mut manifest = Self::new();
        collect(root, root, &mut manifest)?;
        Ok(manifest)
    }
}

pub const MANIFEST_FILE_NAME: &str = "sindri.manifest.json";

#[cfg(not(target_arch = "wasm32"))]
fn collect(
    root: &std::path::Path,
    directory: &std::path::Path,
    manifest: &mut AssetManifest,
) -> Result<(), ManifestError> {
    let listing = std::fs::read_dir(directory)
        .map_err(|error| ManifestError::Read(format!("{}: {error}", directory.display())))?;
    let mut entries: Vec<std::path::PathBuf> = listing
        .map(|entry| {
            entry
                .map(|entry| entry.path())
                .map_err(|error| ManifestError::Read(format!("{}: {error}", directory.display())))
        })
        .collect::<Result<_, _>>()?;
    entries.sort();

    for path in entries {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        if name.starts_with('.') || name == MANIFEST_FILE_NAME {
            continue;
        }
        if path.is_dir() {
            collect(root, &path, manifest)?;
            continue;
        }
        let relative = path
            .strip_prefix(root)
            .map_err(|_| ManifestError::Read(format!("{} is not under the root", path.display())))?
            .to_string_lossy()
            .replace('\\', "/");
        let id = AssetId::new(relative.clone())
            .map_err(|error| ManifestError::AssetId(format!("{relative}: {error}")))?;
        let bytes = std::fs::read(&path)
            .map_err(|error| ManifestError::Read(format!("{}: {error}", path.display())))?;
        manifest.insert(id, &bytes);
    }
    Ok(())
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ManifestError {
    #[error("manifest format version {0} is not supported")]
    UnsupportedVersion(u32),
    #[error("'{0}' does not name a digest this build understands")]
    UnknownAlgorithm(String),
    #[error("'{0}' is not a well-formed digest")]
    MalformedHash(String),
    #[error("could not read the project: {0}")]
    Read(String),
    #[error("a file does not name a valid asset: {0}")]
    AssetId(String),
    #[error("manifest JSON: {0}")]
    Json(String),
}
