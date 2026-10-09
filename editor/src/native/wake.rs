//! Waking an editor at rest when something outside it changes.
//!
//! An editor nobody is touching draws nothing: egui runs a frame only when
//! input, a run, an animation or a load asks for one. What it would miss is
//! the disk — a script saved in another editor, a stylesheet, a texture —
//! because the editor notices those by looking during a frame. This watches
//! the open project's folder off the frame and asks for frames when a file
//! changes: one at once, and one after the editor's own watchers look again,
//! since they look at most once a second.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use eframe::egui;

/// How often the folder is looked at. Cheap: names, sizes and times only.
const LOOK_EVERY: Duration = Duration::from_millis(500);

/// After a change, a second frame this much later, once the editor's own
/// watchers (scripts and textures look once a second) are due to see it.
const AND_AGAIN_AFTER: Duration = Duration::from_millis(1100);

/// More files than this and the folder is not looked into further: a project
/// is not that big, and a home directory opened by mistake should not cost a
/// core.
const MOST_FILES: usize = 20_000;

/// The folder being watched, shared with the thread that watches it. The
/// thread ends with the editor, when it holds the only reference.
pub(super) struct DiskWatch {
    root: Arc<Mutex<Option<PathBuf>>>,
}

impl DiskWatch {
    pub(super) fn start(context: egui::Context) -> Self {
        let root = Arc::new(Mutex::new(None::<PathBuf>));
        let watched = Arc::clone(&root);
        let started = std::thread::Builder::new()
            .name("sindri-editor-disk-watch".to_owned())
            .spawn(move || {
                let mut last: Option<(PathBuf, u64)> = None;
                while Arc::strong_count(&watched) > 1 {
                    std::thread::sleep(LOOK_EVERY);
                    let Some(folder) = watched.lock().ok().and_then(|root| root.clone()) else {
                        last = None;
                        continue;
                    };
                    let now = stamp(&folder);
                    let changed = last
                        .as_ref()
                        .is_some_and(|(was, then)| *was == folder && *then != now);
                    if changed {
                        context.request_repaint();
                        context.request_repaint_after(AND_AGAIN_AFTER);
                    }
                    last = Some((folder, now));
                }
            });
        if let Err(error) = started {
            eprintln!("sindri-editor: files changing while idle will go unnoticed: {error}");
        }
        Self { root }
    }

    /// Watches `folder` from now on, or nothing.
    pub(super) fn follow(&self, folder: Option<&Path>) {
        if let Ok(mut root) = self.root.lock()
            && root.as_deref() != folder
        {
            *root = folder.map(Path::to_path_buf);
        }
    }
}

/// One number for every file under `root`: its path, size and modified time.
/// Build output and hidden folders are skipped; they are not the project's.
fn stamp(root: &Path) -> u64 {
    let mut hasher = DefaultHasher::new();
    let mut pending = vec![root.to_path_buf()];
    let mut seen = 0;
    while let Some(dir) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with('.') || name == "target" || name == "node_modules" {
                continue;
            }
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            if metadata.is_dir() {
                pending.push(entry.path());
                continue;
            }
            seen += 1;
            if seen > MOST_FILES {
                return hasher.finish();
            }
            entry.path().hash(&mut hasher);
            metadata.len().hash(&mut hasher);
            metadata.modified().ok().hash(&mut hasher);
        }
    }
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::stamp;

    #[test]
    fn a_saved_file_changes_the_stamp_and_build_output_does_not() {
        let root = std::env::temp_dir().join(format!("sindri-wake-{}", std::process::id()));
        std::fs::create_dir_all(root.join("target")).expect("a folder");
        std::fs::write(root.join("a.decay"), "one").expect("a file");
        let before = stamp(&root);
        std::fs::write(root.join("target/out"), "built").expect("build output");
        assert_eq!(stamp(&root), before, "build output is not the project's");
        std::fs::write(root.join("a.decay"), "one two").expect("a save");
        assert_ne!(stamp(&root), before, "a save is noticed");
        let _ = std::fs::remove_dir_all(&root);
    }
}
