//! What Sindri installs for the assistant, and where it keeps it.
//!
//! Everything lives in one folder of the person's own — no installer, no
//! system-wide change, no password — so setting the assistant up is a download
//! and removing it is deleting that folder. The runner is a prebuilt llama.cpp
//! server unpacked there; the model is one file beside it. Both come from
//! committed manifests that pin a URL, a size and a SHA-256, and nothing is
//! used until its bytes match.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::Feature;
use super::fetch::{Asset, architecture, platform};

const RUNTIME_MANIFEST: &str = include_str!("../../assets/ai-runtime.json");
const MODEL_FILES: &str = include_str!("../../assets/ai-model-files.json");
const SCHEMA_VERSION: u32 = 1;

/// The model Sindri sets up: the one its repair cases are tuned against, and
/// the only one whose file the manifest pins today.
pub const MODEL: &str = "qwen2.5-coder:7b";

#[derive(Deserialize)]
struct Manifest {
    schema_version: u32,
    #[serde(alias = "runtimes")]
    models: BTreeMap<String, Vec<Asset>>,
}

fn entries(text: &str) -> Vec<Asset> {
    serde_json::from_str::<Manifest>(text)
        .ok()
        .filter(|manifest| manifest.schema_version == SCHEMA_VERSION)
        .map(|manifest| manifest.models.into_values().flatten().collect())
        .unwrap_or_default()
}

/// The runner published for this machine, if there is one.
pub fn runtime() -> Option<Asset> {
    runtime_for(platform(), architecture())
}

pub fn runtime_for(platform: &str, architecture: &str) -> Option<Asset> {
    entries(RUNTIME_MANIFEST)
        .into_iter()
        .find(|asset| asset.suits(platform, architecture))
}

/// The model file for a catalogue id, if the manifest pins one.
pub fn model_file(id: &str) -> Option<Asset> {
    entries(MODEL_FILES)
        .into_iter()
        .find(|asset| asset.id == id)
}

/// The assistant's folder, and the names of what goes in it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Home {
    root: PathBuf,
}

impl Home {
    pub fn at(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Where the assistant lives on this platform, in the person's own data
    /// folder: `~/.local/share` on Linux, Application Support on a Mac, and
    /// local app data on Windows.
    pub fn standard() -> Option<Self> {
        let env = |key: &str| std::env::var_os(key).map(PathBuf::from);
        let base = if cfg!(target_os = "macos") {
            env("HOME").map(|home| home.join("Library/Application Support/Sindri"))
        } else if cfg!(target_os = "windows") {
            env("LOCALAPPDATA").map(|data| data.join("Sindri"))
        } else {
            env("XDG_DATA_HOME")
                .or_else(|| env("HOME").map(|home| home.join(".local/share")))
                .map(|data| data.join("sindri"))
        };
        base.map(|base| Self::at(base.join("assistant")))
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn downloads(&self) -> PathBuf {
        self.root.join("downloads")
    }

    /// Where one version of the runner is unpacked. Versioned, so moving the
    /// pinned build forward never runs half of the old one.
    pub fn runtime_dir(&self, asset: &Asset) -> PathBuf {
        self.root.join("runtime").join(&asset.version)
    }

    pub fn model_path(&self, asset: &Asset) -> PathBuf {
        self.root
            .join("models")
            .join(asset.url.rsplit('/').next().unwrap_or("model.gguf"))
    }

    pub fn log(&self) -> PathBuf {
        self.root.join("runner.log")
    }

    fn saved(&self) -> PathBuf {
        self.root.join("assistant.json")
    }

    /// The runner's executable, when this version is unpacked.
    pub fn server(&self, asset: &Asset) -> Option<PathBuf> {
        find_server(&self.runtime_dir(asset))
    }

    /// Whether the model file is in place.
    ///
    /// By size rather than by hash: the hash was checked when it arrived, and
    /// hashing five gigabytes every time the editor opens would make opening
    /// the panel take seconds. A file of the right size that is not the right
    /// file is one somebody wrote there on purpose.
    pub fn has_model(&self, asset: &Asset) -> bool {
        fs::metadata(self.model_path(asset)).is_ok_and(|meta| meta.len() == asset.size)
    }

    /// Bytes the assistant takes up on disk.
    pub fn footprint(&self) -> u64 {
        fn walk(path: &Path) -> u64 {
            let Ok(meta) = fs::symlink_metadata(path) else {
                return 0;
            };
            if meta.is_dir() {
                fs::read_dir(path).map_or(0, |entries| {
                    entries.flatten().map(|entry| walk(&entry.path())).sum()
                })
            } else {
                meta.len()
            }
        }
        walk(&self.root)
    }

    /// What was proved about the installed model, if anything.
    pub fn load(&self) -> Option<Saved> {
        serde_json::from_str(&fs::read_to_string(self.saved()).ok()?).ok()
    }

    pub fn save(&self, saved: &Saved) -> std::io::Result<()> {
        fs::create_dir_all(&self.root)?;
        let text = serde_json::to_string_pretty(saved).map_err(std::io::Error::other)?;
        let staging = self.saved().with_extension("json.part");
        fs::write(&staging, text)?;
        fs::rename(staging, self.saved())
    }

    /// Removes everything the assistant put on this machine.
    pub fn remove(&self) -> std::io::Result<()> {
        match fs::remove_dir_all(&self.root) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            other => other,
        }
    }
}

/// What verification proved, kept so it is not re-run every time the editor
/// opens. Tied to the exact model file and runner build it was run against:
/// either changing means proving it again.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Saved {
    pub model_sha256: String,
    pub runtime_version: String,
    pub verified: Vec<String>,
}

impl Saved {
    pub fn new(model: &Asset, runtime: &Asset, verified: &[Feature]) -> Self {
        Self {
            model_sha256: model.sha256.clone(),
            runtime_version: runtime.version.clone(),
            verified: verified
                .iter()
                .map(|feature| feature.id().to_owned())
                .collect(),
        }
    }

    /// The features proved, if this record is about these files.
    pub fn features_for(&self, model: &Asset, runtime: &Asset) -> Option<Vec<Feature>> {
        (self.model_sha256 == model.sha256 && self.runtime_version == runtime.version).then(|| {
            self.verified
                .iter()
                .filter_map(|id| Feature::from_id(id))
                .collect()
        })
    }
}

/// The server executable's name on this platform.
pub const fn server_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "llama-server.exe"
    } else {
        "llama-server"
    }
}

/// Finds the server anywhere under a folder, since release archives nest it at
/// different depths from one build to the next.
pub fn find_server(dir: &Path) -> Option<PathBuf> {
    let entries = fs::read_dir(dir).ok()?;
    let mut folders = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            folders.push(path);
        } else if path.file_name().is_some_and(|name| name == server_name()) {
            return Some(path);
        }
    }
    folders.iter().find_map(|folder| find_server(folder))
}

/// Unpacks a runner archive into `into`, keeping its layout so the server
/// finds the libraries shipped beside it.
///
/// With the platform's own `tar`, which reads both the `.tar.gz` and `.zip`
/// builds llama.cpp publishes, for the reason downloads use curl: no archive
/// crates in the editor's tree. Unpacked beside the destination and renamed
/// into place, so a half-unpacked runner never sits where one is looked for.
pub fn unpack(archive: &Path, into: &Path) -> std::io::Result<PathBuf> {
    let staging = into.with_extension("unpacking");
    let _ = fs::remove_dir_all(&staging);
    fs::create_dir_all(&staging)?;
    let status = std::process::Command::new("tar")
        .arg("-xf")
        .arg(archive)
        .arg("-C")
        .arg(&staging)
        .status()?;
    if !status.success() {
        let _ = fs::remove_dir_all(&staging);
        return Err(std::io::Error::other(
            "the runner archive could not be unpacked",
        ));
    }
    let Some(found) = find_server(&staging) else {
        let _ = fs::remove_dir_all(&staging);
        return Err(std::io::Error::other("the runner archive held no server"));
    };
    executable(&found)?;
    let _ = fs::remove_dir_all(into);
    if let Some(parent) = into.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::rename(&staging, into)?;
    find_server(into).ok_or_else(|| std::io::Error::other("the runner went missing"))
}

#[cfg(unix)]
fn executable(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(permissions.mode() | 0o755);
    fs::set_permissions(path, permissions)
}

#[cfg(not(unix))]
fn executable(_: &Path) -> std::io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests;
