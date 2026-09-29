//! Making, moving, copying and removing the files a project is made of.
//!
//! Every asset used to have to arrive from outside the editor: there was no
//! create, no folder, no rename, no delete, no duplicate and no import, so
//! building a project meant a file manager beside the window and the Refresh
//! button afterwards.
//!
//! These are disk writes, and disk writes are not commands. Nothing here goes
//! through the undo history, because the history describes a world and these
//! describe a directory — undoing a delete would mean the editor holding the
//! bytes of every file anyone removed for as long as the session lasted. So
//! the rules are the other ones that keep a destructive act honest: every
//! operation is checked before it runs, refuses rather than overwrites, and
//! the one that cannot be taken back is asked about first by the panel that
//! calls it.
//!
//! Kept apart from the drawing so that "what does this do to the directory"
//! is a question a test can ask with a temporary folder and no window.

use std::path::{Path, PathBuf};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AssetOpError {
    #[error("a name cannot be empty")]
    EmptyName,
    #[error("'{0}' is not a name: it points somewhere else in the file system")]
    NotAName(String),
    #[error("'{0}' already exists here")]
    Exists(String),
    #[error("that file is not inside this project")]
    OutsideProject,
    #[error("{path} could not be read or written: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

impl AssetOpError {
    fn io(path: &Path, source: std::io::Error) -> Self {
        Self::Io {
            path: path.display().to_string(),
            source,
        }
    }
}

fn checked_name(name: &str) -> Result<&str, AssetOpError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AssetOpError::EmptyName);
    }
    let mut parts = Path::new(name).components();
    let one_plain_part =
        matches!(parts.next(), Some(std::path::Component::Normal(_))) && parts.next().is_none();
    if name.contains(['/', '\\']) || !one_plain_part {
        return Err(AssetOpError::NotAName(name.to_owned()));
    }
    Ok(name)
}

fn inside(root: &Path, path: &Path) -> Result<(), AssetOpError> {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let anchor = path.parent().unwrap_or(path);
    let anchor = anchor
        .canonicalize()
        .unwrap_or_else(|_| anchor.to_path_buf());
    if anchor.starts_with(&root) {
        Ok(())
    } else {
        Err(AssetOpError::OutsideProject)
    }
}

fn vacant(path: &Path) -> Result<(), AssetOpError> {
    if path.exists() {
        return Err(AssetOpError::Exists(path.file_name().map_or_else(
            || path.display().to_string(),
            |name| name.to_string_lossy().into_owned(),
        )));
    }
    Ok(())
}

pub fn create_folder(root: &Path, parent: &Path, name: &str) -> Result<PathBuf, AssetOpError> {
    let name = checked_name(name)?;
    let path = parent.join(name);
    inside(root, &path)?;
    vacant(&path)?;
    std::fs::create_dir(&path).map_err(|source| AssetOpError::io(&path, source))?;
    Ok(path)
}

pub fn create_file(
    root: &Path,
    parent: &Path,
    name: &str,
    contents: &str,
) -> Result<PathBuf, AssetOpError> {
    let name = checked_name(name)?;
    let path = parent.join(name);
    inside(root, &path)?;
    vacant(&path)?;
    std::fs::write(&path, contents).map_err(|source| AssetOpError::io(&path, source))?;
    Ok(path)
}

pub fn rename(root: &Path, path: &Path, name: &str) -> Result<PathBuf, AssetOpError> {
    let name = checked_name(name)?;
    inside(root, path)?;
    let parent = path.parent().ok_or(AssetOpError::OutsideProject)?;
    let target = parent.join(name);
    if target == path {
        return Ok(target);
    }
    vacant(&target)?;
    std::fs::rename(path, &target).map_err(|source| AssetOpError::io(path, source))?;
    Ok(target)
}

pub fn duplicate(root: &Path, path: &Path) -> Result<PathBuf, AssetOpError> {
    inside(root, path)?;
    let parent = path.parent().ok_or(AssetOpError::OutsideProject)?;
    let target = unused_beside(parent, path);
    if path.is_dir() {
        copy_tree(path, &target)?;
    } else {
        std::fs::copy(path, &target).map_err(|source| AssetOpError::io(path, source))?;
    }
    Ok(target)
}

pub fn delete(root: &Path, path: &Path) -> Result<(), AssetOpError> {
    inside(root, path)?;
    if path.is_dir() {
        std::fs::remove_dir_all(path).map_err(|source| AssetOpError::io(path, source))
    } else {
        std::fs::remove_file(path).map_err(|source| AssetOpError::io(path, source))
    }
}

pub fn import(root: &Path, into: &Path, sources: &[PathBuf]) -> (Vec<PathBuf>, Vec<AssetOpError>) {
    let mut arrived = Vec::new();
    let mut refused = Vec::new();
    for source in sources {
        match import_one(root, into, source) {
            Ok(path) => arrived.push(path),
            Err(error) => refused.push(error),
        }
    }
    (arrived, refused)
}

fn import_one(root: &Path, into: &Path, source: &Path) -> Result<PathBuf, AssetOpError> {
    let name = source
        .file_name()
        .ok_or_else(|| AssetOpError::NotAName(source.display().to_string()))?;
    let target = into.join(name);
    inside(root, &target)?;
    vacant(&target)?;
    std::fs::copy(source, &target).map_err(|error| AssetOpError::io(source, error))?;
    Ok(target)
}

fn unused_beside(parent: &Path, path: &Path) -> PathBuf {
    let (stem, suffix) = split_name(path);
    let mut candidate = parent.join(format!("{stem} copy{suffix}"));
    let mut nth = 2_u32;
    while candidate.exists() {
        candidate = parent.join(format!("{stem} copy {nth}{suffix}"));
        nth += 1;
    }
    candidate
}

/// A file's name split into the part to add to and the part to keep.
///
/// Native `.scene` and legacy `.scene.json` are both indivisible suffixes, as
/// is `.sheet.json`. Keeping the whole asset suffix means duplication cannot
/// accidentally turn a recognized asset into an ordinary file.
fn split_name(path: &Path) -> (String, String) {
    let name = path
        .file_name()
        .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
    for suffix in [
        sindri_core::LEGACY_SCENE_SUFFIX,
        ".sheet.json",
        sindri_core::SCENE_SUFFIX,
    ] {
        if let Some(stem) = name.strip_suffix(suffix) {
            return (stem.to_owned(), suffix.to_owned());
        }
    }
    match name.rsplit_once('.') {
        Some((stem, extension)) if !stem.is_empty() => (stem.to_owned(), format!(".{extension}")),
        _ => (name, String::new()),
    }
}

fn copy_tree(from: &Path, to: &Path) -> Result<(), AssetOpError> {
    std::fs::create_dir_all(to).map_err(|source| AssetOpError::io(to, source))?;
    for entry in std::fs::read_dir(from).map_err(|source| AssetOpError::io(from, source))? {
        let entry = entry.map_err(|source| AssetOpError::io(from, source))?;
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), &target)
                .map_err(|source| AssetOpError::io(&entry.path(), source))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
