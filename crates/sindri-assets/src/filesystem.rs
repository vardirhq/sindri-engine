use std::{io, path::PathBuf};

use sindri_core::{AssetId, AssetLoadErrorKind};

use crate::{AssetBytes, AssetSource, AssetSourceError, AssetSourceFuture};

#[derive(Clone, Debug)]
pub struct FileSystemAssetSource {
    root: PathBuf,
}

impl FileSystemAssetSource {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &std::path::Path {
        &self.root
    }

    /// Every asset under the root with this extension, as the IDs a scene
    /// would name them by: relative, with forward slashes, in a stable order.
    ///
    /// For assets that belong to the whole project rather than to whatever
    /// references them — a script's type can be named by any other script, so
    /// every script has to be known, not only those a scene points at. Hidden
    /// directories and build output are skipped, and the walk stops a few
    /// levels down: a project's assets are not a tree worth searching deeply.
    #[must_use]
    pub fn assets_with_extension(&self, extension: &str) -> Vec<String> {
        fn walk(
            root: &std::path::Path,
            dir: &std::path::Path,
            depth: usize,
            extension: &str,
            into: &mut Vec<String>,
        ) {
            let Ok(entries) = std::fs::read_dir(dir) else {
                return;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if name.starts_with('.') || name == "target" || name == "node_modules" {
                    continue;
                }
                if path.is_dir() {
                    if depth < 6 {
                        walk(root, &path, depth + 1, extension, into);
                    }
                } else if path.extension().is_some_and(|found| found == extension)
                    && let Ok(relative) = path.strip_prefix(root)
                {
                    into.push(
                        relative
                            .components()
                            .map(|part| part.as_os_str().to_string_lossy().into_owned())
                            .collect::<Vec<_>>()
                            .join("/"),
                    );
                }
            }
        }
        let mut found = Vec::new();
        walk(&self.root, &self.root, 0, extension, &mut found);
        found.sort();
        found
    }

    fn read(&self, id: &AssetId) -> Result<AssetBytes, AssetSourceError> {
        let root = std::fs::canonicalize(&self.root)
            .map_err(|error| io_error(id, "filesystem", "resolve asset root", &error))?;
        let candidate = self.root.join(id.as_str());
        let resolved = std::fs::canonicalize(&candidate)
            .map_err(|error| io_error(id, "filesystem", "resolve asset path", &error))?;

        if !resolved.starts_with(&root) {
            return Err(AssetSourceError::new(
                id.clone(),
                "filesystem",
                AssetLoadErrorKind::AccessDenied,
                format!(
                    "resolved path '{}' escapes asset root '{}'",
                    resolved.display(),
                    root.display()
                ),
            ));
        }

        let bytes = std::fs::read(&resolved)
            .map_err(|error| io_error(id, "filesystem", "read asset", &error))?;
        Ok(AssetBytes::new(id.clone(), bytes))
    }
}

impl AssetSource for FileSystemAssetSource {
    fn name(&self) -> &'static str {
        "filesystem"
    }

    fn load<'a>(&'a self, id: &'a AssetId) -> AssetSourceFuture<'a> {
        Box::pin(async move { self.read(id) })
    }
}

fn io_error(
    id: &AssetId,
    source: &'static str,
    operation: &str,
    error: &io::Error,
) -> AssetSourceError {
    let kind = match error.kind() {
        io::ErrorKind::NotFound => AssetLoadErrorKind::NotFound,
        io::ErrorKind::PermissionDenied => AssetLoadErrorKind::AccessDenied,
        _ => AssetLoadErrorKind::Io,
    };
    AssetSourceError::new(
        id.clone(),
        source,
        kind,
        format!("could not {operation}: {error}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filesystem_source_reads_relative_logical_ids() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::create_dir(directory.path().join("scenes")).unwrap();
        std::fs::write(directory.path().join("scenes/room.json"), b"room").unwrap();
        let source = FileSystemAssetSource::new(directory.path());
        let id = AssetId::new("scenes/room.json").unwrap();

        let loaded = pollster::block_on(source.load(&id)).unwrap();
        assert_eq!(loaded.id(), &id);
        assert_eq!(loaded.as_slice(), b"room");
    }

    #[test]
    fn filesystem_source_classifies_missing_assets() {
        let directory = tempfile::tempdir().unwrap();
        let source = FileSystemAssetSource::new(directory.path());
        let id = AssetId::new("textures/missing.png").unwrap();

        let error = pollster::block_on(source.load(&id)).unwrap_err();
        assert_eq!(error.kind(), AssetLoadErrorKind::NotFound);
        assert!(error.to_string().contains("textures/missing.png"));
    }

    #[cfg(unix)]
    #[test]
    fn filesystem_source_rejects_symlinks_outside_the_root() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("secret.txt"), b"secret").unwrap();
        symlink(outside.path(), root.path().join("linked")).unwrap();
        let source = FileSystemAssetSource::new(root.path());
        let id = AssetId::new("linked/secret.txt").unwrap();

        let error = pollster::block_on(source.load(&id)).unwrap_err();
        assert_eq!(error.kind(), AssetLoadErrorKind::AccessDenied);
    }
}
