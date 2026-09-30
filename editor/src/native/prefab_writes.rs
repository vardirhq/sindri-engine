//! Prefab files the editor writes, taken back and put again by Undo and Redo.
//!
//! Apply and Make Prefab change a file as well as the scene. The scene's half
//! is a transaction in the history like any other edit; the file's half is
//! recorded here against the two revisions that transaction moved between, so
//! stepping the history across it writes the file back as it was. The file is
//! only touched if it still says what the editor wrote: one changed on disk
//! since is somebody else's edit, and is left alone with a note saying so.

use std::path::{Path, PathBuf};

use sindri_core::PrefabDocument;

use super::EditorApp;

/// What the editor keeps about prefabs across edits.
#[derive(Default)]
pub(super) struct PrefabSession {
    /// The scene a prefab was opened from to be edited, to go back to.
    pub(super) returning_to: Option<PathBuf>,
    /// Every prefab file an edit wrote, in the order they were written.
    writes: Vec<PrefabWrite>,
}

/// One prefab file an edit wrote.
struct PrefabWrite {
    /// The history's revision before the edit, and after it.
    before_revision: u64,
    after_revision: u64,
    path: PathBuf,
    /// The asset ID the scene places it by.
    source: String,
    /// The file before the edit, or `None` when the edit made it.
    before: Option<String>,
    after: String,
}

/// A prefab file an edit is about to write, before the edit has a revision.
pub(super) struct PendingWrite {
    pub(super) path: PathBuf,
    pub(super) source: String,
    pub(super) before: Option<String>,
    pub(super) after: String,
}

impl PendingWrite {
    /// Writes `document` to `path`, remembering what was there.
    pub(super) fn write(
        path: &Path,
        source: &str,
        document: &PrefabDocument,
    ) -> Result<Self, String> {
        let after = document
            .to_canonical_json()
            .map_err(|error| error.to_string())?;
        let before = std::fs::read_to_string(path).ok();
        std::fs::create_dir_all(path.parent().unwrap_or(path))
            .and_then(|()| std::fs::write(path, &after))
            .map_err(|error| format!("{} was not written: {error}", path.display()))?;
        Ok(Self {
            path: path.to_path_buf(),
            source: source.to_owned(),
            before,
            after,
        })
    }
}

impl PrefabSession {
    /// Records the files an edit wrote as moving from revision `before` to
    /// `after`.
    fn record(&mut self, writes: Vec<PendingWrite>, before: u64, after: u64) {
        if after == before {
            return;
        }
        self.writes
            .extend(writes.into_iter().map(|write| PrefabWrite {
                before_revision: before,
                after_revision: after,
                path: write.path,
                source: write.source,
                before: write.before,
                after: write.after,
            }));
    }

    /// Puts the files a step of the history from `from` to `to` crossed back
    /// or forward, answering with what each prefab now says — its asset ID
    /// and text, or `None` for a file taken away — or why one was left alone.
    fn step(&self, from: u64, to: u64) -> Vec<Result<(String, Option<String>), String>> {
        let mut stepped = Vec::new();
        for write in &self.writes {
            let (expected, wanted) = if write.after_revision == from && write.before_revision == to
            {
                (Some(&write.after), write.before.as_ref())
            } else if write.before_revision == from && write.after_revision == to {
                (write.before.as_ref(), Some(&write.after))
            } else {
                continue;
            };
            let on_disk = std::fs::read_to_string(&write.path).ok();
            if on_disk.as_ref() != expected {
                stepped.push(Err(format!(
                    "{} changed on disk since, and was left as it is",
                    write.path.display()
                )));
                continue;
            }
            let outcome = match wanted {
                Some(text) => std::fs::write(&write.path, text).map(|()| Some(text.clone())),
                None => std::fs::remove_file(&write.path).map(|()| None),
            };
            stepped.push(
                outcome
                    .map(|text| (write.source.clone(), text))
                    .map_err(|error| format!("{}: {error}", write.path.display())),
            );
        }
        stepped
    }
}

impl EditorApp {
    /// Records the files an edit wrote against the revisions it moved between.
    pub(super) fn record_prefab_writes(&mut self, writes: Vec<PendingWrite>, before: u64) {
        let after = self.history.revision();
        self.prefab_session.record(writes, before, after);
    }

    /// Puts prefab files where a step of the history from revision `from`
    /// left them: back after an undo, forward again after a redo.
    pub(super) fn replay_prefab_writes(&mut self, from: u64) {
        let to = self.history.revision();
        for outcome in self.prefab_session.step(from, to) {
            match outcome {
                Ok((source, Some(text))) => {
                    if let Ok(prefab) = PrefabDocument::from_json(&text) {
                        self.file.prefabs_mut().replace(&source, prefab);
                    }
                }
                Ok((_, None)) => self.refresh_project(),
                Err(note) => self.console.warning(note),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use sindri_core::{PrefabDocument, SceneEntity, SceneEntityId};

    use super::{PendingWrite, PrefabSession};

    fn prefab(name: &str) -> PrefabDocument {
        PrefabDocument::single(SceneEntity {
            name: Some(name.to_owned()),
            ..SceneEntity::new(SceneEntityId::new("coin").unwrap())
        })
    }

    #[test]
    fn a_write_is_taken_back_and_put_again_with_the_history() {
        let directory = std::env::temp_dir().join(format!("sindri-writes-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        let path = directory.join("prefabs/coin.prefab");
        let old = PendingWrite::write(&path, "prefabs/coin.prefab", &prefab("Old")).unwrap();
        let mut session = PrefabSession::default();
        session.record(vec![old], 1, 2);
        let new = PendingWrite::write(&path, "prefabs/coin.prefab", &prefab("New")).unwrap();
        session.record(vec![new], 2, 3);

        let read = || std::fs::read_to_string(&path).ok();
        let old_text = prefab("Old").to_canonical_json().unwrap();
        let new_text = prefab("New").to_canonical_json().unwrap();
        assert_eq!(session.step(3, 2).len(), 1);
        assert_eq!(read(), Some(old_text));
        session.step(2, 1);
        assert_eq!(read(), None, "made by the first edit, so gone before it");
        session.step(1, 2);
        session.step(2, 3);
        assert_eq!(read(), Some(new_text));

        std::fs::write(&path, "somebody else's").unwrap();
        let stepped = session.step(3, 2);
        assert!(stepped[0].is_err(), "a file changed since is left alone");
        assert_eq!(read().as_deref(), Some("somebody else's"));
        let _ = std::fs::remove_dir_all(&directory);
    }
}
